//! In-memory idempotency store for Work duplicate replay tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use core_contracts::metadata::{IdempotencyKey, OperationName};
use work_application::{
    IdempotencyConflict, IdempotencyError, IdempotencyRecord, IdempotencyRepository,
    IdempotencyReservation, IdempotencyStatus, RequestDigest, UnitOfWorkHandle,
};
use work_contracts::ApplicationResultRef;

/// P0 fake idempotency repository keyed by `(operation, key)`.
#[derive(Clone, Default)]
pub struct InMemoryIdempotencyRepository {
    inner: Arc<Mutex<HashMap<(String, String), IdempotencyRecord>>>,
}

impl InMemoryIdempotencyRepository {
    /// Creates an empty in-memory idempotency repository.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl IdempotencyRepository for InMemoryIdempotencyRepository {
    async fn get(
        &self,
        key: IdempotencyKey,
        operation: OperationName,
    ) -> Result<Option<IdempotencyRecord>, IdempotencyError> {
        let inner = self
            .inner
            .lock()
            .map_err(|_| IdempotencyError::StoreUnavailable)?;
        Ok(inner
            .get(&(operation.as_str().to_owned(), key.as_str().to_owned()))
            .cloned())
    }

    async fn reserve(
        &self,
        key: IdempotencyKey,
        operation: OperationName,
        request_digest: RequestDigest,
        _uow: &UnitOfWorkHandle,
    ) -> Result<IdempotencyReservation, IdempotencyError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| IdempotencyError::StoreUnavailable)?;
        let map_key = (operation.as_str().to_owned(), key.as_str().to_owned());
        if let Some(existing) = inner.get(&map_key) {
            if existing.request_digest != request_digest {
                return Ok(IdempotencyReservation::Conflict(IdempotencyConflict {
                    idempotency_key: key,
                    operation,
                    existing_digest: existing.request_digest.clone(),
                    incoming_digest: request_digest,
                }));
            }
            return match existing.status {
                IdempotencyStatus::Completed => existing
                    .result_ref
                    .clone()
                    .map(IdempotencyReservation::Duplicate)
                    .ok_or(IdempotencyError::StoreUnavailable),
                IdempotencyStatus::Reserved => Err(IdempotencyError::AlreadyReserved),
                IdempotencyStatus::Conflict => Err(IdempotencyError::Conflict),
            };
        }

        let record = IdempotencyRecord::reserved(key.clone(), operation.clone(), request_digest);
        inner.insert(map_key, record.clone());
        Ok(IdempotencyReservation::Reserved(record))
    }

    async fn complete(
        &self,
        reservation: IdempotencyReservation,
        result_ref: ApplicationResultRef,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), IdempotencyError> {
        let IdempotencyReservation::Reserved(record) = reservation else {
            return Err(IdempotencyError::StoreUnavailable);
        };

        let mut inner = self
            .inner
            .lock()
            .map_err(|_| IdempotencyError::StoreUnavailable)?;
        let key = (
            record.operation.as_str().to_owned(),
            record.idempotency_key.as_str().to_owned(),
        );
        let current = inner
            .get_mut(&key)
            .ok_or(IdempotencyError::StoreUnavailable)?;
        current.status = IdempotencyStatus::Completed;
        current.result_ref = Some(result_ref);
        Ok(())
    }

    async fn mark_conflict(
        &self,
        conflict: IdempotencyConflict,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), IdempotencyError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| IdempotencyError::StoreUnavailable)?;
        let key = (
            conflict.operation.as_str().to_owned(),
            conflict.idempotency_key.as_str().to_owned(),
        );
        let record = inner
            .get_mut(&key)
            .ok_or(IdempotencyError::StoreUnavailable)?;
        record.status = IdempotencyStatus::Conflict;
        Ok(())
    }
}
