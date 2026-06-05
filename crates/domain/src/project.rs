//! Project and backlog domain objects.

use core_contracts::actor::ActorRef;
use core_contracts::metadata::Timestamp;

use crate::policies::{FormalWorkPolicy, assert_work_lifecycle_reason};
use crate::{
    BacklogAvailabilityPolicy, DomainError, MemberResponsibilityPolicy, ProjectLifecyclePolicy,
};
use work_contracts::{
    BacklogAvailabilityTarget, BacklogId, BacklogMaintenanceReason, BacklogRef, BacklogState,
    CapabilityRefSet, ChildWorkItemId, EvidenceVerifiedState, ExternalEvidenceRef,
    ExternalSourceRef, ExternalSourceSystem, FormalWorkIntent, FormalWorkRef, GlobalMemberRef,
    IterationRef, ProjectId, ProjectLifecycleReason, ProjectLifecycleState, ProjectLifecycleTarget,
    ProjectMemberId, ProjectMemberReason, ProjectMemberReasonKind, ProjectMemberRef,
    ProjectMemberResponsibilityState, ProjectOwnerRef, ProjectRef, ProjectResponsibilitySpec,
    ProjectSpec, SourceWorkRef, WorkItemId, WorkItemState, WorkLifecycleReason,
    WorkLifecycleTarget,
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

    /// Validates that one formal work candidate may enter this backlog.
    pub fn assert_can_accept(&self, intent: &FormalWorkIntent) -> Result<(), DomainError> {
        if self.backlog_state != BacklogState::Open {
            return Err(DomainError::PolicyRejected);
        }
        if intent.assignee_ref.project_member_id.0.trim().is_empty()
            || intent.title.0.trim().is_empty()
            || intent.title.0.contains('\n')
        {
            return Err(DomainError::PolicyRejected);
        }
        Ok(())
    }

    /// Validates that one work item belongs to this backlog and is formalized work.
    pub fn accept_work_item(
        &self,
        work_item: &WorkItem,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.backlog_state != BacklogState::Open {
            return Err(DomainError::PolicyRejected);
        }
        if work_item.backlog_id != self.backlog_id {
            return Err(DomainError::RefMismatch);
        }
        Ok(())
    }
}

/// Represents a formal collaborative work item admitted into a backlog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkItem {
    /// Stable formal work id.
    pub work_item_id: WorkItemId,
    /// Owning backlog.
    pub backlog_id: BacklogId,
    /// Current assignee inside the project.
    pub assignee_ref: ProjectMemberRef,
    /// Current formal work lifecycle state.
    pub work_state: WorkItemState,
    /// Optional external completion evidence.
    pub completion_ref: Option<ExternalEvidenceRef>,
}

impl WorkItem {
    /// Creates a formal work item from an accepted formalization intent.
    pub fn formalize(
        work_item_id: WorkItemId,
        backlog_id: BacklogId,
        intent: FormalWorkIntent,
        source_ref: SourceWorkRef,
        _actor: ActorRef,
    ) -> Result<Self, DomainError> {
        FormalWorkPolicy::assert_formal_work(intent.clone(), source_ref)?;
        if work_item_id.0.trim().is_empty() || backlog_id.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }

        Ok(Self {
            work_item_id,
            backlog_id,
            assignee_ref: intent.assignee_ref,
            work_state: WorkItemState::Formalized,
            completion_ref: None,
        })
    }

    /// Returns the stable formal work ref.
    pub fn formal_work_ref(&self) -> FormalWorkRef {
        FormalWorkRef::WorkItem(self.work_item_id.clone())
    }

    /// Reassigns the work item.
    pub fn assign(
        &mut self,
        member_ref: ProjectMemberRef,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if member_ref.project_member_id.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        self.assignee_ref = member_ref;
        Ok(())
    }

    /// Marks the work item as committed into an iteration scope.
    pub fn mark_committed(
        &mut self,
        _iteration_ref: IterationRef,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.work_state != WorkItemState::Formalized {
            return Err(DomainError::InvalidStateTransition);
        }
        self.work_state = WorkItemState::Committed;
        Ok(())
    }

    /// Executes one lifecycle transition.
    pub fn transition_lifecycle(
        &mut self,
        target: WorkLifecycleTarget,
        reason: WorkLifecycleReason,
        evidence_ref: Option<ExternalEvidenceRef>,
        actor: ActorRef,
    ) -> Result<(), DomainError> {
        assert_work_lifecycle_reason(target, &reason)?;
        match target {
            WorkLifecycleTarget::InProgress => {
                if !matches!(
                    self.work_state,
                    WorkItemState::Formalized | WorkItemState::Committed
                ) {
                    return Err(DomainError::InvalidStateTransition);
                }
                self.work_state = WorkItemState::InProgress;
                Ok(())
            }
            WorkLifecycleTarget::Completed => {
                let evidence_ref = evidence_ref.ok_or(DomainError::PolicyRejected)?;
                self.mark_completed(evidence_ref, actor)
            }
            WorkLifecycleTarget::Cancelled => {
                if !matches!(
                    self.work_state,
                    WorkItemState::Formalized | WorkItemState::Committed
                ) {
                    return Err(DomainError::InvalidStateTransition);
                }
                self.work_state = WorkItemState::Cancelled;
                Ok(())
            }
            WorkLifecycleTarget::Superseded => {
                if matches!(
                    self.work_state,
                    WorkItemState::Completed | WorkItemState::Cancelled | WorkItemState::Superseded
                ) {
                    return Err(DomainError::InvalidStateTransition);
                }
                self.work_state = WorkItemState::Superseded;
                Ok(())
            }
        }
    }

    /// Marks the work item completed with verified evidence.
    pub fn mark_completed(
        &mut self,
        evidence_ref: ExternalEvidenceRef,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.work_state != WorkItemState::InProgress {
            return Err(DomainError::InvalidStateTransition);
        }
        if evidence_ref.verified_state != EvidenceVerifiedState::Verified {
            return Err(DomainError::PolicyRejected);
        }
        self.completion_ref = Some(evidence_ref);
        self.work_state = WorkItemState::Completed;
        Ok(())
    }
}

