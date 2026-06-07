//! In-memory repositories and unit of work fake for `commit-02-b`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use core_contracts::metadata::{PageRequest, Timestamp, Version};
use work_application::{
    ArchiveSummaryRepository, AuditRepository, BacklogRepository, DependencyRepository,
    FormalWorkRecord, FormalWorkScope, IterationRepository, IterationSummaryViewProjection,
    MemberWorkViewProjection, Page, PageInfo, ProjectBoardViewProjection, ProjectMemberRepository,
    ProjectRepository, ProjectionRepository, PromoteRepository, ReferenceSnapshotRepository,
    RepositoryError, UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, UnitOfWorkId,
    WorkItemRepository, WorkTruthSnapshotRepository,
};
use work_contracts::views::{
    IterationSummaryView, MemberWorkView, ProjectBoardView, ProjectProjectionBatch,
    ProjectWorkTruthSnapshot, WorkSearchProjection,
};
use work_contracts::{
    BacklogRef, DependencyOrBlockerRef, DerivedWorkViewRef, ExternalReferenceRef, FormalWorkRef,
    GlobalMemberRef, IterationRef, MethodDefinitionRef, ProjectMemberRef, ProjectOwnerRef,
    ProjectRef, PromoteResultRef, SourceWorkRef, WorkAuditSubjectRef, WorkBlockerRef,
    WorkDependencyRef, WorkReconciliationScopeKind, WorkReconciliationScopeRef, WorkSearchCriteria,
    WorkTraceId, WorkTraceSubjectRef, WorkTruthCursor,
};
use work_domain::{
    ArchiveHandoffMarker, Backlog, ChildWorkItem, DependencyChangeRecord, DependencyGraphSnapshot,
    DerivedWorkViewState, Iteration, IterationChangeRecord, IterationCommitment,
    MemberCapabilitySnapshot, MethodDefinitionSnapshot, PendingPromoteIntake, ProjectMember,
    ProjectionFailureReason, PromoteDecisionRecord, PromoteResult, ReferenceFailureReason,
    ReferenceResolutionState, TraceHandoffMarker, WorkArchiveSummarySet, WorkAuditTrail,
    WorkBlocker, WorkDependency, WorkItem, WorkTraceRecord,
};

/// Shared in-memory fake stores for CORE command service tests.
#[derive(Clone, Default)]
pub struct InMemoryWorkStores {
    state: Arc<Mutex<Stores>>,
}

#[derive(Default)]
struct Stores {
    next_handle: u64,
    strict_reference_versions: bool,
    projects: HashMap<String, (work_domain::Project, Version)>,
    backlogs: HashMap<String, (Backlog, Version)>,
    backlog_by_project: HashMap<String, String>,
    backlog_membership: HashMap<String, Vec<FormalWorkRef>>,
    project_members: HashMap<String, (ProjectMember, Version)>,
    project_member_by_project_and_member: HashMap<(String, String), String>,
    work_items: HashMap<String, (WorkItem, Version)>,
    child_work_items: HashMap<String, (ChildWorkItem, Version)>,
    iterations: HashMap<String, (Iteration, Version)>,
    commitments: HashMap<String, (IterationCommitment, Version)>,
    iteration_changes: Vec<IterationChangeRecord>,
    dependencies: HashMap<String, (WorkDependency, Version)>,
    blockers: HashMap<String, (WorkBlocker, Version)>,
    dependency_changes: Vec<DependencyChangeRecord>,
    promote_results: HashMap<String, (PromoteResult, Version)>,
    promote_latest_by_source: HashMap<String, String>,
    promote_decisions: Vec<PromoteDecisionRecord>,
    pending_promote_intakes: Vec<PendingPromoteIntake>,
    reference_states: HashMap<String, (ReferenceResolutionState, Version)>,
    member_snapshots: HashMap<String, (MemberCapabilitySnapshot, Version)>,
    method_snapshots: HashMap<String, (MethodDefinitionSnapshot, Version)>,
    affected_views_by_member: HashMap<String, Vec<DerivedWorkViewRef>>,
    affected_views_by_method: HashMap<String, Vec<DerivedWorkViewRef>>,
    affected_views_by_reference: HashMap<String, Vec<DerivedWorkViewRef>>,
    audit_trails: HashMap<String, (WorkAuditTrail, Version)>,
    traces: Vec<WorkTraceRecord>,
    trace_handoff_markers: HashMap<String, TraceHandoffMarker>,
    archive_handoff_markers: HashMap<String, ArchiveHandoffMarker>,
    project_board_views: HashMap<String, ProjectBoardViewProjection>,
    member_work_views: HashMap<String, MemberWorkViewProjection>,
    iteration_summary_views: HashMap<String, IterationSummaryViewProjection>,
    work_search_rows: HashMap<String, Vec<WorkSearchProjection>>,
    freshness_states: HashMap<String, DerivedWorkViewState>,
    stale_marks: Vec<(Vec<DerivedWorkViewRef>, WorkTruthCursor)>,
    rebuilding_marks: Vec<(Vec<DerivedWorkViewRef>, WorkTruthCursor)>,
    failed_marks: Vec<(
        Vec<DerivedWorkViewRef>,
        WorkTruthCursor,
        ProjectionFailureReason,
    )>,
}

impl InMemoryWorkStores {
    /// Creates empty fake stores.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables strict optimistic-version enforcement for reference states and snapshots.
    pub fn set_strict_reference_versions(&self, enabled: bool) {
        if let Ok(mut state) = self.state.lock() {
            state.strict_reference_versions = enabled;
        }
    }

