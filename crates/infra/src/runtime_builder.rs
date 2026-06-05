//! Runtime assembly shells for the Work bounded context.

use crate::config::WorkRuntimeConfig;

/// Builds runtime wiring from validated configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkRuntimeBuilder;

impl WorkRuntimeBuilder {
    /// Creates a runtime builder shell for the current configuration.
    pub fn new(_config: &WorkRuntimeConfig) -> Self {
        Self
    }
}
