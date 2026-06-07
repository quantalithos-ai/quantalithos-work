//! Inbound and outbound event DTOs for Work.

use serde::{Deserialize, Serialize};

use core_contracts::metadata::Timestamp;

use crate::handoff::WorkTraceContextRef;
use crate::refs::{
    BacklogMaintenanceReason, BacklogRef, DerivedWorkViewRef, ExternalEvidenceRef,
    ExternalSourceRef, ExternalVersionRef, FormalWorkRef, GlobalMemberRef, IterationRef,
    MethodDefinitionKind, MethodDefinitionRef, ProcessTimeboxRef, ProjectMemberRef, ProjectRef,
    PromoteReason, PromoteResultRef, SourceEventId, SourceWorkRef, TraceHandoffRef, WorkBlockerRef,
    WorkDependencyRef, WorkOutboxId, WorkTraceId, WorkTraceSubjectRef, WorkTruthCursor,
};
use crate::states::{
    BacklogState, BlockerState, CommitmentState, DependencyState, DerivedFreshnessState,
    IterationState, ProjectLifecycleState, ProjectMemberResponsibilityState, PromoteResultState,
    WorkItemState,
};

/// Event schema version carried by Work inbound and outbound envelopes.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventSchemaVersion(pub String);

impl EventSchemaVersion {
    /// Returns the only supported P0 event schema version.
    pub fn v1() -> Self {
        Self("v1".to_owned())
    }
}

/// Metadata carried by inbound events before operation-specific payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkInboundEventEnvelope<T> {
    /// Source-owned event id used for deduplication.
    pub source_event_id: SourceEventId,
    /// Source system or bounded context that emitted the event.
    pub source_ref: ExternalSourceRef,
    /// Event schema version.
    pub event_version: EventSchemaVersion,
    /// Core trace and request pointer.
    pub trace_context_ref: WorkTraceContextRef,
    /// Event occurrence timestamp.
    pub occurred_at: Timestamp,
    /// Operation-specific event payload.
    pub payload: T,
}

/// Identity member change consumed by Work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IdentityMemberChangedPayload {
    /// Referenced identity member.
    pub member_ref: GlobalMemberRef,
    /// Capability refs safe for Work responsibility checks.
    pub capability_refs: crate::refs::CapabilityRefSet,
    /// Upstream member version or cursor.
    pub source_version_ref: ExternalVersionRef,
}

/// Method definition change consumed by Work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MethodDefinitionChangedPayload {
    /// Referenced method definition.
    pub definition_ref: MethodDefinitionRef,
    /// Definition category.
    pub definition_kind: MethodDefinitionKind,
    /// Upstream definition version or cursor.
    pub source_version_ref: ExternalVersionRef,
}

/// Conversation work context change consumed by Work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConversationWorkContextChangedPayload {
    /// Source work reference derived from conversation context.
    pub source_ref: SourceWorkRef,
    /// Optional digest of the source summary.
    pub source_digest: Option<crate::refs::SourceDigest>,
}

/// Process timing change consumed by Work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcessTimingChangedPayload {
    /// Process timebox reference.
    pub timebox_ref: ProcessTimeboxRef,
    /// Project affected when known.
    pub project_ref: Option<ProjectRef>,
    /// Upstream timing version or cursor.
    pub source_version_ref: ExternalVersionRef,
}

/// Governance decision change consumed by Work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GovernanceDecisionChangedPayload {
    /// Governance decision source reference.
    pub source_ref: SourceWorkRef,
    /// Evidence reference when the decision can support a Work transition.
    pub evidence_ref: Option<ExternalEvidenceRef>,
    /// Upstream decision version or cursor.
    pub source_version_ref: ExternalVersionRef,
}

/// Artifact evidence change consumed by Work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactEvidenceChangedPayload {
    /// External evidence reference.
    pub evidence_ref: ExternalEvidenceRef,
    /// Upstream artifact version or cursor.
    pub source_version_ref: ExternalVersionRef,
}

/// Runtime promote request consumed by Work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimePromoteRequestedPayload {
    /// Runtime source ref that may become a Work promotion.
    pub source_ref: SourceWorkRef,
    /// Reason supplied by runtime.
    pub promote_reason: PromoteReason,
}