    /// Returns the number of stored traces.
    pub fn trace_count(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.traces.len())
            .unwrap_or_default()
    }

    /// Returns the number of stale marker writes.
    pub fn stale_mark_count(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.stale_marks.len())
            .unwrap_or_default()
    }

    /// Returns all stored stale marker writes in append order.
    pub fn stale_marks(&self) -> Vec<(Vec<DerivedWorkViewRef>, WorkTruthCursor)> {
        self.state
            .lock()
            .map(|state| state.stale_marks.clone())
            .unwrap_or_default()
    }

    /// Returns all rebuilding marker writes in append order.
    pub fn rebuilding_marks(&self) -> Vec<(Vec<DerivedWorkViewRef>, WorkTruthCursor)> {
        self.state
            .lock()
            .map(|state| state.rebuilding_marks.clone())
            .unwrap_or_default()
    }

    /// Returns all failed marker writes in append order.
    pub fn failed_marks(
        &self,
    ) -> Vec<(
        Vec<DerivedWorkViewRef>,
        WorkTruthCursor,
        ProjectionFailureReason,
    )> {
        self.state
            .lock()
            .map(|state| state.failed_marks.clone())
            .unwrap_or_default()
    }

    /// Returns one stored project and version by id.
    pub fn project_snapshot(
        &self,
        project_ref: &ProjectRef,
    ) -> Option<(work_domain::Project, Version)> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.projects.get(&project_ref.project_id.0).cloned())
    }

    /// Returns one stored backlog and version by ref.
    pub fn backlog_snapshot(&self, backlog_ref: &BacklogRef) -> Option<(Backlog, Version)> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.backlogs.get(&backlog_ref.backlog_id.0).cloned())
    }

    /// Returns one stored project member and version by ref.
    pub fn project_member_snapshot(
        &self,
        project_member_ref: &ProjectMemberRef,
    ) -> Option<(ProjectMember, Version)> {
        self.state.lock().ok().and_then(|state| {
            state
                .project_members
                .get(&project_member_ref.project_member_id.0)
                .cloned()
        })
    }

    /// Returns one stored member snapshot and version by member ref.
    pub fn member_snapshot(
        &self,
        member_ref: &GlobalMemberRef,
    ) -> Option<(MemberCapabilitySnapshot, Version)> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.member_snapshots.get(&member_ref.0).cloned())
    }

    /// Returns one stored reference state and version by external reference key.
    pub fn reference_state_snapshot(
        &self,
        reference_ref: &ExternalReferenceRef,
    ) -> Option<(ReferenceResolutionState, Version)> {
        self.state.lock().ok().and_then(|state| {
            state
                .reference_states
                .get(&reference_key(reference_ref))
                .cloned()
        })
    }

    /// Returns one stored method snapshot and version by definition ref.
    pub fn method_snapshot(
        &self,
        definition_ref: &MethodDefinitionRef,
    ) -> Option<(MethodDefinitionSnapshot, Version)> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.method_snapshots.get(&definition_ref.0).cloned())
    }

    /// Returns one stored root work item and version by ref.
    pub fn work_item_snapshot(&self, work_ref: &FormalWorkRef) -> Option<(WorkItem, Version)> {
        let FormalWorkRef::WorkItem(work_item_id) = work_ref else {
            return None;
        };
        self.state
            .lock()
            .ok()
            .and_then(|state| state.work_items.get(&work_item_id.0).cloned())
    }

    /// Returns one stored child work item and version by ref.
    pub fn child_work_item_snapshot(
        &self,
        work_ref: &FormalWorkRef,
    ) -> Option<(ChildWorkItem, Version)> {
        let FormalWorkRef::ChildWorkItem(child_work_item_id) = work_ref else {
            return None;
        };
        self.state
            .lock()
            .ok()
            .and_then(|state| state.child_work_items.get(&child_work_item_id.0).cloned())
    }

    /// Returns one stored promote result and version by ref.
    pub fn promote_result_snapshot(
        &self,
        promote_result_ref: &PromoteResultRef,
    ) -> Option<(PromoteResult, Version)> {
        self.state.lock().ok().and_then(|state| {
            state
                .promote_results
                .get(&promote_result_ref.promote_result_id.0)
                .cloned()
        })
    }

    /// Returns all recorded promote decisions in append order.
    pub fn promote_decisions(&self) -> Vec<PromoteDecisionRecord> {
        self.state
            .lock()
            .map(|state| state.promote_decisions.clone())
            .unwrap_or_default()
    }

    /// Returns one stored iteration and version by ref.
    pub fn iteration_snapshot(&self, iteration_ref: &IterationRef) -> Option<(Iteration, Version)> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.iterations.get(&iteration_ref.iteration_id.0).cloned())
    }

    /// Returns one stored commitment and version by iteration ref.
    pub fn commitment_snapshot(
        &self,
        iteration_ref: &IterationRef,
    ) -> Option<(IterationCommitment, Version)> {
        self.state.lock().ok().and_then(|state| {
            state
                .commitments
                .get(&iteration_ref.iteration_id.0)
                .cloned()
        })
    }

    /// Returns all stored iteration change records in append order.
    pub fn iteration_changes(&self) -> Vec<IterationChangeRecord> {
        self.state
            .lock()
            .map(|state| state.iteration_changes.clone())
            .unwrap_or_default()
    }

    /// Returns all pending runtime promote intake markers.
    pub fn pending_promote_intakes(&self) -> Vec<PendingPromoteIntake> {
        self.state
            .lock()
            .map(|state| state.pending_promote_intakes.clone())
            .unwrap_or_default()
    }

    /// Returns the formal work membership for one backlog.
    pub fn backlog_membership(&self, backlog_ref: &BacklogRef) -> Vec<FormalWorkRef> {
        self.state
            .lock()
            .ok()
            .and_then(|state| {
                state
                    .backlog_membership
                    .get(&backlog_ref.backlog_id.0)
                    .cloned()
            })
            .unwrap_or_default()
    }

    /// Returns the project ref that owns one backlog id.
    pub fn project_ref_for_backlog(
        &self,
        backlog_id: &work_contracts::BacklogId,
    ) -> Option<ProjectRef> {
        self.state.lock().ok().and_then(|state| {
            state
                .backlogs
                .get(&backlog_id.0)
                .map(|(backlog, _)| ProjectRef {
                    project_id: backlog.project_id.clone(),
                })
        })
    }

    /// Seeds one member-work projection.
    pub fn seed_member_work_view(&self, projection: MemberWorkViewProjection) {
        if let Ok(mut state) = self.state.lock() {
            let key = projection.view.member_ref.project_member_id.0.clone();
            state
                .member_work_views
                .insert(key.clone(), projection.clone());
            state
                .freshness_states
                .insert(format!("member:{key}"), projection.freshness);
        }
    }

    /// Seeds one member-work projection from a public view marker.
    pub fn seed_member_work_public_view(&self, view: MemberWorkView) {
        self.seed_member_work_view(MemberWorkViewProjection {
            freshness: DerivedWorkViewState {
                view_ref: view.marker.view_ref.clone(),
                source_cursor: view.marker.source_cursor.clone(),
                freshness_state: view.marker.freshness_state,
            },
            view,
        });
    }

    /// Removes one member-work projection while leaving truth untouched.
    pub fn clear_member_work_view(&self, member_ref: &ProjectMemberRef) {
        if let Ok(mut state) = self.state.lock() {
            state
                .member_work_views
                .remove(&member_ref.project_member_id.0);
        }
    }

    /// Seeds one iteration-summary projection.
    pub fn seed_iteration_summary_view(&self, projection: IterationSummaryViewProjection) {
        if let Ok(mut state) = self.state.lock() {
            let key = projection.view.iteration_ref.iteration_id.0.clone();
            state
                .iteration_summary_views
                .insert(key.clone(), projection.clone());
            state
                .freshness_states
                .insert(format!("iteration:{key}"), projection.freshness);
        }
    }

    /// Seeds one iteration-summary projection from a public view marker.
    pub fn seed_iteration_summary_public_view(&self, view: IterationSummaryView) {
        self.seed_iteration_summary_view(IterationSummaryViewProjection {
            freshness: DerivedWorkViewState {
                view_ref: view.marker.view_ref.clone(),
                source_cursor: view.marker.source_cursor.clone(),
                freshness_state: view.marker.freshness_state,
            },
            view,
        });
    }

    /// Seeds one project-board projection.
    pub fn seed_project_board_view(&self, projection: ProjectBoardViewProjection) {
        if let Ok(mut state) = self.state.lock() {
            let key = projection.view.project_ref.project_id.0.clone();
            state
                .project_board_views
                .insert(key.clone(), projection.clone());
            state
                .freshness_states
                .insert(format!("project:{key}"), projection.freshness);
        }
    }

    /// Seeds one project-board projection from a public view marker.
    pub fn seed_project_board_public_view(&self, view: ProjectBoardView) {
        self.seed_project_board_view(ProjectBoardViewProjection {
            freshness: DerivedWorkViewState {
                view_ref: view.marker.view_ref.clone(),
                source_cursor: view.marker.source_cursor.clone(),
                freshness_state: view.marker.freshness_state,
            },
            view,
        });
    }

    /// Seeds search rows for one project.
    pub fn seed_search_rows(&self, project_ref: &ProjectRef, rows: Vec<WorkSearchProjection>) {
        if let Ok(mut state) = self.state.lock() {
            state
                .work_search_rows
                .insert(project_ref.project_id.0.clone(), rows);
        }
    }

    /// Seeds already-existing public affected views for one identity member.
    pub fn seed_affected_views_for_member(
        &self,
        member_ref: &GlobalMemberRef,
        view_refs: Vec<DerivedWorkViewRef>,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state
                .affected_views_by_member
                .insert(member_ref.0.clone(), view_refs);
        }
    }

    /// Seeds already-existing public affected views for one method definition.
    pub fn seed_affected_views_for_method(
        &self,
        definition_ref: &MethodDefinitionRef,
        view_refs: Vec<DerivedWorkViewRef>,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state
                .affected_views_by_method
                .insert(definition_ref.0.clone(), view_refs);
        }
    }

    /// Seeds already-existing public affected views for one external reference.
    pub fn seed_affected_views_for_reference(
        &self,
        reference_ref: &ExternalReferenceRef,
        view_refs: Vec<DerivedWorkViewRef>,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state
                .affected_views_by_reference
                .insert(reference_key(reference_ref), view_refs);
        }
    }
}

fn reference_key(reference_ref: &ExternalReferenceRef) -> String {
    match reference_ref {
        ExternalReferenceRef::Member(member_ref) => format!("member:{}", member_ref.0),
        ExternalReferenceRef::MethodDefinition(definition_ref) => {
            format!("method:{}", definition_ref.0)
        }
        ExternalReferenceRef::SourceWork(source_ref) => format!(
            "source:{}:{}",
            source_ref.external_ref.source_system as u8, source_ref.external_ref.external_id
        ),
        ExternalReferenceRef::Evidence(evidence_ref) => format!(
            "evidence:{}:{}",
            evidence_ref.external_ref.source_system as u8, evidence_ref.external_ref.external_id
        ),
        ExternalReferenceRef::ProcessTimebox(timebox_ref) => format!("timebox:{}", timebox_ref.0),
    }
}

#[async_trait]
impl UnitOfWork for InMemoryWorkStores {
    async fn begin(&self) -> Result<UnitOfWorkHandle, UnitOfWorkError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| UnitOfWorkError::BeginFailed)?;
        state.next_handle += 1;
        Ok(UnitOfWorkHandle {
            handle_id: UnitOfWorkId(format!("uow-{}", state.next_handle)),
        })
    }

    async fn commit(&self, _handle: UnitOfWorkHandle) -> Result<(), UnitOfWorkError> {
        Ok(())
    }

    async fn rollback(&self, _handle: UnitOfWorkHandle) -> Result<(), UnitOfWorkError> {
        Ok(())
    }
}

#[async_trait]
impl ProjectRepository for InMemoryWorkStores {
    async fn get(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<work_domain::Project>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .projects
            .get(&project_ref.project_id.0)
            .map(|(project, _)| project.clone()))
    }

    async fn list_by_owner(
        &self,
        owner_ref: ProjectOwnerRef,
        _page: PageRequest,
    ) -> Result<Page<work_domain::Project>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let items = state
            .projects
            .values()
            .filter_map(|(project, _)| (project.owner_ref == owner_ref).then_some(project.clone()))
            .collect();
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn create(
        &self,
        project: work_domain::Project,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state.projects.contains_key(&project.project_id.0) {
            return Err(RepositoryError::VersionConflict);
        }
        state
            .projects
            .insert(project.project_id.0.clone(), (project, 1));
        Ok(1)
    }

    async fn save(
        &self,
        project: work_domain::Project,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let entry = state
            .projects
            .get_mut(&project.project_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if entry.1 != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        entry.0 = project;
        entry.1 += 1;
        Ok(entry.1)
    }
}

