//! App-layer bot harnesses and evaluation workbenches.

mod combat_env;

pub use combat_env::{
    ActionMask, CombatAction, CombatEnv, CombatEnvDrawOrderVariant, CombatEnvSpec,
    CombatEpisodeOutcome, CombatObservation, CombatRewardBreakdown,
};
