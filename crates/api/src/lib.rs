//! API entrypoints for the Work bounded context.

use work_application::{
    ApplicationError, ProjectCommandService, ProjectMemberCommandService, WorkItemCommandService,
};
use work_contracts::{
    AssignProjectMemberRequest, BacklogCommandResult, CreateChildWorkItemRequest,
    CreateProjectRequest, CreateWorkItemRequest, ProjectCommandResult, ProjectMemberCommandResult,
    UpdateBacklogAvailabilityRequest, UpdateProjectLifecycleRequest,
    UpdateProjectMemberResponsibilityRequest, UpdateWorkItemLifecycleRequest, WorkCommandEnvelope,
    WorkItemCommandResult, WorkProtocolError,
};

/// Thin command handlers that validate protocol shape and delegate to application services.
pub struct WorkCommandHandlers<P, M, W> {
    /// Project-scoped command service.
    pub project_service: P,
    /// Project-member command service.
    pub member_service: M,
    /// Formal work command service.
    pub workitem_service: W,
}

impl<P, M, W> WorkCommandHandlers<P, M, W> {
    /// Creates a handler set for command delegation.
    pub fn new(project_service: P, member_service: M, workitem_service: W) -> Self {
        Self {
            project_service,
            member_service,
            workitem_service,
        }
    }
}

impl<
    P,
    B,
    A,
    O,
    R,
    PR,
    U,
    I,
    C,
    IDEM,
    MP,
    MPM,
    MRS,
    MA,
    MO,
    MR,
    MPR,
    MU,
    MM,
    MI,
    MC,
    MIDEM,
    WP,
    WB,
    WPM,
    WW,
    WA,
    WO,
    WR,
    WPR,
    WU,
    WS,
    WE,
    WI,
    WC,
    WIDEM,
>
    WorkCommandHandlers<
        ProjectCommandService<P, B, A, O, R, PR, U, I, C, IDEM>,
        ProjectMemberCommandService<MP, MPM, MRS, MA, MO, MR, MPR, MU, MM, MI, MC, MIDEM>,
        WorkItemCommandService<WP, WB, WPM, WW, WA, WO, WR, WPR, WU, WS, WE, WI, WC, WIDEM>,
    >
