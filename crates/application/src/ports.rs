//! Repository and adapter traits consumed by Work application services.

use async_trait::async_trait;
use core_contracts::metadata::{PageRequest, PageToken, Timestamp, Version};
use serde::{Deserialize, Serialize};

use crate::UnitOfWorkHandle;
use work_contracts::{
    BacklogRef, DependencyOrBlockerRef, DerivedWorkViewRef, ExternalEvidenceRef,
    ExternalSourceSummary, FormalWorkRef, GlobalMemberRef, OutboxFailureReason,
    OutboxPublicationRef, OutboxRetryReason, ProjectMemberId, ProjectMemberRef, ProjectOwnerRef,
    ProjectRef, PromoteResultRef, SourceWorkRef, WorkAuditSubjectRef, WorkBlockerId,
    WorkBlockerRef, WorkDependencyId, WorkDependencyRef, WorkOutboxId, WorkTruthCursor,
};
use work_domain::{
    Backlog, ChildWorkItem, DependencyChangeRecord, DependencyGraphSnapshot,
    MemberCapabilitySnapshot, PendingPromoteIntake, ProjectMember, PromoteDecisionRecord,
    PromoteResult, TraceHandoffMarker, WorkAuditTrail, WorkBlocker, WorkDependency, WorkItem,
    WorkOutboxRecord, WorkTraceRecord,
};

/// A repository page returned before public query mapping.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    /// Items returned by repository read.
    pub items: Vec<T>,
    /// Cursor metadata for the next repository read.
    pub page_info: PageInfo,
}

/// Cursor metadata for repository page reads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PageInfo {
    /// Token for the next page.
    pub next_page_token: Option<PageToken>,
    /// Whether more repository items may exist.
    pub has_more: bool,
}

/// Classifies persistence and local store failures before service mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryError {
    /// The requested record does not exist.
    NotFound,
    /// The expected optimistic version did not match the stored version.
    VersionConflict,
    /// The local transaction boundary rejected the operation.
    TransactionRejected,
    /// The store is unavailable or failed for a technical reason.
    StoreUnavailable,
}

/// Classifies external resolver, publisher, and handoff failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortError {
    /// The referenced external object could not be resolved.
    NotFound,
    /// The external reference exists but cannot be used by this operation.
    Rejected,
    /// The external dependency is temporarily unavailable.
    Unavailable,
    /// The external dependency returned an invalid or unsupported response.
    InvalidResponse,
}

/// Safe resolver input used to build a member capability snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemberCapabilitySnapshotInput {
    /// Referenced identity member.
    pub member_ref: GlobalMemberRef,
    /// Safe capability references returned by the identity boundary.
    pub capability_refs: work_contracts::CapabilityRefSet,
}

/// Safe source summary returned by the source resolver.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceWorkResolution {
    /// Stable source reference accepted by the resolver.
    pub source_ref: SourceWorkRef,
    /// Safe summary for forbidden-body checks.
    pub summary: ExternalSourceSummary,
}

/// Safe evidence summary returned by the evidence resolver.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceResolution {
    /// Stable evidence reference accepted by the resolver.
    pub evidence_ref: ExternalEvidenceRef,
    /// Verified state returned by the resolver.
    pub verified_state: work_contracts::EvidenceVerifiedState,
    /// Reference-resolution state returned by the resolver.
    pub reference_state: work_domain::ReferenceResolutionState,
}

/// Repository-loaded formal work truth.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormalWorkRecord {
    /// A root work item record.
    WorkItem(WorkItem),
    /// A child work item record.
    ChildWorkItem(ChildWorkItem),
}

impl FormalWorkRecord {
    /// Returns the stable formal work reference for this record.
    pub fn formal_work_ref(&self) -> FormalWorkRef {
        match self {
            Self::WorkItem(work_item) => work_item.formal_work_ref(),
            Self::ChildWorkItem(child) => child.formal_work_ref(),
        }
    }

    /// Returns the root work item id when this record is a root work item.
    pub fn as_root_work_item_id(&self) -> Option<work_contracts::WorkItemId> {
        match self {
            Self::WorkItem(work_item) => Some(work_item.work_item_id.clone()),
            Self::ChildWorkItem(_) => None,
        }
    }

