//! Inbound event consumer services for Work.

use core_contracts::metadata::{IdempotencyKey, OperationName, PageRequest};
use serde::Serialize;

use crate::{
    IdempotencyError, IdempotencyRepository, IdempotencyReservation, PortError,
    ProjectMemberRepository, ProjectionRepository, PromoteRepository, ReferenceSnapshotRepository,
    RepositoryError, RequestDigest, SourceWorkResolverPort, UnitOfWork, UnitOfWorkError,
    UnitOfWorkHandle,
};
use work_contracts::{
    ApplicationResultRef, ArtifactEvidenceChangedPayload, ConversationWorkContextChangedPayload,
    EventSchemaVersion, GovernanceDecisionChangedPayload, IdentityMemberChangedPayload,
    MethodDefinitionChangedPayload, ProcessTimingChangedPayload, ResultId,
    RuntimePromoteRequestedPayload, WorkInboundEventEnvelope, WorkTruthCursor,
};
use work_domain::{
    MemberCapabilitySnapshot, MethodDefinitionSnapshot, PendingPromoteIntake,
    ReferenceResolutionState, ReferenceStaleReason, WorkTruthPolicy,
};

const AFFECTED_VIEWS_PAGE_LIMIT: u32 = 100;

/// Internal runtime result used by inbound event consumers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsumerDisposition {
    /// Event was accepted and fully applied.
    Ack,
    /// Event was a same-digest duplicate and no new state was written.
    AckDuplicate,
    /// Event was accepted only as marker/snapshot writes.
    AckWithMarker,
    /// Event failed due to a retryable transient problem.
    Retry,
    /// Event is malformed, unsupported, or permanently conflicted.
    DeadLetter,
}

/// Coordinates inbound marker and snapshot writes inside one application transaction.
pub struct WorkInboundConsumerService<PM, RS, PR, P, U, S, IDEM> {
    /// Project-member truth repository.
    pub member_repo: PM,
    /// Reference snapshot repository.
    pub reference_repo: RS,
    /// Projection freshness repository.
    pub projection_repo: PR,
    /// Promote repository for pending runtime intake markers.
    pub promote_repo: P,
    /// Local unit-of-work factory.
    pub unit_of_work: U,
    /// Source resolver used by runtime promote events.
    pub source_resolver: S,
    /// Shared idempotency repository.
    pub idempotency: IDEM,
}

