//! Shared id, ref, reason, and helper value objects for Work.

use serde::{Deserialize, Serialize};

use crate::states::{BacklogState, ProjectLifecycleState};

macro_rules! string_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
    };
}

string_newtype!(ProjectId, "Identifies a Work-owned project subject.");
string_newtype!(
    ProjectMemberId,
    "Identifies a project-local member responsibility."
);
string_newtype!(
    GlobalMemberRef,
    "References an identity-owned global member."
);
string_newtype!(BacklogId, "Identifies a Work-owned backlog.");
string_newtype!(WorkItemId, "Identifies a formal root work item.");
string_newtype!(ChildWorkItemId, "Identifies a formal child work item.");
string_newtype!(WorkDependencyId, "Identifies a formal work dependency.");
string_newtype!(WorkBlockerId, "Identifies a formal work blocker.");
string_newtype!(
    DependencyChangeId,
    "Identifies a dependency or blocker change record."
);
string_newtype!(IterationId, "Identifies a Work-owned iteration.");
string_newtype!(
    IterationCommitmentId,
    "Identifies a Work-owned iteration commitment set."
);
string_newtype!(IterationChangeId, "Identifies an iteration change record.");
string_newtype!(PromoteResultId, "Identifies a promote result.");
string_newtype!(PromoteDecisionId, "Identifies a promote decision record.");
string_newtype!(
    CapabilityRef,
    "Capability reference from identity or method policy."
);
string_newtype!(
    MethodDefinitionRef,
    "References a method-library definition."
);
string_newtype!(
    WorkTruthCursor,
    "Committed Work truth source position used by stale markers and rebuilds."
);
string_newtype!(WorkTraceId, "Identifies a Work trace record.");
string_newtype!(WorkAuditTrailId, "Identifies a Work audit trail.");
string_newtype!(WorkOutboxId, "Identifies a Work outbox record.");
string_newtype!(TraceHandoffRef, "External trace handoff pointer.");
string_newtype!(ArchiveHandoffRef, "External archive handoff pointer.");
string_newtype!(
    SafeSummaryText,
    "Safe short text stored by Work for protocol-visible summaries."
);
string_newtype!(
    WorkTitle,
    "Human-readable title for formal Work public views."
);
string_newtype!(
    WorkSearchText,
    "Free-text query for Work search after protocol validation."
);
string_newtype!(
    WorkSearchCriteriaDigest,
    "Stable digest over normalized WorkSearchCriteria."
);
string_newtype!(
    SourceDigest,
    "Digest supplied by an external source summary."
);
string_newtype!(
    ExternalVersionRef,
    "References an upstream version or cursor for inbound event snapshots."
);
string_newtype!(ResultId, "Stable result or receipt identifier.");
string_newtype!(SourceEventId, "Identifies an upstream source event.");
string_newtype!(
    OutboxPublicationRef,
    "Publication reference returned by the outbox publisher."
);
string_newtype!(
    ProcessTimeboxRef,
    "References one external process timebox."
);
string_newtype!(JobRunId, "Stable id for one operations job run.");

/// Derived view category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivedWorkViewKind {
    /// Project board projection.
    ProjectBoard,
    /// Project-member work projection.
    MemberWork,
    /// Iteration summary projection.
    IterationSummary,
    /// Work search projection.
    Search,
}

/// References a Work-owned project subject across APIs and events.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectRef {
    /// Stable Work project id.
    pub project_id: ProjectId,
}

/// References a project-local member responsibility.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemberRef {
    /// Stable project member responsibility id.
    pub project_member_id: ProjectMemberId,
}

/// References a Work-owned backlog.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BacklogRef {
    /// Stable backlog id.
    pub backlog_id: BacklogId,
}

/// References a promote result.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct PromoteResultRef {
    /// Stable promote result id.
    pub promote_result_id: PromoteResultId,
}

/// References a formal work dependency.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct WorkDependencyRef {
    /// Stable dependency id.
    pub dependency_id: WorkDependencyId,
}

/// References a formal work blocker.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct WorkBlockerRef {
    /// Stable blocker id.
    pub blocker_id: WorkBlockerId,
}

/// References formal Work truth regardless of root or child shape.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FormalWorkRef {
    /// A root formal work item.
    WorkItem(WorkItemId),
    /// A formal child work item.
    ChildWorkItem(ChildWorkItemId),
}

/// References a Work-owned iteration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IterationRef {
    /// Stable iteration id.
    pub iteration_id: IterationId,
}

/// Stable set of formal work refs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormalWorkRefSet {
    /// Stable formal work refs in deterministic order.
    pub refs: Vec<FormalWorkRef>,
}

/// Scope used to derive a stable projection key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DerivedWorkViewScopeRef {
    /// Project-scoped view.
    Project(ProjectRef),
    /// Project-member-scoped view.
    ProjectMember(ProjectMemberRef),
    /// Iteration-scoped view.
    Iteration(IterationRef),
    /// Search-scoped view derived from a full criteria digest.
    Search(ProjectRef, WorkSearchCriteriaDigest),
}

