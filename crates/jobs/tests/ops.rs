use work_application::{
    AuditRepository, BacklogRepository, ProjectMemberRepository, ProjectRepository,
    ProjectionRepository, ReferenceSnapshotRepository, WorkItemRepository,
};
use work_contracts::fixtures;
use work_contracts::{
    DerivedFreshnessState, DerivedWorkViewRef, ExternalReferenceRef, FormalWorkRef,
    WorkJobFailureRef,
};
use work_domain::{Backlog, Project, ProjectMember, WorkItem, WorkTraceRecord};
use work_infra::{
    clock_id::{DeterministicWorkIdGenerator, FixedClock},
    command_result_store::InMemoryCommandResultRepository,
    idempotency_store::InMemoryIdempotencyRepository,
    outbox_store::InMemoryWorkOutboxRepository,
    repositories::InMemoryWorkStores,
    source_resolvers::{
        FakeArchiveHandoffPort, FakeEvidenceResolverPort, FakeMemberReferencePort,
        FakeMethodDefinitionResolverPort, FakeProcessTimeboxResolverPort,
        FakeSourceWorkResolverPort, FakeTraceHandoffPort, MemberResolverOutcome,
        MethodDefinitionResolverOutcome,
    },
};
use work_jobs::WorkOperationsJobRunner;

fn seed_uow() -> work_application::UnitOfWorkHandle {
    work_application::UnitOfWorkHandle {
        handle_id: work_application::UnitOfWorkId("seed-uow".to_owned()),
    }
}

fn stores() -> InMemoryWorkStores {
    InMemoryWorkStores::new()
}

fn rebuild_service(
    stores: InMemoryWorkStores,
) -> work_application::WorkDerivedMaintenanceService<
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryIdempotencyRepository,
    InMemoryCommandResultRepository,
    DeterministicWorkIdGenerator,
> {
    work_application::WorkDerivedMaintenanceService {
        truth_snapshot_repo: stores.clone(),
        projection_repo: stores.clone(),
        unit_of_work: stores,
        idempotency: InMemoryIdempotencyRepository::new(),
        job_results: InMemoryCommandResultRepository::new(),
        ids: DeterministicWorkIdGenerator::new(),
    }
}

fn refresh_service(
    stores: InMemoryWorkStores,
) -> (
    work_application::WorkReferenceRefreshService<
        InMemoryWorkStores,
        InMemoryWorkStores,
        FakeMemberReferencePort,
        FakeMethodDefinitionResolverPort,
        FakeSourceWorkResolverPort,
        FakeEvidenceResolverPort,
        FakeProcessTimeboxResolverPort,
        FixedClock,
        InMemoryWorkStores,
        InMemoryIdempotencyRepository,
        InMemoryCommandResultRepository,
        DeterministicWorkIdGenerator,
    >,
    FakeMemberReferencePort,
    FakeMethodDefinitionResolverPort,
    FakeSourceWorkResolverPort,
) {
    let member_resolver = FakeMemberReferencePort::new();
    let method_resolver = FakeMethodDefinitionResolverPort::new();
    let source_resolver = FakeSourceWorkResolverPort::new();
    let service = work_application::WorkReferenceRefreshService {
        reference_repo: stores.clone(),
        projection_repo: stores.clone(),
        member_resolver: member_resolver.clone(),
        method_resolver: method_resolver.clone(),
        source_resolver: source_resolver.clone(),
        evidence_resolver: FakeEvidenceResolverPort::new(),
        process_timebox_resolver: FakeProcessTimeboxResolverPort::new(),
        clock: FixedClock::new(fixtures::request_metadata(None).requested_at),
        unit_of_work: stores,
        idempotency: InMemoryIdempotencyRepository::new(),
        job_results: InMemoryCommandResultRepository::new(),
        ids: DeterministicWorkIdGenerator::new(),
    };
    (service, member_resolver, method_resolver, source_resolver)
}

fn reconciliation_service(
    stores: InMemoryWorkStores,
) -> work_application::WorkReconciliationService<
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkOutboxRepository,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryIdempotencyRepository,
    InMemoryCommandResultRepository,
    DeterministicWorkIdGenerator,
