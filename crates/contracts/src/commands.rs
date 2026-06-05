//! Command DTOs and result DTOs for Work.

use serde::{Deserialize, Serialize};

use core_contracts::{actor::ActorContext, metadata::CommandMetadata};

use crate::handoff::WorkCommandReceipt;
use crate::refs::{
    BacklogMaintenanceReason, BacklogRef, ProjectLifecycleReason, ProjectLifecycleTarget,
    ProjectOwnerRef, ProjectRef, SourceWorkRef,
};
use crate::states::{BacklogAvailabilityTarget, BacklogState, ProjectLifecycleState};

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
