//! Public query view DTO exports for Work.

pub use crate::queries::{
    BacklogView, FormalWorkSummaryView, IterationSummaryView, MemberWorkView, ProjectBoardView,
    ProjectMemberSummaryView, ProjectWorkFactsView, ProjectionViewMarker, PublicPageInfo,
    ReconciliationReport, WorkItemView, WorkQueryResponse, WorkRelationStateView,
    WorkRelationSummaryView, WorkSearchProjection, WorkSearchResult, WorkTraceRecordView,
    WorkTraceView,
};

use serde::{Deserialize, Serialize};

use crate::jobs::WorkProjectionSet;
use crate::refs::{
    BacklogRef, DependencyOrBlockerRef, ExternalEvidenceRef, FormalWorkRef, GlobalMemberRef,
    IterationRef, MethodDefinitionRef, ProjectMemberRef, ProjectRef, SourceWorkKind, SourceWorkRef,
    WorkTitle, WorkTruthCursor,
};
use crate::states::{
    BacklogState, CommitmentState, IterationState, ProjectLifecycleState,
    ProjectMemberResponsibilityState, WorkItemState,
};

/// Body-free project summary used by projection rebuild.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectTruthSummary {
    /// Project reference.
    pub project_ref: ProjectRef,
    /// Project source when present.
    pub source_ref: Option<SourceWorkRef>,
    /// Current project lifecycle state.
    pub lifecycle_state: ProjectLifecycleState,
    /// Current backlog when available.
    pub backlog_ref: Option<BacklogRef>,
}

/// Body-free backlog summary used by projection rebuild.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BacklogTruthSummary {
    /// Backlog reference.
    pub backlog_ref: BacklogRef,
    /// Owning project.
    pub project_ref: ProjectRef,
    /// Current backlog availability state.
    pub backlog_state: BacklogState,
}

/// Body-free project member summary used by projection rebuild.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectMemberTruthSummary {
    /// Project member reference.
    pub project_member_ref: ProjectMemberRef,
    /// Owning project.
    pub project_ref: ProjectRef,
    /// Referenced identity member.
    pub member_ref: GlobalMemberRef,
    /// Current responsibility state.
    pub responsibility_state: ProjectMemberResponsibilityState,
}

/// Body-free formal work summary used by projection rebuild.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FormalWorkTruthSummary {
    /// Formal work reference.
    pub work_ref: FormalWorkRef,
    /// Owning project.
    pub project_ref: ProjectRef,
    /// Owning backlog.
    pub backlog_ref: BacklogRef,
    /// Parent work when this is a child work item.
    pub parent_ref: Option<FormalWorkRef>,
    /// Searchable title.
    pub title: WorkTitle,
    /// Current work lifecycle state.
    pub work_state: WorkItemState,
    /// Current project member assignee when available.
    pub assignee_ref: Option<ProjectMemberRef>,
    /// Source used to create or promote this work when available.
    pub source_ref: Option<SourceWorkRef>,
    /// Admission method definition when present.
    pub method_definition_ref: Option<MethodDefinitionRef>,
    /// Source kind copied out for projection filtering.
    pub source_kind: Option<SourceWorkKind>,
    /// Completion evidence reference when present.
    pub completion_ref: Option<ExternalEvidenceRef>,
    /// Iteration scope when the work is currently committed.
    pub iteration_ref: Option<IterationRef>,
}

/// Body-free dependency or blocker summary used by projection rebuild.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkRelationTruthSummary {
    /// Dependency or blocker reference.
    pub relation_ref: DependencyOrBlockerRef,
    /// Formal work affected by this relation.
    pub affected_work_refs: Vec<FormalWorkRef>,
    /// Current relation state marker.
    pub relation_state: WorkRelationStateView,
}

/// Body-free iteration and commitment summary used by projection rebuild.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IterationTruthSummary {
    /// Iteration reference.
    pub iteration_ref: IterationRef,
    /// Owning project.
    pub project_ref: ProjectRef,
    /// Current iteration state.
    pub iteration_state: IterationState,
    /// Current commitment state when a commitment exists.
    pub commitment_state: Option<CommitmentState>,
    /// Formal work currently committed to this iteration.
    pub committed_work_refs: Vec<FormalWorkRef>,
}

/// Committed truth snapshot used to rebuild project-scoped projections.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectWorkTruthSnapshot {
    /// Project truth summary.
    pub project: ProjectTruthSummary,
    /// Backlog truth summary when present.
    pub backlog: Option<BacklogTruthSummary>,
    /// Project members in scope.
    pub members: Vec<ProjectMemberTruthSummary>,
    /// Formal work summaries in scope.
    pub work_items: Vec<FormalWorkTruthSummary>,
    /// Dependency and blocker summaries in scope.
    pub relations: Vec<WorkRelationTruthSummary>,
    /// Iteration summaries in scope.
    pub iterations: Vec<IterationTruthSummary>,
    /// Source cursor covered by this snapshot.
    pub source_cursor: WorkTruthCursor,
}

