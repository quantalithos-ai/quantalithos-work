//! API entrypoints for the Work bounded context.

use work_application::{
    ApplicationError, AuthorizedWorkQueryService, DependencyBlockerService,
    IterationCommandService, ProjectCommandService, ProjectMemberCommandService,
    PromoteCommandService, WorkItemCommandService,
};
use work_contracts::{
    AssignProjectMemberRequest, BacklogCommandResult, BlockerCommandResult,
    CommitIterationScopeRequest, CreateChildWorkItemRequest, CreateProjectRequest,
    CreateWorkItemRequest, DependencyCommandResult, GetBacklogRequest, GetIterationSummaryRequest,
    GetProjectBoardViewRequest, GetProjectWorkFactsRequest, GetWorkItemRequest,
    GetWorkTraceRequest, IterationCommandResult, LinkWorkDependencyRequest, ListMemberWorkRequest,
    OpenIterationRequest, OpenWorkBlockerRequest, ProjectCommandResult, ProjectMemberCommandResult,
    PromoteCommandResult, RequestWorkPromotionRequest, ResolveWorkBlockerRequest,
    ReviewWorkPromotionRequest, SearchWorkRequest, UpdateBacklogAvailabilityRequest,
    UpdateIterationCommitmentRequest, UpdateIterationLifecycleRequest,
    UpdateProjectLifecycleRequest, UpdateProjectMemberResponsibilityRequest,
    UpdateWorkDependencyStateRequest, UpdateWorkItemLifecycleRequest, WorkCommandEnvelope,
    WorkItemCommandResult, WorkProtocolError, WorkQueryEnvelope, WorkQueryResponse,
};

/// Thin command handlers that validate protocol shape and delegate to application services.
pub struct WorkCommandHandlers<P, M, W, PR, D, I> {
    /// Project-scoped command service.
    pub project_service: P,
    /// Project-member command service.
    pub member_service: M,
    /// Formal work command service.
    pub workitem_service: W,
    /// Promote command service.
    pub promote_service: PR,
    /// Dependency and blocker command service.
    pub dependency_service: D,
    /// Iteration command service.
    pub iteration_service: I,
}

/// Thin query handlers that validate protocol shape and delegate to application services.
pub struct WorkQueryHandlers<Q> {
    /// Authorized read-only query service.
    pub query_service: Q,
}

impl<Q> WorkQueryHandlers<Q> {
    /// Creates a query handler set for read delegation.
    pub fn new(query_service: Q) -> Self {
        Self { query_service }
    }
}