impl<PM, RS, PR, P, U, S, IDEM> WorkInboundConsumerService<PM, RS, PR, P, U, S, IDEM>
where
    PM: ProjectMemberRepository,
    RS: ReferenceSnapshotRepository,
    PR: ProjectionRepository,
    P: PromoteRepository,
    U: UnitOfWork,
    S: SourceWorkResolverPort,
    IDEM: IdempotencyRepository,
{
    /// Consumes `identity.member.changed.v1`.
    pub async fn consume_identity_member_changed(
        &self,
        envelope: WorkInboundEventEnvelope<IdentityMemberChangedPayload>,
    ) -> ConsumerDisposition {
        if !self.validate_event_version(&envelope.event_version, "identity.member.changed.v1") {
            return ConsumerDisposition::DeadLetter;
        }
        if envelope.payload.member_ref.0.trim().is_empty()
            || envelope.payload.capability_refs.refs.is_empty()
        {
            return ConsumerDisposition::DeadLetter;
        }

        let reservation = match self
            .reserve_inbound(
                "consume_identity_member_changed",
                "identity.member.changed.v1",
                &envelope,
            )
            .await
        {
            ReserveInboundOutcome::Reserved(parts) => parts,
            ReserveInboundOutcome::AckDuplicate => return ConsumerDisposition::AckDuplicate,
            ReserveInboundOutcome::Retry => return ConsumerDisposition::Retry,
            ReserveInboundOutcome::DeadLetter => return ConsumerDisposition::DeadLetter,
        };

        let reference_ref =
            work_contracts::ExternalReferenceRef::from_member(envelope.payload.member_ref.clone());
        let mut state = ReferenceResolutionState::unresolved(reference_ref);
        if state.mark_resolved(envelope.occurred_at.clone()).is_err() {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                .await;
        }
        let snapshot = match MemberCapabilitySnapshot::from_identity(
            envelope.payload.member_ref.clone(),
            envelope.payload.capability_refs.clone(),
        ) {
            Ok(snapshot) => snapshot,
            Err(_) => {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                    .await;
            }
        };
        if self
            .reference_repo
            .save_reference_state(state, None, &reservation.uow)
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }
        if self
            .reference_repo
            .save_member_snapshot(snapshot, None, &reservation.uow)
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }

        let affected_page = Self::affected_page();
        let affected_members = match self
            .member_repo
            .list_by_member(envelope.payload.member_ref.clone(), affected_page.clone())
            .await
        {
            Ok(page) => page,
            Err(_) => {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                    .await;
            }
        };
        let affected_views = match self
            .projection_repo
            .list_views_affected_by_member(envelope.payload.member_ref.clone(), affected_page)
            .await
        {
            Ok(page) => page,
            Err(_) => {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                    .await;
            }
        };
        let disposition = if !affected_members.items.is_empty() && !affected_views.items.is_empty()
        {
            if self
                .projection_repo
                .mark_stale(
                    affected_views.items,
                    truth_cursor_from_event("identity.member.changed.v1", &envelope),
                    &reservation.uow,
                )
                .await
                .is_err()
            {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                    .await;
            }
            ConsumerDisposition::Ack
        } else {
            ConsumerDisposition::AckWithMarker
        };

        self.complete_inbound(
            reservation,
            "consume_identity_member_changed",
            &envelope,
            disposition,
        )
        .await
    }

    /// Consumes `method.definition.changed.v1`.
    pub async fn consume_method_definition_changed(
        &self,
        envelope: WorkInboundEventEnvelope<MethodDefinitionChangedPayload>,
    ) -> ConsumerDisposition {
        if !self.validate_event_version(&envelope.event_version, "method.definition.changed.v1") {
            return ConsumerDisposition::DeadLetter;
        }
        if envelope.payload.definition_ref.0.trim().is_empty() {
            return ConsumerDisposition::DeadLetter;
        }

        let reservation = match self
            .reserve_inbound(
                "consume_method_definition_changed",
                "method.definition.changed.v1",
                &envelope,
            )
            .await
        {
            ReserveInboundOutcome::Reserved(parts) => parts,
            ReserveInboundOutcome::AckDuplicate => return ConsumerDisposition::AckDuplicate,
            ReserveInboundOutcome::Retry => return ConsumerDisposition::Retry,
            ReserveInboundOutcome::DeadLetter => return ConsumerDisposition::DeadLetter,
        };

        let snapshot = match MethodDefinitionSnapshot::from_method_library(
            envelope.payload.definition_ref.clone(),
            envelope.payload.definition_kind,
        ) {
            Ok(snapshot) => snapshot,
            Err(_) => {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                    .await;
            }
        };
        let mut state = ReferenceResolutionState::unresolved(
            work_contracts::ExternalReferenceRef::from_method_definition(
                envelope.payload.definition_ref.clone(),
            ),
        );
        if state.mark_resolved(envelope.occurred_at.clone()).is_err() {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                .await;
        }
        if self
            .reference_repo
            .save_method_snapshot(snapshot, None, &reservation.uow)
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }
        if self
            .reference_repo
            .save_reference_state(state, None, &reservation.uow)
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }

        let affected_views = match self
            .projection_repo
            .list_views_affected_by_method(
                envelope.payload.definition_ref.clone(),
                Self::affected_page(),
            )
            .await
        {
            Ok(page) => page,
            Err(_) => {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                    .await;
            }
        };
        let disposition = if affected_views.items.is_empty() {
            ConsumerDisposition::AckWithMarker
        } else {
            if self
                .projection_repo
                .mark_stale(
                    affected_views.items,
                    truth_cursor_from_event("method.definition.changed.v1", &envelope),
                    &reservation.uow,
                )
                .await
                .is_err()
            {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                    .await;
            }
            ConsumerDisposition::Ack
        };

        self.complete_inbound(
            reservation,
            "consume_method_definition_changed",
            &envelope,
            disposition,
        )
        .await
    }

    /// Consumes `conversation.work_context.changed.v1`.
    pub async fn consume_conversation_work_context_changed(
        &self,
        envelope: WorkInboundEventEnvelope<ConversationWorkContextChangedPayload>,
    ) -> ConsumerDisposition {
        if !self.validate_event_version(
            &envelope.event_version,
            "conversation.work_context.changed.v1",
        ) {
            return ConsumerDisposition::DeadLetter;
        }
        if envelope
            .payload
            .source_ref
            .external_ref
            .external_id
            .trim()
            .is_empty()
        {
            return ConsumerDisposition::DeadLetter;
        }

        let reservation = match self
            .reserve_inbound(
                "consume_conversation_work_context_changed",
                "conversation.work_context.changed.v1",
                &envelope,
            )
            .await
        {
            ReserveInboundOutcome::Reserved(parts) => parts,
            ReserveInboundOutcome::AckDuplicate => return ConsumerDisposition::AckDuplicate,
            ReserveInboundOutcome::Retry => return ConsumerDisposition::Retry,
            ReserveInboundOutcome::DeadLetter => return ConsumerDisposition::DeadLetter,
        };

        let source_ref = envelope.payload.source_ref.clone();
        let mut state = ReferenceResolutionState::unresolved(
            work_contracts::ExternalReferenceRef::from_source_work(source_ref.clone()),
        );
        if source_ref.source_digest == envelope.payload.source_digest
            && state.mark_resolved(envelope.occurred_at.clone()).is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                .await;
        }
        if self
            .reference_repo
            .save_reference_state(state, None, &reservation.uow)
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }

        self.complete_inbound(
            reservation,
            "consume_conversation_work_context_changed",
            &envelope,
            ConsumerDisposition::AckWithMarker,
        )
        .await
    }

    /// Consumes `process.timing.changed.v1`.
    pub async fn consume_process_timing_changed(
        &self,
        envelope: WorkInboundEventEnvelope<ProcessTimingChangedPayload>,
    ) -> ConsumerDisposition {
        if !self.validate_event_version(&envelope.event_version, "process.timing.changed.v1") {
            return ConsumerDisposition::DeadLetter;
        }
        if envelope.payload.timebox_ref.0.trim().is_empty() {
            return ConsumerDisposition::DeadLetter;
        }

        let reservation = match self
            .reserve_inbound(
                "consume_process_timing_changed",
                "process.timing.changed.v1",
                &envelope,
            )
            .await
        {
            ReserveInboundOutcome::Reserved(parts) => parts,
            ReserveInboundOutcome::AckDuplicate => return ConsumerDisposition::AckDuplicate,
            ReserveInboundOutcome::Retry => return ConsumerDisposition::Retry,
            ReserveInboundOutcome::DeadLetter => return ConsumerDisposition::DeadLetter,
        };

        let mut state = ReferenceResolutionState::unresolved(
            work_contracts::ExternalReferenceRef::from_process_timebox(
                envelope.payload.timebox_ref.clone(),
            ),
        );
        if state.mark_resolved(envelope.occurred_at.clone()).is_err() {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                .await;
        }
        if self
            .reference_repo
            .save_reference_state(state, None, &reservation.uow)
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }

        self.complete_inbound(
            reservation,
            "consume_process_timing_changed",
            &envelope,
            ConsumerDisposition::AckWithMarker,
        )
        .await
    }

    /// Consumes `governance.decision.changed.v1`.
    pub async fn consume_governance_decision_changed(
        &self,
        envelope: WorkInboundEventEnvelope<GovernanceDecisionChangedPayload>,
    ) -> ConsumerDisposition {
        if !self.validate_event_version(&envelope.event_version, "governance.decision.changed.v1") {
            return ConsumerDisposition::DeadLetter;
        }
        if envelope
            .payload
            .source_ref
            .external_ref
            .external_id
            .trim()
            .is_empty()
            && envelope.payload.evidence_ref.is_none()
        {
            return ConsumerDisposition::DeadLetter;
        }

        let reservation = match self
            .reserve_inbound(
                "consume_governance_decision_changed",
                "governance.decision.changed.v1",
                &envelope,
            )
            .await
        {
            ReserveInboundOutcome::Reserved(parts) => parts,
            ReserveInboundOutcome::AckDuplicate => return ConsumerDisposition::AckDuplicate,
            ReserveInboundOutcome::Retry => return ConsumerDisposition::Retry,
            ReserveInboundOutcome::DeadLetter => return ConsumerDisposition::DeadLetter,
        };

        if !envelope
            .payload
            .source_ref
            .external_ref
            .external_id
            .trim()
            .is_empty()
            && save_resolved_reference(
                &self.reference_repo,
                work_contracts::ExternalReferenceRef::from_source_work(
                    envelope.payload.source_ref.clone(),
                ),
                envelope.occurred_at.clone(),
                &reservation.uow,
            )
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }
        if let Some(evidence_ref) = envelope.payload.evidence_ref.clone() {
            if save_resolved_reference(
                &self.reference_repo,
                work_contracts::ExternalReferenceRef::from_evidence(evidence_ref),
                envelope.occurred_at.clone(),
                &reservation.uow,
            )
            .await
            .is_err()
            {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                    .await;
            }
        }

        self.complete_inbound(
            reservation,
            "consume_governance_decision_changed",
            &envelope,
            ConsumerDisposition::AckWithMarker,
        )
        .await
    }

    /// Consumes `artifact.evidence.changed.v1`.
    pub async fn consume_artifact_evidence_changed(
        &self,
        envelope: WorkInboundEventEnvelope<ArtifactEvidenceChangedPayload>,
    ) -> ConsumerDisposition {
        if !self.validate_event_version(&envelope.event_version, "artifact.evidence.changed.v1") {
            return ConsumerDisposition::DeadLetter;
        }
        if envelope
            .payload
            .evidence_ref
            .external_ref
            .external_id
            .trim()
            .is_empty()
        {
            return ConsumerDisposition::DeadLetter;
        }

        let reservation = match self
            .reserve_inbound(
                "consume_artifact_evidence_changed",
                "artifact.evidence.changed.v1",
                &envelope,
            )
            .await
        {
            ReserveInboundOutcome::Reserved(parts) => parts,
            ReserveInboundOutcome::AckDuplicate => return ConsumerDisposition::AckDuplicate,
            ReserveInboundOutcome::Retry => return ConsumerDisposition::Retry,
            ReserveInboundOutcome::DeadLetter => return ConsumerDisposition::DeadLetter,
        };

        let evidence_ref = envelope.payload.evidence_ref.clone();
        let mut state = ReferenceResolutionState::unresolved(
            work_contracts::ExternalReferenceRef::from_evidence(evidence_ref.clone()),
        );
        match evidence_ref.verified_state {
            work_contracts::EvidenceVerifiedState::Verified => {
                if state.mark_resolved(envelope.occurred_at.clone()).is_err() {
                    return self
                        .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                        .await;
                }
            }
            work_contracts::EvidenceVerifiedState::Rejected => {
                if state
                    .mark_stale(ReferenceStaleReason::rejected_evidence(
                        evidence_ref.clone(),
                    ))
                    .is_err()
                {
                    return self
                        .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                        .await;
                }
            }
            work_contracts::EvidenceVerifiedState::Unverified => {}
        }
        if self
            .reference_repo
            .save_reference_state(state, None, &reservation.uow)
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }

        self.complete_inbound(
            reservation,
            "consume_artifact_evidence_changed",
            &envelope,
            ConsumerDisposition::AckWithMarker,
        )
        .await
    }

    /// Consumes `runtime.work_promote.requested.v1`.
    pub async fn consume_runtime_promote_requested(
        &self,
        envelope: WorkInboundEventEnvelope<RuntimePromoteRequestedPayload>,
    ) -> ConsumerDisposition {
        if !self
            .validate_event_version(&envelope.event_version, "runtime.work_promote.requested.v1")
        {
            return ConsumerDisposition::DeadLetter;
        }
        if envelope
            .payload
            .source_ref
            .external_ref
            .external_id
            .trim()
            .is_empty()
        {
            return ConsumerDisposition::DeadLetter;
        }

        let reservation = match self
            .reserve_inbound(
                "consume_runtime_promote_requested",
                "runtime.work_promote.requested.v1",
                &envelope,
            )
            .await
        {
            ReserveInboundOutcome::Reserved(parts) => parts,
            ReserveInboundOutcome::AckDuplicate => return ConsumerDisposition::AckDuplicate,
            ReserveInboundOutcome::Retry => return ConsumerDisposition::Retry,
            ReserveInboundOutcome::DeadLetter => return ConsumerDisposition::DeadLetter,
        };

        let source_resolution = match self
            .source_resolver
            .resolve_source_work(envelope.payload.source_ref.clone())
            .await
        {
            Ok(source) => source,
            Err(PortError::Unavailable) => {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                    .await;
            }
            Err(PortError::NotFound | PortError::Rejected) => {
                let save_result = self
                    .reference_repo
                    .save_reference_state(
                        ReferenceResolutionState::unresolved(
                            work_contracts::ExternalReferenceRef::from_source_work(
                                envelope.payload.source_ref.clone(),
                            ),
                        ),
                        None,
                        &reservation.uow,
                    )
                    .await;
                if save_result.is_err() {
                    return self
                        .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                        .await;
                }
                return self
                    .complete_inbound(
                        reservation,
                        "consume_runtime_promote_requested",
                        &envelope,
                        ConsumerDisposition::AckWithMarker,
                    )
                    .await;
            }
            Err(PortError::InvalidResponse) => {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                    .await;
            }
        };
        if WorkTruthPolicy::assert_no_external_body(source_resolution.summary).is_err() {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                .await;
        }

        let intake = match PendingPromoteIntake::from_runtime_event(
            envelope.payload.source_ref.clone(),
            envelope.payload.promote_reason.clone(),
            envelope.source_event_id.clone(),
        ) {
            Ok(intake) => intake,
            Err(_) => {
                return self
                    .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                    .await;
            }
        };
        let mut state = ReferenceResolutionState::unresolved(
            work_contracts::ExternalReferenceRef::from_source_work(
                envelope.payload.source_ref.clone(),
            ),
        );
        if state.mark_resolved(envelope.occurred_at.clone()).is_err() {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::DeadLetter)
                .await;
        }
        if self
            .reference_repo
            .save_reference_state(state, None, &reservation.uow)
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }
        if self
            .promote_repo
            .save_pending_intake(intake, &reservation.uow)
            .await
            .is_err()
        {
            return self
                .rollback_disposition(reservation.uow, ConsumerDisposition::Retry)
                .await;
        }

        self.complete_inbound(
            reservation,
            "consume_runtime_promote_requested",
            &envelope,
            ConsumerDisposition::AckWithMarker,
        )
        .await
    }

    fn validate_event_version(&self, event_version: &EventSchemaVersion, topic: &str) -> bool {
        event_version.0 == "v1" && topic.ends_with(".v1")
    }

    fn affected_page() -> PageRequest {
        PageRequest {
            limit: AFFECTED_VIEWS_PAGE_LIMIT,
            page_token: None,
        }
    }

    async fn reserve_inbound<T: Serialize>(
        &self,
        operation_name: &'static str,
        topic: &'static str,
        envelope: &WorkInboundEventEnvelope<T>,
    ) -> ReserveInboundOutcome {
        let uow = match self.unit_of_work.begin().await {
            Ok(uow) => uow,
            Err(
                UnitOfWorkError::BeginFailed
                | UnitOfWorkError::CommitFailed
                | UnitOfWorkError::RollbackFailed,
            ) => return ReserveInboundOutcome::Retry,
        };
        let digest = match RequestDigest::from_canonical_command_input(
            &OperationName::new(operation_name),
            &work_contracts::fixtures::actor_context(),
            envelope,
        ) {
            Ok(digest) => digest,
            Err(_) => {
                let _ = self.unit_of_work.rollback(uow).await;
                return ReserveInboundOutcome::DeadLetter;
            }
        };
        let key = inbound_dedup_key(envelope, topic);
        match self
            .idempotency
            .reserve(key, OperationName::new(operation_name), digest, &uow)
            .await
        {
            Ok(IdempotencyReservation::Reserved(record)) => {
                ReserveInboundOutcome::Reserved(InboundReservation {
                    reservation: IdempotencyReservation::Reserved(record),
                    uow,
                })
            }
            Ok(IdempotencyReservation::Duplicate(_)) => {
                let _ = self.unit_of_work.rollback(uow).await;
                ReserveInboundOutcome::AckDuplicate
            }
            Ok(IdempotencyReservation::Conflict(conflict)) => {
                let _ = self.idempotency.mark_conflict(conflict, &uow).await;
                let _ = self.unit_of_work.rollback(uow).await;
                ReserveInboundOutcome::DeadLetter
            }
            Err(IdempotencyError::AlreadyReserved | IdempotencyError::StoreUnavailable) => {
                let _ = self.unit_of_work.rollback(uow).await;
                ReserveInboundOutcome::Retry
            }
            Err(IdempotencyError::Conflict) => {
                let _ = self.unit_of_work.rollback(uow).await;
                ReserveInboundOutcome::DeadLetter
            }
        }
    }

    async fn complete_inbound<T: Serialize>(
        &self,
        reservation: InboundReservation,
        operation_name: &'static str,
        envelope: &WorkInboundEventEnvelope<T>,
        disposition: ConsumerDisposition,
    ) -> ConsumerDisposition {
        let result_ref = ApplicationResultRef::for_operation(
            OperationName::new(operation_name),
            ResultId(envelope.source_event_id.0.clone()),
        );
        if self
            .idempotency
            .complete(reservation.reservation, result_ref, &reservation.uow)
            .await
            .is_err()
        {
            let _ = self.unit_of_work.rollback(reservation.uow).await;
            return ConsumerDisposition::Retry;
        }
        if self.unit_of_work.commit(reservation.uow).await.is_err() {
            return ConsumerDisposition::Retry;
        }
        disposition
    }

    async fn rollback_disposition(
        &self,
        uow: UnitOfWorkHandle,
        disposition: ConsumerDisposition,
    ) -> ConsumerDisposition {
        let _ = self.unit_of_work.rollback(uow).await;
        disposition
    }
}

