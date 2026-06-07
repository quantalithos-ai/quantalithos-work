//! Shared contracts for the Work bounded context.

pub mod commands;
pub mod errors;
pub mod events;
pub mod handoff;
pub mod jobs;
pub mod metadata;
pub mod queries;
pub mod refs;
pub mod states;
pub mod views;

pub use commands::{
    AssignProjectMemberRequest, BacklogCommandResult, BlockerCommandResult,
    CommitIterationScopeRequest, CreateChildWorkItemRequest, CreateProjectRequest,
    CreateWorkItemRequest, DependencyCommandResult, IdempotencyResultView, IterationCommandResult,
    LinkWorkDependencyRequest, OpenIterationRequest, OpenWorkBlockerRequest, ProjectCommandResult,
    ProjectMemberCommandResult, ProjectResponsibilitySpec, ProjectSpec, PromoteCommandResult,
    RequestWorkPromotionRequest, ResolveWorkBlockerRequest, ReviewWorkPromotionRequest,
    UpdateBacklogAvailabilityRequest, UpdateIterationCommitmentRequest,
    UpdateIterationLifecycleRequest, UpdateProjectLifecycleRequest,
    UpdateProjectMemberResponsibilityRequest, UpdateWorkDependencyStateRequest,
    UpdateWorkItemLifecycleRequest, WorkCommandEnvelope, WorkItemCommandResult,
};
pub use errors::WorkProtocolError;
pub use events::{
    ArtifactEvidenceChangedPayload, BacklogChangedEvent, ConversationWorkContextChangedPayload,
    DerivedWorkViewChangedEvent, EventSchemaVersion, GovernanceDecisionChangedPayload,
    IdentityMemberChangedPayload, IterationChangedEvent, MethodDefinitionChangedPayload,
    ProcessTimingChangedPayload, ProjectChangedEvent, ProjectMemberChangedEvent,
    PromoteResultRecordedEvent, RuntimePromoteRequestedPayload, WorkBlockerChangedEvent,
    WorkDependencyChangedEvent, WorkInboundEventEnvelope, WorkItemChangedEvent,
    WorkOutboundEventEnvelope, WorkTraceAvailableEvent,
};
pub use handoff::{ApplicationResultRef, WorkCommandReceipt, WorkTraceContextRef};
pub use jobs::{
    PrepareArchiveHandoffJobInput, PrepareWorkTraceHandoffJobInput, PublishWorkOutboxJobInput,
    RebuildWorkProjectionsJobInput, RefreshExternalReferenceSnapshotsJobInput,
    RunWorkReconciliationJobInput, WorkJobMetadata, WorkJobReport, WorkProjectionSet,
};
pub use metadata::fixtures;
pub use queries::{
    BacklogQueryFilter, GetBacklogRequest, GetIterationSummaryRequest, GetProjectBoardViewRequest,
    GetProjectWorkFactsRequest, GetWorkItemRequest, GetWorkTraceRequest, ListMemberWorkRequest,
    ProjectBoardView, ProjectMemberSummaryView, ProjectWorkFactsView, ProjectionViewMarker,
    PublicPageInfo, QuerySurface, SearchWorkRequest, WorkItemView, WorkQueryEnvelope,
    WorkQueryResponse, WorkRelationStateView, WorkRelationSummaryView, WorkSearchCriteria,
    WorkSearchProjection, WorkSearchResult, WorkTraceRecordView, WorkTraceView,
};
pub use refs::{
    ArchiveHandoffRef, ArchiveHandoffScope, ArchiveHandoffScopeKind, ArchiveHandoffTargetKind,
    ArchiveHandoffTargetRef, BacklogId, BacklogMaintenanceReason, BacklogMaintenanceReasonKind,
    BacklogRef, BlockerCauseRef, BlockerCloseReason, BlockerCloseReasonKind,
    BlockerImpactExplanation, BlockerMitigationReason, BlockerMitigationReasonKind, CapabilityRef,
    CapabilityRefSet, ChildWorkItemId, CommitmentChangeReason, CommitmentChangeReasonKind,
    DependencyChangeId, DependencyChangeReason, DependencyChangeReasonKind, DependencyOrBlockerRef,
    DependencyReason, DependencyReasonKind, DependencyTarget, DerivedWorkViewKind,
    DerivedWorkViewRef, DerivedWorkViewScopeRef, EvidenceKind, EvidenceVerifiedState,
    ExternalEvidenceRef, ExternalReferenceRef, ExternalReferenceScope, ExternalReferenceScopeKind,
    ExternalSourceRef, ExternalSourceSummary, ExternalSourceSystem, FormalWorkCandidateSummary,
    FormalWorkIntent, FormalWorkRef, FormalWorkRefSet, GlobalMemberRef, IterationChangeId,
    IterationChangeReason, IterationChangeReasonKind, IterationCloseReason,
    IterationCloseReasonKind, IterationCommitmentChangeSet, IterationCommitmentId, IterationId,
    IterationLifecycleTarget, IterationRef, JobRunId, MethodDefinitionKind, MethodDefinitionRef,
    OutboxFailureReason, OutboxFailureReasonKind, OutboxPublicationRef, OutboxRetryReason,
    ProcessTimeboxRef, ProcessTimeboxSummary, ProjectId, ProjectLifecycleReason,
    ProjectLifecycleReasonKind, ProjectLifecycleTarget, ProjectMemberId, ProjectMemberReason,
    ProjectMemberReasonKind, ProjectMemberRef, ProjectOwnerKind, ProjectOwnerRef, ProjectRef,
    ProjectResponsibilityKind, PromoteDecision, PromoteDecisionId, PromoteReason,
    PromoteReasonKind, PromoteRejectReason, PromoteRejectReasonKind, PromoteResultId,
    PromoteResultRef, PromoteReviewDecision, ResponsibilityTarget, ResultId, SafeSummaryText,
    SourceDigest, SourceEventId, SourceWorkKind, SourceWorkRef, TraceHandoffIntent,
    TraceHandoffRef, TraceHandoffTargetKind, TraceHandoffTargetRef, WorkAuditSubjectRef,
    WorkAuditTrailId, WorkBlockerId, WorkBlockerRef, WorkDependencyId, WorkDependencyRef,
    WorkItemId, WorkJobFailureRef, WorkLifecycleReason, WorkLifecycleReasonKind,
    WorkLifecycleTarget, WorkOutboundPublication, WorkOutboxEventKind, WorkOutboxId,
    WorkOutboxSourceRef, WorkPolicyScope, WorkReconciliationScopeKind, WorkReconciliationScopeRef,
    WorkSearchCriteriaDigest, WorkSearchText, WorkTitle, WorkTraceId, WorkTraceRecordRefSet,
    WorkTraceSubjectRef, WorkTruthChange, WorkTruthCursor, WorkTruthSnapshot,
};
pub use states::{
    BacklogAvailabilityTarget, BacklogState, BlockerState, CommitmentState, DependencyState,
    DerivedFreshnessState, IterationState, OutboxPublicationState, ProjectLifecycleState,
    ProjectMemberResponsibilityState, PromoteResultState, ReferenceResolutionStatus, WorkItemState,
};
pub use views::{
    BacklogTruthSummary, FormalWorkTruthSummary, IterationTruthSummary, ProjectMemberTruthSummary,
    ProjectProjectionBatch, ProjectTruthSummary, ProjectWorkTruthSnapshot,
    WorkRelationTruthSummary,
};

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde::de::DeserializeOwned;

    use core_contracts::metadata::{PageRequest, QueryConsistency, QueryMetadata};

    use super::commands::{
        AssignProjectMemberRequest, BacklogCommandResult, BlockerCommandResult,
        CommitIterationScopeRequest, CreateChildWorkItemRequest, CreateProjectRequest,
        CreateWorkItemRequest, DependencyCommandResult, IdempotencyResultView,
        IterationCommandResult, LinkWorkDependencyRequest, OpenIterationRequest,
        OpenWorkBlockerRequest, ProjectCommandResult, ProjectMemberCommandResult,
        PromoteCommandResult, RequestWorkPromotionRequest, ResolveWorkBlockerRequest,
        ReviewWorkPromotionRequest, UpdateBacklogAvailabilityRequest,
        UpdateIterationCommitmentRequest, UpdateIterationLifecycleRequest,
        UpdateProjectLifecycleRequest, UpdateProjectMemberResponsibilityRequest,
        UpdateWorkDependencyStateRequest, UpdateWorkItemLifecycleRequest, WorkCommandEnvelope,
        WorkItemCommandResult,
    };
    use super::events::{EventSchemaVersion, WorkInboundEventEnvelope, WorkOutboundEventEnvelope};
    use super::handoff::{WorkCommandReceipt, WorkTraceContextRef};
    use super::jobs::{WorkJobMetadata, WorkProjectionSet};
    use super::metadata::fixtures;
    use super::queries::{
        BacklogQueryFilter, BacklogView, FormalWorkSummaryView, GetBacklogRequest,
        GetIterationSummaryRequest, GetProjectBoardViewRequest, GetProjectWorkFactsRequest,
        GetWorkItemRequest, GetWorkTraceRequest, IterationSummaryView, ListMemberWorkRequest,
        MemberWorkView, ProjectBoardView, ProjectMemberSummaryView, ProjectWorkFactsView,
        ProjectionViewMarker, PublicPageInfo, QuerySurface, SearchWorkRequest, WorkItemView,
        WorkQueryEnvelope, WorkQueryResponse, WorkRelationStateView, WorkRelationSummaryView,
        WorkSearchResult, WorkTraceRecordView, WorkTraceView,
    };
    use super::refs::{
        ArchiveHandoffScope, ArchiveHandoffScopeKind, ArchiveHandoffTargetKind,
        ArchiveHandoffTargetRef, BacklogMaintenanceReason, BacklogMaintenanceReasonKind,
        DependencyOrBlockerRef, DependencyTarget, DerivedWorkViewRef, ExternalReferenceRef,
        ExternalReferenceScope, ExternalReferenceScopeKind, IterationLifecycleTarget, JobRunId,
        ProjectLifecycleReason, ProjectLifecycleReasonKind, ProjectLifecycleTarget,
        ProjectMemberReason, ProjectMemberReasonKind, PromoteReviewDecision, ResponsibilityTarget,
        TraceHandoffIntent, TraceHandoffTargetKind, TraceHandoffTargetRef, WorkJobFailureRef,
        WorkLifecycleTarget, WorkReconciliationScopeKind, WorkReconciliationScopeRef,
        WorkTruthChange,
    };
    use super::states::{
        BacklogAvailabilityTarget, BacklogState, BlockerState, CommitmentState, DependencyState,
        DerivedFreshnessState, IterationState, ProjectLifecycleState,
        ProjectMemberResponsibilityState, PromoteResultState, WorkItemState,
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
        roundtrip(&OpenIterationRequest {
            project_ref: fixtures::project_ref(),
            timebox_ref: fixtures::process_timebox_ref(),
        });
        roundtrip(&CommitIterationScopeRequest {
            iteration_ref: fixtures::iteration_ref(),
            candidate_work_refs: fixtures::formal_work_ref_set(),
            expected_iteration_version: 2,
        });
        roundtrip(&UpdateIterationCommitmentRequest {
            iteration_ref: fixtures::iteration_ref(),
            change_set: fixtures::iteration_commitment_change_set(),
            reason: fixtures::iteration_commitment_changed_reason(),
            expected_commitment_version: 3,
        });
        roundtrip(&UpdateIterationLifecycleRequest {
            iteration_ref: fixtures::iteration_ref(),
            target: IterationLifecycleTarget::InProgress,
            change_reason: Some(fixtures::iteration_started_reason()),
            close_reason: None,
            expected_version: 4,
        });
        roundtrip(&UpdateIterationLifecycleRequest {
            iteration_ref: fixtures::iteration_ref(),
            target: IterationLifecycleTarget::Closed,
            change_reason: None,
            close_reason: Some(fixtures::iteration_closed_reason()),
            expected_version: 5,
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
        roundtrip(&IterationCommandResult {
            iteration_ref: fixtures::iteration_ref(),
            iteration_state: IterationState::Committed,
            commitment_state: Some(CommitmentState::Committed),
            receipt: WorkCommandReceipt {
                result_ref: fixtures::application_result_ref("commit_iteration_scope", "result-7"),
                idempotency: IdempotencyResultView::Applied,
                trace_ref: Some(fixtures::trace_id()),
                outbox_record_refs: vec![fixtures::outbox_id()],
                applied_version: Some(2),
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

        roundtrip(&WorkTruthChange::ProjectCreated(
            fixtures::project_ref(),
            fixtures::project_created_reason(),
        ));
        roundtrip(&WorkTruthChange::ProjectMemberChanged(
            fixtures::project_member_ref(),
        ));
        roundtrip(&WorkTruthChange::BacklogAvailabilityChanged(
            fixtures::backlog_ref(),
            fixtures::backlog_changed_reason(),
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
        roundtrip(&WorkTruthChange::IterationChanged(fixtures::iteration_ref()));
        roundtrip(&WorkTraceContextRef::from_metadata(
            &fixtures::request_metadata(None),
        ));
    }

    #[test]
    fn event_schema_and_job_contracts_roundtrip() {
        roundtrip(&EventSchemaVersion::v1());
        roundtrip(&fixtures::event_schema_version());

        roundtrip(&ExternalReferenceRef::from_member(
            fixtures::global_member_ref(),
        ));
        roundtrip(&ExternalReferenceRef::from_method_definition(
            fixtures::method_definition_ref(),
        ));
        roundtrip(&ExternalReferenceRef::from_source_work(
            fixtures::source_work_ref(),
        ));
        roundtrip(&ExternalReferenceRef::from_evidence(
            fixtures::completion_evidence_ref(),
        ));
        roundtrip(&ExternalReferenceRef::from_process_timebox(
            fixtures::process_timebox_ref(),
        ));

        roundtrip(&WorkInboundEventEnvelope {
            source_event_id: fixtures::source_event_id(),
            source_ref: fixtures::external_source_ref(),
            event_version: fixtures::event_schema_version(),
            trace_context_ref: fixtures::trace_context_ref(),
            occurred_at: fixtures::request_metadata(None).requested_at,
            payload: fixtures::identity_member_changed_payload(),
        });
        roundtrip(&fixtures::inbound_event_envelope(
            fixtures::method_definition_changed_payload(),
        ));
        roundtrip(&fixtures::inbound_event_envelope(
            fixtures::conversation_work_context_changed_payload(),
        ));
        roundtrip(&fixtures::inbound_event_envelope(
            fixtures::process_timing_changed_payload(),
        ));
        roundtrip(&fixtures::inbound_event_envelope(
            fixtures::governance_decision_changed_payload(),
        ));
        roundtrip(&fixtures::inbound_event_envelope(
            fixtures::artifact_evidence_changed_payload(),
        ));
        roundtrip(&fixtures::inbound_event_envelope(
            fixtures::runtime_promote_requested_payload(),
        ));

        roundtrip(&WorkOutboundEventEnvelope {
            outbox_id: fixtures::outbox_id(),
            event_version: fixtures::event_schema_version(),
            trace_context_ref: fixtures::trace_context_ref(),
            occurred_at: fixtures::request_metadata(None).requested_at,
            payload: fixtures::project_changed_event(),
        });
        roundtrip(&fixtures::outbound_event_envelope(
            fixtures::backlog_changed_event(),
        ));
        roundtrip(&fixtures::outbound_event_envelope(
            fixtures::project_member_changed_event(),
        ));
        roundtrip(&fixtures::outbound_event_envelope(
            fixtures::work_item_changed_event(),
        ));
        roundtrip(&fixtures::outbound_event_envelope(
            fixtures::promote_result_recorded_event(),
        ));
        roundtrip(&fixtures::outbound_event_envelope(
            fixtures::work_dependency_changed_event(),
        ));
        roundtrip(&fixtures::outbound_event_envelope(
            fixtures::work_blocker_changed_event(),
        ));
        roundtrip(&fixtures::outbound_event_envelope(
            fixtures::iteration_changed_event(),
        ));
        roundtrip(&fixtures::outbound_event_envelope(
            fixtures::trace_available_event(),
        ));
        roundtrip(&fixtures::outbound_event_envelope(
            fixtures::derived_work_view_changed_event(),
        ));

        roundtrip(&JobRunId("job-run-1".to_owned()));
        roundtrip(&WorkProjectionSet::All);
        roundtrip(&ExternalReferenceScope {
            scope_kind: ExternalReferenceScopeKind::ExplicitRefs,
            project_ref: Some(fixtures::project_ref()),
            reference_refs: vec![fixtures::member_external_reference_ref()],
        });
        roundtrip(&WorkReconciliationScopeRef {
            scope_kind: WorkReconciliationScopeKind::Project,
            project_ref: Some(fixtures::project_ref()),
            view_ref: None,
            reference_ref: None,
        });
        roundtrip(&ArchiveHandoffScope {
            scope_kind: ArchiveHandoffScopeKind::Subjects,
            project_ref: None,
            subject_refs: vec![fixtures::project_trace_subject()],
            source_cursor: Some(fixtures::truth_cursor()),
        });
        roundtrip(&ArchiveHandoffTargetRef {
            target_kind: ArchiveHandoffTargetKind::ArchiveStore,
            external_ref: fixtures::external_source_ref(),
        });
        roundtrip(&WorkJobFailureRef::ExternalReference(
            fixtures::member_external_reference_ref(),
        ));
        roundtrip(&WorkJobFailureRef::TraceHandoff {
            trace_id: fixtures::trace_id(),
            subject_ref: fixtures::project_trace_subject(),
            target_ref: fixtures::trace_handoff_target_ref(),
        });
        roundtrip(&WorkJobFailureRef::ArchiveHandoff {
            archive_scope: fixtures::archive_handoff_scope(),
            target_ref: fixtures::archive_handoff_target_ref(),
        });
        roundtrip(&WorkJobFailureRef::WorkOutbox(fixtures::outbox_id()));
        roundtrip(&WorkJobFailureRef::DerivedWorkView(
            DerivedWorkViewRef::project_board(fixtures::project_ref()),
        ));
        roundtrip(&WorkJobMetadata {
            job_run_id: fixtures::job_run_id(),
            actor: fixtures::actor_context(),
            command_metadata: fixtures::command_metadata("job-idem"),
        });
        roundtrip(&fixtures::job_report());
        roundtrip(&fixtures::publish_outbox_job_input());
        roundtrip(&fixtures::rebuild_projections_job_input());
        roundtrip(&fixtures::refresh_references_job_input());
        roundtrip(&fixtures::reconciliation_job_input());
        roundtrip(&fixtures::trace_handoff_job_input());
        roundtrip(&fixtures::archive_handoff_job_input());
        roundtrip(&fixtures::reconciliation_report());
    }

    #[test]
    fn query_envelope_and_view_roundtrip() {
        let metadata = QueryMetadata {
            request: fixtures::request_metadata(None),
            page: Some(PageRequest {
                limit: 25,
                page_token: Some(fixtures::page_token("page-2")),
            }),
            consistency: QueryConsistency::Eventual,
        };
        roundtrip(&WorkQueryEnvelope {
            actor: fixtures::query_actor_context(),
            metadata: metadata.clone(),
            query: SearchWorkRequest {
                project_ref: fixtures::project_ref(),
                criteria: fixtures::work_search_criteria(),
            },
        });

        let marker = ProjectionViewMarker {
            view_ref: DerivedWorkViewRef::search(
                fixtures::project_ref(),
                fixtures::work_search_criteria_digest(),
            ),
            source_cursor: fixtures::truth_cursor(),
            freshness_state: DerivedFreshnessState::Fresh,
        };
        let page = PublicPageInfo {
            next_page_token: Some(fixtures::page_token("page-3")),
            has_more: true,
        };
        let formal_work = FormalWorkSummaryView {
            work_ref: fixtures::formal_work_ref(),
            work_state: WorkItemState::InProgress,
            assignee_ref: Some(fixtures::project_member_ref()),
            completion_ref: None,
        };
        let relation = WorkRelationSummaryView {
            relation_ref: DependencyOrBlockerRef::Dependency(fixtures::work_dependency_ref()),
            affected_work_refs: vec![
                fixtures::formal_work_ref(),
                fixtures::child_formal_work_ref(),
            ],
            relation_state: WorkRelationStateView::Dependency(DependencyState::Active),
        };

        roundtrip(&GetProjectWorkFactsRequest {
            project_ref: fixtures::project_ref(),
        });
        roundtrip(&GetBacklogRequest {
            project_ref: fixtures::project_ref(),
            filter: Some(BacklogQueryFilter {
                work_state: Some(WorkItemState::Formalized),
                assignee_ref: Some(fixtures::project_member_ref()),
            }),
        });
        roundtrip(&GetWorkItemRequest {
            work_ref: fixtures::formal_work_ref(),
        });
        roundtrip(&ListMemberWorkRequest {
            project_member_ref: fixtures::project_member_ref(),
            work_state: Some(WorkItemState::Committed),
        });
        roundtrip(&GetIterationSummaryRequest {
            iteration_ref: fixtures::iteration_ref(),
        });
        roundtrip(&SearchWorkRequest {
            project_ref: fixtures::project_ref(),
            criteria: fixtures::work_search_criteria(),
        });
        roundtrip(&GetWorkTraceRequest {
            subject_ref: fixtures::project_trace_subject(),
        });
        roundtrip(&GetProjectBoardViewRequest {
            project_ref: fixtures::project_ref(),
        });

        roundtrip(&ProjectWorkFactsView {
            project_ref: fixtures::project_ref(),
            lifecycle_state: ProjectLifecycleState::Active,
            backlog_ref: Some(fixtures::backlog_ref()),
            members: vec![ProjectMemberSummaryView {
                project_member_ref: fixtures::project_member_ref(),
                member_ref: fixtures::global_member_ref(),
                responsibility_state: ProjectMemberResponsibilityState::Active,
            }],
            formal_work: vec![formal_work.clone()],
            relations: vec![relation.clone()],
        });
        roundtrip(&BacklogView {
            backlog_ref: fixtures::backlog_ref(),
            project_ref: fixtures::project_ref(),
            backlog_state: BacklogState::Open,
            items: vec![formal_work.clone()],
            page: page.clone(),
        });
        roundtrip(&WorkItemView {
            work_ref: fixtures::child_formal_work_ref(),
            parent_ref: Some(fixtures::formal_work_ref()),
            work_state: WorkItemState::Completed,
            assignee_ref: Some(fixtures::project_member_ref()),
            source_ref: Some(fixtures::source_work_ref()),
            completion_ref: Some(fixtures::completion_evidence_ref()),
            relations: vec![relation.clone()],
        });
        roundtrip(&MemberWorkView {
            member_ref: fixtures::project_member_ref(),
            assigned_work: vec![formal_work.clone()],
            marker: ProjectionViewMarker {
                view_ref: DerivedWorkViewRef::member_work(fixtures::project_member_ref()),
                source_cursor: fixtures::truth_cursor(),
                freshness_state: DerivedFreshnessState::Stale,
            },
            page: page.clone(),
        });
        roundtrip(&IterationSummaryView {
            iteration_ref: fixtures::iteration_ref(),
            iteration_state: IterationState::Committed,
            commitment_state: Some(CommitmentState::Committed),
            committed_work: vec![formal_work.clone()],
            marker: ProjectionViewMarker {
                view_ref: DerivedWorkViewRef::iteration_summary(fixtures::iteration_ref()),
                source_cursor: fixtures::truth_cursor(),
                freshness_state: DerivedFreshnessState::Fresh,
            },
        });
        roundtrip(&WorkSearchResult {
            project_ref: fixtures::project_ref(),
            criteria: fixtures::work_search_criteria(),
            items: vec![formal_work.clone()],
            marker: marker.clone(),
            page: page.clone(),
        });
        roundtrip(&WorkTraceView {
            subject_ref: fixtures::project_trace_subject(),
            records: vec![WorkTraceRecordView {
                trace_id: fixtures::trace_id(),
                subject_ref: fixtures::project_trace_subject(),
                trace_context_ref: fixtures::trace_context_ref(),
            }],
            page: page.clone(),
        });
        roundtrip(&ProjectBoardView {
            project_ref: fixtures::project_ref(),
            work_cards: vec![formal_work],
            marker,
        });
        roundtrip(&WorkQueryResponse {
            surface: QuerySurface::Visible,
            data: Some(ProjectBoardView {
                project_ref: fixtures::project_ref(),
                work_cards: vec![],
                marker: ProjectionViewMarker {
                    view_ref: DerivedWorkViewRef::project_board(fixtures::project_ref()),
                    source_cursor: fixtures::truth_cursor(),
                    freshness_state: DerivedFreshnessState::Rebuilding,
                },
            }),
        });
    }
}
