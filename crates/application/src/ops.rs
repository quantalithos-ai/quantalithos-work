//! Operations services for rebuild, refresh, reconciliation, and handoff jobs.

use std::collections::BTreeSet;

use core_contracts::metadata::{OperationName, PageRequest, PageToken, Version};

use crate::results::{JobResultRepository, StoredJobResult};
use crate::{
    ApplicationError, ArchiveHandoffPort, ArchiveSummaryRepository, AuditRepository, ClockPort,
    IdGeneratorPort, IdempotencyError, IdempotencyRecord, IdempotencyRepository,
    IdempotencyReservation, MemberReferencePort, MethodDefinitionResolverPort, Page, PageInfo,
    PortError, ProjectionRepository, ReferenceSnapshotRepository, RepositoryError, RequestDigest,
    TraceHandoffPort, UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, WorkOutboxRepository,
    WorkTruthSnapshotRepository,
};
use work_contracts::views::{ProjectProjectionBatch, ReconciliationReport};
use work_contracts::{
    ApplicationResultRef, ArchiveHandoffScope, ArchiveHandoffScopeKind, DerivedFreshnessState,
    DerivedWorkViewRef, ExternalReferenceRef, ExternalReferenceScope, ExternalReferenceScopeKind,
    ProjectRef, RebuildWorkProjectionsJobInput, RefreshExternalReferenceSnapshotsJobInput,
    RunWorkReconciliationJobInput, WorkJobFailureRef, WorkJobMetadata, WorkJobReport,
    WorkOutboxSourceRef, WorkReconciliationScopeKind, WorkReconciliationScopeRef, WorkTruthCursor,
};
use work_domain::{
    ArchiveHandoffIntent, ArchiveHandoffMarker, MemberCapabilitySnapshot, MethodDefinitionSnapshot,
    ProjectionFailureReason, ReferenceFailureReason, ReferenceResolutionState, TraceHandoffMarker,
    WorkOutboxRecord,
};

const AFFECTED_VIEWS_PAGE_LIMIT: u32 = 100;
const ARCHIVE_HANDOFF_PAGE_LIMIT: u32 = 100;
const RECONCILIATION_PAGE_LIMIT: u32 = 100;
const TRACE_HANDOFF_PAGE_LIMIT: u32 = 100;

/// Rebuilds project-scoped derived views from committed truth summaries.
pub struct WorkDerivedMaintenanceService<TRUTH, PROJ, U, IDEM, JR, IDS> {
    /// Truth snapshot reader.
    pub truth_snapshot_repo: TRUTH,
    /// Projection repository.
    pub projection_repo: PROJ,
    /// Unit-of-work factory.
    pub unit_of_work: U,
    /// Shared idempotency repository.
    pub idempotency: IDEM,
    /// Job result repository for duplicate replay.
    pub job_results: JR,
    /// Deterministic result id generator.
    pub ids: IDS,
}

impl<TRUTH, PROJ, U, IDEM, JR, IDS> WorkDerivedMaintenanceService<TRUTH, PROJ, U, IDEM, JR, IDS>
where
    TRUTH: WorkTruthSnapshotRepository,
    PROJ: ProjectionRepository,
    U: UnitOfWork,
    IDEM: IdempotencyRepository,
    JR: JobResultRepository,
    IDS: IdGeneratorPort,
{
    /// Rebuilds selected project projections from committed truth.
    pub async fn rebuild_work_projections(
        &self,
        input: RebuildWorkProjectionsJobInput,
    ) -> Result<WorkJobReport, ApplicationError> {
        let operation = OperationName::new("rebuild_work_projections");
        let (job_uow, reservation) = match reserve_job(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            &operation,
            &input,
        )
        .await?
        {
            JobReservationOutcome::Reserved(parts) => parts,
            JobReservationOutcome::Duplicate(report) => return Ok(report),
        };

        let snapshot = match self
            .truth_snapshot_repo
            .load_project_truth_snapshot(input.project_ref.clone())
            .await
        {
            Ok(snapshot) => snapshot,
            Err(error) => {
                rollback_uow(&self.unit_of_work, job_uow).await?;
                return Err(map_repository_error(error));
            }
        };
        let cursor = match self
            .truth_snapshot_repo
            .load_truth_cursor(input.project_ref.clone())
            .await
        {
            Ok(cursor) => cursor,
            Err(error) => {
                rollback_uow(&self.unit_of_work, job_uow).await?;
                return Err(map_repository_error(error));
            }
        };

        let batch = ProjectProjectionBatch::from_truth(snapshot, input.projection_set);
        let affected = batch.view_refs();
        if let Err(error) = self
            .projection_repo
            .mark_rebuilding(affected.clone(), cursor.clone(), &job_uow)
            .await
        {
            rollback_uow(&self.unit_of_work, job_uow).await?;
            return Err(map_repository_error(error));
        }
        if let Err(error) = self
            .projection_repo
            .replace_project_views(batch, cursor.clone(), &job_uow)
            .await
        {
            let reason =
                ProjectionFailureReason::from_build_error(cursor.clone(), format!("{error:?}"));
            let _ = self
                .projection_repo
                .mark_failed(affected, cursor, reason, &job_uow)
                .await;
            rollback_uow(&self.unit_of_work, job_uow).await?;
            return Err(map_repository_error(error));
        }

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(map_port_error)?,
        );
        let report = WorkJobReport {
            job_run_id: input.metadata.job_run_id,
            receipt: Some(work_contracts::WorkCommandReceipt::applied(
                result_ref.clone(),
                None,
                Vec::new(),
                0,
            )),
            scanned_count: affected_count(&self.projection_repo, input.project_ref).await?,
            changed_count: affected.len() as u64,
            failed_refs: Vec::new(),
        };

        save_job_report_and_commit(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            reservation,
            result_ref,
            StoredJobResult::WorkJob(report.clone()),
            job_uow,
        )
        .await?;

        Ok(report)
    }
}

