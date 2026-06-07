//! Operations job entrypoints for the Work bounded context.

use work_application::{
    ApplicationError, WorkArchiveHandoffService, WorkDerivedMaintenanceService,
    WorkOutboxPublishService, WorkReconciliationService, WorkReferenceRefreshService,
    WorkTraceHandoffService,
};
use work_contracts::{
    PrepareArchiveHandoffJobInput, PrepareWorkTraceHandoffJobInput, PublishWorkOutboxJobInput,
    RebuildWorkProjectionsJobInput, RefreshExternalReferenceSnapshotsJobInput,
    RunWorkReconciliationJobInput,
};
use work_infra as _;

/// Thin operations runner for Work maintenance jobs.
pub struct WorkOperationsJobRunner<OUTBOX, REBUILD, REFRESH, RECONCILE, TRACE, ARCHIVE> {
    /// Outbox publication application service.
    pub outbox_publish: OUTBOX,
    /// Projection rebuild application service.
    pub rebuild_projections: REBUILD,
    /// Reference refresh application service.
    pub refresh_references: REFRESH,
    /// Reconciliation application service.
    pub reconcile: RECONCILE,
    /// Trace handoff application service.
    pub trace_handoff: TRACE,
    /// Archive handoff application service.
    pub archive_handoff: ARCHIVE,
}

impl<OUTBOX, REBUILD, REFRESH, RECONCILE, TRACE, ARCHIVE>
    WorkOperationsJobRunner<OUTBOX, REBUILD, REFRESH, RECONCILE, TRACE, ARCHIVE>
where
    OUTBOX: Send + Sync,
    REBUILD: Send + Sync,
    REFRESH: Send + Sync,
    RECONCILE: Send + Sync,
    TRACE: Send + Sync,
    ARCHIVE: Send + Sync,
{
    /// Creates a new job runner from assembled services.
    pub fn new(
        outbox_publish: OUTBOX,
        rebuild_projections: REBUILD,
        refresh_references: REFRESH,
        reconcile: RECONCILE,
        trace_handoff: TRACE,
        archive_handoff: ARCHIVE,
    ) -> Self {
        Self {
            outbox_publish,
            rebuild_projections,
            refresh_references,
            reconcile,
            trace_handoff,
            archive_handoff,
        }
    }
}

impl<
    PJ,
    PM,
    W,
    PR,
    B,
    D,
    ITR,
    A,
    O,
    PROJ,
    PUB,
    U,
    IDEM,
    JR,
    IDS,
    TRUTH,
    PROJ2,
    U2,
    IDEM2,
    JR2,
    IDS2,
    RS,
    PROJ3,
    MEM,
    METHOD,
    SRC,
    EVID,
    PROC,
    CLOCK,
    U3,
    IDEM3,
    JR3,
    IDS3,
    TRUTH2,
    PROJ4,
    O2,
    RS2,
    U4,
    IDEM4,
    JR4,
    IDS4,
    AUDIT2,
    OUTBOX2,
    HANDOFF2,
    CLOCK2,
    U5,
    IDEM5,
    JR5,
    IDS5,
    SUMMARY,
    AUDIT3,
    OUTBOX3,
    HANDOFF3,
    CLOCK3,
    U6,
    IDEM6,
    JR6,
    IDS6,
>
    WorkOperationsJobRunner<
        WorkOutboxPublishService<PJ, PM, W, PR, B, D, ITR, A, O, PROJ, PUB, U, IDEM, JR, IDS>,
        WorkDerivedMaintenanceService<TRUTH, PROJ2, U2, IDEM2, JR2, IDS2>,
        WorkReferenceRefreshService<
            RS,
            PROJ3,
            MEM,
            METHOD,
            SRC,
            EVID,
            PROC,
            CLOCK,
            U3,
            IDEM3,
            JR3,
            IDS3,
        >,
        WorkReconciliationService<TRUTH2, PROJ4, O2, RS2, U4, IDEM4, JR4, IDS4>,
        WorkTraceHandoffService<AUDIT2, OUTBOX2, HANDOFF2, CLOCK2, U5, IDEM5, JR5, IDS5>,
        WorkArchiveHandoffService<SUMMARY, AUDIT3, OUTBOX3, HANDOFF3, CLOCK3, U6, IDEM6, JR6, IDS6>,
    >
