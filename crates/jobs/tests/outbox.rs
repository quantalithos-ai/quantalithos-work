use work_application::{
    BacklogRepository, ProjectMemberRepository, ProjectRepository, WorkItemRepository,
    WorkOutboxPublishService, WorkOutboxRepository,
};
use work_contracts::fixtures;
use work_domain::{Backlog, Project, ProjectMember, WorkItem, WorkOutboxRecord};
use work_infra::{
    clock_id::DeterministicWorkIdGenerator, command_result_store::InMemoryCommandResultRepository,
    idempotency_store::InMemoryIdempotencyRepository, outbox_store::InMemoryWorkOutboxRepository,
    publishers::FakeWorkOutboxPublisher, repositories::InMemoryWorkStores,
};
use work_jobs::WorkOperationsJobRunner;

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

#[tokio::test]
async fn runner_delegates_publish_work_outbox() {
    let svc = service();
    seed_project_family(&svc).await;
    let record = WorkOutboxRecord::from_truth_change(
        fixtures::outbox_id(),
        fixtures::project_created_change(),
        fixtures::trace_context_ref(),
        fixtures::request_metadata(None).requested_at,
    )
    .unwrap();
    WorkOutboxRepository::enqueue(&svc.outbox_repo, record, &seed_uow())
        .await
        .unwrap();
    let stores = InMemoryWorkStores::new();
    let runner = WorkOperationsJobRunner::new(
        svc,
        work_application::WorkDerivedMaintenanceService {
            truth_snapshot_repo: stores.clone(),
            projection_repo: stores.clone(),
            unit_of_work: stores.clone(),
            idempotency: InMemoryIdempotencyRepository::new(),
            job_results: InMemoryCommandResultRepository::new(),
            ids: DeterministicWorkIdGenerator::new(),
        },
        work_application::WorkReferenceRefreshService {
            reference_repo: stores.clone(),
            projection_repo: stores.clone(),
            member_resolver: work_infra::source_resolvers::FakeMemberReferencePort::new(),
            method_resolver:
                work_infra::source_resolvers::FakeMethodDefinitionResolverPort::new(),
            source_resolver: work_infra::source_resolvers::FakeSourceWorkResolverPort::new(),
            evidence_resolver: work_infra::source_resolvers::FakeEvidenceResolverPort::new(),
            process_timebox_resolver:
                work_infra::source_resolvers::FakeProcessTimeboxResolverPort::new(),
            clock: work_infra::clock_id::FixedClock::new(fixtures::request_metadata(None).requested_at),
            unit_of_work: stores.clone(),
            idempotency: InMemoryIdempotencyRepository::new(),
            job_results: InMemoryCommandResultRepository::new(),
            ids: DeterministicWorkIdGenerator::new(),
        },
        work_application::WorkReconciliationService {
            truth_snapshot_repo: stores.clone(),
            projection_repo: stores.clone(),
            outbox_repo: InMemoryWorkOutboxRepository::new(),
            reference_repo: stores,
            unit_of_work: InMemoryWorkStores::new(),
            idempotency: InMemoryIdempotencyRepository::new(),
            job_results: InMemoryCommandResultRepository::new(),
            ids: DeterministicWorkIdGenerator::new(),
        },
    );

    let report = runner
        .run_publish_work_outbox(fixtures::publish_outbox_job_input())
        .await
        .expect("runner publish");

    assert_eq!(report.scanned_count, 1);
    assert_eq!(report.changed_count, 1);
}
