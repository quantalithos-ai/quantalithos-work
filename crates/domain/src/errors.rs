//! Domain error types for Work.

use thiserror::Error;

/// Errors returned by Work domain objects and policies.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum DomainError {
    /// A required field or input was missing.
    #[error("missing required field")]
    MissingField,
    /// The requested state transition is not allowed.
    #[error("invalid state transition")]
    InvalidStateTransition,
    /// The provided reference does not match the expected owner or subject.
    #[error("reference mismatch")]
    RefMismatch,
}
