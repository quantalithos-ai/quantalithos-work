//! Application services for the Work bounded context.

mod errors;
mod idempotency;
mod member;
mod ports;
mod project;
mod results;
mod unit_of_work;

pub use errors::ApplicationError;
pub use idempotency::{
    IdempotencyConflict, IdempotencyError, IdempotencyRecord, IdempotencyRepository,
    IdempotencyReservation, IdempotencyStatus, RequestDigest,
};
pub use member::ProjectMemberCommandService;
pub use ports::{
    AuditRepository, BacklogRepository, ClockPort, IdGeneratorPort, MemberCapabilitySnapshotInput,
    MemberReferencePort, Page, PageInfo, PortError, ProjectMemberRepository, ProjectRepository,
    ProjectionRepository, ReferenceSnapshotRepository, RepositoryError, WorkOutboxRepository,
};
pub use project::ProjectCommandService;
pub use results::{CommandResultRepository, StoredCommandResult};
pub use unit_of_work::{UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, UnitOfWorkId};
