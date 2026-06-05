//! Idempotency contracts and helpers for Work write paths.

use async_trait::async_trait;
use core_contracts::{
    actor::ActorContext,
    metadata::{IdempotencyKey, OperationName},
};
use serde::{Deserialize, Serialize};

use crate::UnitOfWorkHandle;
use work_contracts::ApplicationResultRef;

/// Canonical digest used to distinguish same-intent retries from key reuse conflicts.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestDigest(pub String);

impl RequestDigest {
    /// Builds a canonical command digest from stable business input only.
    pub fn from_canonical_command_input<T>(
        operation: &OperationName,
        actor: &ActorContext,
        command: &T,
    ) -> Result<Self, serde_json::Error>
    where
        T: Serialize,
    {
        let actor_id = actor.actor.actor_id.as_str();
        let payload = serde_json::json!({
            "operation": operation.as_str(),
            "actor_id": actor_id,
            "actor_kind": actor.actor.actor_kind,
            "delegated_by": actor.delegated_by.as_ref().map(|v| v.actor_id.as_str()),
            "role_refs": actor.role_refs.iter().map(|v| v.as_str()).collect::<Vec<_>>(),
            "request_origin": actor.request_origin,
            "command": command,
        });
        serde_json::to_string(&payload).map(Self)
    }
}

/// Current idempotency lifecycle state for one protected operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdempotencyStatus {
    /// The key has been reserved and may still be in flight.
    Reserved,
    /// The key completed successfully and has a stable result surface.
    Completed,
    /// The key was reused with a different canonical digest.
    Conflict,
}

/// Stores idempotency state for a write or job operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    /// Operation-specific idempotency key.
    pub idempotency_key: IdempotencyKey,
    /// Operation name protected by this record.
    pub operation: OperationName,
    /// Canonical request digest.
    pub request_digest: RequestDigest,
    /// Existing result when completed.
    pub result_ref: Option<ApplicationResultRef>,
    /// Current idempotency status.
    pub status: IdempotencyStatus,
}

impl IdempotencyRecord {
    /// Returns a new reserved record for the supplied request identity.
    pub fn reserved(
        idempotency_key: IdempotencyKey,
        operation: OperationName,
        request_digest: RequestDigest,
    ) -> Self {
        Self {
            idempotency_key,
            operation,
            request_digest,
            result_ref: None,
            status: IdempotencyStatus::Reserved,
        }
    }
}

/// Records a conflicting idempotency request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IdempotencyConflict {
    /// Idempotency key that collided.
    pub idempotency_key: IdempotencyKey,
    /// Operation protected by the key.
    pub operation: OperationName,
    /// Digest stored for the existing request.
    pub existing_digest: RequestDigest,
    /// Digest presented by the new request.
    pub incoming_digest: RequestDigest,
}

/// Result of reserving an idempotency key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IdempotencyReservation {
    /// This request may execute the operation.
    Reserved(IdempotencyRecord),
    /// The same digest already completed and should return the stored result.
    Duplicate(ApplicationResultRef),
    /// The same key was used with a different request digest.
    Conflict(IdempotencyConflict),
}

/// Classifies idempotency storage failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdempotencyError {
    /// The same key is already in-flight.
    AlreadyReserved,
    /// The same key was used with a different digest.
    Conflict,
    /// The idempotency store failed.
    StoreUnavailable,
}

/// Stores idempotency state for commands, event consumers, and jobs.
#[async_trait]
pub trait IdempotencyRepository: Send + Sync {
    /// Reads an idempotency record for duplicate recovery and commit-status audit.
    async fn get(
        &self,
        key: IdempotencyKey,
        operation: OperationName,
    ) -> Result<Option<IdempotencyRecord>, IdempotencyError>;

    /// Reserves an idempotency key for an operation and canonical request digest.
    async fn reserve(
        &self,
        key: IdempotencyKey,
        operation: OperationName,
        request_digest: RequestDigest,
        uow: &UnitOfWorkHandle,
    ) -> Result<IdempotencyReservation, IdempotencyError>;

    /// Completes a reservation with the stable application result reference.
    async fn complete(
        &self,
        reservation: IdempotencyReservation,
        result_ref: ApplicationResultRef,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), IdempotencyError>;

    /// Marks an idempotency key as conflicted.
    async fn mark_conflict(
        &self,
        conflict: IdempotencyConflict,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), IdempotencyError>;
}
