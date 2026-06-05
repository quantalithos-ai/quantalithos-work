//! Domain types for the Work bounded context.

mod audit;
mod errors;
mod policies;
mod project;

pub use audit::{TraceHandoffMarker, WorkAuditTrail, WorkOutboxRecord, WorkTraceRecord};
pub use errors::DomainError;
pub use policies::{BacklogAvailabilityPolicy, MemberResponsibilityPolicy, ProjectLifecyclePolicy};
pub use project::{
    Backlog, MemberCapabilitySnapshot, Project, ProjectMember, ReferenceResolutionState,
};

#[cfg(test)]
mod tests {
    use core_contracts::actor::{ActorKind, ActorRef};

    use crate::{
        Backlog, DomainError, MemberCapabilitySnapshot, Project, ProjectMember, TraceHandoffMarker,
        WorkAuditTrail, WorkOutboxRecord, WorkTraceRecord,
    };
    use work_contracts::{
        BacklogAvailabilityTarget, BacklogMaintenanceReason, BacklogMaintenanceReasonKind,
        OutboxPublicationState, ProjectLifecycleReason, ProjectLifecycleReasonKind,
        ProjectLifecycleState, ProjectLifecycleTarget, ProjectMemberReason,
        ProjectMemberReasonKind, ProjectMemberResponsibilityState, TraceHandoffTargetKind,
        TraceHandoffTargetRef, WorkAuditSubjectRef, WorkOutboxEventKind, WorkTraceSubjectRef,
        fixtures,
    };

    fn actor() -> ActorRef {
        ActorRef::new("domain-actor-1", ActorKind::Human)
    }

    #[test]
    fn project_lifecycle_transitions_follow_matrix() {
        let actor = actor();
        let mut project = Project::create(
            fixtures::project_id(),
            fixtures::project_spec(),
            actor.clone(),
        )
        .expect("project create should succeed");
        assert_eq!(project.lifecycle_state, ProjectLifecycleState::Active);

        project
            .transition_lifecycle(
                ProjectLifecycleTarget::ReadOnly,
                ProjectLifecycleReason {
                    reason_kind: ProjectLifecycleReasonKind::Maintenance,
                    reason_ref: None,
                    note: Some(fixtures::safe_summary("maintenance")),
                },
                actor.clone(),
            )
            .expect("active -> read_only should succeed");
        assert_eq!(project.lifecycle_state, ProjectLifecycleState::ReadOnly);

        project
            .transition_lifecycle(
                ProjectLifecycleTarget::Closed,
                ProjectLifecycleReason {
                    reason_kind: ProjectLifecycleReasonKind::OwnerRequest,
                    reason_ref: None,
                    note: Some(fixtures::safe_summary("close")),
                },
                actor.clone(),
            )
            .expect("read_only -> closed should succeed");
        assert_eq!(project.lifecycle_state, ProjectLifecycleState::Closed);

        project
            .transition_lifecycle(
                ProjectLifecycleTarget::Archived,
                ProjectLifecycleReason {
                    reason_kind: ProjectLifecycleReasonKind::ArchivePrepared,
                    reason_ref: None,
                    note: Some(fixtures::safe_summary("archive")),
                },
                actor.clone(),
            )
            .expect("closed -> archived should succeed");
        assert_eq!(project.lifecycle_state, ProjectLifecycleState::Archived);

        let err = project
            .transition_lifecycle(
                ProjectLifecycleTarget::Closed,
                ProjectLifecycleReason {
                    reason_kind: ProjectLifecycleReasonKind::OwnerRequest,
                    reason_ref: None,
                    note: Some(fixtures::safe_summary("again")),
                },
                actor,
            )
            .expect_err("archived transition should fail");
        assert_eq!(err, DomainError::InvalidStateTransition);
    }

    #[test]
    fn backlog_state_transitions_follow_matrix() {
        let actor = actor();
        let mut backlog = Backlog::open_for_project(
            fixtures::backlog_id(),
            fixtures::project_id(),
            actor.clone(),
        )
        .expect("backlog open should succeed");

        backlog
            .transition_availability(
                BacklogAvailabilityTarget::LockedForMaintenance,
                BacklogMaintenanceReason {
                    reason_kind: BacklogMaintenanceReasonKind::MaintenanceWindow,
                    reason_ref: None,
                },
                actor.clone(),
            )
            .expect("open -> locked should succeed");
        assert_eq!(
            backlog.backlog_state,
            work_contracts::BacklogState::LockedForMaintenance
        );

        backlog
            .transition_availability(
                BacklogAvailabilityTarget::Open,
                BacklogMaintenanceReason {
                    reason_kind: BacklogMaintenanceReasonKind::ManualUnlock,
                    reason_ref: None,
                },
                actor.clone(),
            )
            .expect("locked -> open should succeed");
        assert_eq!(backlog.backlog_state, work_contracts::BacklogState::Open);

        backlog
            .archive_with_project(fixtures::project_ref(), actor.clone())
            .expect("archive should succeed");
        assert_eq!(
            backlog.backlog_state,
            work_contracts::BacklogState::Archived
        );

        let err = backlog
            .transition_availability(
                BacklogAvailabilityTarget::Open,
                BacklogMaintenanceReason {
                    reason_kind: BacklogMaintenanceReasonKind::ManualUnlock,
                    reason_ref: None,
                },
                actor,
            )
            .expect_err("archived backlog reopen should fail");
        assert_eq!(err, DomainError::InvalidStateTransition);
    }

