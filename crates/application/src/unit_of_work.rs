//! Local transaction boundary contracts for Work write services.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Stable unit-of-work id used by fake adapters and diagnostics.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnitOfWorkId(pub String);

/// Opaque local transaction handle passed to repository writes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnitOfWorkHandle {
    /// Stable handle id for logging and fake adapter assertions.
    pub handle_id: UnitOfWorkId,
}

/// Classifies local transaction boundary failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitOfWorkError {
    /// The transaction could not be started.
    BeginFailed,
    /// The transaction could not be committed.
    CommitFailed,
    /// The transaction could not be rolled back.
    RollbackFailed,
}

/// Opens and closes local transaction boundaries.
#[async_trait]
pub trait UnitOfWork: Send + Sync {
    /// Starts a new local write boundary.
    async fn begin(&self) -> Result<UnitOfWorkHandle, UnitOfWorkError>;

    /// Commits the current transaction boundary.
    async fn commit(&self, handle: UnitOfWorkHandle) -> Result<(), UnitOfWorkError>;

    /// Rolls back the current transaction boundary.
    async fn rollback(&self, handle: UnitOfWorkHandle) -> Result<(), UnitOfWorkError>;
}
