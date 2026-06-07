//! In-memory command result store for duplicate replay tests.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use work_application::{
    CommandResultRepository, JobResultRepository, RepositoryError, StoredCommandResult,
    StoredJobResult, UnitOfWorkHandle,
};
use work_contracts::ApplicationResultRef;

/// P0 fake command result store keyed by `ApplicationResultRef`.
#[derive(Clone, Default)]
pub struct InMemoryCommandResultRepository {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    stored_commands: HashMap<ApplicationResultRef, StoredCommandResult>,
    stored_jobs: HashMap<ApplicationResultRef, StoredJobResult>,
    missing_reads: HashSet<ApplicationResultRef>,
}

impl InMemoryCommandResultRepository {
    /// Creates an empty in-memory command result store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Forces one result ref to read as missing for failure injection.
    pub fn inject_missing(&self, result_ref: ApplicationResultRef) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.missing_reads.insert(result_ref);
        }
    }
}

#[async_trait]
impl CommandResultRepository for InMemoryCommandResultRepository {
    async fn save_result(
        &self,
        result_ref: ApplicationResultRef,
        result: StoredCommandResult,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        inner.stored_commands.insert(result_ref, result);
        Ok(())
    }

    async fn get_result(
        &self,
        result_ref: ApplicationResultRef,
    ) -> Result<Option<StoredCommandResult>, RepositoryError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if inner.missing_reads.contains(&result_ref) {
            return Ok(None);
        }
        Ok(inner.stored_commands.get(&result_ref).cloned())
    }
}

#[async_trait]
impl JobResultRepository for InMemoryCommandResultRepository {
    async fn save_report(
        &self,
        result_ref: ApplicationResultRef,
        result: StoredJobResult,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        inner.stored_jobs.insert(result_ref, result);
        Ok(())
    }

    async fn get_report(
        &self,
        result_ref: ApplicationResultRef,
    ) -> Result<Option<StoredJobResult>, RepositoryError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if inner.missing_reads.contains(&result_ref) {
            return Ok(None);
        }
        Ok(inner.stored_jobs.get(&result_ref).cloned())
    }
}
