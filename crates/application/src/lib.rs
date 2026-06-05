//! Application services for the Work bounded context.

mod errors;
mod idempotency;
mod ports;
mod project;
mod results;
mod unit_of_work;

pub use errors::ApplicationError;
pub use idempotency::{
    IdempotencyConflict, IdempotencyError, IdempotencyRecord, IdempotencyRepository,
    IdempotencyReservation, IdempotencyStatus, RequestDigest,
};
pub use ports::{
    AuditRepository, BacklogRepository, ClockPort, IdGeneratorPort, Page, PageInfo, PortError,
    ProjectRepository, ProjectionRepository, RepositoryError, WorkOutboxRepository,
};
pub use project::ProjectCommandService;
pub use results::{CommandResultRepository, StoredCommandResult};
pub use unit_of_work::{UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, UnitOfWorkId};