/// Refreshes local reference snapshots from adjacent boundaries.
pub struct WorkReferenceRefreshService<
    RS,
    PROJ,
    MEM,
    METHOD,
    SRC,
    EVID,
    PROC,
    CLOCK,
    U,
    IDEM,
    JR,
    IDS,
> {
    /// Reference snapshot repository.
    pub reference_repo: RS,
    /// Projection repository.
    pub projection_repo: PROJ,
    /// Member resolver.
    pub member_resolver: MEM,
    /// Method definition resolver.
    pub method_resolver: METHOD,
    /// Source resolver.
    pub source_resolver: SRC,
    /// Evidence resolver.
    pub evidence_resolver: EVID,
    /// Process timebox resolver.
    pub process_timebox_resolver: PROC,
    /// Clock port.
    pub clock: CLOCK,
    /// Unit-of-work factory.
    pub unit_of_work: U,
    /// Shared idempotency repository.
    pub idempotency: IDEM,
    /// Job result repository.
    pub job_results: JR,
    /// Deterministic result id generator.
    pub ids: IDS,
}

impl<RS, PROJ, MEM, METHOD, SRC, EVID, PROC, CLOCK, U, IDEM, JR, IDS>
    WorkReferenceRefreshService<RS, PROJ, MEM, METHOD, SRC, EVID, PROC, CLOCK, U, IDEM, JR, IDS>