impl<P, M, W, PR, D, I> WorkCommandHandlers<P, M, W, PR, D, I> {
    /// Creates a handler set for command delegation.
    pub fn new(
        project_service: P,
        member_service: M,
        workitem_service: W,
        promote_service: PR,
        dependency_service: D,
        iteration_service: I,
    ) -> Self {
        Self {
            project_service,
            member_service,
            workitem_service,
            promote_service,
            dependency_service,
            iteration_service,
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
    PP,
    PB,
    PW,
    PPR,
    PA,
    PO,
    PRR,
    PPROJ,
    PU,
    PS,
    PI,
    PCC,
    PIDEM,
    DP,
    DW,
    DA,
    DO,
    DR,
    DPR,
    DU,
    DE,
    DI,
    DC,
    DIDEM,
    IP,
    IB,
    IW,
    IITR,
    IA,
    IO,
    IR,
    IPR,
    IU,
    IPT,
    II,
    IC,
    IIDEM,
>
    WorkCommandHandlers<
        ProjectCommandService<P, B, A, O, R, PR, U, I, C, IDEM>,
        ProjectMemberCommandService<MP, MPM, MRS, MA, MO, MR, MPR, MU, MM, MI, MC, MIDEM>,
        WorkItemCommandService<WP, WB, WPM, WW, WA, WO, WR, WPR, WU, WS, WE, WI, WC, WIDEM>,
        PromoteCommandService<PP, PB, PW, PPR, PA, PO, PRR, PPROJ, PU, PS, PI, PCC, PIDEM>,
        DependencyBlockerService<DP, DW, DA, DO, DR, DPR, DU, DE, DI, DC, DIDEM>,
        IterationCommandService<IP, IB, IW, IITR, IA, IO, IR, IPR, IU, IPT, II, IC, IIDEM>,
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
    PP: work_application::ProjectMemberRepository,
    PB: work_application::BacklogRepository,
    PW: work_application::WorkItemRepository,
    PPR: work_application::PromoteRepository,
    PA: work_application::AuditRepository,
    PO: work_application::WorkOutboxRepository,
    PRR: work_application::CommandResultRepository,
    PPROJ: work_application::ProjectionRepository,
    PU: work_application::UnitOfWork,
    PS: work_application::SourceWorkResolverPort,
    PI: work_application::IdGeneratorPort,
    PCC: work_application::ClockPort,
    PIDEM: work_application::IdempotencyRepository,
    DP: work_application::DependencyRepository,
    DW: work_application::WorkItemRepository,
    DA: work_application::AuditRepository,
    DO: work_application::WorkOutboxRepository,
    DR: work_application::CommandResultRepository,
    DPR: work_application::ProjectionRepository,
    DU: work_application::UnitOfWork,
    DE: work_application::EvidenceResolverPort,
    DI: work_application::IdGeneratorPort,
    DC: work_application::ClockPort,
    DIDEM: work_application::IdempotencyRepository,
    IP: work_application::ProjectRepository,
    IB: work_application::BacklogRepository,
    IW: work_application::WorkItemRepository,
    IITR: work_application::IterationRepository,
    IA: work_application::AuditRepository,
    IO: work_application::WorkOutboxRepository,
    IR: work_application::CommandResultRepository,
    IPR: work_application::ProjectionRepository,
    IU: work_application::UnitOfWork,
    IPT: work_application::ProcessTimeboxResolverPort,
    II: work_application::IdGeneratorPort,
    IC: work_application::ClockPort,
    IIDEM: work_application::IdempotencyRepository,
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

    /// Handles `RequestWorkPromotion`.
    pub async fn handle_request_work_promotion(
        &self,
        envelope: WorkCommandEnvelope<RequestWorkPromotionRequest>,
    ) -> Result<PromoteCommandResult, WorkProtocolError> {
        self.promote_service
            .request_promotion(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `ReviewWorkPromotion`.
    pub async fn handle_review_work_promotion(
        &self,
        envelope: WorkCommandEnvelope<ReviewWorkPromotionRequest>,
    ) -> Result<PromoteCommandResult, WorkProtocolError> {
        self.promote_service
            .review_promotion(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `LinkWorkDependency`.
    pub async fn handle_link_work_dependency(
        &self,
        envelope: WorkCommandEnvelope<LinkWorkDependencyRequest>,
    ) -> Result<DependencyCommandResult, WorkProtocolError> {
        self.dependency_service
            .link_dependency(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `UpdateWorkDependencyState`.
    pub async fn handle_update_work_dependency_state(
        &self,
        envelope: WorkCommandEnvelope<UpdateWorkDependencyStateRequest>,
    ) -> Result<DependencyCommandResult, WorkProtocolError> {
        self.dependency_service
            .update_dependency_state(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `OpenWorkBlocker`.
    pub async fn handle_open_work_blocker(
        &self,
        envelope: WorkCommandEnvelope<OpenWorkBlockerRequest>,
    ) -> Result<BlockerCommandResult, WorkProtocolError> {
        self.dependency_service
            .open_blocker(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `ResolveWorkBlocker`.
    pub async fn handle_resolve_work_blocker(
        &self,
        envelope: WorkCommandEnvelope<ResolveWorkBlockerRequest>,
    ) -> Result<BlockerCommandResult, WorkProtocolError> {
        self.dependency_service
            .resolve_blocker(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `OpenIteration`.
    pub async fn handle_open_iteration(
        &self,
        envelope: WorkCommandEnvelope<OpenIterationRequest>,
    ) -> Result<IterationCommandResult, WorkProtocolError> {
        self.iteration_service
            .open_iteration(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `CommitIterationScope`.
    pub async fn handle_commit_iteration_scope(
        &self,
        envelope: WorkCommandEnvelope<CommitIterationScopeRequest>,
    ) -> Result<IterationCommandResult, WorkProtocolError> {
        self.iteration_service
            .commit_iteration_scope(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `UpdateIterationCommitment`.
    pub async fn handle_update_iteration_commitment(
        &self,
        envelope: WorkCommandEnvelope<UpdateIterationCommitmentRequest>,
    ) -> Result<IterationCommandResult, WorkProtocolError> {
        self.iteration_service
            .update_iteration_commitment(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `UpdateIterationLifecycle`.
    pub async fn handle_update_iteration_lifecycle(
        &self,
        envelope: WorkCommandEnvelope<UpdateIterationLifecycleRequest>,
    ) -> Result<IterationCommandResult, WorkProtocolError> {
        self.iteration_service
            .update_iteration_lifecycle(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }
}

impl<PJ, PM, B, W, PRM, D, I, A, PROJ, AMR>
    WorkQueryHandlers<AuthorizedWorkQueryService<PJ, PM, B, W, PRM, D, I, A, PROJ, AMR>>
where
    PJ: work_application::ProjectRepository,
    PM: work_application::ProjectMemberRepository,
    B: work_application::BacklogRepository,
    W: work_application::WorkItemRepository,
    PRM: work_application::PromoteRepository,
    D: work_application::DependencyRepository,
    I: work_application::IterationRepository,
    A: work_application::AuditRepository,
    PROJ: work_application::ProjectionRepository,
    AMR: work_application::ActorMemberResolverPort,
{
    /// Handles `GetProjectWorkFacts`.
    pub async fn handle_get_project_work_facts(
        &self,
        envelope: WorkQueryEnvelope<GetProjectWorkFactsRequest>,
    ) -> Result<WorkQueryResponse<work_contracts::ProjectWorkFactsView>, WorkProtocolError> {
        self.query_service
            .get_project_work_facts(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `GetBacklog`.
    pub async fn handle_get_backlog(
        &self,
        envelope: WorkQueryEnvelope<GetBacklogRequest>,
    ) -> Result<WorkQueryResponse<work_contracts::views::BacklogView>, WorkProtocolError> {
        self.query_service
            .get_backlog(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `GetWorkItem`.
    pub async fn handle_get_work_item(
        &self,
        envelope: WorkQueryEnvelope<GetWorkItemRequest>,
    ) -> Result<WorkQueryResponse<work_contracts::WorkItemView>, WorkProtocolError> {
        self.query_service
            .get_work_item(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `ListMemberWork`.
    pub async fn handle_list_member_work(
        &self,
        envelope: WorkQueryEnvelope<ListMemberWorkRequest>,
    ) -> Result<WorkQueryResponse<work_contracts::views::MemberWorkView>, WorkProtocolError> {
        self.query_service
            .list_member_work(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `GetIterationSummary`.
    pub async fn handle_get_iteration_summary(
        &self,
        envelope: WorkQueryEnvelope<GetIterationSummaryRequest>,
    ) -> Result<WorkQueryResponse<work_contracts::views::IterationSummaryView>, WorkProtocolError>
    {
        self.query_service
            .get_iteration_summary(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `SearchWork`.
    pub async fn handle_search_work(
        &self,
        envelope: WorkQueryEnvelope<SearchWorkRequest>,
    ) -> Result<WorkQueryResponse<work_contracts::views::WorkSearchResult>, WorkProtocolError> {
        self.query_service
            .search_work(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `GetWorkTrace`.
    pub async fn handle_get_work_trace(
        &self,
        envelope: WorkQueryEnvelope<GetWorkTraceRequest>,
    ) -> Result<WorkQueryResponse<work_contracts::views::WorkTraceView>, WorkProtocolError> {
        self.query_service
            .get_work_trace(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }

    /// Handles `GetProjectBoardView`.
    pub async fn handle_get_project_board_view(
        &self,
        envelope: WorkQueryEnvelope<GetProjectBoardViewRequest>,
    ) -> Result<WorkQueryResponse<work_contracts::views::ProjectBoardView>, WorkProtocolError> {
        self.query_service
            .get_project_board_view(envelope)
            .await
            .map_err(ApplicationError::into_protocol_error)
    }
}

#[cfg(test)]
mod tests {
    use core_contracts::metadata::Timestamp;

    use super::{WorkCommandHandlers, WorkQueryHandlers};
    use work_application::{
        AuditRepository, AuthorizedWorkQueryService, BacklogRepository, CommandResultRepository,
        DependencyBlockerService, IterationCommandService, IterationRepository,
        ProjectCommandService, ProjectMemberCommandService, ProjectionRepository,
        PromoteCommandService, UnitOfWork, WorkItemCommandService, WorkQueryVisibilityPolicy,
    };
    use work_contracts::metadata::fixtures;
    use work_contracts::{
        AssignProjectMemberRequest, BacklogAvailabilityTarget, BacklogState,
        CommitIterationScopeRequest, CommitmentState, CreateChildWorkItemRequest,
        CreateProjectRequest, CreateWorkItemRequest, DependencyTarget, DerivedWorkViewRef,
        GetBacklogRequest, GetIterationSummaryRequest, GetProjectBoardViewRequest,
        GetProjectWorkFactsRequest, GetWorkItemRequest, GetWorkTraceRequest, IdempotencyResultView,
        IterationLifecycleTarget, IterationState, LinkWorkDependencyRequest, ListMemberWorkRequest,
        OpenIterationRequest, OpenWorkBlockerRequest, ProjectLifecycleReason,
        ProjectLifecycleReasonKind, ProjectLifecycleState, ProjectLifecycleTarget,
        ProjectMemberReason, ProjectMemberReasonKind, ProjectMemberResponsibilityState,
        PromoteResultState, PromoteReviewDecision, QuerySurface, RequestWorkPromotionRequest,
        ResolveWorkBlockerRequest, ResponsibilityTarget, ReviewWorkPromotionRequest,
        SearchWorkRequest, UpdateBacklogAvailabilityRequest, UpdateIterationCommitmentRequest,
        UpdateIterationLifecycleRequest, UpdateProjectLifecycleRequest,
        UpdateProjectMemberResponsibilityRequest, UpdateWorkDependencyStateRequest,
        UpdateWorkItemLifecycleRequest, WorkCommandEnvelope, WorkItemState, WorkProtocolError,
        WorkQueryEnvelope, WorkTraceSubjectRef,
    };
    use work_infra::clock_id::{DeterministicWorkIdGenerator, FixedClock};
    use work_infra::command_result_store::InMemoryCommandResultRepository;
    use work_infra::idempotency_store::InMemoryIdempotencyRepository;
    use work_infra::outbox_store::InMemoryWorkOutboxRepository;
    use work_infra::repositories::InMemoryWorkStores;
    use work_infra::source_resolvers::{
        ActorMemberResolverOutcome, EvidenceResolverOutcome, FakeActorMemberResolverPort,
        FakeEvidenceResolverPort, FakeMemberReferencePort, FakeProcessTimeboxResolverPort,
        FakeSourceWorkResolverPort, MemberResolverOutcome, ProcessTimeboxResolverOutcome,
        SourceResolverOutcome,
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
        PromoteCommandService<
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
            DeterministicWorkIdGenerator,
            FixedClock,
            InMemoryIdempotencyRepository,
        >,
        DependencyBlockerService<
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkOutboxRepository,
            InMemoryCommandResultRepository,
            InMemoryWorkStores,
            InMemoryWorkStores,
            FakeEvidenceResolverPort,
            DeterministicWorkIdGenerator,
            FixedClock,
            InMemoryIdempotencyRepository,
        >,
        IterationCommandService<
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkOutboxRepository,
            InMemoryCommandResultRepository,
            InMemoryWorkStores,
            InMemoryWorkStores,
            FakeProcessTimeboxResolverPort,
            DeterministicWorkIdGenerator,
            FixedClock,
            InMemoryIdempotencyRepository,
        >,
    >;

    fn build_handlers_with_process() -> (
        TestHandlers,
        InMemoryWorkStores,
        InMemoryWorkOutboxRepository,
        InMemoryCommandResultRepository,
        InMemoryIdempotencyRepository,
        FakeMemberReferencePort,
        FakeSourceWorkResolverPort,
        FakeEvidenceResolverPort,
        FakeProcessTimeboxResolverPort,
    ) {
        let stores = InMemoryWorkStores::new();
        let outbox = InMemoryWorkOutboxRepository::new();
        let results = InMemoryCommandResultRepository::new();
        let idempotency = InMemoryIdempotencyRepository::new();
        let member_refs = FakeMemberReferencePort::new();
        let source_refs = FakeSourceWorkResolverPort::new();
        let evidence_refs = FakeEvidenceResolverPort::new();
        let process_refs = FakeProcessTimeboxResolverPort::new();
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
            ids: ids.clone(),
            clock: clock.clone(),
            idempotency: idempotency.clone(),
        };
        let promote_service = PromoteCommandService {
            member_repo: stores.clone(),
            backlog_repo: stores.clone(),
            work_repo: stores.clone(),
            promote_repo: stores.clone(),
            audit_repo: stores.clone(),
            outbox_repo: outbox.clone(),
            command_results: results.clone(),
            projection_repo: stores.clone(),
            unit_of_work: stores.clone(),
            source_resolver: source_refs.clone(),
            ids: ids.clone(),
            clock: clock.clone(),
            idempotency: idempotency.clone(),
        };
        let dependency_service = DependencyBlockerService {
            dependency_repo: stores.clone(),
            work_repo: stores.clone(),
            audit_repo: stores.clone(),
            outbox_repo: outbox.clone(),
            command_results: results.clone(),
            projection_repo: stores.clone(),
            unit_of_work: stores.clone(),
            evidence_resolver: evidence_refs.clone(),
            ids: ids.clone(),
            clock: clock.clone(),
            idempotency: idempotency.clone(),
        };
        let iteration_service = IterationCommandService {
            project_repo: stores.clone(),
            backlog_repo: stores.clone(),
            work_repo: stores.clone(),
            iteration_repo: stores.clone(),
            audit_repo: stores.clone(),
            outbox_repo: outbox.clone(),
            command_results: results.clone(),
            projection_repo: stores.clone(),
            unit_of_work: stores.clone(),
            timebox_resolver: process_refs.clone(),
            ids: ids.clone(),
            clock: clock.clone(),
            idempotency: idempotency.clone(),
        };
        (
            WorkCommandHandlers::new(
                project_service,
                member_service,
                workitem_service,
                promote_service,
                dependency_service,
                iteration_service,
            ),
            stores,
            outbox,
            results,
            idempotency,
            member_refs,
            source_refs,
            evidence_refs,
            process_refs,
        )
    }

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
        let (
            handlers,
            stores,
            outbox,
            results,
            idempotency,
            member_refs,
            source_refs,
            evidence_refs,
            _process_refs,
        ) = build_handlers_with_process();
        (
            handlers,
            stores,
            outbox,
            results,
            idempotency,
            member_refs,
            source_refs,
            evidence_refs,
        )
    }

    type TestQueryService = AuthorizedWorkQueryService<
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        InMemoryWorkStores,
        FakeActorMemberResolverPort,
    >;

    type TestQueryHandlers = WorkQueryHandlers<TestQueryService>;

    fn build_query_service(
        stores: InMemoryWorkStores,
        actor_member_resolver: FakeActorMemberResolverPort,
    ) -> TestQueryService {
        AuthorizedWorkQueryService {
            project_repo: stores.clone(),
            member_repo: stores.clone(),
            backlog_repo: stores.clone(),
            work_repo: stores.clone(),
            promote_repo: stores.clone(),
            dependency_repo: stores.clone(),
            iteration_repo: stores.clone(),
            audit_repo: stores.clone(),
            projection_repo: stores,
            actor_member_resolver,
            visibility: WorkQueryVisibilityPolicy,
        }
    }

    fn build_query_handlers(
        stores: InMemoryWorkStores,
        actor_member_resolver: FakeActorMemberResolverPort,
    ) -> TestQueryHandlers {
        WorkQueryHandlers::new(build_query_service(stores, actor_member_resolver))
    }

    async fn prepare_query_context() -> (
        TestQueryService,
        InMemoryWorkStores,
        InMemoryWorkOutboxRepository,
        work_contracts::ProjectRef,
        work_contracts::ProjectMemberRef,
        work_contracts::FormalWorkRef,
    ) {
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
        let work = create_root_work(
            &handlers,
            created.project_ref.clone(),
            assigned.project_member_ref.clone(),
            &source_refs,
            "query-work-root",
            "Query formal work",
        )
        .await;

        let actor_resolver = FakeActorMemberResolverPort::new();
        actor_resolver.seed(
            &fixtures::query_actor_context(),
            ActorMemberResolverOutcome::Success(fixtures::global_member_ref()),
        );

        (
            build_query_service(stores.clone(), actor_resolver),
            stores,
            outbox,
            created.project_ref,
            assigned.project_member_ref,
            work.work_ref,
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

    async fn create_root_work(
        handlers: &TestHandlers,
        project_ref: work_contracts::ProjectRef,
        assignee_ref: work_contracts::ProjectMemberRef,
        source_refs: &FakeSourceWorkResolverPort,
        key: &str,
        title: &str,
    ) -> work_contracts::WorkItemCommandResult {
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: false,
            },
        );
        handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata(key),
                command: CreateWorkItemRequest {
                    project_ref,
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref,
                        title: fixtures::work_title(title),
                        ..fixtures::formal_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect("create work should succeed")
    }

    async fn create_child_work(
        handlers: &TestHandlers,
        parent_ref: work_contracts::FormalWorkRef,
        assignee_ref: work_contracts::ProjectMemberRef,
        source_refs: &FakeSourceWorkResolverPort,
        key: &str,
    ) -> work_contracts::WorkItemCommandResult {
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: false,
            },
        );
        handlers
            .handle_create_child_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata(key),
                command: CreateChildWorkItemRequest {
                    parent_ref,
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref,
                        ..fixtures::child_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect("create child work should succeed")
    }

    fn query_metadata_with_page(
        limit: u32,
        token: Option<&str>,
    ) -> core_contracts::metadata::QueryMetadata {
        core_contracts::metadata::QueryMetadata {
            request: fixtures::request_metadata(None),
            page: Some(core_contracts::metadata::PageRequest {
                limit,
                page_token: token.map(fixtures::page_token),
            }),
            consistency: core_contracts::metadata::QueryConsistency::Eventual,
        }
    }

    #[tokio::test]
    async fn tc_work_query_001_project_work_facts_hit_missing_not_visible() {
        let (service, stores, outbox, project_ref, _member_ref, _work_ref) =
            prepare_query_context().await;
        let trace_before = stores.trace_count();
        let stale_before = stores.stale_mark_count();
        let outbox_before = outbox.count();

        let visible = service
            .get_project_work_facts(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetProjectWorkFactsRequest {
                    project_ref: project_ref.clone(),
                },
            })
            .await
            .expect("visible query should succeed");
        assert_eq!(visible.surface, QuerySurface::Visible);
        assert_eq!(
            visible.data.expect("payload should exist").project_ref,
            project_ref.clone()
        );

        let missing = service
            .get_project_work_facts(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetProjectWorkFactsRequest {
                    project_ref: work_contracts::ProjectRef {
                        project_id: work_contracts::ProjectId("missing-project".to_owned()),
                    },
                },
            })
            .await
            .expect("missing project should use surface");
        assert_eq!(missing.surface, QuerySurface::Missing);
        assert!(missing.data.is_none());

        let hidden_resolver = FakeActorMemberResolverPort::new();
        hidden_resolver.seed(
            &fixtures::query_actor_context(),
            ActorMemberResolverOutcome::Rejected,
        );
        let hidden_service = build_query_service(stores.clone(), hidden_resolver);
        let hidden = hidden_service
            .get_project_work_facts(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetProjectWorkFactsRequest { project_ref },
            })
            .await
            .expect("not visible should remain query surface");
        assert_eq!(hidden.surface, QuerySurface::NotVisible);
        assert!(hidden.data.is_none());

        assert_eq!(stores.trace_count(), trace_before);
        assert_eq!(stores.stale_mark_count(), stale_before);
        assert_eq!(outbox.count(), outbox_before);
    }

    #[tokio::test]
    async fn tc_work_query_002_backlog_page_and_empty() {
        let (service, stores, outbox, project_ref, _member_ref, _work_ref) =
            prepare_query_context().await;
        let trace_before = stores.trace_count();
        let stale_before = stores.stale_mark_count();
        let outbox_before = outbox.count();

        let visible = service
            .get_backlog(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetBacklogRequest {
                    project_ref: project_ref.clone(),
                    filter: None,
                },
            })
            .await
            .expect("backlog should be visible");
        assert_eq!(visible.surface, QuerySurface::Visible);
        assert_eq!(visible.data.expect("payload should exist").items.len(), 1);

        let empty = service
            .get_backlog(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetBacklogRequest {
                    project_ref,
                    filter: Some(work_contracts::BacklogQueryFilter {
                        work_state: Some(WorkItemState::Completed),
                        assignee_ref: None,
                    }),
                },
            })
            .await
            .expect("filtered backlog should succeed");
        assert_eq!(empty.surface, QuerySurface::Empty);
        assert!(
            empty
                .data
                .expect("empty response keeps payload")
                .items
                .is_empty()
        );

        assert_eq!(stores.trace_count(), trace_before);
        assert_eq!(stores.stale_mark_count(), stale_before);
        assert_eq!(outbox.count(), outbox_before);
    }

    #[tokio::test]
    async fn tc_work_query_003_work_item_visible_and_not_visible() {
        let (service, stores, _outbox, _project_ref, _member_ref, work_ref) =
            prepare_query_context().await;

        let visible = service
            .get_work_item(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetWorkItemRequest {
                    work_ref: work_ref.clone(),
                },
            })
            .await
            .expect("work item should be visible");
        assert_eq!(visible.surface, QuerySurface::Visible);
        assert_eq!(
            visible.data.expect("payload should exist").work_ref,
            work_ref.clone()
        );

        let hidden_resolver = FakeActorMemberResolverPort::new();
        hidden_resolver.seed(
            &fixtures::query_actor_context(),
            ActorMemberResolverOutcome::Unresolved,
        );
        let hidden_service = build_query_service(stores, hidden_resolver);
        let hidden = hidden_service
            .get_work_item(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetWorkItemRequest { work_ref },
            })
            .await
            .expect("not visible should stay in surface");
        assert_eq!(hidden.surface, QuerySurface::NotVisible);
        assert!(hidden.data.is_none());
    }

    #[tokio::test]
    async fn tc_work_query_004_member_work_projection_surfaces() {
        let (service, stores, outbox, _project_ref, member_ref, work_ref) =
            prepare_query_context().await;
        let trace_before = stores.trace_count();
        let stale_before = stores.stale_mark_count();
        let outbox_before = outbox.count();

        stores.seed_member_work_public_view(work_contracts::views::MemberWorkView {
            member_ref: member_ref.clone(),
            assigned_work: vec![work_contracts::views::FormalWorkSummaryView {
                work_ref,
                work_state: WorkItemState::Formalized,
                assignee_ref: Some(member_ref.clone()),
                completion_ref: None,
            }],
            marker: work_contracts::ProjectionViewMarker {
                view_ref: work_contracts::DerivedWorkViewRef::member_work(member_ref.clone()),
                source_cursor: fixtures::truth_cursor(),
                freshness_state: work_contracts::DerivedFreshnessState::Stale,
            },
            page: work_contracts::PublicPageInfo {
                next_page_token: None,
                has_more: false,
            },
        });

        let stale = service
            .list_member_work(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: ListMemberWorkRequest {
                    project_member_ref: member_ref.clone(),
                    work_state: None,
                },
            })
            .await
            .expect("stale projection should succeed");
        assert_eq!(stale.surface, QuerySurface::Stale);

        stores.seed_member_work_public_view(work_contracts::views::MemberWorkView {
            member_ref: member_ref.clone(),
            assigned_work: Vec::new(),
            marker: work_contracts::ProjectionViewMarker {
                view_ref: work_contracts::DerivedWorkViewRef::member_work(member_ref.clone()),
                source_cursor: fixtures::truth_cursor(),
                freshness_state: work_contracts::DerivedFreshnessState::Failed,
            },
            page: work_contracts::PublicPageInfo {
                next_page_token: None,
                has_more: false,
            },
        });
        let failed = service
            .list_member_work(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: ListMemberWorkRequest {
                    project_member_ref: member_ref.clone(),
                    work_state: None,
                },
            })
            .await
            .expect("failed projection should succeed");
        assert_eq!(failed.surface, QuerySurface::Failed);

        stores.clear_member_work_view(&member_ref);
        let rebuilding = service
            .list_member_work(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: ListMemberWorkRequest {
                    project_member_ref: member_ref.clone(),
                    work_state: None,
                },
            })
            .await
            .expect("missing projection should map to rebuilding");
        assert_eq!(rebuilding.surface, QuerySurface::Rebuilding);
        assert!(rebuilding.data.is_none());

        stores.seed_member_work_public_view(work_contracts::views::MemberWorkView {
            member_ref: member_ref.clone(),
            assigned_work: Vec::new(),
            marker: work_contracts::ProjectionViewMarker {
                view_ref: work_contracts::DerivedWorkViewRef::member_work(member_ref.clone()),
                source_cursor: fixtures::truth_cursor(),
                freshness_state: work_contracts::DerivedFreshnessState::Fresh,
            },
            page: work_contracts::PublicPageInfo {
                next_page_token: None,
                has_more: false,
            },
        });
        let visible = service
            .list_member_work(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: ListMemberWorkRequest {
                    project_member_ref: member_ref,
                    work_state: Some(WorkItemState::Completed),
                },
            })
            .await
            .expect("filter should preserve projection and return empty page");
        assert_eq!(visible.surface, QuerySurface::Visible);
        assert!(
            visible
                .data
                .expect("fresh response should keep payload")
                .assigned_work
                .is_empty()
        );

        assert_eq!(stores.trace_count(), trace_before);
        assert_eq!(stores.stale_mark_count(), stale_before);
        assert_eq!(outbox.count(), outbox_before);
    }

    #[tokio::test]
    async fn tc_work_query_005_iteration_summary_projection_surface() {
        let (service, stores, outbox, project_ref, _member_ref, _work_ref) =
            prepare_query_context().await;
        let trace_before = stores.trace_count();
        let stale_before = stores.stale_mark_count();
        let outbox_before = outbox.count();

        let iteration_ref = work_contracts::IterationRef {
            iteration_id: work_contracts::IterationId("query-iteration-1".to_owned()),
        };
        let uow = stores.begin().await.expect("uow should begin");
        stores
            .create_iteration(
                work_domain::Iteration::open(
                    iteration_ref.iteration_id.clone(),
                    project_ref.project_id.clone(),
                    fixtures::process_timebox_ref(),
                    fixtures::actor_context().actor,
                )
                .expect("iteration should open"),
                &uow,
            )
            .await
            .expect("iteration should persist");
        stores.commit(uow).await.expect("uow should commit");
        stores.seed_iteration_summary_public_view(work_contracts::views::IterationSummaryView {
            iteration_ref: iteration_ref.clone(),
            iteration_state: work_contracts::IterationState::Planning,
            commitment_state: None,
            committed_work: Vec::new(),
            marker: work_contracts::ProjectionViewMarker {
                view_ref: work_contracts::DerivedWorkViewRef::iteration_summary(
                    iteration_ref.clone(),
                ),
                source_cursor: fixtures::truth_cursor(),
                freshness_state: work_contracts::DerivedFreshnessState::Stale,
            },
        });

        let stale = service
            .get_iteration_summary(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetIterationSummaryRequest {
                    iteration_ref: iteration_ref.clone(),
                },
            })
            .await
            .expect("stale summary should succeed");
        assert_eq!(stale.surface, QuerySurface::Stale);

        stores.seed_iteration_summary_public_view(work_contracts::views::IterationSummaryView {
            iteration_ref: iteration_ref.clone(),
            iteration_state: work_contracts::IterationState::Planning,
            commitment_state: None,
            committed_work: Vec::new(),
            marker: work_contracts::ProjectionViewMarker {
                view_ref: work_contracts::DerivedWorkViewRef::iteration_summary(
                    iteration_ref.clone(),
                ),
                source_cursor: fixtures::truth_cursor(),
                freshness_state: work_contracts::DerivedFreshnessState::Fresh,
            },
        });

        let visible = service
            .get_iteration_summary(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetIterationSummaryRequest {
                    iteration_ref: iteration_ref.clone(),
                },
            })
            .await
            .expect("fresh summary should succeed");
        assert_eq!(visible.surface, QuerySurface::Visible);

        let missing_projection_ref = work_contracts::IterationRef {
            iteration_id: work_contracts::IterationId("query-iteration-2".to_owned()),
        };
        let uow = stores.begin().await.expect("uow should begin");
        stores
            .create_iteration(
                work_domain::Iteration::open(
                    missing_projection_ref.iteration_id.clone(),
                    project_ref.project_id.clone(),
                    fixtures::process_timebox_ref(),
                    fixtures::actor_context().actor,
                )
                .expect("iteration should open"),
                &uow,
            )
            .await
            .expect("iteration should persist");
        stores.commit(uow).await.expect("uow should commit");

        let missing_projection = service
            .get_iteration_summary(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetIterationSummaryRequest {
                    iteration_ref: missing_projection_ref,
                },
            })
            .await
            .expect("missing projection should use surface");
        assert_eq!(missing_projection.surface, QuerySurface::Missing);

        let missing = service
            .get_iteration_summary(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetIterationSummaryRequest {
                    iteration_ref: work_contracts::IterationRef {
                        iteration_id: work_contracts::IterationId("missing-iteration".to_owned()),
                    },
                },
            })
            .await
            .expect("missing iteration should use surface");
        assert_eq!(missing.surface, QuerySurface::Missing);

        assert_eq!(stores.trace_count(), trace_before);
        assert_eq!(stores.stale_mark_count(), stale_before);
        assert_eq!(outbox.count(), outbox_before);
    }

    #[tokio::test]
    async fn tc_work_query_006_search_work_criteria_failed_and_no_write() {
        let (service, stores, outbox, project_ref, member_ref, work_ref) =
            prepare_query_context().await;
        let trace_before = stores.trace_count();
        let stale_before = stores.stale_mark_count();
        let outbox_before = outbox.count();

        stores.seed_search_rows(
            &project_ref,
            vec![work_contracts::views::WorkSearchProjection {
                project_ref: project_ref.clone(),
                work_ref: work_ref.clone(),
                title: fixtures::work_title("Query formal work"),
                work_state: WorkItemState::Formalized,
                assignee_ref: Some(member_ref.clone()),
                source_cursor: fixtures::truth_cursor(),
            }],
        );
        let search_view_ref = DerivedWorkViewRef::search(
            project_ref.clone(),
            fixtures::work_search_criteria_digest(),
        );
        let uow = stores.begin().await.expect("uow should begin");
        stores
            .mark_failed(
                vec![search_view_ref.clone()],
                fixtures::truth_cursor(),
                work_domain::ProjectionFailureReason::from_build_error(
                    fixtures::truth_cursor(),
                    "search projection failed".to_owned(),
                ),
                &uow,
            )
            .await
            .expect("failed marker should persist");
        stores.commit(uow).await.expect("uow should commit");

        let failed = service
            .search_work(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: query_metadata_with_page(25, Some("page-2")),
                query: SearchWorkRequest {
                    project_ref: project_ref.clone(),
                    criteria: fixtures::work_search_criteria(),
                },
            })
            .await
            .expect("failed projection should still surface");
        assert_eq!(failed.surface, QuerySurface::Failed);
        let failed_data = failed.data.expect("failed projection keeps payload");
        assert_eq!(failed_data.project_ref, project_ref.clone());
        assert_eq!(failed_data.items.len(), 1);
        assert_eq!(failed_data.items[0].work_ref, work_ref.clone());
        assert_eq!(
            failed_data.marker.view_ref,
            DerivedWorkViewRef::search(
                project_ref.clone(),
                fixtures::work_search_criteria_digest()
            )
        );
        assert!(failed_data.page.next_page_token.is_none());

        let hidden_resolver = FakeActorMemberResolverPort::new();
        hidden_resolver.seed(
            &fixtures::query_actor_context(),
            ActorMemberResolverOutcome::Rejected,
        );
        let hidden_handlers = build_query_handlers(stores.clone(), hidden_resolver);
        let hidden = hidden_handlers
            .handle_search_work(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: SearchWorkRequest {
                    project_ref: project_ref.clone(),
                    criteria: fixtures::work_search_criteria(),
                },
            })
            .await
            .expect("not visible remains query surface");
        assert_eq!(hidden.surface, QuerySurface::NotVisible);
        assert!(hidden.data.is_none());

        assert_eq!(stores.trace_count(), trace_before);
        assert_eq!(stores.stale_mark_count(), stale_before);
        assert_eq!(outbox.count(), outbox_before);
    }

    #[tokio::test]
    async fn tc_work_query_007_get_work_trace_page_empty_not_visible() {
        let (service, stores, outbox, project_ref, _member_ref, _work_ref) =
            prepare_query_context().await;
        let trace_before = stores.trace_count();
        let project_trace_before = stores
            .list_trace_records(
                WorkTraceSubjectRef::Project(project_ref.clone()),
                core_contracts::metadata::PageRequest {
                    limit: 10,
                    page_token: None,
                },
            )
            .await
            .expect("trace baseline should load")
            .items
            .len();
        let stale_before = stores.stale_mark_count();
        let outbox_before = outbox.count();

        let uow = stores.begin().await.expect("uow should begin");
        stores
            .append_trace(
                work_domain::WorkTraceRecord {
                    trace_id: fixtures::trace_id(),
                    subject_ref: WorkTraceSubjectRef::Project(project_ref.clone()),
                    trace_context_ref: fixtures::trace_context_ref(),
                },
                &uow,
            )
            .await
            .expect("trace should persist");
        stores.commit(uow).await.expect("uow should commit");

        let visible = service
            .get_work_trace(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: query_metadata_with_page(10, Some("page-1")),
                query: GetWorkTraceRequest {
                    subject_ref: WorkTraceSubjectRef::Project(project_ref.clone()),
                },
            })
            .await
            .expect("trace page should succeed");
        assert_eq!(visible.surface, QuerySurface::Visible);
        let trace_view = visible.data.expect("trace payload should exist");
        assert_eq!(
            trace_view.subject_ref,
            WorkTraceSubjectRef::Project(project_ref.clone())
        );
        assert_eq!(trace_view.records.len(), project_trace_before + 1);
        assert_eq!(
            trace_view
                .records
                .last()
                .expect("trace should exist")
                .trace_id,
            fixtures::trace_id()
        );

        let backlog_ref = stores
            .get_by_project(project_ref.clone())
            .await
            .expect("backlog should load")
            .expect("backlog should exist")
            .backlog_ref();
        let empty = service
            .get_work_trace(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetWorkTraceRequest {
                    subject_ref: WorkTraceSubjectRef::Backlog(backlog_ref),
                },
            })
            .await
            .expect("empty trace view should succeed");
        assert_eq!(empty.surface, QuerySurface::Empty);

        let hidden_resolver = FakeActorMemberResolverPort::new();
        hidden_resolver.seed(
            &fixtures::query_actor_context(),
            ActorMemberResolverOutcome::Rejected,
        );
        let hidden_handlers = build_query_handlers(stores.clone(), hidden_resolver);
        let hidden = hidden_handlers
            .handle_get_work_trace(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetWorkTraceRequest {
                    subject_ref: WorkTraceSubjectRef::Project(project_ref),
                },
            })
            .await
            .expect("hidden trace should remain query surface");
        assert_eq!(hidden.surface, QuerySurface::NotVisible);
        assert!(hidden.data.is_none());

        assert_eq!(stores.trace_count(), trace_before + 1);
        assert_eq!(stores.stale_mark_count(), stale_before);
        assert_eq!(outbox.count(), outbox_before);
    }

    #[tokio::test]
    async fn tc_work_query_008_get_project_board_view_board_and_rebuilding() {
        let (service, stores, outbox, project_ref, member_ref, work_ref) =
            prepare_query_context().await;
        let trace_before = stores.trace_count();
        let stale_before = stores.stale_mark_count();
        let outbox_before = outbox.count();

        stores.seed_project_board_public_view(work_contracts::views::ProjectBoardView {
            project_ref: project_ref.clone(),
            work_cards: vec![work_contracts::views::FormalWorkSummaryView {
                work_ref: work_ref.clone(),
                work_state: WorkItemState::Formalized,
                assignee_ref: Some(member_ref),
                completion_ref: None,
            }],
            marker: work_contracts::ProjectionViewMarker {
                view_ref: DerivedWorkViewRef::project_board(project_ref.clone()),
                source_cursor: fixtures::truth_cursor(),
                freshness_state: work_contracts::DerivedFreshnessState::Fresh,
            },
        });
        let handlers = build_query_handlers(stores.clone(), {
            let resolver = FakeActorMemberResolverPort::new();
            resolver.seed(
                &fixtures::query_actor_context(),
                ActorMemberResolverOutcome::Success(fixtures::global_member_ref()),
            );
            resolver
        });

        let visible = handlers
            .handle_get_project_board_view(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetProjectBoardViewRequest {
                    project_ref: project_ref.clone(),
                },
            })
            .await
            .expect("board view should succeed");
        assert_eq!(visible.surface, QuerySurface::Visible);
        assert_eq!(
            visible
                .data
                .expect("board payload should exist")
                .work_cards
                .len(),
            1
        );

        let rebuilding = service
            .get_project_board_view(WorkQueryEnvelope {
                actor: fixtures::query_actor_context(),
                metadata: fixtures::query_metadata(),
                query: GetProjectBoardViewRequest {
                    project_ref: work_contracts::ProjectRef {
                        project_id: work_contracts::ProjectId("project-board-missing".to_owned()),
                    },
                },
            })
            .await
            .expect("missing board should use query surface");
        assert_eq!(rebuilding.surface, QuerySurface::Missing);

        assert_eq!(stores.trace_count(), trace_before);
        assert_eq!(stores.stale_mark_count(), stale_before);
        assert_eq!(outbox.count(), outbox_before);
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
            EvidenceResolverOutcome::Success(work_contracts::EvidenceVerifiedState::Verified),
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
    async fn tc_work_dep_001_link_work_dependency_persists_truth_and_side_effects() {
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

        let upstream = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-001-upstream"),
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
            .expect("upstream create should succeed");
        let downstream = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-001-downstream"),
                command: CreateWorkItemRequest {
                    project_ref: created.project_ref.clone(),
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        title: fixtures::work_title("Downstream work"),
                        ..fixtures::formal_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect("downstream create should succeed");

        let result = handlers
            .handle_link_work_dependency(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-001"),
                command: LinkWorkDependencyRequest {
                    upstream_work_ref: upstream.work_ref.clone(),
                    downstream_work_ref: downstream.work_ref.clone(),
                    reason: fixtures::dependency_reason(),
                },
            })
            .await
            .expect("link should succeed");

        assert_eq!(
            result.dependency_state,
            work_contracts::DependencyState::Active
        );
        assert_eq!(result.receipt.idempotency, IdempotencyResultView::Applied);
        assert_eq!(result.receipt.outbox_record_refs.len(), 1);
        assert!(
            results
                .get_result(result.receipt.result_ref.clone())
                .await
                .expect("stored dependency result read should succeed")
                .is_some()
        );
        assert_eq!(stores.trace_count(), 5);
        assert_eq!(stores.stale_mark_count(), 5);
        assert_eq!(outbox.count(), 5);
        let stale_marks = stores.stale_marks();
        let stale = stale_marks.last().expect("stale mark should exist");
        assert_eq!(
            stale.0,
            vec![
                DerivedWorkViewRef::project_board(created.project_ref),
                DerivedWorkViewRef::member_work(assigned.project_member_ref),
            ]
        );
    }

    #[tokio::test]
    async fn tc_work_dep_002_cycle_reject_does_not_write_truth() {
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

        let first = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-002-first"),
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
            .expect("first work should succeed");
        let second = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-002-second"),
                command: CreateWorkItemRequest {
                    project_ref: created.project_ref.clone(),
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        title: fixtures::work_title("Second"),
                        ..fixtures::formal_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect("second work should succeed");

        handlers
            .handle_link_work_dependency(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-002-a"),
                command: LinkWorkDependencyRequest {
                    upstream_work_ref: first.work_ref.clone(),
                    downstream_work_ref: second.work_ref.clone(),
                    reason: fixtures::dependency_reason(),
                },
            })
            .await
            .expect("first link should succeed");

        let error = handlers
            .handle_link_work_dependency(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-002-b"),
                command: LinkWorkDependencyRequest {
                    upstream_work_ref: second.work_ref,
                    downstream_work_ref: first.work_ref,
                    reason: fixtures::dependency_reason(),
                },
            })
            .await
            .expect_err("cycle should be rejected");

        assert_eq!(error, WorkProtocolError::DomainRejected);
        assert_eq!(stores.trace_count(), 5);
        assert_eq!(stores.stale_mark_count(), 5);
        assert_eq!(outbox.count(), 5);
    }

    #[tokio::test]
    async fn tc_work_dep_003_update_dependency_state_requires_reason_and_evidence() {
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

        let upstream = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-003-upstream"),
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
            .expect("upstream create should succeed");
        let downstream = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-003-downstream"),
                command: CreateWorkItemRequest {
                    project_ref: created.project_ref,
                    work_intent: work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        title: fixtures::work_title("dep-downstream"),
                        ..fixtures::formal_work_intent()
                    },
                    source_ref: fixtures::source_work_ref(),
                },
            })
            .await
            .expect("downstream create should succeed");

        let linked = handlers
            .handle_link_work_dependency(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-003-link"),
                command: LinkWorkDependencyRequest {
                    upstream_work_ref: upstream.work_ref.clone(),
                    downstream_work_ref: downstream.work_ref.clone(),
                    reason: fixtures::dependency_reason(),
                },
            })
            .await
            .expect("link should succeed");

        let mismatch = handlers
            .handle_update_work_dependency_state(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-003-mismatch"),
                command: UpdateWorkDependencyStateRequest {
                    dependency_ref: linked.dependency_ref.clone(),
                    target: DependencyTarget::Waived,
                    reason: fixtures::dependency_activated_reason(),
                    evidence_ref: None,
                    expected_version: 1,
                },
            })
            .await
            .expect_err("reason-kind mismatch should fail");
        assert_eq!(mismatch, WorkProtocolError::InvalidRequest);

        evidence_refs.seed(
            fixtures::completion_evidence_ref(),
            EvidenceResolverOutcome::Success(work_contracts::EvidenceVerifiedState::Verified),
        );
        let satisfied = handlers
            .handle_update_work_dependency_state(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-003-satisfied"),
                command: UpdateWorkDependencyStateRequest {
                    dependency_ref: linked.dependency_ref.clone(),
                    target: DependencyTarget::Satisfied,
                    reason: fixtures::dependency_satisfied_reason(),
                    evidence_ref: Some(fixtures::completion_evidence_ref()),
                    expected_version: 1,
                },
            })
            .await
            .expect("satisfied should succeed");
        assert_eq!(
            satisfied.dependency_state,
            work_contracts::DependencyState::Satisfied
        );

        let reopen = handlers
            .handle_update_work_dependency_state(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-003-reopen"),
                command: UpdateWorkDependencyStateRequest {
                    dependency_ref: linked.dependency_ref,
                    target: DependencyTarget::Active,
                    reason: fixtures::dependency_activated_reason(),
                    evidence_ref: None,
                    expected_version: 2,
                },
            })
            .await
            .expect_err("terminal dependency must not reopen");
        assert_eq!(reopen, WorkProtocolError::DomainRejected);
        assert_eq!(stores.trace_count(), 6);
        assert_eq!(stores.stale_mark_count(), 6);
        assert_eq!(outbox.count(), 6);
    }

    #[tokio::test]
    async fn tc_work_dep_004_open_blocker_persists_truth_and_side_effects() {
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

        let work = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-004-work"),
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
            .expect("work create should succeed");

        let result = handlers
            .handle_open_work_blocker(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-004"),
                command: OpenWorkBlockerRequest {
                    blocked_work_ref: work.work_ref,
                    cause_ref: fixtures::blocker_cause_ref(),
                },
            })
            .await
            .expect("open blocker should succeed");

        assert_eq!(result.blocker_state, work_contracts::BlockerState::Open);
        assert!(
            results
                .get_result(result.receipt.result_ref.clone())
                .await
                .expect("stored blocker result read should succeed")
                .is_some()
        );
        assert_eq!(stores.trace_count(), 4);
        assert_eq!(stores.stale_mark_count(), 4);
        assert_eq!(outbox.count(), 4);
    }

    #[tokio::test]
    async fn tc_work_dep_005_resolve_blocker_requires_verified_evidence() {
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

        let work = handlers
            .handle_create_work_item(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-005-work"),
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
            .expect("work create should succeed");
        let opened = handlers
            .handle_open_work_blocker(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-005-open"),
                command: OpenWorkBlockerRequest {
                    blocked_work_ref: work.work_ref.clone(),
                    cause_ref: fixtures::blocker_cause_ref(),
                },
            })
            .await
            .expect("open blocker should succeed");

        let unresolved = handlers
            .handle_resolve_work_blocker(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-005-unresolved"),
                command: ResolveWorkBlockerRequest {
                    blocker_ref: opened.blocker_ref.clone(),
                    evidence_ref: fixtures::blocker_resolution_evidence_ref(),
                    expected_version: 1,
                },
            })
            .await
            .expect_err("missing evidence resolution should fail");
        assert_eq!(unresolved, WorkProtocolError::ExternalReferenceUnresolved);

        evidence_refs.seed(
            fixtures::blocker_resolution_evidence_ref(),
            EvidenceResolverOutcome::Success(work_contracts::EvidenceVerifiedState::Verified),
        );
        let resolved = handlers
            .handle_resolve_work_blocker(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-005-resolved"),
                command: ResolveWorkBlockerRequest {
                    blocker_ref: opened.blocker_ref.clone(),
                    evidence_ref: fixtures::blocker_resolution_evidence_ref(),
                    expected_version: 1,
                },
            })
            .await
            .expect("verified blocker evidence should succeed");
        assert_eq!(
            resolved.blocker_state,
            work_contracts::BlockerState::Resolved
        );

        let closed_again = handlers
            .handle_resolve_work_blocker(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-dep-005-retry"),
                command: ResolveWorkBlockerRequest {
                    blocker_ref: opened.blocker_ref,
                    evidence_ref: fixtures::blocker_resolution_evidence_ref(),
                    expected_version: 2,
                },
            })
            .await
            .expect_err("resolved blocker must not resolve again");
        assert_eq!(closed_again, WorkProtocolError::DomainRejected);
        assert_eq!(stores.trace_count(), 5);
        assert_eq!(stores.stale_mark_count(), 5);
        assert_eq!(outbox.count(), 5);
    }

    #[tokio::test]
    async fn tc_work_promote_001_request_promotion_persists_result_trace_and_outbox() {
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
        let (_created, _assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: false,
            },
        );

        let result = handlers
            .handle_request_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-001"),
                command: RequestWorkPromotionRequest {
                    source_ref: fixtures::source_work_ref(),
                    reason: fixtures::promote_reason(),
                },
            })
            .await
            .expect("request promotion should succeed");

        assert_eq!(result.result_state, PromoteResultState::PendingReview);
        assert_eq!(result.created_work_ref, None);
        assert_eq!(result.receipt.idempotency, IdempotencyResultView::Applied);
        assert_eq!(stores.trace_count(), 3);
        assert_eq!(stores.stale_mark_count(), 2);
        assert_eq!(outbox.count(), 3);
        assert!(
            results
                .get_result(result.receipt.result_ref.clone())
                .await
                .expect("stored result read should succeed")
                .is_some()
        );
        let (stored, version) = stores
            .promote_result_snapshot(&result.promote_result_ref)
            .expect("promote result should be stored");
        assert_eq!(stored.result_state, PromoteResultState::PendingReview);
        assert_eq!(version, 1);
    }

    #[tokio::test]
    async fn tc_work_promote_002_review_accept_creates_formal_work() {
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
        let requested = handlers
            .handle_request_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-002-request"),
                command: RequestWorkPromotionRequest {
                    source_ref: fixtures::source_work_ref(),
                    reason: fixtures::promote_reason(),
                },
            })
            .await
            .expect("request promotion should succeed");

        let reviewed = handlers
            .handle_review_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-002-review"),
                command: ReviewWorkPromotionRequest {
                    promote_result_ref: requested.promote_result_ref.clone(),
                    decision: PromoteReviewDecision::Accept,
                    accepted_work_intent: Some(work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        ..fixtures::formal_work_intent()
                    }),
                    expected_version: 1,
                },
            })
            .await
            .expect("review accept should succeed");

        assert_eq!(reviewed.result_state, PromoteResultState::Accepted);
        assert_eq!(stores.trace_count(), 5);
        assert_eq!(stores.stale_mark_count(), 3);
        let created_work_ref = reviewed
            .created_work_ref
            .clone()
            .expect("accept should create work");
        let (promote, promote_version) = stores
            .promote_result_snapshot(&requested.promote_result_ref)
            .expect("promote result should remain stored");
        assert_eq!(promote.result_state, PromoteResultState::Accepted);
        assert_eq!(promote_version, 2);
        let backlog = stores
            .get_by_project(created.project_ref.clone())
            .await
            .expect("backlog lookup should succeed")
            .expect("backlog should exist");
        assert!(
            stores
                .backlog_membership(&backlog.backlog_ref())
                .contains(&created_work_ref)
        );
        let (work_item, _version) = stores
            .work_item_snapshot(&created_work_ref)
            .expect("accepted promotion should create root work item");
        assert_eq!(work_item.assignee_ref, assigned.project_member_ref);
        let stale_marks = stores.stale_marks();
        let (affected_views, _) = stale_marks
            .last()
            .expect("accept should append one stale marker write");
        assert_eq!(
            affected_views,
            &vec![
                DerivedWorkViewRef::project_board(created.project_ref.clone()),
                DerivedWorkViewRef::member_work(assigned.project_member_ref.clone()),
            ]
        );
        assert_eq!(stores.promote_decisions().len(), 1);
        assert_eq!(outbox.count(), 5);
    }

    #[tokio::test]
    async fn tc_work_promote_003_review_reject_records_decision_without_work() {
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
        let requested = handlers
            .handle_request_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-003-request"),
                command: RequestWorkPromotionRequest {
                    source_ref: fixtures::source_work_ref(),
                    reason: fixtures::promote_reason(),
                },
            })
            .await
            .expect("request promotion should succeed");

        let invalid = handlers
            .handle_review_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-003-invalid-reject"),
                command: ReviewWorkPromotionRequest {
                    promote_result_ref: requested.promote_result_ref.clone(),
                    decision: PromoteReviewDecision::Reject(fixtures::promote_reject_reason()),
                    accepted_work_intent: Some(work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        ..fixtures::formal_work_intent()
                    }),
                    expected_version: 1,
                },
            })
            .await
            .expect_err("reject with accepted intent should be invalid");
        assert_eq!(invalid, WorkProtocolError::InvalidRequest);
        assert_eq!(stores.trace_count(), 3);
        assert_eq!(stores.stale_mark_count(), 2);
        assert_eq!(stores.promote_decisions().len(), 0);
        assert_eq!(outbox.count(), 3);

        let reviewed = handlers
            .handle_review_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-003-review"),
                command: ReviewWorkPromotionRequest {
                    promote_result_ref: requested.promote_result_ref.clone(),
                    decision: PromoteReviewDecision::Reject(fixtures::promote_reject_reason()),
                    accepted_work_intent: None,
                    expected_version: 1,
                },
            })
            .await
            .expect("review reject should succeed");

        assert_eq!(reviewed.result_state, PromoteResultState::Rejected);
        assert_eq!(reviewed.created_work_ref, None);
        assert_eq!(stores.trace_count(), 4);
        assert_eq!(stores.stale_mark_count(), 2);
        assert_eq!(stores.promote_decisions().len(), 1);
        let backlog = stores
            .get_by_project(created.project_ref)
            .await
            .expect("backlog lookup should succeed")
            .expect("backlog should exist");
        assert!(stores.backlog_membership(&backlog.backlog_ref()).is_empty());
        assert_eq!(outbox.count(), 4);
    }

    #[tokio::test]
    async fn tc_work_promote_004_runtime_body_rejects_and_runtime_intake_fixture_stays_marker_only()
    {
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
        let (_created, _assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: true,
            },
        );

        let error = handlers
            .handle_request_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-004"),
                command: RequestWorkPromotionRequest {
                    source_ref: fixtures::source_work_ref(),
                    reason: fixtures::promote_reason(),
                },
            })
            .await
            .expect_err("body leak should be rejected");
        assert_eq!(error, WorkProtocolError::DomainRejected);
        assert_eq!(stores.promote_decisions().len(), 0);
        assert!(stores.pending_promote_intakes().is_empty());
        assert_eq!(outbox.count(), 2);

        let intake = work_application::PendingPromoteIntake::from_runtime_event(
            fixtures::runtime_source_work_ref(),
            fixtures::promote_reason(),
            fixtures::source_event_id(),
        )
        .expect("runtime intake fixture should build");
        assert_eq!(intake.source_ref, fixtures::runtime_source_work_ref());
    }

    #[tokio::test]
    async fn tc_work_promote_005_review_version_conflict_has_single_winner() {
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
        let (_created, assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        source_refs.seed(
            fixtures::source_work_ref(),
            SourceResolverOutcome::Success {
                has_external_body: false,
            },
        );
        let requested = handlers
            .handle_request_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-005-request"),
                command: RequestWorkPromotionRequest {
                    source_ref: fixtures::source_work_ref(),
                    reason: fixtures::promote_reason(),
                },
            })
            .await
            .expect("request promotion should succeed");

        let accepted = handlers
            .handle_review_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-005-accept"),
                command: ReviewWorkPromotionRequest {
                    promote_result_ref: requested.promote_result_ref.clone(),
                    decision: PromoteReviewDecision::Accept,
                    accepted_work_intent: Some(work_contracts::FormalWorkIntent {
                        assignee_ref: assigned.project_member_ref.clone(),
                        ..fixtures::formal_work_intent()
                    }),
                    expected_version: 1,
                },
            })
            .await
            .expect("first review should succeed");
        assert_eq!(accepted.result_state, PromoteResultState::Accepted);

        let loser = handlers
            .handle_review_work_promotion(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-promote-005-reject"),
                command: ReviewWorkPromotionRequest {
                    promote_result_ref: requested.promote_result_ref.clone(),
                    decision: PromoteReviewDecision::Reject(fixtures::promote_reject_reason()),
                    accepted_work_intent: None,
                    expected_version: 1,
                },
            })
            .await
            .expect_err("stale review must lose with version conflict");
        assert_eq!(loser, WorkProtocolError::VersionConflict);
        assert_eq!(stores.stale_mark_count(), 3);
        assert_eq!(stores.promote_decisions().len(), 1);
        assert_eq!(outbox.count(), 5);
    }

    #[tokio::test]
    async fn tc_work_iter_001_open_iteration_validates_process_timebox_summary_and_duplicate() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
            process_refs,
        ) = build_handlers_with_process();
        let created = create_project(&handlers, "idem-iter-001-project").await;
        process_refs.seed(
            fixtures::process_timebox_ref(),
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref: created.project_ref.clone(),
                can_open_iteration: true,
                summary: Some(fixtures::safe_summary("iteration window")),
                source_digest: Some(work_contracts::SourceDigest("timebox-digest-1".to_owned())),
            },
        );

        let envelope = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-iter-001-open"),
            command: OpenIterationRequest {
                project_ref: created.project_ref.clone(),
                timebox_ref: fixtures::process_timebox_ref(),
            },
        };
        let opened = handlers
            .handle_open_iteration(envelope.clone())
            .await
            .expect("open iteration should succeed");
        assert_eq!(opened.iteration_state, IterationState::Planning);
        assert_eq!(opened.commitment_state, None);
        assert_eq!(opened.receipt.idempotency, IdempotencyResultView::Applied);

        let duplicate = handlers
            .handle_open_iteration(envelope)
            .await
            .expect("duplicate open should replay stored result");
        assert_eq!(duplicate.iteration_ref, opened.iteration_ref);
        assert_eq!(duplicate.receipt.result_ref, opened.receipt.result_ref);
        assert_eq!(
            duplicate.receipt.idempotency,
            IdempotencyResultView::Duplicate
        );

        process_refs.seed(
            work_contracts::ProcessTimeboxRef("process/timeboxes/mismatch".to_owned()),
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref: work_contracts::ProjectRef {
                    project_id: work_contracts::ProjectId("project-mismatch".to_owned()),
                },
                can_open_iteration: true,
                summary: Some(fixtures::safe_summary("mismatch")),
                source_digest: Some(work_contracts::SourceDigest("timebox-digest-2".to_owned())),
            },
        );
        let mismatch = handlers
            .handle_open_iteration(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-001-mismatch"),
                command: OpenIterationRequest {
                    project_ref: created.project_ref.clone(),
                    timebox_ref: work_contracts::ProcessTimeboxRef(
                        "process/timeboxes/mismatch".to_owned(),
                    ),
                },
            })
            .await
            .expect_err("project mismatch should fail");
        assert_eq!(mismatch, WorkProtocolError::ExternalReferenceUnresolved);

        process_refs.seed(
            work_contracts::ProcessTimeboxRef("process/timeboxes/no-open".to_owned()),
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref: created.project_ref.clone(),
                can_open_iteration: false,
                summary: Some(fixtures::safe_summary("closed window")),
                source_digest: Some(work_contracts::SourceDigest("timebox-digest-3".to_owned())),
            },
        );
        let cannot_open = handlers
            .handle_open_iteration(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-001-no-open"),
                command: OpenIterationRequest {
                    project_ref: created.project_ref.clone(),
                    timebox_ref: work_contracts::ProcessTimeboxRef(
                        "process/timeboxes/no-open".to_owned(),
                    ),
                },
            })
            .await
            .expect_err("timebox gate should fail");
        assert_eq!(cannot_open, WorkProtocolError::ExternalReferenceUnresolved);

        process_refs.seed(
            work_contracts::ProcessTimeboxRef("process/timeboxes/no-digest".to_owned()),
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref: created.project_ref.clone(),
                can_open_iteration: true,
                summary: Some(fixtures::safe_summary("missing digest")),
                source_digest: None,
            },
        );
        let missing_digest = handlers
            .handle_open_iteration(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-001-no-digest"),
                command: OpenIterationRequest {
                    project_ref: created.project_ref.clone(),
                    timebox_ref: work_contracts::ProcessTimeboxRef(
                        "process/timeboxes/no-digest".to_owned(),
                    ),
                },
            })
            .await
            .expect_err("missing digest should fail");
        assert_eq!(
            missing_digest,
            WorkProtocolError::ExternalReferenceUnresolved
        );

        assert_eq!(stores.trace_count(), 2);
        assert_eq!(stores.stale_mark_count(), 2);
        assert_eq!(outbox.count(), 2);
    }

    #[tokio::test]
    async fn tc_work_iter_002_commit_scope_marks_root_and_child_work_committed() {
        let (
            handlers,
            stores,
            outbox,
            results,
            _idempotency,
            member_refs,
            source_refs,
            _evidence_refs,
            process_refs,
        ) = build_handlers_with_process();
        let (created, assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        let root = create_root_work(
            &handlers,
            created.project_ref.clone(),
            assigned.project_member_ref.clone(),
            &source_refs,
            "idem-iter-002-root",
            "Root work",
        )
        .await;
        let child = create_child_work(
            &handlers,
            root.work_ref.clone(),
            assigned.project_member_ref.clone(),
            &source_refs,
            "idem-iter-002-child",
        )
        .await;
        process_refs.seed(
            fixtures::process_timebox_ref(),
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref: created.project_ref.clone(),
                can_open_iteration: true,
                summary: Some(fixtures::safe_summary("commit scope timebox")),
                source_digest: Some(work_contracts::SourceDigest("timebox-digest-4".to_owned())),
            },
        );
        let opened = handlers
            .handle_open_iteration(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-002-open"),
                command: OpenIterationRequest {
                    project_ref: created.project_ref.clone(),
                    timebox_ref: fixtures::process_timebox_ref(),
                },
            })
            .await
            .expect("open iteration should succeed");

        let committed = handlers
            .handle_commit_iteration_scope(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-002-commit"),
                command: CommitIterationScopeRequest {
                    iteration_ref: opened.iteration_ref.clone(),
                    candidate_work_refs: work_contracts::FormalWorkRefSet {
                        refs: vec![root.work_ref.clone(), child.work_ref.clone()],
                    },
                    expected_iteration_version: 1,
                },
            })
            .await
            .expect("commit iteration scope should succeed");
        assert_eq!(committed.iteration_state, IterationState::Committed);
        assert_eq!(committed.commitment_state, Some(CommitmentState::Committed));
        assert!(
            results
                .get_result(committed.receipt.result_ref.clone())
                .await
                .expect("stored iteration result should load")
                .is_some()
        );

        let (stored_iteration, iteration_version) = stores
            .iteration_snapshot(&opened.iteration_ref)
            .expect("iteration should be stored");
        assert_eq!(stored_iteration.iteration_state, IterationState::Committed);
        assert_eq!(iteration_version, 2);
        let (stored_commitment, commitment_version) = stores
            .commitment_snapshot(&opened.iteration_ref)
            .expect("commitment should be stored");
        assert_eq!(
            stored_commitment.commitment_state,
            CommitmentState::Committed
        );
        assert_eq!(commitment_version, 1);
        let (stored_root, _) = stores
            .work_item_snapshot(&root.work_ref)
            .expect("root should be stored");
        assert_eq!(stored_root.work_state, WorkItemState::Committed);
        let (stored_child, _) = stores
            .child_work_item_snapshot(&child.work_ref)
            .expect("child should be stored");
        assert_eq!(stored_child.work_state, WorkItemState::Committed);

        let stale = stores
            .stale_marks()
            .last()
            .expect("commit stale mark should exist")
            .0
            .clone();
        assert_eq!(
            stale,
            vec![
                DerivedWorkViewRef::project_board(created.project_ref),
                DerivedWorkViewRef::iteration_summary(opened.iteration_ref),
                DerivedWorkViewRef::member_work(assigned.project_member_ref),
            ]
        );
        assert_eq!(stores.trace_count(), 6);
        assert_eq!(stores.stale_mark_count(), 6);
        assert_eq!(outbox.count(), 6);
    }

    #[tokio::test]
    async fn tc_work_iter_003_update_commitment_rejects_non_member_work_and_updates_state() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            member_refs,
            source_refs,
            _evidence_refs,
            process_refs,
        ) = build_handlers_with_process();
        let (created, assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        let root = create_root_work(
            &handlers,
            created.project_ref.clone(),
            assigned.project_member_ref.clone(),
            &source_refs,
            "idem-iter-003-root",
            "Root work",
        )
        .await;
        let child = create_child_work(
            &handlers,
            root.work_ref.clone(),
            assigned.project_member_ref.clone(),
            &source_refs,
            "idem-iter-003-child",
        )
        .await;
        process_refs.seed(
            fixtures::process_timebox_ref(),
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref: created.project_ref.clone(),
                can_open_iteration: true,
                summary: Some(fixtures::safe_summary("update commitment timebox")),
                source_digest: Some(work_contracts::SourceDigest("timebox-digest-5".to_owned())),
            },
        );
        let opened = handlers
            .handle_open_iteration(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-003-open"),
                command: OpenIterationRequest {
                    project_ref: created.project_ref.clone(),
                    timebox_ref: fixtures::process_timebox_ref(),
                },
            })
            .await
            .expect("open iteration should succeed");
        handlers
            .handle_commit_iteration_scope(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-003-commit"),
                command: CommitIterationScopeRequest {
                    iteration_ref: opened.iteration_ref.clone(),
                    candidate_work_refs: work_contracts::FormalWorkRefSet {
                        refs: vec![root.work_ref.clone()],
                    },
                    expected_iteration_version: 1,
                },
            })
            .await
            .expect("commit scope should succeed");

        let changed = handlers
            .handle_update_iteration_commitment(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-003-change"),
                command: UpdateIterationCommitmentRequest {
                    iteration_ref: opened.iteration_ref.clone(),
                    change_set: work_contracts::IterationCommitmentChangeSet {
                        add_work_refs: vec![child.work_ref.clone()],
                        remove_work_refs: vec![root.work_ref.clone()],
                    },
                    reason: fixtures::iteration_commitment_changed_reason(),
                    expected_commitment_version: 1,
                },
            })
            .await
            .expect("commitment change should succeed");
        assert_eq!(changed.iteration_state, IterationState::Committed);
        assert_eq!(changed.commitment_state, Some(CommitmentState::Changed));
        let (stored_commitment, version) = stores
            .commitment_snapshot(&opened.iteration_ref)
            .expect("commitment should be stored");
        assert_eq!(stored_commitment.commitment_state, CommitmentState::Changed);
        assert_eq!(version, 2);
        assert_eq!(
            stored_commitment.committed_work_refs.refs,
            vec![child.work_ref.clone()]
        );
        let stale = stores
            .stale_marks()
            .last()
            .expect("commitment stale mark should exist")
            .0
            .clone();
        assert_eq!(
            stale,
            vec![
                DerivedWorkViewRef::project_board(created.project_ref.clone()),
                DerivedWorkViewRef::iteration_summary(opened.iteration_ref.clone()),
                DerivedWorkViewRef::member_work(assigned.project_member_ref.clone()),
            ]
        );

        let invalid = handlers
            .handle_update_iteration_commitment(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-003-invalid"),
                command: UpdateIterationCommitmentRequest {
                    iteration_ref: opened.iteration_ref,
                    change_set: work_contracts::IterationCommitmentChangeSet {
                        add_work_refs: vec![work_contracts::FormalWorkRef::WorkItem(
                            work_contracts::WorkItemId("work-item-missing".to_owned()),
                        )],
                        remove_work_refs: Vec::new(),
                    },
                    reason: fixtures::iteration_commitment_changed_reason(),
                    expected_commitment_version: 2,
                },
            })
            .await
            .expect_err("non-member work should fail");
        assert_eq!(invalid, WorkProtocolError::DomainRejected);
        assert_eq!(stores.trace_count(), 7);
        assert_eq!(stores.stale_mark_count(), 7);
        assert_eq!(outbox.count(), 7);
    }

    #[tokio::test]
    async fn tc_work_iter_004_lifecycle_validates_reason_shape_and_closes_commitment() {
        let (
            handlers,
            stores,
            outbox,
            _results,
            _idempotency,
            member_refs,
            source_refs,
            _evidence_refs,
            process_refs,
        ) = build_handlers_with_process();
        let (created, assigned) = prepare_formal_work_context(&handlers, &member_refs).await;
        let root = create_root_work(
            &handlers,
            created.project_ref.clone(),
            assigned.project_member_ref.clone(),
            &source_refs,
            "idem-iter-004-root",
            "Lifecycle root",
        )
        .await;
        process_refs.seed(
            fixtures::process_timebox_ref(),
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref: created.project_ref.clone(),
                can_open_iteration: true,
                summary: Some(fixtures::safe_summary("lifecycle timebox")),
                source_digest: Some(work_contracts::SourceDigest("timebox-digest-6".to_owned())),
            },
        );
        let opened = handlers
            .handle_open_iteration(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-004-open"),
                command: OpenIterationRequest {
                    project_ref: created.project_ref.clone(),
                    timebox_ref: fixtures::process_timebox_ref(),
                },
            })
            .await
            .expect("open iteration should succeed");
        handlers
            .handle_commit_iteration_scope(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-004-commit"),
                command: CommitIterationScopeRequest {
                    iteration_ref: opened.iteration_ref.clone(),
                    candidate_work_refs: work_contracts::FormalWorkRefSet {
                        refs: vec![root.work_ref],
                    },
                    expected_iteration_version: 1,
                },
            })
            .await
            .expect("commit scope should succeed");

        let wrong_reason = handlers
            .handle_update_iteration_lifecycle(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-004-wrong-reason"),
                command: UpdateIterationLifecycleRequest {
                    iteration_ref: opened.iteration_ref.clone(),
                    target: IterationLifecycleTarget::InProgress,
                    change_reason: None,
                    close_reason: Some(fixtures::iteration_closed_reason()),
                    expected_version: 2,
                },
            })
            .await
            .expect_err("wrong reason shape should fail");
        assert_eq!(wrong_reason, WorkProtocolError::InvalidRequest);

        let started = handlers
            .handle_update_iteration_lifecycle(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-004-start"),
                command: UpdateIterationLifecycleRequest {
                    iteration_ref: opened.iteration_ref.clone(),
                    target: IterationLifecycleTarget::InProgress,
                    change_reason: Some(fixtures::iteration_started_reason()),
                    close_reason: None,
                    expected_version: 2,
                },
            })
            .await
            .expect("start should succeed");
        assert_eq!(started.iteration_state, IterationState::InProgress);
        assert_eq!(started.commitment_state, Some(CommitmentState::Committed));
        let start_stale = stores
            .stale_marks()
            .last()
            .expect("start stale mark should exist")
            .0
            .clone();
        assert_eq!(
            start_stale,
            vec![
                DerivedWorkViewRef::project_board(created.project_ref.clone()),
                DerivedWorkViewRef::iteration_summary(opened.iteration_ref.clone()),
                DerivedWorkViewRef::member_work(assigned.project_member_ref.clone()),
            ]
        );

        let closed = handlers
            .handle_update_iteration_lifecycle(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-004-close"),
                command: UpdateIterationLifecycleRequest {
                    iteration_ref: opened.iteration_ref.clone(),
                    target: IterationLifecycleTarget::Closed,
                    change_reason: None,
                    close_reason: Some(fixtures::iteration_closed_reason()),
                    expected_version: 3,
                },
            })
            .await
            .expect("close should succeed");
        assert_eq!(closed.iteration_state, IterationState::Closed);
        assert_eq!(closed.commitment_state, Some(CommitmentState::Closed));
        let close_stale = stores
            .stale_marks()
            .last()
            .expect("close stale mark should exist")
            .0
            .clone();
        assert_eq!(
            close_stale,
            vec![
                DerivedWorkViewRef::project_board(created.project_ref.clone()),
                DerivedWorkViewRef::iteration_summary(opened.iteration_ref.clone()),
                DerivedWorkViewRef::member_work(assigned.project_member_ref),
            ]
        );
        assert_eq!(stores.iteration_changes().len(), 1);
        let (stored_commitment, commitment_version) = stores
            .commitment_snapshot(&opened.iteration_ref)
            .expect("closed commitment should be stored");
        assert_eq!(stored_commitment.commitment_state, CommitmentState::Closed);
        assert_eq!(commitment_version, 2);
        let (stored_iteration, iteration_version) = stores
            .iteration_snapshot(&opened.iteration_ref)
            .expect("closed iteration should be stored");
        assert_eq!(stored_iteration.iteration_state, IterationState::Closed);
        assert_eq!(iteration_version, 4);

        let cancelled = handlers
            .handle_update_iteration_lifecycle(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-004-cancel"),
                command: UpdateIterationLifecycleRequest {
                    iteration_ref: handlers
                        .handle_open_iteration(WorkCommandEnvelope {
                            actor: fixtures::actor_context(),
                            metadata: fixtures::command_metadata("idem-iter-004-open-cancel"),
                            command: OpenIterationRequest {
                                project_ref: created.project_ref.clone(),
                                timebox_ref: fixtures::process_timebox_ref(),
                            },
                        })
                        .await
                        .expect("second open iteration should succeed")
                        .iteration_ref,
                    target: IterationLifecycleTarget::Cancelled,
                    change_reason: Some(fixtures::iteration_cancelled_reason()),
                    close_reason: None,
                    expected_version: 1,
                },
            })
            .await
            .expect("cancel should succeed");
        assert_eq!(cancelled.iteration_state, IterationState::Cancelled);
        assert_eq!(cancelled.commitment_state, None);

        let reopen = handlers
            .handle_update_iteration_lifecycle(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-004-reopen"),
                command: UpdateIterationLifecycleRequest {
                    iteration_ref: opened.iteration_ref.clone(),
                    target: IterationLifecycleTarget::InProgress,
                    change_reason: Some(fixtures::iteration_started_reason()),
                    close_reason: None,
                    expected_version: 4,
                },
            })
            .await
            .expect_err("closed iteration must not reopen");
        assert_eq!(reopen, WorkProtocolError::DomainRejected);
        assert_eq!(stores.trace_count(), 9);
        assert_eq!(stores.stale_mark_count(), 9);
        assert_eq!(outbox.count(), 9);
    }

    #[tokio::test]
    async fn tc_work_iter_005_start_requires_existing_commitment() {
        let (
            handlers,
            _stores,
            outbox,
            _results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
            process_refs,
        ) = build_handlers_with_process();
        let created = create_project(&handlers, "idem-iter-005-project").await;
        process_refs.seed(
            fixtures::process_timebox_ref(),
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref: created.project_ref.clone(),
                can_open_iteration: true,
                summary: Some(fixtures::safe_summary("start requires commitment")),
                source_digest: Some(work_contracts::SourceDigest("timebox-digest-8".to_owned())),
            },
        );
        let opened = handlers
            .handle_open_iteration(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-005-open"),
                command: OpenIterationRequest {
                    project_ref: created.project_ref,
                    timebox_ref: fixtures::process_timebox_ref(),
                },
            })
            .await
            .expect("open iteration should succeed");

        let error = handlers
            .handle_update_iteration_lifecycle(WorkCommandEnvelope {
                actor: fixtures::actor_context(),
                metadata: fixtures::command_metadata("idem-iter-005-start"),
                command: UpdateIterationLifecycleRequest {
                    iteration_ref: opened.iteration_ref,
                    target: IterationLifecycleTarget::InProgress,
                    change_reason: Some(fixtures::iteration_started_reason()),
                    close_reason: None,
                    expected_version: 1,
                },
            })
            .await
            .expect_err("start without commitment should fail");
        assert_eq!(error, WorkProtocolError::DomainRejected);
        assert_eq!(outbox.count(), 2);
    }

    #[tokio::test]
    async fn tc_work_iter_006_duplicate_replay_and_missing_result_surface() {
        let (
            handlers,
            stores,
            outbox,
            results,
            _idempotency,
            _member_refs,
            _source_refs,
            _evidence_refs,
            process_refs,
        ) = build_handlers_with_process();
        let created = create_project(&handlers, "idem-iter-005-project").await;
        process_refs.seed(
            fixtures::process_timebox_ref(),
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref: created.project_ref.clone(),
                can_open_iteration: true,
                summary: Some(fixtures::safe_summary("duplicate timebox")),
                source_digest: Some(work_contracts::SourceDigest("timebox-digest-7".to_owned())),
            },
        );

        let envelope = WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-iter-005-open"),
            command: OpenIterationRequest {
                project_ref: created.project_ref,
                timebox_ref: fixtures::process_timebox_ref(),
            },
        };
        let first = handlers
            .handle_open_iteration(envelope.clone())
            .await
            .expect("first open should succeed");
        results.inject_missing(first.receipt.result_ref.clone());

        let error = handlers
            .handle_open_iteration(envelope)
            .await
            .expect_err("missing stored result should fail duplicate replay");
        assert_eq!(error, WorkProtocolError::TemporarilyUnavailable);
        assert_eq!(stores.trace_count(), 2);
        assert_eq!(stores.stale_mark_count(), 2);
        assert_eq!(outbox.count(), 2);
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
