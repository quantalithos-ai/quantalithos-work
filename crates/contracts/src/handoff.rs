//! Shared handoff and receipt helper types.

use serde::{Deserialize, Serialize};

use core_contracts::metadata::{OperationName, RequestId, RequestMetadata, TraceId};

use crate::refs::{ResultId, WorkOutboxId, WorkTraceId};

/// Associates core trace and request metadata with Work trace logic.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkTraceContextRef {
    /// Core trace id.
    pub trace_id: TraceId,
    /// Optional core request id.
    pub request_id: Option<RequestId>,
}

impl WorkTraceContextRef {
    /// Creates a trace context ref from shared request metadata.
    pub fn from_metadata(metadata: &RequestMetadata) -> Self {
        Self {
            trace_id: metadata.trace_id.clone(),
            request_id: Some(metadata.request_id.clone()),
        }
    }
}

/// Carries a stable reference to an application result for idempotency.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ApplicationResultRef {
    /// Operation that produced the result.
    pub operation: OperationName,
    /// Stable result id or receipt id.
    pub result_id: ResultId,
}

impl ApplicationResultRef {
    /// Builds a stable application result ref for one operation.
    pub fn for_operation(operation: OperationName, result_id: ResultId) -> Self {
        Self {
            operation,
            result_id,
        }
    }
}

/// Shared write receipt returned by Work commands.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkCommandReceipt {
    /// Stable application result reference used by idempotency.
    pub result_ref: ApplicationResultRef,
    /// Idempotency outcome for this request.
    pub idempotency: crate::commands::IdempotencyResultView,
    /// Trace record created or reused by the operation.
    pub trace_ref: Option<WorkTraceId>,
    /// Outbox records enqueued by the operation.
    pub outbox_record_refs: Vec<WorkOutboxId>,
    /// Version of the primary changed record after commit.
    pub applied_version: Option<core_contracts::metadata::Version>,
}

impl WorkCommandReceipt {
    /// Creates an applied receipt for a newly accepted write result.
    pub fn applied(
        result_ref: ApplicationResultRef,
        trace_ref: Option<WorkTraceId>,
        outbox_record_refs: Vec<WorkOutboxId>,
        applied_version: core_contracts::metadata::Version,
    ) -> Self {
        Self {
            result_ref,
            idempotency: crate::commands::IdempotencyResultView::Applied,
            trace_ref,
            outbox_record_refs,
            applied_version: Some(applied_version),
        }
    }

    /// Returns a duplicate replay view of the same stored receipt.
    pub fn with_duplicate_overlay(&self) -> Self {
        let mut receipt = self.clone();
        receipt.idempotency = crate::commands::IdempotencyResultView::Duplicate;
        receipt
    }
}
