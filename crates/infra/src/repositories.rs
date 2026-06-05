//! In-memory repositories and unit of work fake for `commit-02-b`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use core_contracts::metadata::{PageRequest, Version};
use work_application::{
    AuditRepository, BacklogRepository, Page, PageInfo, ProjectMemberRepository, ProjectRepository,
    ProjectionRepository, ReferenceSnapshotRepository, RepositoryError, UnitOfWork,
    UnitOfWorkError, UnitOfWorkHandle, UnitOfWorkId,
};
use work_contracts::{
    BacklogRef, DerivedWorkViewRef, GlobalMemberRef, ProjectMemberRef, ProjectOwnerRef, ProjectRef,
    WorkAuditSubjectRef, WorkTruthCursor,
};
use work_domain::{
    Backlog, MemberCapabilitySnapshot, ProjectMember, TraceHandoffMarker, WorkAuditTrail,
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
    project_members: HashMap<String, (ProjectMember, Version)>,
    project_member_by_project_and_member: HashMap<(String, String), String>,
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
        let mut state = self
            .state
            .lock()
            .map_err(|_| RepositoryError::StoreUnavailable)?;
        if state.backlogs.contains_key(&backlog.backlog_id.0) {
            return Err(RepositoryError::VersionConflict);
        }
        state
            .backlog_by_project
            .insert(backlog.project_id.0.clone(), backlog.backlog_id.0.clone());
        state
            .backlogs
            .insert(backlog.backlog_id.0.clone(), (backlog, 1));
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