/// Stable reference to one derived Work view freshness marker.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerivedWorkViewRef {
    /// Derived view category.
    pub view_kind: DerivedWorkViewKind,
    /// Stable scope that owns this derived view.
    pub scope_ref: DerivedWorkViewScopeRef,
}

impl DerivedWorkViewRef {
    /// Builds the project board derived view ref for one project.
    pub fn project_board(project_ref: ProjectRef) -> Self {
        Self {
            view_kind: DerivedWorkViewKind::ProjectBoard,
            scope_ref: DerivedWorkViewScopeRef::Project(project_ref),
        }
    }

    /// Builds the member work derived view ref for one project member.
    pub fn member_work(project_member_ref: ProjectMemberRef) -> Self {
        Self {
            view_kind: DerivedWorkViewKind::MemberWork,
            scope_ref: DerivedWorkViewScopeRef::ProjectMember(project_member_ref),
        }
    }

    /// Builds the iteration summary derived view ref for one iteration.
    pub fn iteration_summary(iteration_ref: IterationRef) -> Self {
        Self {
            view_kind: DerivedWorkViewKind::IterationSummary,
            scope_ref: DerivedWorkViewScopeRef::Iteration(iteration_ref),
        }
    }

    /// Builds the work search derived view ref for one project and criteria digest.
    pub fn search(project_ref: ProjectRef, criteria_digest: WorkSearchCriteriaDigest) -> Self {
        Self {
            view_kind: DerivedWorkViewKind::Search,
            scope_ref: DerivedWorkViewScopeRef::Search(project_ref, criteria_digest),
        }
    }
}

/// Reference to either a dependency or a blocker.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum DependencyOrBlockerRef {
    /// Dependency reference.
    Dependency(WorkDependencyRef),
    /// Blocker reference.
    Blocker(WorkBlockerRef),
}

/// Safe blocker cause reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerCauseRef {
    /// External source carrying the cause.
    pub source_ref: ExternalSourceRef,
    /// Optional evidence ref for the cause.
    pub evidence_ref: Option<ExternalEvidenceRef>,
}

/// Read-only blocker impact explanation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerImpactExplanation {
    /// Blocker reference.
    pub blocker_ref: WorkBlockerRef,
    /// Work affected by the blocker.
    pub affected_work_ref: FormalWorkRef,
    /// Safe summary text.
    pub summary: SafeSummaryText,
}

/// Opaque pointer to an external source boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExternalSourceRef {
    /// External system category.
    pub source_system: ExternalSourceSystem,
    /// Stable external id or URI-like pointer.
    pub external_id: String,
}

/// External source system category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSourceSystem {
    /// Workspace or owner system.
    Workspace,
    /// Identity system.
    Identity,
    /// Method library system.
    MethodLibrary,
    /// Conversation system.
    Conversation,
    /// Runtime system.
    Runtime,
    /// Process system.
    Process,
    /// Governance system.
    Governance,
    /// Artifact system.
    Artifact,
    /// Archive or observability boundary.
    Archive,
}

/// Stable key for one external reference snapshot entry.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ExternalReferenceRef {
    /// Identity member reference.
    Member(GlobalMemberRef),
    /// Method definition reference.
    MethodDefinition(MethodDefinitionRef),
    /// Conversation, runtime, governance, or artifact source reference.
    SourceWork(SourceWorkRef),
    /// Completion, blocker, governance, or artifact evidence reference.
    Evidence(ExternalEvidenceRef),
    /// Process timebox reference.
    ProcessTimebox(ProcessTimeboxRef),
}

impl ExternalReferenceRef {
    /// Creates an external reference key from a global member ref.
    pub fn from_member(member_ref: GlobalMemberRef) -> Self {
        Self::Member(member_ref)
    }

    /// Creates an external reference key from a method definition ref.
    pub fn from_method_definition(definition_ref: MethodDefinitionRef) -> Self {
        Self::MethodDefinition(definition_ref)
    }

    /// Creates an external reference key from a source work ref.
    pub fn from_source_work(source_ref: SourceWorkRef) -> Self {
        Self::SourceWork(source_ref)
    }

    /// Creates an external reference key from an evidence ref.
    pub fn from_evidence(evidence_ref: ExternalEvidenceRef) -> Self {
        Self::Evidence(evidence_ref)
    }

    /// Creates an external reference key from a process timebox ref.
    pub fn from_process_timebox(timebox_ref: ProcessTimeboxRef) -> Self {
        Self::ProcessTimebox(timebox_ref)
    }
}

/// Kind of external owner for a Work project.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectOwnerKind {
    /// Workspace project owner.
    WorkspaceProject,
    /// Organization owner.
    Organization,
    /// External project owner.
    ExternalProject,
}

/// Project responsibility category used by Work policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectResponsibilityKind {
    /// Person or agent accountable for project work.
    Owner,
    /// Contributor who can be assigned work.
    Contributor,
    /// Reviewer who can inspect or approve work.
    Reviewer,
    /// Observer with read-only responsibility.
    Observer,
}

/// Capability references required by one responsibility spec.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityRefSet {
    /// Stable capability refs only.
    pub refs: Vec<CapabilityRef>,
}

