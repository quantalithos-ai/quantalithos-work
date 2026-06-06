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
    SourceDigest,
    "Digest supplied by an external source summary."
);
string_newtype!(ResultId, "Stable result or receipt identifier.");
string_newtype!(SourceEventId, "Identifies an upstream source event.");
string_newtype!(
    OutboxPublicationRef,
    "Publication reference returned by the outbox publisher."
);

/// Derived view category.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivedWorkViewKind {
    /// Project board projection.
    ProjectBoard,
    /// Project-member work projection.
    MemberWork,
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

/// Scope used to derive a stable projection key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DerivedWorkViewScopeRef {
    /// Project-scoped view.
    Project(ProjectRef),
    /// Project-member-scoped view.
    ProjectMember(ProjectMemberRef),
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
    ProjectCreated(ProjectRef),
    /// A project lifecycle changed.
    ProjectLifecycleChanged(ProjectRef),
    /// A project member responsibility changed.
    ProjectMemberChanged(ProjectMemberRef),
    /// A backlog availability state changed.
    BacklogAvailabilityChanged(BacklogRef),
    /// A formal work item changed.
    WorkItemChanged(FormalWorkRef),
    /// A promote result was recorded.
    PromoteResultRecorded(PromoteResultRef),
    /// A dependency or blocker changed.
    WorkRelationChanged(DependencyOrBlockerRef),
}

impl WorkTruthChange {
    /// Returns the trace subject implied by this truth change.
    pub fn subject_ref(&self) -> WorkTraceSubjectRef {
        match self {
            Self::ProjectCreated(project_ref) | Self::ProjectLifecycleChanged(project_ref) => {
                WorkTraceSubjectRef::Project(project_ref.clone())
            }
            Self::ProjectMemberChanged(project_member_ref) => {
                WorkTraceSubjectRef::ProjectMember(project_member_ref.clone())
            }
            Self::BacklogAvailabilityChanged(backlog_ref) => {
                WorkTraceSubjectRef::Backlog(backlog_ref.clone())
            }
            Self::WorkItemChanged(work_ref) => WorkTraceSubjectRef::FormalWork(work_ref.clone()),
            Self::PromoteResultRecorded(promote_result_ref) => {
                WorkTraceSubjectRef::PromoteResult(promote_result_ref.clone())
            }
            Self::WorkRelationChanged(relation_ref) => {
                WorkTraceSubjectRef::Relation(relation_ref.clone())
            }
        }
    }

    /// Returns the audit subject implied by this truth change.
    pub fn audit_subject_ref(&self) -> WorkAuditSubjectRef {
        match self {
            Self::ProjectCreated(project_ref) | Self::ProjectLifecycleChanged(project_ref) => {
                WorkAuditSubjectRef::Project(project_ref.clone())
            }
            Self::ProjectMemberChanged(project_member_ref) => {
                WorkAuditSubjectRef::ProjectMember(project_member_ref.clone())
            }
            Self::BacklogAvailabilityChanged(backlog_ref) => {
                WorkAuditSubjectRef::Backlog(backlog_ref.clone())
            }
            Self::WorkItemChanged(work_ref) => WorkAuditSubjectRef::FormalWork(work_ref.clone()),
            Self::PromoteResultRecorded(promote_result_ref) => {
                WorkAuditSubjectRef::PromoteResult(promote_result_ref.clone())
            }
            Self::WorkRelationChanged(relation_ref) => {
                WorkAuditSubjectRef::Relation(relation_ref.clone())
            }
        }
    }

    /// Returns the outbox event kind implied by this truth change.
    pub fn event_kind(&self) -> WorkOutboxEventKind {
        match self {
            Self::ProjectCreated(_) | Self::ProjectLifecycleChanged(_) => {
                WorkOutboxEventKind::ProjectChanged
            }
            Self::ProjectMemberChanged(_) => WorkOutboxEventKind::ProjectMemberChanged,
            Self::BacklogAvailabilityChanged(_) => WorkOutboxEventKind::BacklogChanged,
            Self::WorkItemChanged(_) => WorkOutboxEventKind::WorkItemChanged,
            Self::PromoteResultRecorded(_) => WorkOutboxEventKind::PromoteResultRecorded,
            Self::WorkRelationChanged(DependencyOrBlockerRef::Dependency(_)) => {
                WorkOutboxEventKind::WorkDependencyChanged
            }
            Self::WorkRelationChanged(DependencyOrBlockerRef::Blocker(_)) => {
                WorkOutboxEventKind::WorkBlockerChanged
            }
        }
    }
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
