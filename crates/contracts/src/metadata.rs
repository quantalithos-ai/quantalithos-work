//! Shared fixtures and metadata helpers for contract and domain tests.

use core_contracts::{
    actor::{ActorContext, ActorKind, ActorRef, RequestOrigin},
    metadata::{
        CommandMetadata, IdempotencyKey, OperationName, RequestId, RequestMetadata, Timestamp,
        TraceId,
    },
};

use crate::{
    commands::{ProjectResponsibilitySpec, ProjectSpec},
    handoff::{ApplicationResultRef, WorkTraceContextRef},
    refs::{
        BacklogId, BacklogRef, BlockerCauseRef, CapabilityRef, CapabilityRefSet, ChildWorkItemId,
        DependencyChangeReason, DependencyChangeReasonKind, DependencyOrBlockerRef,
        DependencyReason, DependencyReasonKind, EvidenceKind, EvidenceVerifiedState,
        ExternalEvidenceRef, ExternalSourceRef, ExternalSourceSystem, FormalWorkIntent,
        FormalWorkRef, GlobalMemberRef, IterationId, IterationRef, MethodDefinitionRef, ProjectId,
        ProjectMemberId, ProjectMemberRef, ProjectOwnerKind, ProjectOwnerRef, ProjectRef,
        ProjectResponsibilityKind, PromoteReason, PromoteReasonKind, PromoteRejectReason,
        PromoteRejectReasonKind, PromoteResultId, PromoteResultRef, ResultId, SafeSummaryText,
        SourceDigest, SourceEventId, SourceWorkKind, SourceWorkRef, TraceHandoffRef,
        WorkAuditSubjectRef, WorkAuditTrailId, WorkBlockerId, WorkBlockerRef, WorkDependencyId,
        WorkDependencyRef, WorkItemId, WorkLifecycleReason, WorkLifecycleReasonKind, WorkOutboxId,
        WorkTitle, WorkTraceId, WorkTraceRecordRefSet, WorkTraceSubjectRef, WorkTruthChange,
    },
};

/// Test fixture builders for `commit-02-a` contract/domain slices.
pub mod fixtures {
    use super::*;

    /// Returns a deterministic actor context for tests.
    pub fn actor_context() -> ActorContext {
        ActorContext::new(
            ActorRef::new("work-actor-1", ActorKind::Human),
            RequestOrigin::Command,
        )
    }

    /// Returns deterministic request metadata.
    pub fn request_metadata(idempotency_key: Option<&str>) -> RequestMetadata {
        RequestMetadata::new(
            RequestId::new("request-1"),
            TraceId::new("trace-1"),
            idempotency_key.map(IdempotencyKey::new),
            Timestamp::new("2026-06-05T09:00:00Z"),
        )
    }

    /// Returns deterministic command metadata.
    pub fn command_metadata(idempotency_key: &str) -> CommandMetadata {
        CommandMetadata {
            request: request_metadata(Some(idempotency_key)),
            reason: None,
            external_ref: None,
        }
    }

    /// Returns a deterministic safe summary.
    pub fn safe_summary(value: &str) -> SafeSummaryText {
        SafeSummaryText(value.to_owned())
    }

    /// Returns a deterministic external source ref.
    pub fn external_source_ref() -> ExternalSourceRef {
        ExternalSourceRef {
            source_system: ExternalSourceSystem::Workspace,
            external_id: "workspace/projects/l1-work".to_owned(),
        }
    }

    /// Returns a deterministic project owner ref.
    pub fn project_owner_ref() -> ProjectOwnerRef {
        ProjectOwnerRef {
            owner_kind: ProjectOwnerKind::WorkspaceProject,
            external_ref: external_source_ref(),
        }
    }

    /// Returns a deterministic project spec.
    pub fn project_spec() -> ProjectSpec {
        ProjectSpec {
            owner_ref: project_owner_ref(),
            source_ref: None,
        }
    }

    /// Returns a deterministic project id.
    pub fn project_id() -> ProjectId {
        ProjectId("project-1".to_owned())
    }

    /// Returns a deterministic project ref.
    pub fn project_ref() -> ProjectRef {
        ProjectRef {
            project_id: project_id(),
        }
    }

    /// Returns a deterministic project member id.
    pub fn project_member_id() -> ProjectMemberId {
        ProjectMemberId("project-member-1".to_owned())
    }

    /// Returns a deterministic project member ref.
    pub fn project_member_ref() -> ProjectMemberRef {
        ProjectMemberRef {
            project_member_id: project_member_id(),
        }
    }

