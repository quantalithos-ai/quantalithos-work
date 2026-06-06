//! Command DTOs and result DTOs for Work.

use serde::{Deserialize, Serialize};

use core_contracts::{actor::ActorContext, metadata::CommandMetadata};

use crate::handoff::WorkCommandReceipt;
use crate::refs::{
    BacklogMaintenanceReason, BacklogRef, BlockerCauseRef, CapabilityRefSet,
    DependencyChangeReason, DependencyReason, DependencyTarget, ExternalEvidenceRef,
    FormalWorkIntent, FormalWorkRef, FormalWorkRefSet, GlobalMemberRef, IterationChangeReason,
    IterationCloseReason, IterationCommitmentChangeSet, IterationLifecycleTarget, IterationRef,
    ProcessTimeboxRef, ProjectLifecycleReason, ProjectLifecycleTarget, ProjectMemberReason,
    ProjectMemberRef, ProjectOwnerRef, ProjectRef, ProjectResponsibilityKind, PromoteReason,
    PromoteResultRef, PromoteReviewDecision, ResponsibilityTarget, SourceWorkRef, WorkBlockerRef,
    WorkDependencyRef, WorkLifecycleReason, WorkLifecycleTarget,
};
use crate::states::{
    BacklogAvailabilityTarget, BacklogState, BlockerState, CommitmentState, DependencyState,
    IterationState, ProjectLifecycleState, ProjectMemberResponsibilityState, PromoteResultState,
    WorkItemState,
};

/// A synchronous Work command envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkCommandEnvelope<T> {
    /// Effective actor and entrypoint context.
    pub actor: ActorContext,
    /// Core command metadata; request.idempotency_key must be Some.
    pub metadata: CommandMetadata,
    /// Operation-specific command body.
    pub command: T,
}

/// Describes a project to be created by Work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectSpec {
    /// External owner pointer for the project.
    pub owner_ref: ProjectOwnerRef,
    /// Optional external source summary ref for audit.
    pub source_ref: Option<SourceWorkRef>,
}

/// Describes a project-local member responsibility.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectResponsibilitySpec {
    /// Responsibility kind expected in the project.
    pub responsibility_kind: ProjectResponsibilityKind,
    /// Required capability references.
    pub required_capability_refs: CapabilityRefSet,
}

/// Requests creation of a Work-owned project subject.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    /// Project creation specification.
    pub project_spec: ProjectSpec,
}

/// Requests a project lifecycle transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateProjectLifecycleRequest {
    /// Project to update.
    pub project_ref: ProjectRef,
    /// Target lifecycle state.
    pub target: ProjectLifecycleTarget,
    /// Reason for the transition.
    pub reason: ProjectLifecycleReason,
    /// Expected project version.
    pub expected_version: core_contracts::metadata::Version,
}

/// Requests a backlog availability transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateBacklogAvailabilityRequest {
    /// Backlog to update.
    pub backlog_ref: BacklogRef,
    /// Target availability state.
    pub target: BacklogAvailabilityTarget,
    /// Maintenance reason.
    pub reason: BacklogMaintenanceReason,
    /// Expected backlog version.
    pub expected_version: core_contracts::metadata::Version,
}

/// Requests assignment of a project-local responsibility.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssignProjectMemberRequest {
    /// Project that owns the responsibility.
    pub project_ref: ProjectRef,
    /// Referenced identity member.
    pub member_ref: GlobalMemberRef,
    /// Responsibility specification.
    pub responsibility_spec: ProjectResponsibilitySpec,
}

/// Requests a project member responsibility state transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateProjectMemberResponsibilityRequest {
    /// Project member responsibility to update.
    pub project_member_ref: ProjectMemberRef,
    /// Target responsibility transition.
    pub target: ResponsibilityTarget,
    /// Reason for the transition.
    pub reason: ProjectMemberReason,
    /// Expected project member version.
    pub expected_version: core_contracts::metadata::Version,
}

/// Requests creation of a root formal work item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateWorkItemRequest {
    /// Project that owns the work.
    pub project_ref: ProjectRef,
    /// Formal work intent.
    pub work_intent: FormalWorkIntent,
    /// External source reference.
    pub source_ref: SourceWorkRef,
}

/// Requests creation of a formal child work item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateChildWorkItemRequest {
    /// Parent formal work item.
    pub parent_ref: FormalWorkRef,
    /// Formal child work intent.
    pub work_intent: FormalWorkIntent,
    /// External source reference.
    pub source_ref: SourceWorkRef,
}

