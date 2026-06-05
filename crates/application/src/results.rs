//! Stored command result surfaces for duplicate replay.

use async_trait::async_trait;

use crate::{RepositoryError, UnitOfWorkHandle};
use work_contracts::{
    ApplicationResultRef, BacklogCommandResult, ProjectCommandResult, ProjectMemberCommandResult,
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
}
