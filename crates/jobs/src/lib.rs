//! Operations job entrypoints for the Work bounded context.

use work_application::{ApplicationError, WorkOutboxPublishService};
use work_contracts as _;
use work_contracts::PublishWorkOutboxJobInput;
use work_infra as _;

/// Thin operations runner for Work maintenance jobs.
pub struct WorkOperationsJobRunner<OPS> {
    /// Outbox publication application service.
    pub outbox_publish: OPS,
}

impl<OPS> WorkOperationsJobRunner<OPS>
where
    OPS: Send + Sync,
{
    /// Creates a new job runner from assembled services.
    pub fn new(outbox_publish: OPS) -> Self {
        Self { outbox_publish }
    }
}

impl<PJ, PM, W, PR, B, D, ITR, A, O, PROJ, PUB, U, IDEM, JR, IDS>
    WorkOperationsJobRunner<
        WorkOutboxPublishService<PJ, PM, W, PR, B, D, ITR, A, O, PROJ, PUB, U, IDEM, JR, IDS>,
    >
where
    PJ: work_application::ProjectRepository,
    PM: work_application::ProjectMemberRepository,
    W: work_application::WorkItemRepository,
    PR: work_application::PromoteRepository,
    B: work_application::BacklogRepository,
    D: work_application::DependencyRepository,
    ITR: work_application::IterationRepository,
    A: work_application::AuditRepository,
    O: work_application::WorkOutboxRepository,
    PROJ: work_application::ProjectionRepository,
    PUB: work_application::WorkOutboxPublisherPort,
    U: work_application::UnitOfWork,
    IDEM: work_application::IdempotencyRepository,
    JR: work_application::JobResultRepository,
    IDS: work_application::IdGeneratorPort,
{
    /// Runs the P0 outbox publication job.
    pub async fn run_publish_work_outbox(
        &self,
        input: PublishWorkOutboxJobInput,
    ) -> Result<work_contracts::WorkJobReport, ApplicationError> {
        self.outbox_publish.publish_outbox(input).await
    }
}
