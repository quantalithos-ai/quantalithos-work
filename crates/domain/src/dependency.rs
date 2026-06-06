//! Dependency, blocker, and relation history domain objects.

use core_contracts::actor::ActorRef;

use crate::DomainError;
use work_contracts::{
    BlockerCauseRef, BlockerImpactExplanation, BlockerState, DependencyChangeId,
    DependencyChangeReason, DependencyChangeReasonKind, DependencyOrBlockerRef, DependencyReason,
    DependencyState, DependencyTarget, ExternalEvidenceRef, FormalWorkRef, ProjectRef,
    SafeSummaryText, WorkBlockerId, WorkBlockerRef, WorkDependencyId, WorkDependencyRef,
};

/// Represents an explainable dependency between formal work records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkDependency {
    /// Stable dependency id.
    pub dependency_id: WorkDependencyId,
    /// Work that must happen first.
    pub upstream_work_ref: FormalWorkRef,
    /// Work affected by the dependency.
    pub downstream_work_ref: FormalWorkRef,
    /// Current dependency lifecycle state.
    pub dependency_state: DependencyState,
}

impl WorkDependency {
    /// Establishes a proposed dependency between two formal work records.
    pub fn link(
        dependency_id: WorkDependencyId,
        upstream: FormalWorkRef,
        downstream: FormalWorkRef,
        _reason: DependencyReason,
    ) -> Result<Self, DomainError> {
        if dependency_id.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        if upstream == downstream {
            return Err(DomainError::PolicyRejected);
        }

        Ok(Self {
            dependency_id,
            upstream_work_ref: upstream,
            downstream_work_ref: downstream,
            dependency_state: DependencyState::Proposed,
        })
    }

    /// Activates a proposed dependency.
    pub fn activate(
        &mut self,
        _actor: ActorRef,
        reason: DependencyChangeReason,
    ) -> Result<(), DomainError> {
        if self.dependency_state != DependencyState::Proposed {
            return Err(DomainError::InvalidStateTransition);
        }
        if reason.reason_kind != DependencyChangeReasonKind::Activated {
            return Err(DomainError::PolicyRejected);
        }
        self.dependency_state = DependencyState::Active;
        Ok(())
    }

    /// Marks an active dependency as satisfied with verified evidence.
    pub fn mark_satisfied(
        &mut self,
        evidence_ref: ExternalEvidenceRef,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.dependency_state != DependencyState::Active {
            return Err(DomainError::InvalidStateTransition);
        }
        if evidence_ref.verified_state != work_contracts::EvidenceVerifiedState::Verified {
            return Err(DomainError::PolicyRejected);
        }
        self.dependency_state = DependencyState::Satisfied;
        Ok(())
    }

    /// Waives an active dependency.
    pub fn waive(
        &mut self,
        reason: DependencyChangeReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.dependency_state != DependencyState::Active {
            return Err(DomainError::InvalidStateTransition);
        }
        if reason.reason_kind != DependencyChangeReasonKind::Waived {
            return Err(DomainError::PolicyRejected);
        }
        self.dependency_state = DependencyState::Waived;
        Ok(())
    }

    /// Cancels a proposed or active dependency.
    pub fn cancel(
        &mut self,
        reason: DependencyChangeReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if !matches!(
            self.dependency_state,
            DependencyState::Proposed | DependencyState::Active
        ) {
            return Err(DomainError::InvalidStateTransition);
        }
        if reason.reason_kind != DependencyChangeReasonKind::Cancelled {
            return Err(DomainError::PolicyRejected);
        }
        self.dependency_state = DependencyState::Cancelled;
        Ok(())
    }

    /// Returns the stable dependency reference.
    pub fn dependency_ref(&self) -> WorkDependencyRef {
        WorkDependencyRef {
            dependency_id: self.dependency_id.clone(),
        }
    }
}