#[async_trait]
impl BacklogRepository for InMemoryWorkStores {
    async fn get(&self, backlog_ref: BacklogRef) -> Result<Option<Backlog>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .backlogs
            .get(&backlog_ref.backlog_id.0)
            .map(|(backlog, _)| backlog.clone()))
    }

    async fn get_by_project(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<Backlog>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let Some(backlog_id) = state.backlog_by_project.get(&project_ref.project_id.0) else {
            return Ok(None);
        };
        Ok(state
            .backlogs
            .get(backlog_id)
            .map(|(backlog, _)| backlog.clone()))
    }

    async fn get_by_project_with_version(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<(Backlog, Version)>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let Some(backlog_id) = state.backlog_by_project.get(&project_ref.project_id.0) else {
            return Ok(None);
        };
        Ok(state.backlogs.get(backlog_id).cloned())
    }

    async fn create(
        &self,
        backlog: Backlog,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let backlog_id = backlog.backlog_id.0.clone();
        let project_id = backlog.project_id.0.clone();
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state.backlogs.contains_key(&backlog_id) {
            return Err(RepositoryError::VersionConflict);
        }
        state
            .backlog_by_project
            .insert(project_id, backlog_id.clone());
        state.backlogs.insert(backlog_id.clone(), (backlog, 1));
        state.backlog_membership.entry(backlog_id).or_default();
        Ok(1)
    }

    async fn save(
        &self,
        backlog: Backlog,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let entry = state
            .backlogs
            .get_mut(&backlog.backlog_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if entry.1 != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        entry.0 = backlog;
        entry.1 += 1;
        Ok(entry.1)
    }

    async fn contains_formal_work(
        &self,
        backlog_ref: BacklogRef,
        work_ref: FormalWorkRef,
    ) -> Result<bool, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .backlog_membership
            .get(&backlog_ref.backlog_id.0)
            .map(|refs| refs.contains(&work_ref))
            .unwrap_or(false))
    }

    async fn add_formal_work(
        &self,
        backlog_ref: BacklogRef,
        work_ref: FormalWorkRef,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        state
            .backlog_membership
            .entry(backlog_ref.backlog_id.0)
            .or_default()
            .push(work_ref);
        Ok(())
    }
}

#[async_trait]
impl WorkItemRepository for InMemoryWorkStores {
    async fn get_formal_work(
        &self,
        work_ref: FormalWorkRef,
    ) -> Result<Option<FormalWorkRecord>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(match work_ref {
            FormalWorkRef::WorkItem(work_item_id) => state
                .work_items
                .get(&work_item_id.0)
                .map(|(work_item, _)| FormalWorkRecord::WorkItem(work_item.clone())),
            FormalWorkRef::ChildWorkItem(child_work_item_id) => state
                .child_work_items
                .get(&child_work_item_id.0)
                .map(|(child, _)| FormalWorkRecord::ChildWorkItem(child.clone())),
        })
    }

    async fn get_formal_work_with_version(
        &self,
        work_ref: FormalWorkRef,
    ) -> Result<Option<(FormalWorkRecord, Version)>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(match work_ref {
            FormalWorkRef::WorkItem(work_item_id) => {
                state
                    .work_items
                    .get(&work_item_id.0)
                    .map(|(work_item, version)| {
                        (FormalWorkRecord::WorkItem(work_item.clone()), *version)
                    })
            }
            FormalWorkRef::ChildWorkItem(child_work_item_id) => state
                .child_work_items
                .get(&child_work_item_id.0)
                .map(|(child, version)| (FormalWorkRecord::ChildWorkItem(child.clone()), *version)),
        })
    }

    async fn get_formal_work_scope(
        &self,
        work_ref: FormalWorkRef,
    ) -> Result<Option<FormalWorkScope>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        match work_ref.clone() {
            FormalWorkRef::WorkItem(work_item_id) => {
                let Some((work_item, _)) = state.work_items.get(&work_item_id.0) else {
                    return Ok(None);
                };
                let Some((backlog, _)) = state.backlogs.get(&work_item.backlog_id.0) else {
                    return Ok(None);
                };
                Ok(Some(FormalWorkScope {
                    work_ref,
                    project_ref: ProjectRef {
                        project_id: backlog.project_id.clone(),
                    },
                    backlog_ref: backlog.backlog_ref(),
                    assignee_ref: Some(work_item.assignee_ref.clone()),
                }))
            }
            FormalWorkRef::ChildWorkItem(child_work_item_id) => {
                let Some((child, _)) = state.child_work_items.get(&child_work_item_id.0) else {
                    return Ok(None);
                };
                let Some((parent, _)) = state.work_items.get(&child.parent_work_item_id.0) else {
                    return Ok(None);
                };
                let Some((backlog, _)) = state.backlogs.get(&parent.backlog_id.0) else {
                    return Ok(None);
                };
                Ok(Some(FormalWorkScope {
                    work_ref,
                    project_ref: ProjectRef {
                        project_id: backlog.project_id.clone(),
                    },
                    backlog_ref: backlog.backlog_ref(),
                    assignee_ref: Some(parent.assignee_ref.clone()),
                }))
            }
        }
    }

    async fn list_by_backlog(
        &self,
        backlog_ref: BacklogRef,
        page: PageRequest,
    ) -> Result<Page<FormalWorkRef>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let items = state
            .backlog_membership
            .get(&backlog_ref.backlog_id.0)
            .cloned()
            .unwrap_or_default();

        let start = page
            .page_token
            .as_ref()
            .and_then(|token| token.as_str().parse::<usize>().ok())
            .unwrap_or(0);
        let limit = usize::try_from(page.limit.max(1)).unwrap_or(usize::MAX);
        let page_items = items
            .iter()
            .skip(start)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        let next = start + page_items.len();
        let has_more = next < items.len();

        Ok(Page {
            items: page_items,
            page_info: PageInfo {
                next_page_token: has_more
                    .then(|| core_contracts::metadata::PageToken::new(next.to_string())),
                has_more,
            },
        })
    }

    async fn create_work_item(
        &self,
        work_item: WorkItem,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state.work_items.contains_key(&work_item.work_item_id.0) {
            return Err(RepositoryError::VersionConflict);
        }
        state
            .work_items
            .insert(work_item.work_item_id.0.clone(), (work_item, 1));
        Ok(1)
    }

    async fn create_child_work_item(
        &self,
        child_work_item: ChildWorkItem,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state
            .child_work_items
            .contains_key(&child_work_item.child_work_item_id.0)
        {
            return Err(RepositoryError::VersionConflict);
        }
        state.child_work_items.insert(
            child_work_item.child_work_item_id.0.clone(),
            (child_work_item, 1),
        );
        Ok(1)
    }

    async fn save_formal_work(
        &self,
        record: FormalWorkRecord,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        match record {
            FormalWorkRecord::WorkItem(work_item) => {
                let entry = state
                    .work_items
                    .get_mut(&work_item.work_item_id.0)
                    .ok_or(RepositoryError::NotFound)?;
                if entry.1 != expected_version {
                    return Err(RepositoryError::VersionConflict);
                }
                entry.0 = work_item;
                entry.1 += 1;
                Ok(entry.1)
            }
            FormalWorkRecord::ChildWorkItem(child) => {
                let entry = state
                    .child_work_items
                    .get_mut(&child.child_work_item_id.0)
                    .ok_or(RepositoryError::NotFound)?;
                if entry.1 != expected_version {
                    return Err(RepositoryError::VersionConflict);
                }
                entry.0 = child;
                entry.1 += 1;
                Ok(entry.1)
            }
        }
    }
}