> {
    work_application::WorkReconciliationService {
        truth_snapshot_repo: stores.clone(),
        projection_repo: stores.clone(),
        outbox_repo: InMemoryWorkOutboxRepository::new(),
        reference_repo: stores.clone(),
        unit_of_work: stores,
        idempotency: InMemoryIdempotencyRepository::new(),
        job_results: InMemoryCommandResultRepository::new(),
        ids: DeterministicWorkIdGenerator::new(),
    }
}

fn trace_handoff_service(
    stores: InMemoryWorkStores,
) -> (
    work_application::WorkTraceHandoffService<
        InMemoryWorkStores,
        InMemoryWorkOutboxRepository,
        FakeTraceHandoffPort,
        FixedClock,
        InMemoryWorkStores,
        InMemoryIdempotencyRepository,
        InMemoryCommandResultRepository,
        DeterministicWorkIdGenerator,
    >,
    FakeTraceHandoffPort,
) {
    let handoff = FakeTraceHandoffPort::new();
    let service = work_application::WorkTraceHandoffService {
        audit_repo: stores.clone(),
        outbox_repo: InMemoryWorkOutboxRepository::new(),
        trace_handoff: handoff.clone(),
        clock: FixedClock::new(fixtures::request_metadata(None).requested_at),
        unit_of_work: stores,
        idempotency: InMemoryIdempotencyRepository::new(),
        job_results: InMemoryCommandResultRepository::new(),
        ids: DeterministicWorkIdGenerator::new(),
    };
    (service, handoff)
}

fn archive_handoff_service(
    stores: InMemoryWorkStores,
) -> (
    work_application::WorkArchiveHandoffService<
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkOutboxRepository,
        FakeArchiveHandoffPort,
        FixedClock,
        InMemoryWorkStores,
        InMemoryIdempotencyRepository,
        InMemoryCommandResultRepository,
        DeterministicWorkIdGenerator,
    >,
    FakeArchiveHandoffPort,
) {
    let handoff = FakeArchiveHandoffPort::new();
    let service = work_application::WorkArchiveHandoffService {
        archive_summary_repo: stores.clone(),
        audit_repo: stores.clone(),
        outbox_repo: InMemoryWorkOutboxRepository::new(),
        archive_handoff: handoff.clone(),
        clock: FixedClock::new(fixtures::request_metadata(None).requested_at),
        unit_of_work: stores,
        idempotency: InMemoryIdempotencyRepository::new(),
        job_results: InMemoryCommandResultRepository::new(),
        ids: DeterministicWorkIdGenerator::new(),
    };
    (service, handoff)
}

async fn seed_project_family(stores: &InMemoryWorkStores) {
    let uow = seed_uow();
    let actor = fixtures::actor_context().actor_ref().clone();
    let project = Project::create(
        fixtures::project_id(),
        fixtures::project_spec(),
        actor.clone(),
    )
    .unwrap();
    ProjectRepository::create(stores, project, &uow)
        .await
        .unwrap();

    let backlog = Backlog::open_for_project(
        fixtures::backlog_id(),
        fixtures::project_id(),
        actor.clone(),
    )
    .unwrap();
    BacklogRepository::create(stores, backlog, &uow)
        .await
        .unwrap();

    let member = ProjectMember::assign(
        fixtures::project_member_id(),
        fixtures::project_id(),
        fixtures::global_member_ref(),
        fixtures::responsibility_spec(),
    )
    .unwrap();
    ProjectMemberRepository::create(stores, member, &uow)
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
    WorkItemRepository::create_work_item(stores, work, &uow)
        .await
        .unwrap();
}

async fn seed_trace_record(stores: &InMemoryWorkStores) {
    let uow = seed_uow();
    let trace = WorkTraceRecord::from_truth_change(
        fixtures::trace_id(),
        fixtures::project_created_change(),
        fixtures::trace_context_ref(),
    )
    .expect("trace");
    AuditRepository::append_trace(stores, trace, &uow)
        .await
        .expect("seed trace");
}