    /// Returns a deterministic global member ref.
    pub fn global_member_ref() -> GlobalMemberRef {
        GlobalMemberRef("global-member-1".to_owned())
    }

    /// Returns a deterministic capability ref set.
    pub fn capability_ref_set() -> CapabilityRefSet {
        CapabilityRefSet {
            refs: vec![CapabilityRef("capability.assign".to_owned())],
        }
    }

    /// Returns a deterministic project responsibility spec.
    pub fn responsibility_spec() -> ProjectResponsibilitySpec {
        ProjectResponsibilitySpec {
            responsibility_kind: ProjectResponsibilityKind::Contributor,
            required_capability_refs: capability_ref_set(),
        }
    }

    /// Returns a deterministic backlog id.
    pub fn backlog_id() -> BacklogId {
        BacklogId("backlog-1".to_owned())
    }

    /// Returns a deterministic backlog ref.
    pub fn backlog_ref() -> BacklogRef {
        BacklogRef {
            backlog_id: backlog_id(),
        }
    }

    /// Returns a deterministic work item id.
    pub fn work_item_id() -> WorkItemId {
        WorkItemId("work-item-1".to_owned())
    }

    /// Returns a deterministic child work item id.
    pub fn child_work_item_id() -> ChildWorkItemId {
        ChildWorkItemId("child-work-item-1".to_owned())
    }

    /// Returns a deterministic formal work ref.
    pub fn formal_work_ref() -> FormalWorkRef {
        FormalWorkRef::WorkItem(work_item_id())
    }

    /// Returns a deterministic child formal work ref.
    pub fn child_formal_work_ref() -> FormalWorkRef {
        FormalWorkRef::ChildWorkItem(child_work_item_id())
    }

    /// Returns a deterministic work dependency id.
    pub fn work_dependency_id() -> WorkDependencyId {
        WorkDependencyId("dependency-1".to_owned())
    }

    /// Returns a deterministic work dependency ref.
    pub fn work_dependency_ref() -> WorkDependencyRef {
        WorkDependencyRef {
            dependency_id: work_dependency_id(),
        }
    }

    /// Returns a deterministic second formal work ref for dependency tests.
    pub fn downstream_formal_work_ref() -> FormalWorkRef {
        FormalWorkRef::ChildWorkItem(ChildWorkItemId("child-work-item-2".to_owned()))
    }

    /// Returns a deterministic work blocker id.
    pub fn work_blocker_id() -> WorkBlockerId {
        WorkBlockerId("blocker-1".to_owned())
    }

    /// Returns a deterministic work blocker ref.
    pub fn work_blocker_ref() -> WorkBlockerRef {
        WorkBlockerRef {
            blocker_id: work_blocker_id(),
        }
    }

    /// Returns a deterministic iteration ref.
    pub fn iteration_ref() -> IterationRef {
        IterationRef {
            iteration_id: IterationId("iteration-1".to_owned()),
        }
    }

    /// Returns a deterministic work title.
    pub fn work_title(value: &str) -> WorkTitle {
        WorkTitle(value.to_owned())
    }

    /// Returns a deterministic method definition ref.
    pub fn method_definition_ref() -> MethodDefinitionRef {
        MethodDefinitionRef("method-definition-1".to_owned())
    }

    /// Returns a deterministic source work ref.
    pub fn source_work_ref() -> SourceWorkRef {
        SourceWorkRef {
            source_kind: SourceWorkKind::Conversation,
            external_ref: external_source_ref(),
            source_digest: Some(SourceDigest("digest-1".to_owned())),
        }
    }

    /// Returns a deterministic runtime source work ref.
    pub fn runtime_source_work_ref() -> SourceWorkRef {
        SourceWorkRef {
            source_kind: SourceWorkKind::Runtime,
            external_ref: external_source_ref(),
            source_digest: Some(SourceDigest("runtime-digest-1".to_owned())),
        }
    }

    /// Returns a deterministic completion evidence ref.
    pub fn completion_evidence_ref() -> ExternalEvidenceRef {
        ExternalEvidenceRef {
            evidence_kind: EvidenceKind::Completion,
            external_ref: external_source_ref(),
            verified_state: EvidenceVerifiedState::Verified,
        }
    }