#[async_trait]
impl DependencyRepository for InMemoryWorkStores {
    async fn get_dependency(
        &self,
        dependency_ref: WorkDependencyRef,
    ) -> Result<Option<WorkDependency>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .dependencies
            .get(&dependency_ref.dependency_id.0)
            .map(|(dependency, _)| dependency.clone()))
    }

    async fn get_blocker(
        &self,
        blocker_ref: WorkBlockerRef,
    ) -> Result<Option<WorkBlocker>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .blockers
            .get(&blocker_ref.blocker_id.0)
            .map(|(blocker, _)| blocker.clone()))
    }

    async fn list_active_for_work(
        &self,
        work_ref: FormalWorkRef,
        _page: PageRequest,
    ) -> Result<Page<DependencyOrBlockerRef>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let mut items = Vec::new();
        for (dependency, _) in state.dependencies.values() {
            if matches!(
                dependency.dependency_state,
                work_contracts::DependencyState::Active
            ) && (dependency.upstream_work_ref == work_ref
                || dependency.downstream_work_ref == work_ref)
            {
                items.push(DependencyOrBlockerRef::Dependency(
                    dependency.dependency_ref(),
                ));
            }
        }
        for (blocker, _) in state.blockers.values() {
            if matches!(
                blocker.blocker_state,
                work_contracts::BlockerState::Open | work_contracts::BlockerState::Mitigating
            ) && blocker.blocked_work_ref == work_ref
            {
                items.push(DependencyOrBlockerRef::Blocker(blocker.blocker_ref()));
            }
        }
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn load_graph_snapshot(
        &self,
        project_ref: ProjectRef,
    ) -> Result<DependencyGraphSnapshot, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let dependency_edges = state
            .dependencies
            .values()
            .filter_map(|(dependency, _)| {
                matches!(
                    dependency.dependency_state,
                    work_contracts::DependencyState::Active
                )
                .then_some((
                    dependency.upstream_work_ref.clone(),
                    dependency.downstream_work_ref.clone(),
                ))
            })
            .collect();
        let active_blockers = state
            .blockers
            .values()
            .filter_map(|(blocker, _)| {
                matches!(
                    blocker.blocker_state,
                    work_contracts::BlockerState::Open | work_contracts::BlockerState::Mitigating
                )
                .then_some((blocker.blocked_work_ref.clone(), blocker.blocker_ref()))
            })
            .collect();
        Ok(DependencyGraphSnapshot {
            project_ref,
            dependency_edges,
            active_blockers,
        })
    }

    async fn create_dependency(
        &self,
        dependency: WorkDependency,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state.dependencies.contains_key(&dependency.dependency_id.0) {
            return Err(RepositoryError::VersionConflict);
        }
        state
            .dependencies
            .insert(dependency.dependency_id.0.clone(), (dependency, 1));
        Ok(1)
    }

    async fn save_dependency(
        &self,
        dependency: WorkDependency,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let entry = state
            .dependencies
            .get_mut(&dependency.dependency_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if entry.1 != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        entry.0 = dependency;
        entry.1 += 1;
        Ok(entry.1)
    }

    async fn create_blocker(
        &self,
        blocker: WorkBlocker,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state.blockers.contains_key(&blocker.blocker_id.0) {
            return Err(RepositoryError::VersionConflict);
        }
        state
            .blockers
            .insert(blocker.blocker_id.0.clone(), (blocker, 1));
        Ok(1)
    }

    async fn save_blocker(
        &self,
        blocker: WorkBlocker,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let entry = state
            .blockers
            .get_mut(&blocker.blocker_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if entry.1 != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        entry.0 = blocker;
        entry.1 += 1;
        Ok(entry.1)
    }

    async fn append_change(
        &self,
        change: DependencyChangeRecord,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        state.dependency_changes.push(change);
        Ok(())
    }
}

#[async_trait]
impl PromoteRepository for InMemoryWorkStores {
    async fn get(
        &self,
        promote_result_ref: PromoteResultRef,
    ) -> Result<Option<PromoteResult>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .promote_results
            .get(&promote_result_ref.promote_result_id.0)
            .map(|(result, _)| result.clone()))
    }

    async fn find_latest_by_source(
        &self,
        source_ref: SourceWorkRef,
    ) -> Result<Option<PromoteResult>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let Some(promote_result_id) = state
            .promote_latest_by_source
            .get(&source_ref.external_ref.external_id)
        else {
            return Ok(None);
        };
        Ok(state
            .promote_results
            .get(promote_result_id)
            .map(|(result, _)| result.clone()))
    }

    async fn create(
        &self,
        result: PromoteResult,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state
            .promote_results
            .contains_key(&result.promote_result_id.0)
        {
            return Err(RepositoryError::VersionConflict);
        }
        state.promote_latest_by_source.insert(
            result.source_ref.external_ref.external_id.clone(),
            result.promote_result_id.0.clone(),
        );
        state
            .promote_results
            .insert(result.promote_result_id.0.clone(), (result, 1));
        Ok(1)
    }

    async fn save(
        &self,
        result: PromoteResult,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let entry = state
            .promote_results
            .get_mut(&result.promote_result_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if entry.1 != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        entry.0 = result;
        entry.1 += 1;
        Ok(entry.1)
    }

    async fn append_decision(
        &self,
        decision: PromoteDecisionRecord,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        state.promote_decisions.push(decision);
        Ok(())
    }

    async fn save_pending_intake(
        &self,
        intake: PendingPromoteIntake,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        state.pending_promote_intakes.push(intake);
        Ok(())
    }
}

#[async_trait]
impl AuditRepository for InMemoryWorkStores {
    async fn get_audit_trail(
        &self,
        subject_ref: WorkAuditSubjectRef,
    ) -> Result<Option<WorkAuditTrail>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let key = format!("{subject_ref:?}");
        Ok(state.audit_trails.get(&key).map(|(trail, _)| trail.clone()))
    }

    async fn get_trace_record(
        &self,
        trace_id: WorkTraceId,
    ) -> Result<Option<WorkTraceRecord>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .traces
            .iter()
            .find(|record| record.trace_id == trace_id)
            .cloned())
    }

    async fn list_trace_records(
        &self,
        subject_ref: WorkTraceSubjectRef,
        _page: PageRequest,
    ) -> Result<Page<WorkTraceRecord>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let items = state
            .traces
            .iter()
            .filter(|record| record.subject_ref == subject_ref)
            .cloned()
            .collect();
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn append_trace(
        &self,
        record: WorkTraceRecord,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        state.traces.push(record);
        Ok(())
    }

    async fn save_audit_trail(
        &self,
        audit_trail: WorkAuditTrail,
        expected_version: Option<Version>,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let key = format!("{:?}", audit_trail.subject_ref);
        match state.audit_trails.get_mut(&key) {
            Some((stored, version)) => {
                if Some(*version) != expected_version {
                    return Err(RepositoryError::VersionConflict);
                }
                *stored = audit_trail;
                *version += 1;
                Ok(*version)
            }
            None => {
                if expected_version.is_some() {
                    return Err(RepositoryError::VersionConflict);
                }
                state.audit_trails.insert(key, (audit_trail, 1));
                Ok(1)
            }
        }
    }

    async fn save_trace_handoff_marker(
        &self,
        marker: TraceHandoffMarker,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        state
            .trace_handoff_markers
            .insert(marker.handoff_ref.0.clone(), marker);
        Ok(())
    }

    async fn get_trace_handoff_marker(
        &self,
        handoff_ref: work_contracts::TraceHandoffRef,
    ) -> Result<Option<TraceHandoffMarker>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state.trace_handoff_markers.get(&handoff_ref.0).cloned())
    }

    async fn save_archive_handoff_marker(
        &self,
        marker: ArchiveHandoffMarker,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        state
            .archive_handoff_markers
            .insert(marker.archive_ref.0.clone(), marker);
        Ok(())
    }
}

fn freshness_key(view_ref: &DerivedWorkViewRef) -> String {
    match &view_ref.scope_ref {
        work_contracts::DerivedWorkViewScopeRef::Project(project_ref) => {
            format!("project:{}", project_ref.project_id.0)
        }
        work_contracts::DerivedWorkViewScopeRef::ProjectMember(member_ref) => {
            format!("member:{}", member_ref.project_member_id.0)
        }
        work_contracts::DerivedWorkViewScopeRef::Iteration(iteration_ref) => {
            format!("iteration:{}", iteration_ref.iteration_id.0)
        }
        work_contracts::DerivedWorkViewScopeRef::Search(project_ref, digest) => {
            format!("search:{}:{}", project_ref.project_id.0, digest.0)
        }
    }
}

#[async_trait]
impl ProjectMemberRepository for InMemoryWorkStores {
    async fn get(
        &self,
        member_ref: ProjectMemberRef,
    ) -> Result<Option<ProjectMember>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .project_members
            .get(&member_ref.project_member_id.0)
            .map(|(member, _)| member.clone()))
    }

    async fn get_by_member(
        &self,
        project_ref: ProjectRef,
        member_ref: GlobalMemberRef,
    ) -> Result<Option<ProjectMember>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let Some(project_member_id) = state
            .project_member_by_project_and_member
            .get(&(project_ref.project_id.0, member_ref.0))
        else {
            return Ok(None);
        };
        Ok(state
            .project_members
            .get(project_member_id)
            .map(|(member, _)| member.clone()))
    }

    async fn list_by_project(
        &self,
        project_ref: ProjectRef,
        _page: PageRequest,
    ) -> Result<Page<ProjectMember>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let items = state
            .project_members
            .values()
            .filter_map(|(member, _)| {
                (member.project_id == project_ref.project_id).then_some(member.clone())
            })
            .collect();
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn list_by_member(
        &self,
        member_ref: GlobalMemberRef,
        _page: PageRequest,
    ) -> Result<Page<ProjectMember>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let mut items = state
            .project_members
            .values()
            .filter_map(|(member, _)| (member.member_ref == member_ref).then_some(member.clone()))
            .collect::<Vec<_>>();
        items.sort_by(|left, right| left.project_member_id.0.cmp(&right.project_member_id.0));
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn create(
        &self,
        project_member: ProjectMember,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state.project_member_by_project_and_member.contains_key(&(
            project_member.project_id.0.clone(),
            project_member.member_ref.0.clone(),
        )) {
            return Err(RepositoryError::VersionConflict);
        }
        state.project_member_by_project_and_member.insert(
            (
                project_member.project_id.0.clone(),
                project_member.member_ref.0.clone(),
            ),
            project_member.project_member_id.0.clone(),
        );
        state.project_members.insert(
            project_member.project_member_id.0.clone(),
            (project_member, 1),
        );
        Ok(1)
    }

    async fn save(
        &self,
        project_member: ProjectMember,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let entry = state
            .project_members
            .get_mut(&project_member.project_member_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if entry.1 != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        entry.0 = project_member;
        entry.1 += 1;
        Ok(entry.1)
    }
}

