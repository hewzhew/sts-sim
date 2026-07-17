use super::{
    CompleteTurnOption, CompleteTurnOptionBoundary, ExactImmediateOptionProspect, ReplayError,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OptionProspectId(pub u64);

#[derive(Clone, Debug)]
pub struct OptionProspect {
    id: OptionProspectId,
    option: CompleteTurnOption,
    immediate: ExactImmediateOptionProspect,
    continuation: ContinuationEvidence,
}

impl OptionProspect {
    pub(crate) fn new(
        id: OptionProspectId,
        option: CompleteTurnOption,
        immediate: ExactImmediateOptionProspect,
        continuation: ContinuationEvidence,
    ) -> Self {
        Self {
            id,
            option,
            immediate,
            continuation,
        }
    }

    pub fn id(&self) -> OptionProspectId {
        self.id
    }

    pub fn option(&self) -> &CompleteTurnOption {
        &self.option
    }

    pub fn immediate(&self) -> &ExactImmediateOptionProspect {
        &self.immediate
    }

    pub fn continuation(&self) -> &ContinuationEvidence {
        &self.continuation
    }

    pub(crate) fn set_continuation(&mut self, evidence: ContinuationEvidence) {
        self.continuation = evidence;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContinuationEvidence {
    PendingBoundaryVerification,
    VerifiedBoundary(BoundaryWitnessEvidence),
    Unavailable(ContinuationUnavailable),
    Interrupted(ContinuationInterruption),
    VerificationFailed(ReplayError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundaryWitnessEvidence {
    pub boundary: CompleteTurnOptionBoundary,
    pub exact_successor_hash: String,
    pub replay_engine_steps: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContinuationUnavailable {
    FutureTurnPlanningNotStarted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContinuationInterruption {
    EngineStepBudget,
    Deadline,
}