#[tokio::test]
async fn refresh_job_marks_repository_returned_affected_views_stale() {
    let stores = stores();
    seed_project_family(&stores).await;
    let (refresh, member_resolver, _method_resolver, _source_resolver) =
        refresh_service(stores.clone());
    member_resolver.seed(
        fixtures::global_member_ref(),
        MemberResolverOutcome::Success(fixtures::capability_ref_set()),
    );
    stores.seed_affected_views_for_reference(
        &ExternalReferenceRef::from_member(fixtures::global_member_ref()),
        vec![DerivedWorkViewRef::member_work(
            fixtures::project_member_ref(),
        )],
    );
    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh,
        reconciliation_service(stores.clone()),
        trace_handoff_service(stores.clone()).0,
        archive_handoff_service(stores.clone()).0,
    );
    let input = work_contracts::RefreshExternalReferenceSnapshotsJobInput {
        metadata: fixtures::job_metadata("job-refresh-existing-views"),
        reference_scope: Some(work_contracts::ExternalReferenceScope {
            scope_kind: work_contracts::ExternalReferenceScopeKind::ExplicitRefs,
            project_ref: None,
            reference_refs: vec![ExternalReferenceRef::from_member(
                fixtures::global_member_ref(),
            )],
        }),
        page: core_contracts::metadata::PageRequest {
            limit: 10,
            page_token: None,
        },
    };

    let report = runner
        .run_refresh_external_reference_snapshots(input)
        .await
        .expect("refresh succeeds");

    assert_eq!(report.scanned_count, 1);
    assert_eq!(report.changed_count, 1);
    assert!(report.failed_refs.is_empty());
    assert_eq!(stores.stale_marks().len(), 1);
}

#[tokio::test]
async fn refresh_job_with_empty_affected_view_page_writes_no_stale_marker() {
    let stores = stores();
    seed_project_family(&stores).await;
    let (refresh, member_resolver, _method_resolver, _source_resolver) =
        refresh_service(stores.clone());
    member_resolver.seed(
        fixtures::global_member_ref(),
        MemberResolverOutcome::Success(fixtures::capability_ref_set()),
    );
    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh,
        reconciliation_service(stores.clone()),
        trace_handoff_service(stores.clone()).0,
        archive_handoff_service(stores.clone()).0,
    );
    let input = work_contracts::RefreshExternalReferenceSnapshotsJobInput {
        metadata: fixtures::job_metadata("job-refresh-empty-views"),
        reference_scope: Some(work_contracts::ExternalReferenceScope {
            scope_kind: work_contracts::ExternalReferenceScopeKind::ExplicitRefs,
            project_ref: None,
            reference_refs: vec![ExternalReferenceRef::from_member(
                fixtures::global_member_ref(),
            )],
        }),
        page: core_contracts::metadata::PageRequest {
            limit: 10,
            page_token: None,
        },
    };

    let report = runner
        .run_refresh_external_reference_snapshots(input)
        .await
        .expect("refresh succeeds");

    assert_eq!(report.changed_count, 1);
    assert!(stores.stale_marks().is_empty());
}

