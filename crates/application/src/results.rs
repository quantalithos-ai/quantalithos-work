//! Stored command result surfaces for duplicate replay.

use async_trait::async_trait;

use crate::{RepositoryError, UnitOfWorkHandle};
use work_contracts::{
    ApplicationResultRef, BacklogCommandResult, BlockerCommandResult, DependencyCommandResult,
    IterationCommandResult, ProjectCommandResult, ProjectMemberCommandResult, PromoteCommandResult,
    WorkItemCommandResult,
};

/// Stores public command result surfaces for idempotency duplicate replay.
#[async_trait]
pub trait CommandResultRepository: Send + Sync {
    /// Saves the command result surface under its stable application result ref.
    async fn save_result(
        &self,
        result_ref: ApplicationResultRef,
        result: StoredCommandResult,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;

    /// Loads a previously saved command result surface by result ref.
    async fn get_result(
        &self,
        result_ref: ApplicationResultRef,
    ) -> Result<Option<StoredCommandResult>, RepositoryError>;
}

/// Application-local union of public command result DTOs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoredCommandResult {
    /// Stored result for Project command operations.
    Project(ProjectCommandResult),
    /// Stored result for Backlog command operations.
    Backlog(BacklogCommandResult),
    /// Stored result for ProjectMember command operations.
    ProjectMember(ProjectMemberCommandResult),
    /// Stored result for WorkItem command operations.
    WorkItem(WorkItemCommandResult),
    /// Stored result for Promote command operations.
    Promote(PromoteCommandResult),
    /// Stored result for dependency command operations.
    Dependency(DependencyCommandResult),
    /// Stored result for blocker command operations.
    Blocker(BlockerCommandResult),
    /// Stored result for iteration command operations.
    Iteration(IterationCommandResult),
}

impl StoredCommandResult {
    /// Returns the stored project result when the operation expects it.
    pub fn into_project_result(
        self,
        operation: &core_contracts::metadata::OperationName,
    ) -> Option<ProjectCommandResult> {
        match self {
            Self::Project(result) if result.receipt.result_ref.operation == *operation => {
                Some(result)
            }
            _ => None,
        }
    }

    /// Returns the stored backlog result when the operation expects it.
    pub fn into_backlog_result(
        self,
        operation: &core_contracts::metadata::OperationName,
    ) -> Option<BacklogCommandResult> {
        match self {
            Self::Backlog(result) if result.receipt.result_ref.operation == *operation => {
                Some(result)
            }
            _ => None,
        }
    }

    /// Returns the stored member result when the operation expects it.
    pub fn into_project_member_result(
        self,
        operation: &core_contracts::metadata::OperationName,
    ) -> Option<ProjectMemberCommandResult> {
        match self {
            Self::ProjectMember(result) if result.receipt.result_ref.operation == *operation => {
                Some(result)
            }
            _ => None,
        }
    }

    /// Returns the stored work-item result when the operation expects it.
    pub fn into_work_item_result(
        self,
        operation: &core_contracts::metadata::OperationName,
    ) -> Option<WorkItemCommandResult> {
        match self {
            Self::WorkItem(result) if result.receipt.result_ref.operation == *operation => {
                Some(result)
            }
            _ => None,
        }
    }

    /// Returns the stored promote result when the operation expects it.
    pub fn into_promote_result(
        self,
        operation: &core_contracts::metadata::OperationName,
    ) -> Option<PromoteCommandResult> {
        match self {
            Self::Promote(result) if result.receipt.result_ref.operation == *operation => {
                Some(result)
            }
            _ => None,
        }
    }

    /// Returns the stored dependency result when the operation expects it.
    pub fn into_dependency_result(
        self,
        operation: &core_contracts::metadata::OperationName,
    ) -> Option<DependencyCommandResult> {
        match self {
            Self::Dependency(result) if result.receipt.result_ref.operation == *operation => {
                Some(result)
            }
            _ => None,
        }
    }

    /// Returns the stored blocker result when the operation expects it.
    pub fn into_blocker_result(
        self,
        operation: &core_contracts::metadata::OperationName,
    ) -> Option<BlockerCommandResult> {
        match self {
            Self::Blocker(result) if result.receipt.result_ref.operation == *operation => {
                Some(result)
            }
            _ => None,
        }
    }

    /// Returns the stored iteration result when the operation expects it.
    pub fn into_iteration_result(
        self,
        operation: &core_contracts::metadata::OperationName,
    ) -> Option<IterationCommandResult> {
        match self {
            Self::Iteration(result) if result.receipt.result_ref.operation == *operation => {
                Some(result)
            }
            _ => None,
        }
    }
}