#[async_trait]
impl ReferenceSnapshotRepository for InMemoryWorkStores {
    async fn get_reference_state(
        &self,
        reference_ref: ExternalReferenceRef,
    ) -> Result<Option<ReferenceResolutionState>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .reference_states
            .get(&reference_key(&reference_ref))
            .map(|(snapshot, _)| snapshot.clone()))
    }

    async fn get_reference_state_with_version(
        &self,
        reference_ref: ExternalReferenceRef,
    ) -> Result<Option<(ReferenceResolutionState, Version)>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .reference_states
            .get(&reference_key(&reference_ref))
            .cloned())
    }

    async fn save_reference_state(
        &self,
        state_snapshot: ReferenceResolutionState,
        expected_version: Option<Version>,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let strict_reference_versions = state.strict_reference_versions;
        let key = reference_key(&state_snapshot.reference_ref);
        match state.reference_states.get_mut(&key) {
            Some((stored, version)) => {
                if strict_reference_versions {
                    if Some(*version) != expected_version {
                        return Err(RepositoryError::VersionConflict);
                    }
                } else if expected_version.is_some() && Some(*version) != expected_version {
                    return Err(RepositoryError::VersionConflict);
                }
                *stored = state_snapshot;
                *version += 1;
                Ok(*version)
            }
            None => {
                if expected_version.is_some() {
                    return Err(RepositoryError::VersionConflict);
                }
                state.reference_states.insert(key, (state_snapshot, 1));
                Ok(1)
            }
        }
    }

    async fn get_member_snapshot(
        &self,
        member_ref: GlobalMemberRef,
    ) -> Result<Option<MemberCapabilitySnapshot>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .member_snapshots
            .get(&member_ref.0)
            .map(|(snapshot, _)| snapshot.clone()))
    }

    async fn get_member_snapshot_with_version(
        &self,
        member_ref: GlobalMemberRef,
    ) -> Result<Option<(MemberCapabilitySnapshot, Version)>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state.member_snapshots.get(&member_ref.0).cloned())
    }

    async fn save_member_snapshot(
        &self,
        snapshot: MemberCapabilitySnapshot,
        expected_version: Option<Version>,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let strict_reference_versions = state.strict_reference_versions;
        match state.member_snapshots.get_mut(&snapshot.member_ref.0) {
            Some((stored, version)) => {
                if strict_reference_versions {
                    if Some(*version) != expected_version {
                        return Err(RepositoryError::VersionConflict);
                    }
                } else if expected_version.is_some() && Some(*version) != expected_version {
                    return Err(RepositoryError::VersionConflict);
                }
                *stored = snapshot;
                *version += 1;
                Ok(*version)
            }
            None => {
                if expected_version.is_some() {
                    return Err(RepositoryError::VersionConflict);
                }
                let key = snapshot.member_ref.0.clone();
                state.member_snapshots.insert(key, (snapshot, 1));
                Ok(1)
            }
        }
    }

    async fn get_method_snapshot(
        &self,
        definition_ref: MethodDefinitionRef,
    ) -> Result<Option<MethodDefinitionSnapshot>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .method_snapshots
            .get(&definition_ref.0)
            .map(|(snapshot, _)| snapshot.clone()))
    }

    async fn get_method_snapshot_with_version(
        &self,
        definition_ref: MethodDefinitionRef,
    ) -> Result<Option<(MethodDefinitionSnapshot, Version)>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state.method_snapshots.get(&definition_ref.0).cloned())
    }

    async fn save_method_snapshot(
        &self,
        snapshot: MethodDefinitionSnapshot,
        expected_version: Option<Version>,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let strict_reference_versions = state.strict_reference_versions;
        match state.method_snapshots.get_mut(&snapshot.definition_ref.0) {
            Some((stored, version)) => {
                if strict_reference_versions {
                    if Some(*version) != expected_version {
                        return Err(RepositoryError::VersionConflict);
                    }
                } else if expected_version.is_some() && Some(*version) != expected_version {
                    return Err(RepositoryError::VersionConflict);
                }
                *stored = snapshot;
                *version += 1;
                Ok(*version)
            }
            None => {
                if expected_version.is_some() {
                    return Err(RepositoryError::VersionConflict);
                }
                let key = snapshot.definition_ref.0.clone();
                state.method_snapshots.insert(key, (snapshot, 1));
                Ok(1)
            }
        }
    }

    async fn list_stale_references(
        &self,
        page: PageRequest,
    ) -> Result<Page<ExternalReferenceRef>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let mut refs = state
            .reference_states
            .values()
            .filter(|(snapshot, _)| {
                matches!(
                    snapshot.resolution_state,
                    work_contracts::ReferenceResolutionStatus::Stale
                        | work_contracts::ReferenceResolutionStatus::Failed
                )
            })
            .map(|(snapshot, _)| snapshot.reference_ref.clone())
            .collect::<Vec<_>>();
        refs.sort_by_key(reference_key);
        let start: usize = page
            .page_token
            .as_ref()
            .and_then(|token| token.as_str().parse::<usize>().ok())
            .unwrap_or(0);
        let limit = page.limit as usize;
        let end = start.saturating_add(limit).min(refs.len());
        let items = refs.get(start..end).unwrap_or(&[]).to_vec();
        let has_more = end < refs.len();
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: has_more
                    .then(|| core_contracts::metadata::PageToken::new(end.to_string())),
                has_more,
            },
        })
    }

    async fn list_project_references(
        &self,
        project_ref: ProjectRef,
        page: PageRequest,
    ) -> Result<Page<ExternalReferenceRef>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let mut refs = Vec::new();

        for (project, _) in state.projects.values() {
            if project.project_ref() == project_ref {
                if let Some(source_ref) = project.source_ref.clone() {
                    refs.push(ExternalReferenceRef::from_source_work(source_ref));
                }
            }
        }
        for (member, _) in state.project_members.values() {
            if member.project_id == project_ref.project_id {
                refs.push(ExternalReferenceRef::from_member(member.member_ref.clone()));
            }
        }
        for (work, _) in state.work_items.values() {
            let Some((backlog, _)) = state.backlogs.get(&work.backlog_id.0) else {
                continue;
            };
            if backlog.project_id != project_ref.project_id {
                continue;
            }
            refs.push(ExternalReferenceRef::from_source_work(
                work.source_ref.clone(),
            ));
            if let Some(definition_ref) = work.method_definition_ref.clone() {
                refs.push(ExternalReferenceRef::from_method_definition(definition_ref));
            }
            if let Some(evidence_ref) = work.completion_ref.clone() {
                refs.push(ExternalReferenceRef::from_evidence(evidence_ref));
            }
        }
        for (child, _) in state.child_work_items.values() {
            let Some((parent, _)) = state.work_items.get(&child.parent_work_item_id.0) else {
                continue;
            };
            let Some((backlog, _)) = state.backlogs.get(&parent.backlog_id.0) else {
                continue;
            };
            if backlog.project_id != project_ref.project_id {
                continue;
            }
            refs.push(ExternalReferenceRef::from_source_work(
                child.source_ref.clone(),
            ));
            if let Some(definition_ref) = child.method_definition_ref.clone() {
                refs.push(ExternalReferenceRef::from_method_definition(definition_ref));
            }
            if let Some(evidence_ref) = child.completion_ref.clone() {
                refs.push(ExternalReferenceRef::from_evidence(evidence_ref));
            }
        }
        for (result, _) in state.promote_results.values() {
            let Some(created_work_ref) = result.created_work_ref.clone() else {
                continue;
            };
            let work_project_matches = match created_work_ref {
                FormalWorkRef::WorkItem(work_item_id) => state
                    .work_items
                    .get(&work_item_id.0)
                    .and_then(|(work, _)| state.backlogs.get(&work.backlog_id.0))
                    .map(|(backlog, _)| backlog.project_id == project_ref.project_id)
                    .unwrap_or(false),
                FormalWorkRef::ChildWorkItem(child_id) => state
                    .child_work_items
                    .get(&child_id.0)
                    .and_then(|(child, _)| state.work_items.get(&child.parent_work_item_id.0))
                    .and_then(|(parent, _)| state.backlogs.get(&parent.backlog_id.0))
                    .map(|(backlog, _)| backlog.project_id == project_ref.project_id)
                    .unwrap_or(false),
            };
            if work_project_matches {
                refs.push(ExternalReferenceRef::from_source_work(
                    result.source_ref.clone(),
                ));
            }
        }
        for intake in &state.pending_promote_intakes {
            refs.push(ExternalReferenceRef::from_source_work(
                intake.source_ref.clone(),
            ));
        }
        for (dependency, _) in state.dependencies.values() {
            let scope_matches =
                project_scope_contains_work(&state, &project_ref, &dependency.upstream_work_ref)
                    || project_scope_contains_work(
                        &state,
                        &project_ref,
                        &dependency.downstream_work_ref,
                    );
            if scope_matches {
                // No persisted evidence field yet beyond satisfied transition contract.
            }
        }
        for (blocker, _) in state.blockers.values() {
            if project_scope_contains_work(&state, &project_ref, &blocker.blocked_work_ref) {
                if let Some(evidence_ref) = blocker.resolved_evidence_ref.clone() {
                    refs.push(ExternalReferenceRef::from_evidence(evidence_ref));
                }
                if let Some(evidence_ref) = blocker.cause_ref.evidence_ref.clone() {
                    refs.push(ExternalReferenceRef::from_evidence(evidence_ref));
                }
            }
        }
        for (iteration, _) in state.iterations.values() {
            if iteration.project_id == project_ref.project_id {
                refs.push(ExternalReferenceRef::from_process_timebox(
                    iteration.timebox_ref.clone(),
                ));
            }
        }

        refs.sort_by_key(reference_key);
        refs.dedup_by(|left, right| reference_key(left) == reference_key(right));
        Ok(paginate_refs(refs, page))
    }

    async fn mark_reference_failed(
        &self,
        reference_ref: ExternalReferenceRef,
        reason: ReferenceFailureReason,
        occurred_at: Timestamp,
        expected_version: Option<Version>,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state_snapshot = self
            .get_reference_state(reference_ref.clone())
            .await?
            .unwrap_or_else(|| ReferenceResolutionState::unresolved(reference_ref));
        state_snapshot
            .mark_failed(reason, occurred_at)
            .map_err(|_| RepositoryError::VersionConflict)?;
        self.save_reference_state(state_snapshot, expected_version, uow)
            .await
    }
}

