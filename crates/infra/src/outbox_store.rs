//! In-memory outbox store for CORE command service tests.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use core_contracts::metadata::{PageRequest, Version};
use work_application::{
    Page, PageInfo, RepositoryError, UnitOfWorkHandle, Versioned, WorkOutboxRepository,
};
use work_contracts::{OutboxFailureReason, OutboxPublicationRef, OutboxRetryReason, WorkOutboxId};
use work_domain::WorkOutboxRecord;

/// P0 fake outbox repository supporting enqueue and status transitions.
#[derive(Clone, Default)]
pub struct InMemoryWorkOutboxRepository {
    state: Arc<Mutex<HashMap<String, (WorkOutboxRecord, Version)>>>,
    version_conflicts: Arc<Mutex<HashSet<String>>>,
}

impl InMemoryWorkOutboxRepository {
    /// Creates an empty outbox store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the number of stored outbox records.
    pub fn count(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.len())
            .unwrap_or_default()
    }

    /// Forces one outbox id to return a version conflict on the next state marker update.
    pub fn inject_version_conflict(&self, outbox_id: &WorkOutboxId) {
        if let Ok(mut conflicts) = self.version_conflicts.lock() {
            conflicts.insert(outbox_id.0.clone());
        }
    }
}

#[async_trait]
impl WorkOutboxRepository for InMemoryWorkOutboxRepository {
    async fn enqueue(
        &self,
        record: WorkOutboxRecord,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        state.insert(record.outbox_id.0.clone(), (record, 1));
        Ok(())
    }

    async fn list_pending(
        &self,
        _page: PageRequest,
    ) -> Result<Page<Versioned<WorkOutboxRecord>>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let mut items = state
            .values()
            .filter_map(|(record, version)| {
                (record.publication_state == work_contracts::OutboxPublicationState::Pending)
                    .then_some(Versioned {
                        record: record.clone(),
                        version: *version,
                    })
            })
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.record.outbox_id.0.cmp(&right.record.outbox_id.0));
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn get(
        &self,
        outbox_id: WorkOutboxId,
    ) -> Result<Option<Versioned<WorkOutboxRecord>>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state.get(&outbox_id.0).map(|(record, version)| Versioned {
            record: record.clone(),
            version: *version,
        }))
    }

    async fn mark_published(
        &self,
        outbox_id: WorkOutboxId,
        publication_ref: OutboxPublicationRef,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        if self
            .version_conflicts
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?
            .remove(&outbox_id.0)
        {
            return Err(RepositoryError::VersionConflict);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let (record, version) = state
            .get_mut(&outbox_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if *version != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        record
            .mark_published(publication_ref)
            .map_err(|_| RepositoryError::TransactionRejected)?;
        *version += 1;
        Ok(*version)
    }

    async fn mark_failed(
        &self,
        outbox_id: WorkOutboxId,
        reason: OutboxFailureReason,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        if self
            .version_conflicts
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?
            .remove(&outbox_id.0)
        {
            return Err(RepositoryError::VersionConflict);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let (record, version) = state
            .get_mut(&outbox_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if *version != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        record
            .mark_failed(reason)
            .map_err(|_| RepositoryError::TransactionRejected)?;
        *version += 1;
        Ok(*version)
    }

    async fn mark_pending_for_retry(
        &self,
        outbox_id: WorkOutboxId,
        reason: OutboxRetryReason,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let (record, version) = state
            .get_mut(&outbox_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if *version != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        record
            .mark_pending_for_retry(reason)
            .map_err(|_| RepositoryError::TransactionRejected)?;
        *version += 1;
        Ok(*version)
    }
}
