//! Promote command application services.

use core_contracts::metadata::{OperationName, RequestMetadata, Version};
use serde::Serialize;

use crate::results::CommandResultRepository;
use crate::{
    ApplicationError, AuditRepository, BacklogRepository, ClockPort, IdGeneratorPort,
    IdempotencyError, IdempotencyRecord, IdempotencyRepository, IdempotencyReservation, PortError,
    ProjectMemberRepository, ProjectionRepository, PromoteRepository, RepositoryError,
    RequestDigest, SourceWorkResolverPort, StoredCommandResult, UnitOfWork, UnitOfWorkError,
    UnitOfWorkHandle, WorkItemRepository, WorkOutboxRepository,
};
use work_contracts::{
    ApplicationResultRef, DerivedWorkViewRef, FormalWorkIntent, FormalWorkRef,
    ProjectMemberResponsibilityState, PromoteCommandResult, PromoteDecision, PromoteReviewDecision,
    RequestWorkPromotionRequest, ReviewWorkPromotionRequest, WorkCommandEnvelope,
    WorkCommandReceipt, WorkTruthChange, WorkTruthCursor,
};
use work_domain::{
    DomainError, PromoteDecisionRecord, PromotePolicy, PromoteResult, WorkAuditTrail, WorkItem,
    WorkOutboxRecord, WorkTraceRecord, WorkTruthPolicy,
};

/// Coordinates promote commands inside one application transaction boundary.
pub struct PromoteCommandService<PM, B, W, P, A, O, R, PR, U, S, I, C, IDEM> {
    /// Project-member repository port.
    pub member_repo: PM,
    /// Backlog repository port.
    pub backlog_repo: B,
    /// Work item repository port.
    pub work_repo: W,
    /// Promote repository port.
    pub promote_repo: P,
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
    /// Source resolver port.
    pub source_resolver: S,
    /// Id generator.
    pub ids: I,
    /// Clock port.
    pub clock: C,
    /// Idempotency repository.
    pub idempotency: IDEM,
}

impl<PM, B, W, P, A, O, R, PR, U, S, I, C, IDEM>
    PromoteCommandService<PM, B, W, P, A, O, R, PR, U, S, I, C, IDEM>