where
    RS: ReferenceSnapshotRepository,
    PROJ: ProjectionRepository,
    MEM: MemberReferencePort,
    METHOD: MethodDefinitionResolverPort,
    SRC: crate::SourceWorkResolverPort,
    EVID: crate::EvidenceResolverPort,
    PROC: crate::ProcessTimeboxResolverPort,
    CLOCK: ClockPort,
    U: UnitOfWork,
    IDEM: IdempotencyRepository,
    JR: JobResultRepository,
    IDS: IdGeneratorPort,
{
    /// Refreshes external reference snapshots for the selected scope.
    pub async fn refresh_external_reference_snapshots(
        &self,
        input: RefreshExternalReferenceSnapshotsJobInput,
    ) -> Result<WorkJobReport, ApplicationError> {
        let operation = OperationName::new("refresh_external_reference_snapshots");
        let (job_uow, reservation) = match reserve_job(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            &operation,
            &input,
        )
        .await?
        {
            JobReservationOutcome::Reserved(parts) => parts,
            JobReservationOutcome::Duplicate(report) => return Ok(report),
        };

        let refs = match self
            .load_refresh_refs(input.reference_scope.clone(), input.page.clone())
            .await
        {
            Ok(page) => page,
            Err(error) => {
                rollback_uow(&self.unit_of_work, job_uow).await?;
                return Err(error);
            }
        };

        let mut changed_refs = Vec::new();
        let mut failed_refs = Vec::new();
        for reference_ref in refs.items.clone() {
            match self
                .save_snapshot_update(reference_ref.clone(), &job_uow)
                .await
            {
                Ok(()) => changed_refs.push(reference_ref),
                Err(ApplicationError::ExternalReferenceUnresolved)
                | Err(ApplicationError::TemporarilyUnavailable)
                | Err(ApplicationError::InvalidRequest) => {
                    let expected_version = self
                        .reference_state_expected_version(reference_ref.clone())
                        .await?;
                    let reason = ReferenceFailureReason::from_resolver_error(
                        reference_ref.clone(),
                        "reference refresh failed".to_owned(),
                    );
                    self.reference_repo
                        .mark_reference_failed(
                            reference_ref.clone(),
                            reason,
                            self.clock.now().map_err(map_port_error)?,
                            expected_version,
                            &job_uow,
                        )
                        .await
                        .map_err(map_repository_error)?;
                    failed_refs.push(WorkJobFailureRef::ExternalReference(reference_ref));
                }
                Err(error) => {
                    rollback_uow(&self.unit_of_work, job_uow).await?;
                    return Err(error);
                }
            }
        }

        if !changed_refs.is_empty() {
            let affected_views = self
                .projection_repo
                .list_views_affected_by_references(changed_refs.clone(), affected_views_page())
                .await
                .map_err(map_repository_error)?;
            if !affected_views.items.is_empty() {
                self.projection_repo
                    .mark_stale(
                        affected_views.items,
                        current_cursor(&changed_refs),
                        &job_uow,
                    )
                    .await
                    .map_err(map_repository_error)?;
            }
        }

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(map_port_error)?,
        );
        let report = WorkJobReport {
            job_run_id: input.metadata.job_run_id,
            receipt: Some(work_contracts::WorkCommandReceipt::applied(
                result_ref.clone(),
                None,
                Vec::new(),
                0,
            )),
            scanned_count: refs.items.len() as u64,
            changed_count: changed_refs.len() as u64,
            failed_refs,
        };

        save_job_report_and_commit(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            reservation,
            result_ref,
            StoredJobResult::WorkJob(report.clone()),
            job_uow,
        )
        .await?;

        Ok(report)
    }

    async fn load_refresh_refs(
        &self,
        scope: Option<ExternalReferenceScope>,
        page: PageRequest,
    ) -> Result<Page<ExternalReferenceRef>, ApplicationError> {
        match scope {
            None => self
                .reference_repo
                .list_stale_references(page)
                .await
                .map_err(map_repository_error),
            Some(scope) if scope.scope_kind == ExternalReferenceScopeKind::StaleOnly => {
                if scope.project_ref.is_some() || !scope.reference_refs.is_empty() {
                    return Err(ApplicationError::InvalidRequest);
                }
                self.reference_repo
                    .list_stale_references(page)
                    .await
                    .map_err(map_repository_error)
            }
            Some(scope) if scope.scope_kind == ExternalReferenceScopeKind::Project => {
                let Some(project_ref) = scope.project_ref else {
                    return Err(ApplicationError::InvalidRequest);
                };
                if !scope.reference_refs.is_empty() {
                    return Err(ApplicationError::InvalidRequest);
                }
                self.reference_repo
                    .list_project_references(project_ref, page)
                    .await
                    .map_err(map_repository_error)
            }
            Some(scope) if scope.scope_kind == ExternalReferenceScopeKind::ExplicitRefs => {
                if scope.project_ref.is_some() || scope.reference_refs.is_empty() {
                    return Err(ApplicationError::InvalidRequest);
                }
                Ok(paginate_items(
                    stable_dedup_reference_refs(scope.reference_refs),
                    page,
                ))
            }
            Some(_) => Err(ApplicationError::InvalidRequest),
        }
    }

    async fn reference_state_expected_version(
        &self,
        reference_ref: ExternalReferenceRef,
    ) -> Result<Option<Version>, ApplicationError> {
        self.reference_repo
            .get_reference_state_with_version(reference_ref)
            .await
            .map_err(map_repository_error)
            .map(|state| state.map(|(_, version)| version))
    }

    async fn save_snapshot_update(
        &self,
        reference_ref: ExternalReferenceRef,
        job_uow: &UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        match reference_ref.clone() {
            ExternalReferenceRef::Member(member_ref) => {
                let input = self
                    .member_resolver
                    .resolve_member_capability(member_ref.clone())
                    .await
                    .map_err(map_port_error)?;
                let mut snapshot = MemberCapabilitySnapshot::from_identity(
                    input.member_ref,
                    input.capability_refs,
                )
                .map_err(|_| ApplicationError::InvalidRequest)?;
                snapshot
                    .snapshot_state
                    .mark_resolved(self.clock.now().map_err(map_port_error)?)
                    .map_err(|_| ApplicationError::InvalidRequest)?;
                let expected_member_snapshot_version = self
                    .reference_repo
                    .get_member_snapshot_with_version(member_ref.clone())
                    .await
                    .map_err(map_repository_error)?
                    .map(|(_, version)| version);
                let expected_reference_version = self
                    .reference_state_expected_version(ExternalReferenceRef::from_member(
                        member_ref.clone(),
                    ))
                    .await?;
                self.reference_repo
                    .save_member_snapshot(snapshot, expected_member_snapshot_version, job_uow)
                    .await
                    .map_err(map_repository_error)?;
                self.reference_repo
                    .save_reference_state(
                        ReferenceResolutionState {
                            reference_ref: ExternalReferenceRef::from_member(member_ref),
                            resolution_state: work_contracts::ReferenceResolutionStatus::Resolved,
                            last_resolved_at: Some(self.clock.now().map_err(map_port_error)?),
                        },
                        expected_reference_version,
                        job_uow,
                    )
                    .await
                    .map_err(map_repository_error)?;
                Ok(())
            }
            ExternalReferenceRef::MethodDefinition(definition_ref) => {
                let input = self
                    .method_resolver
                    .resolve_definition(definition_ref.clone())
                    .await
                    .map_err(map_port_error)?;
                let mut snapshot = MethodDefinitionSnapshot::from_method_library(
                    input.definition_ref,
                    input.definition_kind,
                )
                .map_err(|_| ApplicationError::InvalidRequest)?;
                snapshot
                    .snapshot_state
                    .mark_resolved(self.clock.now().map_err(map_port_error)?)
                    .map_err(|_| ApplicationError::InvalidRequest)?;
                let expected_method_snapshot_version = self
                    .reference_repo
                    .get_method_snapshot_with_version(definition_ref.clone())
                    .await
                    .map_err(map_repository_error)?
                    .map(|(_, version)| version);
                let expected_reference_version = self
                    .reference_state_expected_version(ExternalReferenceRef::from_method_definition(
                        definition_ref.clone(),
                    ))
                    .await?;
                self.reference_repo
                    .save_method_snapshot(snapshot, expected_method_snapshot_version, job_uow)
                    .await
                    .map_err(map_repository_error)?;
                self.reference_repo
                    .save_reference_state(
                        ReferenceResolutionState {
                            reference_ref: ExternalReferenceRef::from_method_definition(
                                definition_ref,
                            ),
                            resolution_state: work_contracts::ReferenceResolutionStatus::Resolved,
                            last_resolved_at: Some(self.clock.now().map_err(map_port_error)?),
                        },
                        expected_reference_version,
                        job_uow,
                    )
                    .await
                    .map_err(map_repository_error)?;
                Ok(())
            }
            ExternalReferenceRef::SourceWork(source_ref) => {
                let resolution = self
                    .source_resolver
                    .resolve_source_work(source_ref.clone())
                    .await
                    .map_err(map_port_error)?;
                let mut state = ReferenceResolutionState::resolved(
                    ExternalReferenceRef::from_source_work(resolution.source_ref),
                );
                state
                    .mark_resolved(self.clock.now().map_err(map_port_error)?)
                    .map_err(|_| ApplicationError::InvalidRequest)?;
                let expected_reference_version =
                    self.reference_state_expected_version(reference_ref).await?;
                self.reference_repo
                    .save_reference_state(state, expected_reference_version, job_uow)
                    .await
                    .map_err(map_repository_error)?;
                Ok(())
            }
            ExternalReferenceRef::Evidence(evidence_ref) => {
                let resolution = self
                    .evidence_resolver
                    .resolve_evidence(evidence_ref)
                    .await
                    .map_err(map_port_error)?;
                let mut state = resolution.reference_state;
                state
                    .mark_resolved(self.clock.now().map_err(map_port_error)?)
                    .map_err(|_| ApplicationError::InvalidRequest)?;
                let expected_reference_version =
                    self.reference_state_expected_version(reference_ref).await?;
                self.reference_repo
                    .save_reference_state(state, expected_reference_version, job_uow)
                    .await
                    .map_err(map_repository_error)?;
                Ok(())
            }
            ExternalReferenceRef::ProcessTimebox(timebox_ref) => {
                let resolution = self
                    .process_timebox_resolver
                    .resolve_timebox(timebox_ref)
                    .await
                    .map_err(map_port_error)?;
                let mut state = ReferenceResolutionState::resolved(
                    ExternalReferenceRef::from_process_timebox(resolution.timebox_ref),
                );
                state
                    .mark_resolved(self.clock.now().map_err(map_port_error)?)
                    .map_err(|_| ApplicationError::InvalidRequest)?;
                let expected_reference_version =
                    self.reference_state_expected_version(reference_ref).await?;
                self.reference_repo
                    .save_reference_state(state, expected_reference_version, job_uow)
                    .await
                    .map_err(map_repository_error)?;
                Ok(())
            }
        }
    }
}

