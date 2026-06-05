//! Project member application services.

use core_contracts::metadata::{OperationName, RequestMetadata, Version};
use serde::Serialize;

use crate::results::CommandResultRepository;
use crate::{
    ApplicationError, AuditRepository, ClockPort, IdGeneratorPort, IdempotencyError,
    IdempotencyRecord, IdempotencyRepository, IdempotencyReservation,
    MemberCapabilitySnapshotInput, MemberReferencePort, PortError, ProjectMemberRepository,
    ProjectRepository, ProjectionRepository, ReferenceSnapshotRepository, RepositoryError,
    RequestDigest, StoredCommandResult, UnitOfWork, UnitOfWorkError, UnitOfWorkHandle,
    WorkOutboxRepository,
};
use work_contracts::{
    ApplicationResultRef, AssignProjectMemberRequest, DerivedWorkViewRef,
    ProjectMemberCommandResult, ProjectMemberReasonKind, ProjectMemberResponsibilityState,
    ResponsibilityTarget, UpdateProjectMemberResponsibilityRequest, WorkCommandEnvelope,
    WorkCommandReceipt, WorkTruthChange, WorkTruthCursor,
};
use work_domain::{MemberCapabilitySnapshot, ProjectMember, WorkAuditTrail, WorkTraceRecord};

/// Coordinates project-member commands inside one application transaction boundary.
pub struct ProjectMemberCommandService<P, PM, RS, A, O, R, PR, U, M, I, C, IDEM> {
    /// Project repository port.
    pub project_repo: P,
    /// Project member repository port.
    pub member_repo: PM,
    /// Reference snapshot repository port.
    pub snapshot_repo: RS,
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
    /// Member resolver port.
    pub member_refs: M,
    /// Id generator.
    pub ids: I,
    /// Clock port.
    pub clock: C,
    /// Idempotency repository.
    pub idempotency: IDEM,
}

impl<P, PM, RS, A, O, R, PR, U, M, I, C, IDEM>
    ProjectMemberCommandService<P, PM, RS, A, O, R, PR, U, M, I, C, IDEM>