where
    P: work_application::ProjectRepository,
    B: work_application::BacklogRepository,
    A: work_application::AuditRepository,
    O: work_application::WorkOutboxRepository,
    R: work_application::CommandResultRepository,
    PR: work_application::ProjectionRepository,
    U: work_application::UnitOfWork,
    I: work_application::IdGeneratorPort,
    C: work_application::ClockPort,
    IDEM: work_application::IdempotencyRepository,
    MP: work_application::ProjectRepository,
    MPM: work_application::ProjectMemberRepository,
    MRS: work_application::ReferenceSnapshotRepository,
    MA: work_application::AuditRepository,
    MO: work_application::WorkOutboxRepository,
    MR: work_application::CommandResultRepository,
    MPR: work_application::ProjectionRepository,
    MU: work_application::UnitOfWork,
    MM: work_application::MemberReferencePort,
    MI: work_application::IdGeneratorPort,
    MC: work_application::ClockPort,
    MIDEM: work_application::IdempotencyRepository,
    WP: work_application::ProjectRepository,
    WB: work_application::BacklogRepository,
    WPM: work_application::ProjectMemberRepository,
    WW: work_application::WorkItemRepository,
    WA: work_application::AuditRepository,
    WO: work_application::WorkOutboxRepository,
    WR: work_application::CommandResultRepository,
    WPR: work_application::ProjectionRepository,
    WU: work_application::UnitOfWork,
    WS: work_application::SourceWorkResolverPort,
    WE: work_application::EvidenceResolverPort,
    WI: work_application::IdGeneratorPort,
    WC: work_application::ClockPort,
    WIDEM: work_application::IdempotencyRepository,
{
    /// Handles `CreateProject`.
    pub async fn handle_create_project(
        &self,
        envelope: WorkCommandEnvelope<CreateProjectRequest>,
    ) -> Result<ProjectCommandResult, WorkProtocolError> {
        self.project_service
            .create_project(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `UpdateProjectLifecycle`.
    pub async fn handle_update_project_lifecycle(
        &self,
        envelope: WorkCommandEnvelope<UpdateProjectLifecycleRequest>,
    ) -> Result<ProjectCommandResult, WorkProtocolError> {
        self.project_service
            .update_lifecycle(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `UpdateBacklogAvailability`.
    pub async fn handle_update_backlog_availability(
        &self,
        envelope: WorkCommandEnvelope<UpdateBacklogAvailabilityRequest>,
    ) -> Result<BacklogCommandResult, WorkProtocolError> {
        self.project_service
            .update_backlog_availability(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `AssignProjectMember`.
    pub async fn handle_assign_project_member(
        &self,
        envelope: WorkCommandEnvelope<AssignProjectMemberRequest>,
    ) -> Result<ProjectMemberCommandResult, WorkProtocolError> {
        self.member_service
            .assign_project_member(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `UpdateProjectMemberResponsibility`.
    pub async fn handle_update_project_member_responsibility(
        &self,
        envelope: WorkCommandEnvelope<UpdateProjectMemberResponsibilityRequest>,
    ) -> Result<ProjectMemberCommandResult, WorkProtocolError> {
        self.member_service
            .update_project_member_responsibility(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `CreateWorkItem`.
    pub async fn handle_create_work_item(
        &self,
        envelope: WorkCommandEnvelope<CreateWorkItemRequest>,
    ) -> Result<WorkItemCommandResult, WorkProtocolError> {
        self.workitem_service
            .create_work_item(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `CreateChildWorkItem`.
    pub async fn handle_create_child_work_item(
        &self,
        envelope: WorkCommandEnvelope<CreateChildWorkItemRequest>,
    ) -> Result<WorkItemCommandResult, WorkProtocolError> {
        self.workitem_service
            .create_child_work_item(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `UpdateWorkItemLifecycle`.
    pub async fn handle_update_work_item_lifecycle(
        &self,
        envelope: WorkCommandEnvelope<UpdateWorkItemLifecycleRequest>,
    ) -> Result<WorkItemCommandResult, WorkProtocolError> {
        self.workitem_service
            .update_lifecycle(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }
}

#[cfg(test)]
mod tests {
    use core_contracts::metadata::Timestamp;

    use super::WorkCommandHandlers;
    use work_application::{
        BacklogRepository, CommandResultRepository, ProjectCommandService,
        ProjectMemberCommandService, WorkItemCommandService,
    };
    use work_contracts::metadata::fixtures;
    use work_contracts::{
        AssignProjectMemberRequest, BacklogAvailabilityTarget, BacklogState,
        CreateChildWorkItemRequest, CreateProjectRequest, CreateWorkItemRequest,
        IdempotencyResultView, ProjectLifecycleReason, ProjectLifecycleReasonKind,
        ProjectLifecycleState, ProjectLifecycleTarget, ProjectMemberReason,
        ProjectMemberReasonKind, ProjectMemberResponsibilityState, ResponsibilityTarget,
        UpdateBacklogAvailabilityRequest, UpdateProjectLifecycleRequest,
        UpdateProjectMemberResponsibilityRequest, UpdateWorkItemLifecycleRequest,
        WorkCommandEnvelope, WorkItemState, WorkProtocolError,
    };
    use work_infra::clock_id::{DeterministicWorkIdGenerator, FixedClock};
    use work_infra::command_result_store::InMemoryCommandResultRepository;
    use work_infra::idempotency_store::InMemoryIdempotencyRepository;
    use work_infra::outbox_store::InMemoryWorkOutboxRepository;
    use work_infra::repositories::InMemoryWorkStores;
    use work_infra::source_resolvers::{
        EvidenceResolverOutcome, FakeEvidenceResolverPort, FakeMemberReferencePort,
        FakeSourceWorkResolverPort, MemberResolverOutcome, SourceResolverOutcome,
    };

    type TestHandlers = WorkCommandHandlers<
        ProjectCommandService<
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkOutboxRepository,
            InMemoryCommandResultRepository,
            InMemoryWorkStores,
            InMemoryWorkStores,
            DeterministicWorkIdGenerator,
            FixedClock,
            InMemoryIdempotencyRepository,
        >,
        ProjectMemberCommandService<
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkOutboxRepository,
            InMemoryCommandResultRepository,
            InMemoryWorkStores,
            InMemoryWorkStores,
            FakeMemberReferencePort,
            DeterministicWorkIdGenerator,
            FixedClock,
            InMemoryIdempotencyRepository,
        >,
        WorkItemCommandService<
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkOutboxRepository,
            InMemoryCommandResultRepository,
            InMemoryWorkStores,
            InMemoryWorkStores,
            FakeSourceWorkResolverPort,
            FakeEvidenceResolverPort,
            DeterministicWorkIdGenerator,
            FixedClock,
            InMemoryIdempotencyRepository,
        >,
    >;

    fn build_handlers() -> (
        TestHandlers,
        InMemoryWorkStores,
        InMemoryWorkOutboxRepository,
        InMemoryCommandResultRepository,
        InMemoryIdempotencyRepository,
        FakeMemberReferencePort,
        FakeSourceWorkResolverPort,
        FakeEvidenceResolverPort,
    ) {
        let stores = InMemoryWorkStores::new();
        let outbox = InMemoryWorkOutboxRepository::new();
        let results = InMemoryCommandResultRepository::new();
        let idempotency = InMemoryIdempotencyRepository::new();
        let member_refs = FakeMemberReferencePort::new();
        let source_refs = FakeSourceWorkResolverPort::new();
        let evidence_refs = FakeEvidenceResolverPort::new();
        let ids = DeterministicWorkIdGenerator::new();
        let clock = FixedClock::new(Timestamp::new("2026-06-05T09:00:00Z"));
        let project_service = ProjectCommandService {
            project_repo: stores.clone(),
            backlog_repo: stores.clone(),
            audit_repo: stores.clone(),
            outbox_repo: outbox.clone(),
            command_results: results.clone(),
            projection_repo: stores.clone(),
            unit_of_work: stores.clone(),
            ids: ids.clone(),
            clock: clock.clone(),
            idempotency: idempotency.clone(),
        };
        let member_service = ProjectMemberCommandService {
            project_repo: stores.clone(),
            member_repo: stores.clone(),
            snapshot_repo: stores.clone(),
            audit_repo: stores.clone(),
            outbox_repo: outbox.clone(),
            command_results: results.clone(),
            projection_repo: stores.clone(),
            unit_of_work: stores.clone(),
            member_refs: member_refs.clone(),
            ids: ids.clone(),
            clock: clock.clone(),
            idempotency: idempotency.clone(),
        };
        let workitem_service = WorkItemCommandService {
            project_repo: stores.clone(),
            backlog_repo: stores.clone(),
            member_repo: stores.clone(),
            work_repo: stores.clone(),
            audit_repo: stores.clone(),
            outbox_repo: outbox.clone(),
            command_results: results.clone(),
            projection_repo: stores.clone(),
            unit_of_work: stores.clone(),
            source_resolver: source_refs.clone(),
            evidence_resolver: evidence_refs.clone(),
            ids,
            clock,
            idempotency: idempotency.clone(),
        };
        (
            WorkCommandHandlers::new(project_service, member_service, workitem_service),
            stores,
            outbox,
            results,
            idempotency,
            member_refs,
            source_refs,
            evidence_refs,
        )
    }

    async fn create_project(
        handlers: &TestHandlers,
        key: &str,
    ) -> work_contracts::ProjectCommandResult {
        handlers
            .handle_create_project(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata(key),
                command: CreateProjectRequest {
                    project_spec: fixtures::project_spec(),
                },
            })
            .await
            .expect("create_project should succeed")
    }

    async fn assign_member(
        handlers: &TestHandlers,
        project_ref: work_contracts::ProjectRef,
        key: &str,
    ) -> Result<work_contracts::ProjectMemberCommandResult, WorkProtocolError> {
        handlers
            .handle_assign_project_member(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata(key),
                command: AssignProjectMemberRequest {
                    project_ref,
                    member_ref: fixtures::global_member_ref(),
                    responsibility_spec: fixtures::responsibility_spec(),
                },
            })
            .await
    }

    async fn prepare_formal_work_context(
        handlers: &TestHandlers,
        member_refs: &FakeMemberReferencePort,
    ) -> (
        work_contracts::ProjectCommandResult,
        work_contracts::ProjectMemberCommandResult,
    ) {
        let created = create_project(handlers, "idem-formal-project").await;
        member_refs.seed(
            fixtures::global_member_ref(),
            MemberResolverOutcome::Success(fixtures::capability_ref_set()),
        );
        let assigned = assign_member(handlers, created.project_ref.clone(), "idem-formal-member")
            .await
            .expect("assign should succeed");
        (created, assigned)
    }

    #[tokio::test]
    async fn tc_work_member_001_assign_project_member_persists_member_snapshot_and_side_effects() {
        let (
            handlers,
            stores,
            outbox,
            results,
            _idempotency,
            member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let created = create_project(&handlers, "idem-member-001-project").await;
        member_refs.seed(
            fixtures::global_member_ref(),
            MemberResolverOutcome::Success(fixtures::capability_ref_set()),
        );

        let result = assign_member(&handlers, created.project_ref.clone(), "idem-member-001")
            .await
            .expect("assign should succeed");

        assert_eq!(
            result.responsibility_state,
            ProjectMemberResponsibilityState::Active
        );
        assert_eq!(result.receipt.idempotency, IdempotencyResultView::Applied);
        assert_eq!(result.receipt.outbox_record_refs.len(), 1);
        assert_eq!(stores.trace_count(), 2);
        assert_eq!(stores.stale_mark_count(), 2);
        assert_eq!(outbox.count(), 2);
        assert!(
            results
                .get_result(result.receipt.result_ref.clone())
                .await
                .expect("stored result read should succeed")
                .is_some()
        );

        let (member, version) = stores
            .project_member_snapshot(&result.project_member_ref)
            .expect("member should be stored");
        assert_eq!(member.member_ref, fixtures::global_member_ref());
        assert_eq!(version, 1);
        let (snapshot, snapshot_version) = stores
            .member_snapshot(&fixtures::global_member_ref())
            .expect("member snapshot should be stored");
        assert_eq!(snapshot.member_ref, fixtures::global_member_ref());
        assert_eq!(snapshot_version, 1);
    }

    #[tokio::test]
    async fn tc_work_member_002_identity_resolver_unresolved_or_unavailable_does_not_save_truth() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let created = create_project(&handlers, "idem-member-002-project").await;
        member_refs.seed(
            fixtures::global_member_ref(),
            MemberResolverOutcome::Unresolved,
        );

        let unresolved = assign_member(
            &handlers,
            created.project_ref.clone(),
            "idem-member-002-unresolved",
        )
        .await
        .expect_err("unresolved member should fail");
        assert_eq!(unresolved, WorkProtocolError::ExternalReferenceUnresolved);
        assert!(
            stores
                .member_snapshot(&fixtures::global_member_ref())
                .is_none()
        );
        assert_eq!(stores.trace_count(), 1);
        assert_eq!(stores.stale_mark_count(), 1);
        assert_eq!(outbox.count(), 1);

        member_refs.seed(
            fixtures::global_member_ref(),
            MemberResolverOutcome::Unavailable,
        );
        let unavailable = assign_member(
            &handlers,
            created.project_ref.clone(),
            "idem-member-002-unavailable",
        )
        .await
        .expect_err("unavailable member should fail");
        assert_eq!(unavailable, WorkProtocolError::TemporarilyUnavailable);
        assert!(
            stores
                .member_snapshot(&fixtures::global_member_ref())
                .is_none()
        );
        assert_eq!(stores.trace_count(), 1);
        assert_eq!(stores.stale_mark_count(), 1);
        assert_eq!(outbox.count(), 1);
    }

    #[tokio::test]
    async fn tc_work_member_003_body_leak_rejects_identity_truth_takeover() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let created = create_project(&handlers, "idem-member-003-project").await;
        member_refs.seed(
            fixtures::global_member_ref(),
            MemberResolverOutcome::BodyLeak,
        );

        let error = assign_member(&handlers, created.project_ref, "idem-member-003")
            .await
            .expect_err("body leak should be rejected");

        assert_eq!(error, WorkProtocolError::DomainRejected);
        assert!(
            stores
                .member_snapshot(&fixtures::global_member_ref())
                .is_none()
        );
        assert_eq!(stores.trace_count(), 1);
        assert_eq!(stores.stale_mark_count(), 1);
        assert_eq!(outbox.count(), 1);
    }

    #[tokio::test]
    async fn tc_work_member_004_released_member_cannot_return_to_active() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let created = create_project(&handlers, "idem-member-004-project").await;
        member_refs.seed(
            fixtures::global_member_ref(),
            MemberResolverOutcome::Success(fixtures::capability_ref_set()),
        );
        let assigned = assign_member(
            &handlers,
            created.project_ref.clone(),
            "idem-member-004-assign",
        )
        .await
        .expect("assign should succeed");

        let released = handlers
            .handle_update_project_member_responsibility(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-member-004-release"),
                command: UpdateProjectMemberResponsibilityRequest {
                    project_member_ref: assigned.project_member_ref.clone(),
                    target: ResponsibilityTarget::Released,
                    reason: ProjectMemberReason {
                        reason_kind: ProjectMemberReasonKind::Released,
                        reason_ref: None,
                    },
                    expected_version: 1,
                },
            })
            .await
            .expect("release should succeed");
        assert_eq!(
            released.responsibility_state,
            ProjectMemberResponsibilityState::Released
        );

        let error = handlers
            .handle_update_project_member_responsibility(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-member-004-reactivate"),
                command: UpdateProjectMemberResponsibilityRequest {
                    project_member_ref: assigned.project_member_ref.clone(),
                    target: ResponsibilityTarget::Active,
                    reason: ProjectMemberReason {
                        reason_kind: ProjectMemberReasonKind::Assigned,
                        reason_ref: None,
                    },
                    expected_version: 2,
                },
            })
            .await
            .expect_err("released member should remain terminal");

        assert_eq!(error, WorkProtocolError::DomainRejected);
        let (_member, version) = stores
            .project_member_snapshot(&assigned.project_member_ref)
            .expect("member should remain stored");
        assert_eq!(version, 2);
        assert_eq!(stores.trace_count(), 3);
        assert_eq!(stores.stale_mark_count(), 3);
        assert_eq!(outbox.count(), 3);
    }

    #[tokio::test]
    async fn tc_work_formal_001_create_work_item_persists_truth_membership_and_side_effects() {
        let (
            handlers,
            stores,
            outbox,
            results,
            _idempotency,
            member_refs,
            source_refs,
            _evidence_refs,
        ) = build_handlers();
        let (created, assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: false,
            },
        );

        let result = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-001"),
                command: CreateWorkItemRequest {
                    project_ref: created.project_ref.clone(),
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        ..fixtures::formal_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect("create_work_item should succeed");

        assert_eq!(result.work_state, WorkItemState::Formalized);
        assert_eq!(result.receipt.idempotency, IdempotencyResultView::Applied);
        assert_eq!(stores.trace_count(), 3);
        assert_eq!(stores.stale_mark_count(), 3);
        assert_eq!(outbox.count(), 3);
        assert!(
            results
                .get_result(result.receipt.result_ref.clone())
                .await
                .expect("stored result read should succeed")
                .is_some()
        );

        let backlog = stores
            .get_by_project(created.project_ref.clone())
            .await
            .expect("backlog lookup should succeed")
            .expect("backlog should exist");
        let membership = stores.backlog_membership(&backlog.backlog_ref());
        assert_eq!(membership, vec![result.work_ref.clone()]);

        let (work_item, version) = stores
            .work_item_snapshot(&result.work_ref)
            .expect("work item should be stored");
        assert_eq!(work_item.assignee_ref, assigned.project_member_ref);
        assert_eq!(work_item.work_state, WorkItemState::Formalized);
        assert_eq!(version, 1);
    }

    #[tokio::test]
    async fn tc_work_formal_002_external_body_rejected_without_work_truth_write() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            member_refs,
            source_refs,
            _evidence_refs,
        ) = build_handlers();
        let (created, assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: true,
            },
        );

        let error = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-002"),
                command: CreateWorkItemRequest {
                    project_ref: created.project_ref.clone(),
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref,
                        ..fixtures::formal_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect_err("body leak should be rejected");

        assert_eq!(error, WorkProtocolError::DomainRejected);
        let backlog = stores
            .get_by_project(created.project_ref)
            .await
            .expect("backlog lookup should succeed")
            .expect("backlog should exist");
        assert!(stores.backlog_membership(&backlog.backlog_ref()).is_empty());
        assert_eq!(stores.trace_count(), 2);
        assert_eq!(stores.stale_mark_count(), 2);
        assert_eq!(outbox.count(), 2);
    }

    #[tokio::test]
    async fn tc_work_formal_003_locked_backlog_rejects_new_work_item() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            member_refs,
            source_refs,
            _evidence_refs,
        ) = build_handlers();
        let (created, assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: false,
            },
        );
        let backlog_ref = stores
            .get_by_project(created.project_ref.clone())
            .await
            .expect("backlog lookup should succeed")
            .expect("backlog should exist")
            .backlog_ref();
        handlers
            .handle_update_backlog_availability(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-003-lock"),
                command: UpdateBacklogAvailabilityRequest {
                    backlog_ref: backlog_ref.clone(),
                    target: BacklogAvailabilityTarget::LockedForMaintenance,
                    reason: work_contracts::BacklogMaintenanceReason {
                        reason_kind:
                            work_contracts::BacklogMaintenanceReasonKind::MaintenanceWindow,
                        reason_ref: None,
                    },
                    expected_version: 1,
                },
            })
            .await
            .expect("lock should succeed");

        let error = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-003"),
                command: CreateWorkItemRequest {
                    project_ref: created.project_ref,
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref,
                        ..fixtures::formal_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect_err("locked backlog should reject formal work create");

        assert_eq!(error, WorkProtocolError::DomainRejected);
        assert!(stores.backlog_membership(&backlog_ref).is_empty());
        assert_eq!(stores.trace_count(), 3);
        assert_eq!(stores.stale_mark_count(), 3);
        assert_eq!(outbox.count(), 3);
    }

    #[tokio::test]
    async fn tc_work_formal_004_create_work_item_duplicate_replays_stored_result() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            member_refs,
            source_refs,
            _evidence_refs,
        ) = build_handlers();
        let (created, assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: false,
            },
        );

        let envelope = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-formal-004"),
            command: CreateWorkItemRequest {
                project_ref: created.project_ref,
                work_intent: work_contracts::FormalWorkIntent {
                    assignee_ref: assigned.project_member_ref,
                    ..fixtures::formal_work_intent()
                },
                source_ref: fixtures::source_work_ref(),
            },
        };

        let first = handlers
            .handle_create_work_item(envelope.clone())
            .await
            .expect("first create_work_item should succeed");
        let duplicate = handlers
            .handle_create_work_item(envelope)
            .await
            .expect("duplicate create_work_item should replay stored result");

        assert_eq!(first.work_ref, duplicate.work_ref);
        assert_eq!(first.work_state, duplicate.work_state);
        assert_eq!(first.receipt.result_ref, duplicate.receipt.result_ref);
        assert_eq!(
            duplicate.receipt.idempotency,
            IdempotencyResultView::Duplicate
        );
        assert_eq!(stores.trace_count(), 3);
        assert_eq!(stores.stale_mark_count(), 3);
        assert_eq!(outbox.count(), 3);
    }

    #[tokio::test]
    async fn tc_work_formal_005_child_create_and_invalid_parent_lifecycle_completion() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            member_refs,
            source_refs,
            evidence_refs,
        ) = build_handlers();
        let (created, assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: false,
            },
        );

        let parent = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-005-parent"),
                command: CreateWorkItemRequest {
                    project_ref: created.project_ref.clone(),
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        ..fixtures::formal_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect("parent create should succeed");

        let child = handlers
            .handle_create_child_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-005-child"),
                command: CreateChildWorkItemRequest {
                    parent_ref: parent.work_ref.clone(),
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        ..fixtures::child_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect("child create should succeed");
        assert_eq!(child.work_state, WorkItemState::Formalized);
        let (_stored_child, child_version) = stores
            .child_work_item_snapshot(&child.work_ref)
            .expect("child should be stored");
        assert_eq!(child_version, 1);

        let invalid_parent = handlers
            .handle_create_child_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-005-invalid-parent"),
                command: CreateChildWorkItemRequest {
                    parent_ref: child.work_ref.clone(),
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        ..fixtures::child_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect_err("child parent should be rejected");
        assert_eq!(invalid_parent, WorkProtocolError::DomainRejected);

        handlers
            .handle_update_work_item_lifecycle(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-005-parent-start"),
                command: UpdateWorkItemLifecycleRequest {
                    work_ref: parent.work_ref.clone(),
                    target: work_contracts::WorkLifecycleTarget::InProgress,
                    reason: fixtures::start_work_reason(),
                    evidence_ref: None,
                    expected_version: 1,
                },
            })
            .await
            .expect("parent start should succeed");

        let missing_evidence = handlers
            .handle_update_work_item_lifecycle(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-005-missing-evidence"),
                command: UpdateWorkItemLifecycleRequest {
                    work_ref: parent.work_ref.clone(),
                    target: work_contracts::WorkLifecycleTarget::Completed,
                    reason: fixtures::completion_work_reason(),
                    evidence_ref: None,
                    expected_version: 2,
                },
            })
            .await
            .expect_err("completion without evidence should fail");
        assert_eq!(missing_evidence, WorkProtocolError::InvalidRequest);

        evidence_refs.seed(
            fixtures::completion_evidence_ref(),
            EvidenceResolverOutcome::Success,
        );
        let completed = handlers
            .handle_update_work_item_lifecycle(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-formal-005-complete"),
                command: UpdateWorkItemLifecycleRequest {
                    work_ref: parent.work_ref.clone(),
                    target: work_contracts::WorkLifecycleTarget::Completed,
                    reason: fixtures::completion_work_reason(),
                    evidence_ref: Some(fixtures::completion_evidence_ref()),
                    expected_version: 2,
                },
            })
            .await
            .expect("completion with verified evidence should succeed");
        assert_eq!(completed.work_state, WorkItemState::Completed);
        let (stored_parent, parent_version) = stores
            .work_item_snapshot(&parent.work_ref)
            .expect("parent should be stored");
        assert_eq!(
            stored_parent.completion_ref,
            Some(fixtures::completion_evidence_ref())
        );
        assert_eq!(parent_version, 3);
        assert_eq!(stores.trace_count(), 6);
        assert_eq!(stores.stale_mark_count(), 6);
        assert_eq!(outbox.count(), 6);
    }

    #[tokio::test]
    async fn tc_work_core_001_create_project_persists_project_backlog_and_side_effects() {
        let (
            handlers,
            stores,
            outbox,
            results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let envelope = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-001"),
            command: CreateProjectRequest {
                project_spec: fixtures::project_spec(),
            },
        };

        let result = handlers
            .handle_create_project(envelope)
            .await
            .expect("create_project should succeed");

        assert_eq!(result.lifecycle_state, ProjectLifecycleState::Active);
        assert_eq!(result.receipt.idempotency, IdempotencyResultView::Applied);
        assert_eq!(result.receipt.outbox_record_refs.len(), 1);
        assert_eq!(stores.trace_count(), 1);
        assert_eq!(stores.stale_mark_count(), 1);
        assert_eq!(outbox.count(), 1);
        assert!(
            results
                .get_result(result.receipt.result_ref.clone())
                .await
                .expect("stored result read should succeed")
                .is_some()
        );

        let (project, project_version) = stores
            .project_snapshot(&result.project_ref)
            .expect("project should be stored");
        assert_eq!(project.lifecycle_state, ProjectLifecycleState::Active);
        assert_eq!(project_version, 1);
        let (backlog, backlog_version) = stores
            .get_by_project_with_version(result.project_ref.clone())
            .await
            .expect("backlog lookup should succeed")
            .expect("backlog should be stored");
        assert_eq!(backlog.backlog_state, BacklogState::Open);
        assert_eq!(backlog_version, 1);
    }

    #[tokio::test]
    async fn tc_work_core_002_missing_project_write_does_not_implicitly_create_truth() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let envelope = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-002"),
            command: UpdateProjectLifecycleRequest {
                project_ref: fixtures::project_ref(),
                target: ProjectLifecycleTarget::Closed,
                reason: ProjectLifecycleReason {
                    reason_kind: ProjectLifecycleReasonKind::OwnerRequest,
                    reason_ref: None,
                    note: Some(fixtures::safe_summary("close-missing")),
                },
                expected_version: 1,
            },
        };

        let error = handlers
            .handle_update_project_lifecycle(envelope)
            .await
            .expect_err("missing project must not be implicitly created");

        assert_eq!(error, WorkProtocolError::NotFound);
        assert_eq!(stores.trace_count(), 0);
        assert_eq!(stores.stale_mark_count(), 0);
        assert_eq!(outbox.count(), 0);
    }

    #[tokio::test]
    async fn tc_work_core_003_update_project_lifecycle_archives_backlog() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let create = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-003-create"),
            command: CreateProjectRequest {
                project_spec: fixtures::project_spec(),
            },
        };
        let created = handlers
            .handle_create_project(create)
            .await
            .expect("create_project should succeed");

        let close = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-003-close"),
            command: UpdateProjectLifecycleRequest {
                project_ref: created.project_ref.clone(),
                target: ProjectLifecycleTarget::Closed,
                reason: ProjectLifecycleReason {
                    reason_kind: ProjectLifecycleReasonKind::OwnerRequest,
                    reason_ref: None,
                    note: Some(fixtures::safe_summary("close")),
                },
                expected_version: 1,
            },
        };
        let closed = handlers
            .handle_update_project_lifecycle(close)
            .await
            .expect("close should succeed");
        assert_eq!(closed.lifecycle_state, ProjectLifecycleState::Closed);
        assert_eq!(closed.receipt.applied_version, Some(2));

        let archive = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-003-archive"),
            command: UpdateProjectLifecycleRequest {
                project_ref: created.project_ref.clone(),
                target: ProjectLifecycleTarget::Archived,
                reason: ProjectLifecycleReason {
                    reason_kind: ProjectLifecycleReasonKind::ArchivePrepared,
                    reason_ref: None,
                    note: Some(fixtures::safe_summary("archive")),
                },
                expected_version: 2,
            },
        };
        let archived = handlers
            .handle_update_project_lifecycle(archive)
            .await
            .expect("archive should succeed");

        assert_eq!(archived.lifecycle_state, ProjectLifecycleState::Archived);
        assert_eq!(archived.receipt.idempotency, IdempotencyResultView::Applied);
        assert_eq!(archived.receipt.applied_version, Some(3));
        assert_eq!(archived.receipt.outbox_record_refs.len(), 2);
        assert_eq!(stores.trace_count(), 4);
        assert_eq!(stores.stale_mark_count(), 3);
        assert_eq!(outbox.count(), 4);

        let (project, project_version) = stores
            .project_snapshot(&created.project_ref)
            .expect("project should still exist");
        assert_eq!(project.lifecycle_state, ProjectLifecycleState::Archived);
        assert_eq!(project_version, 3);
        let (backlog, backlog_version) = stores
            .get_by_project_with_version(created.project_ref.clone())
            .await
            .expect("backlog lookup should succeed")
            .expect("backlog should still exist");
        assert_eq!(backlog.backlog_state, BacklogState::Archived);
        assert_eq!(backlog_version, 2);
    }

    #[tokio::test]
    async fn tc_work_core_004_create_project_duplicate_replays_stored_result() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let envelope = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-004"),
            command: CreateProjectRequest {
                project_spec: fixtures::project_spec(),
            },
        };

        let first = handlers
            .handle_create_project(envelope.clone())
            .await
            .expect("first create_project should succeed");
        let duplicate = handlers
            .handle_create_project(envelope)
            .await
            .expect("duplicate create_project should replay stored result");

        assert_eq!(first.project_ref, duplicate.project_ref);
        assert_eq!(first.lifecycle_state, duplicate.lifecycle_state);
        assert_eq!(first.receipt.result_ref, duplicate.receipt.result_ref);
        assert_eq!(first.receipt.trace_ref, duplicate.receipt.trace_ref);
        assert_eq!(
            first.receipt.outbox_record_refs,
            duplicate.receipt.outbox_record_refs
        );
        assert_eq!(
            first.receipt.applied_version,
            duplicate.receipt.applied_version
        );
        assert_eq!(first.receipt.idempotency, IdempotencyResultView::Applied);
        assert_eq!(
            duplicate.receipt.idempotency,
            IdempotencyResultView::Duplicate
        );
        assert_eq!(stores.trace_count(), 1);
        assert_eq!(stores.stale_mark_count(), 1);
        assert_eq!(outbox.count(), 1);
    }

    #[tokio::test]
    async fn duplicate_missing_result_surface_maps_to_temporarily_unavailable() {
        let (
            handlers,
            stores,
            outbox,
            results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let envelope = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-004-missing"),
            command: CreateProjectRequest {
                project_spec: fixtures::project_spec(),
            },
        };

        let first = handlers
            .handle_create_project(envelope.clone())
            .await
            .expect("first create_project should succeed");
        results.inject_missing(first.receipt.result_ref.clone());

        let error = handlers
            .handle_create_project(envelope)
            .await
            .expect_err("duplicate without stored result should fail");
        assert_eq!(error, WorkProtocolError::TemporarilyUnavailable);
        assert_eq!(stores.trace_count(), 1);
        assert_eq!(stores.stale_mark_count(), 1);
        assert_eq!(outbox.count(), 1);
    }

    #[tokio::test]
    async fn update_backlog_availability_locks_and_reopens_backlog() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let create = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-backlog-create"),
            command: CreateProjectRequest {
                project_spec: fixtures::project_spec(),
            },
        };
        let created = handlers
            .handle_create_project(create)
            .await
            .expect("create_project should succeed");
        let backlog_ref = stores
            .get_by_project(created.project_ref.clone())
            .await
            .expect("backlog lookup should succeed")
            .expect("backlog should exist")
            .backlog_ref();

        let lock = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-backlog-lock"),
            command: UpdateBacklogAvailabilityRequest {
                backlog_ref: backlog_ref.clone(),
                target: BacklogAvailabilityTarget::LockedForMaintenance,
                reason: work_contracts::BacklogMaintenanceReason {
                    reason_kind: work_contracts::BacklogMaintenanceReasonKind::MaintenanceWindow,
                    reason_ref: None,
                },
                expected_version: 1,
            },
        };
        let locked = handlers
            .handle_update_backlog_availability(lock)
            .await
            .expect("lock should succeed");
        assert_eq!(locked.backlog_state, BacklogState::LockedForMaintenance);
        assert_eq!(locked.receipt.applied_version, Some(2));

        let reopen = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-core-backlog-open"),
            command: UpdateBacklogAvailabilityRequest {
                backlog_ref: backlog_ref.clone(),
                target: BacklogAvailabilityTarget::Open,
                reason: work_contracts::BacklogMaintenanceReason {
                    reason_kind: work_contracts::BacklogMaintenanceReasonKind::ManualUnlock,
                    reason_ref: None,
                },
                expected_version: 2,
            },
        };
        let reopened = handlers
            .handle_update_backlog_availability(reopen)
            .await
            .expect("reopen should succeed");
        assert_eq!(reopened.backlog_state, BacklogState::Open);
        assert_eq!(reopened.receipt.applied_version, Some(3));
        assert_eq!(stores.trace_count(), 3);
        assert_eq!(stores.stale_mark_count(), 3);
        assert_eq!(outbox.count(), 3);
    }

    #[tokio::test]
    async fn invalid_request_maps_to_protocol_error_without_side_effects() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
        ) = build_handlers();
        let envelope = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: core_contracts::metadata::CommandMetadata {
                request: fixtures::request_metadata(None),
                reason: None,
                external_ref: None,
            },
            command: CreateProjectRequest {
                project_spec: fixtures::project_spec(),
            },
        };

        let error = handlers
            .handle_create_project(envelope)
            .await
            .expect_err("missing idempotency key should fail");
        assert_eq!(error, WorkProtocolError::InvalidRequest);
        assert_eq!(stores.trace_count(), 0);
        assert_eq!(stores.stale_mark_count(), 0);
        assert_eq!(outbox.count(), 0);
    }
}
