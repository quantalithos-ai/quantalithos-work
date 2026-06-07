//! Application services for the Work bounded context.

mod consumer;
mod dependency;
mod errors;
mod idempotency;
mod iteration;
mod member;
mod ops;
mod outbox;
mod ports;
mod project;
mod promote;
mod query;
mod results;
mod unit_of_work;
mod workitem;

pub use consumer::{ConsumerDisposition, WorkInboundConsumerService};
pub use dependency::DependencyBlockerService;
pub use errors::ApplicationError;
pub use idempotency::{
    IdempotencyConflict, IdempotencyError, IdempotencyRecord, IdempotencyRepository,
    IdempotencyReservation, IdempotencyStatus, RequestDigest,
};
pub use iteration::IterationCommandService;
pub use member::ProjectMemberCommandService;
pub use ops::{
    WorkDerivedMaintenanceService, WorkReconciliationService, WorkReferenceRefreshService,
};
pub use outbox::WorkOutboxPublishService;
pub use ports::{
    ActorMemberResolverPort, AuditRepository, BacklogRepository, ClockPort, DependencyRepository,
    EvidenceResolution, EvidenceResolverPort, FormalWorkRecord, FormalWorkScope, IdGeneratorPort,
    IterationRepository, IterationSummaryViewProjection, MemberCapabilitySnapshotInput,
    MemberReferencePort, MemberWorkViewProjection, MethodDefinitionResolverPort,
    MethodDefinitionSnapshotInput, Page, PageInfo, PortError, ProcessTimeboxResolution,
    ProcessTimeboxResolverPort, ProjectBoardViewProjection, ProjectMemberRepository,
    ProjectRepository, ProjectionRepository, PromoteRepository, QueryActorMemberRef,
    ReferenceSnapshotRepository, RepositoryError, SourceWorkResolution, SourceWorkResolverPort,
    Versioned, WorkItemRepository, WorkOutboxPublisherPort, WorkOutboxRepository,
    WorkTruthSnapshotRepository,
};
pub use project::ProjectCommandService;
pub use promote::PromoteCommandService;
pub use query::{AuthorizedWorkQueryService, WorkQueryVisibilityPolicy};
pub use results::{
    CommandResultRepository, JobResultRepository, StoredCommandResult, StoredJobResult,
};
pub use unit_of_work::{UnitOfWork, UnitOfWorkError, UnitOfWorkHandle, UnitOfWorkId};
pub use work_domain::PendingPromoteIntake;
pub use workitem::WorkItemCommandService;
