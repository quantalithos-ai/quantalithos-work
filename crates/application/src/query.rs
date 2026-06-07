//! Authorized Work query service and visibility helpers.

use core_contracts::actor::ActorContext;
use core_contracts::metadata::PageRequest;

use crate::{
    ActorMemberResolverPort, ApplicationError, BacklogRepository, DependencyRepository,
    FormalWorkRecord, FormalWorkScope, IterationRepository, ProjectMemberRepository,
    ProjectRepository, ProjectionRepository, QueryActorMemberRef, RepositoryError,
    WorkItemRepository,
};
use work_contracts::views::{
    BacklogView, FormalWorkSummaryView, IterationSummaryView, MemberWorkView,
    ProjectMemberSummaryView, ProjectWorkFactsView, WorkItemView,
};
use work_contracts::{
    DerivedFreshnessState, GetBacklogRequest, GetIterationSummaryRequest,
    GetProjectWorkFactsRequest, GetWorkItemRequest, ListMemberWorkRequest, ProjectMemberRef,
    ProjectMemberResponsibilityState, ProjectionViewMarker, PublicPageInfo, QuerySurface,
    WorkQueryEnvelope, WorkQueryResponse,
};
use work_domain::{Iteration, ProjectMember};

/// Application helper for Work query visibility decisions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkQueryVisibilityPolicy;

/// Authorized, read-only query service for Work.
pub struct AuthorizedWorkQueryService<PJ, PM, B, W, D, I, PROJ, AMR> {
    /// Project truth repository.
    pub project_repo: PJ,
    /// Project member truth repository.
    pub member_repo: PM,
    /// Backlog repository.
    pub backlog_repo: B,
    /// Formal work truth repository.
    pub work_repo: W,
    /// Dependency repository.
    pub dependency_repo: D,
    /// Iteration repository.
    pub iteration_repo: I,
    /// Projection repository.
    pub projection_repo: PROJ,
    /// Query actor-member resolver.
    pub actor_member_resolver: AMR,
    /// Stateless visibility helper.
    pub visibility: WorkQueryVisibilityPolicy,
}

