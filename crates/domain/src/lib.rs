//! Domain types for the Work bounded context.

mod audit;
mod errors;
mod policies;
mod project;

pub use audit::{TraceHandoffMarker, WorkAuditTrail, WorkOutboxRecord, WorkTraceRecord};
pub use errors::DomainError;
pub use policies::{
    BacklogAvailabilityPolicy, CompletionEvidencePolicy, FormalWorkPolicy,
    MemberResponsibilityPolicy, ProjectLifecyclePolicy, WorkTruthPolicy,
};
pub use project::{
    Backlog, ChildWorkItem, MemberCapabilitySnapshot, Project, ProjectMember,
    ReferenceResolutionState, WorkItem,
};

#[cfg(test)]
mod tests {
    use core_contracts::actor::{ActorKind, ActorRef};

    use crate::{
        Backlog, ChildWorkItem, DomainError, FormalWorkPolicy, MemberCapabilitySnapshot, Project,
        ProjectMember, TraceHandoffMarker, WorkAuditTrail, WorkItem, WorkOutboxRecord,
        WorkTraceRecord, WorkTruthPolicy,
    };
    use work_contracts::{
        BacklogAvailabilityTarget, BacklogMaintenanceReason, BacklogMaintenanceReasonKind,
        ExternalSourceSummary, OutboxPublicationState, ProjectLifecycleReason,
        ProjectLifecycleReasonKind, ProjectLifecycleState, ProjectLifecycleTarget,
        ProjectMemberReason, ProjectMemberReasonKind, ProjectMemberResponsibilityState,
        TraceHandoffTargetKind, TraceHandoffTargetRef, WorkAuditSubjectRef, WorkLifecycleTarget,
        WorkOutboxEventKind, WorkTraceSubjectRef, WorkTruthChange, fixtures,
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
    fn formal_work_item_create_and_complete_follow_matrix() {
        let actor = actor();
        let backlog = Backlog::open_for_project(
            fixtures::backlog_id(),
            fixtures::project_id(),
            actor.clone(),
        )
        .expect("backlog open should succeed");
        backlog
            .assert_can_accept(&fixtures::formal_work_intent())
            .expect("open backlog should accept valid formal intent");

        let mut work = WorkItem::formalize(
            fixtures::work_item_id(),
            fixtures::backlog_id(),
            fixtures::formal_work_intent(),
            fixtures::source_work_ref(),
            actor.clone(),
        )
        .expect("formalize should succeed");
        assert_eq!(work.formal_work_ref(), fixtures::formal_work_ref());
        assert_eq!(work.work_state, work_contracts::WorkItemState::Formalized);
        assert_eq!(work.completion_ref, None);

        backlog
            .accept_work_item(&work, actor.clone())
            .expect("backlog should accept matching work item");

        work.transition_lifecycle(
            WorkLifecycleTarget::InProgress,
            fixtures::start_work_reason(),
            None,
            actor.clone(),
        )
        .expect("formalized -> in_progress should succeed");
        work.transition_lifecycle(
            WorkLifecycleTarget::Completed,
            fixtures::completion_work_reason(),
            Some(fixtures::completion_evidence_ref()),
            actor,
        )
        .expect("in_progress -> completed should succeed");
        assert_eq!(work.work_state, work_contracts::WorkItemState::Completed);
        assert_eq!(
            work.completion_ref,
            Some(fixtures::completion_evidence_ref())
        );
    }

    #[test]
    fn work_truth_policy_rejects_external_body_summary() {
        let err = WorkTruthPolicy::assert_no_external_body(ExternalSourceSummary {
            source_ref: fixtures::source_work_ref(),
            source_kind: work_contracts::SourceWorkKind::Conversation,
            source_digest: fixtures::source_work_ref().source_digest,
            has_external_body: true,
        })
        .expect_err("external body must be rejected");
        assert_eq!(err, DomainError::ExternalBodyRejected);
    }

    #[test]
    fn work_truth_policy_rejects_formal_work_change_when_backlog_locked() {
        let policy = WorkTruthPolicy {
            policy_scope: work_contracts::WorkPolicyScope {
                project_ref: fixtures::project_ref(),
                work_ref: Some(fixtures::formal_work_ref()),
                source_ref: Some(fixtures::source_work_ref()),
            },
            truth_snapshot: work_contracts::WorkTruthSnapshot {
                project_ref: fixtures::project_ref(),
                lifecycle_state: ProjectLifecycleState::Active,
                backlog_state: Some(work_contracts::BacklogState::LockedForMaintenance),
                source_cursor: work_contracts::WorkTruthCursor("cursor-1".to_owned()),
            },
        };

        let err = policy
            .assert_truth_change_allowed(
                WorkTruthChange::WorkItemChanged(fixtures::formal_work_ref()),
                &actor(),
            )
            .expect_err("locked backlog should reject formal work truth change");
        assert_eq!(err, DomainError::PolicyRejected);
    }

    #[test]
    fn backlog_maintenance_lock_rejects_new_formal_work() {
        let actor = actor();
        let mut backlog = Backlog::open_for_project(
            fixtures::backlog_id(),
            fixtures::project_id(),
            actor.clone(),
        )
        .expect("backlog open should succeed");
        backlog
            .lock_for_maintenance(
                BacklogMaintenanceReason {
                    reason_kind: BacklogMaintenanceReasonKind::MaintenanceWindow,
                    reason_ref: None,
                },
                actor,
            )
            .expect("lock should succeed");

        let err = backlog
            .assert_can_accept(&fixtures::formal_work_intent())
            .expect_err("locked backlog must reject new work");
        assert_eq!(err, DomainError::PolicyRejected);
    }

    #[test]
    fn child_work_item_create_and_complete_follow_matrix() {
        let actor = actor();
        let mut child = ChildWorkItem::create_child(
            fixtures::child_work_item_id(),
            fixtures::work_item_id(),
            fixtures::child_work_intent(),
            fixtures::source_work_ref(),
        )
        .expect("child create should succeed");
        assert_eq!(child.formal_work_ref(), fixtures::child_formal_work_ref());
        assert_eq!(child.completion_ref, None);

        child
            .attach_to_parent(fixtures::work_item_id(), actor.clone())
            .expect("same parent should succeed");
        let err = child
            .attach_to_parent(
                work_contracts::WorkItemId("other-parent".to_owned()),
                actor.clone(),
            )
            .expect_err("different parent should fail");
        assert_eq!(err, DomainError::RefMismatch);

        child
            .transition_lifecycle(
                WorkLifecycleTarget::InProgress,
                fixtures::start_work_reason(),
                None,
                actor.clone(),
            )
            .expect("formalized child -> in_progress should succeed");
        child
            .transition_lifecycle(
                WorkLifecycleTarget::Completed,
                fixtures::completion_work_reason(),
                Some(fixtures::completion_evidence_ref()),
                actor,
            )
            .expect("child in_progress -> completed should succeed");
        assert_eq!(child.work_state, work_contracts::WorkItemState::Completed);
        assert_eq!(
            child.completion_ref,
            Some(fixtures::completion_evidence_ref())
        );
    }

    #[test]
    fn formal_work_policy_and_completion_guard_reject_invalid_inputs() {
        let actor = actor();
        let err = FormalWorkPolicy::assert_formal_work(
            fixtures::formal_work_intent(),
            fixtures::runtime_source_work_ref(),
        )
        .expect_err("runtime source must not directly formalize");
        assert_eq!(err, DomainError::PolicyRejected);

        let mut work = WorkItem::formalize(
            fixtures::work_item_id(),
            fixtures::backlog_id(),
            fixtures::formal_work_intent(),
            fixtures::source_work_ref(),
            actor.clone(),
        )
        .expect("formalize should succeed");
        work.transition_lifecycle(
            WorkLifecycleTarget::InProgress,
            fixtures::start_work_reason(),
            None,
            actor.clone(),
        )
        .expect("start should succeed");
        let err = work
            .transition_lifecycle(
                WorkLifecycleTarget::Completed,
                fixtures::completion_work_reason(),
                Some(fixtures::unverified_completion_evidence_ref()),
                actor.clone(),
            )
            .expect_err("unverified evidence must fail");
        assert_eq!(err, DomainError::PolicyRejected);

        let mut child = ChildWorkItem::create_child(
            fixtures::child_work_item_id(),
            fixtures::work_item_id(),
            fixtures::child_work_intent(),
            fixtures::source_work_ref(),
        )
        .expect("child create should succeed");
        let err = child
            .transition_lifecycle(
                WorkLifecycleTarget::Completed,
                fixtures::completion_work_reason(),
                Some(fixtures::completion_evidence_ref()),
                actor,
            )
            .expect_err("completed from formalized should fail");
        assert_eq!(err, DomainError::InvalidStateTransition);
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

        let work_trace = WorkTraceRecord::from_truth_change(
            fixtures::trace_id(),
            fixtures::work_item_changed_change(),
            fixtures::trace_context_ref(),
        )
        .expect("work trace should succeed");
        assert_eq!(
            work_trace.subject_ref,
            WorkTraceSubjectRef::FormalWork(fixtures::formal_work_ref())
        );

        let work_outbox = WorkOutboxRecord::from_truth_change(
            fixtures::outbox_id(),
            WorkTruthChange::WorkItemChanged(fixtures::formal_work_ref()),
        )
        .expect("work outbox should succeed");
        assert_eq!(work_outbox.event_kind, WorkOutboxEventKind::WorkItemChanged);

        let mut work_audit = WorkAuditTrail::start_for_subject(WorkAuditSubjectRef::FormalWork(
            fixtures::formal_work_ref(),
        ));
        work_audit
            .append(work_trace)
            .expect("work audit append should succeed");
    }
}