    /// Returns a deterministic unverified completion evidence ref.
    pub fn unverified_completion_evidence_ref() -> ExternalEvidenceRef {
        ExternalEvidenceRef {
            evidence_kind: EvidenceKind::Completion,
            external_ref: external_source_ref(),
            verified_state: EvidenceVerifiedState::Unverified,
        }
    }

    /// Returns deterministic blocker resolution evidence.
    pub fn blocker_resolution_evidence_ref() -> ExternalEvidenceRef {
        ExternalEvidenceRef {
            evidence_kind: EvidenceKind::BlockerResolution,
            external_ref: external_source_ref(),
            verified_state: EvidenceVerifiedState::Verified,
        }
    }

    /// Returns deterministic unverified blocker resolution evidence.
    pub fn unverified_blocker_resolution_evidence_ref() -> ExternalEvidenceRef {
        ExternalEvidenceRef {
            evidence_kind: EvidenceKind::BlockerResolution,
            external_ref: external_source_ref(),
            verified_state: EvidenceVerifiedState::Unverified,
        }
    }

    /// Returns a deterministic formal work intent.
    pub fn formal_work_intent() -> FormalWorkIntent {
        FormalWorkIntent {
            title: work_title("Formal work"),
            method_definition_ref: Some(method_definition_ref()),
            assignee_ref: project_member_ref(),
            parent_ref: None,
        }
    }

    /// Returns a deterministic child work intent.
    pub fn child_work_intent() -> FormalWorkIntent {
        FormalWorkIntent {
            title: work_title("Child work"),
            method_definition_ref: Some(method_definition_ref()),
            assignee_ref: project_member_ref(),
            parent_ref: Some(formal_work_ref()),
        }
    }

    /// Returns a deterministic work lifecycle reason for starting work.
    pub fn start_work_reason() -> WorkLifecycleReason {
        WorkLifecycleReason {
            reason_kind: WorkLifecycleReasonKind::Start,
            superseding_ref: None,
            reason_ref: None,
        }
    }

    /// Returns a deterministic cancellation reason.
    pub fn cancellation_work_reason() -> WorkLifecycleReason {
        WorkLifecycleReason {
            reason_kind: WorkLifecycleReasonKind::Cancellation,
            superseding_ref: None,
            reason_ref: None,
        }
    }

    /// Returns a deterministic completion reason.
    pub fn completion_work_reason() -> WorkLifecycleReason {
        WorkLifecycleReason {
            reason_kind: WorkLifecycleReasonKind::CompletionEvidence,
            superseding_ref: None,
            reason_ref: Some(completion_evidence_ref()),
        }
    }

    /// Returns a deterministic superseded reason.
    pub fn superseded_work_reason() -> WorkLifecycleReason {
        WorkLifecycleReason {
            reason_kind: WorkLifecycleReasonKind::Superseded,
            superseding_ref: Some(child_formal_work_ref()),
            reason_ref: None,
        }
    }

    /// Returns a deterministic trace id.
    pub fn trace_id() -> WorkTraceId {
        WorkTraceId("trace-record-1".to_owned())
    }

    /// Returns a deterministic audit trail id.
    pub fn audit_trail_id() -> WorkAuditTrailId {
        WorkAuditTrailId("audit-1".to_owned())
    }

    /// Returns a deterministic outbox id.
    pub fn outbox_id() -> WorkOutboxId {
        WorkOutboxId("outbox-1".to_owned())
    }

    /// Returns a deterministic handoff ref.
    pub fn trace_handoff_ref() -> TraceHandoffRef {
        TraceHandoffRef("handoff-1".to_owned())
    }

    /// Returns a deterministic application result ref.
    pub fn application_result_ref(operation: &str, result_id: &str) -> ApplicationResultRef {
        ApplicationResultRef {
            operation: OperationName::new(operation),
            result_id: ResultId(result_id.to_owned()),
        }
    }

    /// Returns a deterministic promote result id.
    pub fn promote_result_id() -> PromoteResultId {
        PromoteResultId("promote-result-1".to_owned())
    }

    /// Returns a deterministic promote result ref.
    pub fn promote_result_ref() -> PromoteResultRef {
        PromoteResultRef {
            promote_result_id: promote_result_id(),
        }
    }

    /// Returns a deterministic promote request reason.
    pub fn promote_reason() -> PromoteReason {
        PromoteReason {
            reason_kind: PromoteReasonKind::ManualReview,
            source_summary_ref: Some(source_work_ref()),
        }
    }