#[async_trait]
impl IterationRepository for InMemoryWorkStores {
    async fn get_iteration(
        &self,
        iteration_ref: IterationRef,
    ) -> Result<Option<Iteration>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .iterations
            .get(&iteration_ref.iteration_id.0)
            .map(|(iteration, _)| iteration.clone()))
    }

    async fn get_commitment(
        &self,
        iteration_ref: IterationRef,
    ) -> Result<Option<IterationCommitment>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .commitments
            .get(&iteration_ref.iteration_id.0)
            .map(|(commitment, _)| commitment.clone()))
    }

    async fn get_commitment_with_version(
        &self,
        iteration_ref: IterationRef,
    ) -> Result<Option<(IterationCommitment, Version)>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .commitments
            .get(&iteration_ref.iteration_id.0)
            .cloned())
    }

    async fn list_by_project(
        &self,
        project_ref: ProjectRef,
        _page: PageRequest,
    ) -> Result<Page<Iteration>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let items = state
            .iterations
            .values()
            .filter_map(|(iteration, _)| {
                (iteration.project_id == project_ref.project_id).then_some(iteration.clone())
            })
            .collect();
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn create_iteration(
        &self,
        iteration: Iteration,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state.iterations.contains_key(&iteration.iteration_id.0) {
            return Err(RepositoryError::VersionConflict);
        }
        state
            .iterations
            .insert(iteration.iteration_id.0.clone(), (iteration, 1));
        Ok(1)
    }

    async fn save_iteration(
        &self,
        iteration: Iteration,
        expected_version: Version,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let entry = state
            .iterations
            .get_mut(&iteration.iteration_id.0)
            .ok_or(RepositoryError::NotFound)?;
        if entry.1 != expected_version {
            return Err(RepositoryError::VersionConflict);
        }
        entry.0 = iteration;
        entry.1 += 1;
        Ok(entry.1)
    }

    async fn save_commitment(
        &self,
        commitment: IterationCommitment,
        expected_version: Option<Version>,
        _uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        match state.commitments.get_mut(&commitment.iteration_id.0) {
            Some((stored, version)) => {
                if Some(*version) != expected_version {
                    return Err(RepositoryError::VersionConflict);
                }
                *stored = commitment;
                *version += 1;
                Ok(*version)
            }
            None => {
                if expected_version.is_some() {
                    return Err(RepositoryError::VersionConflict);
                }
                state
                    .commitments
                    .insert(commitment.iteration_id.0.clone(), (commitment, 1));
                Ok(1)
            }
        }
    }

    async fn append_change(
        &self,
        change: IterationChangeRecord,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        state.iteration_changes.push(change);
        Ok(())
    }
}

#[async_trait]
impl ProjectionRepository for InMemoryWorkStores {
    async fn get_project_board_view(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<ProjectBoardViewProjection>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .project_board_views
            .get(&project_ref.project_id.0)
            .cloned())
    }

    async fn get_member_work_view(
        &self,
        member_ref: ProjectMemberRef,
    ) -> Result<Option<MemberWorkViewProjection>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .member_work_views
            .get(&member_ref.project_member_id.0)
            .cloned())
    }

    async fn get_iteration_summary_view(
        &self,
        iteration_ref: IterationRef,
    ) -> Result<Option<IterationSummaryViewProjection>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .iteration_summary_views
            .get(&iteration_ref.iteration_id.0)
            .cloned())
    }

    async fn search_work(
        &self,
        project_ref: ProjectRef,
        _criteria: WorkSearchCriteria,
        _page: PageRequest,
    ) -> Result<Page<WorkSearchProjection>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(Page {
            items: state
                .work_search_rows
                .get(&project_ref.project_id.0)
                .cloned()
                .unwrap_or_default(),
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn get_freshness_state(
        &self,
        view_ref: DerivedWorkViewRef,
    ) -> Result<Option<DerivedWorkViewState>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        Ok(state
            .freshness_states
            .get(&freshness_key(&view_ref))
            .cloned())
    }

    async fn list_freshness_states(
        &self,
        scope_ref: WorkReconciliationScopeRef,
        page: PageRequest,
    ) -> Result<Page<DerivedWorkViewState>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let mut items = state
            .freshness_states
            .values()
            .filter(|view_state| match scope_ref.scope_kind {
                WorkReconciliationScopeKind::All => true,
                WorkReconciliationScopeKind::Project => {
                    scope_ref.project_ref.as_ref().is_none_or(|project_ref| {
                        matches_project_view(&view_state.view_ref, project_ref)
                    })
                }
                WorkReconciliationScopeKind::DerivedView => scope_ref
                    .view_ref
                    .as_ref()
                    .is_some_and(|view_ref| &view_state.view_ref == view_ref),
                WorkReconciliationScopeKind::ExternalReference => false,
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by_key(|state| freshness_key(&state.view_ref));
        paginate_states(items, page)
    }

    async fn list_views_affected_by_member(
        &self,
        member_ref: GlobalMemberRef,
        _page: PageRequest,
    ) -> Result<Page<DerivedWorkViewRef>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let mut items = state
            .affected_views_by_member
            .get(&member_ref.0)
            .cloned()
            .unwrap_or_default();
        items.sort_by_key(freshness_key);
        items.dedup();
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn list_views_affected_by_method(
        &self,
        definition_ref: MethodDefinitionRef,
        _page: PageRequest,
    ) -> Result<Page<DerivedWorkViewRef>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let mut items = state
            .affected_views_by_method
            .get(&definition_ref.0)
            .cloned()
            .unwrap_or_default();
        items.sort_by_key(freshness_key);
        items.dedup();
        Ok(Page {
            items,
            page_info: PageInfo {
                next_page_token: None,
                has_more: false,
            },
        })
    }

    async fn list_views_affected_by_references(
        &self,
        reference_refs: Vec<ExternalReferenceRef>,
        page: PageRequest,
    ) -> Result<Page<DerivedWorkViewRef>, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let mut ref_keys = reference_refs
            .into_iter()
            .map(|reference_ref| reference_key(&reference_ref))
            .collect::<Vec<_>>();
        ref_keys.sort();
        ref_keys.dedup();

        let mut items = Vec::new();
        for key in ref_keys {
            if let Some(view_refs) = state.affected_views_by_reference.get(&key) {
                items.extend(view_refs.clone());
            }
        }
        items.sort_by_key(freshness_key);
        items.dedup();
        paginate_view_refs(items, page)
    }

    async fn replace_project_views(
        &self,
        views: ProjectProjectionBatch,
        source_cursor: WorkTruthCursor,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let project_ids = views
            .board_views
            .iter()
            .map(|view| view.project_ref.project_id.0.clone())
            .chain(views.member_views.iter().filter_map(|view| {
                match &view.marker.view_ref.scope_ref {
                    work_contracts::DerivedWorkViewScopeRef::ProjectMember(member_ref) => state
                        .project_members
                        .get(&member_ref.project_member_id.0)
                        .map(|(member, _)| member.project_id.0.clone()),
                    _ => None,
                }
            }))
            .chain(views.iteration_views.iter().filter_map(|view| {
                state
                    .iterations
                    .get(&view.iteration_ref.iteration_id.0)
                    .map(|(iteration, _)| iteration.project_id.0.clone())
            }))
            .chain(
                views
                    .search_records
                    .iter()
                    .map(|row| row.project_ref.project_id.0.clone()),
            )
            .collect::<std::collections::BTreeSet<_>>();
        for project_id in project_ids {
            state.project_board_views.remove(&project_id);
            state.work_search_rows.remove(&project_id);
        }
        for view in views.board_views {
            let key = view.project_ref.project_id.0.clone();
            state.project_board_views.insert(
                key,
                ProjectBoardViewProjection {
                    freshness: DerivedWorkViewState {
                        view_ref: view.marker.view_ref.clone(),
                        source_cursor: source_cursor.clone(),
                        freshness_state: work_contracts::DerivedFreshnessState::Fresh,
                    },
                    view,
                },
            );
        }
        for view in views.member_views {
            let key = view.member_ref.project_member_id.0.clone();
            state.member_work_views.insert(
                key,
                MemberWorkViewProjection {
                    freshness: DerivedWorkViewState {
                        view_ref: view.marker.view_ref.clone(),
                        source_cursor: source_cursor.clone(),
                        freshness_state: work_contracts::DerivedFreshnessState::Fresh,
                    },
                    view,
                },
            );
        }
        for view in views.iteration_views {
            let key = view.iteration_ref.iteration_id.0.clone();
            state.iteration_summary_views.insert(
                key,
                IterationSummaryViewProjection {
                    freshness: DerivedWorkViewState {
                        view_ref: view.marker.view_ref.clone(),
                        source_cursor: source_cursor.clone(),
                        freshness_state: work_contracts::DerivedFreshnessState::Fresh,
                    },
                    view,
                },
            );
        }
        for row in views.search_records {
            state
                .work_search_rows
                .entry(row.project_ref.project_id.0.clone())
                .or_default()
                .push(row);
        }
        Ok(())
    }

    async fn mark_stale(
        &self,
        affected: Vec<DerivedWorkViewRef>,
        source_cursor: WorkTruthCursor,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        for view_ref in &affected {
            let key = freshness_key(view_ref);
            let freshness = state
                .freshness_states
                .entry(key)
                .or_insert_with(|| DerivedWorkViewState::for_view(view_ref.clone()));
            let _ = freshness.mark_stale(source_cursor.clone());
        }
        state.stale_marks.push((affected, source_cursor));
        Ok(())
    }

    async fn mark_rebuilding(
        &self,
        affected: Vec<DerivedWorkViewRef>,
        source_cursor: WorkTruthCursor,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        for view_ref in &affected {
            let key = freshness_key(view_ref);
            let freshness = state
                .freshness_states
                .entry(key)
                .or_insert_with(|| DerivedWorkViewState::for_view(view_ref.clone()));
            let _ = freshness.mark_rebuilding(source_cursor.clone());
        }
        state.rebuilding_marks.push((affected, source_cursor));
        Ok(())
    }

    async fn mark_failed(
        &self,
        affected: Vec<DerivedWorkViewRef>,
        source_cursor: WorkTruthCursor,
        reason: ProjectionFailureReason,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        for view_ref in &affected {
            let key = freshness_key(view_ref);
            let freshness = state
                .freshness_states
                .entry(key)
                .or_insert_with(|| DerivedWorkViewState::for_view(view_ref.clone()));
            let _ = freshness.mark_failed(source_cursor.clone(), reason.clone());
        }
        state.failed_marks.push((affected, source_cursor, reason));
        Ok(())
    }
}

