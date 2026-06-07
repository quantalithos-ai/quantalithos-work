//! Outbox publication services for Work operations jobs.

use core_contracts::metadata::{OperationName, PageRequest, Version};

use crate::results::{JobResultRepository, StoredJobResult};
use crate::{
    ApplicationError, AuditRepository, BacklogRepository, DependencyRepository, IdGeneratorPort,
    IdempotencyError, IdempotencyRecord, IdempotencyRepository, IdempotencyReservation,
    IterationRepository, PortError, ProjectMemberRepository, ProjectRepository,
    ProjectionRepository, PromoteRepository, RepositoryError, RequestDigest, UnitOfWork,
    UnitOfWorkError, UnitOfWorkHandle, Versioned, WorkItemRepository, WorkOutboxPublisherPort,
    WorkOutboxRepository,
};
use work_contracts::events::EventSchemaVersion;
use work_contracts::{
    ApplicationResultRef, DerivedWorkViewChangedEvent, OutboxFailureReason,
    OutboxFailureReasonKind, OutboxPublicationRef, ProjectChangedEvent, ProjectMemberChangedEvent,
    PromoteResultRecordedEvent, PublishWorkOutboxJobInput, SafeSummaryText,
    WorkBlockerChangedEvent, WorkDependencyChangedEvent, WorkItemChangedEvent, WorkJobFailureRef,
    WorkJobReport, WorkOutboundEventEnvelope, WorkOutboundPublication, WorkOutboxEventKind,
    WorkOutboxId, WorkOutboxSourceRef, WorkTraceAvailableEvent,
};
use work_domain::WorkOutboxRecord;

/// Coordinates pending outbox publication and duplicate replay for operations jobs.
pub struct WorkOutboxPublishService<PJ, PM, W, PR, B, D, ITR, A, O, PROJ, PUB, U, IDEM, JR, IDS> {
    /// Project truth repository.
    pub project_repo: PJ,
    /// Project-member truth repository.
    pub member_repo: PM,
    /// Formal work repository.
    pub work_repo: W,
    /// Promote result repository.
    pub promote_repo: PR,
    /// Backlog truth repository.
    pub backlog_repo: B,
    /// Dependency and blocker repository.
    pub dependency_repo: D,
    /// Iteration repository.
    pub iteration_repo: ITR,
    /// Audit repository.
    pub audit_repo: A,
    /// Work outbox repository.
    pub outbox_repo: O,
    /// Projection freshness repository.
    pub projection_repo: PROJ,
    /// Runtime publisher seam.
    pub publisher: PUB,
    /// Unit-of-work factory.
    pub unit_of_work: U,
    /// Shared idempotency repository.
    pub idempotency: IDEM,
    /// Job result repository for duplicate replay.
    pub job_results: JR,
    /// Deterministic result id generator.
    pub ids: IDS,
}

impl<PJ, PM, W, PR, B, D, ITR, A, O, PROJ, PUB, U, IDEM, JR, IDS>
    WorkOutboxPublishService<PJ, PM, W, PR, B, D, ITR, A, O, PROJ, PUB, U, IDEM, JR, IDS>