impl<PJ, PM, B, W, D, I, PROJ, AMR> AuthorizedWorkQueryService<PJ, PM, B, W, D, I, PROJ, AMR>
where
    PJ: ProjectRepository,
    PM: ProjectMemberRepository,
    B: BacklogRepository,
    W: WorkItemRepository,
    D: DependencyRepository,
    I: IterationRepository,
    PROJ: ProjectionRepository,
    AMR: ActorMemberResolverPort,
{
    /// Reads project facts from committed truth after visibility checks.
    pub async fn get_project_work_facts(
        &self,
        envelope: WorkQueryEnvelope<GetProjectWorkFactsRequest>,
    ) -> Result<WorkQueryResponse<ProjectWorkFactsView>, ApplicationError> {
        let project = self
            .project_repo
            .get(envelope.query.project_ref.clone())
            .await
            .map_err(Self::map_repo_error)?;
        let Some(project) = project else {
            return Ok(missing_response());
        };

        if let Err(error) = Self::authorize_project_read(
            &envelope.actor,
            project.project_ref(),
            &self.actor_member_resolver,
            &self.member_repo,
        )
        .await
        {
            return Self::visibility_surface(error);
        }

        let members = self
            .member_repo
            .list_by_project(project.project_ref(), Self::default_page())
            .await
            .map_err(Self::map_repo_error)?;
        let backlog = self
            .backlog_repo
            .get_by_project(project.project_ref())
            .await
            .map_err(Self::map_repo_error)?;

        let mut formal_work = Vec::new();
        let mut relations = Vec::new();
        let backlog_ref = backlog.as_ref().map(|backlog| backlog.backlog_ref());
        if let Some(backlog) = backlog {
            let work_refs = self
                .work_repo
                .list_by_backlog(backlog.backlog_ref(), Self::default_page())
                .await
                .map_err(Self::map_repo_error)?;
            for work_ref in work_refs.items {
                let Some(record) = self
                    .work_repo
                    .get_formal_work(work_ref.clone())
                    .await
                    .map_err(Self::map_repo_error)?
                else {
                    continue;
                };
                formal_work.push(Self::formal_work_summary_from_record(&record));

                relations.extend(self.load_relation_summaries(work_ref).await?);
            }
        }

        Ok(visible_response(ProjectWorkFactsView {
            project_ref: project.project_ref(),
            lifecycle_state: project.lifecycle_state,
            backlog_ref,
            members: members
                .items
                .into_iter()
                .map(Self::project_member_summary)
                .collect(),
            formal_work,
            relations,
        }))
    }

    /// Reads a backlog page from committed truth after visibility checks.
    pub async fn get_backlog(
        &self,
        envelope: WorkQueryEnvelope<GetBacklogRequest>,
    ) -> Result<WorkQueryResponse<BacklogView>, ApplicationError> {
        let project = self
            .project_repo
            .get(envelope.query.project_ref.clone())
            .await
            .map_err(Self::map_repo_error)?;
        let Some(project) = project else {
            return Ok(missing_response());
        };

        if let Err(error) = Self::authorize_project_read(
            &envelope.actor,
            project.project_ref(),
            &self.actor_member_resolver,
            &self.member_repo,
        )
        .await
        {
            return Self::visibility_surface(error);
        }

        let backlog = self
            .backlog_repo
            .get_by_project(project.project_ref())
            .await
            .map_err(Self::map_repo_error)?;
        let Some(backlog) = backlog else {
            return Ok(missing_response());
        };

        let refs_page = self
            .work_repo
            .list_by_backlog(
                backlog.backlog_ref(),
                envelope
                    .metadata
                    .page
                    .clone()
                    .unwrap_or_else(Self::default_page),
            )
            .await
            .map_err(Self::map_repo_error)?;
        let items = self
            .load_backlog_items(refs_page.items, envelope.query.filter.as_ref())
            .await?;

        let view = BacklogView {
            backlog_ref: backlog.backlog_ref(),
            project_ref: project.project_ref(),
            backlog_state: backlog.backlog_state,
            items,
            page: Self::public_page_info(refs_page.page_info),
        };

        if view.items.is_empty() {
            return Ok(empty_response(view));
        }
        Ok(visible_response(view))
    }

    /// Reads one formal work record after visibility checks.
    pub async fn get_work_item(
        &self,
        envelope: WorkQueryEnvelope<GetWorkItemRequest>,
    ) -> Result<WorkQueryResponse<WorkItemView>, ApplicationError> {
        let record = self
            .work_repo
            .get_formal_work(envelope.query.work_ref.clone())
            .await
            .map_err(Self::map_repo_error)?;
        let Some(record) = record else {
            return Ok(missing_response());
        };

        let scope = self
            .work_repo
            .get_formal_work_scope(record.formal_work_ref())
            .await
            .map_err(Self::map_repo_error)?;
        let Some(scope) = scope else {
            return Ok(not_visible_response());
        };

        if let Err(error) = Self::authorize_work_read(
            &envelope.actor,
            &scope,
            &self.actor_member_resolver,
            &self.member_repo,
        )
        .await
        {
            return Self::visibility_surface(error);
        }

        let relations = self
            .load_relation_summaries(record.formal_work_ref())
            .await?;

        Ok(visible_response(Self::work_item_view(record, relations)))
    }

    /// Reads one member-work projection after visibility checks.
    pub async fn list_member_work(
        &self,
        envelope: WorkQueryEnvelope<ListMemberWorkRequest>,
    ) -> Result<WorkQueryResponse<MemberWorkView>, ApplicationError> {
        let member = self
            .member_repo
            .get(envelope.query.project_member_ref.clone())
            .await
            .map_err(Self::map_repo_error)?;
        let Some(member) = member else {
            return Ok(missing_response());
        };

        if let Err(error) = Self::authorize_member_work_read(
            &envelope.actor,
            &member,
            &self.actor_member_resolver,
            &self.member_repo,
        )
        .await
        {
            return Self::visibility_surface(error);
        }

        let projection = self
            .projection_repo
            .get_member_work_view(member.project_member_ref())
            .await
            .map_err(Self::map_repo_error)?;
        let Some(projection) = projection else {
            return Ok(rebuilding_response());
        };

        let assigned_work = projection
            .view
            .assigned_work
            .into_iter()
            .filter(|summary| {
                envelope
                    .query
                    .work_state
                    .is_none_or(|work_state| summary.work_state == work_state)
            })
            .collect();
        let view = MemberWorkView {
            member_ref: projection.view.member_ref,
            assigned_work,
            marker: Self::projection_marker(&projection.freshness),
            page: projection.view.page,
        };
        Ok(Self::map_projection_surface(
            projection.freshness.freshness_state,
            view,
        ))
    }

    /// Reads one iteration summary projection after visibility checks.
    pub async fn get_iteration_summary(
        &self,
        envelope: WorkQueryEnvelope<GetIterationSummaryRequest>,
    ) -> Result<WorkQueryResponse<IterationSummaryView>, ApplicationError> {
        let iteration = self
            .iteration_repo
            .get_iteration(envelope.query.iteration_ref.clone())
            .await
            .map_err(Self::map_repo_error)?;
        let Some(iteration) = iteration else {
            return Ok(missing_response());
        };

        if let Err(error) = Self::authorize_iteration_read(
            &envelope.actor,
            &iteration,
            &self.actor_member_resolver,
            &self.member_repo,
        )
        .await
        {
            return Self::visibility_surface(error);
        }

        let projection = self
            .projection_repo
            .get_iteration_summary_view(iteration.iteration_ref())
            .await
            .map_err(Self::map_repo_error)?;
        let Some(projection) = projection else {
            return Ok(missing_response());
        };

        let view = IterationSummaryView {
            iteration_ref: projection.view.iteration_ref,
            iteration_state: projection.view.iteration_state,
            commitment_state: projection.view.commitment_state,
            committed_work: projection.view.committed_work,
            marker: Self::projection_marker(&projection.freshness),
        };
        Ok(Self::map_projection_surface(
            projection.freshness.freshness_state,
            view,
        ))
    }

    fn project_member_summary(member: ProjectMember) -> ProjectMemberSummaryView {
        ProjectMemberSummaryView {
            project_member_ref: member.project_member_ref(),
            member_ref: member.member_ref,
            responsibility_state: member.responsibility_state,
        }
    }

    fn formal_work_summary_from_record(record: &FormalWorkRecord) -> FormalWorkSummaryView {
        match record {
            FormalWorkRecord::WorkItem(work_item) => FormalWorkSummaryView {
                work_ref: work_item.formal_work_ref(),
                work_state: work_item.work_state,
                assignee_ref: Some(work_item.assignee_ref.clone()),
                completion_ref: work_item.completion_ref.clone(),
            },
            FormalWorkRecord::ChildWorkItem(child) => FormalWorkSummaryView {
                work_ref: child.formal_work_ref(),
                work_state: child.work_state,
                assignee_ref: None,
                completion_ref: child.completion_ref.clone(),
            },
        }
    }

    fn work_item_view(
        record: FormalWorkRecord,
        relations: Vec<work_contracts::WorkRelationSummaryView>,
    ) -> WorkItemView {
        match record {
            FormalWorkRecord::WorkItem(work_item) => WorkItemView {
                work_ref: work_item.formal_work_ref(),
                parent_ref: None,
                work_state: work_item.work_state,
                assignee_ref: Some(work_item.assignee_ref),
                source_ref: None,
                completion_ref: work_item.completion_ref,
                relations,
            },
            FormalWorkRecord::ChildWorkItem(child) => WorkItemView {
                work_ref: child.formal_work_ref(),
                parent_ref: Some(work_contracts::FormalWorkRef::WorkItem(
                    child.parent_work_item_id,
                )),
                work_state: child.work_state,
                assignee_ref: None,
                source_ref: Some(child.source_ref),
                completion_ref: child.completion_ref,
                relations,
            },
        }
    }

    async fn load_backlog_items(
        &self,
        work_refs: Vec<work_contracts::FormalWorkRef>,
        filter: Option<&work_contracts::BacklogQueryFilter>,
    ) -> Result<Vec<FormalWorkSummaryView>, ApplicationError> {
        let mut items = Vec::new();
        for work_ref in work_refs {
            let Some(record) = self
                .work_repo
                .get_formal_work(work_ref)
                .await
                .map_err(Self::map_repo_error)?
            else {
                continue;
            };
            let summary = Self::formal_work_summary_from_record(&record);
            let allowed = filter.is_none_or(|filter| {
                filter
                    .work_state
                    .is_none_or(|work_state| summary.work_state == work_state)
                    && filter
                        .assignee_ref
                        .as_ref()
                        .is_none_or(|assignee| summary.assignee_ref.as_ref() == Some(assignee))
            });
            if allowed {
                items.push(summary);
            }
        }
        Ok(items)
    }

    async fn load_relation_summaries(
        &self,
        work_ref: work_contracts::FormalWorkRef,
    ) -> Result<Vec<work_contracts::WorkRelationSummaryView>, ApplicationError> {
        let relations = self
            .dependency_repo
            .list_active_for_work(work_ref, Self::default_page())
            .await
            .map_err(Self::map_repo_error)?;

        let mut summaries = Vec::new();
        for relation_ref in relations.items {
            match relation_ref.clone() {
                work_contracts::DependencyOrBlockerRef::Dependency(dependency_ref) => {
                    let Some(dependency) = self
                        .dependency_repo
                        .get_dependency(dependency_ref)
                        .await
                        .map_err(Self::map_repo_error)?
                    else {
                        continue;
                    };
                    summaries.push(work_contracts::WorkRelationSummaryView {
                        relation_ref,
                        affected_work_refs: vec![
                            dependency.upstream_work_ref,
                            dependency.downstream_work_ref,
                        ],
                        relation_state: work_contracts::WorkRelationStateView::Dependency(
                            dependency.dependency_state,
                        ),
                    });
                }
                work_contracts::DependencyOrBlockerRef::Blocker(blocker_ref) => {
                    let Some(blocker) = self
                        .dependency_repo
                        .get_blocker(blocker_ref)
                        .await
                        .map_err(Self::map_repo_error)?
                    else {
                        continue;
                    };
                    summaries.push(work_contracts::WorkRelationSummaryView {
                        relation_ref,
                        affected_work_refs: vec![blocker.blocked_work_ref],
                        relation_state: work_contracts::WorkRelationStateView::Blocker(
                            blocker.blocker_state,
                        ),
                    });
                }
            }
        }

        Ok(summaries)
    }

    fn projection_marker(freshness: &work_domain::DerivedWorkViewState) -> ProjectionViewMarker {
        ProjectionViewMarker {
            view_ref: freshness.view_ref.clone(),
            source_cursor: freshness.source_cursor.clone(),
            freshness_state: freshness.freshness_state,
        }
    }

    fn map_projection_surface<T>(
        freshness_state: DerivedFreshnessState,
        view: T,
    ) -> WorkQueryResponse<T> {
        match freshness_state {
            DerivedFreshnessState::Fresh => visible_response(view),
            DerivedFreshnessState::Stale => stale_response(view),
            DerivedFreshnessState::Rebuilding => rebuilding_with_response(view),
            DerivedFreshnessState::Failed => failed_response(view),
        }
    }

    fn visibility_surface<T>(
        error: ApplicationError,
    ) -> Result<WorkQueryResponse<T>, ApplicationError> {
        match error {
            ApplicationError::NotVisible => Ok(not_visible_response()),
            other => Err(other),
        }
    }

    fn public_page_info(page_info: crate::PageInfo) -> PublicPageInfo {
        PublicPageInfo {
            next_page_token: page_info.next_page_token,
            has_more: page_info.has_more,
        }
    }

    fn default_page() -> PageRequest {
        PageRequest {
            limit: 50,
            page_token: None,
        }
    }

    fn map_repo_error(error: RepositoryError) -> ApplicationError {
        match error {
            RepositoryError::NotFound => ApplicationError::NotFound,
            RepositoryError::VersionConflict
            | RepositoryError::TransactionRejected
            | RepositoryError::StoreUnavailable => ApplicationError::TemporarilyUnavailable,
        }
    }

    fn member_is_visible(member: &ProjectMember) -> bool {
        matches!(
            member.responsibility_state,
            ProjectMemberResponsibilityState::Active | ProjectMemberResponsibilityState::Paused
        )
    }

    async fn resolve_query_actor_member(
        actor: &ActorContext,
        actor_member_resolver: &dyn ActorMemberResolverPort,
    ) -> Result<QueryActorMemberRef, ApplicationError> {
        actor_member_resolver
            .resolve_actor_member(actor)
            .await
            .map_err(|error| match error {
                crate::PortError::NotFound | crate::PortError::Rejected => {
                    ApplicationError::NotVisible
                }
                crate::PortError::Unavailable | crate::PortError::InvalidResponse => {
                    ApplicationError::TemporarilyUnavailable
                }
            })
    }

    async fn authorize_project_read(
        actor: &ActorContext,
        project_ref: work_contracts::ProjectRef,
        actor_member_resolver: &dyn ActorMemberResolverPort,
        member_repo: &dyn ProjectMemberRepository,
    ) -> Result<ProjectMemberRef, ApplicationError> {
        let actor_member = Self::resolve_query_actor_member(actor, actor_member_resolver).await?;
        let member = member_repo
            .get_by_member(project_ref, actor_member.member_ref)
            .await
            .map_err(Self::map_repo_error)?;
        let Some(member) = member else {
            return Err(ApplicationError::NotVisible);
        };
        if !Self::member_is_visible(&member) {
            return Err(ApplicationError::NotVisible);
        }
        Ok(member.project_member_ref())
    }

    async fn authorize_work_read(
        actor: &ActorContext,
        scope: &FormalWorkScope,
        actor_member_resolver: &dyn ActorMemberResolverPort,
        member_repo: &dyn ProjectMemberRepository,
    ) -> Result<ProjectMemberRef, ApplicationError> {
        Self::authorize_project_read(
            actor,
            scope.project_ref.clone(),
            actor_member_resolver,
            member_repo,
        )
        .await
    }

    async fn authorize_member_work_read(
        actor: &ActorContext,
        target_member: &ProjectMember,
        actor_member_resolver: &dyn ActorMemberResolverPort,
        member_repo: &dyn ProjectMemberRepository,
    ) -> Result<ProjectMemberRef, ApplicationError> {
        if !Self::member_is_visible(target_member) {
            return Err(ApplicationError::NotVisible);
        }
        Self::authorize_project_read(
            actor,
            work_contracts::ProjectRef {
                project_id: target_member.project_id.clone(),
            },
            actor_member_resolver,
            member_repo,
        )
        .await
    }

    async fn authorize_iteration_read(
        actor: &ActorContext,
        iteration: &Iteration,
        actor_member_resolver: &dyn ActorMemberResolverPort,
        member_repo: &dyn ProjectMemberRepository,
    ) -> Result<ProjectMemberRef, ApplicationError> {
        Self::authorize_project_read(
            actor,
            work_contracts::ProjectRef {
                project_id: iteration.project_id.clone(),
            },
            actor_member_resolver,
            member_repo,
        )
        .await
    }
}

