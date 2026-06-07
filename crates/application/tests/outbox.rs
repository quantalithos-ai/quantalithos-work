use work_application::{
    AuditRepository, BacklogRepository, JobResultRepository, PortError, ProjectBoardViewProjection,
    ProjectMemberRepository, ProjectRepository, WorkItemRepository, WorkOutboxPublishService,
    WorkOutboxRepository,
};
use work_contracts::fixtures;
use work_contracts::{DerivedFreshnessState, DerivedWorkViewRef, WorkOutboxSourceRef};
use work_domain::{
    Backlog, DerivedWorkViewState, Project, ProjectMember, WorkAuditTrail, WorkItem,
    WorkOutboxRecord, WorkTraceRecord,
};
use work_infra::{
    clock_id::DeterministicWorkIdGenerator, command_result_store::InMemoryCommandResultRepository,
    idempotency_store::InMemoryIdempotencyRepository, outbox_store::InMemoryWorkOutboxRepository,
    publishers::FakeWorkOutboxPublisher, repositories::InMemoryWorkStores,
};

fn seed_uow() -> work_application::UnitOfWorkHandle {
    work_application::UnitOfWorkHandle {
        handle_id: work_application::UnitOfWorkId("seed-uow".to_owned()),
    }
}

fn service() -> WorkOutboxPublishService<
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkOutboxRepository,
    InMemoryWorkStores,
    FakeWorkOutboxPublisher,
    InMemoryWorkStores,
    InMemoryIdempotencyRepository,
    InMemoryCommandResultRepository,
    DeterministicWorkIdGenerator,
> {
    let stores = InMemoryWorkStores::new();
    WorkOutboxPublishService {
        project_repo: stores.clone(),
        member_repo: stores.clone(),
        work_repo: stores.clone(),
        promote_repo: stores.clone(),
        backlog_repo: stores.clone(),
        dependency_repo: stores.clone(),
        iteration_repo: stores.clone(),
        audit_repo: stores.clone(),
        outbox_repo: InMemoryWorkOutboxRepository::new(),
        projection_repo: stores.clone(),
        publisher: FakeWorkOutboxPublisher::new(),
        unit_of_work: stores,
        idempotency: InMemoryIdempotencyRepository::new(),
        job_results: InMemoryCommandResultRepository::new(),
        ids: DeterministicWorkIdGenerator::new(),
    }
}

async fn enqueue(
    svc: &WorkOutboxPublishService<
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkOutboxRepository,
        InMemoryWorkStores,
        FakeWorkOutboxPublisher,
        InMemoryWorkStores,
        InMemoryIdempotencyRepository,
        InMemoryCommandResultRepository,
        DeterministicWorkIdGenerator,
    >,
    record: WorkOutboxRecord,
) {
    let uow = seed_uow();
    svc.outbox_repo
        .enqueue(record, &uow)
        .await
        .expect("seed outbox");
}

async fn seed_project_family(
    svc: &WorkOutboxPublishService<
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkOutboxRepository,
        InMemoryWorkStores,
        FakeWorkOutboxPublisher,
        InMemoryWorkStores,
        InMemoryIdempotencyRepository,
        InMemoryCommandResultRepository,
        DeterministicWorkIdGenerator,
    >,
) {
    let uow = seed_uow();
    let actor = fixtures::actor_context().actor_ref().clone();
    let project = Project::create(
        fixtures::project_id(),
        fixtures::project_spec(),
        actor.clone(),
    )
    .unwrap();
    ProjectRepository::create(&svc.project_repo, project, &uow)
        .await
        .unwrap();

    let backlog = Backlog::open_for_project(
        fixtures::backlog_id(),
        fixtures::project_id(),
        actor.clone(),
    )
    .unwrap();
    BacklogRepository::create(&svc.backlog_repo, backlog, &uow)
        .await
        .unwrap();

    let member = ProjectMember::assign(
        fixtures::project_member_id(),
        fixtures::project_id(),
        fixtures::global_member_ref(),
        fixtures::responsibility_spec(),
    )
    .unwrap();
    ProjectMemberRepository::create(&svc.member_repo, member, &uow)
        .await
        .unwrap();

    let work = WorkItem::formalize(
        fixtures::work_item_id(),
        fixtures::backlog_id(),
        fixtures::formal_work_intent(),
        fixtures::source_work_ref(),
        actor,
    )
    .unwrap();
    WorkItemRepository::create_work_item(&svc.work_repo, work, &uow)
        .await
        .unwrap();
}

