//! Command DTOs and result DTOs for Work.

use serde::{Deserialize, Serialize};

use core_contracts::{actor::ActorContext, metadata::CommandMetadata};

use crate::handoff::WorkCommandReceipt;
use crate::refs::{
    BacklogMaintenanceReason, BacklogRef, CapabilityRefSet, FormalWorkIntent, FormalWorkRef,
    GlobalMemberRef, ProjectLifecycleReason, ProjectLifecycleTarget, ProjectMemberReason,
    ProjectMemberRef, ProjectOwnerRef, ProjectRef, ProjectResponsibilityKind, ResponsibilityTarget,
    SourceWorkRef,
};
use crate::states::{
    BacklogAvailabilityTarget, BacklogState, ProjectLifecycleState,
    ProjectMemberResponsibilityState, WorkItemState,
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
