//! Runtime configuration shells for the Work bounded context.

/// Runtime configuration root for the Work implementation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkRuntimeConfig {
    /// Store configuration.
    pub store: WorkStoreConfig,
    /// Command and query boundary configuration.
    pub boundary: WorkBoundaryConfig,
    /// Idempotency and event deduplication configuration.
    pub idempotency: WorkIdempotencyConfig,
    /// Projection and rebuild configuration.
    pub projection: WorkProjectionConfig,
    /// Operations job configuration.
    pub jobs: WorkJobConfig,
    /// External seam configuration.
    pub external: WorkExternalConfig,
    /// Outbox publisher configuration.
    pub outbox: WorkOutboxConfig,
    /// Trace and archive handoff configuration.
    pub handoff: WorkHandoffConfig,
    /// Runtime feature switches.
    pub features: WorkFeatureConfig,
}

/// Store configuration loaded by `infra/config.rs`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkStoreConfig;

/// Command and query boundary configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkBoundaryConfig;

/// Idempotency and deduplication retention configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkIdempotencyConfig;

/// Projection store and rebuild configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkProjectionConfig;

/// Operations job configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkJobConfig;

/// External seam configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkExternalConfig;

/// Outbox publisher configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkOutboxConfig;

/// Trace and archive handoff configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkHandoffConfig;

/// Runtime feature switches.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkFeatureConfig;