/// Points to the external owner of a Work project without owning that body.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectOwnerRef {
    /// Owner system or tenant family.
    pub owner_kind: ProjectOwnerKind,
    /// Stable external owner pointer.
    pub external_ref: ExternalSourceRef,
}

/// Classifies an external work source that can be evaluated for formalization.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceWorkKind {
    /// A conversation-originated suggestion or context marker.
    Conversation,
    /// A runtime plan item or execution-local source.
    Runtime,
    /// An artifact or evidence-originated source.
    Artifact,
    /// A process planning or timing source.
    Process,
    /// A governance-originated recommendation or decision pointer.
    Governance,
}

/// Method definition category safe for Work intent classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MethodDefinitionKind {
    /// Task-oriented method definition.
    Task,
    /// Product-oriented method definition.
    Product,
    /// Process-oriented method definition.
    Process,
    /// View-profile method definition.
    ViewProfile,
}

/// Indicates whether an external evidence reference is safe to use.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceVerifiedState {
    /// The evidence has not been checked by an accepted resolver.
    Unverified,
    /// The evidence was checked and may be used for completion or resolution.
    Verified,
    /// The evidence resolver failed or rejected the evidence.
    Rejected,
}

/// Evidence category safe for Work policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Completion evidence.
    Completion,
    /// Blocker resolution evidence.
    BlockerResolution,
    /// Governance evidence.
    Governance,
    /// Artifact evidence.
    Artifact,
}

/// Points to an external source that may be formalized or promoted into Work truth.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceWorkRef {
    /// Source category.
    pub source_kind: SourceWorkKind,
    /// External stable pointer.
    pub external_ref: ExternalSourceRef,
    /// Optional digest for source summary verification.
    pub source_digest: Option<SourceDigest>,
}

/// Points to external evidence without storing the body.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExternalEvidenceRef {
    /// Evidence category.
    pub evidence_kind: EvidenceKind,
    /// External stable pointer.
    pub external_ref: ExternalSourceRef,
    /// Verification state of the referenced evidence.
    pub verified_state: EvidenceVerifiedState,
}

/// Safe summary resolved from Process for iteration opening.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcessTimeboxSummary {
    /// Process timebox reference resolved by the port.
    pub timebox_ref: ProcessTimeboxRef,
    /// Project scope that this timebox may bind to.
    pub project_ref: ProjectRef,
    /// Whether Process currently allows Work to open an iteration for this timebox.
    pub can_open_iteration: bool,
    /// Optional safe summary supplied by Process.
    pub summary: Option<SafeSummaryText>,
    /// Digest of the Process-owned timebox summary snapshot.
    pub source_digest: SourceDigest,
}

/// Target lifecycle state requested for formal work.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkLifecycleTarget {
    /// Start work.
    InProgress,
    /// Mark work complete.
    Completed,
    /// Cancel work.
    Cancelled,
    /// Supersede work with another formal record.
    Superseded,
}

/// Review decision for a promote result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromoteReviewDecision {
    /// Accept the source into formal Work.
    Accept,
    /// Reject the source with an auditable reason.
    Reject(PromoteRejectReason),
}

/// Target state requested for a dependency.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyTarget {
    /// Activate a proposed dependency.
    Active,
    /// Mark the dependency satisfied.
    Satisfied,
    /// Waive the dependency.
    Waived,
    /// Cancel the dependency.
    Cancelled,
}

/// Target lifecycle state requested for a Work project.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLifecycleTarget {
    /// Move the project to read-only mode.
    ReadOnly,
    /// Close the project for normal Work writes.
    Closed,
    /// Archive a closed project.
    Archived,
}

/// Reason supplied for a project lifecycle transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectLifecycleReason {
    /// Reason category.
    pub reason_kind: ProjectLifecycleReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
    /// Optional safe short summary.
    pub note: Option<SafeSummaryText>,
}

impl ProjectLifecycleReason {
    /// Returns the canonical create reason captured for initial project publication.
    pub fn created() -> Self {
        Self {
            reason_kind: ProjectLifecycleReasonKind::Created,
            reason_ref: None,
            note: None,
        }
    }
}

/// Target responsibility state requested for a project member.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponsibilityTarget {
    /// Activate or resume the responsibility.
    Active,
    /// Pause the responsibility.
    Paused,
    /// Release the responsibility.
    Released,
}

/// Reason supplied for project member responsibility transitions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemberReason {
    /// Reason category.
    pub reason_kind: ProjectMemberReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Project lifecycle explanation category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLifecycleReasonKind {
    /// The project was created.
    Created,
    /// Transition required by policy.
    Policy,
    /// Transition driven by maintenance.
    Maintenance,
    /// Transition requested by the project owner.
    OwnerRequest,
    /// Transition prepared for archive.
    ArchivePrepared,
}

/// Project member responsibility explanation category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMemberReasonKind {
    /// Responsibility was assigned.
    Assigned,
    /// Identity capability changed.
    CapabilityChanged,
    /// Responsibility was paused.
    Paused,
    /// Responsibility was released.
    Released,
}