#[async_trait]
impl WorkTruthSnapshotRepository for InMemoryWorkStores {
    async fn load_project_truth_snapshot(
        &self,
        project_ref: ProjectRef,
    ) -> Result<ProjectWorkTruthSnapshot, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let (project, _) = state
            .projects
            .get(&project_ref.project_id.0)
            .ok_or(RepositoryError::NotFound)?;
        let backlog = state
            .backlog_by_project
            .get(&project_ref.project_id.0)
            .and_then(|backlog_id| state.backlogs.get(backlog_id));

        let members = state
            .project_members
            .values()
            .filter(|(member, _)| member.project_id == project_ref.project_id)
            .map(
                |(member, _)| work_contracts::views::ProjectMemberTruthSummary {
                    project_member_ref: member.project_member_ref(),
                    project_ref: ProjectRef {
                        project_id: member.project_id.clone(),
                    },
                    member_ref: member.member_ref.clone(),
                    responsibility_state: member.responsibility_state,
                },
            )
            .collect::<Vec<_>>();

        let mut work_items = Vec::new();
        for (work, _) in state.work_items.values() {
            let Some((backlog, _)) = state.backlogs.get(&work.backlog_id.0) else {
                continue;
            };
            if backlog.project_id != project_ref.project_id {
                continue;
            }
            work_items.push(work_contracts::views::FormalWorkTruthSummary {
                work_ref: work.formal_work_ref(),
                project_ref: ProjectRef {
                    project_id: backlog.project_id.clone(),
                },
                backlog_ref: backlog.backlog_ref(),
                parent_ref: None,
                title: work.title.clone(),
                work_state: work.work_state,
                assignee_ref: Some(work.assignee_ref.clone()),
                source_ref: Some(work.source_ref.clone()),
                source_kind: Some(work.source_ref.source_kind),
                method_definition_ref: work.method_definition_ref.clone(),
                completion_ref: work.completion_ref.clone(),
                iteration_ref: find_iteration_ref_for_work(&state, &work.formal_work_ref()),
            });
        }
        for (child, _) in state.child_work_items.values() {
            let Some((parent, _)) = state.work_items.get(&child.parent_work_item_id.0) else {
                continue;
            };
            let Some((backlog, _)) = state.backlogs.get(&parent.backlog_id.0) else {
                continue;
            };
            if backlog.project_id != project_ref.project_id {
                continue;
            }
            work_items.push(work_contracts::views::FormalWorkTruthSummary {
                work_ref: child.formal_work_ref(),
                project_ref: ProjectRef {
                    project_id: backlog.project_id.clone(),
                },
                backlog_ref: backlog.backlog_ref(),
                parent_ref: Some(parent.formal_work_ref()),
                title: child.title.clone(),
                work_state: child.work_state,
                assignee_ref: Some(parent.assignee_ref.clone()),
                source_ref: Some(child.source_ref.clone()),
                source_kind: Some(child.source_ref.source_kind),
                method_definition_ref: child.method_definition_ref.clone(),
                completion_ref: child.completion_ref.clone(),
                iteration_ref: find_iteration_ref_for_work(&state, &child.formal_work_ref()),
            });
        }

        let relations = state
            .dependencies
            .values()
            .filter(|(dependency, _)| {
                project_scope_contains_work(&state, &project_ref, &dependency.upstream_work_ref)
                    || project_scope_contains_work(
                        &state,
                        &project_ref,
                        &dependency.downstream_work_ref,
                    )
            })
            .map(
                |(dependency, _)| work_contracts::views::WorkRelationTruthSummary {
                    relation_ref: DependencyOrBlockerRef::Dependency(dependency.dependency_ref()),
                    affected_work_refs: vec![
                        dependency.upstream_work_ref.clone(),
                        dependency.downstream_work_ref.clone(),
                    ],
                    relation_state: work_contracts::views::WorkRelationStateView::Dependency(
                        dependency.dependency_state,
                    ),
                },
            )
            .chain(state.blockers.values().filter_map(|(blocker, _)| {
                project_scope_contains_work(&state, &project_ref, &blocker.blocked_work_ref).then(
                    || work_contracts::views::WorkRelationTruthSummary {
                        relation_ref: DependencyOrBlockerRef::Blocker(blocker.blocker_ref()),
                        affected_work_refs: vec![blocker.blocked_work_ref.clone()],
                        relation_state: work_contracts::views::WorkRelationStateView::Blocker(
                            blocker.blocker_state,
                        ),
                    },
                )
            }))
            .collect::<Vec<_>>();

        let iterations = state
            .iterations
            .values()
            .filter(|(iteration, _)| iteration.project_id == project_ref.project_id)
            .map(|(iteration, _)| {
                let commitment = state.commitments.get(&iteration.iteration_id.0);
                work_contracts::views::IterationTruthSummary {
                    iteration_ref: iteration.iteration_ref(),
                    project_ref: ProjectRef {
                        project_id: iteration.project_id.clone(),
                    },
                    iteration_state: iteration.iteration_state,
                    commitment_state: commitment.as_ref().map(|(value, _)| value.commitment_state),
                    committed_work_refs: commitment
                        .as_ref()
                        .map(|(value, _)| value.committed_work_refs.refs.clone())
                        .unwrap_or_default(),
                }
            })
            .collect::<Vec<_>>();

        Ok(ProjectWorkTruthSnapshot {
            project: work_contracts::views::ProjectTruthSummary {
                project_ref: project.project_ref(),
                source_ref: project.source_ref.clone(),
                lifecycle_state: project.lifecycle_state,
                backlog_ref: backlog.map(|(value, _)| value.backlog_ref()),
            },
            backlog: backlog.map(|(value, _)| work_contracts::views::BacklogTruthSummary {
                backlog_ref: value.backlog_ref(),
                project_ref: ProjectRef {
                    project_id: value.project_id.clone(),
                },
                backlog_state: value.backlog_state,
            }),
            members,
            work_items,
            relations,
            iterations,
            source_cursor: WorkTruthCursor(project_ref.project_id.0),
        })
    }

    async fn load_truth_cursor(
        &self,
        project_ref: ProjectRef,
    ) -> Result<WorkTruthCursor, RepositoryError> {
        Ok(WorkTruthCursor(project_ref.project_id.0))
    }
}

