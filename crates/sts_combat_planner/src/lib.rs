//! Exact, resumable planning over complete player-turn options.
//!
//! This module owns no combat policy. It turns the simulator's exact legal
//! input surface into replayable options ending at the next supported combat
//! boundary. Partial action prefixes remain private generator work.
//! The crate boundary also keeps planner iteration out of the core unit-test
//! harness; production integration belongs to the control layer.

mod agenda;
mod decision;
mod evidence;
mod generator;
mod prospect;
mod replay;
mod selection_transaction;
mod types;

pub use agenda::{
    CombatPlannerAgendaBudget, CombatPlannerAgendaConfig, CombatPlannerAgendaCounters,
    CombatPlannerAgendaInterruption, CombatPlannerAgendaQuantum, CombatPlannerAgendaReport,
    CombatPlannerAgendaSession, CombatPlannerAgendaStatus,
};
pub use decision::{
    decide_combat_option, CombatEvaluationContext, CombatPlannerDecision,
    CombatPlannerDecisionBasis, CombatPlannerDecisionDeferral, CombatPlannerDecisionGap,
    CombatPlannerDecisionResult, CombatPlannerIncumbentEvaluator, ProspectEvidenceGap,
};
pub use evidence::{
    BoundaryWitnessEvidence, ContinuationEvidence, ContinuationInterruption, ExactHorizonEvidence,
    ExactHorizonGenerationGapEvidence, OptionProspect, OptionProspectId,
};
pub use generator::TurnOptionGeneratorSession;
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
    TurnOptionGenerationGap, TurnOptionGenerationGapKind, TurnOptionGenerationReport,
    TurnOptionGenerationStatus, TurnOptionGeneratorConfig,
};

#[cfg(test)]
mod tests;
