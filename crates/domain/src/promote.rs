//! Promote result and runtime intake domain objects.

use core_contracts::actor::ActorRef;

use crate::DomainError;
use work_contracts::{
    FormalWorkRef, PromoteDecisionId, PromoteReason, PromoteRejectReason, PromoteResultId,
    PromoteResultRef, PromoteResultState, SourceEventId, SourceWorkRef,
};

/// Records whether an external source was accepted into formal Work truth.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromoteResult {
    /// Stable promote result id.
    pub promote_result_id: PromoteResultId,
    /// Evaluated source reference.
    pub source_ref: SourceWorkRef,
    /// Current promote decision state.
    pub result_state: PromoteResultState,
    /// Formal work created after acceptance.
    pub created_work_ref: Option<FormalWorkRef>,
}

impl PromoteResult {
    /// Creates a pending-review promote result from a source evaluation request.
    pub fn evaluate(
        promote_result_id: PromoteResultId,
        source_ref: SourceWorkRef,
        _reason: PromoteReason,
        _actor: ActorRef,
    ) -> Result<Self, DomainError> {
        if promote_result_id.0.trim().is_empty()
            || source_ref.external_ref.external_id.trim().is_empty()
        {
            return Err(DomainError::MissingRequiredValue);
        }

        Ok(Self {
            promote_result_id,
            source_ref,
            result_state: PromoteResultState::PendingReview,
            created_work_ref: None,
        })
    }

    /// Accepts the promote result into a formal Work reference.
    pub fn accept(&mut self, work_ref: FormalWorkRef, _actor: ActorRef) -> Result<(), DomainError> {
        if self.result_state != PromoteResultState::PendingReview {
            return Err(DomainError::InvalidStateTransition);
        }

        self.result_state = PromoteResultState::Accepted;
        self.created_work_ref = Some(work_ref);
        Ok(())
    }

    /// Rejects the promote result with an auditable reason.
    pub fn reject(
        &mut self,
        reason: PromoteRejectReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.result_state != PromoteResultState::PendingReview {
            return Err(DomainError::InvalidStateTransition);
        }
        if matches!(
            reason.reason_kind,
            work_contracts::PromoteRejectReasonKind::Duplicate
        ) && self.created_work_ref.is_some()
        {
            return Err(DomainError::InvariantViolation);
        }

        self.result_state = PromoteResultState::Rejected;
        self.created_work_ref = None;
        Ok(())
    }

    /// Returns the stable promote result ref.
    pub fn promote_result_ref(&self) -> PromoteResultRef {
        PromoteResultRef {
            promote_result_id: self.promote_result_id.clone(),
        }
    }
}

/// Records an inbound runtime promote request without creating Work truth.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingPromoteIntake {
    /// Runtime source that may later be promoted through an explicit command.
    pub source_ref: SourceWorkRef,
    /// Reason supplied by runtime.
    pub promote_reason: PromoteReason,
    /// Source event that produced the intake marker.
    pub source_event_id: SourceEventId,
}

impl PendingPromoteIntake {
    /// Creates a pending runtime intake marker from a runtime event.
    pub fn from_runtime_event(
        source_ref: SourceWorkRef,
        promote_reason: PromoteReason,
        source_event_id: SourceEventId,
    ) -> Result<Self, DomainError> {
        if source_ref.external_ref.external_id.trim().is_empty()
            || source_event_id.0.trim().is_empty()
        {
            return Err(DomainError::MissingRequiredValue);
        }

        Ok(Self {
            source_ref,
            promote_reason,
            source_event_id,
        })
    }
}

/// Records one promote review decision without mutating the promote result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromoteDecisionRecord {
    /// Stable decision history id.
    pub decision_id: PromoteDecisionId,
    /// Reviewed source reference.
    pub source_ref: SourceWorkRef,
    /// Promote result that produced this decision.
    pub result_ref: PromoteResultRef,
}

impl PromoteDecisionRecord {
    /// Builds one decision record from a promote result.
    pub fn from_result(
        decision_id: PromoteDecisionId,
        result: PromoteResult,
        _actor: ActorRef,
    ) -> Result<Self, DomainError> {
        if decision_id.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        let result_ref = result.promote_result_ref();

        Ok(Self {
            decision_id,
            source_ref: result.source_ref,
            result_ref,
        })
    }
}
