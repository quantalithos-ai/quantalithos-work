//! Fake external resolvers for Work P0 service tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use work_application::{MemberCapabilitySnapshotInput, MemberReferencePort, PortError};
use work_contracts::{CapabilityRefSet, GlobalMemberRef};

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