/// Reason supplied for backlog maintenance transitions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BacklogMaintenanceReason {
    /// Reason category.
    pub reason_kind: BacklogMaintenanceReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Reason supplied for formal work lifecycle transitions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkLifecycleReason {
    /// Reason category.
    pub reason_kind: WorkLifecycleReasonKind,
    /// Formal work superseding this record when applicable.
    pub superseding_ref: Option<FormalWorkRef>,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Reason supplied for a promote request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromoteReason {
    /// Reason category.
    pub reason_kind: PromoteReasonKind,
    /// Optional source summary reference.
    pub source_summary_ref: Option<SourceWorkRef>,
}

/// Reason supplied when a promote review rejects the source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromoteRejectReason {
    /// Rejection category.
    pub reason_kind: PromoteRejectReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Reason supplied when a dependency is linked.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DependencyReason {
    /// Reason category.
    pub reason_kind: DependencyReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Reason supplied when a dependency changes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DependencyChangeReason {
    /// Reason category.
    pub reason_kind: DependencyChangeReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
    /// Optional blocker cause that produced this dependency change.
    pub blocker_cause_ref: Option<BlockerCauseRef>,
}

impl DependencyChangeReason {
    /// Builds the activation change reason derived from an accepted link reason.
    pub fn from_link_reason(reason: DependencyReason) -> Self {
        Self {
            reason_kind: DependencyChangeReasonKind::Activated,
            reason_ref: reason.reason_ref,
            blocker_cause_ref: None,
        }
    }

    /// Builds a blocker-derived change reason without persisting external body content.
    pub fn from_blocker_cause(cause_ref: BlockerCauseRef) -> Self {
        Self {
            reason_kind: DependencyChangeReasonKind::FromBlockerCause,
            reason_ref: None,
            blocker_cause_ref: Some(cause_ref),
        }
    }
}

/// Reason supplied when blocker mitigation starts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerMitigationReason {
    /// Reason category.
    pub reason_kind: BlockerMitigationReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Reason supplied when a blocker record is closed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerCloseReason {
    /// Reason category.
    pub reason_kind: BlockerCloseReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Backlog availability explanation category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BacklogMaintenanceReasonKind {
    /// Planned maintenance window.
    MaintenanceWindow,
    /// Policy-driven hold.
    PolicyHold,
    /// Manual unlock after maintenance.
    ManualUnlock,
}

/// Formal work lifecycle explanation category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkLifecycleReasonKind {
    /// Work started.
    Start,
    /// Work completed with evidence.
    CompletionEvidence,
    /// Work cancelled before completion.
    Cancellation,
    /// Work superseded by another formal record.
    Superseded,
}

/// Promote request explanation category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromoteReasonKind {
    /// Promotion request came from runtime.
    RuntimeRequest,
    /// Promotion request came from conversation context.
    ConversationSignal,
    /// Promotion request came from governance recommendation.
    GovernanceRecommendation,
    /// Promotion request was raised manually for review.
    ManualReview,
}

/// Promote rejection category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromoteRejectReasonKind {
    /// The source is not collaborative Work.
    NotCollaborativeWork,
    /// The source duplicates existing truth.
    Duplicate,
    /// The source lacks sufficient evidence.
    InsufficientEvidence,
    /// Policy rejected the promotion.
    PolicyRejected,
}

/// Dependency creation category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyReasonKind {
    /// Dependency expresses explicit ordering between formal work.
    ExplicitOrdering,
    /// Dependency requires evidence or artifact completion first.
    EvidencePrerequisite,
    /// Dependency follows governance requirement.
    GovernanceRequirement,
    /// Dependency was created from manual review.
    ManualReview,
}

/// Dependency change category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyChangeReasonKind {
    /// Proposed dependency was activated.
    Activated,
    /// Active dependency was satisfied by verified evidence.
    SatisfiedByEvidence,
    /// Active dependency was explicitly waived.
    Waived,
    /// Dependency was cancelled.
    Cancelled,
    /// Change history was derived from blocker cause.
    FromBlockerCause,
}

/// Blocker mitigation category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerMitigationReasonKind {
    /// A mitigation plan was created.
    PlanCreated,
    /// Work owner took mitigation action.
    OwnerAction,
    /// Mitigation depends on an external dependency.
    ExternalDependency,
}

/// Blocker close category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerCloseReasonKind {
    /// Blocker was resolved with verified evidence.
    ResolvedVerified,
    /// Blocker no longer applies.
    NoLongerApplies,
    /// Blocker was superseded by later truth.
    Superseded,
}

/// Work outbox event category derived from a truth change.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkOutboxEventKind {
    /// Project changed event.
    ProjectChanged,
    /// Backlog changed event.
    BacklogChanged,
    /// Project member changed event.
    ProjectMemberChanged,
    /// Work item changed event.
    WorkItemChanged,
    /// Promote result recorded event.
    PromoteResultRecorded,
    /// Dependency changed event.
    WorkDependencyChanged,
    /// Blocker changed event.
    WorkBlockerChanged,
    /// Iteration changed event.
    IterationChanged,
    /// Trace became available.
    WorkTraceAvailable,
    /// Derived view changed event.
    DerivedWorkViewChanged,
}

