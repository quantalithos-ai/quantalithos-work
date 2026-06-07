//! Shared fixtures and metadata helpers for contract and domain tests.

use core_contracts::{
    actor::{ActorContext, ActorKind, ActorRef, RequestOrigin},
    metadata::{
        CommandMetadata, IdempotencyKey, OperationName, PageRequest, PageToken, QueryConsistency,
        QueryMetadata, RequestId, RequestMetadata, Timestamp, TraceId,
    },
};

use crate::{
    commands::{ProjectResponsibilitySpec, ProjectSpec},
    events::EventSchemaVersion,
    handoff::{ApplicationResultRef, WorkCommandReceipt, WorkTraceContextRef},
    jobs::{
        PrepareArchiveHandoffJobInput, PrepareWorkTraceHandoffJobInput, PublishWorkOutboxJobInput,
        RebuildWorkProjectionsJobInput, RefreshExternalReferenceSnapshotsJobInput,
        RunWorkReconciliationJobInput, WorkJobMetadata, WorkJobReport, WorkProjectionSet,
    },
    queries::ReconciliationReport,
    refs::{
        ArchiveHandoffScope, ArchiveHandoffScopeKind, ArchiveHandoffTargetKind,
        ArchiveHandoffTargetRef, BacklogId, BacklogRef, BlockerCauseRef, CapabilityRef,
        CapabilityRefSet, ChildWorkItemId, CommitmentChangeReason, CommitmentChangeReasonKind,
        DependencyChangeReason, DependencyChangeReasonKind, DependencyOrBlockerRef,
        DependencyReason, DependencyReasonKind, DerivedWorkViewRef, EvidenceKind,
        EvidenceVerifiedState, ExternalEvidenceRef, ExternalReferenceRef, ExternalReferenceScope,
        ExternalReferenceScopeKind, ExternalSourceRef, ExternalSourceSystem, ExternalVersionRef,
        FormalWorkIntent, FormalWorkRef, FormalWorkRefSet, GlobalMemberRef, IterationChangeReason,
        IterationChangeReasonKind, IterationCloseReason, IterationCloseReasonKind,
        IterationCommitmentChangeSet, IterationCommitmentId, IterationId, IterationRef, JobRunId,
        MethodDefinitionKind, MethodDefinitionRef, ProcessTimeboxRef, ProjectId, ProjectMemberId,
        ProjectMemberRef, ProjectOwnerKind, ProjectOwnerRef, ProjectRef, ProjectResponsibilityKind,
        PromoteReason, PromoteReasonKind, PromoteRejectReason, PromoteRejectReasonKind,
        PromoteResultId, PromoteResultRef, ResultId, SafeSummaryText, SourceDigest, SourceEventId,
        SourceWorkKind, SourceWorkRef, TraceHandoffRef, TraceHandoffTargetKind,
        TraceHandoffTargetRef, WorkAuditSubjectRef, WorkAuditTrailId, WorkBlockerId,
        WorkBlockerRef, WorkDependencyId, WorkDependencyRef, WorkItemId, WorkLifecycleReason,
        WorkLifecycleReasonKind, WorkOutboxId, WorkReconciliationScopeKind,
        WorkReconciliationScopeRef, WorkSearchCriteriaDigest, WorkSearchText, WorkTitle,
        WorkTraceId, WorkTraceRecordRefSet, WorkTraceSubjectRef, WorkTruthChange, WorkTruthCursor,
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

    /// Returns a deterministic query actor context for tests.
    pub fn query_actor_context() -> ActorContext {
        ActorContext::new(
            ActorRef::new("work-query-actor-1", ActorKind::Human),
            RequestOrigin::Query,
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

    /// Returns deterministic query metadata.
    pub fn query_metadata() -> QueryMetadata {
        QueryMetadata {
            request: request_metadata(None),
            page: None,
            consistency: QueryConsistency::Eventual,
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

    /// Returns a deterministic iteration commitment id.
    pub fn iteration_commitment_id() -> IterationCommitmentId {
        IterationCommitmentId("iteration-commitment-1".to_owned())
    }

    /// Returns a deterministic process timebox ref.
    pub fn process_timebox_ref() -> ProcessTimeboxRef {
        ProcessTimeboxRef("process/timeboxes/timebox-1".to_owned())
    }

    /// Returns a deterministic formal work ref set.
    pub fn formal_work_ref_set() -> FormalWorkRefSet {
        FormalWorkRefSet {
            refs: vec![formal_work_ref(), child_formal_work_ref()],
        }
    }

    /// Returns a deterministic work title.
    pub fn work_title(value: &str) -> WorkTitle {
        WorkTitle(value.to_owned())
    }

    /// Returns a deterministic work search text.
    pub fn work_search_text(value: &str) -> WorkSearchText {
        WorkSearchText(value.to_owned())
    }

    /// Returns a deterministic work search criteria digest.
    pub fn work_search_criteria_digest() -> WorkSearchCriteriaDigest {
        WorkSearchCriteriaDigest(
            "work_state=in_progress|assignee_ref=project-member-1|source_kind=conversation|text_query=formal work".to_owned(),
        )
    }

    /// Returns the only supported P0 event schema version.
    pub fn event_schema_version() -> EventSchemaVersion {
        EventSchemaVersion::v1()
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

    /// Returns a deterministic external reference ref for an identity member.
    pub fn member_external_reference_ref() -> ExternalReferenceRef {
        ExternalReferenceRef::from_member(global_member_ref())
    }

    /// Returns a deterministic external reference ref for a method definition.
    pub fn method_external_reference_ref() -> ExternalReferenceRef {
        ExternalReferenceRef::from_method_definition(method_definition_ref())
    }

    /// Returns a deterministic external reference ref for a source work pointer.
    pub fn source_external_reference_ref() -> ExternalReferenceRef {
        ExternalReferenceRef::from_source_work(source_work_ref())
    }

    /// Returns a deterministic external reference ref for evidence.
    pub fn evidence_external_reference_ref() -> ExternalReferenceRef {
        ExternalReferenceRef::from_evidence(completion_evidence_ref())
    }

    /// Returns a deterministic external reference ref for a process timebox.
    pub fn process_timebox_external_reference_ref() -> ExternalReferenceRef {
        ExternalReferenceRef::from_process_timebox(process_timebox_ref())
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

    /// Returns a deterministic truth cursor.
    pub fn truth_cursor() -> WorkTruthCursor {
        WorkTruthCursor("cursor-1".to_owned())
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

    /// Returns a deterministic page token.
    pub fn page_token(value: &str) -> PageToken {
        PageToken::new(value)
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

    /// Returns a deterministic iteration change reason for commitment creation.
    pub fn iteration_commitment_created_reason() -> IterationChangeReason {
        IterationChangeReason {
            reason_kind: IterationChangeReasonKind::CommitmentCreated,
            reason_ref: None,
        }
    }

    /// Returns a deterministic iteration change reason for commitment updates.
    pub fn iteration_commitment_changed_reason() -> IterationChangeReason {
        IterationChangeReason {
            reason_kind: IterationChangeReasonKind::CommitmentChanged,
            reason_ref: None,
        }
    }

    /// Returns a deterministic iteration start reason.
    pub fn iteration_started_reason() -> IterationChangeReason {
        IterationChangeReason {
            reason_kind: IterationChangeReasonKind::Started,
            reason_ref: None,
        }
    }

    /// Returns a deterministic iteration cancellation reason.
    pub fn iteration_cancelled_reason() -> IterationChangeReason {
        IterationChangeReason {
            reason_kind: IterationChangeReasonKind::Cancelled,
            reason_ref: None,
        }
    }

    /// Returns a deterministic iteration close reason.
    pub fn iteration_closed_reason() -> IterationCloseReason {
        IterationCloseReason {
            reason_kind: IterationCloseReasonKind::ManualClose,
            reason_ref: None,
        }
    }

    /// Returns a deterministic commitment change reason.
    pub fn commitment_change_reason() -> CommitmentChangeReason {
        CommitmentChangeReason {
            reason_kind: CommitmentChangeReasonKind::ManualAdjustment,
            reason_ref: None,
        }
    }

    /// Returns a deterministic iteration commitment change set.
    pub fn iteration_commitment_change_set() -> IterationCommitmentChangeSet {
        IterationCommitmentChangeSet {
            add_work_refs: vec![downstream_formal_work_ref()],
            remove_work_refs: vec![formal_work_ref()],
        }
    }

    /// Returns deterministic search criteria.
    pub fn work_search_criteria() -> crate::queries::WorkSearchCriteria {
        crate::queries::WorkSearchCriteria {
            work_state: Some(crate::states::WorkItemState::InProgress),
            assignee_ref: Some(project_member_ref()),
            source_kind: Some(SourceWorkKind::Conversation),
            text_query: Some(work_search_text("formal work")),
        }
    }

    /// Returns a deterministic job run id.
    pub fn job_run_id() -> JobRunId {
        JobRunId("job-run-1".to_owned())
    }

    /// Returns deterministic job metadata.
    pub fn job_metadata(idempotency_key: &str) -> WorkJobMetadata {
        WorkJobMetadata {
            job_run_id: job_run_id(),
            actor: actor_context(),
            command_metadata: command_metadata(idempotency_key),
        }
    }

    /// Returns deterministic external reference scope.
    pub fn external_reference_scope() -> ExternalReferenceScope {
        ExternalReferenceScope {
            scope_kind: ExternalReferenceScopeKind::ExplicitRefs,
            project_ref: Some(project_ref()),
            reference_refs: vec![
                member_external_reference_ref(),
                method_external_reference_ref(),
            ],
        }
    }

    /// Returns deterministic reconciliation scope.
    pub fn reconciliation_scope_ref() -> WorkReconciliationScopeRef {
        WorkReconciliationScopeRef {
            scope_kind: WorkReconciliationScopeKind::Project,
            project_ref: Some(project_ref()),
            view_ref: None,
            reference_ref: None,
        }
    }

    /// Returns deterministic archive handoff scope.
    pub fn archive_handoff_scope() -> ArchiveHandoffScope {
        ArchiveHandoffScope {
            scope_kind: ArchiveHandoffScopeKind::Subjects,
            subject_refs: vec![project_trace_subject()],
            source_cursor: Some(truth_cursor()),
        }
    }

    /// Returns deterministic archive handoff target.
    pub fn archive_handoff_target_ref() -> ArchiveHandoffTargetRef {
        ArchiveHandoffTargetRef {
            target_kind: ArchiveHandoffTargetKind::ArchiveStore,
            external_ref: external_source_ref(),
        }
    }

    /// Returns deterministic trace handoff target.
    pub fn trace_handoff_target_ref() -> TraceHandoffTargetRef {
        TraceHandoffTargetRef {
            target_kind: TraceHandoffTargetKind::Observability,
            external_ref: external_source_ref(),
        }
    }

    /// Returns a deterministic source event id.
    pub fn source_event_id() -> SourceEventId {
        SourceEventId("source-event-1".to_owned())
    }

    /// Returns a deterministic external version ref.
    pub fn external_version_ref() -> ExternalVersionRef {
        ExternalVersionRef("external-version-1".to_owned())
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

    /// Returns deterministic publish-outbox job input.
    pub fn publish_outbox_job_input() -> PublishWorkOutboxJobInput {
        PublishWorkOutboxJobInput {
            metadata: job_metadata("job-publish-outbox"),
            page: PageRequest {
                limit: 25,
                page_token: Some(page_token("job-page-1")),
            },
        }
    }

    /// Returns deterministic rebuild-projections job input.
    pub fn rebuild_projections_job_input() -> RebuildWorkProjectionsJobInput {
        RebuildWorkProjectionsJobInput {
            metadata: job_metadata("job-rebuild-projections"),
            project_ref: project_ref(),
            projection_set: WorkProjectionSet::All,
        }
    }

    /// Returns deterministic refresh-references job input.
    pub fn refresh_references_job_input() -> RefreshExternalReferenceSnapshotsJobInput {
        RefreshExternalReferenceSnapshotsJobInput {
            metadata: job_metadata("job-refresh-references"),
            reference_scope: Some(external_reference_scope()),
            page: PageRequest {
                limit: 50,
                page_token: None,
            },
        }
    }

    /// Returns deterministic reconciliation job input.
    pub fn reconciliation_job_input() -> RunWorkReconciliationJobInput {
        RunWorkReconciliationJobInput {
            metadata: job_metadata("job-reconcile"),
            scope_ref: reconciliation_scope_ref(),
        }
    }

    /// Returns deterministic trace handoff job input.
    pub fn trace_handoff_job_input() -> PrepareWorkTraceHandoffJobInput {
        PrepareWorkTraceHandoffJobInput {
            metadata: job_metadata("job-trace-handoff"),
            subject_ref: project_trace_subject(),
            target_ref: trace_handoff_target_ref(),
        }
    }

    /// Returns deterministic archive handoff job input.
    pub fn archive_handoff_job_input() -> PrepareArchiveHandoffJobInput {
        PrepareArchiveHandoffJobInput {
            metadata: job_metadata("job-archive-handoff"),
            archive_scope: archive_handoff_scope(),
            archive_target_ref: archive_handoff_target_ref(),
        }
    }

    /// Returns deterministic work job report.
    pub fn job_report() -> WorkJobReport {
        WorkJobReport {
            job_run_id: job_run_id(),
            receipt: Some(WorkCommandReceipt::applied(
                application_result_ref("job_run", "job-result-1"),
                Some(trace_id()),
                vec![outbox_id()],
                3,
            )),
            scanned_count: 9,
            changed_count: 4,
            failed_refs: vec![source_external_reference_ref()],
        }
    }

    /// Returns deterministic reconciliation report.
    pub fn reconciliation_report() -> ReconciliationReport {
        ReconciliationReport {
            scope_ref: reconciliation_scope_ref(),
            truth_cursor: truth_cursor(),
            projection_gaps: vec![DerivedWorkViewRef::project_board(project_ref())],
            outbox_gaps: vec![outbox_id()],
            reference_gaps: vec![process_timebox_external_reference_ref()],
        }
    }

    /// Returns deterministic inbound identity-member payload.
    pub fn identity_member_changed_payload() -> crate::events::IdentityMemberChangedPayload {
        crate::events::IdentityMemberChangedPayload {
            member_ref: global_member_ref(),
            capability_refs: capability_ref_set(),
            source_version_ref: external_version_ref(),
        }
    }

    /// Returns deterministic inbound method-definition payload.
    pub fn method_definition_changed_payload() -> crate::events::MethodDefinitionChangedPayload {
        crate::events::MethodDefinitionChangedPayload {
            definition_ref: method_definition_ref(),
            definition_kind: MethodDefinitionKind::Task,
            source_version_ref: external_version_ref(),
        }
    }

    /// Returns deterministic inbound conversation payload.
    pub fn conversation_work_context_changed_payload()
    -> crate::events::ConversationWorkContextChangedPayload {
        crate::events::ConversationWorkContextChangedPayload {
            source_ref: source_work_ref(),
            source_digest: Some(SourceDigest("digest-1".to_owned())),
        }
    }

    /// Returns deterministic inbound process timing payload.
    pub fn process_timing_changed_payload() -> crate::events::ProcessTimingChangedPayload {
        crate::events::ProcessTimingChangedPayload {
            timebox_ref: process_timebox_ref(),
            project_ref: Some(project_ref()),
            source_version_ref: external_version_ref(),
        }
    }

    /// Returns deterministic inbound governance decision payload.
    pub fn governance_decision_changed_payload() -> crate::events::GovernanceDecisionChangedPayload
    {
        crate::events::GovernanceDecisionChangedPayload {
            source_ref: source_work_ref(),
            evidence_ref: Some(completion_evidence_ref()),
            source_version_ref: external_version_ref(),
        }
    }

    /// Returns deterministic inbound artifact evidence payload.
    pub fn artifact_evidence_changed_payload() -> crate::events::ArtifactEvidenceChangedPayload {
        crate::events::ArtifactEvidenceChangedPayload {
            evidence_ref: completion_evidence_ref(),
            source_version_ref: external_version_ref(),
        }
    }

    /// Returns deterministic inbound runtime promote payload.
    pub fn runtime_promote_requested_payload() -> crate::events::RuntimePromoteRequestedPayload {
        crate::events::RuntimePromoteRequestedPayload {
            source_ref: runtime_source_work_ref(),
            promote_reason: promote_reason(),
        }
    }

    /// Returns deterministic inbound event envelope.
    pub fn inbound_event_envelope<T>(payload: T) -> crate::events::WorkInboundEventEnvelope<T> {
        crate::events::WorkInboundEventEnvelope {
            source_event_id: source_event_id(),
            source_ref: external_source_ref(),
            event_version: event_schema_version(),
            trace_context_ref: trace_context_ref(),
            occurred_at: request_metadata(None).requested_at,
            payload,
        }
    }

    /// Returns deterministic outbound event envelope.
    pub fn outbound_event_envelope<T>(payload: T) -> crate::events::WorkOutboundEventEnvelope<T> {
        crate::events::WorkOutboundEventEnvelope {
            outbox_id: outbox_id(),
            event_version: event_schema_version(),
            trace_context_ref: trace_context_ref(),
            occurred_at: request_metadata(None).requested_at,
            payload,
        }
    }

    /// Returns deterministic project changed payload.
    pub fn project_changed_event() -> crate::events::ProjectChangedEvent {
        crate::events::ProjectChangedEvent {
            project_ref: project_ref(),
            lifecycle_state: crate::states::ProjectLifecycleState::Active,
            reason: crate::refs::ProjectLifecycleReason {
                reason_kind: crate::refs::ProjectLifecycleReasonKind::OwnerRequest,
                reason_ref: None,
                note: Some(safe_summary("owner request")),
            },
        }
    }

    /// Returns deterministic backlog changed payload.
    pub fn backlog_changed_event() -> crate::events::BacklogChangedEvent {
        crate::events::BacklogChangedEvent {
            backlog_ref: backlog_ref(),
            project_ref: project_ref(),
            backlog_state: crate::states::BacklogState::Open,
            reason: crate::refs::BacklogMaintenanceReason {
                reason_kind: crate::refs::BacklogMaintenanceReasonKind::ManualUnlock,
                reason_ref: None,
            },
        }
    }

    /// Returns deterministic project-member changed payload.
    pub fn project_member_changed_event() -> crate::events::ProjectMemberChangedEvent {
        crate::events::ProjectMemberChangedEvent {
            project_member_ref: project_member_ref(),
            project_ref: project_ref(),
            member_ref: global_member_ref(),
            responsibility_state: crate::states::ProjectMemberResponsibilityState::Active,
        }
    }

    /// Returns deterministic work-item changed payload.
    pub fn work_item_changed_event() -> crate::events::WorkItemChangedEvent {
        crate::events::WorkItemChangedEvent {
            work_ref: formal_work_ref(),
            project_ref: project_ref(),
            work_state: crate::states::WorkItemState::InProgress,
            source_ref: Some(source_work_ref()),
            evidence_ref: Some(completion_evidence_ref()),
        }
    }

    /// Returns deterministic promote-result recorded payload.
    pub fn promote_result_recorded_event() -> crate::events::PromoteResultRecordedEvent {
        crate::events::PromoteResultRecordedEvent {
            promote_result_ref: promote_result_ref(),
            source_ref: source_work_ref(),
            result_state: crate::states::PromoteResultState::Accepted,
            created_work_ref: Some(formal_work_ref()),
        }
    }

    /// Returns deterministic dependency-changed payload.
    pub fn work_dependency_changed_event() -> crate::events::WorkDependencyChangedEvent {
        crate::events::WorkDependencyChangedEvent {
            dependency_ref: work_dependency_ref(),
            upstream_work_ref: formal_work_ref(),
            downstream_work_ref: downstream_formal_work_ref(),
            dependency_state: crate::states::DependencyState::Active,
        }
    }

    /// Returns deterministic blocker-changed payload.
    pub fn work_blocker_changed_event() -> crate::events::WorkBlockerChangedEvent {
        crate::events::WorkBlockerChangedEvent {
            blocker_ref: work_blocker_ref(),
            blocked_work_ref: formal_work_ref(),
            blocker_state: crate::states::BlockerState::Resolved,
            evidence_ref: Some(blocker_resolution_evidence_ref()),
        }
    }

    /// Returns deterministic iteration-changed payload.
    pub fn iteration_changed_event() -> crate::events::IterationChangedEvent {
        crate::events::IterationChangedEvent {
            iteration_ref: iteration_ref(),
            project_ref: project_ref(),
            iteration_state: crate::states::IterationState::Committed,
            commitment_state: Some(crate::states::CommitmentState::Committed),
            affected_work_refs: vec![formal_work_ref(), child_formal_work_ref()],
        }
    }

    /// Returns deterministic trace-available payload.
    pub fn trace_available_event() -> crate::events::WorkTraceAvailableEvent {
        crate::events::WorkTraceAvailableEvent {
            subject_ref: project_trace_subject(),
            trace_id: trace_id(),
            handoff_ref: Some(trace_handoff_ref()),
        }
    }

    /// Returns deterministic derived-view-changed payload.
    pub fn derived_work_view_changed_event() -> crate::events::DerivedWorkViewChangedEvent {
        crate::events::DerivedWorkViewChangedEvent {
            view_ref: DerivedWorkViewRef::search(project_ref(), work_search_criteria_digest()),
            freshness_state: crate::states::DerivedFreshnessState::Stale,
            source_cursor: truth_cursor(),
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

    /// Returns a deterministic iteration change.
    pub fn iteration_changed_change() -> WorkTruthChange {
        WorkTruthChange::IterationChanged(iteration_ref())
    }

    /// Returns a deterministic trace context ref.
    pub fn trace_context_ref() -> WorkTraceContextRef {
        WorkTraceContextRef::from_metadata(&request_metadata(None))
    }
}