/// Shared envelope for Work outbound events.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkOutboundEventEnvelope<T> {
    /// Work-owned outbox record id.
    pub outbox_id: WorkOutboxId,
    /// Event schema version.
    pub event_version: EventSchemaVersion,
    /// Core trace and request pointer.
    pub trace_context_ref: WorkTraceContextRef,
    /// Event creation timestamp.
    pub occurred_at: Timestamp,
    /// Operation-specific payload.
    pub payload: T,
}

/// Project change event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectChangedEvent {
    /// Changed project.
    pub project_ref: ProjectRef,
    /// Current lifecycle state.
    pub lifecycle_state: ProjectLifecycleState,
    /// Change reason.
    pub reason: crate::refs::ProjectLifecycleReason,
}

/// Backlog availability change event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BacklogChangedEvent {
    /// Changed backlog.
    pub backlog_ref: BacklogRef,
    /// Owning project.
    pub project_ref: ProjectRef,
    /// Current backlog availability state.
    pub backlog_state: BacklogState,
    /// Maintenance reason for lock / reopen availability transitions.
    pub reason: BacklogMaintenanceReason,
}

/// Project member change event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemberChangedEvent {
    /// Changed project member.
    pub project_member_ref: ProjectMemberRef,
    /// Owning project.
    pub project_ref: ProjectRef,
    /// Referenced identity member.
    pub member_ref: GlobalMemberRef,
    /// Current responsibility state.
    pub responsibility_state: ProjectMemberResponsibilityState,
}

/// Formal work change event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkItemChangedEvent {
    /// Changed formal work.
    pub work_ref: FormalWorkRef,
    /// Owning project.
    pub project_ref: ProjectRef,
    /// Current work state.
    pub work_state: WorkItemState,
    /// Source reference when relevant.
    pub source_ref: Option<SourceWorkRef>,
    /// Completion evidence when relevant.
    pub evidence_ref: Option<ExternalEvidenceRef>,
}

/// Promote result event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromoteResultRecordedEvent {
    /// Promote result reference.
    pub promote_result_ref: PromoteResultRef,
    /// Source that was reviewed.
    pub source_ref: SourceWorkRef,
    /// Current promote state.
    pub result_state: PromoteResultState,
    /// Created formal work when accepted.
    pub created_work_ref: Option<FormalWorkRef>,
}

/// Dependency change event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkDependencyChangedEvent {
    /// Changed dependency.
    pub dependency_ref: WorkDependencyRef,
    /// Upstream work.
    pub upstream_work_ref: FormalWorkRef,
    /// Downstream work.
    pub downstream_work_ref: FormalWorkRef,
    /// Current dependency state.
    pub dependency_state: DependencyState,
}

/// Blocker change event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkBlockerChangedEvent {
    /// Changed blocker.
    pub blocker_ref: WorkBlockerRef,
    /// Blocked work.
    pub blocked_work_ref: FormalWorkRef,
    /// Current blocker state.
    pub blocker_state: BlockerState,
    /// Evidence when resolved; sourced from committed blocker truth.
    pub evidence_ref: Option<ExternalEvidenceRef>,
}

/// Iteration change event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IterationChangedEvent {
    /// Changed iteration.
    pub iteration_ref: IterationRef,
    /// Owning project.
    pub project_ref: ProjectRef,
    /// Current iteration state.
    pub iteration_state: IterationState,
    /// Current commitment state when available.
    pub commitment_state: Option<CommitmentState>,
    /// Affected formal work refs.
    pub affected_work_refs: Vec<FormalWorkRef>,
}

/// Trace availability event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkTraceAvailableEvent {
    /// Trace subject.
    pub subject_ref: WorkTraceSubjectRef,
    /// Trace record id.
    pub trace_id: WorkTraceId,
    /// Optional handoff reference.
    pub handoff_ref: Option<TraceHandoffRef>,
}

/// Derived view change event payload.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerivedWorkViewChangedEvent {
    /// Changed derived view.
    pub view_ref: DerivedWorkViewRef,
    /// Current freshness state.
    pub freshness_state: DerivedFreshnessState,
    /// Source cursor covered by the view.
    pub source_cursor: WorkTruthCursor,
}