where
    PJ: work_application::ProjectRepository,
    PM: work_application::ProjectMemberRepository,
    W: work_application::WorkItemRepository,
    PR: work_application::PromoteRepository,
    B: work_application::BacklogRepository,
    D: work_application::DependencyRepository,
    ITR: work_application::IterationRepository,
    A: work_application::AuditRepository,
    O: work_application::WorkOutboxRepository,
    PROJ: work_application::ProjectionRepository,
    PUB: work_application::WorkOutboxPublisherPort,
    U: work_application::UnitOfWork,
    IDEM: work_application::IdempotencyRepository,
    JR: work_application::JobResultRepository,
    IDS: work_application::IdGeneratorPort,
    TRUTH: work_application::WorkTruthSnapshotRepository,
    PROJ2: work_application::ProjectionRepository,
    U2: work_application::UnitOfWork,
    IDEM2: work_application::IdempotencyRepository,
    JR2: work_application::JobResultRepository,
    IDS2: work_application::IdGeneratorPort,
    RS: work_application::ReferenceSnapshotRepository,
    PROJ3: work_application::ProjectionRepository,
    MEM: work_application::MemberReferencePort,
    METHOD: work_application::MethodDefinitionResolverPort,
    SRC: work_application::SourceWorkResolverPort,
    EVID: work_application::EvidenceResolverPort,
    PROC: work_application::ProcessTimeboxResolverPort,
    CLOCK: work_application::ClockPort,
    U3: work_application::UnitOfWork,
    IDEM3: work_application::IdempotencyRepository,
    JR3: work_application::JobResultRepository,
    IDS3: work_application::IdGeneratorPort,
    TRUTH2: work_application::WorkTruthSnapshotRepository,
    PROJ4: work_application::ProjectionRepository,
    O2: work_application::WorkOutboxRepository,
    RS2: work_application::ReferenceSnapshotRepository,
    U4: work_application::UnitOfWork,
    IDEM4: work_application::IdempotencyRepository,
    JR4: work_application::JobResultRepository,
    IDS4: work_application::IdGeneratorPort,
    AUDIT2: work_application::AuditRepository,
    OUTBOX2: work_application::WorkOutboxRepository,
    HANDOFF2: work_application::TraceHandoffPort,
    CLOCK2: work_application::ClockPort,
    U5: work_application::UnitOfWork,
    IDEM5: work_application::IdempotencyRepository,
    JR5: work_application::JobResultRepository,
    IDS5: work_application::IdGeneratorPort,
    SUMMARY: work_application::ArchiveSummaryRepository,
    AUDIT3: work_application::AuditRepository,
    OUTBOX3: work_application::WorkOutboxRepository,
    HANDOFF3: work_application::ArchiveHandoffPort,
    CLOCK3: work_application::ClockPort,
    U6: work_application::UnitOfWork,
    IDEM6: work_application::IdempotencyRepository,
    JR6: work_application::JobResultRepository,
    IDS6: work_application::IdGeneratorPort,
{
    /// Runs the P0 outbox publication job.
    pub async fn run_publish_work_outbox(
        &self,
        input: PublishWorkOutboxJobInput,
    ) -> Result<work_contracts::WorkJobReport, ApplicationError> {
        self.outbox_publish.publish_outbox(input).await
    }

    /// Runs the P0 projection rebuild job.
    pub async fn run_rebuild_work_projections(
        &self,
        input: RebuildWorkProjectionsJobInput,
    ) -> Result<work_contracts::WorkJobReport, ApplicationError> {
        self.rebuild_projections
            .rebuild_work_projections(input)
            .await
    }

    /// Runs the P0 reference refresh job.
    pub async fn run_refresh_external_reference_snapshots(
        &self,
        input: RefreshExternalReferenceSnapshotsJobInput,
    ) -> Result<work_contracts::WorkJobReport, ApplicationError> {
        self.refresh_references
            .refresh_external_reference_snapshots(input)
            .await
    }

    /// Runs the P0 reconciliation job.
    pub async fn run_work_reconciliation(
        &self,
        input: RunWorkReconciliationJobInput,
    ) -> Result<work_contracts::views::ReconciliationReport, ApplicationError> {
        self.reconcile.run_work_reconciliation(input).await
    }

    /// Runs the P0 trace handoff job.
    pub async fn run_prepare_work_trace_handoff(
        &self,
        input: PrepareWorkTraceHandoffJobInput,
    ) -> Result<work_contracts::WorkJobReport, ApplicationError> {
        self.trace_handoff.prepare_work_trace_handoff(input).await
    }

    /// Runs the P0 archive handoff job.
    pub async fn run_prepare_archive_handoff(
        &self,
        input: PrepareArchiveHandoffJobInput,
    ) -> Result<work_contracts::WorkJobReport, ApplicationError> {
        self.archive_handoff.prepare_archive_handoff(input).await
    }
}