where
    P: ProjectRepository,
    PM: ProjectMemberRepository,
    RS: ReferenceSnapshotRepository,
    A: AuditRepository,
    O: WorkOutboxRepository,
    R: CommandResultRepository,
    PR: ProjectionRepository,
    U: UnitOfWork,
    M: MemberReferencePort,
    I: IdGeneratorPort,
    C: ClockPort,
    IDEM: IdempotencyRepository,
{
    /// Assigns a project-local member responsibility and activates it when capability checks pass.
    pub async fn assign_project_member(
        &self,
        envelope: WorkCommandEnvelope<AssignProjectMemberRequest>,
    ) -> Result<ProjectMemberCommandResult, ApplicationError> {
        let operation = OperationName::new("assign_project_member");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let project = match self.project_repo.get(request.project_ref.clone()).await {
            Ok(Some(project)) => project,
            Ok(None) => return self.rollback_and_err(uow, ApplicationError::NotFound).await,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };
        match self
            .member_repo
            .get_by_member(project.project_ref(), request.member_ref.clone())
            .await
        {
            Ok(Some(_)) => {
                return self
                    .rollback_and_err(uow, ApplicationError::DomainRejected)
                    .await;
            }
            Ok(None) => {}
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        }

        let snapshot = match self
            .load_or_resolve_member_snapshot(request.member_ref.clone(), &uow)
            .await
        {
            Ok(snapshot) => snapshot,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };
        let member_id = match self.ids.next_project_member_id() {
            Ok(member_id) => member_id,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_member_port_error(error))
                    .await;
            }
        };
        let mut member = match ProjectMember::assign(
            member_id,
            project.project_id.clone(),
            request.member_ref,
            request.responsibility_spec,
        ) {
            Ok(member) => member,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_domain_error(error))
                    .await;
            }
        };
        if let Err(error) = member.activate(snapshot, actor) {
            return self
                .rollback_and_err(uow, Self::map_domain_error(error))
                .await;
        }

        let member_ref = member.project_member_ref();
        let version = match self.member_repo.create(member, &uow).await {
            Ok(version) => version,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };
        let trace_id = match self
            .append_trace_and_audit(
                WorkTruthChange::ProjectMemberChanged(member_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await
        {
            Ok(trace_id) => trace_id,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };
        let outbox_id = match self
            .enqueue_outbox(
                WorkTruthChange::ProjectMemberChanged(member_ref.clone()),
                &uow,
            )
            .await
        {
            Ok(outbox_id) => outbox_id,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };
        if let Err(error) = self
            .projection_repo
            .mark_stale(
                vec![
                    DerivedWorkViewRef::project_board(request.project_ref),
                    DerivedWorkViewRef::member_work(member_ref.clone()),
                ],
                Self::member_cursor(&member_ref, version),
                &uow,
            )
            .await
        {
            return self
                .rollback_and_err(uow, Self::map_repository_error(error))
                .await;
        }

        let result_id = match self.ids.next_result_id() {
            Ok(result_id) => result_id,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_member_port_error(error))
                    .await;
            }
        };
        let result_ref = ApplicationResultRef::for_operation(operation, result_id);
        let result = ProjectMemberCommandResult {
            project_member_ref: member_ref,
            responsibility_state: ProjectMemberResponsibilityState::Active,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };

        if let Err(error) = self
            .finish_member_command(result_ref, result.clone(), reservation, uow)
            .await
        {
            return Err(error);
        }
        Ok(result)
    }

    /// Updates a project-member responsibility state.
    pub async fn update_project_member_responsibility(
        &self,
        envelope: WorkCommandEnvelope<UpdateProjectMemberResponsibilityRequest>,
    ) -> Result<ProjectMemberCommandResult, ApplicationError> {
        let operation = OperationName::new("update_project_member_responsibility");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let mut member = match self
            .member_repo
            .get(request.project_member_ref.clone())
            .await
        {
            Ok(Some(member)) => member,
            Ok(None) => return self.rollback_and_err(uow, ApplicationError::NotFound).await,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };

        match request.target {
            ResponsibilityTarget::Active => {
                let snapshot = match self
                    .load_or_resolve_member_snapshot(member.member_ref.clone(), &uow)
                    .await
                {
                    Ok(snapshot) => snapshot,
                    Err(error) => return self.rollback_and_err(uow, error).await,
                };
                match member.responsibility_state {
                    ProjectMemberResponsibilityState::Proposed => {
                        if let Err(error) = member.activate(snapshot, actor) {
                            return self
                                .rollback_and_err(uow, Self::map_domain_error(error))
                                .await;
                        }
                    }
                    ProjectMemberResponsibilityState::Paused => {
                        if let Err(error) = member.resume(snapshot, actor) {
                            return self
                                .rollback_and_err(uow, Self::map_domain_error(error))
                                .await;
                        }
                    }
                    ProjectMemberResponsibilityState::Active
                    | ProjectMemberResponsibilityState::Released => {
                        return self
                            .rollback_and_err(uow, ApplicationError::DomainRejected)
                            .await;
                    }
                }
            }
            ResponsibilityTarget::Paused => {
                if let Err(error) = member.pause(request.reason, actor) {
                    return self
                        .rollback_and_err(uow, Self::map_domain_error(error))
                        .await;
                }
            }
            ResponsibilityTarget::Released => {
                if request.reason.reason_kind != ProjectMemberReasonKind::Released {
                    return self
                        .rollback_and_err(uow, ApplicationError::DomainRejected)
                        .await;
                }
                if let Err(error) = member.release(request.reason, actor) {
                    return self
                        .rollback_and_err(uow, Self::map_domain_error(error))
                        .await;
                }
            }
        }

        let member_ref = member.project_member_ref();
        let project_ref = work_contracts::ProjectRef {
            project_id: member.project_id.clone(),
        };
        let version = match self
            .member_repo
            .save(member.clone(), request.expected_version, &uow)
            .await
        {
            Ok(version) => version,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };
        let trace_id = match self
            .append_trace_and_audit(
                WorkTruthChange::ProjectMemberChanged(member_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await
        {
            Ok(trace_id) => trace_id,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };
        let outbox_id = match self
            .enqueue_outbox(
                WorkTruthChange::ProjectMemberChanged(member_ref.clone()),
                &uow,
            )
            .await
        {
            Ok(outbox_id) => outbox_id,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };
        if let Err(error) = self
            .projection_repo
            .mark_stale(
                vec![
                    DerivedWorkViewRef::project_board(project_ref),
                    DerivedWorkViewRef::member_work(member_ref.clone()),
                ],
                Self::member_cursor(&member_ref, version),
                &uow,
            )
            .await
        {
            return self
                .rollback_and_err(uow, Self::map_repository_error(error))
                .await;
        }

        let result_id = match self.ids.next_result_id() {
            Ok(result_id) => result_id,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_member_port_error(error))
                    .await;
            }
        };
        let result_ref = ApplicationResultRef::for_operation(operation, result_id);
        let result = ProjectMemberCommandResult {
            project_member_ref: member_ref,
            responsibility_state: member.responsibility_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };

        if let Err(error) = self
            .finish_member_command(result_ref, result.clone(), reservation, uow)
            .await
        {
            return Err(error);
        }
        Ok(result)
    }

    async fn load_or_resolve_member_snapshot(
        &self,
        member_ref: work_contracts::GlobalMemberRef,
        uow: &UnitOfWorkHandle,
    ) -> Result<MemberCapabilitySnapshot, ApplicationError> {
        if let Some(snapshot) = self
            .snapshot_repo
            .get_member_snapshot(member_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
        {
            return Ok(snapshot);
        }

        let input = self
            .member_refs
            .resolve_member_capability(member_ref)
            .await
            .map_err(Self::map_member_port_error)?;
        let snapshot = Self::snapshot_from_input(input)?;
        self.snapshot_repo
            .save_member_snapshot(snapshot.clone(), None, uow)
            .await
            .map_err(Self::map_repository_error)?;
        Ok(snapshot)
    }

    fn snapshot_from_input(
        input: MemberCapabilitySnapshotInput,
    ) -> Result<MemberCapabilitySnapshot, ApplicationError> {
        MemberCapabilitySnapshot::from_identity(input.member_ref, input.capability_refs)
            .map_err(Self::map_domain_error)
    }

    async fn load_duplicate_member_result(
        &self,
        operation: OperationName,
        result_ref: ApplicationResultRef,
    ) -> Result<ProjectMemberCommandResult, ApplicationError> {
        let stored = self
            .command_results
            .get_result(result_ref)
            .await
            .map_err(Self::map_repository_error)?;
        match stored.and_then(|result| result.into_project_member_result(&operation)) {
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
                self.load_duplicate_member_result(operation.clone(), result_ref)
                    .await
                    .map(ReservationOutcome::Duplicate)
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
        let trace_id = self
            .ids
            .next_trace_id()
            .map_err(Self::map_member_port_error)?;
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
        uow: &UnitOfWorkHandle,
    ) -> Result<work_contracts::WorkOutboxId, ApplicationError> {
        let outbox_id = self
            .ids
            .next_outbox_id()
            .map_err(Self::map_member_port_error)?;
        let outbox = work_domain::WorkOutboxRecord::from_truth_change(outbox_id.clone(), change)
            .map_err(Self::map_domain_error)?;
        self.outbox_repo
            .enqueue(outbox, uow)
            .await
            .map_err(Self::map_repository_error)?;
        Ok(outbox_id)
    }

    async fn finish_member_command(
        &self,
        result_ref: ApplicationResultRef,
        result: ProjectMemberCommandResult,
        reservation: IdempotencyRecord,
        uow: UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        self.command_results
            .save_result(
                result_ref.clone(),
                StoredCommandResult::ProjectMember(result),
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

    async fn rollback_and_err<T>(
        &self,
        uow: UnitOfWorkHandle,
        error: ApplicationError,
    ) -> Result<T, ApplicationError> {
        self.unit_of_work
            .rollback(uow)
            .await
            .map_err(Self::map_uow_error_rollback)?;
        Err(error)
    }

    fn member_cursor(
        member_ref: &work_contracts::ProjectMemberRef,
        version: Version,
    ) -> WorkTruthCursor {
        WorkTruthCursor(format!(
            "project_member:{}:v{}",
            member_ref.project_member_id.0, version
        ))
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

    fn map_member_port_error(error: PortError) -> ApplicationError {
        match error {
            PortError::NotFound | PortError::Rejected => {
                ApplicationError::ExternalReferenceUnresolved
            }
            PortError::InvalidResponse => ApplicationError::DomainRejected,
            PortError::Unavailable => ApplicationError::TemporarilyUnavailable,
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
    Duplicate(ProjectMemberCommandResult),
}
