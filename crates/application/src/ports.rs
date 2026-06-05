//! Repository and adapter traits consumed by Work application services.

use async_trait::async_trait;
use core_contracts::metadata::{PageRequest, PageToken, Timestamp, Version};
use serde::{Deserialize, Serialize};

use crate::UnitOfWorkHandle;
use work_contracts::{
    BacklogRef, DerivedWorkViewRef, OutboxFailureReason, OutboxPublicationRef, OutboxRetryReason,
    ProjectOwnerRef, ProjectRef, WorkAuditSubjectRef, WorkOutboxId, WorkTruthCursor,
};
use work_domain::{Backlog, TraceHandoffMarker, WorkAuditTrail, WorkOutboxRecord, WorkTraceRecord};

/// A repository page returned before public query mapping.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    /// Items returned by repository read.
    pub items: Vec<T>,
    /// Cursor metadata for the next repository read.
    pub page_info: PageInfo,
}

/// Cursor metadata for repository page reads.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PageInfo {
    /// Token for the next page.
    pub next_page_token: Option<PageToken>,
    /// Whether more repository items may exist.
    pub has_more: bool,
}

/// Classifies persistence and local store failures before service mapping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryError {
    /// The requested record does not exist.
    NotFound,
    /// The expected optimistic version did not match the stored version.
    VersionConflict,
    /// The local transaction boundary rejected the operation.
    TransactionRejected,
    /// The store is unavailable or failed for a technical reason.
    StoreUnavailable,
}

/// Classifies external resolver, publisher, and handoff failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortError {
    /// The referenced external object could not be resolved.
    NotFound,
    /// The external reference exists but cannot be used by this operation.
    Rejected,
    /// The external dependency is temporarily unavailable.
    Unavailable,
    /// The external dependency returned an invalid or unsupported response.
    InvalidResponse,
}

/// Stores Work-owned project truth.
#[async_trait]
pub trait ProjectRepository: Send + Sync {
    /// Loads a project by stable Work identity.
    async fn get(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<work_domain::Project>, RepositoryError>;

    /// Lists projects owned by the same external owner.
    async fn list_by_owner(
        &self,
        owner_ref: ProjectOwnerRef,
        page: PageRequest,
    ) -> Result<Page<work_domain::Project>, RepositoryError>;

    /// Creates a project inside the current unit of work.
    async fn create(
        &self,
        project: work_domain::Project,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves a project lifecycle change inside the current unit of work.
    async fn save(
        &self,
        project: work_domain::Project,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;
}

/// Stores backlog truth and formal work membership.
#[async_trait]
pub trait BacklogRepository: Send + Sync {
    /// Loads a backlog by Work identity.
    async fn get(&self, backlog_ref: BacklogRef) -> Result<Option<Backlog>, RepositoryError>;

    /// Loads the backlog that owns a project.
    async fn get_by_project(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<Backlog>, RepositoryError>;

    /// Loads the backlog and its optimistic version for one project.
    async fn get_by_project_with_version(
        &self,
        project_ref: ProjectRef,
    ) -> Result<Option<(Backlog, Version)>, RepositoryError>;

    /// Creates a backlog inside the current unit of work.
    async fn create(
        &self,
        backlog: Backlog,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves backlog availability changes inside the current unit of work.
    async fn save(
        &self,
        backlog: Backlog,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;
}

/// Stores Work trace and audit records.
#[async_trait]
pub trait AuditRepository: Send + Sync {
    /// Loads the audit trail for a Work subject.
    async fn get_audit_trail(
        &self,
        subject_ref: WorkAuditSubjectRef,
    ) -> Result<Option<WorkAuditTrail>, RepositoryError>;

    /// Appends a trace record inside the current unit of work.
    async fn append_trace(
        &self,
        record: WorkTraceRecord,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;

    /// Creates or updates an audit trail inside the current unit of work.
    async fn save_audit_trail(
        &self,
        audit_trail: WorkAuditTrail,
        expected_version: Option<Version>,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Saves a trace handoff marker inside the current unit of work.
    async fn save_trace_handoff_marker(
        &self,
        marker: TraceHandoffMarker,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;
}

/// Stores Work outbox records and publication state.
#[async_trait]
pub trait WorkOutboxRepository: Send + Sync {
    /// Enqueues a committed Work outbox record inside the current unit of work.
    async fn enqueue(
        &self,
        record: WorkOutboxRecord,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;

    /// Loads pending outbox records for publication.
    async fn list_pending(
        &self,
        page: PageRequest,
    ) -> Result<Page<WorkOutboxRecord>, RepositoryError>;

    /// Loads one outbox record.
    async fn get(
        &self,
        outbox_id: WorkOutboxId,
    ) -> Result<Option<WorkOutboxRecord>, RepositoryError>;

    /// Marks an outbox record as published.
    async fn mark_published(
        &self,
        outbox_id: WorkOutboxId,
        publication_ref: OutboxPublicationRef,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Marks an outbox record as failed and retryable.
    async fn mark_failed(
        &self,
        outbox_id: WorkOutboxId,
        reason: OutboxFailureReason,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;

    /// Marks a failed outbox record as pending when retry policy accepts it.
    async fn mark_pending_for_retry(
        &self,
        outbox_id: WorkOutboxId,
        reason: OutboxRetryReason,
        expected_version: Version,
        uow: &UnitOfWorkHandle,
    ) -> Result<Version, RepositoryError>;
}

/// Stores derived Work read views and their freshness state.
#[async_trait]
pub trait ProjectionRepository: Send + Sync {
    /// Marks affected derived views stale after a truth or snapshot change.
    async fn mark_stale(
        &self,
        affected: Vec<DerivedWorkViewRef>,
        source_cursor: WorkTruthCursor,
        uow: &UnitOfWorkHandle,
    ) -> Result<(), RepositoryError>;
}

/// Generates Work-owned identifiers.
pub trait IdGeneratorPort: Send + Sync {
    /// Generates a project id.
    fn next_project_id(&self) -> Result<work_contracts::ProjectId, PortError>;

    /// Generates a backlog id.
    fn next_backlog_id(&self) -> Result<work_contracts::BacklogId, PortError>;

    /// Generates a result id.
    fn next_result_id(&self) -> Result<work_contracts::ResultId, PortError>;

    /// Generates an outbox id.
    fn next_outbox_id(&self) -> Result<work_contracts::WorkOutboxId, PortError>;

    /// Generates a trace id.
    fn next_trace_id(&self) -> Result<work_contracts::WorkTraceId, PortError>;
}

/// Provides timestamps for application services.
pub trait ClockPort: Send + Sync {
    /// Returns the current timestamp.
    fn now(&self) -> Result<Timestamp, PortError>;
}