/// Typed outbox source identity used to rebuild outbound payloads from committed state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkOutboxSourceRef {
    /// Project source with lifecycle reason captured at enqueue time.
    Project {
        /// Changed project.
        project_ref: ProjectRef,
        /// Lifecycle reason required by the outbound payload.
        reason: ProjectLifecycleReason,
    },
    /// Backlog source with maintenance reason captured at enqueue time.
    Backlog {
        /// Changed backlog.
        backlog_ref: BacklogRef,
        /// Maintenance reason required by the outbound payload.
        reason: BacklogMaintenanceReason,
    },
    /// Project member source.
    ProjectMember(ProjectMemberRef),
    /// Formal work source.
    FormalWork(FormalWorkRef),
    /// Promote result source.
    PromoteResult(PromoteResultRef),
    /// Dependency source.
    Dependency(WorkDependencyRef),
    /// Blocker source.
    Blocker(WorkBlockerRef),
    /// Iteration source.
    Iteration(IterationRef),
    /// Trace availability source.
    TraceAvailable {
        /// Trace that became available.
        trace_id: WorkTraceId,
        /// Optional handoff pointer.
        handoff_ref: Option<TraceHandoffRef>,
    },
    /// Derived view freshness source.
    DerivedView(DerivedWorkViewRef),
}

impl WorkOutboxSourceRef {
    /// Returns the unique outbound event kind implied by this source identity.
    pub fn event_kind(&self) -> WorkOutboxEventKind {
        match self {
            Self::Project { .. } => WorkOutboxEventKind::ProjectChanged,
            Self::Backlog { .. } => WorkOutboxEventKind::BacklogChanged,
            Self::ProjectMember(_) => WorkOutboxEventKind::ProjectMemberChanged,
            Self::FormalWork(_) => WorkOutboxEventKind::WorkItemChanged,
            Self::PromoteResult(_) => WorkOutboxEventKind::PromoteResultRecorded,
            Self::Dependency(_) => WorkOutboxEventKind::WorkDependencyChanged,
            Self::Blocker(_) => WorkOutboxEventKind::WorkBlockerChanged,
            Self::Iteration(_) => WorkOutboxEventKind::IterationChanged,
            Self::TraceAvailable { .. } => WorkOutboxEventKind::WorkTraceAvailable,
            Self::DerivedView(_) => WorkOutboxEventKind::DerivedWorkViewChanged,
        }
    }
}

/// Failure reason recorded for an outbox publish attempt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutboxFailureReason {
    /// Safe failure category.
    pub reason_kind: OutboxFailureReasonKind,
    /// Safe short message.
    pub message: SafeSummaryText,
}

/// Outbox publication failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxFailureReasonKind {
    /// Publisher returned a retryable error.
    Retryable,
    /// Publisher returned a terminal error.
    Terminal,
}

/// Retry reason recorded when a failed outbox record is made pending again.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutboxRetryReason {
    /// Safe retry message.
    pub message: SafeSummaryText,
    /// Previous publish failure.
    pub previous_failure: OutboxFailureReason,
}

/// Subject affected by a Work trace record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkTraceSubjectRef {
    /// Project subject.
    Project(ProjectRef),
    /// Backlog subject.
    Backlog(BacklogRef),
    /// Project member subject.
    ProjectMember(ProjectMemberRef),
    /// Formal work subject.
    FormalWork(FormalWorkRef),
    /// Promote result subject.
    PromoteResult(PromoteResultRef),
    /// Dependency or blocker subject.
    Relation(DependencyOrBlockerRef),
    /// Iteration subject.
    Iteration(IterationRef),
    /// Trace or archive handoff subject.
    Handoff(TraceHandoffRef),
}

/// Subject used to own an audit trail.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkAuditSubjectRef {
    /// Project audit subject.
    Project(ProjectRef),
    /// Backlog audit subject.
    Backlog(BacklogRef),
    /// Project member audit subject.
    ProjectMember(ProjectMemberRef),
    /// Formal work audit subject.
    FormalWork(FormalWorkRef),
    /// Promote result audit subject.
    PromoteResult(PromoteResultRef),
    /// Dependency or blocker audit subject.
    Relation(DependencyOrBlockerRef),
    /// Iteration audit subject.
    Iteration(IterationRef),
}

/// Set of trace records linked from an audit trail.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkTraceRecordRefSet {
    /// Trace record ids in append order.
    pub trace_ids: Vec<WorkTraceId>,
}

/// Describes an accepted Work truth change for trace and outbox construction.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkTruthChange {
    /// A project was created.
    ProjectCreated(ProjectRef, ProjectLifecycleReason),
    /// A project lifecycle changed.
    ProjectLifecycleChanged(ProjectRef, ProjectLifecycleReason),
    /// A project member responsibility changed.
    ProjectMemberChanged(ProjectMemberRef),
    /// A backlog availability state changed.
    BacklogAvailabilityChanged(BacklogRef, BacklogMaintenanceReason),
    /// A formal work item changed.
    WorkItemChanged(FormalWorkRef),
    /// A promote result was recorded.
    PromoteResultRecorded(PromoteResultRef),
    /// A dependency or blocker changed.
    WorkRelationChanged(DependencyOrBlockerRef),
    /// An iteration or commitment changed.
    IterationChanged(IterationRef),
}

