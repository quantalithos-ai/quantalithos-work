//! Query DTOs and response wrappers for Work.

use serde::{Deserialize, Serialize};

use core_contracts::{
    actor::ActorContext,
    metadata::{PageToken, QueryMetadata},
};

use crate::handoff::WorkTraceContextRef;
use crate::refs::{
    BacklogRef, DependencyOrBlockerRef, DerivedWorkViewRef, ExternalEvidenceRef, FormalWorkRef,
    GlobalMemberRef, IterationRef, ProjectMemberRef, ProjectRef, SourceWorkKind, SourceWorkRef,
    WorkSearchText, WorkTraceId, WorkTraceSubjectRef,
};
use crate::states::{
    BacklogState, BlockerState, CommitmentState, DependencyState, DerivedFreshnessState,
    IterationState, ProjectLifecycleState, ProjectMemberResponsibilityState, WorkItemState,
};

/// A synchronous Work query envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkQueryEnvelope<T> {
    /// Effective actor and entrypoint context.
    pub actor: ActorContext,
    /// Core query metadata; pagination and consistency live here.
    pub metadata: QueryMetadata,
    /// Operation-specific query body.
    pub query: T,
}

/// Public page metadata returned by Work query DTOs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PublicPageInfo {
    /// Token to request the next page.
    pub next_page_token: Option<PageToken>,
    /// Whether more items may exist.
    pub has_more: bool,
}

/// Public marker describing projection freshness.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectionViewMarker {
    /// Stable derived view reference.
    pub view_ref: DerivedWorkViewRef,
    /// Source cursor covered by this view.
    pub source_cursor: crate::refs::WorkTruthCursor,
    /// Current freshness state.
    pub freshness_state: DerivedFreshnessState,
}

/// Public query response wrapper.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkQueryResponse<T> {
    /// Visibility and degradation surface for this response.
    pub surface: QuerySurface,
    /// Optional payload; absent for not-visible and missing.
    pub data: Option<T>,
}

/// Query response surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuerySurface {
    /// The requested data is visible and usable.
    Visible,
    /// The requested scope exists but has no data.
    Empty,
    /// The caller cannot see the requested scope.
    NotVisible,
    /// The response includes stale projection data.
    Stale,
    /// The projection is rebuilding and data may be absent.
    Rebuilding,
    /// The projection or reference failed and data is degraded.
    Failed,
    /// The requested resource is missing.
    Missing,
}

/// Summarizes one formal work record for public query views.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormalWorkSummaryView {
    /// Formal work reference.
    pub work_ref: FormalWorkRef,
    /// Current work lifecycle state.
    pub work_state: WorkItemState,
    /// Current project member assignee.
    pub assignee_ref: Option<ProjectMemberRef>,
    /// Optional completion evidence reference.
    pub completion_ref: Option<ExternalEvidenceRef>,
}

/// Summarizes a project member responsibility for public query views.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemberSummaryView {
    /// Project member reference.
    pub project_member_ref: ProjectMemberRef,
    /// Referenced identity member.
    pub member_ref: GlobalMemberRef,
    /// Current responsibility state.
    pub responsibility_state: ProjectMemberResponsibilityState,
}

/// Summarizes a dependency or blocker for public query views.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkRelationSummaryView {
    /// Relation or blocker reference.
    pub relation_ref: DependencyOrBlockerRef,
    /// Formal work affected by this relation.
    pub affected_work_refs: Vec<FormalWorkRef>,
    /// Current relation state marker.
    pub relation_state: WorkRelationStateView,
}

/// Public relation state marker that avoids exposing domain-only variants.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkRelationStateView {
    /// Dependency state marker.
    Dependency(DependencyState),
    /// Blocker state marker.
    Blocker(BlockerState),
}

/// Search criteria accepted by Work search queries.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkSearchCriteria {
    /// Optional formal work state filter.
    pub work_state: Option<WorkItemState>,
    /// Optional assignee filter.
    pub assignee_ref: Option<ProjectMemberRef>,
    /// Optional source kind filter.
    pub source_kind: Option<SourceWorkKind>,
    /// Optional free-text query over indexed summaries.
    pub text_query: Option<WorkSearchText>,
}

/// Filters backlog reads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BacklogQueryFilter {
    /// Optional formal work state filter.
    pub work_state: Option<WorkItemState>,
    /// Optional assignee filter.
    pub assignee_ref: Option<ProjectMemberRef>,
}

/// Requests project work facts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetProjectWorkFactsRequest {
    /// Project to read.
    pub project_ref: ProjectRef,
}

/// Requests a backlog page.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetBacklogRequest {
    /// Project whose backlog is read.
    pub project_ref: ProjectRef,
    /// Optional backlog filter.
    pub filter: Option<BacklogQueryFilter>,
}

/// Requests one formal work item.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetWorkItemRequest {
    /// Formal work to read.
    pub work_ref: FormalWorkRef,
}

/// Requests work assigned to a project member.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ListMemberWorkRequest {
    /// Project member scope.
    pub project_member_ref: ProjectMemberRef,
    /// Optional work state filter.
    pub work_state: Option<WorkItemState>,
}

