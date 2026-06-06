//! Dependency and blocker command application services.

use core_contracts::metadata::{OperationName, RequestMetadata, Version};
use serde::Serialize;

use crate::results::CommandResultRepository;
use crate::{
    ApplicationError, AuditRepository, ClockPort, DependencyRepository, EvidenceResolverPort,
    FormalWorkScope, IdGeneratorPort, IdempotencyError, IdempotencyRecord, IdempotencyRepository,
    IdempotencyReservation, PortError, ProjectionRepository, RepositoryError, RequestDigest,
    StoredCommandResult, UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, WorkItemRepository,
    WorkOutboxRepository,
};
use work_contracts::{
    ApplicationResultRef, DependencyChangeReason, DependencyChangeReasonKind,
    DependencyCommandResult, DependencyOrBlockerRef, DependencyTarget, DerivedWorkViewRef,
    LinkWorkDependencyRequest, OpenWorkBlockerRequest, ResolveWorkBlockerRequest,
    UpdateWorkDependencyStateRequest, WorkCommandEnvelope, WorkCommandReceipt, WorkTruthChange,
    WorkTruthCursor,
};
use work_domain::{
    CompletionEvidencePolicy, DependencyChangeRecord, DependencyGraphPolicy, DomainError,
    WorkAuditTrail, WorkBlocker, WorkDependency, WorkOutboxRecord, WorkTraceRecord,
};

/// Coordinates dependency and blocker commands inside one application transaction boundary.
pub struct DependencyBlockerService<D, W, A, O, R, PR, U, E, I, C, IDEM> {
    /// Dependency and blocker repository port.
    pub dependency_repo: D,
    /// Formal work repository port.
    pub work_repo: W,
    /// Audit repository port.
    pub audit_repo: A,
    /// Outbox repository port.
    pub outbox_repo: O,
    /// Stored command result repository port.
    pub command_results: R,
    /// Projection freshness repository port.
    pub projection_repo: PR,
    /// Unit-of-work factory.
    pub unit_of_work: U,
    /// Evidence resolver port.
    pub evidence_resolver: E,
    /// Work-owned id generator.
    pub ids: I,
    /// Clock port.
    pub clock: C,
    /// Idempotency repository.
    pub idempotency: IDEM,
}

impl<D, W, A, O, R, PR, U, E, I, C, IDEM>
    DependencyBlockerService<D, W, A, O, R, PR, U, E, I, C, IDEM>
