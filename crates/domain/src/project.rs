//! Project and backlog domain objects.

use core_contracts::actor::ActorRef;

use crate::{BacklogAvailabilityPolicy, DomainError, ProjectLifecyclePolicy};
use work_contracts::{
    BacklogAvailabilityTarget, BacklogId, BacklogMaintenanceReason, BacklogRef, BacklogState,
    ProjectId, ProjectLifecycleReason, ProjectLifecycleState, ProjectLifecycleTarget,
    ProjectOwnerRef, ProjectRef, ProjectSpec,
};

/// Represents the Work-owned project subject and protects its lifecycle boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    /// Stable Work project identity.
    pub project_id: ProjectId,
    /// External owner pointer without owner body.
    pub owner_ref: ProjectOwnerRef,
    /// Current lifecycle state.
    pub lifecycle_state: ProjectLifecycleState,
}

impl Project {
    /// Creates an active Work project.
    pub fn create(
        project_id: ProjectId,
        spec: ProjectSpec,
        _actor: ActorRef,
    ) -> Result<Self, DomainError> {
        if spec.owner_ref.external_ref.external_id.trim().is_empty() {
            return Err(DomainError::MissingField);
        }

        Ok(Self {
            project_id,
            owner_ref: spec.owner_ref,
            lifecycle_state: ProjectLifecycleState::Active,
        })
    }

    /// Executes a lifecycle transition.
    pub fn transition_lifecycle(
        &mut self,
        target: ProjectLifecycleTarget,
        reason: ProjectLifecycleReason,
        actor: ActorRef,
    ) -> Result<(), DomainError> {
        ProjectLifecyclePolicy::assert_lifecycle_transition_allowed(self, target, &reason, &actor)?;
        self.lifecycle_state = match target {
            ProjectLifecycleTarget::ReadOnly => ProjectLifecycleState::ReadOnly,
            ProjectLifecycleTarget::Closed => ProjectLifecycleState::Closed,
            ProjectLifecycleTarget::Archived => ProjectLifecycleState::Archived,
        };
        Ok(())
    }

    /// Closes the project for new Work writes.
    pub fn close(
        &mut self,
        actor: ActorRef,
        reason: ProjectLifecycleReason,
    ) -> Result<(), DomainError> {
        self.transition_lifecycle(ProjectLifecycleTarget::Closed, reason, actor)
    }

    /// Returns the project ref for this domain object.
    pub fn project_ref(&self) -> ProjectRef {
        ProjectRef {
            project_id: self.project_id.clone(),
        }
    }
}

/// Owns the formal Work universe for one project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Backlog {
    /// Stable backlog identity.
    pub backlog_id: BacklogId,
    /// Project that owns this backlog.
    pub project_id: ProjectId,
    /// Current availability state.
    pub backlog_state: BacklogState,
}

impl Backlog {
    /// Creates the formal backlog for one project.
    pub fn open_for_project(
        backlog_id: BacklogId,
        project_id: ProjectId,
        _actor: ActorRef,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            backlog_id,
            project_id,
            backlog_state: BacklogState::Open,
        })
    }

    /// Executes an availability transition.
    pub fn transition_availability(
        &mut self,
        target: BacklogAvailabilityTarget,
        reason: BacklogMaintenanceReason,
        actor: ActorRef,
    ) -> Result<(), DomainError> {
        BacklogAvailabilityPolicy::assert_availability_transition_allowed(
            self, target, &reason, &actor,
        )?;
        self.backlog_state = match target {
            BacklogAvailabilityTarget::Open => BacklogState::Open,
            BacklogAvailabilityTarget::LockedForMaintenance => BacklogState::LockedForMaintenance,
        };
        Ok(())
    }

    /// Locks the backlog for maintenance.
    pub fn lock_for_maintenance(
        &mut self,
        reason: BacklogMaintenanceReason,
        actor: ActorRef,
    ) -> Result<(), DomainError> {
        self.transition_availability(
            BacklogAvailabilityTarget::LockedForMaintenance,
            reason,
            actor,
        )
    }

    /// Reopens the backlog after maintenance.
    pub fn reopen_after_maintenance(
        &mut self,
        reason: BacklogMaintenanceReason,
        actor: ActorRef,
    ) -> Result<(), DomainError> {
        self.transition_availability(BacklogAvailabilityTarget::Open, reason, actor)
    }

    /// Archives the backlog with its owning project.
    pub fn archive_with_project(
        &mut self,
        project_ref: ProjectRef,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if project_ref.project_id != self.project_id {
            return Err(DomainError::RefMismatch);
        }

        match self.backlog_state {
            BacklogState::Open => {
                self.backlog_state = BacklogState::Archived;
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition),
        }
    }

    /// Returns the backlog ref for this domain object.
    pub fn backlog_ref(&self) -> BacklogRef {
        BacklogRef {
            backlog_id: self.backlog_id.clone(),
        }
    }
}
