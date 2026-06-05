//! Deterministic clock and id generator fakes for Work tests.

use std::sync::{Arc, Mutex};

use core_contracts::metadata::Timestamp;
use work_application::{ClockPort, IdGeneratorPort, PortError};
use work_contracts::{
    BacklogId, ChildWorkItemId, ProjectId, ProjectMemberId, ResultId, WorkItemId, WorkOutboxId,
    WorkTraceId,
};

/// Deterministic id generator for P0 fake adapters and tests.
#[derive(Clone, Default)]
pub struct DeterministicWorkIdGenerator {
    counters: Arc<Mutex<Counters>>,
}

#[derive(Default)]
struct Counters {
    project: u64,
    backlog: u64,
    project_member: u64,
    work_item: u64,
    child_work_item: u64,
    result: u64,
    outbox: u64,
    trace: u64,
}

impl DeterministicWorkIdGenerator {
    /// Creates a deterministic id generator with zero-based counters.
    pub fn new() -> Self {
        Self::default()
    }
}

impl IdGeneratorPort for DeterministicWorkIdGenerator {
    fn next_project_id(&self) -> Result<ProjectId, PortError> {
        let mut counters = self.counters.lock().map_err(|_| PortError::Unavailable)?;
        counters.project += 1;
        Ok(ProjectId(format!("project-{}", counters.project)))
    }

    fn next_backlog_id(&self) -> Result<BacklogId, PortError> {
        let mut counters = self.counters.lock().map_err(|_| PortError::Unavailable)?;
        counters.backlog += 1;
        Ok(BacklogId(format!("backlog-{}", counters.backlog)))
    }

    fn next_project_member_id(&self) -> Result<ProjectMemberId, PortError> {
        let mut counters = self.counters.lock().map_err(|_| PortError::Unavailable)?;
        counters.project_member += 1;
        Ok(ProjectMemberId(format!(
            "project-member-{}",
            counters.project_member
        )))
    }

    fn next_work_item_id(&self) -> Result<WorkItemId, PortError> {
        let mut counters = self.counters.lock().map_err(|_| PortError::Unavailable)?;
        counters.work_item += 1;
        Ok(WorkItemId(format!("work-item-{}", counters.work_item)))
    }

    fn next_child_work_item_id(&self) -> Result<ChildWorkItemId, PortError> {
        let mut counters = self.counters.lock().map_err(|_| PortError::Unavailable)?;
        counters.child_work_item += 1;
        Ok(ChildWorkItemId(format!(
            "child-work-item-{}",
            counters.child_work_item
        )))
    }

    fn next_result_id(&self) -> Result<ResultId, PortError> {
        let mut counters = self.counters.lock().map_err(|_| PortError::Unavailable)?;
        counters.result += 1;
        Ok(ResultId(format!("result-{}", counters.result)))
    }

    fn next_outbox_id(&self) -> Result<WorkOutboxId, PortError> {
        let mut counters = self.counters.lock().map_err(|_| PortError::Unavailable)?;
        counters.outbox += 1;
        Ok(WorkOutboxId(format!("outbox-{}", counters.outbox)))
    }

    fn next_trace_id(&self) -> Result<WorkTraceId, PortError> {
        let mut counters = self.counters.lock().map_err(|_| PortError::Unavailable)?;
        counters.trace += 1;
        Ok(WorkTraceId(format!("trace-{}", counters.trace)))
    }
}

/// Fixed timestamp source for tests.
#[derive(Clone)]
pub struct FixedClock {
    now: Timestamp,
}

impl FixedClock {
    /// Creates a fixed clock returning one deterministic timestamp.
    pub fn new(now: Timestamp) -> Self {
        Self { now }
    }
}

impl ClockPort for FixedClock {
    fn now(&self) -> Result<Timestamp, PortError> {
        Ok(self.now.clone())
    }
}