impl WorkTruthChange {
    /// Returns the trace subject implied by this truth change.
    pub fn subject_ref(&self) -> WorkTraceSubjectRef {
        match self {
            Self::ProjectCreated(project_ref, _)
            | Self::ProjectLifecycleChanged(project_ref, _) => {
                WorkTraceSubjectRef::Project(project_ref.clone())
            }
            Self::ProjectMemberChanged(project_member_ref) => {
                WorkTraceSubjectRef::ProjectMember(project_member_ref.clone())
            }
            Self::BacklogAvailabilityChanged(backlog_ref, _) => {
                WorkTraceSubjectRef::Backlog(backlog_ref.clone())
            }
            Self::WorkItemChanged(work_ref) => WorkTraceSubjectRef::FormalWork(work_ref.clone()),
            Self::PromoteResultRecorded(promote_result_ref) => {
                WorkTraceSubjectRef::PromoteResult(promote_result_ref.clone())
            }
            Self::WorkRelationChanged(relation_ref) => {
                WorkTraceSubjectRef::Relation(relation_ref.clone())
            }
            Self::IterationChanged(iteration_ref) => {
                WorkTraceSubjectRef::Iteration(iteration_ref.clone())
            }
        }
    }

    /// Returns the audit subject implied by this truth change.
    pub fn audit_subject_ref(&self) -> WorkAuditSubjectRef {
        match self {
            Self::ProjectCreated(project_ref, _)
            | Self::ProjectLifecycleChanged(project_ref, _) => {
                WorkAuditSubjectRef::Project(project_ref.clone())
            }
            Self::ProjectMemberChanged(project_member_ref) => {
                WorkAuditSubjectRef::ProjectMember(project_member_ref.clone())
            }
            Self::BacklogAvailabilityChanged(backlog_ref, _) => {
                WorkAuditSubjectRef::Backlog(backlog_ref.clone())
            }
            Self::WorkItemChanged(work_ref) => WorkAuditSubjectRef::FormalWork(work_ref.clone()),
            Self::PromoteResultRecorded(promote_result_ref) => {
                WorkAuditSubjectRef::PromoteResult(promote_result_ref.clone())
            }
            Self::WorkRelationChanged(relation_ref) => {
                WorkAuditSubjectRef::Relation(relation_ref.clone())
            }
            Self::IterationChanged(iteration_ref) => {
                WorkAuditSubjectRef::Iteration(iteration_ref.clone())
            }
        }
    }

    /// Returns the outbox event kind implied by this truth change.
    pub fn event_kind(&self) -> WorkOutboxEventKind {
        match self {
            Self::ProjectCreated(_, _) | Self::ProjectLifecycleChanged(_, _) => {
                WorkOutboxEventKind::ProjectChanged
            }
            Self::ProjectMemberChanged(_) => WorkOutboxEventKind::ProjectMemberChanged,
            Self::BacklogAvailabilityChanged(_, _) => WorkOutboxEventKind::BacklogChanged,
            Self::WorkItemChanged(_) => WorkOutboxEventKind::WorkItemChanged,
            Self::PromoteResultRecorded(_) => WorkOutboxEventKind::PromoteResultRecorded,
            Self::WorkRelationChanged(DependencyOrBlockerRef::Dependency(_)) => {
                WorkOutboxEventKind::WorkDependencyChanged
            }
            Self::WorkRelationChanged(DependencyOrBlockerRef::Blocker(_)) => {
                WorkOutboxEventKind::WorkBlockerChanged
            }
            Self::IterationChanged(_) => WorkOutboxEventKind::IterationChanged,
        }
    }

    /// Returns the canonical typed outbox source derived from this accepted truth change.
    pub fn outbox_source_ref(&self) -> WorkOutboxSourceRef {
        match self {
            Self::ProjectCreated(project_ref, reason)
            | Self::ProjectLifecycleChanged(project_ref, reason) => WorkOutboxSourceRef::Project {
                project_ref: project_ref.clone(),
                reason: reason.clone(),
            },
            Self::ProjectMemberChanged(project_member_ref) => {
                WorkOutboxSourceRef::ProjectMember(project_member_ref.clone())
            }
            Self::BacklogAvailabilityChanged(backlog_ref, reason) => WorkOutboxSourceRef::Backlog {
                backlog_ref: backlog_ref.clone(),
                reason: reason.clone(),
            },
            Self::WorkItemChanged(work_ref) => WorkOutboxSourceRef::FormalWork(work_ref.clone()),
            Self::PromoteResultRecorded(promote_result_ref) => {
                WorkOutboxSourceRef::PromoteResult(promote_result_ref.clone())
            }
            Self::WorkRelationChanged(DependencyOrBlockerRef::Dependency(dependency_ref)) => {
                WorkOutboxSourceRef::Dependency(dependency_ref.clone())
            }
            Self::WorkRelationChanged(DependencyOrBlockerRef::Blocker(blocker_ref)) => {
                WorkOutboxSourceRef::Blocker(blocker_ref.clone())
            }
            Self::IterationChanged(iteration_ref) => {
                WorkOutboxSourceRef::Iteration(iteration_ref.clone())
            }
        }
    }
}