/// Represents a formal child work item split from a parent work item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChildWorkItem {
    /// Stable child work id.
    pub child_work_item_id: ChildWorkItemId,
    /// Parent formal work id.
    pub parent_work_item_id: WorkItemId,
    /// Source used for split or promotion.
    pub source_ref: SourceWorkRef,
    /// Current child work lifecycle state.
    pub work_state: WorkItemState,
    /// Completion evidence reference when the child work is completed.
    pub completion_ref: Option<ExternalEvidenceRef>,
}

impl ChildWorkItem {
    /// Creates a formal child work item from an accepted split intent.
    pub fn create_child(
        child_work_item_id: ChildWorkItemId,
        parent_id: WorkItemId,
        intent: FormalWorkIntent,
        source_ref: SourceWorkRef,
    ) -> Result<Self, DomainError> {
        FormalWorkPolicy::assert_formal_work(intent, source_ref.clone())?;
        if child_work_item_id.0.trim().is_empty() || parent_id.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        Ok(Self {
            child_work_item_id,
            parent_work_item_id: parent_id,
            source_ref,
            work_state: WorkItemState::Formalized,
            completion_ref: None,
        })
    }

    /// Returns the stable formal work ref.
    pub fn formal_work_ref(&self) -> FormalWorkRef {
        FormalWorkRef::ChildWorkItem(self.child_work_item_id.clone())
    }

    /// Rebinds the child to one parent if it is unchanged.
    pub fn attach_to_parent(
        &mut self,
        parent_id: WorkItemId,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.parent_work_item_id != parent_id {
            return Err(DomainError::RefMismatch);
        }
        Ok(())
    }

    /// Records an explicit promote source without storing the external body.
    pub fn promote_from_source(
        &mut self,
        source_ref: SourceWorkRef,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if source_ref.external_ref.external_id.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        self.source_ref = source_ref;
        Ok(())
    }

    /// Executes one lifecycle transition for child work.
    pub fn transition_lifecycle(
        &mut self,
        target: WorkLifecycleTarget,
        reason: WorkLifecycleReason,
        evidence_ref: Option<ExternalEvidenceRef>,
        actor: ActorRef,
    ) -> Result<(), DomainError> {
        assert_work_lifecycle_reason(target, &reason)?;
        match target {
            WorkLifecycleTarget::InProgress => {
                if !matches!(
                    self.work_state,
                    WorkItemState::Formalized | WorkItemState::Committed
                ) {
                    return Err(DomainError::InvalidStateTransition);
                }
                self.work_state = WorkItemState::InProgress;
                Ok(())
            }
            WorkLifecycleTarget::Completed => {
                if self.work_state != WorkItemState::InProgress {
                    return Err(DomainError::InvalidStateTransition);
                }
                let evidence_ref = evidence_ref.ok_or(DomainError::PolicyRejected)?;
                if evidence_ref.verified_state != EvidenceVerifiedState::Verified {
                    return Err(DomainError::PolicyRejected);
                }
                self.completion_ref = Some(evidence_ref);
                self.work_state = WorkItemState::Completed;
                let _ = actor;
                Ok(())
            }
            WorkLifecycleTarget::Cancelled => {
                if !matches!(
                    self.work_state,
                    WorkItemState::Formalized | WorkItemState::Committed
                ) {
                    return Err(DomainError::InvalidStateTransition);
                }
                self.work_state = WorkItemState::Cancelled;
                Ok(())
            }
            WorkLifecycleTarget::Superseded => {
                if matches!(
                    self.work_state,
                    WorkItemState::Completed | WorkItemState::Cancelled | WorkItemState::Superseded
                ) {
                    return Err(DomainError::InvalidStateTransition);
                }
                self.work_state = WorkItemState::Superseded;
                Ok(())
            }
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
