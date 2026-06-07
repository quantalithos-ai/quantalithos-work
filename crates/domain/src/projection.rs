//! Projection freshness state used by query and maintenance paths.

use crate::DomainError;
use work_contracts::{DerivedFreshnessState, DerivedWorkViewRef, WorkTruthCursor};

/// Safe failure reason for one projection rebuild or replace attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionFailureReason {
    /// Safe error message without raw body content.
    pub message: String,
    /// Source cursor associated with the failure.
    pub source_cursor: WorkTruthCursor,
}

impl ProjectionFailureReason {
    /// Builds a failure reason from one rebuild or replace error.
    pub fn from_build_error(source_cursor: WorkTruthCursor, message: String) -> Self {
        Self {
            message,
            source_cursor,
        }
    }
}

/// Tracks freshness for one derived Work consumption view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedWorkViewState {
    /// Derived view reference.
    pub view_ref: DerivedWorkViewRef,
    /// Last source cursor covered by the view.
    pub source_cursor: WorkTruthCursor,
    /// Current freshness state.
    pub freshness_state: DerivedFreshnessState,
}

impl DerivedWorkViewState {
    /// Initializes a fresh state wrapper for one derived view.
    pub fn for_view(view_ref: DerivedWorkViewRef) -> Self {
        Self {
            view_ref,
            source_cursor: WorkTruthCursor(String::new()),
            freshness_state: DerivedFreshnessState::Fresh,
        }
    }

    /// Marks this view stale at the given source cursor.
    pub fn mark_stale(&mut self, cursor: WorkTruthCursor) -> Result<(), DomainError> {
        self.source_cursor = cursor;
        self.freshness_state = DerivedFreshnessState::Stale;
        Ok(())
    }

    /// Marks this view as rebuilding at the given source cursor.
    pub fn mark_rebuilding(&mut self, cursor: WorkTruthCursor) -> Result<(), DomainError> {
        self.source_cursor = cursor;
        self.freshness_state = DerivedFreshnessState::Rebuilding;
        Ok(())
    }

    /// Marks this view fresh at the given source cursor.
    pub fn mark_fresh(&mut self, cursor: WorkTruthCursor) -> Result<(), DomainError> {
        self.source_cursor = cursor;
        self.freshness_state = DerivedFreshnessState::Fresh;
        Ok(())
    }

    /// Marks this view failed while preserving query-visible freshness state.
    pub fn mark_failed(
        &mut self,
        cursor: WorkTruthCursor,
        _reason: ProjectionFailureReason,
    ) -> Result<(), DomainError> {
        self.source_cursor = cursor;
        self.freshness_state = DerivedFreshnessState::Failed;
        Ok(())
    }
}