    /// Returns the current work state regardless of variant.
    pub fn work_state(&self) -> work_contracts::WorkItemState {
        match self {
            Self::WorkItem(work_item) => work_item.work_state,
            Self::ChildWorkItem(child) => child.work_state,
        }
    }
}

/// Repository-resolved scope for one formal work record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormalWorkScope {
    /// Stable formal work reference for the resolved scope.
    pub work_ref: FormalWorkRef,
    /// Project that owns the formal work.
    pub project_ref: ProjectRef,
    /// Backlog that owns the formal work.
    pub backlog_ref: BacklogRef,
    /// Assignee whose member-work view should be marked stale when known.
    pub assignee_ref: Option<ProjectMemberRef>,
}

/// Stores Work-owned project truth.
#[async_trait]
pub trait ProjectRepository: Send + Sync {
    /// Loads a project by stable Work identity.
    async fn get(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<work_domain::Project>, RepositoryError>;

    /// Lists projects owned by the same external owner.
    async fn list_by_owner(
        &self,
        owner_ref: ProjectOwnerRef,
        page: PageRequest,
    ) -> Result<Page<work_domain::Project>, RepositoryError>;

    /// Creates a project inside the current unit of work.
    async fn create(
        &self,
        project: work_domain::Project,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves a project lifecycle change inside the current unit of work.
    async fn save(
        &self,
        project: work_domain::Project,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;
}

/// Stores backlog truth and formal work membership.
#[async_trait]
pub trait BacklogRepository: Send + Sync {
    /// Loads a backlog by Work identity.
    async fn get(&self, backlog_ref: BacklogRef) -> Result<Option<Backlog>, RepositoryError>;

    /// Loads the backlog that owns a project.
    async fn get_by_project(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<Backlog>, RepositoryError>;

    /// Loads the backlog and its optimistic version for one project.
    async fn get_by_project_with_version(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<(Backlog, Version)>, RepositoryError>;

    /// Checks whether one formal work ref belongs to the backlog.
    async fn contains_formal_work(
        &self,
        backlog_ref: BacklogRef,
        work_ref: FormalWorkRef,
    ) -> Result<bool, RepositoryError>;

    /// Creates a backlog inside the current unit of work.
    async fn create(
        &self,
        backlog: Backlog,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves backlog availability changes inside the current unit of work.
    async fn save(
        &self,
        backlog: Backlog,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Adds one formal work membership inside the current unit of work.
    async fn add_formal_work(
        &self,
        backlog_ref: BacklogRef,
        work_ref: FormalWorkRef,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;
}

/// Stores project-local member responsibility truth.
#[async_trait]
pub trait ProjectMemberRepository: Send + Sync {
    /// Loads one project member responsibility by Work identity.
    async fn get(
        &self,
        member_ref: ProjectMemberRef,
    ) -> Result<Option<ProjectMember>, RepositoryError>;

    /// Loads one project-scoped responsibility for the identity member.
    async fn get_by_member(
        &self,
        project_ref: ProjectRef,
        member_ref: GlobalMemberRef,
    ) -> Result<Option<ProjectMember>, RepositoryError>;

    /// Lists project-scoped member responsibilities.
    async fn list_by_project(
        &self,
        project_ref: ProjectRef,
        page: PageRequest,
    ) -> Result<Page<ProjectMember>, RepositoryError>;

    /// Creates one project member responsibility in the current unit of work.
    async fn create(
        &self,
        project_member: ProjectMember,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves one project member responsibility in the current unit of work.
    async fn save(
        &self,
        project_member: ProjectMember,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;
}

/// Stores formal work item and child work item truth.
#[async_trait]
pub trait WorkItemRepository: Send + Sync {
    /// Loads a formal work record by a unified formal work reference.
    async fn get_formal_work(
        &self,
        work_ref: FormalWorkRef,
    ) -> Result<Option<FormalWorkRecord>, RepositoryError>;

    /// Loads project/backlog/member scope for one formal work reference.
    async fn get_formal_work_scope(
        &self,
        work_ref: FormalWorkRef,
    ) -> Result<Option<FormalWorkScope>, RepositoryError>;

    /// Creates a root work item inside the current unit of work.
    async fn create_work_item(
        &self,
        work_item: WorkItem,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Creates a child work item inside the current unit of work.
    async fn create_child_work_item(
        &self,
        child_work_item: ChildWorkItem,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves a formal work lifecycle change inside the current unit of work.
    async fn save_formal_work(
        &self,
        record: FormalWorkRecord,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;
}

/// Stores dependency and blocker truth plus relation history.
#[async_trait]
pub trait DependencyRepository: Send + Sync {
    /// Loads one dependency relation by stable ref.
    async fn get_dependency(
        &self,
        dependency_ref: WorkDependencyRef,
    ) -> Result<Option<WorkDependency>, RepositoryError>;

    /// Loads one blocker relation by stable ref.
    async fn get_blocker(
        &self,
        blocker_ref: WorkBlockerRef,
    ) -> Result<Option<WorkBlocker>, RepositoryError>;

    /// Lists active dependency and blocker refs for one formal work record.
    async fn list_active_for_work(
        &self,
        work_ref: FormalWorkRef,
        page: PageRequest,
    ) -> Result<Page<DependencyOrBlockerRef>, RepositoryError>;

    /// Loads the dependency graph snapshot for one project scope.
    async fn load_graph_snapshot(
        &self,
        project_ref: ProjectRef,
    ) -> Result<DependencyGraphSnapshot, RepositoryError>;

    /// Creates a dependency relation inside the current unit of work.
    async fn create_dependency(
        &self,
        dependency: WorkDependency,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves a dependency relation inside the current unit of work.
    async fn save_dependency(
        &self,
        dependency: WorkDependency,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Creates a blocker relation inside the current unit of work.
    async fn create_blocker(
        &self,
        blocker: WorkBlocker,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves a blocker relation inside the current unit of work.
    async fn save_blocker(
        &self,
        blocker: WorkBlocker,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Appends dependency or blocker history inside the current unit of work.
    async fn append_change(
        &self,
        change: DependencyChangeRecord,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;
}

/// Stores promote decisions and runtime intake markers.
#[async_trait]
pub trait PromoteRepository: Send + Sync {
    /// Loads a promote result by Work identity.
    async fn get(
        &self,
        promote_result_ref: PromoteResultRef,
    ) -> Result<Option<PromoteResult>, RepositoryError>;

    /// Finds the latest promote result for one source reference.
    async fn find_latest_by_source(
        &self,
        source_ref: SourceWorkRef,
    ) -> Result<Option<PromoteResult>, RepositoryError>;

    /// Creates a promote result inside the current unit of work.
    async fn create(
        &self,
        result: PromoteResult,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves a promote state change inside the current unit of work.
    async fn save(
        &self,
        result: PromoteResult,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Appends a promote decision history record.
    async fn append_decision(
        &self,
        decision: PromoteDecisionRecord,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;

    /// Saves an inbound runtime promote request marker without creating promote truth.
    async fn save_pending_intake(
        &self,
        intake: PendingPromoteIntake,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;
}

/// Stores local member snapshots needed by member command flows.
#[async_trait]
pub trait ReferenceSnapshotRepository: Send + Sync {
    /// Loads one cached member capability snapshot.
    async fn get_member_snapshot(
        &self,
        member_ref: GlobalMemberRef,
    ) -> Result<Option<MemberCapabilitySnapshot>, RepositoryError>;

    /// Saves one member capability snapshot in the current unit of work.
    async fn save_member_snapshot(
        &self,
        snapshot: MemberCapabilitySnapshot,
        expected_version: Option<Version>,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;
}

/// Stores Work trace and audit records.
#[async_trait]
pub trait AuditRepository: Send + Sync {
    /// Loads the audit trail for a Work subject.
    async fn get_audit_trail(
        &self,
        subject_ref: WorkAuditSubjectRef,
    ) -> Result<Option<WorkAuditTrail>, RepositoryError>;

    /// Appends a trace record inside the current unit of work.
    async fn append_trace(
        &self,
        record: WorkTraceRecord,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;

    /// Creates or updates an audit trail inside the current unit of work.
    async fn save_audit_trail(
        &self,
        audit_trail: WorkAuditTrail,
        expected_version: Option<Version>,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves a trace handoff marker inside the current unit of work.
    async fn save_trace_handoff_marker(
        &self,
        marker: TraceHandoffMarker,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;
}

/// Stores Work outbox records and publication state.
#[async_trait]
pub trait WorkOutboxRepository: Send + Sync {
    /// Enqueues a committed Work outbox record inside the current unit of work.
    async fn enqueue(
        &self,
        record: WorkOutboxRecord,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;

    /// Loads pending outbox records for publication.
    async fn list_pending(
        &self,
        page: PageRequest,
    ) -> Result<Page<WorkOutboxRecord>, RepositoryError>;

    /// Loads one outbox record.
    async fn get(
        &self,
        outbox_id: WorkOutboxId,
    ) -> Result<Option<WorkOutboxRecord>, RepositoryError>;

    /// Marks an outbox record as published.
    async fn mark_published(
        &self,
        outbox_id: WorkOutboxId,
        publication_ref: OutboxPublicationRef,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Marks an outbox record as failed and retryable.
    async fn mark_failed(
        &self,
        outbox_id: WorkOutboxId,
        reason: OutboxFailureReason,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Marks a failed outbox record as pending when retry policy accepts it.
    async fn mark_pending_for_retry(
        &self,
        outbox_id: WorkOutboxId,
        reason: OutboxRetryReason,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;
}

/// Stores derived Work read views and their freshness state.
#[async_trait]
pub trait ProjectionRepository: Send + Sync {
    /// Marks affected derived views stale after a truth or snapshot change.
    async fn mark_stale(
        &self,
        affected: Vec<DerivedWorkViewRef>,
        source_cursor: WorkTruthCursor,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;
}

/// Resolves member capability summaries from the identity boundary.
#[async_trait]
pub trait MemberReferencePort: Send + Sync {
    /// Resolves safe capability input for one identity member.
    async fn resolve_member_capability(
        &self,
        member_ref: GlobalMemberRef,
    ) -> Result<MemberCapabilitySnapshotInput, PortError>;
}

/// Resolves safe source summaries from adjacent boundaries.
#[async_trait]
pub trait SourceWorkResolverPort: Send + Sync {
    /// Resolves one source reference into a safe summary.
    async fn resolve_source_work(
        &self,
        source_ref: SourceWorkRef,
    ) -> Result<SourceWorkResolution, PortError>;
}

/// Resolves completion or governance evidence from adjacent boundaries.
#[async_trait]
pub trait EvidenceResolverPort: Send + Sync {
    /// Resolves one evidence reference into a safe verified reference.
    async fn resolve_evidence(
        &self,
        evidence_ref: ExternalEvidenceRef,
    ) -> Result<EvidenceResolution, PortError>;
}

/// Generates Work-owned identifiers.
pub trait IdGeneratorPort: Send + Sync {
    /// Generates a project id.
    fn next_project_id(&self) -> Result<work_contracts::ProjectId, PortError>;

    /// Generates a backlog id.
    fn next_backlog_id(&self) -> Result<work_contracts::BacklogId, PortError>;

    /// Generates a project member id.
    fn next_project_member_id(&self) -> Result<ProjectMemberId, PortError>;

    /// Generates a root work item id.
    fn next_work_item_id(&self) -> Result<work_contracts::WorkItemId, PortError>;

    /// Generates a child work item id.
    fn next_child_work_item_id(&self) -> Result<work_contracts::ChildWorkItemId, PortError>;

    /// Generates a promote result id.
    fn next_promote_result_id(&self) -> Result<work_contracts::PromoteResultId, PortError>;

    /// Generates a dependency id.
    fn next_work_dependency_id(&self) -> Result<WorkDependencyId, PortError>;

    /// Generates a blocker id.
    fn next_work_blocker_id(&self) -> Result<WorkBlockerId, PortError>;

    /// Generates a promote decision history id.
    fn next_promote_decision_id(&self) -> Result<work_contracts::PromoteDecisionId, PortError>;

    /// Generates a dependency or blocker history id.
    fn next_dependency_change_id(&self) -> Result<work_contracts::DependencyChangeId, PortError>;

    /// Generates a result id.
    fn next_result_id(&self) -> Result<work_contracts::ResultId, PortError>;

    /// Generates an outbox id.
    fn next_outbox_id(&self) -> Result<work_contracts::WorkOutboxId, PortError>;

    /// Generates a trace id.
    fn next_trace_id(&self) -> Result<work_contracts::WorkTraceId, PortError>;
}

/// Provides timestamps for application services.
pub trait ClockPort: Send + Sync {
    /// Returns the current timestamp.
    fn now(&self) -> Result<Timestamp, PortError>;
}
