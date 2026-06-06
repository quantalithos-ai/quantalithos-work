//! Domain types for the Work bounded context.

mod audit;
mod dependency;
mod errors;
mod iteration;
mod policies;
mod project;
mod promote;

pub use audit::{TraceHandoffMarker, WorkAuditTrail, WorkOutboxRecord, WorkTraceRecord};
pub use dependency::{
    DependencyChangeRecord, DependencyGraphPolicy, DependencyGraphSnapshot, WorkBlocker,
    WorkDependency,
};
pub use errors::DomainError;
pub use iteration::{Iteration, IterationChangeRecord, IterationCommitment};
pub use policies::{
    BacklogAvailabilityPolicy, CompletionEvidencePolicy, FormalWorkPolicy,
    IterationCommitmentPolicy, MemberResponsibilityPolicy, ProjectLifecyclePolicy, PromotePolicy,
    WorkTruthPolicy,
};
pub use project::{
    Backlog, ChildWorkItem, MemberCapabilitySnapshot, Project, ProjectMember,
    ReferenceResolutionState, WorkItem,
};
pub use promote::{PendingPromoteIntake, PromoteDecisionRecord, PromoteResult};

#[cfg(test)]
mod tests {
    use core_contracts::actor::{ActorKind, ActorRef};

    use crate::{
        Backlog, ChildWorkItem, DependencyChangeRecord, DependencyGraphPolicy,
        DependencyGraphSnapshot, DomainError, FormalWorkPolicy, Iteration, IterationChangeRecord,
        IterationCommitment, IterationCommitmentPolicy, MemberCapabilitySnapshot,
        PendingPromoteIntake, Project, ProjectMember, PromoteDecisionRecord, PromoteResult,
        TraceHandoffMarker, WorkAuditTrail, WorkBlocker, WorkDependency, WorkItem,
        WorkOutboxRecord, WorkTraceRecord, WorkTruthPolicy,
    };
    use work_contracts::{
        BacklogAvailabilityTarget, BacklogMaintenanceReason, BacklogMaintenanceReasonKind,
        CommitmentState, DependencyChangeId, DependencyOrBlockerRef, ExternalSourceSummary,
        IterationState, OutboxPublicationState, ProjectLifecycleReason, ProjectLifecycleReasonKind,
        ProjectLifecycleState, ProjectLifecycleTarget, ProjectMemberReason,
        ProjectMemberReasonKind, ProjectMemberResponsibilityState, PromoteDecisionId,
        PromoteResultState, TraceHandoffTargetKind, TraceHandoffTargetRef, WorkAuditSubjectRef,
        WorkLifecycleTarget, WorkOutboxEventKind, WorkTraceSubjectRef, WorkTruthChange, fixtures,
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
    fn promote_result_and_runtime_intake_follow_promote_boundary() {
        let actor = actor();
        let mut result = PromoteResult::evaluate(
            fixtures::promote_result_id(),
            fixtures::source_work_ref(),
            fixtures::promote_reason(),
            actor.clone(),
        )
        .expect("promote result should be created");
        assert_eq!(result.result_state, PromoteResultState::PendingReview);
        assert_eq!(result.created_work_ref, None);

        result
            .accept(fixtures::formal_work_ref(), actor.clone())
            .expect("accept should succeed");
        assert_eq!(result.result_state, PromoteResultState::Accepted);
        assert_eq!(result.created_work_ref, Some(fixtures::formal_work_ref()));

        let mut rejected = PromoteResult::evaluate(
            fixtures::promote_result_id(),
            fixtures::source_work_ref(),
            fixtures::promote_reason(),
            actor.clone(),
        )
        .expect("promote result should be created");
        rejected
            .reject(fixtures::promote_reject_reason(), actor.clone())
            .expect("reject should succeed");
        assert_eq!(rejected.result_state, PromoteResultState::Rejected);
        assert_eq!(rejected.created_work_ref, None);

        let intake = PendingPromoteIntake::from_runtime_event(
            fixtures::runtime_source_work_ref(),
            fixtures::promote_reason(),
            fixtures::source_event_id(),
        )
        .expect("runtime intake marker should build");
        assert_eq!(intake.source_ref, fixtures::runtime_source_work_ref());
        assert_eq!(intake.source_event_id, fixtures::source_event_id());

        let decision = PromoteDecisionRecord::from_result(
            PromoteDecisionId("promote-decision-1".to_owned()),
            rejected,
            actor.clone(),
        )
        .expect("decision record should build");
        assert_eq!(decision.result_ref, fixtures::promote_result_ref());
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
    fn dependency_state_transitions_follow_matrix() {
        let actor = actor();
        let mut dependency = WorkDependency::link(
            fixtures::work_dependency_id(),
            fixtures::formal_work_ref(),
            fixtures::downstream_formal_work_ref(),
            fixtures::dependency_reason(),
        )
        .expect("link should create proposed dependency");
        assert_eq!(
            dependency.dependency_state,
            work_contracts::DependencyState::Proposed
        );

        dependency
            .activate(actor.clone(), fixtures::dependency_activated_reason())
            .expect("proposed -> active should succeed");
        assert_eq!(
            dependency.dependency_state,
            work_contracts::DependencyState::Active
        );

        let mut satisfied = dependency.clone();
        satisfied
            .mark_satisfied(fixtures::completion_evidence_ref(), actor.clone())
            .expect("active -> satisfied should succeed");
        assert_eq!(
            satisfied.dependency_state,
            work_contracts::DependencyState::Satisfied
        );

        let mut waived = dependency.clone();
        waived
            .waive(fixtures::dependency_waived_reason(), actor.clone())
            .expect("active -> waived should succeed");
        assert_eq!(
            waived.dependency_state,
            work_contracts::DependencyState::Waived
        );

        let mut cancelled = dependency.clone();
        cancelled
            .cancel(fixtures::dependency_cancelled_reason(), actor.clone())
            .expect("active -> cancelled should succeed");
        assert_eq!(
            cancelled.dependency_state,
            work_contracts::DependencyState::Cancelled
        );

        let mut proposed = WorkDependency::link(
            fixtures::work_dependency_id(),
            fixtures::formal_work_ref(),
            fixtures::downstream_formal_work_ref(),
            fixtures::dependency_reason(),
        )
        .expect("link should create proposed dependency");
        let err = proposed
            .activate(actor, fixtures::dependency_waived_reason())
            .expect_err("activate should reject wrong reason kind");
        assert_eq!(err, DomainError::PolicyRejected);
    }

    #[test]
    fn dependency_graph_policy_rejects_self_edge_cycle_and_terminal_reopen() {
        let actor = actor();
        let policy = DependencyGraphPolicy::from_graph(DependencyGraphSnapshot {
            project_ref: fixtures::project_ref(),
            dependency_edges: vec![(
                fixtures::downstream_formal_work_ref(),
                fixtures::formal_work_ref(),
            )],
            active_blockers: Vec::new(),
        });

        let err = DependencyGraphPolicy::assert_can_link(
            &policy.graph_snapshot,
            fixtures::formal_work_ref(),
            fixtures::formal_work_ref(),
        )
            .expect_err("self dependency must fail");
        assert_eq!(err, DomainError::PolicyRejected);

        let err = DependencyGraphPolicy::assert_can_link(
            &policy.graph_snapshot,
            fixtures::formal_work_ref(),
            fixtures::downstream_formal_work_ref(),
        )
            .expect_err("cycle dependency must fail");
        assert_eq!(err, DomainError::PolicyRejected);

        let mut dependency = WorkDependency::link(
            fixtures::work_dependency_id(),
            fixtures::formal_work_ref(),
            fixtures::downstream_formal_work_ref(),
            fixtures::dependency_reason(),
        )
        .expect("link should succeed");
        dependency
            .activate(actor.clone(), fixtures::dependency_activated_reason())
            .expect("activate should succeed");
        dependency
            .cancel(fixtures::dependency_cancelled_reason(), actor.clone())
            .expect("cancel should succeed");

        let err = dependency
            .activate(actor, fixtures::dependency_activated_reason())
            .expect_err("terminal dependency must not reopen");
        assert_eq!(err, DomainError::InvalidStateTransition);
    }

    #[test]
    fn blocker_state_transitions_follow_matrix() {
        let actor = actor();
        let mut blocker = WorkBlocker::open(
            fixtures::work_blocker_id(),
            fixtures::formal_work_ref(),
            fixtures::blocker_cause_ref(),
            actor.clone(),
        )
        .expect("open blocker should succeed");
        assert_eq!(blocker.blocker_state, work_contracts::BlockerState::Open);
        assert_eq!(blocker.resolved_evidence_ref, None);

        blocker
            .start_mitigation(
                work_contracts::BlockerMitigationReason {
                    reason_kind: work_contracts::BlockerMitigationReasonKind::PlanCreated,
                    reason_ref: None,
                },
                actor.clone(),
            )
            .expect("open -> mitigating should succeed");
        assert_eq!(
            blocker.blocker_state,
            work_contracts::BlockerState::Mitigating
        );

        blocker
            .resolve(fixtures::blocker_resolution_evidence_ref(), actor.clone())
            .expect("mitigating -> resolved should succeed");
        assert_eq!(
            blocker.blocker_state,
            work_contracts::BlockerState::Resolved
        );
        assert_eq!(
            blocker.resolved_evidence_ref,
            Some(fixtures::blocker_resolution_evidence_ref())
        );

        blocker
            .close(
                work_contracts::BlockerCloseReason {
                    reason_kind: work_contracts::BlockerCloseReasonKind::ResolvedVerified,
                    reason_ref: None,
                },
                actor.clone(),
            )
            .expect("resolved -> closed should succeed");
        assert_eq!(blocker.blocker_state, work_contracts::BlockerState::Closed);

        let err = blocker
            .resolve(fixtures::blocker_resolution_evidence_ref(), actor)
            .expect_err("closed blocker must not resolve again");
        assert_eq!(err, DomainError::InvalidStateTransition);
    }

    #[test]
    fn blocker_resolution_and_change_history_reject_invalid_inputs() {
        let actor = actor();
        let mut blocker = WorkBlocker::open(
            fixtures::work_blocker_id(),
            fixtures::formal_work_ref(),
            fixtures::blocker_cause_ref(),
            actor.clone(),
        )
        .expect("open blocker should succeed");

        let err = blocker
            .resolve(
                fixtures::unverified_blocker_resolution_evidence_ref(),
                actor.clone(),
            )
            .expect_err("unverified blocker evidence must fail");
        assert_eq!(err, DomainError::PolicyRejected);

        let dependency = WorkDependency::link(
            fixtures::work_dependency_id(),
            fixtures::formal_work_ref(),
            fixtures::downstream_formal_work_ref(),
            fixtures::dependency_reason(),
        )
        .expect("link should succeed");
        let history = DependencyChangeRecord::from_dependency_change(
            DependencyChangeId("dep-change-1".to_owned()),
            dependency,
            work_contracts::DependencyChangeReason::from_link_reason(fixtures::dependency_reason()),
        )
        .expect("dependency change history should build");
        assert_eq!(
            history.relation_ref,
            DependencyOrBlockerRef::Dependency(fixtures::work_dependency_ref())
        );

        let blocker_history = DependencyChangeRecord::from_blocker_change(
            DependencyChangeId("dep-change-2".to_owned()),
            blocker,
            work_contracts::DependencyChangeReason::from_blocker_cause(
                fixtures::blocker_cause_ref(),
            ),
        )
        .expect("blocker change history should build");
        assert_eq!(
            blocker_history.relation_ref,
            DependencyOrBlockerRef::Blocker(fixtures::work_blocker_ref())
        );
    }

    #[test]
    fn iteration_and_commitment_states_follow_matrix() {
        let actor = actor();
        let mut iteration = Iteration::open(
            fixtures::iteration_ref().iteration_id.clone(),
            fixtures::project_id(),
            fixtures::process_timebox_ref(),
            actor.clone(),
        )
        .expect("iteration open should succeed");
        assert_eq!(iteration.iteration_state, IterationState::Planning);

        IterationCommitmentPolicy::assert_commitment_allowed(
            &iteration,
            fixtures::formal_work_ref_set(),
        )
        .expect("planning iteration should accept candidates");

        let mut commitment = IterationCommitment::from_candidates(
            fixtures::iteration_commitment_id(),
            iteration.iteration_id.clone(),
            fixtures::formal_work_ref_set(),
            actor.clone(),
        )
        .expect("candidate commitment should build");
        assert_eq!(commitment.commitment_state, CommitmentState::Candidate);
        assert!(commitment.contains(fixtures::formal_work_ref()));

        iteration
            .commit(&mut commitment, actor.clone())
            .expect("planning -> committed should succeed");
        assert_eq!(iteration.iteration_state, IterationState::Committed);
        assert_eq!(commitment.commitment_state, CommitmentState::Committed);

        commitment
            .apply_change(
                fixtures::iteration_commitment_change_set(),
                fixtures::iteration_commitment_changed_reason(),
                actor.clone(),
            )
            .expect("committed -> changed should succeed");
        assert_eq!(commitment.commitment_state, CommitmentState::Changed);

        iteration
            .start(fixtures::iteration_started_reason(), actor.clone())
            .expect("committed -> in_progress should succeed");
        assert_eq!(iteration.iteration_state, IterationState::InProgress);

        let change_record = IterationChangeRecord::from_commitment(
            work_contracts::IterationChangeId("iter-change-1".to_owned()),
            iteration.clone(),
            commitment.clone(),
            actor.clone(),
        )
        .expect("iteration change record should build");
        assert_eq!(change_record.iteration_ref, fixtures::iteration_ref());

        iteration
            .close(fixtures::iteration_closed_reason(), actor.clone())
            .expect("in_progress -> closed should succeed");
        commitment
            .close(fixtures::iteration_closed_reason(), actor.clone())
            .expect("changed -> closed should succeed");
        assert_eq!(iteration.iteration_state, IterationState::Closed);
        assert_eq!(commitment.commitment_state, CommitmentState::Closed);

        let err = iteration
            .start(fixtures::iteration_started_reason(), actor)
            .expect_err("closed iteration must remain terminal");
        assert_eq!(err, DomainError::InvalidStateTransition);
    }

    #[test]
    fn iteration_reason_guards_reject_wrong_reason_shapes() {
        let actor = actor();
        let mut committed_iteration = Iteration::open(
            fixtures::iteration_ref().iteration_id.clone(),
            fixtures::project_id(),
            fixtures::process_timebox_ref(),
            actor.clone(),
        )
        .expect("iteration open should succeed");
        let mut committed_commitment = IterationCommitment::from_candidates(
            fixtures::iteration_commitment_id(),
            committed_iteration.iteration_id.clone(),
            fixtures::formal_work_ref_set(),
            actor.clone(),
        )
        .expect("candidate commitment should build");
        committed_iteration
            .commit(&mut committed_commitment, actor.clone())
            .expect("commit should succeed");

        let err = committed_iteration
            .start(fixtures::iteration_cancelled_reason(), actor.clone())
            .expect_err("wrong change reason kind should reject start");
        assert_eq!(err, DomainError::PolicyRejected);

        let mut planning_iteration = Iteration::open(
            fixtures::iteration_ref().iteration_id.clone(),
            fixtures::project_id(),
            fixtures::process_timebox_ref(),
            actor.clone(),
        )
        .expect("iteration open should succeed");
        planning_iteration
            .cancel(fixtures::iteration_cancelled_reason(), actor.clone())
            .expect("planning -> cancelled should succeed");
        assert_eq!(
            planning_iteration.iteration_state,
            IterationState::Cancelled
        );

        let mut in_progress_iteration = Iteration::open(
            fixtures::iteration_ref().iteration_id,
            fixtures::project_id(),
            fixtures::process_timebox_ref(),
            actor.clone(),
        )
        .expect("iteration open should succeed");
        let mut in_progress_commitment = IterationCommitment::from_candidates(
            fixtures::iteration_commitment_id(),
            in_progress_iteration.iteration_id.clone(),
            fixtures::formal_work_ref_set(),
            actor.clone(),
        )
        .expect("candidate commitment should build");
        in_progress_iteration
            .commit(&mut in_progress_commitment, actor.clone())
            .expect("commit should succeed");
        in_progress_iteration
            .start(fixtures::iteration_started_reason(), actor.clone())
            .expect("start should succeed");

        let err = in_progress_iteration
            .close(
                work_contracts::IterationCloseReason {
                    reason_kind: work_contracts::IterationCloseReasonKind::Completed,
                    reason_ref: None,
                },
                actor.clone(),
            )
            .expect_err("completed close without evidence ref should reject");
        assert_eq!(err, DomainError::PolicyRejected);

        let err = in_progress_commitment
            .apply_change(
                work_contracts::IterationCommitmentChangeSet {
                    add_work_refs: Vec::new(),
                    remove_work_refs: Vec::new(),
                },
                fixtures::iteration_commitment_changed_reason(),
                actor,
            )
            .expect_err("empty change set should reject");
        assert_eq!(err, DomainError::PolicyRejected);
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

        let promote_trace = WorkTraceRecord::from_truth_change(
            fixtures::trace_id(),
            fixtures::promote_result_recorded_change(),
            fixtures::trace_context_ref(),
        )
        .expect("promote trace should succeed");
        assert_eq!(
            promote_trace.subject_ref,
            WorkTraceSubjectRef::PromoteResult(fixtures::promote_result_ref())
        );

        let promote_outbox = WorkOutboxRecord::from_truth_change(
            fixtures::outbox_id(),
            fixtures::promote_result_recorded_change(),
        )
        .expect("promote outbox should succeed");
        assert_eq!(
            promote_outbox.event_kind,
            WorkOutboxEventKind::PromoteResultRecorded
        );

        let relation_trace = WorkTraceRecord::from_truth_change(
            fixtures::trace_id(),
            fixtures::dependency_changed_change(),
            fixtures::trace_context_ref(),
        )
        .expect("relation trace should succeed");
        assert_eq!(
            relation_trace.subject_ref,
            WorkTraceSubjectRef::Relation(DependencyOrBlockerRef::Dependency(
                fixtures::work_dependency_ref()
            ))
        );

        let relation_outbox = WorkOutboxRecord::from_truth_change(
            fixtures::outbox_id(),
            fixtures::dependency_changed_change(),
        )
        .expect("relation outbox should succeed");
        assert_eq!(
            relation_outbox.event_kind,
            WorkOutboxEventKind::WorkDependencyChanged
        );

        let blocker_outbox = WorkOutboxRecord::from_truth_change(
            fixtures::outbox_id(),
            fixtures::blocker_changed_change(),
        )
        .expect("blocker outbox should succeed");
        assert_eq!(
            blocker_outbox.event_kind,
            WorkOutboxEventKind::WorkBlockerChanged
        );

        let iteration_trace = WorkTraceRecord::from_truth_change(
            fixtures::trace_id(),
            fixtures::iteration_changed_change(),
            fixtures::trace_context_ref(),
        )
        .expect("iteration trace should succeed");
        assert_eq!(
            iteration_trace.subject_ref,
            WorkTraceSubjectRef::Iteration(fixtures::iteration_ref())
        );

        let iteration_outbox = WorkOutboxRecord::from_truth_change(
            fixtures::outbox_id(),
            fixtures::iteration_changed_change(),
        )
        .expect("iteration outbox should succeed");
        assert_eq!(
            iteration_outbox.event_kind,
            WorkOutboxEventKind::IterationChanged
        );

        let mut work_audit = WorkAuditTrail::start_for_subject(WorkAuditSubjectRef::FormalWork(
            fixtures::formal_work_ref(),
        ));
        work_audit
            .append(work_trace)
            .expect("work audit append should succeed");

        let mut promote_audit = WorkAuditTrail::start_for_subject(
            WorkAuditSubjectRef::PromoteResult(fixtures::promote_result_ref()),
        );
        promote_audit
            .append(promote_trace)
            .expect("promote audit append should succeed");

        let mut relation_audit = WorkAuditTrail::start_for_subject(WorkAuditSubjectRef::Relation(
            DependencyOrBlockerRef::Dependency(fixtures::work_dependency_ref()),
        ));
        relation_audit
            .append(relation_trace)
            .expect("relation audit append should succeed");

        let mut iteration_audit = WorkAuditTrail::start_for_subject(
            WorkAuditSubjectRef::Iteration(fixtures::iteration_ref()),
        );
        iteration_audit
            .append(iteration_trace)
            .expect("iteration audit append should succeed");
    }
}
