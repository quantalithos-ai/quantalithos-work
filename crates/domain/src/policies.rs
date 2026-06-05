//! Domain policy helpers for Work project and backlog state transitions.

use core_contracts::actor::ActorRef;

use crate::{Backlog, DomainError, Project};
use work_contracts::{
    BacklogAvailabilityTarget, BacklogMaintenanceReason, ProjectLifecycleReason,
    ProjectLifecycleTarget,
};

/// Guards project lifecycle transitions.
pub struct ProjectLifecyclePolicy;

impl ProjectLifecyclePolicy {
    /// Validates a project lifecycle transition before mutation.
    pub fn assert_lifecycle_transition_allowed(
        project: &Project,
        target: ProjectLifecycleTarget,
        _reason: &ProjectLifecycleReason,
        _actor: &ActorRef,
    ) -> Result<(), DomainError> {
        match (project.lifecycle_state, target) {
            (work_contracts::ProjectLifecycleState::Active, ProjectLifecycleTarget::ReadOnly)
            | (work_contracts::ProjectLifecycleState::Active, ProjectLifecycleTarget::Closed)
            | (work_contracts::ProjectLifecycleState::ReadOnly, ProjectLifecycleTarget::Closed)
            | (work_contracts::ProjectLifecycleState::Closed, ProjectLifecycleTarget::Archived) => {
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition),
        }
    }
}

/// Guards backlog availability transitions.
pub struct BacklogAvailabilityPolicy;

impl BacklogAvailabilityPolicy {
    /// Validates a backlog availability transition before mutation.
    pub fn assert_availability_transition_allowed(
        backlog: &Backlog,
        target: BacklogAvailabilityTarget,
        _reason: &BacklogMaintenanceReason,
        _actor: &ActorRef,
    ) -> Result<(), DomainError> {
        match (backlog.backlog_state, target) {
            (
                work_contracts::BacklogState::Open,
                BacklogAvailabilityTarget::LockedForMaintenance,
            )
            | (
                work_contracts::BacklogState::LockedForMaintenance,
                BacklogAvailabilityTarget::Open,
            ) => Ok(()),
            _ => Err(DomainError::InvalidStateTransition),
        }
    }
}