/// Represents a blocker that prevents or degrades progress on formal work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkBlocker {
    /// Stable blocker id.
    pub blocker_id: WorkBlockerId,
    /// Formal work blocked by this record.
    pub blocked_work_ref: FormalWorkRef,
    /// Reference describing the blocker cause.
    pub cause_ref: BlockerCauseRef,
    /// Current blocker lifecycle state.
    pub blocker_state: BlockerState,
    /// Evidence that resolved the blocker, present only after successful resolution.
    pub resolved_evidence_ref: Option<ExternalEvidenceRef>,
}

impl WorkBlocker {
    /// Creates an open blocker record.
    pub fn open(
        blocker_id: WorkBlockerId,
        work_ref: FormalWorkRef,
        cause_ref: BlockerCauseRef,
        _actor: ActorRef,
    ) -> Result<Self, DomainError> {
        if blocker_id.0.trim().is_empty() || cause_ref.source_ref.external_id.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }

        Ok(Self {
            blocker_id,
            blocked_work_ref: work_ref,
            cause_ref,
            blocker_state: BlockerState::Open,
            resolved_evidence_ref: None,
        })
    }

    /// Moves an open blocker into mitigating state.
    pub fn start_mitigation(
        &mut self,
        _reason: work_contracts::BlockerMitigationReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.blocker_state != BlockerState::Open {
            return Err(DomainError::InvalidStateTransition);
        }
        self.blocker_state = BlockerState::Mitigating;
        Ok(())
    }

    /// Resolves an open or mitigating blocker with verified evidence.
    pub fn resolve(
        &mut self,
        evidence_ref: ExternalEvidenceRef,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if !matches!(
            self.blocker_state,
            BlockerState::Open | BlockerState::Mitigating
        ) {
            return Err(DomainError::InvalidStateTransition);
        }
        if evidence_ref.verified_state != work_contracts::EvidenceVerifiedState::Verified {
            return Err(DomainError::PolicyRejected);
        }
        self.blocker_state = BlockerState::Resolved;
        self.resolved_evidence_ref = Some(evidence_ref);
        Ok(())
    }

    /// Closes a resolved blocker record.
    pub fn close(
        &mut self,
        _reason: work_contracts::BlockerCloseReason,
        _actor: ActorRef,
    ) -> Result<(), DomainError> {
        if self.blocker_state != BlockerState::Resolved {
            return Err(DomainError::InvalidStateTransition);
        }
        self.blocker_state = BlockerState::Closed;
        Ok(())
    }

    /// Produces a read-only impact explanation.
    pub fn explain_impact(&self) -> BlockerImpactExplanation {
        BlockerImpactExplanation {
            blocker_ref: self.blocker_ref(),
            affected_work_ref: self.blocked_work_ref.clone(),
            summary: SafeSummaryText("blocked formal work".to_owned()),
        }
    }

    /// Returns the stable blocker reference.
    pub fn blocker_ref(&self) -> WorkBlockerRef {
        WorkBlockerRef {
            blocker_id: self.blocker_id.clone(),
        }
    }
}

/// Append-only dependency or blocker history record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyChangeRecord {
    /// Stable change record id.
    pub change_id: DependencyChangeId,
    /// Dependency or blocker reference affected by the change.
    pub relation_ref: DependencyOrBlockerRef,
    /// Reason for the relation change.
    pub change_reason: DependencyChangeReason,
}

impl DependencyChangeRecord {
    /// Builds a history record from a dependency change.
    pub fn from_dependency_change(
        change_id: DependencyChangeId,
        dependency: WorkDependency,
        reason: DependencyChangeReason,
    ) -> Result<Self, DomainError> {
        if change_id.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        Ok(Self {
            change_id,
            relation_ref: DependencyOrBlockerRef::Dependency(dependency.dependency_ref()),
            change_reason: reason,
        })
    }