/// Prepares trace handoff markers and optional trace-available outbox records.
pub struct WorkTraceHandoffService<AUDIT, OUTBOX, HANDOFF, CLOCK, U, IDEM, JR, IDS> {
    /// Audit repository.
    pub audit_repo: AUDIT,
    /// Work outbox repository.
    pub outbox_repo: OUTBOX,
    /// Runtime trace handoff seam.
    pub trace_handoff: HANDOFF,
    /// Clock port.
    pub clock: CLOCK,
    /// Unit-of-work factory.
    pub unit_of_work: U,
    /// Shared idempotency repository.
    pub idempotency: IDEM,
    /// Job result repository.
    pub job_results: JR,
    /// Deterministic result id generator.
    pub ids: IDS,
}

impl<AUDIT, OUTBOX, HANDOFF, CLOCK, U, IDEM, JR, IDS>
    WorkTraceHandoffService<AUDIT, OUTBOX, HANDOFF, CLOCK, U, IDEM, JR, IDS>
where
    AUDIT: AuditRepository,
    OUTBOX: WorkOutboxRepository,
    HANDOFF: TraceHandoffPort,
    CLOCK: ClockPort,
    U: UnitOfWork,
    IDEM: IdempotencyRepository,
    JR: JobResultRepository,
    IDS: IdGeneratorPort,
{
    /// Prepares trace handoff markers for one subject page.
    pub async fn prepare_work_trace_handoff(
        &self,
        input: work_contracts::PrepareWorkTraceHandoffJobInput,
    ) -> Result<WorkJobReport, ApplicationError> {
        let operation = OperationName::new("prepare_work_trace_handoff");
        let (job_uow, reservation) = match reserve_job(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            &operation,
            &input,
        )
        .await?
        {
            JobReservationOutcome::Reserved(parts) => parts,
            JobReservationOutcome::Duplicate(report) => return Ok(report),
        };

        let records = self
            .audit_repo
            .list_trace_records(input.subject_ref.clone(), trace_handoff_page())
            .await
            .map_err(map_repository_error)?;
        let mut changed_count = 0_u64;
        let mut failed_refs = Vec::new();

        for record in records.items.clone() {
            let intent = record
                .prepare_handoff(input.target_ref.clone())
                .map_err(|_| ApplicationError::InvalidRequest)?;
            match self.trace_handoff.prepare_trace_handoff(intent).await {
                Ok(handoff_ref) => {
                    let marker = TraceHandoffMarker::from_trace(
                        record.trace_id.clone(),
                        handoff_ref.clone(),
                    )
                    .map_err(|_| ApplicationError::InvalidRequest)?;
                    self.audit_repo
                        .save_trace_handoff_marker(marker, &job_uow)
                        .await
                        .map_err(map_repository_error)?;
                    let outbox = WorkOutboxRecord::from_event_source(
                        self.ids.next_outbox_id().map_err(map_port_error)?,
                        WorkOutboxSourceRef::TraceAvailable {
                            trace_id: record.trace_id.clone(),
                            handoff_ref: Some(handoff_ref),
                        },
                        record.trace_context_ref.clone(),
                        self.clock.now().map_err(map_port_error)?,
                    )
                    .map_err(|_| ApplicationError::InvalidRequest)?;
                    self.outbox_repo
                        .enqueue(outbox, &job_uow)
                        .await
                        .map_err(map_repository_error)?;
                    changed_count += 1;
                }
                Err(PortError::Unavailable)
                | Err(PortError::NotFound)
                | Err(PortError::Rejected)
                | Err(PortError::InvalidResponse) => {
                    failed_refs.push(WorkJobFailureRef::TraceHandoff {
                        trace_id: record.trace_id,
                        subject_ref: input.subject_ref.clone(),
                        target_ref: input.target_ref.clone(),
                    });
                }
            }
        }

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(map_port_error)?,
        );
        let report = WorkJobReport {
            job_run_id: input.metadata.job_run_id,
            receipt: Some(work_contracts::WorkCommandReceipt::applied(
                result_ref.clone(),
                None,
                Vec::new(),
                0,
            )),
            scanned_count: records.items.len() as u64,
            changed_count,
            failed_refs,
        };

        save_job_report_and_commit(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            reservation,
            result_ref,
            StoredJobResult::WorkJob(report.clone()),
            job_uow,
        )
        .await?;

        Ok(report)
    }
}

