//! Project and backlog application services.

use core_contracts::metadata::{OperationName, RequestMetadata, Version};
use serde::Serialize;

use crate::results::CommandResultRepository;
use crate::{
    ApplicationError, AuditRepository, BacklogRepository, ClockPort, IdGeneratorPort,
    IdempotencyError, IdempotencyRecord, IdempotencyRepository, IdempotencyReservation, PortError,
    ProjectRepository, ProjectionRepository, RepositoryError, RequestDigest, StoredCommandResult,
    UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, WorkOutboxRepository,
};
use work_contracts::{
    ApplicationResultRef, BacklogCommandResult, BacklogRef, CreateProjectRequest,
    DerivedWorkViewRef, ProjectCommandResult, ProjectLifecycleState, ProjectLifecycleTarget,
    ProjectRef, UpdateBacklogAvailabilityRequest, UpdateProjectLifecycleRequest,
    WorkCommandEnvelope, WorkCommandReceipt, WorkTruthChange, WorkTruthCursor,
};
use work_domain::{Backlog, Project, WorkAuditTrail, WorkOutboxRecord, WorkTraceRecord};

/// Coordinates project commands inside application transaction boundaries.
pub struct ProjectCommandService<P, B, A, O, R, PR, U, I, C, IDEM> {
    /// Project repository port.
    pub project_repo: P,
    /// Backlog repository port.
    pub backlog_repo: B,
    /// Audit repository port.
    pub audit_repo: A,
    /// Outbox repository port.
    pub outbox_repo: O,
    /// Stored command result repository port.
    pub command_results: R,
    /// Projection freshness repository port.
    pub projection_repo: PR,
    /// Unit of work factory.
    pub unit_of_work: U,
    /// Id generator.
    pub ids: I,
    /// Clock port.
    pub clock: C,
    /// Idempotency repository.
    pub idempotency: IDEM,
}

