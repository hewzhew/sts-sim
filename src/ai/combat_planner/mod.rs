//! Exact, resumable planning over complete player-turn options.
//!
//! This module owns no combat policy. It turns the simulator's exact legal
//! input surface into replayable options ending at the next supported combat
//! boundary. Partial action prefixes remain private generator work.

mod generator;
mod prospect;
mod replay;
mod selection_transaction;
mod types;

pub use generator::TurnOptionGeneratorSession;
pub use prospect::{
    ExactCombatZoneCounts, ExactCountChange, ExactI32Change, ExactImmediateOptionProspect,
    ExactProspectError,
};
pub use replay::{replay_turn_option, ReplayError, ReplayLimits, VerifiedTurnOptionReplay};
pub use types::{
    CombatDecisionRoot, CombatDecisionRootError, CombatPlanningCounters, CombatPlanningQuantum,
    CompleteTurnOption, CompleteTurnOptionBoundary, GenerationInterruption, TurnOptionAction,
    TurnOptionGenerationGap, TurnOptionGenerationGapKind, TurnOptionGenerationReport,
    TurnOptionGenerationStatus, TurnOptionGeneratorConfig,
};

#[cfg(test)]
mod tests;
