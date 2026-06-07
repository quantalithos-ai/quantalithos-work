//! Fake external resolvers and handoff adapters for Work P0 service tests.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use core_contracts::actor::ActorContext;
use work_application::{
    ActorMemberResolverPort, ArchiveHandoffPort, EvidenceResolution, EvidenceResolverPort,
    MemberCapabilitySnapshotInput, MemberReferencePort, MethodDefinitionResolverPort,
    MethodDefinitionSnapshotInput, PortError, ProcessTimeboxResolution, ProcessTimeboxResolverPort,
    QueryActorMemberRef, SourceWorkResolution, SourceWorkResolverPort, TraceHandoffPort,
};
use work_contracts::{
    ArchiveHandoffRef, CapabilityRefSet, EvidenceVerifiedState, ExternalEvidenceRef,
    ExternalReferenceRef, ExternalSourceSummary, GlobalMemberRef, MethodDefinitionKind,
    MethodDefinitionRef, ProcessTimeboxRef, ProcessTimeboxSummary, ProjectRef,
    ReferenceResolutionStatus, SafeSummaryText, SourceDigest, SourceWorkRef, TraceHandoffIntent,
    TraceHandoffRef,
};
use work_domain::{ArchiveHandoffIntent, ReferenceResolutionState};

/// Deterministic fake query actor-member resolver keyed by `ActorRef.actor_id`.
#[derive(Clone, Default)]
pub struct FakeActorMemberResolverPort {
    outcomes: Arc<Mutex<HashMap<String, ActorMemberResolverOutcome>>>,
}

/// Configured fake outcome for one actor-member resolver call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActorMemberResolverOutcome {
    /// Resolver returns a safe query actor-member mapping.
    Success(GlobalMemberRef),
    /// Actor could not be resolved to a member.
    Unresolved,
    /// External dependency is temporarily unavailable.
    Unavailable,
    /// External boundary rejects this actor.
    Rejected,
    /// External boundary returned invalid payload.
    Invalid,
}

impl FakeActorMemberResolverPort {
    /// Creates an empty fake resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds one actor id outcome.
    pub fn seed_actor_id(&self, actor_id: impl Into<String>, outcome: ActorMemberResolverOutcome) {
        if let Ok(mut outcomes) = self.outcomes.lock() {
            outcomes.insert(actor_id.into(), outcome);
        }
    }

    /// Seeds one actor context outcome by its effective actor id.
    pub fn seed(&self, actor: &ActorContext, outcome: ActorMemberResolverOutcome) {
        self.seed_actor_id(actor.actor.actor_id.as_str().to_owned(), outcome);
    }
}

#[async_trait]
impl ActorMemberResolverPort for FakeActorMemberResolverPort {
    async fn resolve_actor_member(
        &self,
        actor: &ActorContext,
    ) -> Result<QueryActorMemberRef, PortError> {
        let outcomes = self.outcomes.lock().map_err(|_| PortError::Unavailable)?;
        match outcomes
            .get(actor.actor.actor_id.as_str())
            .cloned()
            .unwrap_or(ActorMemberResolverOutcome::Unresolved)
        {
            ActorMemberResolverOutcome::Success(member_ref) => Ok(QueryActorMemberRef {
                actor_ref: actor.actor.clone(),
                member_ref,
            }),
            ActorMemberResolverOutcome::Unresolved => Err(PortError::NotFound),
            ActorMemberResolverOutcome::Unavailable => Err(PortError::Unavailable),
            ActorMemberResolverOutcome::Rejected => Err(PortError::Rejected),
            ActorMemberResolverOutcome::Invalid => Err(PortError::InvalidResponse),
        }
    }
}

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

/// Deterministic fake method-definition resolver keyed by `MethodDefinitionRef`.
#[derive(Clone, Default)]
pub struct FakeMethodDefinitionResolverPort {
    outcomes: Arc<Mutex<HashMap<String, MethodDefinitionResolverOutcome>>>,
}

/// Configured fake outcome for one method-definition resolver call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MethodDefinitionResolverOutcome {
    /// Resolver returns a safe definition kind.
    Success(MethodDefinitionKind),
    /// External reference does not exist.
    Unresolved,
    /// External dependency is temporarily unavailable.
    Unavailable,
    /// External boundary rejects this definition for Work use.
    Rejected,
    /// External boundary attempted to leak unsupported payload/body.
    Invalid,
}

