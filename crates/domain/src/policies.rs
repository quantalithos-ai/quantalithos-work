//! Domain policy helpers for Work project, backlog, and formal work transitions.

use core_contracts::actor::ActorRef;

use crate::{Backlog, DomainError, Project};
use work_contracts::{
    BacklogAvailabilityTarget, BacklogMaintenanceReason, EvidenceVerifiedState,
    ExternalEvidenceRef, ExternalSourceSummary, FormalWorkIntent, FormalWorkRef, GlobalMemberRef,
    ProjectLifecycleReason, ProjectLifecycleTarget, ProjectResponsibilitySpec, PromoteDecision,
    PromoteReason, PromoteRejectReason, PromoteRejectReasonKind, SourceWorkKind, SourceWorkRef,
    WorkLifecycleReason, WorkLifecycleReasonKind, WorkPolicyScope, WorkTruthChange,
    WorkTruthSnapshot,
};

/// Guards Work truth ownership and forbidden-body invariants.
pub struct WorkTruthPolicy {
    /// Scope where the policy applies.
    pub policy_scope: WorkPolicyScope,
    /// Current Work truth summary.
    pub truth_snapshot: WorkTruthSnapshot,
}

impl WorkTruthPolicy {
    /// Validates that a truth change remains inside Work-owned boundaries.
    pub fn assert_truth_change_allowed(
        &self,
        change: WorkTruthChange,
        _actor: &ActorRef,
    ) -> Result<(), DomainError> {
        if self.policy_scope.project_ref != self.truth_snapshot.project_ref {
            return Err(DomainError::RefMismatch);
        }

        if self.truth_snapshot.lifecycle_state == work_contracts::ProjectLifecycleState::Archived {
            return Err(DomainError::InvalidStateTransition);
        }

        match change {
            WorkTruthChange::ProjectCreated(project_ref)
            | WorkTruthChange::ProjectLifecycleChanged(project_ref) => {
                if project_ref != self.policy_scope.project_ref {
                    return Err(DomainError::RefMismatch);
                }
            }
            WorkTruthChange::ProjectMemberChanged(_)
            | WorkTruthChange::BacklogAvailabilityChanged(_) => {}
            WorkTruthChange::WorkItemChanged(work_ref) => {
                if self.policy_scope.work_ref.as_ref() != Some(&work_ref) {
                    return Err(DomainError::RefMismatch);
                }
                if matches!(
                    self.truth_snapshot.backlog_state,
                    Some(work_contracts::BacklogState::LockedForMaintenance)
                        | Some(work_contracts::BacklogState::Archived)
                ) {
                    return Err(DomainError::PolicyRejected);
                }
            }
            WorkTruthChange::PromoteResultRecorded(promote_result_ref) => {
                if promote_result_ref.promote_result_id.0.trim().is_empty() {
                    return Err(DomainError::RefMismatch);
                }
            }
            WorkTruthChange::WorkRelationChanged(_) => {}
        }

        Ok(())
    }

    /// Rejects any external source summary that still carries body content.
    pub fn assert_no_external_body(source: ExternalSourceSummary) -> Result<(), DomainError> {
        if source.has_external_body {
            return Err(DomainError::ExternalBodyRejected);
        }
        Ok(())
    }
}

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

/// Guards project member responsibility admission.
pub struct MemberResponsibilityPolicy;

impl MemberResponsibilityPolicy {
    /// Validates that one responsibility spec is structurally assignable.
    pub fn assert_can_assign(
        member_ref: GlobalMemberRef,
        spec: ProjectResponsibilitySpec,
    ) -> Result<(), DomainError> {
        if member_ref.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        if spec
            .required_capability_refs
            .refs
            .iter()
            .any(|capability| capability.0.trim().is_empty())
        {
            return Err(DomainError::PolicyRejected);
        }
        Ok(())
    }
}

/// Guards formal work admission into a backlog.
pub struct FormalWorkPolicy;

impl FormalWorkPolicy {
    /// Validates that one candidate source may become formal work.
    pub fn assert_formal_work(
        intent: FormalWorkIntent,
        source_ref: SourceWorkRef,
    ) -> Result<(), DomainError> {
        if intent.title.0.trim().is_empty()
            || intent.title.0.contains('\n')
            || intent.assignee_ref.project_member_id.0.trim().is_empty()
        {
            return Err(DomainError::PolicyRejected);
        }

        if source_ref.external_ref.external_id.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }

        match source_ref.source_kind {
            SourceWorkKind::Runtime | SourceWorkKind::Process => Err(DomainError::PolicyRejected),
            SourceWorkKind::Conversation
            | SourceWorkKind::Artifact
            | SourceWorkKind::Governance => Ok(()),
        }
    }
}

/// Guards evidence references used to complete or close formal work.
pub struct CompletionEvidencePolicy;

impl CompletionEvidencePolicy {
    /// Validates that one evidence reference can support a formal work completion.
    pub fn assert_completion_evidence(
        _work_ref: FormalWorkRef,
        evidence_ref: ExternalEvidenceRef,
    ) -> Result<(), DomainError> {
        if evidence_ref.verified_state != EvidenceVerifiedState::Verified {
            return Err(DomainError::PolicyRejected);
        }
        Ok(())
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

/// Guards whether one source may be promoted into Work truth.
pub struct PromotePolicy;

impl PromotePolicy {
    /// Returns the promote decision for one source and request reason.
    pub fn can_promote(source_ref: SourceWorkRef, _reason: PromoteReason) -> PromoteDecision {
        match source_ref.source_kind {
            SourceWorkKind::Runtime | SourceWorkKind::Process => {
                PromoteDecision::Reject(PromoteRejectReason {
                    reason_kind: PromoteRejectReasonKind::PolicyRejected,
                    reason_ref: None,
                })
            }
            SourceWorkKind::Conversation
            | SourceWorkKind::Artifact
            | SourceWorkKind::Governance => PromoteDecision::Allow,
        }
    }
}

pub(crate) fn assert_work_lifecycle_reason(
    target: work_contracts::WorkLifecycleTarget,
    reason: &WorkLifecycleReason,
) -> Result<(), DomainError> {
    match target {
        work_contracts::WorkLifecycleTarget::InProgress => {
            if reason.reason_kind != WorkLifecycleReasonKind::Start {
                return Err(DomainError::PolicyRejected);
            }
        }
        work_contracts::WorkLifecycleTarget::Completed => {
            if reason.reason_kind != WorkLifecycleReasonKind::CompletionEvidence {
                return Err(DomainError::PolicyRejected);
            }
        }
        work_contracts::WorkLifecycleTarget::Cancelled => {
            if reason.reason_kind != WorkLifecycleReasonKind::Cancellation {
                return Err(DomainError::PolicyRejected);
            }
        }
        work_contracts::WorkLifecycleTarget::Superseded => {
            if reason.reason_kind != WorkLifecycleReasonKind::Superseded
                || reason.superseding_ref.is_none()
            {
                return Err(DomainError::PolicyRejected);
            }
        }
    }

    Ok(())
}