/// Prepares archive handoff markers and optional archive trace-available outbox records.
pub struct WorkArchiveHandoffService<SUMMARY, AUDIT, OUTBOX, HANDOFF, CLOCK, U, IDEM, JR, IDS> {
    /// Archive summary repository.
    pub archive_summary_repo: SUMMARY,
    /// Audit repository.
    pub audit_repo: AUDIT,
    /// Work outbox repository.
    pub outbox_repo: OUTBOX,
    /// Runtime archive handoff seam.
    pub archive_handoff: HANDOFF,
    /// Clock port.
    pub clock: CLOCK,
    /// Unit-of-work factory.
    pub unit_of_work: U,
    /// Shared idempotency repository.
    pub idempotency: IDEM,
    /// Job result repository.
    pub job_results: JR,
    /// Deterministic result id generator.
    pub ids: IDS,
}

impl<SUMMARY, AUDIT, OUTBOX, HANDOFF, CLOCK, U, IDEM, JR, IDS>
    WorkArchiveHandoffService<SUMMARY, AUDIT, OUTBOX, HANDOFF, CLOCK, U, IDEM, JR, IDS>
where
    SUMMARY: ArchiveSummaryRepository,
    AUDIT: AuditRepository,
    OUTBOX: WorkOutboxRepository,
    HANDOFF: ArchiveHandoffPort,
    CLOCK: ClockPort,
    U: UnitOfWork,
    IDEM: IdempotencyRepository,
    JR: JobResultRepository,
    IDS: IdGeneratorPort,
{
    /// Prepares one archive handoff marker from scoped Work summaries.
    pub async fn prepare_archive_handoff(
        &self,
        input: work_contracts::PrepareArchiveHandoffJobInput,
    ) -> Result<WorkJobReport, ApplicationError> {
        let operation = OperationName::new("prepare_archive_handoff");
        let (job_uow, reservation) = match reserve_job(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            &operation,
            &input,
        )
        .await?
        {
            JobReservationOutcome::Reserved(parts) => parts,
            JobReservationOutcome::Duplicate(report) => return Ok(report),
        };

        let summaries = self
            .load_archive_scope_summaries(input.archive_scope.clone())
            .await?;
        let intent = ArchiveHandoffIntent::from_work_summaries(
            summaries.clone(),
            input.archive_target_ref.clone(),
        )
        .map_err(|_| ApplicationError::InvalidRequest)?;

        let report = match self.archive_handoff.prepare_archive_handoff(intent).await {
            Ok(archive_ref) => {
                let marker = ArchiveHandoffMarker::from_archive_ref(
                    input.archive_scope.clone(),
                    archive_ref,
                )
                .map_err(|_| ApplicationError::InvalidRequest)?;
                self.audit_repo
                    .save_archive_handoff_marker(marker, &job_uow)
                    .await
                    .map_err(map_repository_error)?;

                WorkJobReport {
                    job_run_id: input.metadata.job_run_id,
                    receipt: Some(work_contracts::WorkCommandReceipt::applied(
                        ApplicationResultRef::for_operation(
                            operation.clone(),
                            self.ids.next_result_id().map_err(map_port_error)?,
                        ),
                        None,
                        Vec::new(),
                        0,
                    )),
                    scanned_count: summaries.truth_refs.len() as u64,
                    changed_count: 1,
                    failed_refs: Vec::new(),
                }
            }
            Err(PortError::Unavailable)
            | Err(PortError::NotFound)
            | Err(PortError::Rejected)
            | Err(PortError::InvalidResponse) => WorkJobReport {
                job_run_id: input.metadata.job_run_id,
                receipt: Some(work_contracts::WorkCommandReceipt::applied(
                    ApplicationResultRef::for_operation(
                        operation.clone(),
                        self.ids.next_result_id().map_err(map_port_error)?,
                    ),
                    None,
                    Vec::new(),
                    0,
                )),
                scanned_count: summaries.truth_refs.len() as u64,
                changed_count: 0,
                failed_refs: vec![WorkJobFailureRef::ArchiveHandoff {
                    archive_scope: input.archive_scope.clone(),
                    target_ref: input.archive_target_ref.clone(),
                }],
            },
        };

        let result_ref = report
            .receipt
            .as_ref()
            .expect("job report receipt must exist")
            .result_ref
            .clone();
        save_job_report_and_commit(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            reservation,
            result_ref,
            StoredJobResult::WorkJob(report.clone()),
            job_uow,
        )
        .await?;

        Ok(report)
    }

    async fn load_archive_scope_summaries(
        &self,
        archive_scope: ArchiveHandoffScope,
    ) -> Result<work_domain::WorkArchiveSummarySet, ApplicationError> {
        match archive_scope.scope_kind {
            ArchiveHandoffScopeKind::Subjects => {
                if archive_scope.project_ref.is_some() || archive_scope.subject_refs.is_empty() {
                    return Err(ApplicationError::InvalidRequest);
                }
                self.archive_summary_repo
                    .load_subject_archive_summaries(
                        archive_scope.subject_refs,
                        archive_scope.source_cursor,
                        archive_handoff_page(),
                    )
                    .await
                    .map_err(map_repository_error)
            }
            ArchiveHandoffScopeKind::ProjectCursor => {
                let Some(project_ref) = archive_scope.project_ref else {
                    return Err(ApplicationError::InvalidRequest);
                };
                let Some(source_cursor) = archive_scope.source_cursor else {
                    return Err(ApplicationError::InvalidRequest);
                };
                if !archive_scope.subject_refs.is_empty() {
                    return Err(ApplicationError::InvalidRequest);
                }
                self.archive_summary_repo
                    .load_project_archive_summaries(
                        project_ref,
                        source_cursor,
                        archive_handoff_page(),
                    )
                    .await
                    .map_err(map_repository_error)
            }
        }
    }
}