#[async_trait]
impl ArchiveSummaryRepository for InMemoryWorkStores {
    async fn load_subject_archive_summaries(
        &self,
        subject_refs: Vec<WorkTraceSubjectRef>,
        source_cursor: Option<WorkTruthCursor>,
        _page: PageRequest,
    ) -> Result<WorkArchiveSummarySet, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        let trace_refs = state
            .traces
            .iter()
            .filter(|record| {
                subject_refs
                    .iter()
                    .any(|subject| record.subject_ref == *subject)
            })
            .map(|record| record.trace_id.clone())
            .collect::<Vec<_>>();
        let archive_scope = work_contracts::ArchiveHandoffScope {
            scope_kind: work_contracts::ArchiveHandoffScopeKind::Subjects,
            project_ref: None,
            subject_refs: subject_refs.clone(),
            source_cursor: source_cursor.clone(),
        };
        Ok(WorkArchiveSummarySet {
            archive_scope,
            truth_refs: subject_refs,
            trace_refs,
            source_cursor: source_cursor
                .unwrap_or_else(|| WorkTruthCursor("archive-subjects".to_owned())),
        })
    }

    async fn load_project_archive_summaries(
        &self,
        project_ref: ProjectRef,
        source_cursor: WorkTruthCursor,
        _page: PageRequest,
    ) -> Result<WorkArchiveSummarySet, RepositoryError> {
        let state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;

        let mut truth_refs = Vec::new();
        if state.projects.contains_key(&project_ref.project_id.0) {
            truth_refs.push(WorkTraceSubjectRef::Project(project_ref.clone()));
        }
        if let Some(backlog_id) = state.backlog_by_project.get(&project_ref.project_id.0) {
            if let Some((backlog, _)) = state.backlogs.get(backlog_id) {
                truth_refs.push(WorkTraceSubjectRef::Backlog(backlog.backlog_ref()));
            }
        }
        truth_refs.extend(
            state
                .project_members
                .values()
                .filter(|(member, _)| member.project_id == project_ref.project_id)
                .map(|(member, _)| WorkTraceSubjectRef::ProjectMember(member.project_member_ref())),
        );
        truth_refs.extend(state.work_items.values().filter_map(|(work, _)| {
            state
                .backlogs
                .get(&work.backlog_id.0)
                .filter(|(backlog, _)| backlog.project_id == project_ref.project_id)
                .map(|_| WorkTraceSubjectRef::FormalWork(work.formal_work_ref()))
        }));
        truth_refs.extend(state.child_work_items.values().filter_map(|(child, _)| {
            state
                .work_items
                .get(&child.parent_work_item_id.0)
                .and_then(|(parent, _)| state.backlogs.get(&parent.backlog_id.0))
                .filter(|(backlog, _)| backlog.project_id == project_ref.project_id)
                .map(|_| WorkTraceSubjectRef::FormalWork(child.formal_work_ref()))
        }));
        truth_refs.extend(state.dependencies.values().filter_map(|(dependency, _)| {
            (project_scope_contains_work(&state, &project_ref, &dependency.upstream_work_ref)
                || project_scope_contains_work(
                    &state,
                    &project_ref,
                    &dependency.downstream_work_ref,
                ))
            .then(|| {
                WorkTraceSubjectRef::Relation(DependencyOrBlockerRef::Dependency(
                    dependency.dependency_ref(),
                ))
            })
        }));
        truth_refs.extend(state.blockers.values().filter_map(|(blocker, _)| {
            project_scope_contains_work(&state, &project_ref, &blocker.blocked_work_ref).then(
                || {
                    WorkTraceSubjectRef::Relation(DependencyOrBlockerRef::Blocker(
                        blocker.blocker_ref(),
                    ))
                },
            )
        }));
        truth_refs.extend(
            state
                .iterations
                .values()
                .filter(|(iteration, _)| iteration.project_id == project_ref.project_id)
                .map(|(iteration, _)| WorkTraceSubjectRef::Iteration(iteration.iteration_ref())),
        );

        let trace_refs = state
            .traces
            .iter()
            .filter(|record| {
                truth_refs
                    .iter()
                    .any(|subject| record.subject_ref == *subject)
            })
            .map(|record| record.trace_id.clone())
            .collect::<Vec<_>>();
        let archive_scope = work_contracts::ArchiveHandoffScope {
            scope_kind: work_contracts::ArchiveHandoffScopeKind::ProjectCursor,
            project_ref: Some(project_ref),
            subject_refs: Vec::new(),
            source_cursor: Some(source_cursor.clone()),
        };

        Ok(WorkArchiveSummarySet {
            archive_scope,
            truth_refs,
            trace_refs,
            source_cursor,
        })
    }
}

fn paginate_refs(
    items: Vec<ExternalReferenceRef>,
    page: PageRequest,
) -> Page<ExternalReferenceRef> {
    let start = page
        .page_token
        .as_ref()
        .and_then(|token| token.as_str().parse::<usize>().ok())
        .unwrap_or(0);
    let limit = usize::try_from(page.limit.max(1)).unwrap_or(usize::MAX);
    let page_items = items
        .iter()
        .skip(start)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let next = start + page_items.len();
    let has_more = next < items.len();

    Page {
        items: page_items,
        page_info: PageInfo {
            next_page_token: has_more
                .then(|| core_contracts::metadata::PageToken::new(next.to_string())),
            has_more,
        },
    }
}

fn paginate_states(
    items: Vec<DerivedWorkViewState>,
    page: PageRequest,
) -> Result<Page<DerivedWorkViewState>, RepositoryError> {
    let start = page
        .page_token
        .as_ref()
        .and_then(|token| token.as_str().parse::<usize>().ok())
        .unwrap_or(0);
    let limit = usize::try_from(page.limit.max(1)).unwrap_or(usize::MAX);
    let page_items = items
        .iter()
        .skip(start)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let next = start + page_items.len();
    let has_more = next < items.len();
    Ok(Page {
        items: page_items,
        page_info: PageInfo {
            next_page_token: has_more
                .then(|| core_contracts::metadata::PageToken::new(next.to_string())),
            has_more,
        },
    })
}

fn paginate_view_refs(
    items: Vec<DerivedWorkViewRef>,
    page: PageRequest,
) -> Result<Page<DerivedWorkViewRef>, RepositoryError> {
    let start = page
        .page_token
        .as_ref()
        .and_then(|token| token.as_str().parse::<usize>().ok())
        .unwrap_or(0);
    let limit = usize::try_from(page.limit.max(1)).unwrap_or(usize::MAX);
    let page_items = items
        .iter()
        .skip(start)
        .take(limit)
        .cloned()
        .collect::<Vec<_>>();
    let next = start + page_items.len();
    let has_more = next < items.len();
    Ok(Page {
        items: page_items,
        page_info: PageInfo {
            next_page_token: has_more
                .then(|| core_contracts::metadata::PageToken::new(next.to_string())),
            has_more,
        },
    })
}

fn project_scope_contains_work(
    state: &Stores,
    project_ref: &ProjectRef,
    work_ref: &FormalWorkRef,
) -> bool {
    match work_ref {
        FormalWorkRef::WorkItem(work_item_id) => state
            .work_items
            .get(&work_item_id.0)
            .and_then(|(work, _)| state.backlogs.get(&work.backlog_id.0))
            .map(|(backlog, _)| backlog.project_id == project_ref.project_id)
            .unwrap_or(false),
        FormalWorkRef::ChildWorkItem(child_work_item_id) => state
            .child_work_items
            .get(&child_work_item_id.0)
            .and_then(|(child, _)| state.work_items.get(&child.parent_work_item_id.0))
            .and_then(|(parent, _)| state.backlogs.get(&parent.backlog_id.0))
            .map(|(backlog, _)| backlog.project_id == project_ref.project_id)
            .unwrap_or(false),
    }
}

fn matches_project_view(view_ref: &DerivedWorkViewRef, project_ref: &ProjectRef) -> bool {
    match &view_ref.scope_ref {
        work_contracts::DerivedWorkViewScopeRef::Project(candidate) => candidate == project_ref,
        work_contracts::DerivedWorkViewScopeRef::ProjectMember(_) => true,
        work_contracts::DerivedWorkViewScopeRef::Iteration(_) => true,
        work_contracts::DerivedWorkViewScopeRef::Search(candidate, _) => candidate == project_ref,
    }
}

fn find_iteration_ref_for_work(state: &Stores, work_ref: &FormalWorkRef) -> Option<IterationRef> {
    state
        .commitments
        .iter()
        .find(|(_, (commitment, _))| commitment.committed_work_refs.refs.contains(work_ref))
        .map(|(iteration_id, _)| IterationRef {
            iteration_id: work_contracts::IterationId(iteration_id.clone()),
        })
}
