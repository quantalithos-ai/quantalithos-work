//! Formal work item application services.

use core_contracts::metadata::{OperationName, RequestMetadata, Version};
use serde::Serialize;

use crate::results::CommandResultRepository;
use crate::{
    ApplicationError, AuditRepository, BacklogRepository, ClockPort, EvidenceResolverPort,
    FormalWorkRecord, IdGeneratorPort, IdempotencyError, IdempotencyRecord, IdempotencyRepository,
    IdempotencyReservation, PortError, ProjectMemberRepository, ProjectRepository,
    ProjectionRepository, RepositoryError, RequestDigest, SourceWorkResolverPort,
    StoredCommandResult, UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, WorkItemRepository,
    WorkOutboxRepository,
};
use work_contracts::{
    ApplicationResultRef, CreateChildWorkItemRequest, CreateWorkItemRequest, DerivedWorkViewRef,
    FormalWorkRef, ProjectMemberResponsibilityState, ProjectRef, UpdateWorkItemLifecycleRequest,
    WorkCommandEnvelope, WorkCommandReceipt, WorkItemCommandResult, WorkLifecycleTarget,
    WorkTruthChange, WorkTruthCursor,
};
use work_domain::{
    ChildWorkItem, CompletionEvidencePolicy, DomainError, FormalWorkPolicy, WorkAuditTrail,
    WorkItem, WorkOutboxRecord, WorkTraceRecord, WorkTruthPolicy,
};

/// Coordinates formal work commands inside one application transaction boundary.
pub struct WorkItemCommandService<P, B, PM, W, A, O, R, PR, U, S, E, I, C, IDEM> {
    /// Project repository port.
    pub project_repo: P,
    /// Backlog repository port.
    pub backlog_repo: B,
    /// Project member repository port.
    pub member_repo: PM,
    /// Work item repository port.
    pub work_repo: W,
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
    /// Evidence resolver port.
    pub evidence_resolver: E,
    /// Id generator.
    pub ids: I,
    /// Clock port.
    pub clock: C,
    /// Idempotency repository.
    pub idempotency: IDEM,
}

impl<P, B, PM, W, A, O, R, PR, U, S, E, I, C, IDEM>
    WorkItemCommandService<P, B, PM, W, A, O, R, PR, U, S, E, I, C, IDEM>
