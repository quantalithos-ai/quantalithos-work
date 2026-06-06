//! Iteration and commitment domain objects plus history records.

use core_contracts::actor::ActorRef;

use crate::DomainError;
use work_contracts::{
    CommitmentChangeReason, CommitmentState, FormalWorkRef, FormalWorkRefSet, IterationChangeId,
    IterationChangeReason, IterationChangeReasonKind, IterationCloseReason,
    IterationCommitmentChangeSet, IterationCommitmentId, IterationId, IterationRef, IterationState,
    ProcessTimeboxRef, ProjectId,
};

/// Represents a Work-owned commitment window for one project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Iteration {
    /// Stable iteration id.
    pub iteration_id: IterationId,
    /// Project that owns the iteration.
    pub project_id: ProjectId,
    /// External process timebox pointer.
    pub timebox_ref: ProcessTimeboxRef,
    /// Current iteration lifecycle state.
    pub iteration_state: IterationState,
}

impl Iteration {
    /// Establishes a planning iteration for one project and timebox.
    pub fn open(
        iteration_id: IterationId,
        project_id: ProjectId,
        timebox_ref: ProcessTimeboxRef,
        _actor: ActorRef,
    ) -> Result<Self, DomainError> {
        if iteration_id.0.trim().is_empty()
            || project_id.0.trim().is_empty()
            || timebox_ref.0.trim().is_empty()
        {
            return Err(DomainError::MissingRequiredValue);
        }

        Ok(Self {
            iteration_id,
            project_id,
            timebox_ref,
            iteration_state: IterationState::Planning,
        })
    }

    /// Commits a candidate work scope into the iteration.
    pub fn commit(
        &mut self,
        commitment: &mut IterationCommitment,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.iteration_state != IterationState::Planning {
            return Err(DomainError::InvalidStateTransition);
        }
        if commitment.iteration_id != self.iteration_id {
            return Err(DomainError::RefMismatch);
        }
        if commitment.commitment_state != CommitmentState::Candidate {
            return Err(DomainError::InvalidStateTransition);
        }

        self.iteration_state = IterationState::Committed;
        commitment.commitment_state = CommitmentState::Committed;
        Ok(())
    }

    /// Starts a committed iteration.
    pub fn start(
        &mut self,
        reason: IterationChangeReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.iteration_state != IterationState::Committed {
            return Err(DomainError::InvalidStateTransition);
        }
        if reason.reason_kind != IterationChangeReasonKind::Started {
            return Err(DomainError::PolicyRejected);
        }

        self.iteration_state = IterationState::InProgress;
        Ok(())
    }

    /// Closes an in-progress iteration.
    pub fn close(
        &mut self,
        reason: IterationCloseReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.iteration_state != IterationState::InProgress {
            return Err(DomainError::InvalidStateTransition);
        }
        if reason.reason_ref.is_none()
            && matches!(
                reason.reason_kind,
                work_contracts::IterationCloseReasonKind::Completed
            )
        {
            return Err(DomainError::PolicyRejected);
        }

        self.iteration_state = IterationState::Closed;
        Ok(())
    }

    /// Cancels a planning or committed iteration.
    pub fn cancel(
        &mut self,
        reason: IterationChangeReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if !matches!(
            self.iteration_state,
            IterationState::Planning | IterationState::Committed
        ) {
            return Err(DomainError::InvalidStateTransition);
        }
        if reason.reason_kind != IterationChangeReasonKind::Cancelled {
            return Err(DomainError::PolicyRejected);
        }

        self.iteration_state = IterationState::Cancelled;
        Ok(())
    }

    /// Returns the stable iteration reference.
    pub fn iteration_ref(&self) -> IterationRef {
        IterationRef {
            iteration_id: self.iteration_id.clone(),
        }
    }
}

/// Represents the formal work set committed into one iteration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IterationCommitment {
    /// Stable commitment id.
    pub commitment_id: IterationCommitmentId,
    /// Owning iteration.
    pub iteration_id: IterationId,
    /// Formal work committed into the iteration.
    pub committed_work_refs: FormalWorkRefSet,
    /// Current commitment state.
    pub commitment_state: CommitmentState,
}

