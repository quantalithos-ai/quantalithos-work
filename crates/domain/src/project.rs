//! Project and backlog domain objects.

use core_contracts::actor::ActorRef;
use core_contracts::metadata::Timestamp;

use crate::{
    BacklogAvailabilityPolicy, DomainError, MemberResponsibilityPolicy, ProjectLifecyclePolicy,
};
use work_contracts::{
    BacklogAvailabilityTarget, BacklogId, BacklogMaintenanceReason, BacklogRef, BacklogState,
    CapabilityRefSet, ExternalSourceRef, ExternalSourceSystem, GlobalMemberRef, ProjectId,
    ProjectLifecycleReason, ProjectLifecycleState, ProjectLifecycleTarget, ProjectMemberId,
    ProjectMemberReason, ProjectMemberReasonKind, ProjectMemberRef,
    ProjectMemberResponsibilityState, ProjectOwnerRef, ProjectRef, ProjectResponsibilitySpec,
    ProjectSpec,
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
            return Err(DomainError::MissingRequiredValue);
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

/// Tracks a project-local member responsibility without owning identity truth.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectMember {
    /// Stable project member responsibility id.
    pub project_member_id: ProjectMemberId,
    /// Owning project.
    pub project_id: ProjectId,
    /// Referenced identity member.
    pub member_ref: GlobalMemberRef,
    /// Required responsibility specification.
    pub responsibility_spec: ProjectResponsibilitySpec,
    /// Current responsibility state.
    pub responsibility_state: ProjectMemberResponsibilityState,
}

impl ProjectMember {
    /// Creates a proposed project-member responsibility.
    pub fn assign(
        project_member_id: ProjectMemberId,
        project_id: ProjectId,
        member_ref: GlobalMemberRef,
        spec: ProjectResponsibilitySpec,
    ) -> Result<Self, DomainError> {
        if project_member_id.0.trim().is_empty()
            || project_id.0.trim().is_empty()
            || member_ref.0.trim().is_empty()
        {
            return Err(DomainError::MissingRequiredValue);
        }
        MemberResponsibilityPolicy::assert_can_assign(member_ref.clone(), spec.clone())?;
        Ok(Self {
            project_member_id,
            project_id,
            member_ref,
            responsibility_spec: spec,
            responsibility_state: ProjectMemberResponsibilityState::Proposed,
        })
    }

    /// Activates a proposed responsibility when the capability snapshot supports it.
    pub fn activate(
        &mut self,
        snapshot: MemberCapabilitySnapshot,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.responsibility_state != ProjectMemberResponsibilityState::Proposed {
            return Err(DomainError::InvalidStateTransition);
        }
        if snapshot.member_ref != self.member_ref || !snapshot.supports(&self.responsibility_spec) {
            return Err(DomainError::PolicyRejected);
        }
        self.responsibility_state = ProjectMemberResponsibilityState::Active;
        Ok(())
    }

    /// Pauses an active responsibility.
    pub fn pause(
        &mut self,
        reason: ProjectMemberReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.responsibility_state != ProjectMemberResponsibilityState::Active {
            return Err(DomainError::InvalidStateTransition);
        }
        if reason.reason_kind != ProjectMemberReasonKind::Paused {
            return Err(DomainError::PolicyRejected);
        }
        self.responsibility_state = ProjectMemberResponsibilityState::Paused;
        Ok(())
    }

    /// Resumes a paused responsibility when the capability snapshot supports it.
    pub fn resume(
        &mut self,
        snapshot: MemberCapabilitySnapshot,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.responsibility_state != ProjectMemberResponsibilityState::Paused {
            return Err(DomainError::InvalidStateTransition);
        }
        if snapshot.member_ref != self.member_ref || !snapshot.supports(&self.responsibility_spec) {
            return Err(DomainError::PolicyRejected);
        }
        self.responsibility_state = ProjectMemberResponsibilityState::Active;
        Ok(())
    }

    /// Releases a proposed, active, or paused responsibility.
    pub fn release(
        &mut self,
        reason: ProjectMemberReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        match self.responsibility_state {
            ProjectMemberResponsibilityState::Proposed
            | ProjectMemberResponsibilityState::Active
            | ProjectMemberResponsibilityState::Paused => {}
            ProjectMemberResponsibilityState::Released => {
                return Err(DomainError::InvalidStateTransition);
            }
        }
        if reason.reason_kind != ProjectMemberReasonKind::Released {
            return Err(DomainError::PolicyRejected);
        }
        self.responsibility_state = ProjectMemberResponsibilityState::Released;
        Ok(())
    }

    /// Returns the stable project-member ref.
    pub fn project_member_ref(&self) -> ProjectMemberRef {
        ProjectMemberRef {
            project_member_id: self.project_member_id.clone(),
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

/// Tracks whether an external reference is resolved, stale, or failed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceResolutionState {
    /// External reference being tracked.
    pub reference_ref: ExternalSourceRef,
    /// Whether the reference is currently usable.
    pub resolved: bool,
    /// Last successful resolution timestamp.
    pub last_resolved_at: Option<Timestamp>,
}

impl ReferenceResolutionState {
    /// Creates a resolved member-reference state from identity input.
    pub fn resolved_member(member_ref: &GlobalMemberRef) -> Self {
        Self {
            reference_ref: ExternalSourceRef {
                source_system: ExternalSourceSystem::Identity,
                external_id: member_ref.0.clone(),
            },
            resolved: true,
            last_resolved_at: None,
        }
    }
}

/// Stores a safe local summary of a member's responsibility capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberCapabilitySnapshot {
    /// Referenced identity member.
    pub member_ref: GlobalMemberRef,
    /// Capability refs allowed for responsibility checks.
    pub capability_refs: CapabilityRefSet,
    /// Resolution state of this snapshot.
    pub snapshot_state: ReferenceResolutionState,
}

impl MemberCapabilitySnapshot {
    /// Builds a snapshot from safe identity capability input.
    pub fn from_identity(
        member_ref: GlobalMemberRef,
        capability_refs: CapabilityRefSet,
    ) -> Result<Self, DomainError> {
        if member_ref.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        Ok(Self {
            snapshot_state: ReferenceResolutionState::resolved_member(&member_ref),
            member_ref,
            capability_refs,
        })
    }

    /// Returns whether the capability snapshot supports one responsibility spec.
    pub fn supports(&self, spec: &ProjectResponsibilitySpec) -> bool {
        spec.required_capability_refs
            .refs
            .iter()
            .all(|required| self.capability_refs.refs.contains(required))
    }
}
