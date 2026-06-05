//! Application error types for Work services.

use thiserror::Error;

use work_contracts::WorkProtocolError;

/// Errors returned by application services before protocol mapping.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ApplicationError {
    /// Required actor, metadata, idempotency, or body field is missing.
    #[error("invalid request")]
    InvalidRequest,
    /// A Work-owned record required by the operation does not exist.
    #[error("not found")]
    NotFound,
    /// The caller is not allowed to see or modify the target.
    #[error("not visible")]
    NotVisible,
    /// Domain invariants or policies rejected the requested change.
    #[error("domain rejected")]
    DomainRejected,
    /// The expected optimistic version did not match stored state.
    #[error("version conflict")]
    VersionConflict,
    /// The idempotency key was reused with a different canonical digest.
    #[error("idempotency conflict")]
    IdempotencyConflict,
    /// A required external reference could not be resolved or accepted.
    #[error("external reference unresolved")]
    ExternalReferenceUnresolved,
    /// A repository, port, or local transaction is temporarily unavailable.
    #[error("temporarily unavailable")]
    TemporarilyUnavailable,
    /// Commit may have partially applied and requires reconciliation.
    #[error("commit status unknown")]
    CommitStatusUnknown,
    /// Duplicate replay could not load the stored result surface.
    #[error("duplicate result missing")]
    DuplicateResultMissing,
}

impl ApplicationError {
    /// Maps the application error to the public command protocol error.
    pub fn into_protocol_error(self) -> WorkProtocolError {
        match self {
            Self::InvalidRequest => WorkProtocolError::InvalidRequest,
            Self::NotFound => WorkProtocolError::NotFound,
            Self::NotVisible => WorkProtocolError::NotVisible,
            Self::DomainRejected => WorkProtocolError::DomainRejected,
            Self::VersionConflict => WorkProtocolError::VersionConflict,
            Self::IdempotencyConflict => WorkProtocolError::IdempotencyConflict,
            Self::ExternalReferenceUnresolved => WorkProtocolError::ExternalReferenceUnresolved,
            Self::TemporarilyUnavailable
            | Self::CommitStatusUnknown
            | Self::DuplicateResultMissing => WorkProtocolError::TemporarilyUnavailable,
        }
    }
}