#[tokio::test]
async fn refresh_job_existing_reference_and_snapshots_use_versioned_reads() {
    let stores = stores();
    stores.set_strict_reference_versions(true);
    seed_project_family(&stores).await;
    let (refresh, member_resolver, method_resolver, _source_resolver) =
        refresh_service(stores.clone());

    member_resolver.seed(
        fixtures::global_member_ref(),
        MemberResolverOutcome::Success(fixtures::capability_ref_set()),
    );
    method_resolver.seed(
        fixtures::method_definition_ref(),
        MethodDefinitionResolverOutcome::Success(work_contracts::MethodDefinitionKind::Task),
    );

    let uow = seed_uow();
    let mut member_snapshot = work_domain::MemberCapabilitySnapshot::from_identity(
        fixtures::global_member_ref(),
        fixtures::capability_ref_set(),
    )
    .expect("member snapshot");
    member_snapshot
        .snapshot_state
        .mark_resolved(fixtures::request_metadata(None).requested_at)
        .expect("member snapshot resolved");
    stores
        .save_member_snapshot(member_snapshot, None, &uow)
        .await
        .expect("seed member snapshot");

    let mut method_snapshot = work_domain::MethodDefinitionSnapshot::from_method_library(
        fixtures::method_definition_ref(),
        work_contracts::MethodDefinitionKind::Task,
    )
    .expect("method snapshot");
    method_snapshot
        .snapshot_state
        .mark_resolved(fixtures::request_metadata(None).requested_at)
        .expect("method snapshot resolved");
    stores
        .save_method_snapshot(method_snapshot, None, &uow)
        .await
        .expect("seed method snapshot");

    stores
        .save_reference_state(
            work_domain::ReferenceResolutionState::resolved(ExternalReferenceRef::from_member(
                fixtures::global_member_ref(),
            )),
            None,
            &uow,
        )
        .await
        .expect("seed member reference state");
    stores
        .save_reference_state(
            work_domain::ReferenceResolutionState::resolved(
                ExternalReferenceRef::from_method_definition(fixtures::method_definition_ref()),
            ),
            None,
            &uow,
        )
        .await
        .expect("seed method reference state");

    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh,
        reconciliation_service(stores.clone()),
        trace_handoff_service(stores.clone()).0,
        archive_handoff_service(stores.clone()).0,
    );
    let input = work_contracts::RefreshExternalReferenceSnapshotsJobInput {
        metadata: fixtures::job_metadata("job-refresh-versioned-update"),
        reference_scope: Some(work_contracts::ExternalReferenceScope {
            scope_kind: work_contracts::ExternalReferenceScopeKind::ExplicitRefs,
            project_ref: None,
            reference_refs: vec![
                ExternalReferenceRef::from_member(fixtures::global_member_ref()),
                ExternalReferenceRef::from_method_definition(fixtures::method_definition_ref()),
            ],
        }),
        page: core_contracts::metadata::PageRequest {
            limit: 10,
            page_token: None,
        },
    };

    let report = runner
        .run_refresh_external_reference_snapshots(input)
        .await
        .expect("refresh succeeds with existing versioned records");

    assert_eq!(report.changed_count, 2);
    assert!(report.failed_refs.is_empty());
    assert_eq!(
        stores
            .member_snapshot(&fixtures::global_member_ref())
            .expect("member snapshot persisted")
            .1,
        2
    );
    assert_eq!(
        stores
            .method_snapshot(&fixtures::method_definition_ref())
            .expect("method snapshot persisted")
            .1,
        2
    );
    assert_eq!(
        stores
            .reference_state_snapshot(&ExternalReferenceRef::from_member(
                fixtures::global_member_ref()
            ))
            .expect("member ref state persisted")
            .1,
        2
    );
    assert_eq!(
        stores
            .reference_state_snapshot(&ExternalReferenceRef::from_method_definition(
                fixtures::method_definition_ref()
            ))
            .expect("method ref state persisted")
            .1,
        2
    );
}

#[tokio::test]
async fn refresh_job_duplicate_returns_stored_report() {
    let stores = stores();
    seed_project_family(&stores).await;
    let (refresh, _member_resolver, method_resolver, _source_resolver) =
        refresh_service(stores.clone());
    method_resolver.seed(
        fixtures::method_definition_ref(),
        MethodDefinitionResolverOutcome::Success(work_contracts::MethodDefinitionKind::Task),
    );
    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh,
        reconciliation_service(stores.clone()),
        trace_handoff_service(stores.clone()).0,
        archive_handoff_service(stores.clone()).0,
    );
    let input = work_contracts::RefreshExternalReferenceSnapshotsJobInput {
        metadata: fixtures::job_metadata("job-refresh-duplicate"),
        reference_scope: Some(work_contracts::ExternalReferenceScope {
            scope_kind: work_contracts::ExternalReferenceScopeKind::ExplicitRefs,
            project_ref: None,
            reference_refs: vec![ExternalReferenceRef::from_method_definition(
                fixtures::method_definition_ref(),
            )],
        }),
        page: core_contracts::metadata::PageRequest {
            limit: 10,
            page_token: None,
        },
    };

    let first = runner
        .run_refresh_external_reference_snapshots(input.clone())
        .await
        .expect("first refresh");
    let second = runner
        .run_refresh_external_reference_snapshots(input)
        .await
        .expect("duplicate refresh");

    assert_eq!(first.scanned_count, second.scanned_count);
    assert_eq!(first.changed_count, second.changed_count);
    assert_ne!(first.receipt, second.receipt);
}