/// Requests a lifecycle transition for formal work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateWorkItemLifecycleRequest {
    /// Formal work record to update.
    pub work_ref: FormalWorkRef,
    /// Target lifecycle state.
    pub target: WorkLifecycleTarget,
    /// Reason for the transition.
    pub reason: WorkLifecycleReason,
    /// Completion or transition evidence when required.
    pub evidence_ref: Option<ExternalEvidenceRef>,
    /// Expected formal work version.
    pub expected_version: core_contracts::metadata::Version,
}

/// Requests review of an external source for formal Work promotion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RequestWorkPromotionRequest {
    /// Source to evaluate.
    pub source_ref: SourceWorkRef,
    /// Reason for promotion.
    pub reason: PromoteReason,
}

/// Requests a review decision for a promote result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReviewWorkPromotionRequest {
    /// Promote result under review.
    pub promote_result_ref: PromoteResultRef,
    /// Review decision.
    pub decision: PromoteReviewDecision,
    /// Optional formal work intent when accepting into a new Work item.
    pub accepted_work_intent: Option<FormalWorkIntent>,
    /// Expected promote result version.
    pub expected_version: core_contracts::metadata::Version,
}

/// Requests creation of a dependency between formal work records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LinkWorkDependencyRequest {
    /// Work that must happen first.
    pub upstream_work_ref: FormalWorkRef,
    /// Work affected by the dependency.
    pub downstream_work_ref: FormalWorkRef,
    /// Reason for linking.
    pub reason: DependencyReason,
}

/// Requests a dependency state transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateWorkDependencyStateRequest {
    /// Dependency to update.
    pub dependency_ref: WorkDependencyRef,
    /// Target dependency state.
    pub target: DependencyTarget,
    /// Reason for the state change.
    pub reason: DependencyChangeReason,
    /// Evidence when required by target.
    pub evidence_ref: Option<ExternalEvidenceRef>,
    /// Expected dependency version.
    pub expected_version: core_contracts::metadata::Version,
}

/// Requests opening a work blocker.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OpenWorkBlockerRequest {
    /// Formal work blocked by this record.
    pub blocked_work_ref: FormalWorkRef,
    /// Cause reference.
    pub cause_ref: BlockerCauseRef,
}

/// Requests blocker resolution.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResolveWorkBlockerRequest {
    /// Blocker to resolve.
    pub blocker_ref: WorkBlockerRef,
    /// Evidence used for resolution.
    pub evidence_ref: ExternalEvidenceRef,
    /// Expected blocker version.
    pub expected_version: core_contracts::metadata::Version,
}

/// Requests opening a Work-owned iteration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OpenIterationRequest {
    /// Project that owns the iteration.
    pub project_ref: ProjectRef,
    /// External process timebox pointer.
    pub timebox_ref: ProcessTimeboxRef,
}

/// Requests commitment of an iteration work scope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommitIterationScopeRequest {
    /// Iteration to commit.
    pub iteration_ref: IterationRef,
    /// Candidate formal work refs.
    pub candidate_work_refs: FormalWorkRefSet,
    /// Expected iteration version.
    pub expected_iteration_version: core_contracts::metadata::Version,
}

/// Requests changes to an iteration commitment.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateIterationCommitmentRequest {
    /// Iteration whose commitment is changed.
    pub iteration_ref: IterationRef,
    /// Change set to apply.
    pub change_set: IterationCommitmentChangeSet,
    /// Reason for the change.
    pub reason: IterationChangeReason,
    /// Expected commitment version.
    pub expected_commitment_version: core_contracts::metadata::Version,
}

/// Requests an iteration lifecycle transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateIterationLifecycleRequest {
    /// Iteration to update.
    pub iteration_ref: IterationRef,
    /// Target iteration state.
    pub target: IterationLifecycleTarget,
    /// Required for `target = InProgress` and `target = Cancelled`; forbidden for `target = Closed`.
    pub change_reason: Option<IterationChangeReason>,
    /// Required for `target = Closed`; forbidden for `target = InProgress` and `target = Cancelled`.
    pub close_reason: Option<IterationCloseReason>,
    /// Expected iteration version.
    pub expected_version: core_contracts::metadata::Version,
}

/// Idempotency result visible to command and job callers.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyResultView {
    /// The request executed and produced a new result.
    Applied,
    /// The same request digest returned a previously completed result.
    Duplicate,
}

/// Result returned by project commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectCommandResult {
    /// Changed project reference.
    pub project_ref: ProjectRef,
    /// Current project lifecycle state.
    pub lifecycle_state: ProjectLifecycleState,
    /// Shared write receipt.
    pub receipt: WorkCommandReceipt,
}

