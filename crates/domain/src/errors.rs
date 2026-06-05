//! Domain error types for Work.

use thiserror::Error;

/// Errors returned by Work domain objects and policies.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum DomainError {
    /// A required field or input was missing.
    #[error("missing required value")]
    MissingRequiredValue,
    /// The requested state transition is not allowed.
    #[error("invalid state transition")]
    InvalidStateTransition,
    /// A policy rejected the requested truth or state change.
    #[error("policy rejected")]
    PolicyRejected,
    /// A domain invariant was violated by an impossible field combination.
    #[error("invariant violation")]
    InvariantViolation,
    /// External body content was rejected from entering Work truth.
    #[error("external body rejected")]
    ExternalBodyRejected,
    /// A projection or read path attempted to mutate business truth.
    #[error("projection mutation rejected")]
    ProjectionMutationRejected,
    /// The provided reference does not match the expected owner or subject.
    #[error("reference mismatch")]
    RefMismatch,
}