#[tokio::test]
async fn reconciliation_job_is_read_only() {
    let stores = stores();
    seed_project_family(&stores).await;
    stores.seed_project_board_public_view(work_contracts::ProjectBoardView {
        project_ref: fixtures::project_ref(),
        work_cards: Vec::new(),
        marker: work_contracts::ProjectionViewMarker {
            view_ref: DerivedWorkViewRef::project_board(fixtures::project_ref()),
            source_cursor: fixtures::truth_cursor(),
            freshness_state: DerivedFreshnessState::Failed,
        },
    });
    let before_project = stores.project_snapshot(&fixtures::project_ref());
    let before_stale_count = stores.stale_marks().len();
    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh_service(stores.clone()).0,
        reconciliation_service(stores.clone()),
        trace_handoff_service(stores.clone()).0,
        archive_handoff_service(stores.clone()).0,
    );

    let report = runner
        .run_work_reconciliation(fixtures::reconciliation_job_input())
        .await
        .expect("reconcile succeeds");

    assert_eq!(
        before_project,
        stores.project_snapshot(&fixtures::project_ref())
    );
    assert_eq!(before_stale_count, stores.stale_marks().len());
    assert_eq!(report.projection_gaps.len(), 1);
}

#[tokio::test]
async fn rebuild_job_replaces_projection_from_committed_truth() {
    let stores = stores();
    seed_project_family(&stores).await;
    stores.seed_search_rows(
        &fixtures::project_ref(),
        vec![work_contracts::WorkSearchProjection {
            project_ref: fixtures::project_ref(),
            work_ref: FormalWorkRef::WorkItem(work_contracts::WorkItemId("old".to_owned())),
            title: work_contracts::WorkTitle("old".to_owned()),
            work_state: work_contracts::WorkItemState::Cancelled,
            assignee_ref: None,
            source_kind: None,
            source_cursor: fixtures::truth_cursor(),
        }],
    );
    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh_service(stores.clone()).0,
        reconciliation_service(stores.clone()),
        trace_handoff_service(stores.clone()).0,
        archive_handoff_service(stores.clone()).0,
    );

    let report = runner
        .run_rebuild_work_projections(fixtures::rebuild_projections_job_input())
        .await
        .expect("rebuild succeeds");

    let search_rows = ProjectionRepository::search_work(
        &stores,
        fixtures::project_ref(),
        fixtures::work_search_criteria(),
        core_contracts::metadata::PageRequest {
            limit: 50,
            page_token: None,
        },
    )
    .await
    .expect("search rows");
    assert_eq!(report.changed_count, 2);
    assert!(
        search_rows
            .items
            .iter()
            .any(|row| row.title == fixtures::formal_work_intent().title)
    );
}

fn outbox_service(
    stores: InMemoryWorkStores,
) -> work_application::WorkOutboxPublishService<
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
    work_infra::publishers::FakeWorkOutboxPublisher,
    InMemoryWorkStores,
    InMemoryIdempotencyRepository,
    InMemoryCommandResultRepository,
    DeterministicWorkIdGenerator,
> {
    work_application::WorkOutboxPublishService {
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
        publisher: work_infra::publishers::FakeWorkOutboxPublisher::new(),
        unit_of_work: stores,
        idempotency: InMemoryIdempotencyRepository::new(),
        job_results: InMemoryCommandResultRepository::new(),
        ids: DeterministicWorkIdGenerator::new(),
    }
}