where
    PJ: ProjectRepository,
    PM: ProjectMemberRepository,
    W: WorkItemRepository,
    PR: PromoteRepository,
    B: BacklogRepository,
    D: DependencyRepository,
    ITR: IterationRepository,
    A: AuditRepository,
    O: WorkOutboxRepository,
    PROJ: ProjectionRepository,
    PUB: WorkOutboxPublisherPort,
    U: UnitOfWork,
    IDEM: IdempotencyRepository,
    JR: JobResultRepository,
    IDS: IdGeneratorPort,
{
    /// Publishes one pending outbox page and stores a duplicate-replayable job report.
    pub async fn publish_outbox(
        &self,
        input: PublishWorkOutboxJobInput,
    ) -> Result<WorkJobReport, ApplicationError> {
        let operation = OperationName::new("publish_work_outbox");
        let (job_uow, reservation) = match self.reserve_job(&operation, &input).await? {
            JobReservationOutcome::Reserved(parts) => parts,
            JobReservationOutcome::Duplicate(report) => return Ok(report),
        };
        let pending = match self.outbox_repo.list_pending(input.page.clone()).await {
            Ok(pending) => pending,
            Err(error) => {
                self.unit_of_work
                    .rollback(job_uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                return Err(Self::map_repository_error(error));
            }
        };

        let mut changed_count = 0_u64;
        let mut failed_refs = Vec::new();

        for versioned in pending.items.clone() {
            let outbox_id = versioned.record.outbox_id.clone();
            let expected_version = versioned.version;
            let item_outcome = match self.publish_one(versioned).await {
                Ok(outcome) => outcome,
                Err(ApplicationError::InvalidOutboxSource) => {
                    let reason = OutboxFailureReason {
                        reason_kind: OutboxFailureReasonKind::Terminal,
                        message: SafeSummaryText("InvalidOutboxSource".to_owned()),
                    };
                    match self
                        .mark_failed(outbox_id.clone(), reason, expected_version)
                        .await
                    {
                        Ok(outcome) => outcome,
                        Err(error) => {
                            self.unit_of_work
                                .rollback(job_uow)
                                .await
                                .map_err(Self::map_uow_error_rollback)?;
                            return Err(error);
                        }
                    }
                }
                Err(error) => {
                    self.unit_of_work
                        .rollback(job_uow)
                        .await
                        .map_err(Self::map_uow_error_rollback)?;
                    return Err(error);
                }
            };
            match item_outcome {
                PublishOneOutcome::Published => {
                    changed_count += 1;
                }
                PublishOneOutcome::Failed => {
                    changed_count += 1;
                    failed_refs.push(Self::outbox_failed_ref(outbox_id));
                }
                PublishOneOutcome::AlreadyHandled => {
                    failed_refs.push(Self::outbox_failed_ref(outbox_id));
                }
            }
        }

        let result_ref = ApplicationResultRef::for_operation(
            operation,
            self.ids.next_result_id().map_err(Self::map_port_error)?,
        );
        let report = WorkJobReport {
            job_run_id: input.metadata.job_run_id,
            receipt: Some(work_contracts::WorkCommandReceipt::applied(
                result_ref.clone(),
                None,
                Vec::new(),
                0,
            )),
            scanned_count: pending.items.len() as u64,
            changed_count,
            failed_refs,
        };

        if let Err(error) = self
            .job_results
            .save_report(
                result_ref.clone(),
                StoredJobResult::WorkJob(report.clone()),
                &job_uow,
            )
            .await
        {
            self.unit_of_work
                .rollback(job_uow)
                .await
                .map_err(Self::map_uow_error_rollback)?;
            return Err(Self::map_repository_error(error));
        }
        if let Err(error) = self
            .idempotency
            .complete(
                IdempotencyReservation::Reserved(reservation),
                result_ref,
                &job_uow,
            )
            .await
        {
            self.unit_of_work
                .rollback(job_uow)
                .await
                .map_err(Self::map_uow_error_rollback)?;
            return Err(Self::map_idempotency_error(error));
        }
        self.unit_of_work
            .commit(job_uow)
            .await
            .map_err(Self::map_uow_error_commit)?;

        Ok(report)
    }

    async fn publish_one(
        &self,
        versioned: Versioned<WorkOutboxRecord>,
    ) -> Result<PublishOneOutcome, ApplicationError> {
        let expected_version = versioned.version;
        let record = versioned.record;
        let publication = self.build_publication(&record).await?;
        match self.publisher.publish(publication).await {
            Ok(publication_ref) => {
                self.mark_published(record.outbox_id, publication_ref, expected_version)
                    .await
            }
            Err(error) => {
                let reason = Self::failure_reason_from_port(error);
                self.mark_failed(record.outbox_id, reason, expected_version)
                    .await
            }
        }
    }

    async fn build_publication(
        &self,
        record: &WorkOutboxRecord,
    ) -> Result<WorkOutboundPublication, ApplicationError> {
        match (&record.event_kind, &record.source_ref) {
            (
                WorkOutboxEventKind::ProjectChanged,
                WorkOutboxSourceRef::Project {
                    project_ref,
                    reason,
                },
            ) => {
                let project = self
                    .project_repo
                    .get(project_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                Ok(WorkOutboundPublication::ProjectChanged(Self::envelope(
                    record,
                    ProjectChangedEvent {
                        project_ref: project.project_ref(),
                        lifecycle_state: project.lifecycle_state,
                        reason: reason.clone(),
                    },
                )))
            }
            (
                WorkOutboxEventKind::BacklogChanged,
                WorkOutboxSourceRef::Backlog {
                    backlog_ref,
                    reason,
                },
            ) => {
                let backlog = self
                    .backlog_repo
                    .get(backlog_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                Ok(WorkOutboundPublication::BacklogChanged(Self::envelope(
                    record,
                    work_contracts::BacklogChangedEvent {
                        backlog_ref: backlog.backlog_ref(),
                        project_ref: work_contracts::ProjectRef {
                            project_id: backlog.project_id,
                        },
                        backlog_state: backlog.backlog_state,
                        reason: reason.clone(),
                    },
                )))
            }
            (
                WorkOutboxEventKind::ProjectMemberChanged,
                WorkOutboxSourceRef::ProjectMember(project_member_ref),
            ) => {
                let member = self
                    .member_repo
                    .get(project_member_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                Ok(WorkOutboundPublication::ProjectMemberChanged(
                    Self::envelope(
                        record,
                        ProjectMemberChangedEvent {
                            project_member_ref: member.project_member_ref(),
                            project_ref: work_contracts::ProjectRef {
                                project_id: member.project_id,
                            },
                            member_ref: member.member_ref,
                            responsibility_state: member.responsibility_state,
                        },
                    ),
                ))
            }
            (WorkOutboxEventKind::WorkItemChanged, WorkOutboxSourceRef::FormalWork(work_ref)) => {
                let work = self
                    .work_repo
                    .get_formal_work(work_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                let scope = self
                    .work_repo
                    .get_formal_work_scope(work_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                let payload = match work {
                    crate::FormalWorkRecord::WorkItem(work_item) => WorkItemChangedEvent {
                        work_ref: work_item.formal_work_ref(),
                        project_ref: scope.project_ref,
                        work_state: work_item.work_state,
                        source_ref: None,
                        evidence_ref: work_item.completion_ref,
                    },
                    crate::FormalWorkRecord::ChildWorkItem(child) => WorkItemChangedEvent {
                        work_ref: child.formal_work_ref(),
                        project_ref: scope.project_ref,
                        work_state: child.work_state,
                        source_ref: Some(child.source_ref),
                        evidence_ref: child.completion_ref,
                    },
                };
                Ok(WorkOutboundPublication::WorkItemChanged(Self::envelope(
                    record, payload,
                )))
            }
            (
                WorkOutboxEventKind::PromoteResultRecorded,
                WorkOutboxSourceRef::PromoteResult(promote_result_ref),
            ) => {
                let result = self
                    .promote_repo
                    .get(promote_result_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                Ok(WorkOutboundPublication::PromoteResultRecorded(
                    Self::envelope(
                        record,
                        PromoteResultRecordedEvent {
                            promote_result_ref: result.promote_result_ref(),
                            source_ref: result.source_ref,
                            result_state: result.result_state,
                            created_work_ref: result.created_work_ref,
                        },
                    ),
                ))
            }
            (
                WorkOutboxEventKind::WorkDependencyChanged,
                WorkOutboxSourceRef::Dependency(dependency_ref),
            ) => {
                let dependency = self
                    .dependency_repo
                    .get_dependency(dependency_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                Ok(WorkOutboundPublication::WorkDependencyChanged(
                    Self::envelope(
                        record,
                        WorkDependencyChangedEvent {
                            dependency_ref: dependency.dependency_ref(),
                            upstream_work_ref: dependency.upstream_work_ref,
                            downstream_work_ref: dependency.downstream_work_ref,
                            dependency_state: dependency.dependency_state,
                        },
                    ),
                ))
            }
            (
                WorkOutboxEventKind::WorkBlockerChanged,
                WorkOutboxSourceRef::Blocker(blocker_ref),
            ) => {
                let blocker = self
                    .dependency_repo
                    .get_blocker(blocker_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                Ok(WorkOutboundPublication::WorkBlockerChanged(Self::envelope(
                    record,
                    WorkBlockerChangedEvent {
                        blocker_ref: blocker.blocker_ref(),
                        blocked_work_ref: blocker.blocked_work_ref,
                        blocker_state: blocker.blocker_state,
                        evidence_ref: blocker.resolved_evidence_ref,
                    },
                )))
            }
            (
                WorkOutboxEventKind::IterationChanged,
                WorkOutboxSourceRef::Iteration(iteration_ref),
            ) => {
                let iteration = self
                    .iteration_repo
                    .get_iteration(iteration_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                let commitment = self
                    .iteration_repo
                    .get_commitment(iteration_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?;
                Ok(WorkOutboundPublication::IterationChanged(Self::envelope(
                    record,
                    work_contracts::IterationChangedEvent {
                        iteration_ref: iteration.iteration_ref(),
                        project_ref: work_contracts::ProjectRef {
                            project_id: iteration.project_id,
                        },
                        iteration_state: iteration.iteration_state,
                        commitment_state: commitment.as_ref().map(|c| c.commitment_state),
                        affected_work_refs: commitment
                            .map(|c| c.committed_work_refs.refs)
                            .unwrap_or_default(),
                    },
                )))
            }
            (
                WorkOutboxEventKind::WorkTraceAvailable,
                WorkOutboxSourceRef::TraceAvailable {
                    trace_id,
                    handoff_ref,
                },
            ) => {
                let trace = self
                    .audit_repo
                    .get_trace_record(trace_id.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                Ok(WorkOutboundPublication::WorkTraceAvailable(Self::envelope(
                    record,
                    WorkTraceAvailableEvent {
                        subject_ref: trace.subject_ref,
                        trace_id: trace.trace_id,
                        handoff_ref: handoff_ref.clone(),
                    },
                )))
            }
            (
                WorkOutboxEventKind::DerivedWorkViewChanged,
                WorkOutboxSourceRef::DerivedView(view_ref),
            ) => {
                let state = self
                    .projection_repo
                    .get_freshness_state(view_ref.clone())
                    .await
                    .map_err(Self::map_repository_error)?
                    .ok_or(ApplicationError::InvalidOutboxSource)?;
                Ok(WorkOutboundPublication::DerivedWorkViewChanged(
                    Self::envelope(
                        record,
                        DerivedWorkViewChangedEvent {
                            view_ref: state.view_ref,
                            freshness_state: state.freshness_state,
                            source_cursor: state.source_cursor,
                        },
                    ),
                ))
            }
            _ => Err(ApplicationError::InvalidOutboxSource),
        }
    }

    fn envelope<T>(record: &WorkOutboxRecord, payload: T) -> WorkOutboundEventEnvelope<T> {
        WorkOutboundEventEnvelope {
            outbox_id: record.outbox_id.clone(),
            event_version: EventSchemaVersion::v1(),
            trace_context_ref: record.trace_context_ref.clone(),
            occurred_at: record.occurred_at.clone(),
            payload,
        }
    }

    async fn mark_published(
        &self,
        outbox_id: WorkOutboxId,
        publication_ref: OutboxPublicationRef,
        expected_version: Version,
    ) -> Result<PublishOneOutcome, ApplicationError> {
        let uow = self
            .unit_of_work
            .begin()
            .await
            .map_err(Self::map_uow_error_begin)?;
        match self
            .outbox_repo
            .mark_published(outbox_id, publication_ref, expected_version, &uow)
            .await
        {
            Ok(_) => {
                self.unit_of_work
                    .commit(uow)
                    .await
                    .map_err(Self::map_uow_error_commit)?;
                Ok(PublishOneOutcome::Published)
            }
            Err(RepositoryError::VersionConflict) => {
                self.unit_of_work
                    .rollback(uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                Ok(PublishOneOutcome::AlreadyHandled)
            }
            Err(error) => {
                self.unit_of_work
                    .rollback(uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                Err(Self::map_repository_error(error))
            }
        }
    }

    async fn mark_failed(
        &self,
        outbox_id: WorkOutboxId,
        reason: OutboxFailureReason,
        expected_version: Version,
    ) -> Result<PublishOneOutcome, ApplicationError> {
        let uow = self
            .unit_of_work
            .begin()
            .await
            .map_err(Self::map_uow_error_begin)?;
        match self
            .outbox_repo
            .mark_failed(outbox_id, reason, expected_version, &uow)
            .await
        {
            Ok(_) => {
                self.unit_of_work
                    .commit(uow)
                    .await
                    .map_err(Self::map_uow_error_commit)?;
                Ok(PublishOneOutcome::Failed)
            }
            Err(RepositoryError::VersionConflict) => {
                self.unit_of_work
                    .rollback(uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                Ok(PublishOneOutcome::AlreadyHandled)
            }
            Err(error) => {
                self.unit_of_work
                    .rollback(uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                Err(Self::map_repository_error(error))
            }
        }
    }

    async fn reserve_job(
        &self,
        operation: &OperationName,
        input: &PublishWorkOutboxJobInput,
    ) -> Result<JobReservationOutcome, ApplicationError> {
        let key = input
            .metadata
            .command_metadata
            .request
            .idempotency_key
            .clone()
            .ok_or(ApplicationError::InvalidRequest)?;
        let digest = Self::job_digest(operation, &input.page)?;
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
            Ok(IdempotencyReservation::Reserved(record)) => {
                Ok(JobReservationOutcome::Reserved((uow, record)))
            }
            Ok(IdempotencyReservation::Duplicate(result_ref)) => {
                self.unit_of_work
                    .rollback(uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                self.load_duplicate_work_job_report(operation.clone(), result_ref)
                    .await
                    .map(JobReservationOutcome::Duplicate)
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
            Err(error) => {
                self.unit_of_work
                    .rollback(uow)
                    .await
                    .map_err(Self::map_uow_error_rollback)?;
                Err(Self::map_idempotency_error(error))
            }
        }
    }

    async fn load_duplicate_work_job_report(
        &self,
        operation: OperationName,
        result_ref: ApplicationResultRef,
    ) -> Result<WorkJobReport, ApplicationError> {
        let stored = self
            .job_results
            .get_report(result_ref)
            .await
            .map_err(Self::map_repository_error)?;
        match stored.and_then(|result| result.into_work_job_report(&operation)) {
            Some(report) => Ok(report.with_duplicate_receipt()),
            None => Err(ApplicationError::DuplicateResultMissing),
        }
    }

    fn job_digest(
        operation: &OperationName,
        page: &PageRequest,
    ) -> Result<RequestDigest, ApplicationError> {
        serde_json::to_string(&serde_json::json!({
            "operation": operation.as_str(),
            "page_limit": page.limit,
            "page_token": page.page_token.as_ref().map(|token| token.as_str()),
        }))
        .map(RequestDigest)
        .map_err(|_| ApplicationError::InvalidRequest)
    }

    fn outbox_failed_ref(outbox_id: WorkOutboxId) -> WorkJobFailureRef {
        WorkJobFailureRef::WorkOutbox(outbox_id)
    }

    fn failure_reason_from_port(error: PortError) -> OutboxFailureReason {
        let reason_kind = match error {
            PortError::Unavailable => OutboxFailureReasonKind::Retryable,
            PortError::NotFound | PortError::Rejected | PortError::InvalidResponse => {
                OutboxFailureReasonKind::Terminal
            }
        };
        OutboxFailureReason {
            reason_kind,
            message: SafeSummaryText(format!("{error:?}")),
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
            PortError::NotFound | PortError::Rejected => {
                ApplicationError::ExternalReferenceUnresolved
            }
            PortError::InvalidResponse => ApplicationError::InvalidOutboxSource,
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

enum JobReservationOutcome {
    Reserved((UnitOfWorkHandle, IdempotencyRecord)),
    Duplicate(WorkJobReport),
}

enum PublishOneOutcome {
    Published,
    Failed,
    AlreadyHandled,
}