    #[test]
    fn project_member_responsibility_transitions_follow_matrix() {
        let actor = actor();
        let mut project_member = ProjectMember::assign(
            fixtures::project_member_id(),
            fixtures::project_id(),
            fixtures::global_member_ref(),
            fixtures::responsibility_spec(),
        )
        .expect("assign should create proposed responsibility");
        assert_eq!(
            project_member.responsibility_state,
            ProjectMemberResponsibilityState::Proposed
        );

        let snapshot = MemberCapabilitySnapshot::from_identity(
            fixtures::global_member_ref(),
            fixtures::capability_ref_set(),
        )
        .expect("snapshot should build");

        project_member
            .activate(snapshot.clone(), actor.clone())
            .expect("proposed -> active should succeed");
        assert_eq!(
            project_member.responsibility_state,
            ProjectMemberResponsibilityState::Active
        );

        project_member
            .pause(
                ProjectMemberReason {
                    reason_kind: ProjectMemberReasonKind::Paused,
                    reason_ref: None,
                },
                actor.clone(),
            )
            .expect("active -> paused should succeed");
        assert_eq!(
            project_member.responsibility_state,
            ProjectMemberResponsibilityState::Paused
        );

        project_member
            .resume(snapshot, actor.clone())
            .expect("paused -> active should succeed");
        assert_eq!(
            project_member.responsibility_state,
            ProjectMemberResponsibilityState::Active
        );

        project_member
            .release(
                ProjectMemberReason {
                    reason_kind: ProjectMemberReasonKind::Released,
                    reason_ref: None,
                },
                actor.clone(),
            )
            .expect("active -> released should succeed");
        assert_eq!(
            project_member.responsibility_state,
            ProjectMemberResponsibilityState::Released
        );

        let err = project_member
            .resume(
                MemberCapabilitySnapshot::from_identity(
                    fixtures::global_member_ref(),
                    fixtures::capability_ref_set(),
                )
                .expect("snapshot should build"),
                actor,
            )
            .expect_err("released responsibility must remain terminal");
        assert_eq!(err, DomainError::InvalidStateTransition);
    }

    #[test]
    fn member_capability_snapshot_must_match_required_capabilities() {
        let snapshot = MemberCapabilitySnapshot::from_identity(
            fixtures::global_member_ref(),
            fixtures::capability_ref_set(),
        )
        .expect("snapshot should build");
        assert!(snapshot.supports(&fixtures::responsibility_spec()));

        let insufficient = MemberCapabilitySnapshot::from_identity(
            fixtures::global_member_ref(),
            work_contracts::CapabilityRefSet { refs: Vec::new() },
        )
        .expect("snapshot should build");
        assert!(!insufficient.supports(&fixtures::responsibility_spec()));
    }

    #[test]
    fn domain_audit_outbox_records_follow_truth_change() {
        let trace = WorkTraceRecord::from_truth_change(
            fixtures::trace_id(),
            fixtures::project_created_change(),
            fixtures::trace_context_ref(),
        )
        .expect("trace from truth change should succeed");
        assert_eq!(
            trace.subject_ref,
            WorkTraceSubjectRef::Project(fixtures::project_ref())
        );

        let intent = trace
            .prepare_handoff(TraceHandoffTargetRef {
                target_kind: TraceHandoffTargetKind::Observability,
                external_ref: fixtures::external_source_ref(),
            })
            .expect("handoff intent should succeed");
        assert_eq!(
            intent.subject_ref,
            WorkTraceSubjectRef::Project(fixtures::project_ref())
        );

        let marker =
            TraceHandoffMarker::from_trace(fixtures::trace_id(), fixtures::trace_handoff_ref())
                .expect("trace marker should succeed");
        assert_eq!(marker.trace_id, fixtures::trace_id());

        let mut audit = WorkAuditTrail::start_for_subject(WorkAuditSubjectRef::Project(
            fixtures::project_ref(),
        ));
        audit.append(trace.clone()).expect("append should succeed");
        assert!(!audit.has_gap());
        assert_eq!(audit.record_refs.trace_ids, vec![fixtures::trace_id()]);

        let outbox = WorkOutboxRecord::from_truth_change(
            fixtures::outbox_id(),
            fixtures::project_created_change(),
        )
        .expect("outbox from truth change should succeed");
        assert_eq!(outbox.event_kind, WorkOutboxEventKind::ProjectChanged);
        assert_eq!(outbox.publication_state, OutboxPublicationState::Pending);

        let backlog_outbox = WorkOutboxRecord::from_truth_change(
            fixtures::outbox_id(),
            fixtures::backlog_changed_change(),
        )
        .expect("backlog outbox from truth change should succeed");
        assert_eq!(
            backlog_outbox.event_kind,
            WorkOutboxEventKind::BacklogChanged
        );

        let member_trace = WorkTraceRecord::from_truth_change(
            fixtures::trace_id(),
            fixtures::project_member_changed_change(),
            fixtures::trace_context_ref(),
        )
        .expect("member trace should succeed");
        assert_eq!(
            member_trace.subject_ref,
            WorkTraceSubjectRef::ProjectMember(fixtures::project_member_ref())
        );

        let member_outbox = WorkOutboxRecord::from_truth_change(
            fixtures::outbox_id(),
            fixtures::project_member_changed_change(),
        )
        .expect("member outbox should succeed");
        assert_eq!(
            member_outbox.event_kind,
            WorkOutboxEventKind::ProjectMemberChanged
        );
    }
}
