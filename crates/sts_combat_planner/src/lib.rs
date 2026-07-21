//! Exact, resumable planning over complete player-turn options.
//!
//! It turns the simulator's exact legal input surface into replayable options
//! ending at the next supported combat boundary. Optional policies guide work
//! order but never change legality or terminal truth. Partial action prefixes
//! remain private generator work.
//! The crate boundary also keeps planner iteration out of the core unit-test
//! harness; production integration belongs to the control layer.

mod agenda;
mod decision;
mod evidence;
mod generator;
mod layered_witness_search;
mod policy;
mod prospect;
mod replay;
mod selection_transaction;
mod types;
mod witness_search;

pub use agenda::{
    CombatPlannerAgendaBudget, CombatPlannerAgendaConfig, CombatPlannerAgendaCounters,
    CombatPlannerAgendaInterruption, CombatPlannerAgendaQuantum, CombatPlannerAgendaReport,
    CombatPlannerAgendaSession, CombatPlannerAgendaStatus,
};
pub use decision::{
    decide_combat_option, CombatEvaluationContext, CombatPlannerDecision,
    CombatPlannerDecisionBasis, CombatPlannerDecisionDeferral, CombatPlannerDecisionGap,
    CombatPlannerDecisionResult, ProspectEvidenceGap,
};
pub use evidence::{
    BoundaryWitnessEvidence, ContinuationEvidence, ContinuationInterruption, ExactHorizonEvidence,
    ExactHorizonGenerationGapEvidence, OptionProspect, OptionProspectId,
};
pub use generator::TurnOptionGeneratorSession;
pub use layered_witness_search::{
    rank_layered_combat_lineage_parents, search_layered_combat_witness,
    LayeredCombatCandidateRaceConfig, LayeredCombatCandidateRaceCounters,
    LayeredCombatCandidateRaceEntryReport, LayeredCombatCandidateRaceReport,
    LayeredCombatCandidateRaceSession, LayeredCombatCandidateRaceStatus,
    LayeredCombatDeferredWindow, LayeredCombatFrontierState, LayeredCombatLayerReport,
    LayeredCombatLineageParentRank, LayeredCombatLineagePortfolioConfig,
    LayeredCombatLineagePortfolioCounters, LayeredCombatLineagePortfolioEntryReport,
    LayeredCombatLineagePortfolioReport, LayeredCombatLineagePortfolioSession,
    LayeredCombatLineagePortfolioStatus, LayeredCombatLineageWindow, LayeredCombatParentWorkReport,
    LayeredCombatWitnessBudget, LayeredCombatWitnessConfig, LayeredCombatWitnessCounters,
    LayeredCombatWitnessInterruption, LayeredCombatWitnessQuantum, LayeredCombatWitnessReport,
    LayeredCombatWitnessSession, LayeredCombatWitnessStatus,
};
pub use policy::{
    CombatActionPolicy, CombatGuideLaneId, CombatPolicyChoice, CombatPolicyWitnessProposal,
    CombatStateGuide, CombatStateGuideRank, SharedCombatActionPolicy, UniformCombatActionPolicy,
};
pub use prospect::{
    ExactCombatZoneCounts, ExactCountChange, ExactI32Change, ExactImmediateOptionProspect,
    ExactProspectError,
};
pub use replay::{
    replay_turn_option, ReplayError, ReplayFailure, ReplayLimits, VerifiedTurnOptionReplay,
};
pub use types::{
    CombatDecisionRoot, CombatDecisionRootError, CombatPlanningCounters, CombatPlanningQuantum,
    CompleteTurnOption, CompleteTurnOptionBoundary, GenerationInterruption, TurnOptionAction,
    TurnOptionGenerationDiagnostics, TurnOptionGenerationGap, TurnOptionGenerationGapKind,
    TurnOptionGenerationReport, TurnOptionGenerationStatus, TurnOptionGeneratorConfig,
};
pub use witness_search::{
    OracleCombatDeepStateSnapshot, OracleCombatGuideQueueSnapshot, OracleCombatGuideRankSnapshot,
    OracleCombatOneTurnLossEvidence, OracleCombatOneTurnViabilityEvidence,
    OracleCombatRootActionFamilySnapshot, OracleCombatWitness, OracleCombatWitnessConfig,
    OracleCombatWitnessCounters, OracleCombatWitnessDiscoverySource,
    OracleCombatWitnessInterruption, OracleCombatWitnessProgressSnapshot,
    OracleCombatWitnessQuantum, OracleCombatWitnessReplayError, OracleCombatWitnessReport,
    OracleCombatWitnessSatisfaction, OracleCombatWitnessSession,
    OracleCombatWitnessStateMembershipSnapshot, OracleCombatWitnessStateProgressSnapshot,
    OracleCombatWitnessStatus,
};

#[cfg(test)]
mod tests;
