//! Fake external resolvers for Work P0 service tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use work_application::{
    EvidenceResolution, EvidenceResolverPort, MemberCapabilitySnapshotInput, MemberReferencePort,
    PortError, SourceWorkResolution, SourceWorkResolverPort,
};
use work_contracts::{
    CapabilityRefSet, ExternalEvidenceRef, ExternalSourceSummary, GlobalMemberRef, SourceWorkRef,
};

/// Deterministic fake member resolver keyed by `GlobalMemberRef`.
#[derive(Clone, Default)]
pub struct FakeMemberReferencePort {
    outcomes: Arc<Mutex<HashMap<String, MemberResolverOutcome>>>,
}

/// Configured fake outcome for one member resolver call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemberResolverOutcome {
    /// Resolver returns safe capability refs.
    Success(CapabilityRefSet),
    /// External reference does not exist.
    Unresolved,
    /// External dependency is temporarily unavailable.
    Unavailable,
    /// External boundary rejects this member for Work use.
    Rejected,
    /// External boundary attempted to leak unsupported payload/body.
    BodyLeak,
}

impl FakeMemberReferencePort {
    /// Creates an empty fake resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds the outcome returned for one member reference.
    pub fn seed(&self, member_ref: GlobalMemberRef, outcome: MemberResolverOutcome) {
        if let Ok(mut outcomes) = self.outcomes.lock() {
            outcomes.insert(member_ref.0, outcome);
        }
    }
}

#[async_trait]
impl MemberReferencePort for FakeMemberReferencePort {
    async fn resolve_member_capability(
        &self,
        member_ref: GlobalMemberRef,
    ) -> Result<MemberCapabilitySnapshotInput, PortError> {
        let outcomes = self.outcomes.lock().map_err(|_| PortError::Unavailable)?;
        match outcomes
            .get(&member_ref.0)
            .cloned()
            .unwrap_or(MemberResolverOutcome::Unresolved)
        {
            MemberResolverOutcome::Success(capability_refs) => Ok(MemberCapabilitySnapshotInput {
                member_ref,
                capability_refs,
            }),
            MemberResolverOutcome::Unresolved => Err(PortError::NotFound),
            MemberResolverOutcome::Unavailable => Err(PortError::Unavailable),
            MemberResolverOutcome::Rejected => Err(PortError::Rejected),
            MemberResolverOutcome::BodyLeak => Err(PortError::InvalidResponse),
        }
    }
}

/// Deterministic fake source resolver keyed by `SourceWorkRef.external_ref.external_id`.
#[derive(Clone, Default)]
pub struct FakeSourceWorkResolverPort {
    outcomes: Arc<Mutex<HashMap<String, SourceResolverOutcome>>>,
}

/// Configured fake outcome for one source resolver call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceResolverOutcome {
    /// Resolver returns a safe summary without body.
    Success {
        /// Whether an external body was observed.
        has_external_body: bool,
    },
    /// External reference does not exist.
    Unresolved,
    /// External dependency is temporarily unavailable.
    Unavailable,
    /// External boundary rejects this source for Work use.
    Rejected,
}

impl FakeSourceWorkResolverPort {
    /// Creates an empty fake source resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds the outcome returned for one source reference.
    pub fn seed(&self, source_ref: SourceWorkRef, outcome: SourceResolverOutcome) {
        if let Ok(mut outcomes) = self.outcomes.lock() {
            outcomes.insert(source_ref.external_ref.external_id, outcome);
        }
    }
}

#[async_trait]
impl SourceWorkResolverPort for FakeSourceWorkResolverPort {
    async fn resolve_source_work(
        &self,
        source_ref: SourceWorkRef,
    ) -> Result<SourceWorkResolution, PortError> {
        let outcomes = self.outcomes.lock().map_err(|_| PortError::Unavailable)?;
        match outcomes
            .get(&source_ref.external_ref.external_id)
            .cloned()
            .unwrap_or(SourceResolverOutcome::Unresolved)
        {
            SourceResolverOutcome::Success { has_external_body } => Ok(SourceWorkResolution {
                summary: ExternalSourceSummary {
                    source_ref: source_ref.clone(),
                    source_kind: source_ref.source_kind,
                    source_digest: source_ref.source_digest.clone(),
                    has_external_body,
                },
                source_ref,
            }),
            SourceResolverOutcome::Unresolved => Err(PortError::NotFound),
            SourceResolverOutcome::Unavailable => Err(PortError::Unavailable),
            SourceResolverOutcome::Rejected => Err(PortError::Rejected),
        }
    }
}

/// Deterministic fake evidence resolver keyed by `ExternalEvidenceRef.external_ref.external_id`.
#[derive(Clone, Default)]
pub struct FakeEvidenceResolverPort {
    outcomes: Arc<Mutex<HashMap<String, EvidenceResolverOutcome>>>,
}

/// Configured fake outcome for one evidence resolver call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceResolverOutcome {
    /// Resolver returns the evidence ref.
    Success,
    /// External reference does not exist.
    Unresolved,
    /// External dependency is temporarily unavailable.
    Unavailable,
    /// External boundary rejects this evidence for Work use.
    Rejected,
}

impl FakeEvidenceResolverPort {
    /// Creates an empty fake evidence resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds the outcome returned for one evidence reference.
    pub fn seed(&self, evidence_ref: ExternalEvidenceRef, outcome: EvidenceResolverOutcome) {
        if let Ok(mut outcomes) = self.outcomes.lock() {
            outcomes.insert(evidence_ref.external_ref.external_id, outcome);
        }
    }
}

#[async_trait]
impl EvidenceResolverPort for FakeEvidenceResolverPort {
    async fn resolve_evidence(
        &self,
        evidence_ref: ExternalEvidenceRef,
    ) -> Result<EvidenceResolution, PortError> {
        let outcomes = self.outcomes.lock().map_err(|_| PortError::Unavailable)?;
        match outcomes
            .get(&evidence_ref.external_ref.external_id)
            .cloned()
            .unwrap_or(EvidenceResolverOutcome::Unresolved)
        {
            EvidenceResolverOutcome::Success => Ok(EvidenceResolution { evidence_ref }),
            EvidenceResolverOutcome::Unresolved => Err(PortError::NotFound),
            EvidenceResolverOutcome::Unavailable => Err(PortError::Unavailable),
            EvidenceResolverOutcome::Rejected => Err(PortError::Rejected),
        }
    }
}