/// Produces read-only reconciliation reports over truth, projections, outbox, and references.
pub struct WorkReconciliationService<TRUTH, PROJ, OUTBOX, RS, U, IDEM, JR, IDS> {
    /// Truth snapshot reader.
    pub truth_snapshot_repo: TRUTH,
    /// Projection repository.
    pub projection_repo: PROJ,
    /// Outbox repository.
    pub outbox_repo: OUTBOX,
    /// Reference snapshot repository.
    pub reference_repo: RS,
    /// Unit-of-work factory.
    pub unit_of_work: U,
    /// Shared idempotency repository.
    pub idempotency: IDEM,
    /// Job result repository.
    pub job_results: JR,
    /// Deterministic result id generator.
    pub ids: IDS,
}

impl<TRUTH, PROJ, OUTBOX, RS, U, IDEM, JR, IDS>
    WorkReconciliationService<TRUTH, PROJ, OUTBOX, RS, U, IDEM, JR, IDS>
where
    TRUTH: WorkTruthSnapshotRepository,
    PROJ: ProjectionRepository,
    OUTBOX: WorkOutboxRepository,
    RS: ReferenceSnapshotRepository,
    U: UnitOfWork,
    IDEM: IdempotencyRepository,
    JR: JobResultRepository,
    IDS: IdGeneratorPort,
{
    /// Runs read-only reconciliation for the selected scope.
    pub async fn run_work_reconciliation(
        &self,
        input: RunWorkReconciliationJobInput,
    ) -> Result<ReconciliationReport, ApplicationError> {
        let operation = OperationName::new("run_work_reconciliation");
        let (job_uow, reservation) = match reserve_reconciliation_job(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            &operation,
            &input,
        )
        .await?
        {
            ReconciliationReservationOutcome::Reserved(parts) => parts,
            ReconciliationReservationOutcome::Duplicate(report) => return Ok(report),
        };

        let project_ref = project_ref_from_scope(&input.scope_ref)?;
        let truth_cursor = self
            .truth_snapshot_repo
            .load_truth_cursor(project_ref)
            .await
            .map_err(map_repository_error)?;
        let projection_states = self
            .projection_repo
            .list_freshness_states(input.scope_ref.clone(), reconciliation_page())
            .await
            .map_err(map_repository_error)?;
        let outbox_gaps = self
            .outbox_repo
            .list_pending(reconciliation_page())
            .await
            .map_err(map_repository_error)?;
        let reference_gaps = self
            .reference_repo
            .list_stale_references(reconciliation_page())
            .await
            .map_err(map_repository_error)?;

        let report = ReconciliationReport {
            scope_ref: input.scope_ref,
            truth_cursor,
            projection_gaps: projection_states
                .items
                .into_iter()
                .filter(|state| state.freshness_state != DerivedFreshnessState::Fresh)
                .map(|state| state.view_ref)
                .collect(),
            outbox_gaps: outbox_gaps
                .items
                .into_iter()
                .map(|versioned| versioned.record.outbox_id)
                .collect(),
            reference_gaps: reference_gaps.items,
        };

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(map_port_error)?,
        );
        save_job_report_and_commit(
            &self.unit_of_work,
            &self.idempotency,
            &self.job_results,
            reservation,
            result_ref,
            StoredJobResult::Reconciliation(report.clone()),
            job_uow,
        )
        .await?;

        Ok(report)
    }
}

