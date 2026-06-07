//! Iteration and commitment application services.

use core_contracts::metadata::{OperationName, RequestMetadata, Version};
use serde::Serialize;

use crate::results::CommandResultRepository;
use crate::{
    ApplicationError, AuditRepository, BacklogRepository, ClockPort, FormalWorkRecord,
    IdGeneratorPort, IdempotencyError, IdempotencyRecord, IdempotencyRepository,
    IdempotencyReservation, IterationRepository, PortError, ProcessTimeboxResolverPort,
    ProjectRepository, ProjectionRepository, RepositoryError, RequestDigest, StoredCommandResult,
    UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, WorkItemRepository, WorkOutboxRepository,
};
use work_contracts::{
    ApplicationResultRef, DerivedWorkViewRef, FormalWorkRef, FormalWorkRefSet,
    IterationChangeReason, IterationCloseReason, IterationCommandResult, IterationLifecycleTarget,
    IterationRef, OpenIterationRequest, ProcessTimeboxSummary, ProjectLifecycleState, ProjectRef,
    UpdateIterationCommitmentRequest, UpdateIterationLifecycleRequest, WorkCommandEnvelope,
    WorkCommandReceipt, WorkTruthChange, WorkTruthCursor,
};
use work_domain::{
    DomainError, Iteration, IterationChangeRecord, IterationCommitment, IterationCommitmentPolicy,
    WorkAuditTrail, WorkOutboxRecord, WorkTraceRecord,
};

/// Coordinates iteration commands inside application transaction boundaries.
pub struct IterationCommandService<P, B, W, ITR, A, O, R, PR, U, PT, I, C, IDEM> {
    /// Project repository port.
    pub project_repo: P,
    /// Backlog repository port.
    pub backlog_repo: B,
    /// Work repository port.
    pub work_repo: W,
    /// Iteration repository port.
    pub iteration_repo: ITR,
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
    /// Process timebox resolver port.
    pub timebox_resolver: PT,
    /// Id generator.
    pub ids: I,
    /// Clock port.
    pub clock: C,
    /// Idempotency repository.
    pub idempotency: IDEM,
}

impl<P, B, W, ITR, A, O, R, PR, U, PT, I, C, IDEM>
    IterationCommandService<P, B, W, ITR, A, O, R, PR, U, PT, I, C, IDEM>