    /// Builds a history record from a blocker change.
    pub fn from_blocker_change(
        change_id: DependencyChangeId,
        blocker: WorkBlocker,
        reason: DependencyChangeReason,
    ) -> Result<Self, DomainError> {
        if change_id.0.trim().is_empty() {
            return Err(DomainError::MissingRequiredValue);
        }
        Ok(Self {
            change_id,
            relation_ref: DependencyOrBlockerRef::Blocker(blocker.blocker_ref()),
            change_reason: reason,
        })
    }
}

/// Snapshot of dependency edges used by the graph policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyGraphSnapshot {
    /// Project scope that produced this graph snapshot.
    pub project_ref: ProjectRef,
    /// Formal work edges in the project.
    pub dependency_edges: Vec<(FormalWorkRef, FormalWorkRef)>,
    /// Currently active blockers by formal work.
    pub active_blockers: Vec<(FormalWorkRef, WorkBlockerRef)>,
}

/// Guards dependency graph invariants and state transitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyGraphPolicy {
    /// Current dependency graph snapshot.
    pub graph_snapshot: DependencyGraphSnapshot,
}

impl DependencyGraphPolicy {
    /// Builds the policy from one repository-loaded graph snapshot.
    pub fn from_graph(graph_snapshot: DependencyGraphSnapshot) -> Self {
        Self { graph_snapshot }
    }

    /// Validates that a dependency edge may be created.
    pub fn assert_can_link(
        graph: &DependencyGraphSnapshot,
        upstream: FormalWorkRef,
        downstream: FormalWorkRef,
    ) -> Result<(), DomainError> {
        if upstream == downstream {
            return Err(DomainError::PolicyRejected);
        }

        if graph
            .dependency_edges
            .iter()
            .any(|(existing_upstream, existing_downstream)| {
                *existing_upstream == downstream && *existing_downstream == upstream
            })
        {
            return Err(DomainError::PolicyRejected);
        }

        let mut stack = vec![downstream.clone()];
        let mut visited = Vec::new();
        let edges = &graph.dependency_edges;
        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.push(current.clone());
            if current == upstream {
                return Err(DomainError::PolicyRejected);
            }
            for (_, next) in edges.iter().filter(|(src, _)| *src == current) {
                stack.push(next.clone());
            }
        }

        Ok(())
    }

    /// Validates a dependency state transition reason before mutation.
    pub fn assert_dependency_state_transition_allowed(
        &self,
        dependency: &WorkDependency,
        target: DependencyTarget,
        reason: &DependencyChangeReason,
        evidence_ref: Option<&ExternalEvidenceRef>,
    ) -> Result<(), DomainError> {
        match target {
            DependencyTarget::Active => {
                if dependency.dependency_state != DependencyState::Proposed
                    || reason.reason_kind != DependencyChangeReasonKind::Activated
                {
                    return Err(DomainError::PolicyRejected);
                }
                Self::assert_can_link(
                    &self.graph_snapshot,
                    dependency.upstream_work_ref.clone(),
                    dependency.downstream_work_ref.clone(),
                )?;
            }
            DependencyTarget::Satisfied => {
                if dependency.dependency_state != DependencyState::Active
                    || reason.reason_kind != DependencyChangeReasonKind::SatisfiedByEvidence
                    || evidence_ref.is_none()
                {
                    return Err(DomainError::PolicyRejected);
                }
            }
            DependencyTarget::Waived => {
                if dependency.dependency_state != DependencyState::Active
                    || reason.reason_kind != DependencyChangeReasonKind::Waived
                {
                    return Err(DomainError::PolicyRejected);
                }
            }
            DependencyTarget::Cancelled => {
                if !matches!(
                    dependency.dependency_state,
                    DependencyState::Proposed | DependencyState::Active
                ) || reason.reason_kind != DependencyChangeReasonKind::Cancelled
                {
                    return Err(DomainError::PolicyRejected);
                }
            }
        }

        Ok(())
    }
}