async fn reserve_job<T, U, IDEM, JR>(
    unit_of_work: &U,
    idempotency: &IDEM,
    job_results: &JR,
    operation: &OperationName,
    input: &T,
) -> Result<JobReservationOutcome, ApplicationError>
where
    T: serde::Serialize,
    U: UnitOfWork,
    IDEM: IdempotencyRepository,
    JR: JobResultRepository,
{
    let job_uow = unit_of_work.begin().await.map_err(map_uow_error_begin)?;
    let metadata = extract_job_metadata(input)?;
    let idempotency_key = metadata
        .command_metadata
        .request
        .idempotency_key
        .clone()
        .ok_or(ApplicationError::InvalidRequest)?;
    let request_digest =
        RequestDigest::from_canonical_command_input(operation, &metadata.actor, input)
            .map_err(|_| ApplicationError::InvalidRequest)?;
    match idempotency
        .reserve(idempotency_key, operation.clone(), request_digest, &job_uow)
        .await
    {
        Ok(IdempotencyReservation::Reserved(record)) => {
            Ok(JobReservationOutcome::Reserved((job_uow, record)))
        }
        Ok(IdempotencyReservation::Duplicate(result_ref)) => {
            rollback_uow(unit_of_work, job_uow).await?;
            let stored = job_results
                .get_report(result_ref)
                .await
                .map_err(map_repository_error)?;
            let Some(report) = stored.and_then(|value| value.into_work_job_report(operation))
            else {
                return Err(ApplicationError::DuplicateResultMissing);
            };
            Ok(JobReservationOutcome::Duplicate(
                report.with_duplicate_receipt(),
            ))
        }
        Ok(IdempotencyReservation::Conflict(_)) => {
            rollback_uow(unit_of_work, job_uow).await?;
            Err(ApplicationError::IdempotencyConflict)
        }
        Err(error) => {
            rollback_uow(unit_of_work, job_uow).await?;
            Err(map_idempotency_error(error))
        }
    }
}

async fn reserve_reconciliation_job<T, U, IDEM, JR>(
    unit_of_work: &U,
    idempotency: &IDEM,
    job_results: &JR,
    operation: &OperationName,
    input: &T,
) -> Result<ReconciliationReservationOutcome, ApplicationError>
where
    T: serde::Serialize,
    U: UnitOfWork,
    IDEM: IdempotencyRepository,
    JR: JobResultRepository,
{
    let job_uow = unit_of_work.begin().await.map_err(map_uow_error_begin)?;
    let metadata = extract_job_metadata(input)?;
    let idempotency_key = metadata
        .command_metadata
        .request
        .idempotency_key
        .clone()
        .ok_or(ApplicationError::InvalidRequest)?;
    let request_digest =
        RequestDigest::from_canonical_command_input(operation, &metadata.actor, input)
            .map_err(|_| ApplicationError::InvalidRequest)?;
    match idempotency
        .reserve(idempotency_key, operation.clone(), request_digest, &job_uow)
        .await
    {
        Ok(IdempotencyReservation::Reserved(record)) => Ok(
            ReconciliationReservationOutcome::Reserved((job_uow, record)),
        ),
        Ok(IdempotencyReservation::Duplicate(result_ref)) => {
            rollback_uow(unit_of_work, job_uow).await?;
            let stored = job_results
                .get_report(result_ref)
                .await
                .map_err(map_repository_error)?;
            let Some(report) = stored.and_then(|value| value.into_reconciliation_report(operation))
            else {
                return Err(ApplicationError::DuplicateResultMissing);
            };
            Ok(ReconciliationReservationOutcome::Duplicate(report))
        }
        Ok(IdempotencyReservation::Conflict(_)) => {
            rollback_uow(unit_of_work, job_uow).await?;
            Err(ApplicationError::IdempotencyConflict)
        }
        Err(error) => {
            rollback_uow(unit_of_work, job_uow).await?;
            Err(map_idempotency_error(error))
        }
    }
}

async fn save_job_report_and_commit<U, IDEM, JR>(
    unit_of_work: &U,
    idempotency: &IDEM,
    job_results: &JR,
    reservation: IdempotencyRecord,
    result_ref: ApplicationResultRef,
    report: StoredJobResult,
    job_uow: UnitOfWorkHandle,
) -> Result<(), ApplicationError>
where
    U: UnitOfWork,
    IDEM: IdempotencyRepository,
    JR: JobResultRepository,
{
    job_results
        .save_report(result_ref.clone(), report, &job_uow)
        .await
        .map_err(map_repository_error)?;
    idempotency
        .complete(
            IdempotencyReservation::Reserved(reservation),
            result_ref,
            &job_uow,
        )
        .await
        .map_err(map_idempotency_error)?;
    unit_of_work
        .commit(job_uow)
        .await
        .map_err(map_uow_error_commit)?;
    Ok(())
}

async fn rollback_uow<U>(
    unit_of_work: &U,
    job_uow: UnitOfWorkHandle,
) -> Result<(), ApplicationError>
where
    U: UnitOfWork,
{
    unit_of_work
        .rollback(job_uow)
        .await
        .map_err(map_uow_error_rollback)
}

fn extract_job_metadata<T: serde::Serialize>(
    input: &T,
) -> Result<WorkJobMetadata, ApplicationError> {
    let value = serde_json::to_value(input).map_err(|_| ApplicationError::InvalidRequest)?;
    let metadata = value
        .get("metadata")
        .cloned()
        .ok_or(ApplicationError::InvalidRequest)?;
    serde_json::from_value(metadata).map_err(|_| ApplicationError::InvalidRequest)
}

fn map_repository_error(error: RepositoryError) -> ApplicationError {
    match error {
        RepositoryError::NotFound => ApplicationError::NotFound,
        RepositoryError::VersionConflict => ApplicationError::VersionConflict,
        RepositoryError::TransactionRejected | RepositoryError::StoreUnavailable => {
            ApplicationError::TemporarilyUnavailable
        }
    }
}

fn map_idempotency_error(error: IdempotencyError) -> ApplicationError {
    match error {
        IdempotencyError::AlreadyReserved | IdempotencyError::StoreUnavailable => {
            ApplicationError::TemporarilyUnavailable
        }
        IdempotencyError::Conflict => ApplicationError::IdempotencyConflict,
    }
}