where
    P: ProjectRepository,
    B: BacklogRepository,
    W: WorkItemRepository,
    ITR: IterationRepository,
    A: AuditRepository,
    O: WorkOutboxRepository,
    R: CommandResultRepository,
    PR: ProjectionRepository,
    U: UnitOfWork,
    PT: ProcessTimeboxResolverPort,
    I: IdGeneratorPort,
    C: ClockPort,
    IDEM: IdempotencyRepository,
{
    /// Opens a planning iteration for one project and process timebox.
    pub async fn open_iteration(
        &self,
        envelope: WorkCommandEnvelope<OpenIterationRequest>,
    ) -> Result<IterationCommandResult, ApplicationError> {
        let operation = OperationName::new("open_iteration");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let project = self
            .project_repo
            .get(request.project_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        if project.lifecycle_state != ProjectLifecycleState::Active {
            return self
                .rollback_and_err(uow, ApplicationError::DomainRejected)
                .await;
        }

        let resolution = self
            .timebox_resolver
            .resolve_timebox(request.timebox_ref.clone())
            .await
            .map_err(Self::map_port_error)?;
        Self::ensure_timebox_can_bind_to_project(&resolution.summary, request.project_ref.clone())?;

        let iteration_id = self.ids.next_iteration_id().map_err(Self::map_port_error)?;
        let iteration = Iteration::open(
            iteration_id,
            request.project_ref.project_id.clone(),
            request.timebox_ref,
            actor,
        )
        .map_err(Self::map_domain_error)?;
        let iteration_ref = iteration.iteration_ref();
        let iteration_state = iteration.iteration_state;
        let version = self
            .iteration_repo
            .create_iteration(iteration, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::IterationChanged(iteration_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let outbox_id = self
            .enqueue_outbox(
                WorkTruthChange::IterationChanged(iteration_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        self.projection_repo
            .mark_stale(
                vec![
                    DerivedWorkViewRef::project_board(request.project_ref),
                    DerivedWorkViewRef::iteration_summary(iteration_ref.clone()),
                ],
                Self::iteration_cursor(&iteration_ref, version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = IterationCommandResult {
            iteration_ref,
            iteration_state,
            commitment_state: None,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_iteration_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    /// Commits a formal work scope into an iteration.
    pub async fn commit_iteration_scope(
        &self,
        envelope: WorkCommandEnvelope<work_contracts::CommitIterationScopeRequest>,
    ) -> Result<IterationCommandResult, ApplicationError> {
        let operation = OperationName::new("commit_iteration_scope");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let mut iteration = self
            .iteration_repo
            .get_iteration(request.iteration_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let project_ref = ProjectRef {
            project_id: iteration.project_id.clone(),
        };
        let backlog = self
            .backlog_repo
            .get_by_project(project_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let candidates = self
            .load_commit_candidates(backlog.backlog_ref(), &request.candidate_work_refs.refs)
            .await?;
        IterationCommitmentPolicy::assert_commitment_allowed(
            &iteration,
            FormalWorkRefSet {
                refs: request.candidate_work_refs.refs.clone(),
            },
        )
        .map_err(Self::map_domain_error)?;

        let commitment_id = self
            .ids
            .next_iteration_commitment_id()
            .map_err(Self::map_port_error)?;
        let mut commitment = IterationCommitment::from_candidates(
            commitment_id,
            iteration.iteration_id.clone(),
            FormalWorkRefSet {
                refs: request.candidate_work_refs.refs.clone(),
            },
            actor.clone(),
        )
        .map_err(Self::map_domain_error)?;
        iteration
            .commit(&mut commitment, actor.clone())
            .map_err(Self::map_domain_error)?;

        let iteration_state = iteration.iteration_state;
        let commitment_state = Some(commitment.commitment_state);
        let iteration_version = self
            .iteration_repo
            .save_iteration(iteration.clone(), request.expected_iteration_version, &uow)
            .await
            .map_err(Self::map_repository_error)?;
        self.iteration_repo
            .save_commitment(commitment.clone(), None, &uow)
            .await
            .map_err(Self::map_repository_error)?;
        self.mark_candidate_work_committed(candidates, request.iteration_ref.clone(), actor, &uow)
            .await?;
        let change_id = self
            .ids
            .next_iteration_change_id()
            .map_err(Self::map_port_error)?;
        let history = IterationChangeRecord::from_commitment(
            change_id,
            iteration,
            commitment,
            envelope.actor.actor_ref().clone(),
        )
        .map_err(Self::map_domain_error)?;
        self.iteration_repo
            .append_change(history, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::IterationChanged(request.iteration_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let outbox_id = self
            .enqueue_outbox(
                WorkTruthChange::IterationChanged(request.iteration_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        self.projection_repo
            .mark_stale(
                Self::affected_views(
                    project_ref,
                    self.affected_assignees(&request.candidate_work_refs.refs)
                        .await?,
                    request.iteration_ref.clone(),
                ),
                Self::iteration_cursor(&request.iteration_ref, iteration_version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = IterationCommandResult {
            iteration_ref: request.iteration_ref,
            iteration_state,
            commitment_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                iteration_version,
            ),
        };
        self.finish_iteration_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    /// Updates one iteration commitment.
    pub async fn update_iteration_commitment(
        &self,
        envelope: WorkCommandEnvelope<UpdateIterationCommitmentRequest>,
    ) -> Result<IterationCommandResult, ApplicationError> {
        let operation = OperationName::new("update_iteration_commitment");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        if request.change_set.add_work_refs.is_empty()
            && request.change_set.remove_work_refs.is_empty()
        {
            return self
                .rollback_and_err(uow, ApplicationError::InvalidRequest)
                .await;
        }

        let iteration = self
            .iteration_repo
            .get_iteration(request.iteration_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let project_ref = ProjectRef {
            project_id: iteration.project_id.clone(),
        };
        let backlog = self
            .backlog_repo
            .get_by_project(project_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let mut commitment = self
            .iteration_repo
            .get_commitment(request.iteration_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        self.validate_added_work_refs(backlog.backlog_ref(), &request.change_set.add_work_refs)
            .await?;
        commitment
            .apply_change(request.change_set.clone(), request.reason, actor)
            .map_err(Self::map_domain_error)?;

        let commitment_state = Some(commitment.commitment_state);
        let version = self
            .iteration_repo
            .save_commitment(
                commitment.clone(),
                Some(request.expected_commitment_version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;
        let change_id = self
            .ids
            .next_iteration_change_id()
            .map_err(Self::map_port_error)?;
        let history = IterationChangeRecord::from_commitment(
            change_id,
            iteration.clone(),
            commitment,
            envelope.actor.actor_ref().clone(),
        )
        .map_err(Self::map_domain_error)?;
        self.iteration_repo
            .append_change(history, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::IterationChanged(request.iteration_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let outbox_id = self
            .enqueue_outbox(
                WorkTruthChange::IterationChanged(request.iteration_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let affected_work_refs = Self::merge_work_refs(
            &request.change_set.add_work_refs,
            &request.change_set.remove_work_refs,
        );
        self.projection_repo
            .mark_stale(
                Self::affected_views(
                    project_ref,
                    self.affected_assignees(&affected_work_refs).await?,
                    request.iteration_ref.clone(),
                ),
                Self::iteration_cursor(&request.iteration_ref, version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = IterationCommandResult {
            iteration_ref: request.iteration_ref,
            iteration_state: iteration.iteration_state,
            commitment_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_iteration_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    /// Updates one iteration lifecycle.
    pub async fn update_iteration_lifecycle(
        &self,
        envelope: WorkCommandEnvelope<UpdateIterationLifecycleRequest>,
    ) -> Result<IterationCommandResult, ApplicationError> {
        let operation = OperationName::new("update_iteration_lifecycle");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let mut iteration = self
            .iteration_repo
            .get_iteration(request.iteration_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let mut commitment_state = None;
        let mut affected_work_refs = Vec::new();

        match request.target {
            IterationLifecycleTarget::InProgress => {
                let reason =
                    Self::require_change_reason(request.change_reason, request.close_reason)?;
                let commitment = self
                    .iteration_repo
                    .get_commitment(request.iteration_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::DomainRejected)?;
                commitment_state = Some(commitment.commitment_state);
                affected_work_refs = commitment.committed_work_refs.refs;
                iteration
                    .start(reason, actor.clone())
                    .map_err(Self::map_domain_error)?;
            }
            IterationLifecycleTarget::Closed => {
                let reason =
                    Self::require_close_reason(request.close_reason, request.change_reason)?;
                let (mut commitment, commitment_version) = self
                    .iteration_repo
                    .get_commitment_with_version(request.iteration_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::NotFound)?;
                iteration
                    .close(reason.clone(), actor.clone())
                    .map_err(Self::map_domain_error)?;
                commitment
                    .close(reason, actor.clone())
                    .map_err(Self::map_domain_error)?;
                commitment_state = Some(commitment.commitment_state);
                affected_work_refs = commitment.committed_work_refs.refs.clone();
                self.iteration_repo
                    .save_commitment(commitment, Some(commitment_version), &uow)
                    .await
                    .map_err(Self::map_repository_error)?;
            }
            IterationLifecycleTarget::Cancelled => {
                let reason =
                    Self::require_change_reason(request.change_reason, request.close_reason)?;
                if let Some(commitment) = self
                    .iteration_repo
                    .get_commitment(request.iteration_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                {
                    commitment_state = Some(commitment.commitment_state);
                    affected_work_refs = commitment.committed_work_refs.refs;
                }
                iteration
                    .cancel(reason, actor.clone())
                    .map_err(Self::map_domain_error)?;
            }
        }

        let iteration_state = iteration.iteration_state;
        let version = self
            .iteration_repo
            .save_iteration(iteration.clone(), request.expected_version, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::IterationChanged(request.iteration_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let outbox_id = self
            .enqueue_outbox(
                WorkTruthChange::IterationChanged(request.iteration_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        self.projection_repo
            .mark_stale(
                Self::affected_views(
                    ProjectRef {
                        project_id: iteration.project_id.clone(),
                    },
                    self.affected_assignees(&affected_work_refs).await?,
                    request.iteration_ref.clone(),
                ),
                Self::iteration_cursor(&request.iteration_ref, version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = IterationCommandResult {
            iteration_ref: request.iteration_ref,
            iteration_state,
            commitment_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_iteration_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    async fn load_duplicate_iteration_result(
        &self,
        operation: OperationName,
        result_ref: ApplicationResultRef,
    ) -> Result<IterationCommandResult, ApplicationError> {
        let stored = self
            .command_results
            .get_result(result_ref)
            .await
            .map_err(Self::map_repository_error)?;
        match stored.and_then(|result| result.into_iteration_result(&operation)) {
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
                self.load_duplicate_iteration_result(operation.clone(), result_ref)
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

    async fn load_commit_candidates(
        &self,
        backlog_ref: work_contracts::BacklogRef,
        work_refs: &[FormalWorkRef],
    ) -> Result<Vec<(FormalWorkRecord, Version)>, ApplicationError> {
        if work_refs.is_empty() {
            return Err(ApplicationError::InvalidRequest);
        }

        let mut records = Vec::with_capacity(work_refs.len());
        for work_ref in work_refs {
            let contains = self
                .backlog_repo
                .contains_formal_work(backlog_ref.clone(), work_ref.clone())
                .await
                .map_err(Self::map_repository_error)?;
            if !contains {
                return Err(ApplicationError::DomainRejected);
            }
            let (record, version) = self
                .work_repo
                .get_formal_work_with_version(work_ref.clone())
                .await
                .map_err(Self::map_repository_error)?
                .ok_or(ApplicationError::NotFound)?;
            records.push((record, version));
        }
        Ok(records)
    }

    async fn mark_candidate_work_committed(
        &self,
        candidates: Vec<(FormalWorkRecord, Version)>,
        iteration_ref: IterationRef,
        actor: core_contracts::actor::ActorRef,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        for (record, version) in candidates {
            let updated = match record {
                FormalWorkRecord::WorkItem(mut work_item) => {
                    work_item
                        .mark_committed(iteration_ref.clone(), actor.clone())
                        .map_err(Self::map_domain_error)?;
                    FormalWorkRecord::WorkItem(work_item)
                }
                FormalWorkRecord::ChildWorkItem(mut child) => {
                    child
                        .mark_committed(iteration_ref.clone(), actor.clone())
                        .map_err(Self::map_domain_error)?;
                    FormalWorkRecord::ChildWorkItem(child)
                }
            };
            self.work_repo
                .save_formal_work(updated, version, uow)
                .await
                .map_err(Self::map_repository_error)?;
        }
        Ok(())
    }

    async fn validate_added_work_refs(
        &self,
        backlog_ref: work_contracts::BacklogRef,
        work_refs: &[FormalWorkRef],
    ) -> Result<(), ApplicationError> {
        for work_ref in work_refs {
            let contains = self
                .backlog_repo
                .contains_formal_work(backlog_ref.clone(), work_ref.clone())
                .await
                .map_err(Self::map_repository_error)?;
            if !contains {
                return Err(ApplicationError::DomainRejected);
            }
            self.work_repo
                .get_formal_work_with_version(work_ref.clone())
                .await
                .map_err(Self::map_repository_error)?
                .ok_or(ApplicationError::NotFound)?;
        }
        Ok(())
    }

    async fn affected_assignees(
        &self,
        work_refs: &[FormalWorkRef],
    ) -> Result<Vec<work_contracts::ProjectMemberRef>, ApplicationError> {
        let mut assignees = Vec::new();
        for work_ref in work_refs {
            let scope = self
                .work_repo
                .get_formal_work_scope(work_ref.clone())
                .await
                .map_err(Self::map_repository_error)?
                .ok_or(ApplicationError::NotFound)?;
            if let Some(assignee_ref) = scope.assignee_ref {
                if !assignees.contains(&assignee_ref) {
                    assignees.push(assignee_ref);
                }
            }
        }
        Ok(assignees)
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

    async fn finish_iteration_command(
        &self,
        result_ref: ApplicationResultRef,
        result: IterationCommandResult,
        reservation: IdempotencyRecord,
        uow: UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        self.command_results
            .save_result(
                result_ref.clone(),
                StoredCommandResult::Iteration(result),
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

    fn ensure_timebox_can_bind_to_project(
        summary: &ProcessTimeboxSummary,
        project_ref: ProjectRef,
    ) -> Result<(), ApplicationError> {
        if summary.project_ref != project_ref
            || !summary.can_open_iteration
            || summary.source_digest.0.trim().is_empty()
        {
            return Err(ApplicationError::ExternalReferenceUnresolved);
        }
        Ok(())
    }

    fn require_change_reason(
        change_reason: Option<IterationChangeReason>,
        close_reason: Option<IterationCloseReason>,
    ) -> Result<IterationChangeReason, ApplicationError> {
        match (change_reason, close_reason) {
            (Some(reason), None) => Ok(reason),
            _ => Err(ApplicationError::InvalidRequest),
        }
    }

    fn require_close_reason(
        close_reason: Option<IterationCloseReason>,
        change_reason: Option<IterationChangeReason>,
    ) -> Result<IterationCloseReason, ApplicationError> {
        match (close_reason, change_reason) {
            (Some(reason), None) => Ok(reason),
            _ => Err(ApplicationError::InvalidRequest),
        }
    }

    fn merge_work_refs(left: &[FormalWorkRef], right: &[FormalWorkRef]) -> Vec<FormalWorkRef> {
        let mut merged = Vec::with_capacity(left.len() + right.len());
        for work_ref in left.iter().chain(right.iter()) {
            if !merged.contains(work_ref) {
                merged.push(work_ref.clone());
            }
        }
        merged
    }

    fn affected_views(
        project_ref: ProjectRef,
        assignee_refs: Vec<work_contracts::ProjectMemberRef>,
        iteration_ref: IterationRef,
    ) -> Vec<DerivedWorkViewRef> {
        let mut affected = vec![
            DerivedWorkViewRef::project_board(project_ref),
            DerivedWorkViewRef::iteration_summary(iteration_ref),
        ];
        for member_ref in assignee_refs {
            affected.push(DerivedWorkViewRef::member_work(member_ref));
        }
        affected
    }

    fn iteration_cursor(iteration_ref: &IterationRef, version: Version) -> WorkTruthCursor {
        WorkTruthCursor(format!(
            "iteration:{}:v{}",
            iteration_ref.iteration_id.0, version
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

    fn map_port_error(error: PortError) -> ApplicationError {
        match error {
            PortError::NotFound | PortError::Rejected => {
                ApplicationError::ExternalReferenceUnresolved
            }
            PortError::Unavailable => ApplicationError::TemporarilyUnavailable,
            PortError::InvalidResponse => ApplicationError::DomainRejected,
        }
    }

    fn map_domain_error(error: DomainError) -> ApplicationError {
        match error {
            DomainError::MissingRequiredValue => ApplicationError::InvalidRequest,
            DomainError::InvalidStateTransition
            | DomainError::PolicyRejected
            | DomainError::InvariantViolation
            | DomainError::ExternalBodyRejected
            | DomainError::ProjectionMutationRejected
            | DomainError::RefMismatch => ApplicationError::DomainRejected,
        }
    }

    fn map_idempotency_error(error: IdempotencyError) -> ApplicationError {
        match error {
            IdempotencyError::AlreadyReserved => ApplicationError::TemporarilyUnavailable,
            IdempotencyError::Conflict => ApplicationError::IdempotencyConflict,
            IdempotencyError::StoreUnavailable => ApplicationError::TemporarilyUnavailable,
        }
    }

    fn map_uow_error_begin(_error: UnitOfWorkError) -> ApplicationError {
        ApplicationError::TemporarilyUnavailable
    }

    fn map_uow_error_commit(error: UnitOfWorkError) -> ApplicationError {
        match error {
            UnitOfWorkError::BeginFailed | UnitOfWorkError::RollbackFailed => {
                ApplicationError::TemporarilyUnavailable
            }
            UnitOfWorkError::CommitFailed => ApplicationError::CommitStatusUnknown,
        }
    }

    fn map_uow_error_rollback(_error: UnitOfWorkError) -> ApplicationError {
        ApplicationError::TemporarilyUnavailable
    }
}

enum ReservationOutcome {
    Reserved((UnitOfWorkHandle, IdempotencyRecord)),
    Duplicate(IterationCommandResult),
}