where
    PM: ProjectMemberRepository,
    B: BacklogRepository,
    W: WorkItemRepository,
    P: PromoteRepository,
    A: AuditRepository,
    O: WorkOutboxRepository,
    R: CommandResultRepository,
    PR: ProjectionRepository,
    U: UnitOfWork,
    S: SourceWorkResolverPort,
    I: IdGeneratorPort,
    C: ClockPort,
    IDEM: IdempotencyRepository,
{
    /// Records a pending-review promote result from an external source.
    pub async fn request_promotion(
        &self,
        envelope: WorkCommandEnvelope<RequestWorkPromotionRequest>,
    ) -> Result<PromoteCommandResult, ApplicationError> {
        let operation = OperationName::new("request_work_promotion");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let source = match self
            .source_resolver
            .resolve_source_work(request.source_ref.clone())
            .await
        {
            Ok(source) => source,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_port_error(error))
                    .await;
            }
        };
        if let Err(error) = WorkTruthPolicy::assert_no_external_body(source.summary) {
            return self
                .rollback_and_err(uow, Self::map_domain_error(error))
                .await;
        }
        match self
            .promote_repo
            .find_latest_by_source(request.source_ref.clone())
            .await
        {
            Ok(Some(existing))
                if matches!(
                    existing.result_state,
                    work_contracts::PromoteResultState::PendingReview
                ) =>
            {
                return self
                    .rollback_and_err(uow, ApplicationError::DomainRejected)
                    .await;
            }
            Ok(_) => {}
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        }
        match PromotePolicy::can_promote(request.source_ref.clone(), request.reason.clone()) {
            PromoteDecision::Allow => {}
            PromoteDecision::Reject(_) | PromoteDecision::Duplicate(_) => {
                return self
                    .rollback_and_err(uow, ApplicationError::DomainRejected)
                    .await;
            }
        }

        let promote_result_id = match self.ids.next_promote_result_id() {
            Ok(promote_result_id) => promote_result_id,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_port_error(error))
                    .await;
            }
        };
        let result = match PromoteResult::evaluate(
            promote_result_id,
            request.source_ref,
            request.reason,
            actor,
        ) {
            Ok(result) => result,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_domain_error(error))
                    .await;
            }
        };
        let promote_result_ref = result.promote_result_ref();
        let result_state = result.result_state;
        let version = match self.promote_repo.create(result, &uow).await {
            Ok(version) => version,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };

        let trace_id = match self
            .append_trace_and_audit(
                WorkTruthChange::PromoteResultRecorded(promote_result_ref.clone()),
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
                WorkTruthChange::PromoteResultRecorded(promote_result_ref.clone()),
                &uow,
            )
            .await
        {
            Ok(outbox_id) => outbox_id,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = PromoteCommandResult {
            promote_result_ref,
            result_state,
            created_work_ref: None,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_promote_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    /// Reviews one pending promote result and optionally creates formal Work.
    pub async fn review_promotion(
        &self,
        envelope: WorkCommandEnvelope<ReviewWorkPromotionRequest>,
    ) -> Result<PromoteCommandResult, ApplicationError> {
        let operation = OperationName::new("review_work_promotion");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let mut result = match self
            .promote_repo
            .get(request.promote_result_ref.clone())
            .await
        {
            Ok(Some(result)) => result,
            Ok(None) => return self.rollback_and_err(uow, ApplicationError::NotFound).await,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };

        if result.result_state != work_contracts::PromoteResultState::PendingReview {
            return self
                .rollback_and_err(uow, ApplicationError::VersionConflict)
                .await;
        }

        let mut created_work_ref = None;
        let mut created_work_version = None;
        let mut stale_views = Vec::new();
        match request.decision {
            PromoteReviewDecision::Accept => {
                let Some(intent) = request.accepted_work_intent else {
                    return self
                        .rollback_and_err(uow, ApplicationError::InvalidRequest)
                        .await;
                };
                let source = match self
                    .source_resolver
                    .resolve_source_work(result.source_ref.clone())
                    .await
                {
                    Ok(source) => source,
                    Err(error) => {
                        return self
                            .rollback_and_err(uow, Self::map_port_error(error))
                            .await;
                    }
                };
                if let Err(error) = WorkTruthPolicy::assert_no_external_body(source.summary) {
                    return self
                        .rollback_and_err(uow, Self::map_domain_error(error))
                        .await;
                }
                let (backlog, project_ref, assignee_ref) =
                    match self.load_accept_target_backlog(&intent).await {
                        Ok(parts) => parts,
                        Err(error) => return self.rollback_and_err(uow, error).await,
                    };
                let work_item_id = match self.ids.next_work_item_id() {
                    Ok(work_item_id) => work_item_id,
                    Err(error) => {
                        return self
                            .rollback_and_err(uow, Self::map_port_error(error))
                            .await;
                    }
                };
                let work_item = match WorkItem::formalize(
                    work_item_id,
                    backlog.backlog_id.clone(),
                    intent,
                    result.source_ref.clone(),
                    actor.clone(),
                ) {
                    Ok(work_item) => work_item,
                    Err(error) => {
                        return self
                            .rollback_and_err(uow, Self::map_domain_error(error))
                            .await;
                    }
                };
                let work_ref = work_item.formal_work_ref();
                let work_version = match self.work_repo.create_work_item(work_item, &uow).await {
                    Ok(version) => version,
                    Err(error) => {
                        return self
                            .rollback_and_err(uow, Self::map_repository_error(error))
                            .await;
                    }
                };
                if let Err(error) = self
                    .backlog_repo
                    .add_formal_work(backlog.backlog_ref(), work_ref.clone(), &uow)
                    .await
                {
                    return self
                        .rollback_and_err(uow, Self::map_repository_error(error))
                        .await;
                }
                if let Err(error) = result.accept(work_ref.clone(), actor.clone()) {
                    return self
                        .rollback_and_err(uow, Self::map_domain_error(error))
                        .await;
                }
                created_work_ref = Some(work_ref);
                created_work_version = Some(work_version);
                stale_views = vec![
                    DerivedWorkViewRef::project_board(project_ref),
                    DerivedWorkViewRef::member_work(assignee_ref),
                ];
            }
            PromoteReviewDecision::Reject(reason) => {
                if request.accepted_work_intent.is_some() {
                    return self
                        .rollback_and_err(uow, ApplicationError::InvalidRequest)
                        .await;
                }
                if let Err(error) = result.reject(reason, actor.clone()) {
                    return self
                        .rollback_and_err(uow, Self::map_domain_error(error))
                        .await;
                }
            }
        }

        let result_state = result.result_state;
        let promote_result_ref = result.promote_result_ref();
        let version = match self
            .promote_repo
            .save(result.clone(), request.expected_version, &uow)
            .await
        {
            Ok(version) => version,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };
        let decision = match self.ids.next_promote_decision_id() {
            Ok(decision_id) => PromoteDecisionRecord::from_result(decision_id, result, actor)
                .map_err(Self::map_domain_error),
            Err(error) => Err(Self::map_port_error(error)),
        };
        let decision = match decision {
            Ok(decision) => decision,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };
        if let Err(error) = self.promote_repo.append_decision(decision, &uow).await {
            return self
                .rollback_and_err(uow, Self::map_repository_error(error))
                .await;
        }

        let trace_id = match self
            .append_trace_and_audit(
                WorkTruthChange::PromoteResultRecorded(promote_result_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await
        {
            Ok(trace_id) => trace_id,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };
        let mut outbox_refs = vec![match self
            .enqueue_outbox(
                WorkTruthChange::PromoteResultRecorded(promote_result_ref.clone()),
                &uow,
            )
            .await
        {
            Ok(outbox_id) => outbox_id,
            Err(error) => return self.rollback_and_err(uow, error).await,
        }];
        if let Some(work_ref) = created_work_ref.clone() {
            match self
                .append_trace_and_audit(
                    WorkTruthChange::WorkItemChanged(work_ref.clone()),
                    &envelope.metadata.request,
                    &uow,
                )
                .await
            {
                Ok(_) => {}
                Err(error) => return self.rollback_and_err(uow, error).await,
            }
            outbox_refs.push(
                match self
                    .enqueue_outbox(WorkTruthChange::WorkItemChanged(work_ref.clone()), &uow)
                    .await
                {
                    Ok(outbox_id) => outbox_id,
                    Err(error) => return self.rollback_and_err(uow, error).await,
                },
            );
            let work_version = created_work_version.expect("created work version should exist");
            if let Err(error) = self
                .projection_repo
                .mark_stale(
                    stale_views,
                    Self::work_cursor(&work_ref, work_version),
                    &uow,
                )
                .await
            {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        }

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = PromoteCommandResult {
            promote_result_ref,
            result_state,
            created_work_ref,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                outbox_refs,
                version,
            ),
        };
        self.finish_promote_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    async fn load_accept_target_backlog(
        &self,
        intent: &FormalWorkIntent,
    ) -> Result<
        (
            work_domain::Backlog,
            work_contracts::ProjectRef,
            work_contracts::ProjectMemberRef,
        ),
        ApplicationError,
    > {
        let member = self
            .member_repo
            .get(intent.assignee_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        if member.responsibility_state != ProjectMemberResponsibilityState::Active {
            return Err(ApplicationError::NotFound);
        }
        let project_ref = work_contracts::ProjectRef {
            project_id: member.project_id.clone(),
        };
        let backlog = self
            .backlog_repo
            .get_by_project(project_ref.clone())
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        Ok((backlog, project_ref, intent.assignee_ref.clone()))
    }

    async fn load_duplicate_promote_result(
        &self,
        operation: OperationName,
        result_ref: ApplicationResultRef,
    ) -> Result<PromoteCommandResult, ApplicationError> {
        let stored = self
            .command_results
            .get_result(result_ref)
            .await
            .map_err(Self::map_repository_error)?;
        match stored.and_then(|result| result.into_promote_result(&operation)) {
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
                self.load_duplicate_promote_result(operation.clone(), result_ref)
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

    async fn finish_promote_command(
        &self,
        result_ref: ApplicationResultRef,
        result: PromoteCommandResult,
        reservation: IdempotencyRecord,
        uow: UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        self.command_results
            .save_result(
                result_ref.clone(),
                StoredCommandResult::Promote(result),
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

    fn work_cursor(work_ref: &FormalWorkRef, version: Version) -> WorkTruthCursor {
        match work_ref {
            FormalWorkRef::WorkItem(work_item_id) => {
                WorkTruthCursor(format!("work_item:{}:v{version}", work_item_id.0))
            }
            FormalWorkRef::ChildWorkItem(child_work_item_id) => WorkTruthCursor(format!(
                "child_work_item:{}:v{version}",
                child_work_item_id.0
            )),
        }
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
            PortError::InvalidResponse => ApplicationError::DomainRejected,
            PortError::Unavailable => ApplicationError::TemporarilyUnavailable,
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
}

enum ReservationOutcome {
    Reserved((UnitOfWorkHandle, IdempotencyRecord)),
    Duplicate(PromoteCommandResult),
}
