use work_application::{ConsumerDisposition, ProjectMemberRepository, WorkInboundConsumerService};
use work_contracts::fixtures;
use work_domain::ProjectMember;
use work_infra::{
    idempotency_store::InMemoryIdempotencyRepository,
    repositories::InMemoryWorkStores,
    source_resolvers::{FakeSourceWorkResolverPort, SourceResolverOutcome},
};

fn service() -> WorkInboundConsumerService<
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    InMemoryWorkStores,
    FakeSourceWorkResolverPort,
    InMemoryIdempotencyRepository,
> {
    let stores = InMemoryWorkStores::new();
    WorkInboundConsumerService {
        member_repo: stores.clone(),
        reference_repo: stores.clone(),
        projection_repo: stores.clone(),
        promote_repo: stores.clone(),
        unit_of_work: stores,
        source_resolver: FakeSourceWorkResolverPort::new(),
        idempotency: InMemoryIdempotencyRepository::new(),
    }
}

fn seed_uow() -> work_application::UnitOfWorkHandle {
    work_application::UnitOfWorkHandle {
        handle_id: work_application::UnitOfWorkId("seed-uow".to_owned()),
    }
}

#[tokio::test]
async fn consume_identity_member_changed_marks_repository_returned_views_stale() {
    let svc = service();
    let member_ref = fixtures::global_member_ref();
    let member_view =
        work_contracts::DerivedWorkViewRef::member_work(fixtures::project_member_ref());
    let board_view = work_contracts::DerivedWorkViewRef::project_board(fixtures::project_ref());
    ProjectMemberRepository::create(
        &svc.member_repo,
        ProjectMember::assign(
            fixtures::project_member_id(),
            fixtures::project_id(),
            member_ref.clone(),
            fixtures::responsibility_spec(),
        )
        .expect("member should build"),
        &seed_uow(),
    )
    .await
    .expect("seed member");
    svc.projection_repo
        .seed_affected_views_for_member(&member_ref, vec![member_view.clone(), board_view.clone()]);

    let disposition = svc
        .consume_identity_member_changed(fixtures::inbound_event_envelope(
            fixtures::identity_member_changed_payload(),
        ))
        .await;

    assert_eq!(disposition, ConsumerDisposition::Ack);
    let stale_marks = svc.projection_repo.stale_marks();
    assert_eq!(stale_marks.len(), 1);
    assert_eq!(stale_marks[0].0, vec![member_view, board_view]);
    assert!(svc.reference_repo.member_snapshot(&member_ref).is_some());
    assert!(
        svc.reference_repo
            .reference_state_snapshot(&work_contracts::ExternalReferenceRef::from_member(
                member_ref
            ))
            .is_some()
    );
    assert_eq!(svc.unit_of_work.trace_count(), 0);
}

#[tokio::test]
async fn consume_identity_member_changed_with_empty_affected_views_writes_no_stale_marker() {
    let svc = service();
    let member_ref = fixtures::global_member_ref();
    ProjectMemberRepository::create(
        &svc.member_repo,
        ProjectMember::assign(
            fixtures::project_member_id(),
            fixtures::project_id(),
            member_ref.clone(),
            fixtures::responsibility_spec(),
        )
        .expect("member should build"),
        &seed_uow(),
    )
    .await
    .expect("seed member");

    let disposition = svc
        .consume_identity_member_changed(fixtures::inbound_event_envelope(
            fixtures::identity_member_changed_payload(),
        ))
        .await;

    assert_eq!(disposition, ConsumerDisposition::AckWithMarker);
    assert!(svc.projection_repo.stale_marks().is_empty());
    assert!(svc.reference_repo.member_snapshot(&member_ref).is_some());
}

#[tokio::test]
async fn consume_method_definition_changed_marks_repository_returned_views_stale() {
    let svc = service();
    let definition_ref = fixtures::method_definition_ref();
    let board_view = work_contracts::DerivedWorkViewRef::project_board(fixtures::project_ref());
    svc.projection_repo
        .seed_affected_views_for_method(&definition_ref, vec![board_view.clone()]);

    let disposition = svc
        .consume_method_definition_changed(fixtures::inbound_event_envelope(
            fixtures::method_definition_changed_payload(),
        ))
        .await;

    assert_eq!(disposition, ConsumerDisposition::Ack);
    let stale_marks = svc.projection_repo.stale_marks();
    assert_eq!(stale_marks.len(), 1);
    assert_eq!(stale_marks[0].0, vec![board_view]);
    assert!(
        svc.reference_repo
            .method_snapshot(&definition_ref)
            .is_some()
    );
    assert_eq!(svc.unit_of_work.trace_count(), 0);
}

#[tokio::test]
async fn consume_method_definition_changed_with_empty_affected_views_writes_no_stale_marker() {
    let svc = service();

    let disposition = svc
        .consume_method_definition_changed(fixtures::inbound_event_envelope(
            fixtures::method_definition_changed_payload(),
        ))
        .await;

    assert_eq!(disposition, ConsumerDisposition::AckWithMarker);
    assert!(svc.projection_repo.stale_marks().is_empty());
}

