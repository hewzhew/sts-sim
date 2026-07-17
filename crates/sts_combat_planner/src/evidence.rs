use super::{
    CombatDecisionRootError, CombatPlanningCounters, CompleteTurnOption,
    CompleteTurnOptionBoundary, ExactImmediateOptionProspect, ReplayError, TurnOptionGenerationGap,
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

#[derive(Clone, Debug, PartialEq)]
pub enum ContinuationEvidence {
    PendingBoundaryVerification,
    PendingContinuationRefinement,
    VerifiedBoundary(BoundaryWitnessEvidence),
    ExactHorizon(ExactHorizonEvidence),
    ExactHorizonGenerationGap(ExactHorizonGenerationGapEvidence),
    Interrupted(ContinuationInterruption),
    ConstructionFailed(CombatDecisionRootError),
    VerificationFailed(ReplayError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundaryWitnessEvidence {
    pub boundary: CompleteTurnOptionBoundary,
    pub exact_successor_hash: String,
    pub replay_engine_steps: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContinuationInterruption {
    GenerationWorkBudget,
    EngineStepBudget,
    Deadline,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExactHorizonEvidence {
    pub turn_boundaries: u16,
    pub complete_options: Vec<CompleteTurnOption>,
    pub work: CombatPlanningCounters,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExactHorizonGenerationGapEvidence {
    pub requested_turn_boundaries: u16,
    pub complete_options: Vec<CompleteTurnOption>,
    pub gaps: Vec<TurnOptionGenerationGap>,
    pub work: CombatPlanningCounters,
}