impl IterationCommitment {
    /// Builds one candidate commitment set from formal work candidates.
    pub fn from_candidates(
        commitment_id: IterationCommitmentId,
        iteration_id: IterationId,
        candidates: FormalWorkRefSet,
        _actor: ActorRef,
    ) -> Result<Self, DomainError> {
        if commitment_id.0.trim().is_empty() || iteration_id.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        if candidates.refs.is_empty() {
            return Err(DomainError::PolicyRejected);
        }

        let mut unique = Vec::new();
        for work_ref in candidates.refs {
            if unique.contains(&work_ref) {
                return Err(DomainError::PolicyRejected);
            }
            unique.push(work_ref);
        }

        Ok(Self {
            commitment_id,
            iteration_id,
            committed_work_refs: FormalWorkRefSet { refs: unique },
            commitment_state: CommitmentState::Candidate,
        })
    }

    /// Returns whether this commitment contains the given work ref.
    pub fn contains(&self, work_ref: FormalWorkRef) -> bool {
        self.committed_work_refs.refs.contains(&work_ref)
    }

    /// Applies a change set to a committed commitment.
    pub fn apply_change(
        &mut self,
        change_set: IterationCommitmentChangeSet,
        reason: IterationChangeReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.commitment_state != CommitmentState::Committed {
            return Err(DomainError::InvalidStateTransition);
        }
        if reason.reason_kind != IterationChangeReasonKind::CommitmentChanged {
            return Err(DomainError::PolicyRejected);
        }
        if change_set.add_work_refs.is_empty() && change_set.remove_work_refs.is_empty() {
            return Err(DomainError::PolicyRejected);
        }

        for work_ref in change_set.remove_work_refs {
            self.committed_work_refs
                .refs
                .retain(|existing| existing != &work_ref);
        }
        for work_ref in change_set.add_work_refs {
            if !self.committed_work_refs.refs.contains(&work_ref) {
                self.committed_work_refs.refs.push(work_ref);
            }
        }

        self.commitment_state = CommitmentState::Changed;
        Ok(())
    }

    /// Removes one committed work ref while recording an explicit reason.
    pub fn remove(
        &mut self,
        work_ref: FormalWorkRef,
        _reason: CommitmentChangeReason,
    ) -> Result<(), DomainError> {
        if !matches!(
            self.commitment_state,
            CommitmentState::Committed | CommitmentState::Changed
        ) {
            return Err(DomainError::InvalidStateTransition);
        }
        if !self.contains(work_ref.clone()) {
            return Err(DomainError::RefMismatch);
        }

        self.committed_work_refs
            .refs
            .retain(|existing| existing != &work_ref);
        self.commitment_state = CommitmentState::Changed;
        Ok(())
    }

    /// Closes the commitment together with its iteration close path.
    pub fn close(
        &mut self,
        _reason: IterationCloseReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if !matches!(
            self.commitment_state,
            CommitmentState::Committed | CommitmentState::Changed
        ) {
            return Err(DomainError::InvalidStateTransition);
        }

        self.commitment_state = CommitmentState::Closed;
        Ok(())
    }
}

/// Append-only iteration history record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IterationChangeRecord {
    /// Stable change record id.
    pub change_id: IterationChangeId,
    /// Iteration affected by the change.
    pub iteration_ref: IterationRef,
    /// Work refs changed by the commitment update.
    pub changed_work_refs: FormalWorkRefSet,
}

impl IterationChangeRecord {
    /// Builds one change record from an iteration commitment update.
    pub fn from_commitment(
        change_id: IterationChangeId,
        iteration: Iteration,
        commitment: IterationCommitment,
        _actor: ActorRef,
    ) -> Result<Self, DomainError> {
        if change_id.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        if iteration.iteration_id != commitment.iteration_id {
            return Err(DomainError::RefMismatch);
        }

        Ok(Self {
            change_id,
            iteration_ref: iteration.iteration_ref(),
            changed_work_refs: commitment.committed_work_refs,
        })
    }
}
