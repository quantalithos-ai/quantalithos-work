//! Shared fixtures and metadata helpers for contract and domain tests.

use core_contracts::{
    actor::{ActorContext, ActorKind, ActorRef, RequestOrigin},
    metadata::{
        CommandMetadata, IdempotencyKey, OperationName, RequestId, RequestMetadata, Timestamp,
        TraceId,
    },
};

use crate::{
    commands::ProjectSpec,
    handoff::{ApplicationResultRef, WorkTraceContextRef},
    refs::{
        BacklogId, BacklogRef, ExternalSourceRef, ExternalSourceSystem, ProjectId,
        ProjectOwnerKind, ProjectOwnerRef, ProjectRef, ResultId, SafeSummaryText, TraceHandoffRef,
        WorkAuditSubjectRef, WorkAuditTrailId, WorkOutboxId, WorkTraceId, WorkTraceRecordRefSet,
        WorkTraceSubjectRef, WorkTruthChange,
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

    /// Returns a deterministic trace context ref.
    pub fn trace_context_ref() -> WorkTraceContextRef {
        WorkTraceContextRef::from_metadata(&request_metadata(None))
    }
}
