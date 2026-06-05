//! Shared state and target enums used by Work protocol and domain contracts.

use serde::{Deserialize, Serialize};

/// Lifecycle state for a Work project.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectLifecycleState {
    /// The project accepts normal Work changes.
    Active,
    /// The project can be read but normal Work changes are blocked.
    ReadOnly,
    /// The project is closed for new Work changes.
    Closed,
    /// The project is archived and terminal for normal write paths.
    Archived,
}

/// Availability state for a project's formal backlog.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BacklogState {
    /// The backlog accepts formal work changes.
    Open,
    /// The backlog is locked for maintenance.
    LockedForMaintenance,
    /// The backlog is archived with its project.
    Archived,
}

/// Target availability state requested for a backlog.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BacklogAvailabilityTarget {
    /// Reopen the backlog after maintenance.
    Open,
    /// Lock the backlog for maintenance.
    LockedForMaintenance,
}

/// Publication state for a committed Work outbox record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutboxPublicationState {
    /// The event is waiting for publication.
    Pending,
    /// The event was published successfully.
    Published,
    /// The last publication attempt failed.
    Failed,
}