where
    P: ProjectRepository,
    B: BacklogRepository,
    PM: ProjectMemberRepository,
    W: WorkItemRepository,
    A: AuditRepository,
    O: WorkOutboxRepository,
    R: CommandResultRepository,
    PR: ProjectionRepository,
    U: UnitOfWork,
    S: SourceWorkResolverPort,
    E: EvidenceResolverPort,
    I: IdGeneratorPort,
    C: ClockPort,
    IDEM: IdempotencyRepository,
{
    /// Creates a root formal work item.
    pub async fn create_work_item(
        &self,
        envelope: WorkCommandEnvelope<CreateWorkItemRequest>,
    ) -> Result<WorkItemCommandResult, ApplicationError> {
        let operation = OperationName::new("create_work_item");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let project_ref = request.project_ref.clone();
        let assignee_ref = request.work_intent.assignee_ref.clone();
        let work_intent = request.work_intent;
        let source_ref = request.source_ref;

        match self.project_repo.get(project_ref.clone()).await {
            Ok(Some(_)) => {}
            Ok(None) => return self.rollback_and_err(uow, ApplicationError::NotFound).await,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        }

        let backlog = match self.backlog_repo.get_by_project(project_ref.clone()).await {
            Ok(Some(backlog)) => backlog,
            Ok(None) => return self.rollback_and_err(uow, ApplicationError::NotFound).await,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };

        let member = match self.member_repo.get(assignee_ref.clone()).await {
            Ok(Some(member)) => member,
            Ok(None) => return self.rollback_and_err(uow, ApplicationError::NotFound).await,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };
        if member.project_id != project_ref.project_id
            || member.responsibility_state != ProjectMemberResponsibilityState::Active
        {
            return self.rollback_and_err(uow, ApplicationError::NotFound).await;
        }

        let resolved_source = match self
            .source_resolver
            .resolve_source_work(source_ref.clone())
            .await
        {
            Ok(source) => source,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_port_error(error))
                    .await;
            }
        };
        if let Err(error) = WorkTruthPolicy::assert_no_external_body(resolved_source.summary) {
            return self
                .rollback_and_err(uow, Self::map_domain_error(error))
                .await;
        }
        if let Err(error) =
            FormalWorkPolicy::assert_formal_work(work_intent.clone(), source_ref.clone())
        {
            return self
                .rollback_and_err(uow, Self::map_domain_error(error))
                .await;
        }
        if let Err(error) = backlog.assert_can_accept(&work_intent) {
            return self
                .rollback_and_err(uow, Self::map_domain_error(error))
                .await;
        }

        let work_item_id = match self.ids.next_work_item_id() {
            Ok(work_item_id) => work_item_id,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_port_error(error))
                    .await;
            }
        };
        let work = match WorkItem::formalize(
            work_item_id,
            backlog.backlog_id.clone(),
            work_intent,
            source_ref,
            actor,
        ) {
            Ok(work) => work,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_domain_error(error))
                    .await;
            }
        };
        let work_ref = work.formal_work_ref();
        let work_state = work.work_state;
        let stale_views = vec![
            DerivedWorkViewRef::project_board(project_ref),
            DerivedWorkViewRef::member_work(work.assignee_ref.clone()),
        ];
        let version = match self.work_repo.create_work_item(work, &uow).await {
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

        let trace_id = match self
            .append_trace_and_audit(
                WorkTruthChange::WorkItemChanged(work_ref.clone()),
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
                WorkTruthChange::WorkItemChanged(work_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await
        {
            Ok(outbox_id) => outbox_id,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };
        if let Err(error) = self
            .projection_repo
            .mark_stale(stale_views, Self::work_cursor(&work_ref, version), &uow)
            .await
        {
            return self
                .rollback_and_err(uow, Self::map_repository_error(error))
                .await;
        }

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = WorkItemCommandResult {
            work_ref,
            work_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_work_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    /// Creates a child formal work item.
    pub async fn create_child_work_item(
        &self,
        envelope: WorkCommandEnvelope<CreateChildWorkItemRequest>,
    ) -> Result<WorkItemCommandResult, ApplicationError> {
        let operation = OperationName::new("create_child_work_item");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let request = envelope.command;
        let parent_ref = request.parent_ref.clone();
        let work_intent = request.work_intent;
        let source_ref = request.source_ref;
        let parent = match self.work_repo.get_formal_work(parent_ref).await {
            Ok(Some(parent)) => parent,
            Ok(None) => return self.rollback_and_err(uow, ApplicationError::NotFound).await,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };
        let (parent_id, parent_backlog_id, parent_assignee_ref, parent_state) = match &parent {
            FormalWorkRecord::WorkItem(parent_work) => (
                parent_work.work_item_id.clone(),
                parent_work.backlog_id.clone(),
                parent_work.assignee_ref.clone(),
                parent_work.work_state,
            ),
            FormalWorkRecord::ChildWorkItem(_) => {
                return self
                    .rollback_and_err(uow, ApplicationError::DomainRejected)
                    .await;
            }
        };
        if matches_terminal(parent_state) {
            return self
                .rollback_and_err(uow, ApplicationError::DomainRejected)
                .await;
        }
        let project_ref = match self.project_ref_for_backlog(&parent_backlog_id).await {
            Ok(project_ref) => project_ref,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };

        let resolved_source = match self
            .source_resolver
            .resolve_source_work(source_ref.clone())
            .await
        {
            Ok(source) => source,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_port_error(error))
                    .await;
            }
        };
        if let Err(error) = WorkTruthPolicy::assert_no_external_body(resolved_source.summary) {
            return self
                .rollback_and_err(uow, Self::map_domain_error(error))
                .await;
        }
        if let Err(error) =
            FormalWorkPolicy::assert_formal_work(work_intent.clone(), source_ref.clone())
        {
            return self
                .rollback_and_err(uow, Self::map_domain_error(error))
                .await;
        }

        let child_work_item_id = match self.ids.next_child_work_item_id() {
            Ok(child_work_item_id) => child_work_item_id,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_port_error(error))
                    .await;
            }
        };
        let child = match ChildWorkItem::create_child(
            child_work_item_id,
            parent_id,
            work_intent,
            source_ref,
        ) {
            Ok(child) => child,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_domain_error(error))
                    .await;
            }
        };
        let work_ref = child.formal_work_ref();
        let work_state = child.work_state;
        let version = match self.work_repo.create_child_work_item(child, &uow).await {
            Ok(version) => version,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };
        if let Err(error) = self
            .backlog_repo
            .add_formal_work(
                work_contracts::BacklogRef {
                    backlog_id: parent_backlog_id,
                },
                work_ref.clone(),
                &uow,
            )
            .await
        {
            return self
                .rollback_and_err(uow, Self::map_repository_error(error))
                .await;
        }
        let trace_id = match self
            .append_trace_and_audit(
                WorkTruthChange::WorkItemChanged(work_ref.clone()),
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
                WorkTruthChange::WorkItemChanged(work_ref.clone()),
                &envelope.metadata.request,
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
                    DerivedWorkViewRef::member_work(parent_assignee_ref),
                ],
                Self::work_cursor(&work_ref, version),
                &uow,
            )
            .await
        {
            return self
                .rollback_and_err(uow, Self::map_repository_error(error))
                .await;
        }

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = WorkItemCommandResult {
            work_ref,
            work_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_work_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    /// Updates the lifecycle state for formal work.
    pub async fn update_lifecycle(
        &self,
        envelope: WorkCommandEnvelope<UpdateWorkItemLifecycleRequest>,
    ) -> Result<WorkItemCommandResult, ApplicationError> {
        let operation = OperationName::new("update_work_item_lifecycle");
        let (uow, reservation) = match self.reserve_command(&operation, &envelope).await? {
            ReservationOutcome::Reserved(parts) => parts,
            ReservationOutcome::Duplicate(result) => return Ok(result),
        };

        let actor = envelope.actor.actor_ref().clone();
        let request = envelope.command;
        let mut record = match self
            .work_repo
            .get_formal_work(request.work_ref.clone())
            .await
        {
            Ok(Some(record)) => record,
            Ok(None) => return self.rollback_and_err(uow, ApplicationError::NotFound).await,
            Err(error) => {
                return self
                    .rollback_and_err(uow, Self::map_repository_error(error))
                    .await;
            }
        };

        let (project_ref, member_ref) = match &record {
            FormalWorkRecord::WorkItem(work_item) => {
                let project_ref = match self.project_ref_for_backlog(&work_item.backlog_id).await {
                    Ok(project_ref) => project_ref,
                    Err(error) => return self.rollback_and_err(uow, error).await,
                };
                (project_ref, Some(work_item.assignee_ref.clone()))
            }
            FormalWorkRecord::ChildWorkItem(child) => {
                let parent_ref = FormalWorkRef::WorkItem(child.parent_work_item_id.clone());
                let Some(FormalWorkRecord::WorkItem(parent_work)) = self
                    .work_repo
                    .get_formal_work(parent_ref)
                    .await
                    .map_err(Self::map_repository_error)?
                else {
                    return self.rollback_and_err(uow, ApplicationError::NotFound).await;
                };
                let project_ref = match self.project_ref_for_backlog(&parent_work.backlog_id).await
                {
                    Ok(project_ref) => project_ref,
                    Err(error) => return self.rollback_and_err(uow, error).await,
                };
                (project_ref, Some(parent_work.assignee_ref.clone()))
            }
        };

        let evidence_ref = match request.target {
            WorkLifecycleTarget::Completed => {
                let Some(evidence_ref) = request.evidence_ref.clone() else {
                    return self
                        .rollback_and_err(uow, ApplicationError::InvalidRequest)
                        .await;
                };
                let resolved = match self.evidence_resolver.resolve_evidence(evidence_ref).await {
                    Ok(resolution) => resolution,
                    Err(error) => {
                        return self
                            .rollback_and_err(uow, Self::map_port_error(error))
                            .await;
                    }
                };
                if let Err(error) = CompletionEvidencePolicy::assert_completion_evidence(
                    request.work_ref.clone(),
                    resolved.evidence_ref.clone(),
                ) {
                    return self
                        .rollback_and_err(uow, Self::map_domain_error(error))
                        .await;
                }
                Some(resolved.evidence_ref)
            }
            _ => request.evidence_ref.clone(),
        };

        let transition_result = match &mut record {
            FormalWorkRecord::WorkItem(work_item) => work_item.transition_lifecycle(
                request.target,
                request.reason,
                evidence_ref.clone(),
                actor.clone(),
            ),
            FormalWorkRecord::ChildWorkItem(child) => child.transition_lifecycle(
                request.target,
                request.reason,
                evidence_ref,
                actor.clone(),
            ),
        };
        if let Err(error) = transition_result {
            return self
                .rollback_and_err(uow, Self::map_domain_error(error))
                .await;
        }

        let work_ref = record.formal_work_ref();
        let work_state = record.work_state();
        let version = match self
            .work_repo
            .save_formal_work(record, request.expected_version, &uow)
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
                WorkTruthChange::WorkItemChanged(work_ref.clone()),
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
                WorkTruthChange::WorkItemChanged(work_ref.clone()),
                &envelope.metadata.request,
                &uow,
            )
            .await
        {
            Ok(outbox_id) => outbox_id,
            Err(error) => return self.rollback_and_err(uow, error).await,
        };

        let mut stale_views = vec![DerivedWorkViewRef::project_board(project_ref)];
        if let Some(member_ref) = member_ref {
            stale_views.push(DerivedWorkViewRef::member_work(member_ref));
        }
        if let Err(error) = self
            .projection_repo
            .mark_stale(stale_views, Self::work_cursor(&work_ref, version), &uow)
            .await
        {
            return self
                .rollback_and_err(uow, Self::map_repository_error(error))
                .await;
        }

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let result = WorkItemCommandResult {
            work_ref,
            work_state,
            receipt: WorkCommandReceipt::applied(
                result_ref.clone(),
                Some(trace_id),
                vec![outbox_id],
                version,
            ),
        };
        self.finish_work_command(result_ref, result.clone(), reservation, uow)
            .await?;
        Ok(result)
    }

    async fn project_ref_for_backlog(
        &self,
        backlog_id: &work_contracts::BacklogId,
    ) -> Result<ProjectRef, ApplicationError> {
        let backlog_ref = work_contracts::BacklogRef {
            backlog_id: backlog_id.clone(),
        };
        let backlog = self
            .backlog_repo
            .get(backlog_ref)
            .await
            .map_err(Self::map_repository_error)?
            .ok_or(ApplicationError::NotFound)?;
        Ok(ProjectRef {
            project_id: backlog.project_id,
        })
    }

    async fn load_duplicate_work_result(
        &self,
        operation: OperationName,
        result_ref: ApplicationResultRef,
    ) -> Result<WorkItemCommandResult, ApplicationError> {
        let stored = self
            .command_results
            .get_result(result_ref)
            .await
            .map_err(Self::map_repository_error)?;
        match stored.and_then(|result| result.into_work_item_result(&operation)) {
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
                self.load_duplicate_work_result(operation.clone(), result_ref)
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

    async fn finish_work_command(
        &self,
        result_ref: ApplicationResultRef,
        result: WorkItemCommandResult,
        reservation: IdempotencyRecord,
        uow: UnitOfWorkHandle,
    ) -> Result<(), ApplicationError> {
        self.command_results
            .save_result(
                result_ref.clone(),
                StoredCommandResult::WorkItem(result),
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
    fn map_repository_error(error: RepositoryError) -> ApplicationError {
        match error {
            RepositoryError::NotFound => ApplicationError::NotFound,
            RepositoryError::VersionConflict => ApplicationError::VersionConflict,
            RepositoryError::TransactionRejected => ApplicationError::TemporarilyUnavailable,
            RepositoryError::StoreUnavailable => ApplicationError::TemporarilyUnavailable,
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
}

enum ReservationOutcome {
    Reserved((UnitOfWorkHandle, IdempotencyRecord)),
    Duplicate(WorkItemCommandResult),
}

fn matches_terminal(work_state: work_contracts::WorkItemState) -> bool {
    matches!(
        work_state,
        work_contracts::WorkItemState::Completed
            | work_contracts::WorkItemState::Cancelled
            | work_contracts::WorkItemState::Superseded
    )
}