async fn seed_trace_and_view_sources(
    svc: &WorkOutboxPublishService<
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkOutboxRepository,
        InMemoryWorkStores,
        FakeWorkOutboxPublisher,
        InMemoryWorkStores,
        InMemoryIdempotencyRepository,
        InMemoryCommandResultRepository,
        DeterministicWorkIdGenerator,
    >,
) {
    let uow = seed_uow();
    let trace = WorkTraceRecord::from_truth_change(
        fixtures::trace_id(),
        fixtures::project_created_change(),
        fixtures::trace_context_ref(),
    )
    .unwrap();
    AuditRepository::append_trace(&svc.audit_repo, trace.clone(), &uow)
        .await
        .unwrap();
    let mut trail =
        WorkAuditTrail::start_for_subject(fixtures::project_created_change().audit_subject_ref());
    trail.append(trace).unwrap();
    AuditRepository::save_audit_trail(&svc.audit_repo, trail, None, &uow)
        .await
        .unwrap();

    svc.projection_repo
        .seed_project_board_view(ProjectBoardViewProjection {
            view: work_contracts::ProjectBoardView {
                project_ref: fixtures::project_ref(),
                work_cards: Vec::new(),
                marker: work_contracts::ProjectionViewMarker {
                    view_ref: DerivedWorkViewRef::project_board(fixtures::project_ref()),
                    freshness_state: DerivedFreshnessState::Fresh,
                    source_cursor: fixtures::truth_cursor(),
                },
            },
            freshness: DerivedWorkViewState {
                view_ref: DerivedWorkViewRef::project_board(fixtures::project_ref()),
                freshness_state: DerivedFreshnessState::Fresh,
                source_cursor: fixtures::truth_cursor(),
            },
        });
}

#[tokio::test]
async fn publish_outbox_success_marks_published_and_saves_job_report() {
    let svc = service();
    seed_project_family(&svc).await;
    let record = WorkOutboxRecord::from_truth_change(
        fixtures::outbox_id(),
        fixtures::project_created_change(),
        fixtures::trace_context_ref(),
        fixtures::request_metadata(None).requested_at,
    )
    .unwrap();
    enqueue(&svc, record).await;

    let report = svc
        .publish_outbox(fixtures::publish_outbox_job_input())
        .await
        .expect("publish should succeed");

    assert_eq!(report.scanned_count, 1);
    assert_eq!(report.changed_count, 1);
    assert!(report.failed_refs.is_empty());
    assert_eq!(svc.publisher.publish_count(), 1);

    let stored = svc
        .outbox_repo
        .get(fixtures::outbox_id())
        .await
        .expect("load outbox")
        .expect("seeded outbox");
    assert_eq!(
        stored.record.publication_state,
        work_contracts::OutboxPublicationState::Published
    );

    let result_ref = report
        .receipt
        .as_ref()
        .expect("job receipt")
        .result_ref
        .clone();
    let stored_report = svc
        .job_results
        .get_report(result_ref)
        .await
        .expect("load stored report");
    assert!(matches!(
        stored_report,
        Some(work_application::StoredJobResult::WorkJob(saved)) if saved == report
    ));
}

#[tokio::test]
async fn publish_outbox_publisher_failure_marks_failed_and_reports_failed_ref() {
    let svc = service();
    seed_project_family(&svc).await;
    svc.publisher.push_result(Err(PortError::Unavailable));
    let record = WorkOutboxRecord::from_truth_change(
        fixtures::outbox_id(),
        fixtures::project_created_change(),
        fixtures::trace_context_ref(),
        fixtures::request_metadata(None).requested_at,
    )
    .unwrap();
    enqueue(&svc, record).await;

    let report = svc
        .publish_outbox(fixtures::publish_outbox_job_input())
        .await
        .expect("job should report failure, not reject");

    assert_eq!(report.scanned_count, 1);
    assert_eq!(report.changed_count, 1);
    assert_eq!(report.failed_refs.len(), 1);

    let stored = svc
        .outbox_repo
        .get(fixtures::outbox_id())
        .await
        .expect("load outbox")
        .expect("seeded outbox");
    assert_eq!(
        stored.record.publication_state,
        work_contracts::OutboxPublicationState::Failed
    );
}

#[tokio::test]
async fn publish_outbox_invalid_source_marks_failed_without_partial_publish() {
    let svc = service();
    let record = WorkOutboxRecord::from_event_source(
        fixtures::outbox_id(),
        WorkOutboxSourceRef::ProjectMember(fixtures::project_member_ref()),
        fixtures::trace_context_ref(),
        fixtures::request_metadata(None).requested_at,
    )
    .unwrap();
    enqueue(&svc, record).await;

    let report = svc
        .publish_outbox(fixtures::publish_outbox_job_input())
        .await
        .expect("invalid source should become failed marker");

    assert_eq!(report.scanned_count, 1);
    assert_eq!(report.changed_count, 1);
    assert_eq!(report.failed_refs.len(), 1);
    assert_eq!(svc.publisher.publish_count(), 0);

    let stored = svc
        .outbox_repo
        .get(fixtures::outbox_id())
        .await
        .expect("load outbox")
        .expect("seeded outbox");
    assert_eq!(
        stored.record.publication_state,
        work_contracts::OutboxPublicationState::Failed
    );
}

