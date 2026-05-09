//! App-layer bot harnesses and evaluation workbenches.

mod combat_env;
mod combat_lab;
mod combat_policy;

pub use combat_env::{
    ActionMask, CombatAction, CombatEnv, CombatEnvDrawOrderVariant, CombatEnvSpec,
    CombatEpisodeOutcome, CombatObservation, CombatRewardBreakdown,
};
pub use combat_lab::{
    run_combat_case_lab, run_combat_lab, write_sanitized_case_for_local_lab,
    write_sanitized_fixture_for_local_lab, CombatCaseLabConfig, CombatLabConfig, LabVariantMode,
};
pub use combat_policy::PolicyKind;
