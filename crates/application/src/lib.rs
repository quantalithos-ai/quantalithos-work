//! Application services for the Work bounded context.

mod dependency;
mod errors;
mod idempotency;
mod member;
mod ports;
mod project;
mod promote;
mod results;
mod unit_of_work;
mod workitem;

pub use dependency::DependencyBlockerService;
pub use errors::ApplicationError;
pub use idempotency::{
    IdempotencyConflict, IdempotencyError, IdempotencyRecord, IdempotencyRepository,
    IdempotencyReservation, IdempotencyStatus, RequestDigest,
};
pub use member::ProjectMemberCommandService;
pub use ports::{
    AuditRepository, BacklogRepository, ClockPort, DependencyRepository, EvidenceResolution,
    EvidenceResolverPort, FormalWorkRecord, FormalWorkScope, IdGeneratorPort,
    MemberCapabilitySnapshotInput, MemberReferencePort, Page, PageInfo, PortError,
    ProjectMemberRepository, ProjectRepository, ProjectionRepository, PromoteRepository,
    ReferenceSnapshotRepository, RepositoryError, SourceWorkResolution, SourceWorkResolverPort,
    WorkItemRepository, WorkOutboxRepository,
};
pub use project::ProjectCommandService;
pub use promote::PromoteCommandService;
pub use results::{CommandResultRepository, StoredCommandResult};
pub use unit_of_work::{UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, UnitOfWorkId};
pub use work_domain::PendingPromoteIntake;
pub use workitem::WorkItemCommandService;