fn visible_response<T>(data: T) -> WorkQueryResponse<T> {
    WorkQueryResponse {
        surface: QuerySurface::Visible,
        data: Some(data),
    }
}

fn empty_response<T>(data: T) -> WorkQueryResponse<T> {
    WorkQueryResponse {
        surface: QuerySurface::Empty,
        data: Some(data),
    }
}

fn stale_response<T>(data: T) -> WorkQueryResponse<T> {
    WorkQueryResponse {
        surface: QuerySurface::Stale,
        data: Some(data),
    }
}

fn rebuilding_with_response<T>(data: T) -> WorkQueryResponse<T> {
    WorkQueryResponse {
        surface: QuerySurface::Rebuilding,
        data: Some(data),
    }
}

fn failed_response<T>(data: T) -> WorkQueryResponse<T> {
    WorkQueryResponse {
        surface: QuerySurface::Failed,
        data: Some(data),
    }
}

fn missing_response<T>() -> WorkQueryResponse<T> {
    WorkQueryResponse {
        surface: QuerySurface::Missing,
        data: None,
    }
}

fn not_visible_response<T>() -> WorkQueryResponse<T> {
    WorkQueryResponse {
        surface: QuerySurface::NotVisible,
        data: None,
    }
}

fn rebuilding_response<T>() -> WorkQueryResponse<T> {
    WorkQueryResponse {
        surface: QuerySurface::Rebuilding,
        data: None,
    }
}