    /// Returns a deterministic promote reject reason.
    pub fn promote_reject_reason() -> PromoteRejectReason {
        PromoteRejectReason {
            reason_kind: PromoteRejectReasonKind::PolicyRejected,
            reason_ref: Some(completion_evidence_ref()),
        }
    }

    /// Returns a deterministic dependency link reason.
    pub fn dependency_reason() -> DependencyReason {
        DependencyReason {
            reason_kind: DependencyReasonKind::ExplicitOrdering,
            reason_ref: None,
        }
    }

    /// Returns an activation reason for dependency state transitions.
    pub fn dependency_activated_reason() -> DependencyChangeReason {
        DependencyChangeReason {
            reason_kind: DependencyChangeReasonKind::Activated,
            reason_ref: None,
            blocker_cause_ref: None,
        }
    }

    /// Returns a satisfied reason for dependency state transitions.
    pub fn dependency_satisfied_reason() -> DependencyChangeReason {
        DependencyChangeReason {
            reason_kind: DependencyChangeReasonKind::SatisfiedByEvidence,
            reason_ref: Some(completion_evidence_ref()),
            blocker_cause_ref: None,
        }
    }

    /// Returns a waived reason for dependency state transitions.
    pub fn dependency_waived_reason() -> DependencyChangeReason {
        DependencyChangeReason {
            reason_kind: DependencyChangeReasonKind::Waived,
            reason_ref: None,
            blocker_cause_ref: None,
        }
    }

    /// Returns a cancelled reason for dependency state transitions.
    pub fn dependency_cancelled_reason() -> DependencyChangeReason {
        DependencyChangeReason {
            reason_kind: DependencyChangeReasonKind::Cancelled,
            reason_ref: None,
            blocker_cause_ref: None,
        }
    }

    /// Returns a mismatched dependency change reason for negative tests.
    pub fn dependency_mismatched_reason() -> DependencyChangeReason {
        DependencyChangeReason {
            reason_kind: DependencyChangeReasonKind::Waived,
            reason_ref: None,
            blocker_cause_ref: None,
        }
    }

    /// Returns a deterministic blocker cause ref.
    pub fn blocker_cause_ref() -> BlockerCauseRef {
        BlockerCauseRef {
            source_ref: external_source_ref(),
            evidence_ref: Some(blocker_resolution_evidence_ref()),
        }
    }

    /// Returns a deterministic source event id.
    pub fn source_event_id() -> SourceEventId {
        SourceEventId("source-event-1".to_owned())
    }

    /// Returns a deterministic project trace subject.
    pub fn project_trace_subject() -> WorkTraceSubjectRef {
        WorkTraceSubjectRef::Project(project_ref())
    }

    /// Returns a deterministic project audit subject.
    pub fn project_audit_subject() -> WorkAuditSubjectRef {
        WorkAuditSubjectRef::Project(project_ref())
    }

    /// Returns a deterministic trace record ref set.
    pub fn trace_record_ref_set() -> WorkTraceRecordRefSet {
        WorkTraceRecordRefSet {
            trace_ids: vec![trace_id()],
        }
    }

    /// Returns a deterministic project-created change.
    pub fn project_created_change() -> WorkTruthChange {
        WorkTruthChange::ProjectCreated(project_ref())
    }

    /// Returns a deterministic backlog-availability change.
    pub fn backlog_changed_change() -> WorkTruthChange {
        WorkTruthChange::BacklogAvailabilityChanged(backlog_ref())
    }

    /// Returns a deterministic project-member change.
    pub fn project_member_changed_change() -> WorkTruthChange {
        WorkTruthChange::ProjectMemberChanged(project_member_ref())
    }

    /// Returns a deterministic work-item change.
    pub fn work_item_changed_change() -> WorkTruthChange {
        WorkTruthChange::WorkItemChanged(formal_work_ref())
    }

    /// Returns a deterministic promote-result change.
    pub fn promote_result_recorded_change() -> WorkTruthChange {
        WorkTruthChange::PromoteResultRecorded(promote_result_ref())
    }

    /// Returns a deterministic dependency change.
    pub fn dependency_changed_change() -> WorkTruthChange {
        WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Dependency(
            work_dependency_ref(),
        ))
    }

    /// Returns a deterministic blocker change.
    pub fn blocker_changed_change() -> WorkTruthChange {
        WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Blocker(work_blocker_ref()))
    }

    /// Returns a deterministic trace context ref.
    pub fn trace_context_ref() -> WorkTraceContextRef {
        WorkTraceContextRef::from_metadata(&request_metadata(None))
    }
}