where
    D: DependencyRepository,
    W: WorkItemRepository,
    A: AuditRepository,
    O: WorkOutboxRepository,
    R: CommandResultRepository,
    PR: ProjectionRepository,
    U: UnitOfWork,
    E: EvidenceResolverPort,
    I: IdGeneratorPort,
    C: ClockPort,
    IDEM: IdempotencyRepository,
{
    /// Links and activates a dependency relation between two formal work records.
    pub async fn link_dependency(
        &self,
        envelope: WorkCommandEnvelope<LinkWorkDependencyRequest>,
    ) -> Result<DependencyCommandResult, ApplicationError> {
        let operation = OperationName::new("link_work_dependency");
        let (uow, reservation) = match self
            .reserve_dependency_command(&operation, &envelope)
            .await?
        {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        self.ensure_formal_work_exists(request.upstream_work_ref.clone(), &uow)
            .await?;
        self.ensure_formal_work_exists(request.downstream_work_ref.clone(), &uow)
            .await?;
        let downstream_scope = self
            .work_repo
            .get_formal_work_scope(request.downstream_work_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let graph = self
            .dependency_repo
            .load_graph_snapshot(downstream_scope.project_ref.clone())
            .await
            .map_err(Self::map_repository_error)?;
        if graph.project_ref != downstream_scope.project_ref {
            return self
                .rollback_and_err(uow, ApplicationError::DomainRejected)
                .await;
        }
        if let Err(error) = DependencyGraphPolicy::assert_can_link(
            &graph,
            request.upstream_work_ref.clone(),
            request.downstream_work_ref.clone(),
        ) {
            return self
                .rollback_and_err(uow, Self::map_domain_error(error))
                .await;
        }

        let dependency_id = self
            .ids
            .next_work_dependency_id()
            .map_err(Self::map_port_error)?;
        let link_reason = request.reason;
        let activation_reason = DependencyChangeReason::from_link_reason(link_reason.clone());
        let mut dependency = WorkDependency::link(
            dependency_id,
            request.upstream_work_ref,
            request.downstream_work_ref,
            link_reason,
        )
        .map_err(Self::map_domain_error)?;
        dependency
            .activate(actor, activation_reason.clone())
            .map_err(Self::map_domain_error)?;

        let dependency_ref = dependency.dependency_ref();
        let dependency_state = dependency.dependency_state;
        let version = self
            .dependency_repo
            .create_dependency(dependency.clone(), &uow)
            .await
            .map_err(Self::map_repository_error)?;
        let change_id = self
            .ids
            .next_dependency_change_id()
            .map_err(Self::map_port_error)?;
        let history = DependencyChangeRecord::from_dependency_change(
            change_id,
            dependency,
            activation_reason,
        )
        .map_err(Self::map_domain_error)?;
        self.dependency_repo
            .append_change(history, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Dependency(
                    dependency_ref.clone(),
                )),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let outbox_id = self
            .enqueue_outbox(
                WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Dependency(
                    dependency_ref.clone(),
                )),
                &uow,
            )
            .await?;
        self.projection_repo
            .mark_stale(
                Self::affected_relation_views(&downstream_scope),
                Self::relation_cursor("dependency", &dependency_ref.dependency_id.0, version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = DependencyCommandResult {
            dependency_ref,
            dependency_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_dependency_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    /// Updates one dependency relation state.
    pub async fn update_dependency_state(
        &self,
        envelope: WorkCommandEnvelope<UpdateWorkDependencyStateRequest>,
    ) -> Result<DependencyCommandResult, ApplicationError> {
        let operation = OperationName::new("update_work_dependency_state");
        let (uow, reservation) = match self
            .reserve_dependency_command(&operation, &envelope)
            .await?
        {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let mut dependency = self
            .dependency_repo
            .get_dependency(request.dependency_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let downstream_scope = self
            .work_repo
            .get_formal_work_scope(dependency.downstream_work_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;

        match request.target {
            DependencyTarget::Active => {
                if request.reason.reason_kind != DependencyChangeReasonKind::Activated {
                    return self
                        .rollback_and_err(uow, ApplicationError::InvalidRequest)
                        .await;
                }
                self.ensure_formal_work_exists(dependency.upstream_work_ref.clone(), &uow)
                    .await?;
                self.ensure_formal_work_exists(dependency.downstream_work_ref.clone(), &uow)
                    .await?;
                let graph = self
                    .dependency_repo
                    .load_graph_snapshot(downstream_scope.project_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?;
                if graph.project_ref != downstream_scope.project_ref {
                    return self
                        .rollback_and_err(uow, ApplicationError::DomainRejected)
                        .await;
                }
                let policy = DependencyGraphPolicy::from_graph(graph);
                if let Err(error) = policy.assert_dependency_state_transition_allowed(
                    &dependency,
                    request.target,
                    &request.reason,
                    None,
                ) {
                    return self
                        .rollback_and_err(uow, Self::map_domain_error(error))
                        .await;
                }
                dependency
                    .activate(actor, request.reason.clone())
                    .map_err(Self::map_domain_error)?;
            }
            DependencyTarget::Satisfied => {
                if request.reason.reason_kind != DependencyChangeReasonKind::SatisfiedByEvidence {
                    return self
                        .rollback_and_err(uow, ApplicationError::InvalidRequest)
                        .await;
                }
                let Some(evidence_ref) = request.evidence_ref.clone() else {
                    return self
                        .rollback_and_err(uow, ApplicationError::InvalidRequest)
                        .await;
                };
                let evidence = self
                    .evidence_resolver
                    .resolve_evidence(evidence_ref.clone())
                    .await
                    .map_err(Self::map_port_error)?;
                if evidence.verified_state != work_contracts::EvidenceVerifiedState::Verified {
                    return self
                        .rollback_and_err(uow, ApplicationError::InvalidRequest)
                        .await;
                }
                CompletionEvidencePolicy::assert_completion_evidence(
                    dependency.downstream_work_ref.clone(),
                    evidence.evidence_ref.clone(),
                )
                .map_err(Self::map_domain_error)?;
                dependency
                    .mark_satisfied(evidence.evidence_ref, actor)
                    .map_err(Self::map_domain_error)?;
            }
            DependencyTarget::Waived => {
                if request.reason.reason_kind != DependencyChangeReasonKind::Waived {
                    return self
                        .rollback_and_err(uow, ApplicationError::InvalidRequest)
                        .await;
                }
                dependency
                    .waive(request.reason.clone(), actor)
                    .map_err(Self::map_domain_error)?;
            }
            DependencyTarget::Cancelled => {
                if request.reason.reason_kind != DependencyChangeReasonKind::Cancelled {
                    return self
                        .rollback_and_err(uow, ApplicationError::InvalidRequest)
                        .await;
                }
                dependency
                    .cancel(request.reason.clone(), actor)
                    .map_err(Self::map_domain_error)?;
            }
        }

        let dependency_ref = dependency.dependency_ref();
        let dependency_state = dependency.dependency_state;
        let version = self
            .dependency_repo
            .save_dependency(dependency.clone(), request.expected_version, &uow)
            .await
            .map_err(Self::map_repository_error)?;
        let change_id = self
            .ids
            .next_dependency_change_id()
            .map_err(Self::map_port_error)?;
        let history =
            DependencyChangeRecord::from_dependency_change(change_id, dependency, request.reason)
                .map_err(Self::map_domain_error)?;
        self.dependency_repo
            .append_change(history, &uow)
            .await
            .map_err(Self::map_repository_error)?;
        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Dependency(
                    dependency_ref.clone(),
                )),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let outbox_id = self
            .enqueue_outbox(
                WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Dependency(
                    dependency_ref.clone(),
                )),
                &uow,
            )
            .await?;
        self.projection_repo
            .mark_stale(
                Self::affected_relation_views(&downstream_scope),
                Self::relation_cursor("dependency", &dependency_ref.dependency_id.0, version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = DependencyCommandResult {
            dependency_ref,
            dependency_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_dependency_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    /// Opens one blocker for formal work.
    pub async fn open_blocker(
        &self,
        envelope: WorkCommandEnvelope<OpenWorkBlockerRequest>,
    ) -> Result<work_contracts::BlockerCommandResult, ApplicationError> {
        let operation = OperationName::new("open_work_blocker");
        let (uow, reservation) = match self.reserve_blocker_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        self.ensure_formal_work_exists(request.blocked_work_ref.clone(), &uow)
            .await?;
        let blocked_scope = self
            .work_repo
            .get_formal_work_scope(request.blocked_work_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let blocker_id = self
            .ids
            .next_work_blocker_id()
            .map_err(Self::map_port_error)?;
        let blocker = WorkBlocker::open(
            blocker_id,
            request.blocked_work_ref,
            request.cause_ref,
            actor,
        )
        .map_err(Self::map_domain_error)?;
        let blocker_ref = blocker.blocker_ref();
        let blocker_state = blocker.blocker_state;
        let version = self
            .dependency_repo
            .create_blocker(blocker.clone(), &uow)
            .await
            .map_err(Self::map_repository_error)?;
        let change_id = self
            .ids
            .next_dependency_change_id()
            .map_err(Self::map_port_error)?;
        let reason = DependencyChangeReason::from_blocker_cause(blocker.cause_ref.clone());
        let history = DependencyChangeRecord::from_blocker_change(change_id, blocker, reason)
            .map_err(Self::map_domain_error)?;
        self.dependency_repo
            .append_change(history, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Blocker(
                    blocker_ref.clone(),
                )),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let outbox_id = self
            .enqueue_outbox(
                WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Blocker(
                    blocker_ref.clone(),
                )),
                &uow,
            )
            .await?;
        self.projection_repo
            .mark_stale(
                Self::affected_relation_views(&blocked_scope),
                Self::relation_cursor("blocker", &blocker_ref.blocker_id.0, version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = work_contracts::BlockerCommandResult {
            blocker_ref,
            blocker_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_blocker_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    /// Resolves one blocker with verified evidence.
    pub async fn resolve_blocker(
        &self,
        envelope: WorkCommandEnvelope<ResolveWorkBlockerRequest>,
    ) -> Result<work_contracts::BlockerCommandResult, ApplicationError> {
        let operation = OperationName::new("resolve_work_blocker");
        let (uow, reservation) = match self.reserve_blocker_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let mut blocker = self
            .dependency_repo
            .get_blocker(request.blocker_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let blocked_scope = self
            .work_repo
            .get_formal_work_scope(blocker.blocked_work_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        let evidence = self
            .evidence_resolver
            .resolve_evidence(request.evidence_ref.clone())
            .await
            .map_err(Self::map_port_error)?;
        if evidence.verified_state != work_contracts::EvidenceVerifiedState::Verified {
            return self
                .rollback_and_err(uow, ApplicationError::InvalidRequest)
                .await;
        }
        CompletionEvidencePolicy::assert_completion_evidence(
            blocker.blocked_work_ref.clone(),
            evidence.evidence_ref.clone(),
        )
        .map_err(Self::map_domain_error)?;
        blocker
            .resolve(evidence.evidence_ref, actor)
            .map_err(Self::map_domain_error)?;

        let blocker_ref = blocker.blocker_ref();
        let blocker_state = blocker.blocker_state;
        let version = self
            .dependency_repo
            .save_blocker(blocker.clone(), request.expected_version, &uow)
            .await
            .map_err(Self::map_repository_error)?;
        let change_id = self
            .ids
            .next_dependency_change_id()
            .map_err(Self::map_port_error)?;
        let reason = DependencyChangeReason {
            reason_kind: DependencyChangeReasonKind::SatisfiedByEvidence,
            reason_ref: Some(request.evidence_ref),
            blocker_cause_ref: None,
        };
        let history = DependencyChangeRecord::from_blocker_change(change_id, blocker, reason)
            .map_err(Self::map_domain_error)?;
        self.dependency_repo
            .append_change(history, &uow)
            .await
            .map_err(Self::map_repository_error)?;

        let trace_id = self
            .append_trace_and_audit(
                WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Blocker(
                    blocker_ref.clone(),
                )),
                &envelope.metadata.request,
                &uow,
            )
            .await?;
        let outbox_id = self
            .enqueue_outbox(
                WorkTruthChange::WorkRelationChanged(DependencyOrBlockerRef::Blocker(
                    blocker_ref.clone(),
                )),
                &uow,
            )
            .await?;
        self.projection_repo
            .mark_stale(
                Self::affected_relation_views(&blocked_scope),
                Self::relation_cursor("blocker", &blocker_ref.blocker_id.0, version),
                &uow,
            )
            .await
            .map_err(Self::map_repository_error)?;

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = work_contracts::BlockerCommandResult {
            blocker_ref,
            blocker_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_blocker_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    async fn ensure_formal_work_exists(
        &self,
        work_ref: work_contracts::FormalWorkRef,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        match self.work_repo.get_formal_work(work_ref).await {
            Ok(Some(_)) => Ok(()),
            Ok(None) => {
                self.rollback_and_err(uow.clone(), ApplicationError::NotFound)
                    .await
            }
            Err(error) => {
                self.rollback_and_err(uow.clone(), Self::map_repository_error(error))
                    .await
            }
        }
    }

    async fn load_duplicate_dependency_result(
        &self,
        operation: OperationName,
        result_ref: ApplicationResultRef,
    ) -> Result<DependencyCommandResult, ApplicationError> {
        let stored = self
            .command_results
            .get_result(result_ref)
            .await
            .map_err(Self::map_repository_error)?;
        match stored.and_then(|result| result.into_dependency_result(&operation)) {
            Some(result) => Ok(result.with_duplicate_receipt()),
            None => Err(ApplicationError::DuplicateResultMissing),
        }
    }

    async fn load_duplicate_blocker_result(
        &self,
        operation: OperationName,
        result_ref: ApplicationResultRef,
    ) -> Result<work_contracts::BlockerCommandResult, ApplicationError> {
        let stored = self
            .command_results
            .get_result(result_ref)
            .await
            .map_err(Self::map_repository_error)?;
        match stored.and_then(|result| result.into_blocker_result(&operation)) {
            Some(result) => Ok(result.with_duplicate_receipt()),
            None => Err(ApplicationError::DuplicateResultMissing),
        }
    }

    async fn reserve_dependency_command<T: Serialize>(
        &self,
        operation: &OperationName,
        envelope: &WorkCommandEnvelope<T>,
    ) -> Result<ReservationOutcome<DependencyCommandResult>, ApplicationError> {
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
                self.load_duplicate_dependency_result(operation.clone(), result_ref)
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

    async fn reserve_blocker_command<T: Serialize>(
        &self,
        operation: &OperationName,
        envelope: &WorkCommandEnvelope<T>,
    ) -> Result<ReservationOutcome<work_contracts::BlockerCommandResult>, ApplicationError> {
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
                self.load_duplicate_blocker_result(operation.clone(), result_ref)
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
        uow: &UnitOfWorkHandle,
    ) -> Result<work_contracts::WorkOutboxId, ApplicationError> {
        let outbox_id = self.ids.next_outbox_id().map_err(Self::map_port_error)?;
        let outbox = WorkOutboxRecord::from_truth_change(outbox_id.clone(), change)
            .map_err(Self::map_domain_error)?;
        self.outbox_repo
            .enqueue(outbox, uow)
            .await
            .map_err(Self::map_repository_error)?;
        Ok(outbox_id)
    }

    async fn finish_dependency_command(
        &self,
        result_ref: ApplicationResultRef,
        result: DependencyCommandResult,
        reservation: IdempotencyRecord,
        uow: UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        self.command_results
            .save_result(
                result_ref.clone(),
                StoredCommandResult::Dependency(result),
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

    async fn finish_blocker_command(
        &self,
        result_ref: ApplicationResultRef,
        result: work_contracts::BlockerCommandResult,
        reservation: IdempotencyRecord,
        uow: UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        self.command_results
            .save_result(
                result_ref.clone(),
                StoredCommandResult::Blocker(result),
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

    fn affected_relation_views(scope: &FormalWorkScope) -> Vec<DerivedWorkViewRef> {
        let mut affected = vec![DerivedWorkViewRef::project_board(scope.project_ref.clone())];
        if let Some(member_ref) = scope.assignee_ref.clone() {
            affected.push(DerivedWorkViewRef::member_work(member_ref));
        }
        affected
    }

    fn relation_cursor(prefix: &str, relation_id: &str, version: Version) -> WorkTruthCursor {
        WorkTruthCursor(format!("{prefix}:{relation_id}:v{version}"))
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

enum ReservationOutcome<R> {
    Reserved((UnitOfWorkHandle, IdempotencyRecord)),
    Duplicate(R),
}