#[tokio::test]
async fn trace_handoff_job_saves_marker_and_replays_duplicate() {
    let stores = stores();
    seed_trace_record(&stores).await;
    let (trace_handoff, handoff_port) = trace_handoff_service(stores.clone());
    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh_service(stores.clone()).0,
        reconciliation_service(stores.clone()),
        trace_handoff,
        archive_handoff_service(stores.clone()).0,
    );

    let first = runner
        .run_prepare_work_trace_handoff(fixtures::trace_handoff_job_input())
        .await
        .expect("trace handoff succeeds");
    let second = runner
        .run_prepare_work_trace_handoff(fixtures::trace_handoff_job_input())
        .await
        .expect("duplicate trace handoff");

    assert_eq!(first.scanned_count, 1);
    assert_eq!(first.changed_count, 1);
    assert!(first.failed_refs.is_empty());
    assert_eq!(handoff_port.intents().len(), 1);
    assert_eq!(second.failed_refs, first.failed_refs);
    assert_eq!(
        second.receipt.expect("duplicate receipt").idempotency,
        work_contracts::IdempotencyResultView::Duplicate
    );
}

#[tokio::test]
async fn trace_handoff_job_reports_typed_failure_without_body() {
    let stores = stores();
    seed_trace_record(&stores).await;
    let (trace_handoff, handoff_port) = trace_handoff_service(stores.clone());
    handoff_port.push_result(Err(work_application::PortError::Unavailable));
    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh_service(stores.clone()).0,
        reconciliation_service(stores.clone()),
        trace_handoff,
        archive_handoff_service(stores.clone()).0,
    );

    let report = runner
        .run_prepare_work_trace_handoff(fixtures::trace_handoff_job_input())
        .await
        .expect("trace handoff failure stays in report");

    assert_eq!(report.scanned_count, 1);
    assert_eq!(report.changed_count, 0);
    assert_eq!(
        report.failed_refs,
        vec![WorkJobFailureRef::TraceHandoff {
            trace_id: fixtures::trace_id(),
            subject_ref: fixtures::project_trace_subject(),
            target_ref: fixtures::trace_handoff_target_ref(),
        }]
    );
}

#[tokio::test]
async fn archive_handoff_job_saves_marker() {
    let stores = stores();
    seed_project_family(&stores).await;
    seed_trace_record(&stores).await;
    let (archive_handoff, handoff_port) = archive_handoff_service(stores.clone());
    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh_service(stores.clone()).0,
        reconciliation_service(stores.clone()),
        trace_handoff_service(stores.clone()).0,
        archive_handoff,
    );

    let report = runner
        .run_prepare_archive_handoff(fixtures::archive_handoff_job_input())
        .await
        .expect("archive handoff succeeds");

    assert_eq!(report.changed_count, 1);
    assert!(report.failed_refs.is_empty());
    assert_eq!(handoff_port.intents().len(), 1);
}

#[tokio::test]
async fn archive_handoff_job_reports_typed_failure_and_replays_duplicate() {
    let stores = stores();
    seed_project_family(&stores).await;
    seed_trace_record(&stores).await;
    let (archive_handoff, handoff_port) = archive_handoff_service(stores.clone());
    handoff_port.push_result(Err(work_application::PortError::Unavailable));
    let runner = WorkOperationsJobRunner::new(
        outbox_service(stores.clone()),
        rebuild_service(stores.clone()),
        refresh_service(stores.clone()).0,
        reconciliation_service(stores.clone()),
        trace_handoff_service(stores.clone()).0,
        archive_handoff,
    );

    let first = runner
        .run_prepare_archive_handoff(fixtures::archive_handoff_job_input())
        .await
        .expect("archive failure stays in report");
    let second = runner
        .run_prepare_archive_handoff(fixtures::archive_handoff_job_input())
        .await
        .expect("archive duplicate replay");

    assert_eq!(
        first.failed_refs,
        vec![WorkJobFailureRef::ArchiveHandoff {
            archive_scope: fixtures::archive_handoff_scope(),
            target_ref: fixtures::archive_handoff_target_ref(),
        }]
    );
    assert_eq!(second.failed_refs, first.failed_refs);
    assert_eq!(
        second.receipt.expect("duplicate receipt").idempotency,
        work_contracts::IdempotencyResultView::Duplicate
    );
}