/// Requests an iteration summary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetIterationSummaryRequest {
    /// Iteration to read.
    pub iteration_ref: IterationRef,
}

/// Requests a formal work search.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchWorkRequest {
    /// Project search scope.
    pub project_ref: ProjectRef,
    /// Search criteria.
    pub criteria: WorkSearchCriteria,
}

/// Requests Work trace records for a subject.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetWorkTraceRequest {
    /// Trace subject.
    pub subject_ref: WorkTraceSubjectRef,
}

/// Requests the project board projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GetProjectBoardViewRequest {
    /// Project board scope.
    pub project_ref: ProjectRef,
}

/// Project facts visible to authorized consumers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectWorkFactsView {
    /// Project reference.
    pub project_ref: ProjectRef,
    /// Current project lifecycle state.
    pub lifecycle_state: ProjectLifecycleState,
    /// Current backlog reference when available.
    pub backlog_ref: Option<BacklogRef>,
    /// Project members visible to the actor.
    pub members: Vec<ProjectMemberSummaryView>,
    /// Formal work summaries visible to the actor.
    pub formal_work: Vec<FormalWorkSummaryView>,
    /// Dependency and blocker summaries visible to the actor.
    pub relations: Vec<WorkRelationSummaryView>,
}

/// Backlog read view.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BacklogView {
    /// Backlog reference.
    pub backlog_ref: BacklogRef,
    /// Owning project.
    pub project_ref: ProjectRef,
    /// Current backlog state.
    pub backlog_state: BacklogState,
    /// Page of formal work summaries.
    pub items: Vec<FormalWorkSummaryView>,
    /// Public page metadata.
    pub page: PublicPageInfo,
}

/// Formal work read view.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkItemView {
    /// Formal work reference.
    pub work_ref: FormalWorkRef,
    /// Parent work when this is a child work item.
    pub parent_ref: Option<FormalWorkRef>,
    /// Current work state.
    pub work_state: WorkItemState,
    /// Current assignee.
    pub assignee_ref: Option<ProjectMemberRef>,
    /// Source used to create or promote this work.
    pub source_ref: Option<SourceWorkRef>,
    /// Completion evidence when present.
    pub completion_ref: Option<ExternalEvidenceRef>,
    /// Active relations involving this work.
    pub relations: Vec<WorkRelationSummaryView>,
}

/// Member work projection view.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemberWorkView {
    /// Project member reference.
    pub member_ref: ProjectMemberRef,
    /// Assigned work visible in this view.
    pub assigned_work: Vec<FormalWorkSummaryView>,
    /// Projection marker.
    pub marker: ProjectionViewMarker,
    /// Public page metadata.
    pub page: PublicPageInfo,
}

/// Iteration summary projection view.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IterationSummaryView {
    /// Iteration reference.
    pub iteration_ref: IterationRef,
    /// Current iteration state.
    pub iteration_state: IterationState,
    /// Current commitment state.
    pub commitment_state: Option<CommitmentState>,
    /// Committed work summaries.
    pub committed_work: Vec<FormalWorkSummaryView>,
    /// Projection marker.
    pub marker: ProjectionViewMarker,
}

/// Search result over formal work projections.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkSearchResult {
    /// Project searched.
    pub project_ref: ProjectRef,
    /// Criteria applied by the query.
    pub criteria: WorkSearchCriteria,
    /// Matching work items.
    pub items: Vec<FormalWorkSummaryView>,
    /// Projection marker.
    pub marker: ProjectionViewMarker,
    /// Public page metadata.
    pub page: PublicPageInfo,
}

/// Internal search projection row returned by the projection store.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkSearchProjection {
    /// Project searched.
    pub project_ref: ProjectRef,
    /// Formal work represented by this row.
    pub work_ref: FormalWorkRef,
    /// Searchable title.
    pub title: crate::refs::WorkTitle,
    /// Current work state.
    pub work_state: WorkItemState,
    /// Current assignee when available.
    pub assignee_ref: Option<ProjectMemberRef>,
    /// Source cursor that produced this row.
    pub source_cursor: crate::refs::WorkTruthCursor,
}

/// Trace view for one Work subject.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkTraceView {
    /// Trace subject.
    pub subject_ref: WorkTraceSubjectRef,
    /// Trace records visible to the actor.
    pub records: Vec<WorkTraceRecordView>,
    /// Public page metadata.
    pub page: PublicPageInfo,
}

/// Public trace record view.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkTraceRecordView {
    /// Trace record id.
    pub trace_id: WorkTraceId,
    /// Related subject.
    pub subject_ref: WorkTraceSubjectRef,
    /// Core trace and request pointer.
    pub trace_context_ref: WorkTraceContextRef,
}

/// Project board projection view.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectBoardView {
    /// Project reference.
    pub project_ref: ProjectRef,
    /// Work cards grouped for board consumption.
    pub work_cards: Vec<FormalWorkSummaryView>,
    /// Projection marker.
    pub marker: ProjectionViewMarker,
}