impl FakeMethodDefinitionResolverPort {
    /// Creates an empty fake method-definition resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds the outcome returned for one method definition reference.
    pub fn seed(
        &self,
        definition_ref: MethodDefinitionRef,
        outcome: MethodDefinitionResolverOutcome,
    ) {
        if let Ok(mut outcomes) = self.outcomes.lock() {
            outcomes.insert(definition_ref.0, outcome);
        }
    }
}

#[async_trait]
impl MethodDefinitionResolverPort for FakeMethodDefinitionResolverPort {
    async fn resolve_definition(
        &self,
        definition_ref: MethodDefinitionRef,
    ) -> Result<MethodDefinitionSnapshotInput, PortError> {
        let outcomes = self.outcomes.lock().map_err(|_| PortError::Unavailable)?;
        match outcomes
            .get(&definition_ref.0)
            .cloned()
            .unwrap_or(MethodDefinitionResolverOutcome::Unresolved)
        {
            MethodDefinitionResolverOutcome::Success(definition_kind) => {
                Ok(MethodDefinitionSnapshotInput {
                    definition_ref,
                    definition_kind,
                })
            }
            MethodDefinitionResolverOutcome::Unresolved => Err(PortError::NotFound),
            MethodDefinitionResolverOutcome::Unavailable => Err(PortError::Unavailable),
            MethodDefinitionResolverOutcome::Rejected => Err(PortError::Rejected),
            MethodDefinitionResolverOutcome::Invalid => Err(PortError::InvalidResponse),
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
    /// Resolver returns the evidence ref with the supplied verified state.
    Success(EvidenceVerifiedState),
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
            EvidenceResolverOutcome::Success(verified_state) => Ok(EvidenceResolution {
                evidence_ref: evidence_ref.clone(),
                verified_state,
                reference_state: ReferenceResolutionState {
                    reference_ref: ExternalReferenceRef::from_evidence(evidence_ref),
                    resolution_state: if verified_state == EvidenceVerifiedState::Verified {
                        ReferenceResolutionStatus::Resolved
                    } else {
                        ReferenceResolutionStatus::Unresolved
                    },
                    last_resolved_at: None,
                },
            }),
            EvidenceResolverOutcome::Unresolved => Err(PortError::NotFound),
            EvidenceResolverOutcome::Unavailable => Err(PortError::Unavailable),
            EvidenceResolverOutcome::Rejected => Err(PortError::Rejected),
        }
    }
}

/// Deterministic fake process timebox resolver keyed by `ProcessTimeboxRef`.
#[derive(Clone, Default)]
pub struct FakeProcessTimeboxResolverPort {
    outcomes: Arc<Mutex<HashMap<String, ProcessTimeboxResolverOutcome>>>,
}

/// Configured fake outcome for one process timebox resolver call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcessTimeboxResolverOutcome {
    /// Resolver returns a safe summary.
    Resolved {
        /// Project the timebox binds to.
        project_ref: ProjectRef,
        /// Whether the timebox allows iteration opening.
        can_open_iteration: bool,
        /// Optional safe summary text.
        summary: Option<SafeSummaryText>,
        /// Optional digest to simulate missing-digest failures.
        source_digest: Option<SourceDigest>,
    },
    /// External reference does not exist.
    Unresolved,
    /// External dependency is temporarily unavailable.
    Unavailable,
    /// External boundary rejects this timebox for Work use.
    Rejected,
}

impl FakeProcessTimeboxResolverPort {
    /// Creates an empty fake process resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds the outcome returned for one process timebox ref.
    pub fn seed(&self, timebox_ref: ProcessTimeboxRef, outcome: ProcessTimeboxResolverOutcome) {
        if let Ok(mut outcomes) = self.outcomes.lock() {
            outcomes.insert(timebox_ref.0, outcome);
        }
    }
}

