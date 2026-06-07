//! Fake outbound publishers for Work outbox tests.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use work_application::{PortError, WorkOutboxPublisherPort};
use work_contracts::{OutboxPublicationRef, WorkOutboundPublication};

/// In-memory publisher fake for outbox publication tests.
#[derive(Clone, Default)]
pub struct FakeWorkOutboxPublisher {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    published: Vec<WorkOutboundPublication>,
    queued_results: VecDeque<Result<OutboxPublicationRef, PortError>>,
}

impl FakeWorkOutboxPublisher {
    /// Creates an empty publisher fake.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues one publication outcome for the next publish call.
    pub fn push_result(&self, result: Result<OutboxPublicationRef, PortError>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.queued_results.push_back(result);
        }
    }

    /// Returns published envelopes in call order.
    pub fn published(&self) -> Vec<WorkOutboundPublication> {
        self.inner
            .lock()
            .map(|inner| inner.published.clone())
            .unwrap_or_default()
    }

    /// Returns the total publish call count.
    pub fn publish_count(&self) -> usize {
        self.inner
            .lock()
            .map(|inner| inner.published.len())
            .unwrap_or_default()
    }
}

#[async_trait]
impl WorkOutboxPublisherPort for FakeWorkOutboxPublisher {
    async fn publish(
        &self,
        publication: WorkOutboundPublication,
    ) -> Result<OutboxPublicationRef, PortError> {
        let mut inner = self.inner.lock().map_err(|_| PortError::Unavailable)?;
        inner.published.push(publication);
        match inner.queued_results.pop_front() {
            Some(result) => result,
            None => Ok(OutboxPublicationRef(format!(
                "publication-{}",
                inner.published.len()
            ))),
        }
    }
}
