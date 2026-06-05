//! Public error surfaces exposed by Work protocol contracts.

use serde::{Deserialize, Serialize};

/// Public protocol error surface for Work.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkProtocolError {
    /// Required actor, metadata, idempotency, or body field is missing.
    InvalidRequest,
    /// The caller cannot see or modify the requested resource.
    NotVisible,
    /// The requested Work-owned resource does not exist.
    NotFound,
    /// The requested transition violates Work domain rules.
    DomainRejected,
    /// The expected optimistic version did not match.
    VersionConflict,
    /// The idempotency key was reused with a different request digest.
    IdempotencyConflict,
    /// An external reference could not be resolved.
    ExternalReferenceUnresolved,
    /// An inbound event could not be accepted and must be dead-lettered.
    DeadLetter,
    /// A projection, publisher, repository, or handoff dependency failed.
    TemporarilyUnavailable,
}