#[async_trait]
impl ProcessTimeboxResolverPort for FakeProcessTimeboxResolverPort {
    async fn resolve_timebox(
        &self,
        timebox_ref: ProcessTimeboxRef,
    ) -> Result<ProcessTimeboxResolution, PortError> {
        let outcomes = self.outcomes.lock().map_err(|_| PortError::Unavailable)?;
        match outcomes
            .get(&timebox_ref.0)
            .cloned()
            .unwrap_or(ProcessTimeboxResolverOutcome::Unresolved)
        {
            ProcessTimeboxResolverOutcome::Resolved {
                project_ref,
                can_open_iteration,
                summary,
                source_digest,
            } => Ok(ProcessTimeboxResolution {
                timebox_ref: timebox_ref.clone(),
                summary: ProcessTimeboxSummary {
                    timebox_ref,
                    project_ref,
                    can_open_iteration,
                    summary,
                    source_digest: source_digest.unwrap_or_else(|| SourceDigest(String::new())),
                },
            }),
            ProcessTimeboxResolverOutcome::Unresolved => Err(PortError::NotFound),
            ProcessTimeboxResolverOutcome::Unavailable => Err(PortError::Unavailable),
            ProcessTimeboxResolverOutcome::Rejected => Err(PortError::Rejected),
        }
    }
}

/// In-memory fake trace handoff adapter.
#[derive(Clone, Default)]
pub struct FakeTraceHandoffPort {
    inner: Arc<Mutex<FakeTraceHandoffInner>>,
}

#[derive(Default)]
struct FakeTraceHandoffInner {
    intents: Vec<TraceHandoffIntent>,
    queued_results: VecDeque<Result<TraceHandoffRef, PortError>>,
}

impl FakeTraceHandoffPort {
    /// Creates an empty fake trace handoff adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues one handoff outcome for the next prepare call.
    pub fn push_result(&self, result: Result<TraceHandoffRef, PortError>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.queued_results.push_back(result);
        }
    }

    /// Returns captured handoff intents in call order.
    pub fn intents(&self) -> Vec<TraceHandoffIntent> {
        self.inner
            .lock()
            .map(|inner| inner.intents.clone())
            .unwrap_or_default()
    }
}

#[async_trait]
impl TraceHandoffPort for FakeTraceHandoffPort {
    async fn prepare_trace_handoff(
        &self,
        intent: TraceHandoffIntent,
    ) -> Result<TraceHandoffRef, PortError> {
        let mut inner = self.inner.lock().map_err(|_| PortError::Unavailable)?;
        inner.intents.push(intent);
        match inner.queued_results.pop_front() {
            Some(result) => result,
            None => Ok(TraceHandoffRef(format!(
                "trace-handoff-{}",
                inner.intents.len()
            ))),
        }
    }
}

/// In-memory fake archive handoff adapter.
#[derive(Clone, Default)]
pub struct FakeArchiveHandoffPort {
    inner: Arc<Mutex<FakeArchiveHandoffInner>>,
}

#[derive(Default)]
struct FakeArchiveHandoffInner {
    intents: Vec<ArchiveHandoffIntent>,
    queued_results: VecDeque<Result<ArchiveHandoffRef, PortError>>,
}

impl FakeArchiveHandoffPort {
    /// Creates an empty fake archive handoff adapter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues one handoff outcome for the next prepare call.
    pub fn push_result(&self, result: Result<ArchiveHandoffRef, PortError>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.queued_results.push_back(result);
        }
    }

    /// Returns captured archive intents in call order.
    pub fn intents(&self) -> Vec<ArchiveHandoffIntent> {
        self.inner
            .lock()
            .map(|inner| inner.intents.clone())
            .unwrap_or_default()
    }
}

#[async_trait]
impl ArchiveHandoffPort for FakeArchiveHandoffPort {
    async fn prepare_archive_handoff(
        &self,
        intent: ArchiveHandoffIntent,
    ) -> Result<ArchiveHandoffRef, PortError> {
        let mut inner = self.inner.lock().map_err(|_| PortError::Unavailable)?;
        inner.intents.push(intent);
        match inner.queued_results.pop_front() {
            Some(result) => result,
            None => Ok(ArchiveHandoffRef(format!(
                "archive-handoff-{}",
                inner.intents.len()
            ))),
        }
    }
}
