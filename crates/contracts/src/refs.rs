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
string_newtype!(
    CapabilityRef,
    "Capability reference from identity or method policy."
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
    SourceDigest,
    "Digest supplied by an external source summary."
);
string_newtype!(ResultId, "Stable result or receipt identifier.");
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
        }
    }
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
}