#[tokio::test]
async fn duplicate_identity_event_does_not_repeat_snapshot_or_stale_marker() {
    let svc = service();
    let member_ref = fixtures::global_member_ref();
    ProjectMemberRepository::create(
        &svc.member_repo,
        ProjectMember::assign(
            fixtures::project_member_id(),
            fixtures::project_id(),
            member_ref.clone(),
            fixtures::responsibility_spec(),
        )
        .expect("member should build"),
        &seed_uow(),
    )
    .await
    .expect("seed member");
    svc.projection_repo.seed_affected_views_for_member(
        &member_ref,
        vec![work_contracts::DerivedWorkViewRef::member_work(
            fixtures::project_member_ref(),
        )],
    );
    let envelope = fixtures::inbound_event_envelope(fixtures::identity_member_changed_payload());

    let first = svc.consume_identity_member_changed(envelope.clone()).await;
    let second = svc.consume_identity_member_changed(envelope).await;

    assert_eq!(first, ConsumerDisposition::Ack);
    assert_eq!(second, ConsumerDisposition::AckDuplicate);
    assert_eq!(svc.projection_repo.stale_marks().len(), 1);
    assert_eq!(
        svc.reference_repo
            .member_snapshot(&member_ref)
            .map(|(_, version)| version),
        Some(1)
    );
    assert_eq!(
        svc.reference_repo
            .reference_state_snapshot(&work_contracts::ExternalReferenceRef::from_member(
                member_ref
            ))
            .map(|(_, version)| version),
        Some(1)
    );
}

#[tokio::test]
async fn duplicate_method_event_does_not_repeat_snapshot_or_stale_marker() {
    let svc = service();
    let definition_ref = fixtures::method_definition_ref();
    svc.projection_repo.seed_affected_views_for_method(
        &definition_ref,
        vec![work_contracts::DerivedWorkViewRef::project_board(
            fixtures::project_ref(),
        )],
    );
    let envelope = fixtures::inbound_event_envelope(fixtures::method_definition_changed_payload());

    let first = svc
        .consume_method_definition_changed(envelope.clone())
        .await;
    let second = svc.consume_method_definition_changed(envelope).await;

    assert_eq!(first, ConsumerDisposition::Ack);
    assert_eq!(second, ConsumerDisposition::AckDuplicate);
    assert_eq!(svc.projection_repo.stale_marks().len(), 1);
    assert_eq!(
        svc.reference_repo
            .method_snapshot(&definition_ref)
            .map(|(_, version)| version),
        Some(1)
    );
    assert_eq!(
        svc.reference_repo
            .reference_state_snapshot(
                &work_contracts::ExternalReferenceRef::from_method_definition(definition_ref)
            )
            .map(|(_, version)| version),
        Some(1)
    );
}

#[tokio::test]
async fn missing_capability_dead_letters_before_write() {
    let svc = service();
    let mut payload = fixtures::identity_member_changed_payload();
    payload.capability_refs.refs.clear();

    let disposition = svc
        .consume_identity_member_changed(fixtures::inbound_event_envelope(payload))
        .await;

    assert_eq!(disposition, ConsumerDisposition::DeadLetter);
    assert!(svc.projection_repo.stale_marks().is_empty());
    assert!(
        svc.reference_repo
            .member_snapshot(&fixtures::global_member_ref())
            .is_none()
    );
}

#[tokio::test]
async fn missing_definition_dead_letters_before_write() {
    let svc = service();
    let mut payload = fixtures::method_definition_changed_payload();
    payload.definition_ref = work_contracts::MethodDefinitionRef(String::new());

    let disposition = svc
        .consume_method_definition_changed(fixtures::inbound_event_envelope(payload))
        .await;

    assert_eq!(disposition, ConsumerDisposition::DeadLetter);
    assert!(svc.projection_repo.stale_marks().is_empty());
    assert!(
        svc.reference_repo
            .method_snapshot(&fixtures::method_definition_ref())
            .is_none()
    );
}

#[tokio::test]
async fn runtime_promote_requested_writes_pending_intake_without_work_truth() {
    let svc = service();
    svc.source_resolver.seed(
        fixtures::runtime_source_work_ref(),
        SourceResolverOutcome::Success {
            has_external_body: false,
        },
    );

    let disposition = svc
        .consume_runtime_promote_requested(fixtures::inbound_event_envelope(
            fixtures::runtime_promote_requested_payload(),
        ))
        .await;

    assert_eq!(disposition, ConsumerDisposition::AckWithMarker);
    assert_eq!(svc.promote_repo.pending_promote_intakes().len(), 1);
    assert_eq!(svc.unit_of_work.trace_count(), 0);
    assert!(svc.projection_repo.stale_marks().is_empty());
}

#[tokio::test]
async fn runtime_promote_requested_unresolved_source_saves_marker_and_dedups() {
    let svc = service();
    svc.source_resolver.seed(
        fixtures::runtime_source_work_ref(),
        SourceResolverOutcome::Unresolved,
    );
    let envelope = fixtures::inbound_event_envelope(fixtures::runtime_promote_requested_payload());

    let first = svc
        .consume_runtime_promote_requested(envelope.clone())
        .await;
    let second = svc.consume_runtime_promote_requested(envelope).await;

    assert_eq!(first, ConsumerDisposition::AckWithMarker);
    assert_eq!(second, ConsumerDisposition::AckDuplicate);
    assert!(svc.promote_repo.pending_promote_intakes().is_empty());
    assert!(
        svc.reference_repo
            .reference_state_snapshot(&work_contracts::ExternalReferenceRef::from_source_work(
                fixtures::runtime_source_work_ref()
            ))
            .is_some()
    );
}
