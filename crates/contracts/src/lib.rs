//! Shared contracts for the Work bounded context.

pub mod commands;
pub mod errors;
pub mod handoff;
pub mod metadata;
pub mod refs;
pub mod states;

pub use commands::{
    AssignProjectMemberRequest, BacklogCommandResult, BlockerCommandResult,
    CreateChildWorkItemRequest, CreateProjectRequest, CreateWorkItemRequest,
    DependencyCommandResult, IdempotencyResultView, LinkWorkDependencyRequest,
    OpenWorkBlockerRequest, ProjectCommandResult, ProjectMemberCommandResult,
    ProjectResponsibilitySpec, ProjectSpec, PromoteCommandResult, RequestWorkPromotionRequest,
    ResolveWorkBlockerRequest, ReviewWorkPromotionRequest, UpdateBacklogAvailabilityRequest,
    UpdateProjectLifecycleRequest, UpdateProjectMemberResponsibilityRequest,
    UpdateWorkDependencyStateRequest, UpdateWorkItemLifecycleRequest, WorkCommandEnvelope,
    WorkItemCommandResult,
};
pub use errors::WorkProtocolError;
pub use handoff::{ApplicationResultRef, WorkCommandReceipt, WorkTraceContextRef};
pub use metadata::fixtures;
pub use refs::{
    ArchiveHandoffRef, BacklogId, BacklogMaintenanceReason, BacklogMaintenanceReasonKind,
    BacklogRef, BlockerCauseRef, BlockerCloseReason, BlockerCloseReasonKind,
    BlockerImpactExplanation, BlockerMitigationReason, BlockerMitigationReasonKind, CapabilityRef,
    CapabilityRefSet, ChildWorkItemId, DependencyChangeId, DependencyChangeReason,
    DependencyChangeReasonKind, DependencyOrBlockerRef, DependencyReason, DependencyReasonKind,
    DependencyTarget, DerivedWorkViewKind, DerivedWorkViewRef, DerivedWorkViewScopeRef,
    EvidenceKind, EvidenceVerifiedState, ExternalEvidenceRef, ExternalSourceRef,
    ExternalSourceSummary, ExternalSourceSystem, FormalWorkCandidateSummary, FormalWorkIntent,
    FormalWorkRef, GlobalMemberRef, IterationId, IterationRef, MethodDefinitionKind,
    MethodDefinitionRef, OutboxFailureReason, OutboxFailureReasonKind, OutboxPublicationRef,
    OutboxRetryReason, ProjectId, ProjectLifecycleReason, ProjectLifecycleReasonKind,
    ProjectLifecycleTarget, ProjectMemberId, ProjectMemberReason, ProjectMemberReasonKind,
    ProjectMemberRef, ProjectOwnerKind, ProjectOwnerRef, ProjectRef, ProjectResponsibilityKind,
    PromoteDecision, PromoteDecisionId, PromoteReason, PromoteReasonKind, PromoteRejectReason,
    PromoteRejectReasonKind, PromoteResultId, PromoteResultRef, PromoteReviewDecision,
    ResponsibilityTarget, ResultId, SafeSummaryText, SourceDigest, SourceEventId, SourceWorkKind,
    SourceWorkRef, TraceHandoffIntent, TraceHandoffRef, TraceHandoffTargetKind,
    TraceHandoffTargetRef, WorkAuditSubjectRef, WorkAuditTrailId, WorkBlockerId, WorkBlockerRef,
    WorkDependencyId, WorkDependencyRef, WorkItemId, WorkLifecycleReason, WorkLifecycleReasonKind,
    WorkLifecycleTarget, WorkOutboxEventKind, WorkOutboxId, WorkPolicyScope, WorkTitle,
    WorkTraceId, WorkTraceRecordRefSet, WorkTraceSubjectRef, WorkTruthChange, WorkTruthCursor,
    WorkTruthSnapshot,
};
pub use states::{
    BacklogAvailabilityTarget, BacklogState, BlockerState, DependencyState, OutboxPublicationState,
    ProjectLifecycleState, ProjectMemberResponsibilityState, PromoteResultState, WorkItemState,
};

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde::de::DeserializeOwned;

    use super::commands::{
        AssignProjectMemberRequest, BacklogCommandResult, BlockerCommandResult,
        CreateChildWorkItemRequest, CreateProjectRequest, CreateWorkItemRequest,
        DependencyCommandResult, IdempotencyResultView, LinkWorkDependencyRequest,
        OpenWorkBlockerRequest, ProjectCommandResult, ProjectMemberCommandResult,
        PromoteCommandResult, RequestWorkPromotionRequest, ResolveWorkBlockerRequest,
        ReviewWorkPromotionRequest, UpdateBacklogAvailabilityRequest,
        UpdateProjectLifecycleRequest, UpdateProjectMemberResponsibilityRequest,
        UpdateWorkDependencyStateRequest, UpdateWorkItemLifecycleRequest, WorkCommandEnvelope,
        WorkItemCommandResult,
    };
    use super::handoff::{WorkCommandReceipt, WorkTraceContextRef};
    use super::metadata::fixtures;
    use super::refs::{
        BacklogMaintenanceReason, BacklogMaintenanceReasonKind, DependencyOrBlockerRef,
        DependencyTarget, ProjectLifecycleReason, ProjectLifecycleReasonKind,
        ProjectLifecycleTarget, ProjectMemberReason, ProjectMemberReasonKind,
        PromoteReviewDecision, ResponsibilityTarget, TraceHandoffIntent, TraceHandoffTargetKind,
        TraceHandoffTargetRef, WorkLifecycleTarget, WorkTruthChange,
    };
    use super::states::{
        BacklogAvailabilityTarget, BacklogState, BlockerState, DependencyState,
        ProjectLifecycleState, ProjectMemberResponsibilityState, PromoteResultState, WorkItemState,
    };

    fn roundtrip<T>(value: &T)
    where
        T: Clone + core::fmt::Debug + DeserializeOwned + Eq + Serialize,
    {
        let encoded = serde_json::to_value(value).expect("value should serialize");
        let decoded: T =
            serde_json::from_value(encoded).expect("value should deserialize after roundtrip");
        assert_eq!(&decoded, value);
    }

    #[test]
    fn project_and_backlog_commands_roundtrip() {
        roundtrip(&CreateProjectRequest {
            project_spec: fixtures::project_spec(),
        });
        roundtrip(&AssignProjectMemberRequest {
            project_ref: fixtures::project_ref(),
            member_ref: fixtures::global_member_ref(),
            responsibility_spec: fixtures::responsibility_spec(),
        });
        roundtrip(&UpdateProjectLifecycleRequest {
            project_ref: fixtures::project_ref(),
            target: ProjectLifecycleTarget::Closed,
            reason: ProjectLifecycleReason {
                reason_kind: ProjectLifecycleReasonKind::Maintenance,
                reason_ref: None,
                note: Some(fixtures::safe_summary("close for maintenance")),
            },
            expected_version: 3,
        });
        roundtrip(&UpdateBacklogAvailabilityRequest {
            backlog_ref: fixtures::backlog_ref(),
            target: BacklogAvailabilityTarget::LockedForMaintenance,
            reason: BacklogMaintenanceReason {
                reason_kind: BacklogMaintenanceReasonKind::MaintenanceWindow,
                reason_ref: None,
            },
            expected_version: 4,
        });
        roundtrip(&UpdateProjectMemberResponsibilityRequest {
            project_member_ref: fixtures::project_member_ref(),
            target: ResponsibilityTarget::Released,
            reason: ProjectMemberReason {
                reason_kind: ProjectMemberReasonKind::Released,
                reason_ref: None,
            },
            expected_version: 2,
        });
        roundtrip(&CreateWorkItemRequest {
            project_ref: fixtures::project_ref(),
            work_intent: fixtures::formal_work_intent(),
            source_ref: fixtures::source_work_ref(),
        });
        roundtrip(&CreateChildWorkItemRequest {
            parent_ref: fixtures::formal_work_ref(),
            work_intent: fixtures::child_work_intent(),
            source_ref: fixtures::source_work_ref(),
        });
        roundtrip(&UpdateWorkItemLifecycleRequest {
            work_ref: fixtures::formal_work_ref(),
            target: WorkLifecycleTarget::Completed,
            reason: fixtures::completion_work_reason(),
            evidence_ref: Some(fixtures::completion_evidence_ref()),
            expected_version: 7,
        });
        roundtrip(&RequestWorkPromotionRequest {
            source_ref: fixtures::source_work_ref(),
            reason: fixtures::promote_reason(),
        });
        roundtrip(&ReviewWorkPromotionRequest {
            promote_result_ref: fixtures::promote_result_ref(),
            decision: PromoteReviewDecision::Reject(fixtures::promote_reject_reason()),
            accepted_work_intent: None,
            expected_version: 2,
        });
        roundtrip(&LinkWorkDependencyRequest {
            upstream_work_ref: fixtures::formal_work_ref(),
            downstream_work_ref: fixtures::downstream_formal_work_ref(),
            reason: fixtures::dependency_reason(),
        });
        roundtrip(&UpdateWorkDependencyStateRequest {
            dependency_ref: fixtures::work_dependency_ref(),
            target: DependencyTarget::Active,
            reason: fixtures::dependency_activated_reason(),
            evidence_ref: None,
            expected_version: 5,
        });
        roundtrip(&OpenWorkBlockerRequest {
            blocked_work_ref: fixtures::formal_work_ref(),
            cause_ref: fixtures::blocker_cause_ref(),
        });
        roundtrip(&ResolveWorkBlockerRequest {
            blocker_ref: fixtures::work_blocker_ref(),
            evidence_ref: fixtures::blocker_resolution_evidence_ref(),
            expected_version: 6,
        });
    }

    #[test]
    fn command_envelope_and_receipt_roundtrip() {
        roundtrip(&WorkCommandEnvelope {
            actor: fixtures::actor_context(),
            metadata: fixtures::command_metadata("idem-project"),
            command: CreateProjectRequest {
                project_spec: fixtures::project_spec(),
            },
        });

        let receipt = WorkCommandReceipt {
            result_ref: fixtures::application_result_ref("create_project", "result-1"),
            idempotency: IdempotencyResultView::Applied,
            trace_ref: Some(fixtures::trace_id()),
            outbox_record_refs: vec![fixtures::outbox_id()],
            applied_version: Some(1),
        };

        roundtrip(&receipt);
        roundtrip(&ProjectCommandResult {
            project_ref: fixtures::project_ref(),
            lifecycle_state: ProjectLifecycleState::Active,
            receipt: receipt.clone(),
        });
        roundtrip(&BacklogCommandResult {
            backlog_ref: fixtures::backlog_ref(),
            backlog_state: BacklogState::Open,
            receipt,
        });
        roundtrip(&ProjectMemberCommandResult {
            project_member_ref: fixtures::project_member_ref(),
            responsibility_state: ProjectMemberResponsibilityState::Active,
            receipt: WorkCommandReceipt {
                result_ref: fixtures::application_result_ref("assign_project_member", "result-2"),
                idempotency: IdempotencyResultView::Applied,
                trace_ref: Some(fixtures::trace_id()),
                outbox_record_refs: vec![fixtures::outbox_id()],
                applied_version: Some(1),
            },
        });
        roundtrip(&WorkItemCommandResult {
            work_ref: fixtures::formal_work_ref(),
            work_state: WorkItemState::Formalized,
            receipt: WorkCommandReceipt {
                result_ref: fixtures::application_result_ref("create_work_item", "result-3"),
                idempotency: IdempotencyResultView::Applied,
                trace_ref: Some(fixtures::trace_id()),
                outbox_record_refs: vec![fixtures::outbox_id()],
                applied_version: Some(1),
            },
        });
        roundtrip(&PromoteCommandResult {
            promote_result_ref: fixtures::promote_result_ref(),
            result_state: PromoteResultState::PendingReview,
            created_work_ref: None,
            receipt: WorkCommandReceipt {
                result_ref: fixtures::application_result_ref("request_work_promotion", "result-4"),
                idempotency: IdempotencyResultView::Applied,
                trace_ref: Some(fixtures::trace_id()),
                outbox_record_refs: vec![fixtures::outbox_id()],
                applied_version: Some(1),
            },
        });
        roundtrip(&DependencyCommandResult {
            dependency_ref: fixtures::work_dependency_ref(),
            dependency_state: DependencyState::Active,
            receipt: WorkCommandReceipt {
                result_ref: fixtures::application_result_ref("link_work_dependency", "result-5"),
                idempotency: IdempotencyResultView::Applied,
                trace_ref: Some(fixtures::trace_id()),
                outbox_record_refs: vec![fixtures::outbox_id()],
                applied_version: Some(1),
            },
        });
        roundtrip(&BlockerCommandResult {
            blocker_ref: fixtures::work_blocker_ref(),
            blocker_state: BlockerState::Open,
            receipt: WorkCommandReceipt {
                result_ref: fixtures::application_result_ref("open_work_blocker", "result-6"),
                idempotency: IdempotencyResultView::Applied,
                trace_ref: Some(fixtures::trace_id()),
                outbox_record_refs: vec![fixtures::outbox_id()],
                applied_version: Some(1),
            },
        });
    }

    #[test]
    fn trace_handoff_and_truth_change_helpers_roundtrip() {
        roundtrip(&TraceHandoffIntent {
            trace_id: fixtures::trace_id(),
            target_ref: TraceHandoffTargetRef {
                target_kind: TraceHandoffTargetKind::Observability,
                external_ref: fixtures::external_source_ref(),
            },
            subject_ref: fixtures::project_trace_subject(),
        });

        roundtrip(&WorkTruthChange::ProjectCreated(fixtures::project_ref()));
        roundtrip(&WorkTruthChange::ProjectMemberChanged(
            fixtures::project_member_ref(),
        ));
        roundtrip(&WorkTruthChange::BacklogAvailabilityChanged(
            fixtures::backlog_ref(),
        ));
        roundtrip(&WorkTruthChange::WorkItemChanged(
            fixtures::formal_work_ref(),
        ));
        roundtrip(&WorkTruthChange::PromoteResultRecorded(
            fixtures::promote_result_ref(),
        ));
        roundtrip(&WorkTruthChange::WorkRelationChanged(
            DependencyOrBlockerRef::Dependency(fixtures::work_dependency_ref()),
        ));
        roundtrip(&WorkTruthChange::WorkRelationChanged(
            DependencyOrBlockerRef::Blocker(fixtures::work_blocker_ref()),
        ));
        roundtrip(&WorkTraceContextRef::from_metadata(
            &fixtures::request_metadata(None),
        ));
    }
}