struct InboundReservation {
    reservation: IdempotencyReservation,
    uow: UnitOfWorkHandle,
}

enum ReserveInboundOutcome {
    Reserved(InboundReservation),
    AckDuplicate,
    Retry,
    DeadLetter,
}

fn inbound_dedup_key<T>(envelope: &WorkInboundEventEnvelope<T>, topic: &str) -> IdempotencyKey {
    IdempotencyKey::from(format!(
        "{}:{}:{}:{}",
        topic,
        envelope.source_event_id.0,
        envelope.source_ref.source_system as u8,
        envelope.source_ref.external_id
    ))
}

fn truth_cursor_from_event<T>(
    topic: &str,
    envelope: &WorkInboundEventEnvelope<T>,
) -> WorkTruthCursor {
    WorkTruthCursor(format!(
        "{}:{}:{}",
        topic, envelope.source_ref.external_id, envelope.source_event_id.0
    ))
}

async fn save_resolved_reference<RS>(
    reference_repo: &RS,
    reference_ref: work_contracts::ExternalReferenceRef,
    occurred_at: core_contracts::metadata::Timestamp,
    uow: &UnitOfWorkHandle,
) -> Result<(), RepositoryError>
where
    RS: ReferenceSnapshotRepository,
{
    let mut state = ReferenceResolutionState::unresolved(reference_ref);
    state
        .mark_resolved(occurred_at)
        .map_err(|_| RepositoryError::VersionConflict)?;
    reference_repo
        .save_reference_state(state, None, uow)
        .await?;
    Ok(())
}
