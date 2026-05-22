mod combat_start;
mod commands;
mod decision_case;
pub mod outcome;
mod panels;
pub mod registry;
mod render;
mod session;
mod view_model;

pub use commands::{
    parse_run_control_command, run_control_help, run_control_short_hint, RunControlCommand,
};
pub use decision_case::{
    default_run_decision_case_path, save_run_decision_case_v1, RunDecisionCaseV1,
    RUN_DECISION_CASE_SCHEMA_NAME, RUN_DECISION_CASE_SCHEMA_VERSION,
};
pub use outcome::{
    load_combat_baseline_outcome_v1, save_combat_baseline_outcome_v1, CombatBaselineOutcomeV1,
    COMBAT_BASELINE_OUTCOME_SCHEMA_NAME, COMBAT_BASELINE_OUTCOME_SCHEMA_VERSION,
};
pub use registry::{add_case_to_benchmark_registry, BenchmarkCasePaths};
pub use render::{
    render_combat_actions, render_run_control_details, render_run_control_raw,
    render_run_control_state,
};
pub use session::{
    canonical_player_class, RunControlCommandOutcome, RunControlConfig, RunControlSession,
};
