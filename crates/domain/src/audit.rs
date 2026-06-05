//! Trace, audit, and outbox helper domain objects.

use work_contracts::{
    OutboxFailureReason, OutboxPublicationRef, OutboxRetryReason, TraceHandoffIntent,
    TraceHandoffRef, TraceHandoffTargetRef, WorkAuditSubjectRef, WorkAuditTrailId,
    WorkOutboxEventKind, WorkOutboxId, WorkTraceContextRef, WorkTraceId, WorkTraceRecordRefSet,
    WorkTraceSubjectRef, WorkTruthChange,
};

use crate::DomainError;

/// Records traceable context for an accepted Work truth change.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkTraceRecord {
    /// Stable trace record id.
    pub trace_id: WorkTraceId,
    /// Subject affected by the trace.
    pub subject_ref: WorkTraceSubjectRef,
    /// Core trace and request context pointer.
    pub trace_context_ref: WorkTraceContextRef,
}

impl WorkTraceRecord {
    /// Builds a trace record from an accepted truth change.
    pub fn from_truth_change(
        trace_id: WorkTraceId,
        change: WorkTruthChange,
        trace_context_ref: WorkTraceContextRef,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            trace_id,
            subject_ref: change.subject_ref(),
            trace_context_ref,
        })
    }

    /// Returns whether this trace record is associated with the given subject.
    pub fn relates_to(&self, subject_ref: WorkTraceSubjectRef) -> bool {
        self.subject_ref == subject_ref
    }

    /// Forms a trace handoff intent from this accepted trace record.
    pub fn prepare_handoff(
        &self,
        target_ref: TraceHandoffTargetRef,
    ) -> Result<TraceHandoffIntent, DomainError> {
        Ok(TraceHandoffIntent {
            trace_id: self.trace_id.clone(),
            target_ref,
            subject_ref: self.subject_ref.clone(),
        })
    }
}

/// Records that a Work trace was handed off to an external boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceHandoffMarker {
    /// Trace record prepared for handoff.
    pub trace_id: WorkTraceId,
    /// External handoff reference returned by the port.
    pub handoff_ref: TraceHandoffRef,
}

impl TraceHandoffMarker {
    /// Records a successful trace handoff result.
    pub fn from_trace(
        trace_id: WorkTraceId,
        handoff_ref: TraceHandoffRef,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            trace_id,
            handoff_ref,
        })
    }
}

/// Maintains the audit record chain for a Work subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkAuditTrail {
    /// Stable audit trail id.
    pub audit_trail_id: WorkAuditTrailId,
    /// Subject being audited.
    pub subject_ref: WorkAuditSubjectRef,
    /// Trace records associated with the subject.
    pub record_refs: WorkTraceRecordRefSet,
}

impl WorkAuditTrail {
    /// Initializes the audit chain for one subject.
    pub fn start_for_subject(subject_ref: WorkAuditSubjectRef) -> Self {
        let audit_trail_id = match &subject_ref {
            WorkAuditSubjectRef::Project(project_ref) => {
                WorkAuditTrailId(format!("audit:project:{project_ref:?}"))
            }
            WorkAuditSubjectRef::Backlog(backlog_ref) => {
                WorkAuditTrailId(format!("audit:backlog:{backlog_ref:?}"))
            }
            WorkAuditSubjectRef::ProjectMember(project_member_ref) => {
                WorkAuditTrailId(format!("audit:project_member:{project_member_ref:?}"))
            }
        };

        Self {
            audit_trail_id,
            subject_ref,
            record_refs: WorkTraceRecordRefSet {
                trace_ids: Vec::new(),
            },
        }
    }

    /// Appends a trace record reference.
    pub fn append(&mut self, record: WorkTraceRecord) -> Result<(), DomainError> {
        let subject_matches = match (&self.subject_ref, &record.subject_ref) {
            (WorkAuditSubjectRef::Project(lhs), WorkTraceSubjectRef::Project(rhs)) => lhs == rhs,
            (WorkAuditSubjectRef::Backlog(lhs), WorkTraceSubjectRef::Backlog(rhs)) => lhs == rhs,
            (WorkAuditSubjectRef::ProjectMember(lhs), WorkTraceSubjectRef::ProjectMember(rhs)) => {
                lhs == rhs
            }
            _ => false,
        };

        if !subject_matches {
            return Err(DomainError::RefMismatch);
        }

        self.record_refs.trace_ids.push(record.trace_id);
        Ok(())
    }

    /// Returns whether the audit chain has a gap.
    pub fn has_gap(&self) -> bool {
        self.record_refs.trace_ids.is_empty()
    }
}

/// Represents a committed Work truth change pending publication or handoff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkOutboxRecord {
    /// Stable outbox id.
    pub outbox_id: WorkOutboxId,
    /// Work event category.
    pub event_kind: WorkOutboxEventKind,
    /// Current publication state.
    pub publication_state: work_contracts::OutboxPublicationState,
}

impl WorkOutboxRecord {
    /// Builds an outbox record from an accepted truth change.
    pub fn from_truth_change(
        outbox_id: WorkOutboxId,
        change: WorkTruthChange,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            outbox_id,
            event_kind: change.event_kind(),
            publication_state: work_contracts::OutboxPublicationState::Pending,
        })
    }

    /// Marks the outbox record as published.
    pub fn mark_published(
        &mut self,
        _publication_ref: OutboxPublicationRef,
    ) -> Result<(), DomainError> {
        match self.publication_state {
            work_contracts::OutboxPublicationState::Pending => {
                self.publication_state = work_contracts::OutboxPublicationState::Published;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition),
        }
    }

    /// Marks the outbox record as failed.
    pub fn mark_failed(&mut self, _reason: OutboxFailureReason) -> Result<(), DomainError> {
        match self.publication_state {
            work_contracts::OutboxPublicationState::Pending => {
                self.publication_state = work_contracts::OutboxPublicationState::Failed;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition),
        }
    }

    /// Marks the outbox record as pending for retry.
    pub fn mark_pending_for_retry(
        &mut self,
        _reason: OutboxRetryReason,
    ) -> Result<(), DomainError> {
        match self.publication_state {
            work_contracts::OutboxPublicationState::Failed => {
                self.publication_state = work_contracts::OutboxPublicationState::Pending;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition),
        }
    }
}