fn map_port_error(error: PortError) -> ApplicationError {
    match error {
        PortError::Unavailable => ApplicationError::TemporarilyUnavailable,
        PortError::NotFound | PortError::Rejected => ApplicationError::ExternalReferenceUnresolved,
        PortError::InvalidResponse => ApplicationError::InvalidRequest,
    }
}

fn map_uow_error_begin(_error: UnitOfWorkError) -> ApplicationError {
    ApplicationError::TemporarilyUnavailable
}

fn map_uow_error_commit(error: UnitOfWorkError) -> ApplicationError {
    match error {
        UnitOfWorkError::CommitFailed => ApplicationError::CommitStatusUnknown,
        UnitOfWorkError::BeginFailed | UnitOfWorkError::RollbackFailed => {
            ApplicationError::TemporarilyUnavailable
        }
    }
}

fn map_uow_error_rollback(_error: UnitOfWorkError) -> ApplicationError {
    ApplicationError::TemporarilyUnavailable
}

fn affected_views_page() -> PageRequest {
    PageRequest {
        limit: AFFECTED_VIEWS_PAGE_LIMIT,
        page_token: None,
    }
}

fn reconciliation_page() -> PageRequest {
    PageRequest {
        limit: RECONCILIATION_PAGE_LIMIT,
        page_token: None,
    }
}

fn trace_handoff_page() -> PageRequest {
    PageRequest {
        limit: TRACE_HANDOFF_PAGE_LIMIT,
        page_token: None,
    }
}

fn archive_handoff_page() -> PageRequest {
    PageRequest {
        limit: ARCHIVE_HANDOFF_PAGE_LIMIT,
        page_token: None,
    }
}

fn current_cursor(reference_refs: &[ExternalReferenceRef]) -> WorkTruthCursor {
    let mut keys = reference_refs
        .iter()
        .map(reference_sort_key)
        .collect::<Vec<_>>();
    keys.sort();
    WorkTruthCursor(keys.join("|"))
}

fn affected_count<PROJ>(
    _projection_repo: &PROJ,
    _project_ref: ProjectRef,
) -> impl std::future::Future<Output = Result<u64, ApplicationError>>
where
    PROJ: ProjectionRepository,
{
    std::future::ready(Ok(0))
}

fn stable_dedup_reference_refs(items: Vec<ExternalReferenceRef>) -> Vec<ExternalReferenceRef> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for item in items {
        let key = reference_sort_key(&item);
        if seen.insert(key) {
            deduped.push(item);
        }
    }
    deduped
}

fn paginate_items<T: Clone>(items: Vec<T>, page: PageRequest) -> Page<T> {
    let start = page
        .page_token
        .as_ref()
        .and_then(|token| token.as_str().parse::<usize>().ok())
        .unwrap_or(0);
    let limit = usize::try_from(page.limit.max(1)).unwrap_or(usize::MAX);
    let page_items = items
        .iter()
        .skip(start)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let next = start + page_items.len();
    let has_more = next < items.len();

    Page {
        items: page_items,
        page_info: PageInfo {
            next_page_token: has_more.then(|| PageToken::new(next.to_string())),
            has_more,
        },
    }
}

fn reference_sort_key(reference_ref: &ExternalReferenceRef) -> String {
    match reference_ref {
        ExternalReferenceRef::Member(member_ref) => format!("0:{}", member_ref.0),
        ExternalReferenceRef::MethodDefinition(definition_ref) => format!("1:{}", definition_ref.0),
        ExternalReferenceRef::SourceWork(source_ref) => {
            format!(
                "2:{}:{}",
                source_ref.source_kind as u8, source_ref.external_ref.external_id
            )
        }
        ExternalReferenceRef::Evidence(evidence_ref) => format!(
            "3:{}:{}",
            evidence_ref.evidence_kind as u8, evidence_ref.external_ref.external_id
        ),
        ExternalReferenceRef::ProcessTimebox(timebox_ref) => format!("4:{}", timebox_ref.0),
    }
}

fn project_ref_from_scope(
    scope_ref: &WorkReconciliationScopeRef,
) -> Result<ProjectRef, ApplicationError> {
    match scope_ref.scope_kind {
        WorkReconciliationScopeKind::All | WorkReconciliationScopeKind::Project => scope_ref
            .project_ref
            .clone()
            .ok_or(ApplicationError::InvalidRequest),
        WorkReconciliationScopeKind::DerivedView => match scope_ref.view_ref.as_ref() {
            Some(DerivedWorkViewRef {
                scope_ref: work_contracts::DerivedWorkViewScopeRef::Project(project_ref),
                ..
            })
            | Some(DerivedWorkViewRef {
                scope_ref: work_contracts::DerivedWorkViewScopeRef::Search(project_ref, _),
                ..
            }) => Ok(project_ref.clone()),
            _ => scope_ref
                .project_ref
                .clone()
                .ok_or(ApplicationError::InvalidRequest),
        },
        WorkReconciliationScopeKind::ExternalReference => scope_ref
            .project_ref
            .clone()
            .ok_or(ApplicationError::InvalidRequest),
    }
}

enum JobReservationOutcome {
    Reserved((UnitOfWorkHandle, IdempotencyRecord)),
    Duplicate(WorkJobReport),
}

enum ReconciliationReservationOutcome {
    Reserved((UnitOfWorkHandle, IdempotencyRecord)),
    Duplicate(ReconciliationReport),
}
