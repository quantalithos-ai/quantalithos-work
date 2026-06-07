//! Worker entrypoints for the Work bounded context.

use work_application::{ConsumerDisposition, WorkInboundConsumerService};
use work_contracts::{
    ArtifactEvidenceChangedPayload, ConversationWorkContextChangedPayload,
    GovernanceDecisionChangedPayload, IdentityMemberChangedPayload, MethodDefinitionChangedPayload,
    ProcessTimingChangedPayload, RuntimePromoteRequestedPayload, WorkInboundEventEnvelope,
};

/// Thin worker-facing inbound consumer wrapper.
pub struct WorkInboundConsumers<S> {
    /// Application consumer service.
    pub service: S,
}

impl<S> WorkInboundConsumers<S> {
    /// Creates an inbound consumer wrapper.
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

impl<PM, RS, PR, P, U, SRC, IDEM>
    WorkInboundConsumers<WorkInboundConsumerService<PM, RS, PR, P, U, SRC, IDEM>>
where
    PM: work_application::ProjectMemberRepository,
    RS: work_application::ReferenceSnapshotRepository,
    PR: work_application::ProjectionRepository,
    P: work_application::PromoteRepository,
    U: work_application::UnitOfWork,
    SRC: work_application::SourceWorkResolverPort,
    IDEM: work_application::IdempotencyRepository,
{
    /// Consumes one identity member change event.
    pub async fn consume_identity_member_changed(
        &self,
        envelope: WorkInboundEventEnvelope<IdentityMemberChangedPayload>,
    ) -> ConsumerDisposition {
        self.service.consume_identity_member_changed(envelope).await
    }

    /// Consumes one method definition change event.
    pub async fn consume_method_definition_changed(
        &self,
        envelope: WorkInboundEventEnvelope<MethodDefinitionChangedPayload>,
    ) -> ConsumerDisposition {
        self.service
            .consume_method_definition_changed(envelope)
            .await
    }

    /// Consumes one conversation work-context change event.
    pub async fn consume_conversation_work_context_changed(
        &self,
        envelope: WorkInboundEventEnvelope<ConversationWorkContextChangedPayload>,
    ) -> ConsumerDisposition {
        self.service
            .consume_conversation_work_context_changed(envelope)
            .await
    }

    /// Consumes one process timing change event.
    pub async fn consume_process_timing_changed(
        &self,
        envelope: WorkInboundEventEnvelope<ProcessTimingChangedPayload>,
    ) -> ConsumerDisposition {
        self.service.consume_process_timing_changed(envelope).await
    }

    /// Consumes one governance decision change event.
    pub async fn consume_governance_decision_changed(
        &self,
        envelope: WorkInboundEventEnvelope<GovernanceDecisionChangedPayload>,
    ) -> ConsumerDisposition {
        self.service
            .consume_governance_decision_changed(envelope)
            .await
    }

    /// Consumes one artifact evidence change event.
    pub async fn consume_artifact_evidence_changed(
        &self,
        envelope: WorkInboundEventEnvelope<ArtifactEvidenceChangedPayload>,
    ) -> ConsumerDisposition {
        self.service
            .consume_artifact_evidence_changed(envelope)
            .await
    }

    /// Consumes one runtime promote request event.
    pub async fn consume_runtime_promote_requested(
        &self,
        envelope: WorkInboundEventEnvelope<RuntimePromoteRequestedPayload>,
    ) -> ConsumerDisposition {
        self.service
            .consume_runtime_promote_requested(envelope)
            .await
    }
}

#[cfg(test)]
mod tests {
    use work_application::{
        ConsumerDisposition, ProjectMemberRepository, WorkInboundConsumerService,
    };
    use work_contracts::fixtures;
    use work_domain::ProjectMember;
    use work_infra::{
        idempotency_store::InMemoryIdempotencyRepository, repositories::InMemoryWorkStores,
        source_resolvers::FakeSourceWorkResolverPort,
    };

    use super::WorkInboundConsumers;

    fn consumers() -> WorkInboundConsumers<
        WorkInboundConsumerService<
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            InMemoryWorkStores,
            FakeSourceWorkResolverPort,
            InMemoryIdempotencyRepository,
        >,
    > {
        let stores = InMemoryWorkStores::new();
        WorkInboundConsumers::new(WorkInboundConsumerService {
            member_repo: stores.clone(),
            reference_repo: stores.clone(),
            projection_repo: stores.clone(),
            promote_repo: stores.clone(),
            unit_of_work: stores,
            source_resolver: FakeSourceWorkResolverPort::new(),
            idempotency: InMemoryIdempotencyRepository::new(),
        })
    }

    fn seed_uow() -> work_application::UnitOfWorkHandle {
        work_application::UnitOfWorkHandle {
            handle_id: work_application::UnitOfWorkId("seed-uow".to_owned()),
        }
    }

    #[tokio::test]
    async fn unsupported_identity_event_version_dead_letters_before_write() {
        let consumers = consumers();
        let member_ref = fixtures::global_member_ref();
        ProjectMemberRepository::create(
            &consumers.service.member_repo,
            ProjectMember::assign(
                fixtures::project_member_id(),
                fixtures::project_id(),
                member_ref,
                fixtures::responsibility_spec(),
            )
            .expect("member should build"),
            &seed_uow(),
        )
        .await
        .expect("seed member");

        let mut envelope =
            fixtures::inbound_event_envelope(fixtures::identity_member_changed_payload());
        envelope.event_version = work_contracts::EventSchemaVersion("v2".to_owned());

        let disposition = consumers.consume_identity_member_changed(envelope).await;

        assert_eq!(disposition, ConsumerDisposition::DeadLetter);
        assert!(consumers.service.projection_repo.stale_marks().is_empty());
        assert!(
            consumers
                .service
                .reference_repo
                .member_snapshot(&fixtures::global_member_ref())
                .is_none()
        );
    }

    #[tokio::test]
    async fn unsupported_method_event_version_dead_letters_before_write() {
        let consumers = consumers();
        let mut envelope =
            fixtures::inbound_event_envelope(fixtures::method_definition_changed_payload());
        envelope.event_version = work_contracts::EventSchemaVersion("v2".to_owned());

        let disposition = consumers.consume_method_definition_changed(envelope).await;

        assert_eq!(disposition, ConsumerDisposition::DeadLetter);
        assert!(consumers.service.projection_repo.stale_marks().is_empty());
        assert!(
            consumers
                .service
                .reference_repo
                .method_snapshot(&fixtures::method_definition_ref())
                .is_none()
        );
    }

    #[tokio::test]
    async fn unsupported_runtime_promote_event_version_dead_letters_before_write() {
        let consumers = consumers();
        let mut envelope =
            fixtures::inbound_event_envelope(fixtures::runtime_promote_requested_payload());
        envelope.event_version = work_contracts::EventSchemaVersion("v2".to_owned());

        let disposition = consumers.consume_runtime_promote_requested(envelope).await;

        assert_eq!(disposition, ConsumerDisposition::DeadLetter);
        assert!(consumers.service.projection_repo.stale_marks().is_empty());
        assert!(
            consumers
                .service
                .promote_repo
                .pending_promote_intakes()
                .is_empty()
        );
    }
}
