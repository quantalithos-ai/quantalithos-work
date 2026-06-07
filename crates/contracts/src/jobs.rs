//! Operations job DTOs and reports for Work.

use serde::{Deserialize, Serialize};

use core_contracts::{
    actor::ActorContext,
    metadata::{CommandMetadata, PageRequest},
};

use crate::handoff::WorkCommandReceipt;
use crate::refs::{
    ArchiveHandoffScope, ArchiveHandoffTargetRef, ExternalReferenceScope, JobRunId, ProjectRef,
    TraceHandoffTargetRef, WorkJobFailureRef, WorkReconciliationScopeRef, WorkTraceSubjectRef,
};

/// Selects which Work projections a rebuild job should replace.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkProjectionSet {
    /// Rebuild every project-scoped projection.
    All,
    /// Rebuild only project board views.
    ProjectBoard,
    /// Rebuild only member work views.
    MemberWork,
    /// Rebuild only iteration summary views.
    IterationSummary,
    /// Rebuild only search projection rows.
    Search,
}

/// Common metadata for Work operations jobs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkJobMetadata {
    /// Job run id.
    pub job_run_id: JobRunId,
    /// Actor running the job.
    pub actor: ActorContext,
    /// Idempotency metadata for the job.
    pub command_metadata: CommandMetadata,
}

/// Common report returned by Work jobs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkJobReport {
    /// Job run id.
    pub job_run_id: JobRunId,
    /// Shared write receipt when the job wrote local state.
    pub receipt: Option<WorkCommandReceipt>,
    /// Number of records scanned.
    pub scanned_count: u64,
    /// Number of records changed.
    pub changed_count: u64,
    /// Failed item refs that require retry or inspection.
    pub failed_refs: Vec<WorkJobFailureRef>,
}

impl WorkJobReport {
    /// Returns a duplicate replay view while preserving the stored report surface.
    pub fn with_duplicate_receipt(&self) -> Self {
        let mut report = self.clone();
        report.receipt = report
            .receipt
            .as_ref()
            .map(WorkCommandReceipt::with_duplicate_overlay);
        report
    }
}

/// Publishes pending Work outbox records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PublishWorkOutboxJobInput {
    /// Common job metadata.
    pub metadata: WorkJobMetadata,
    /// Page request for pending outbox records.
    pub page: PageRequest,
}

/// Rebuilds derived Work projections from committed truth.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RebuildWorkProjectionsJobInput {
    /// Common job metadata.
    pub metadata: WorkJobMetadata,
    /// Project scope to rebuild.
    pub project_ref: ProjectRef,
    /// Projection set to rebuild.
    pub projection_set: WorkProjectionSet,
}

/// Refreshes external reference snapshots.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RefreshExternalReferenceSnapshotsJobInput {
    /// Common job metadata.
    pub metadata: WorkJobMetadata,
    /// Optional reference scope; absent means stale references.
    pub reference_scope: Option<ExternalReferenceScope>,
    /// Page request for references.
    pub page: PageRequest,
}

/// Runs reconciliation over Work truth, projections, outbox, and references.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunWorkReconciliationJobInput {
    /// Common job metadata.
    pub metadata: WorkJobMetadata,
    /// Reconciliation scope.
    pub scope_ref: WorkReconciliationScopeRef,
}

/// Prepares trace handoff to observability or archive consumers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrepareWorkTraceHandoffJobInput {
    /// Common job metadata.
    pub metadata: WorkJobMetadata,
    /// Trace subject scope.
    pub subject_ref: WorkTraceSubjectRef,
    /// Handoff target.
    pub target_ref: TraceHandoffTargetRef,
}

/// Prepares archive handoff markers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrepareArchiveHandoffJobInput {
    /// Common job metadata.
    pub metadata: WorkJobMetadata,
    /// Archive scope.
    pub archive_scope: ArchiveHandoffScope,
    /// Archive target.
    pub archive_target_ref: ArchiveHandoffTargetRef,
}
