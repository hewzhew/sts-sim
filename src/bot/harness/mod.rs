//! App-layer bot harnesses and evaluation workbenches.

mod boss_validation;
mod combat_env;
mod combat_lab;
mod combat_policy;

pub use boss_validation::{build_ledger_record, validate_case};
pub use combat_env::{
    ActionMask, CombatAction, CombatEnv, CombatEnvSpec, CombatEpisodeOutcome, CombatObservation,
    CombatRewardBreakdown,
};
pub use combat_lab::{
    run_combat_lab, write_sanitized_fixture_for_local_lab, CombatLabConfig, LabVariantMode,
};
pub use combat_policy::PolicyKind;
