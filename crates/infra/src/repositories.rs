//! In-memory repositories and unit of work fake for `commit-02-b`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use core_contracts::metadata::{PageRequest, Version};
use work_application::{
    AuditRepository, BacklogRepository, DependencyRepository, FormalWorkRecord, FormalWorkScope,
    Page, PageInfo, ProjectMemberRepository, ProjectRepository, ProjectionRepository,
    PromoteRepository, ReferenceSnapshotRepository, RepositoryError, UnitOfWork, UnitOfWorkError,
    UnitOfWorkHandle, UnitOfWorkId, WorkItemRepository,
};
use work_contracts::{
    BacklogRef, DependencyOrBlockerRef, DerivedWorkViewRef, FormalWorkRef, GlobalMemberRef,
    ProjectMemberRef, ProjectOwnerRef, ProjectRef, PromoteResultRef, SourceWorkRef,
    WorkAuditSubjectRef, WorkBlockerRef, WorkDependencyRef, WorkTruthCursor,
};
use work_domain::{
    Backlog, ChildWorkItem, DependencyChangeRecord, DependencyGraphSnapshot,
    MemberCapabilitySnapshot, PendingPromoteIntake, ProjectMember, PromoteDecisionRecord,
    PromoteResult, TraceHandoffMarker, WorkAuditTrail, WorkBlocker, WorkDependency, WorkItem,
    WorkTraceRecord,
};

/// Shared in-memory fake stores for CORE command service tests.
#[derive(Clone, Default)]
pub struct InMemoryWorkStores {
    state: Arc<Mutex<Stores>>,
}

#[derive(Default)]
struct Stores {
    next_handle: u64,
    projects: HashMap<String, (work_domain::Project, Version)>,
    backlogs: HashMap<String, (Backlog, Version)>,
    backlog_by_project: HashMap<String, String>,
    backlog_membership: HashMap<String, Vec<FormalWorkRef>>,
    project_members: HashMap<String, (ProjectMember, Version)>,
    project_member_by_project_and_member: HashMap<(String, String), String>,
    work_items: HashMap<String, (WorkItem, Version)>,
    child_work_items: HashMap<String, (ChildWorkItem, Version)>,
    dependencies: HashMap<String, (WorkDependency, Version)>,
    blockers: HashMap<String, (WorkBlocker, Version)>,
    dependency_changes: Vec<DependencyChangeRecord>,
    promote_results: HashMap<String, (PromoteResult, Version)>,
    promote_latest_by_source: HashMap<String, String>,
    promote_decisions: Vec<PromoteDecisionRecord>,
    pending_promote_intakes: Vec<PendingPromoteIntake>,
    member_snapshots: HashMap<String, (MemberCapabilitySnapshot, Version)>,
    audit_trails: HashMap<String, (WorkAuditTrail, Version)>,
    traces: Vec<WorkTraceRecord>,
    stale_marks: Vec<(Vec<DerivedWorkViewRef>, WorkTruthCursor)>,
}

impl InMemoryWorkStores {
    /// Creates empty fake stores.
    pub fn new() -> Self {
        Self::default()
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
        _marker: TraceHandoffMarker,
        _uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError> {
        Ok(())
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
        match state.member_snapshots.get_mut(&snapshot.member_ref.0) {
            Some((stored, version)) => {
                if Some(*version) != expected_version {
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
}

#[async_trait]
impl ProjectionRepository for InMemoryWorkStores {
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
        state.stale_marks.push((affected, source_cursor));
        Ok(())
    }
}