impl ProjectCommandResult {
    /// Returns a duplicate replay view while preserving the stored result surface.
    pub fn with_duplicate_receipt(&self) -> Self {
        let mut result = self.clone();
        result.receipt = result.receipt.with_duplicate_overlay();
        result
    }
}

/// Result returned by backlog commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BacklogCommandResult {
    /// Changed backlog reference.
    pub backlog_ref: BacklogRef,
    /// Current backlog state.
    pub backlog_state: BacklogState,
    /// Shared write receipt.
    pub receipt: WorkCommandReceipt,
}

impl BacklogCommandResult {
    /// Returns a duplicate replay view while preserving the stored result surface.
    pub fn with_duplicate_receipt(&self) -> Self {
        let mut result = self.clone();
        result.receipt = result.receipt.with_duplicate_overlay();
        result
    }
}

/// Result returned by project member commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemberCommandResult {
    /// Changed project member reference.
    pub project_member_ref: ProjectMemberRef,
    /// Current responsibility state.
    pub responsibility_state: ProjectMemberResponsibilityState,
    /// Shared write receipt.
    pub receipt: WorkCommandReceipt,
}

impl ProjectMemberCommandResult {
    /// Returns a duplicate replay view while preserving the stored result surface.
    pub fn with_duplicate_receipt(&self) -> Self {
        let mut result = self.clone();
        result.receipt = result.receipt.with_duplicate_overlay();
        result
    }
}

/// Result returned by formal work commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkItemCommandResult {
    /// Changed formal work reference.
    pub work_ref: FormalWorkRef,
    /// Current formal work lifecycle state.
    pub work_state: WorkItemState,
    /// Shared write receipt.
    pub receipt: WorkCommandReceipt,
}

impl WorkItemCommandResult {
    /// Returns a duplicate replay view while preserving the stored result surface.
    pub fn with_duplicate_receipt(&self) -> Self {
        let mut result = self.clone();
        result.receipt = result.receipt.with_duplicate_overlay();
        result
    }
}

/// Result returned by promote commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PromoteCommandResult {
    /// Promote result reference.
    pub promote_result_ref: PromoteResultRef,
    /// Current promote state.
    pub result_state: PromoteResultState,
    /// Formal work created by accepted promotion.
    pub created_work_ref: Option<FormalWorkRef>,
    /// Shared write receipt.
    pub receipt: WorkCommandReceipt,
}

impl PromoteCommandResult {
    /// Returns a duplicate replay view while preserving the stored result surface.
    pub fn with_duplicate_receipt(&self) -> Self {
        let mut result = self.clone();
        result.receipt = result.receipt.with_duplicate_overlay();
        result
    }
}

/// Result returned by dependency commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DependencyCommandResult {
    /// Changed dependency reference.
    pub dependency_ref: WorkDependencyRef,
    /// Current dependency state.
    pub dependency_state: DependencyState,
    /// Shared write receipt.
    pub receipt: WorkCommandReceipt,
}

impl DependencyCommandResult {
    /// Returns a duplicate replay view while preserving the stored result surface.
    pub fn with_duplicate_receipt(&self) -> Self {
        let mut result = self.clone();
        result.receipt = result.receipt.with_duplicate_overlay();
        result
    }
}

/// Result returned by blocker commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockerCommandResult {
    /// Changed blocker reference.
    pub blocker_ref: WorkBlockerRef,
    /// Current blocker state.
    pub blocker_state: BlockerState,
    /// Shared write receipt.
    pub receipt: WorkCommandReceipt,
}

impl BlockerCommandResult {
    /// Returns a duplicate replay view while preserving the stored result surface.
    pub fn with_duplicate_receipt(&self) -> Self {
        let mut result = self.clone();
        result.receipt = result.receipt.with_duplicate_overlay();
        result
    }
}

/// Result returned by iteration commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IterationCommandResult {
    /// Changed iteration reference.
    pub iteration_ref: IterationRef,
    /// Current iteration state.
    pub iteration_state: IterationState,
    /// Current commitment state when a commitment is involved.
    pub commitment_state: Option<CommitmentState>,
    /// Shared write receipt.
    pub receipt: WorkCommandReceipt,
}

impl IterationCommandResult {
    /// Returns a duplicate replay view while preserving the stored result surface.
    pub fn with_duplicate_receipt(&self) -> Self {
        let mut result = self.clone();
        result.receipt = result.receipt.with_duplicate_overlay();
        result
    }
}