#[tokio::test]
async fn duplicate_job_replays_stored_report_without_rescanning_or_republishing() {
    let svc = service();
    seed_project_family(&svc).await;
    let record = WorkOutboxRecord::from_truth_change(
        fixtures::outbox_id(),
        fixtures::project_created_change(),
        fixtures::trace_context_ref(),
        fixtures::request_metadata(None).requested_at,
    )
    .unwrap();
    enqueue(&svc, record).await;
    let input = fixtures::publish_outbox_job_input();

    let first = svc.publish_outbox(input.clone()).await.expect("first run");
    let second = svc.publish_outbox(input).await.expect("duplicate replay");

    assert_eq!(svc.publisher.publish_count(), 1);
    assert_eq!(first.scanned_count, second.scanned_count);
    assert_eq!(first.changed_count, second.changed_count);
    assert_eq!(first.failed_refs, second.failed_refs);
    assert_eq!(
        second
            .receipt
            .as_ref()
            .expect("duplicate receipt")
            .idempotency,
        work_contracts::IdempotencyResultView::Duplicate
    );
}

#[tokio::test]
async fn publish_outbox_rebuilds_project_changed_payload_from_typed_source() {
    let svc = service();
    seed_project_family(&svc).await;
    let record = WorkOutboxRecord::from_truth_change(
        fixtures::outbox_id(),
        fixtures::project_created_change(),
        fixtures::trace_context_ref(),
        fixtures::request_metadata(None).requested_at,
    )
    .unwrap();
    enqueue(&svc, record).await;

    svc.publish_outbox(fixtures::publish_outbox_job_input())
        .await
        .expect("publish should succeed");

    let published = svc.publisher.published();
    assert_eq!(published.len(), 1);
    match &published[0] {
        work_contracts::WorkOutboundPublication::ProjectChanged(envelope) => {
            assert_eq!(envelope.payload.project_ref, fixtures::project_ref());
            assert_eq!(
                envelope.payload.lifecycle_state,
                work_contracts::ProjectLifecycleState::Active
            );
            assert_eq!(envelope.payload.reason, fixtures::project_created_reason());
        }
        other => panic!("unexpected publication: {other:?}"),
    }
}

#[tokio::test]
async fn publish_outbox_supports_trace_and_derived_view_sources() {
    let svc = service();
    seed_trace_and_view_sources(&svc).await;
    let trace_record = WorkOutboxRecord::from_event_source(
        fixtures::outbox_id(),
        WorkOutboxSourceRef::TraceAvailable {
            trace_id: fixtures::trace_id(),
            handoff_ref: Some(fixtures::trace_handoff_ref()),
        },
        fixtures::trace_context_ref(),
        fixtures::request_metadata(None).requested_at,
    )
    .unwrap();
    let view_record = WorkOutboxRecord::from_event_source(
        work_contracts::WorkOutboxId("outbox-2".to_owned()),
        WorkOutboxSourceRef::DerivedView(
            DerivedWorkViewRef::project_board(fixtures::project_ref()),
        ),
        fixtures::trace_context_ref(),
        fixtures::request_metadata(None).requested_at,
    )
    .unwrap();
    enqueue(&svc, trace_record).await;
    enqueue(&svc, view_record).await;

    let report = svc
        .publish_outbox(fixtures::publish_outbox_job_input())
        .await
        .expect("publish");

    assert_eq!(report.scanned_count, 2);
    assert_eq!(report.changed_count, 2);
    assert!(report.failed_refs.is_empty());
    assert_eq!(svc.publisher.published().len(), 2);
}

#[tokio::test]
async fn publish_outbox_version_conflict_reports_failed_ref_without_rejecting_job() {
    let svc = service();
    seed_project_family(&svc).await;
    let record = WorkOutboxRecord::from_truth_change(
        fixtures::outbox_id(),
        fixtures::project_created_change(),
        fixtures::trace_context_ref(),
        fixtures::request_metadata(None).requested_at,
    )
    .unwrap();
    enqueue(&svc, record).await;
    svc.outbox_repo
        .inject_version_conflict(&fixtures::outbox_id());

    let report = svc
        .publish_outbox(fixtures::publish_outbox_job_input())
        .await
        .expect("version conflict should stay in report surface");

    assert_eq!(report.scanned_count, 1);
    assert_eq!(report.changed_count, 0);
    assert_eq!(report.failed_refs.len(), 1);
}
