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

/// Responsibility state for a member inside one Work project.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMemberResponsibilityState {
    /// The responsibility has been proposed but is not yet active.
    Proposed,
    /// The member can currently take project work.
    Active,
    /// The member responsibility is temporarily paused.
    Paused,
    /// The member responsibility has been released.
    Released,
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

/// Lifecycle state for a formal Work item.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemState {
    /// The work item is formally admitted into the backlog.
    Formalized,
    /// The work item is committed into an iteration scope.
    Committed,
    /// The work item is actively being worked.
    InProgress,
    /// The work item is completed with accepted evidence.
    Completed,
    /// The work item was cancelled before completion.
    Cancelled,
    /// The work item was superseded by another formal work record.
    Superseded,
}

/// Decision state for an external source promotion review.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromoteResultState {
    /// The source is waiting for review.
    PendingReview,
    /// The source was accepted and linked to formal work.
    Accepted,
    /// The source was rejected with a reason.
    Rejected,
    /// The decision was superseded by a later review.
    Superseded,
}

/// Lifecycle state for a formal work dependency.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyState {
    /// The dependency is proposed but not yet active.
    Proposed,
    /// The dependency is active.
    Active,
    /// The dependency was satisfied by evidence.
    Satisfied,
    /// The dependency was explicitly waived.
    Waived,
    /// The dependency was cancelled.
    Cancelled,
}

/// Lifecycle state for a formal work blocker.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerState {
    /// The blocker is open.
    Open,
    /// Mitigation is in progress.
    Mitigating,
    /// The blocker was resolved by evidence.
    Resolved,
    /// The blocker record is closed.
    Closed,
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