/// Typed outbound publication assembled from a committed outbox record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkOutboundPublication {
    /// Project changed publication.
    ProjectChanged(crate::events::WorkOutboundEventEnvelope<crate::events::ProjectChangedEvent>),
    /// Backlog changed publication.
    BacklogChanged(crate::events::WorkOutboundEventEnvelope<crate::events::BacklogChangedEvent>),
    /// Project member changed publication.
    ProjectMemberChanged(
        crate::events::WorkOutboundEventEnvelope<crate::events::ProjectMemberChangedEvent>,
    ),
    /// Formal work changed publication.
    WorkItemChanged(crate::events::WorkOutboundEventEnvelope<crate::events::WorkItemChangedEvent>),
    /// Promote result recorded publication.
    PromoteResultRecorded(
        crate::events::WorkOutboundEventEnvelope<crate::events::PromoteResultRecordedEvent>,
    ),
    /// Dependency changed publication.
    WorkDependencyChanged(
        crate::events::WorkOutboundEventEnvelope<crate::events::WorkDependencyChangedEvent>,
    ),
    /// Blocker changed publication.
    WorkBlockerChanged(
        crate::events::WorkOutboundEventEnvelope<crate::events::WorkBlockerChangedEvent>,
    ),
    /// Iteration changed publication.
    IterationChanged(
        crate::events::WorkOutboundEventEnvelope<crate::events::IterationChangedEvent>,
    ),
    /// Trace available publication.
    WorkTraceAvailable(
        crate::events::WorkOutboundEventEnvelope<crate::events::WorkTraceAvailableEvent>,
    ),
    /// Derived view changed publication.
    DerivedWorkViewChanged(
        crate::events::WorkOutboundEventEnvelope<crate::events::DerivedWorkViewChangedEvent>,
    ),
}

/// Describes a formal collaborative work intent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormalWorkIntent {
    /// Human-readable title for public Work views.
    pub title: WorkTitle,
    /// Stable method definition reference used to classify the work.
    pub method_definition_ref: Option<MethodDefinitionRef>,
    /// Intended assignee inside the project.
    pub assignee_ref: ProjectMemberRef,
    /// Optional parent formal work for split candidates.
    pub parent_ref: Option<FormalWorkRef>,
}

/// Describes changes to an iteration commitment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IterationCommitmentChangeSet {
    /// Formal work refs to add to the commitment.
    pub add_work_refs: Vec<FormalWorkRef>,
    /// Formal work refs to remove from the commitment.
    pub remove_work_refs: Vec<FormalWorkRef>,
}

/// Scope used by Work truth policy checks.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkPolicyScope {
    /// Project scope for the check.
    pub project_ref: ProjectRef,
    /// Optional formal work affected by the check.
    pub work_ref: Option<FormalWorkRef>,
    /// Optional external source considered by the check.
    pub source_ref: Option<SourceWorkRef>,
}

/// Safe candidate summary for formal work admission.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormalWorkCandidateSummary {
    /// Candidate title.
    pub title: WorkTitle,
    /// Source reference.
    pub source_ref: SourceWorkRef,
    /// Optional method definition reference.
    pub method_definition_ref: Option<MethodDefinitionRef>,
    /// Candidate assignee.
    pub assignee_ref: ProjectMemberRef,
}

/// Safe external source summary used by Work policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExternalSourceSummary {
    /// Source reference.
    pub source_ref: SourceWorkRef,
    /// Source kind.
    pub source_kind: SourceWorkKind,
    /// Optional digest supplied by the source.
    pub source_digest: Option<SourceDigest>,
    /// Whether the resolver observed an external body that must be rejected.
    pub has_external_body: bool,
}

/// Decision returned by promote policy before a promote result is mutated.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PromoteDecision {
    /// Promotion may proceed.
    Allow,
    /// Promotion must be rejected with a reason.
    Reject(PromoteRejectReason),
    /// Promotion duplicates an existing formal work record.
    Duplicate(FormalWorkRef),
}

/// Target lifecycle state requested for an iteration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationLifecycleTarget {
    /// Start a committed iteration.
    InProgress,
    /// Close the iteration.
    Closed,
    /// Cancel the iteration.
    Cancelled,
}

/// Reason supplied when an iteration or commitment changes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IterationChangeReason {
    /// Reason category.
    pub reason_kind: IterationChangeReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Reason supplied when an iteration closes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IterationCloseReason {
    /// Reason category.
    pub reason_kind: IterationCloseReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Reason supplied when committed work is removed or adjusted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommitmentChangeReason {
    /// Reason category.
    pub reason_kind: CommitmentChangeReasonKind,
    /// Optional external evidence or decision reference.
    pub reason_ref: Option<ExternalEvidenceRef>,
}

/// Iteration change category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationChangeReasonKind {
    /// Initial commitment was created.
    CommitmentCreated,
    /// Existing commitment changed.
    CommitmentChanged,
    /// Iteration started.
    Started,
    /// Iteration was cancelled.
    Cancelled,
    /// Process timing or signal caused the change.
    ProcessSignal,
}