/// Projection rebuild output for one project.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectProjectionBatch {
    /// Project board views to replace.
    pub board_views: Vec<ProjectBoardView>,
    /// Member work views to replace.
    pub member_views: Vec<MemberWorkView>,
    /// Iteration summary views to replace.
    pub iteration_views: Vec<IterationSummaryView>,
    /// Search projection records to replace.
    pub search_records: Vec<WorkSearchProjection>,
}

impl ProjectProjectionBatch {
    /// Rebuilds selected projection views from committed truth summaries.
    pub fn from_truth(snapshot: ProjectWorkTruthSnapshot, projection_set: WorkProjectionSet) -> Self {
        let ProjectWorkTruthSnapshot {
            project,
            members,
            work_items,
            iterations,
            source_cursor,
            ..
        } = snapshot;

        let board_views = match projection_set {
            WorkProjectionSet::All | WorkProjectionSet::ProjectBoard => vec![ProjectBoardView {
                project_ref: project.project_ref.clone(),
                work_cards: work_items
                    .iter()
                    .map(Self::formal_work_summary)
                    .collect(),
                marker: ProjectionViewMarker {
                    view_ref: crate::refs::DerivedWorkViewRef::project_board(
                        project.project_ref.clone(),
                    ),
                    source_cursor: source_cursor.clone(),
                    freshness_state: crate::states::DerivedFreshnessState::Fresh,
                },
            }],
            _ => Vec::new(),
        };

        let member_views = match projection_set {
            WorkProjectionSet::All | WorkProjectionSet::MemberWork => members
                .iter()
                .map(|member| MemberWorkView {
                    member_ref: member.project_member_ref.clone(),
                    assigned_work: work_items
                        .iter()
                        .filter(|work| work.assignee_ref.as_ref() == Some(&member.project_member_ref))
                        .map(Self::formal_work_summary)
                        .collect(),
                    marker: ProjectionViewMarker {
                        view_ref: crate::refs::DerivedWorkViewRef::member_work(
                            member.project_member_ref.clone(),
                        ),
                        source_cursor: source_cursor.clone(),
                        freshness_state: crate::states::DerivedFreshnessState::Fresh,
                    },
                    page: crate::queries::PublicPageInfo {
                        next_page_token: None,
                        has_more: false,
                    },
                })
                .collect(),
            _ => Vec::new(),
        };

        let iteration_views = match projection_set {
            WorkProjectionSet::All | WorkProjectionSet::IterationSummary => iterations
                .iter()
                .map(|iteration| IterationSummaryView {
                    iteration_ref: iteration.iteration_ref.clone(),
                    iteration_state: iteration.iteration_state,
                    commitment_state: iteration.commitment_state,
                    committed_work: iteration
                        .committed_work_refs
                        .iter()
                        .filter_map(|work_ref| {
                            work_items
                                .iter()
                                .find(|work| &work.work_ref == work_ref)
                                .map(Self::formal_work_summary)
                        })
                        .collect(),
                    marker: ProjectionViewMarker {
                        view_ref: crate::refs::DerivedWorkViewRef::iteration_summary(
                            iteration.iteration_ref.clone(),
                        ),
                        source_cursor: source_cursor.clone(),
                        freshness_state: crate::states::DerivedFreshnessState::Fresh,
                    },
                })
                .collect(),
            _ => Vec::new(),
        };

        let search_records = match projection_set {
            WorkProjectionSet::All | WorkProjectionSet::Search => work_items
                .iter()
                .map(|work| WorkSearchProjection {
                    project_ref: work.project_ref.clone(),
                    work_ref: work.work_ref.clone(),
                    title: work.title.clone(),
                    work_state: work.work_state,
                    assignee_ref: work.assignee_ref.clone(),
                    source_kind: work.source_kind,
                    source_cursor: source_cursor.clone(),
                })
                .collect(),
            _ => Vec::new(),
        };

        Self {
            board_views,
            member_views,
            iteration_views,
            search_records,
        }
    }

    /// Returns the stable view refs covered by this batch.
    pub fn view_refs(&self) -> Vec<crate::refs::DerivedWorkViewRef> {
        let mut refs = Vec::new();
        refs.extend(self.board_views.iter().map(|view| view.marker.view_ref.clone()));
        refs.extend(self.member_views.iter().map(|view| view.marker.view_ref.clone()));
        refs.extend(self.iteration_views.iter().map(|view| view.marker.view_ref.clone()));
        refs
    }

    fn formal_work_summary(summary: &FormalWorkTruthSummary) -> FormalWorkSummaryView {
        FormalWorkSummaryView {
            work_ref: summary.work_ref.clone(),
            work_state: summary.work_state,
            assignee_ref: summary.assignee_ref.clone(),
            completion_ref: summary.completion_ref.clone(),
        }
    }
}