impl<P, B, A, O, R, PR, U, I, C, IDEM> ProjectCommandService<P, B, A, O, R, PR, U, I, C, IDEM>
where
    P: ProjectRepository,
    B: BacklogRepository,
    A: AuditRepository,
    O: WorkOutboxRepository,
    R: CommandResultRepository,
    PR: ProjectionRepository,
    U: UnitOfWork,
    I: IdGeneratorPort,
    C: ClockPort,
    IDEM: IdempotencyRepository,
{
    /// Creates a Work-owned project and its initial backlog.
    pub async fn create_project(
        &self,
        envelope: WorkCommandEnvelope<CreateProjectRequest>,
    ) -> Result<ProjectCommandResult, ApplicationError> {
        let operation = OperationName::new("create_project");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::DuplicateProject(result) => return Ok(result),
            ReservationOutcome::DuplicateBacklog(_) => {
                return Err(ApplicationError::DuplicateResultMissing);
            }
        };

        let actor = envelope.actor.actor_ref().clone();
        let project_id = self.ids.next_project_id().map_err(Self::map_port_error)?;
        let backlog_id = self.ids.next_backlog_id().map_err(Self::map_port_error)?;
        let project = Project::create(project_id, envelope.command.project_spec, actor.clone())
            .map_err(Self::map_domain_error)?;
        let backlog = Backlog::open_for_project(backlog_id, project.project_id.clone(), actor)
            .map_err(Self::map_domain_error)?;
        let project_ref = project.project_ref();

        let project_version = self
            .project_repo
            .create(project, &uow)
            .await
            .map_err(Self::map_repository_error)?;
        let _backlog_version = self
            .backlog_repo
            .create(backlog, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let trace_id = self.ids.next_trace_id().map_err(Self::map_port_error)?;
        let trace = WorkTraceRecord::from_truth_change(
            trace_id.clone(),
            WorkTruthChange::ProjectCreated(
                project_ref.clone(),
                work_contracts::ProjectLifecycleReason::created(),
            ),
            work_contracts::WorkTraceContextRef::from_metadata(&envelope.metadata.request),
        )
        .map_err(Self::map_domain_error)?;
        self.audit_repo
            .append_trace(trace.clone(), &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let mut audit = match self
            .audit_repo
            .get_audit_trail(
                WorkTruthChange::ProjectCreated(
                    project_ref.clone(),
                    work_contracts::ProjectLifecycleReason::created(),
                )
                .audit_subject_ref(),
            )
            .await
            .map_err(Self::map_repository_error)?
        {
            Some(existing) => existing,
            None => WorkAuditTrail::start_for_subject(
                WorkTruthChange::ProjectCreated(
                    project_ref.clone(),
                    work_contracts::ProjectLifecycleReason::created(),
                )
                .audit_subject_ref(),
            ),
        };
        let expected_audit_version = if audit.has_gap() { None } else { Some(1) };
        audit.append(trace).map_err(Self::map_domain_error)?;
        let _audit_version = self
            .audit_repo
            .save_audit_trail(audit, expected_audit_version, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let outbox_id = self.ids.next_outbox_id().map_err(Self::map_port_error)?;
        let outbox = WorkOutboxRecord::from_truth_change(
            outbox_id.clone(),
            WorkTruthChange::ProjectCreated(
                project_ref.clone(),
                work_contracts::ProjectLifecycleReason::created(),
            ),
            work_contracts::WorkTraceContextRef::from_metadata(&envelope.metadata.request),
            self.clock.now().map_err(Self::map_port_error)?,
        )
        .map_err(Self::map_domain_error)?;
        self.outbox_repo
            .enqueue(outbox, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let cursor = work_contracts::WorkTruthCursor(format!(
            "project:{}:v{}",
            project_ref.project_id.0, project_version
        ));
        self.projection_repo
            .mark_stale(
                vec![DerivedWorkViewRef::project_board(project_ref.clone())],
                cursor.clone(),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation.clone(),
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = ProjectCommandResult {
            project_ref,
            lifecycle_state: ProjectLifecycleState::Active,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                project_version,
            ),
        };

        self.finish_project_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    pub async fn update_lifecycle(
        &self,
        envelope: WorkCommandEnvelope<UpdateProjectLifecycleRequest>,
    ) -> Result<ProjectCommandResult, ApplicationError> {
        let operation = OperationName::new("update_project_lifecycle");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::DuplicateProject(result) => return Ok(result),
            ReservationOutcome::DuplicateBacklog(_) => {
                return Err(ApplicationError::DuplicateResultMissing);
            }
        };
        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let project_reason = request.reason.clone();
        let mut project = self
            .project_repo
            .get(request.project_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        project
            .transition_lifecycle(request.target, request.reason, actor.clone())
            .map_err(Self::map_domain_error)?;
        let project_ref = project.project_ref();
        let project_version = self
            .project_repo
            .save(project.clone(), request.expected_version, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::ProjectLifecycleChanged(
                    project_ref.clone(),
                    project_reason.clone(),
                ),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let mut outbox_refs = vec![
            self.enqueue_outbox(
                WorkTruthChange::ProjectLifecycleChanged(
                    project_ref.clone(),
                    project_reason.clone(),
                ),
                &envelope.metadata.request,
                &uow,
            )
            .await?,
        ];
        let mut cursor = Self::project_cursor(&project_ref, project_version);

        if request.target == ProjectLifecycleTarget::Archived {
            let (mut backlog, current_backlog_version) = self
                .backlog_repo
                .get_by_project_with_version(project_ref.clone())
                .await
                .map_err(Self::map_repository_error)?
                .ok_or(ApplicationError::NotFound)?;
            backlog
                .archive_with_project(project_ref.clone(), actor)
                .map_err(Self::map_domain_error)?;
            let backlog_version = self
                .backlog_repo
                .save(backlog.clone(), current_backlog_version, &uow)
                .await
                .map_err(Self::map_repository_error)?;
            let backlog_ref = backlog.backlog_ref();
            let archive_reason = work_contracts::BacklogMaintenanceReason {
                reason_kind: work_contracts::BacklogMaintenanceReasonKind::PolicyHold,
                reason_ref: None,
            };
            self.append_trace_and_audit(
                WorkTruthChange::BacklogAvailabilityChanged(
                    backlog_ref.clone(),
                    archive_reason.clone(),
                ),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
            outbox_refs.push(
                self.enqueue_outbox(
                    WorkTruthChange::BacklogAvailabilityChanged(
                        backlog_ref.clone(),
                        archive_reason,
                    ),
                    &envelope.metadata.request,
                    &uow,
                )
                .await?,
            );
            cursor = Self::backlog_cursor(&backlog_ref, backlog_version);
        }

        self.projection_repo
            .mark_stale(
                vec![DerivedWorkViewRef::project_board(project_ref.clone())],
                cursor,
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = ProjectCommandResult {
            project_ref,
            lifecycle_state: project.lifecycle_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                outbox_refs,
                project_version,
            ),
        };

        self.finish_project_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    pub async fn update_backlog_availability(
        &self,
        envelope: WorkCommandEnvelope<UpdateBacklogAvailabilityRequest>,
    ) -> Result<BacklogCommandResult, ApplicationError> {
        let operation = OperationName::new("update_backlog_availability");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::DuplicateBacklog(result) => return Ok(result),
            ReservationOutcome::DuplicateProject(_) => {
                return Err(ApplicationError::DuplicateResultMissing);
            }
        };
        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let backlog_reason = request.reason.clone();
        let mut backlog = self
            .backlog_repo
            .get(request.backlog_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        match request.target {
            work_contracts::BacklogAvailabilityTarget::LockedForMaintenance => backlog
                .lock_for_maintenance(request.reason, actor)
                .map_err(Self::map_domain_error)?,
            work_contracts::BacklogAvailabilityTarget::Open => backlog
                .reopen_after_maintenance(request.reason, actor)
                .map_err(Self::map_domain_error)?,
        }
        let backlog_ref = backlog.backlog_ref();
        let backlog_version = self
            .backlog_repo
            .save(backlog.clone(), request.expected_version, &uow)
            .await
            .map_err(Self::map_repository_error)?;
        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::BacklogAvailabilityChanged(
                    backlog_ref.clone(),
                    backlog_reason.clone(),
                ),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let outbox_id = self
            .enqueue_outbox(
                WorkTruthChange::BacklogAvailabilityChanged(backlog_ref.clone(), backlog_reason),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        self.projection_repo
            .mark_stale(
                vec![DerivedWorkViewRef::project_board(ProjectRef {
                    project_id: backlog.project_id.clone(),
                })],
                Self::backlog_cursor(&backlog_ref, backlog_version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = BacklogCommandResult {
            backlog_ref,
            backlog_state: backlog.backlog_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                backlog_version,
            ),
        };

        self.finish_backlog_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    async fn load_duplicate_project_result(
        &self,
        operation: OperationName,
        result_ref: ApplicationResultRef,
    ) -> Result<ProjectCommandResult, ApplicationError> {
        let stored = self
            .command_results
            .get_result(result_ref)
            .await
            .map_err(Self::map_repository_error)?;
        match stored.and_then(|result| result.into_project_result(&operation)) {
            Some(result) => Ok(result.with_duplicate_receipt()),
            None => Err(ApplicationError::DuplicateResultMissing),
        }
    }

    async fn load_duplicate_backlog_result(
        &self,
        operation: OperationName,
        result_ref: ApplicationResultRef,
    ) -> Result<BacklogCommandResult, ApplicationError> {
        let stored = self
            .command_results
            .get_result(result_ref)
            .await
            .map_err(Self::map_repository_error)?;
        match stored.and_then(|result| result.into_backlog_result(&operation)) {
            Some(result) => Ok(result.with_duplicate_receipt()),
            None => Err(ApplicationError::DuplicateResultMissing),
        }
    }

    async fn reserve_command<T: Serialize>(
        &self,
        operation: &OperationName,
        envelope: &WorkCommandEnvelope<T>,
    ) -> Result<ReservationOutcome, ApplicationError> {
        let key = envelope
            .metadata
            .request
            .idempotency_key
            .clone()
            .ok_or(ApplicationError::InvalidRequest)?;
        let digest = RequestDigest::from_canonical_command_input(
            operation,
            &envelope.actor,
            &envelope.command,
        )
        .map_err(|_| ApplicationError::InvalidRequest)?;
        let uow = self
            .unit_of_work
            .begin()
            .await
            .map_err(Self::map_uow_error_begin)?;

        match self
            .idempotency
            .reserve(key, operation.clone(), digest, &uow)
            .await
        {
            Ok(IdempotencyReservation::Duplicate(result_ref)) => {
                self.unit_of_work
                    .rollback(uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                if operation.as_str().starts_with("update_backlog") {
                    return self
                        .load_duplicate_backlog_result(operation.clone(), result_ref)
                        .await
                        .map(ReservationOutcome::DuplicateBacklog);
                }
                self.load_duplicate_project_result(operation.clone(), result_ref)
                    .await
                    .map(ReservationOutcome::DuplicateProject)
            }
            Ok(IdempotencyReservation::Conflict(conflict)) => {
                self.idempotency
                    .mark_conflict(conflict, &uow)
                    .await
                    .map_err(Self::map_idempotency_error)?;
                self.unit_of_work
                    .rollback(uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                Err(ApplicationError::IdempotencyConflict)
            }
            Ok(IdempotencyReservation::Reserved(record)) => {
                Ok(ReservationOutcome::Reserved((uow, record)))
            }
            Err(error) => {
                self.unit_of_work
                    .rollback(uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                Err(Self::map_idempotency_error(error))
            }
        }
    }

    async fn append_trace_and_audit(
        &self,
        change: WorkTruthChange,
        request: &RequestMetadata,
        uow: &UnitOfWorkHandle,
    ) -> Result<work_contracts::WorkTraceId, ApplicationError> {
        let trace_id = self.ids.next_trace_id().map_err(Self::map_port_error)?;
        let trace = WorkTraceRecord::from_truth_change(
            trace_id.clone(),
            change.clone(),
            work_contracts::WorkTraceContextRef::from_metadata(request),
        )
        .map_err(Self::map_domain_error)?;
        self.audit_repo
            .append_trace(trace.clone(), uow)
            .await
            .map_err(Self::map_repository_error)?;

        let mut audit = match self
            .audit_repo
            .get_audit_trail(change.audit_subject_ref())
            .await
            .map_err(Self::map_repository_error)?
        {
            Some(existing) => existing,
            None => WorkAuditTrail::start_for_subject(change.audit_subject_ref()),
        };
        let expected_audit_version = if audit.has_gap() {
            None
        } else {
            Some(audit.record_refs.trace_ids.len() as Version)
        };
        audit.append(trace).map_err(Self::map_domain_error)?;
        self.audit_repo
            .save_audit_trail(audit, expected_audit_version, uow)
            .await
            .map_err(Self::map_repository_error)?;
        Ok(trace_id)
    }

    async fn enqueue_outbox(
        &self,
        change: WorkTruthChange,
        request: &RequestMetadata,
        uow: &UnitOfWorkHandle,
    ) -> Result<work_contracts::WorkOutboxId, ApplicationError> {
        let outbox_id = self.ids.next_outbox_id().map_err(Self::map_port_error)?;
        let outbox = WorkOutboxRecord::from_truth_change(
            outbox_id.clone(),
            change,
            work_contracts::WorkTraceContextRef::from_metadata(request),
            self.clock.now().map_err(Self::map_port_error)?,
        )
        .map_err(Self::map_domain_error)?;
        self.outbox_repo
            .enqueue(outbox, uow)
            .await
            .map_err(Self::map_repository_error)?;
        Ok(outbox_id)
    }

    async fn finish_project_command(
        &self,
        result_ref: ApplicationResultRef,
        result: ProjectCommandResult,
        reservation: IdempotencyRecord,
        uow: UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        self.command_results
            .save_result(
                result_ref.clone(),
                StoredCommandResult::Project(result),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;
        self.idempotency
            .complete(
                IdempotencyReservation::Reserved(reservation),
                result_ref,
                &uow,
            )
            .await
            .map_err(Self::map_idempotency_error)?;
        self.unit_of_work
            .commit(uow)
            .await
            .map_err(Self::map_uow_error_commit)
    }

    async fn finish_backlog_command(
        &self,
        result_ref: ApplicationResultRef,
        result: BacklogCommandResult,
        reservation: IdempotencyRecord,
        uow: UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        self.command_results
            .save_result(
                result_ref.clone(),
                StoredCommandResult::Backlog(result),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;
        self.idempotency
            .complete(
                IdempotencyReservation::Reserved(reservation),
                result_ref,
                &uow,
            )
            .await
            .map_err(Self::map_idempotency_error)?;
        self.unit_of_work
            .commit(uow)
            .await
            .map_err(Self::map_uow_error_commit)
    }

    fn project_cursor(project_ref: &ProjectRef, version: Version) -> WorkTruthCursor {
        WorkTruthCursor(format!("project:{}:v{}", project_ref.project_id.0, version))
    }

    fn backlog_cursor(backlog_ref: &BacklogRef, version: Version) -> WorkTruthCursor {
        WorkTruthCursor(format!("backlog:{}:v{}", backlog_ref.backlog_id.0, version))
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

    fn map_port_error(error: PortError) -> ApplicationError {
        match error {
            PortError::NotFound | PortError::Rejected => {
                ApplicationError::ExternalReferenceUnresolved
            }
            PortError::Unavailable | PortError::InvalidResponse => {
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

    fn map_domain_error(_error: work_domain::DomainError) -> ApplicationError {
        ApplicationError::DomainRejected
    }
}

enum ReservationOutcome {
    Reserved((UnitOfWorkHandle, IdempotencyRecord)),
    DuplicateProject(ProjectCommandResult),
    DuplicateBacklog(BacklogCommandResult),
}