/// Iteration close category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IterationCloseReasonKind {
    /// Iteration completed successfully.
    Completed,
    /// Iteration closed because it was cancelled.
    Cancelled,
    /// Iteration closed because the timebox ended.
    TimeboxEnded,
    /// Iteration was closed manually.
    ManualClose,
}

/// Commitment change category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentChangeReasonKind {
    /// Commitment scope was reduced.
    ScopeReduced,
    /// Commitment scope was expanded.
    ScopeExpanded,
    /// Commitment changed because a dependency changed.
    DependencyChanged,
    /// Commitment changed manually.
    ManualAdjustment,
}

/// Trace handoff target category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceHandoffTargetKind {
    /// Observability sink.
    Observability,
    /// Archive boundary.
    Archive,
    /// Diagnostic consumer.
    Diagnostic,
}

/// Trace handoff target.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TraceHandoffTargetRef {
    /// Target category.
    pub target_kind: TraceHandoffTargetKind,
    /// External target pointer.
    pub external_ref: ExternalSourceRef,
}

/// Intent produced before a trace handoff marker is persisted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TraceHandoffIntent {
    /// Trace to hand off.
    pub trace_id: WorkTraceId,
    /// Archive or observability target.
    pub target_ref: TraceHandoffTargetRef,
    /// Subject covered by the handoff.
    pub subject_ref: WorkTraceSubjectRef,
}

/// Scope prepared for archive handoff.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArchiveHandoffScope {
    /// Scope kind.
    pub scope_kind: ArchiveHandoffScopeKind,
    /// Project scope when archiving one project cursor.
    pub project_ref: Option<ProjectRef>,
    /// Work subjects included in the scope.
    pub subject_refs: Vec<WorkTraceSubjectRef>,
    /// Optional truth cursor covered by this handoff.
    pub source_cursor: Option<WorkTruthCursor>,
}

/// Archive handoff scope kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveHandoffScopeKind {
    /// Archive selected Work subjects.
    Subjects,
    /// Archive a project up to the supplied cursor.
    ProjectCursor,
}

/// Target for archive handoff.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArchiveHandoffTargetRef {
    /// Target kind.
    pub target_kind: ArchiveHandoffTargetKind,
    /// External pointer for the target.
    pub external_ref: ExternalSourceRef,
}

/// Archive handoff target kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveHandoffTargetKind {
    /// General archive boundary.
    ArchiveStore,
    /// Compliance export boundary.
    ComplianceExport,
}

/// Failed item identity recorded in a Work job report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkJobFailureRef {
    /// Failed external reference refresh item.
    ExternalReference(ExternalReferenceRef),
    /// Failed trace handoff item.
    TraceHandoff {
        /// Trace record that failed handoff.
        trace_id: WorkTraceId,
        /// Trace subject selected by the job.
        subject_ref: WorkTraceSubjectRef,
        /// Handoff target that rejected or failed the handoff.
        target_ref: TraceHandoffTargetRef,
    },
    /// Failed archive handoff item.
    ArchiveHandoff {
        /// Archive scope requested by the job.
        archive_scope: ArchiveHandoffScope,
        /// Archive target that rejected or failed the handoff.
        target_ref: ArchiveHandoffTargetRef,
    },
    /// Failed outbox publication item.
    WorkOutbox(WorkOutboxId),
    /// Failed derived-view rebuild item.
    DerivedWorkView(DerivedWorkViewRef),
}

/// Scope for refreshing external references.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExternalReferenceScope {
    /// Scope kind.
    pub scope_kind: ExternalReferenceScopeKind,
    /// Project scope when the kind is project-scoped.
    pub project_ref: Option<ProjectRef>,
    /// Explicit reference refs when the kind is explicit.
    pub reference_refs: Vec<ExternalReferenceRef>,
}

/// External reference refresh scope kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalReferenceScopeKind {
    /// Refresh stale references selected by repository.
    StaleOnly,
    /// Refresh references related to one project.
    Project,
    /// Refresh explicitly listed references.
    ExplicitRefs,
}

/// Scope for reconciliation jobs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkReconciliationScopeRef {
    /// Scope kind.
    pub scope_kind: WorkReconciliationScopeKind,
    /// Project scope when applicable.
    pub project_ref: Option<ProjectRef>,
    /// Derived view scope when applicable.
    pub view_ref: Option<DerivedWorkViewRef>,
    /// External reference scope when applicable.
    pub reference_ref: Option<ExternalReferenceRef>,
}

/// Reconciliation scope kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkReconciliationScopeKind {
    /// Inspect all Work reconciliation surfaces.
    All,
    /// Inspect one project.
    Project,
    /// Inspect one derived view.
    DerivedView,
    /// Inspect one external reference.
    ExternalReference,
}

/// Summarizes the truth state used by minimal fixtures and tests.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkTruthSnapshot {
    /// Project covered by this snapshot.
    pub project_ref: ProjectRef,
    /// Project lifecycle state.
    pub lifecycle_state: ProjectLifecycleState,
    /// Backlog state when available.
    pub backlog_state: Option<BacklogState>,
    /// Current truth cursor.
    pub source_cursor: WorkTruthCursor,
}
