use std::time::Duration;

use sts_simulator::ai::combat_search_v2::{
    compile_combat_search_witness_prior_v0, CombatSearchV2Config, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
    CombatSearchV2WitnessReplay,
};
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::sim::combat::CombatTerminal;

use super::super::options::ReviewOptions;
use super::super::search_runner::run_configured_search;
use super::types::{CombatReviewFocus, CombatReviewFocusPriorRerun};
use super::witness::focus_witness_line;

pub(crate) fn witness_prior_rerun(
    options: &ReviewOptions,
    case: &CombatCase,
    focus: &CombatReviewFocus,
    replay: &CombatSearchV2WitnessReplay,
) -> Option<CombatReviewFocusPriorRerun> {
    if focus.progress.source != "rollout_frontier"
        || !matches!(replay.terminal, CombatTerminal::Win)
    {
        return None;
    }
    let witness_prior =
        compile_combat_search_witness_prior_v0(&case.position, &focus_witness_line(focus));
    if witness_prior.prior.is_empty() {
        return None;
    }
    let prior_states = witness_prior.prior_states;
    let duplicate_prior_hints = witness_prior.duplicate_prior_hints;
    let rollout_policy = if options.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    let (rerun, _) = run_configured_search(
        "focus_witness_prior_rerun",
        case,
        CombatSearchV2Config {
            max_nodes: options.fast_nodes,
            wall_time: Some(Duration::from_millis(options.fast_ms)),
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            potion_policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: Some(0),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
            rollout_policy,
            child_rollout_policy: options.child_rollout_policy(),
            root_action_prior: Some(witness_prior.prior),
            ..CombatSearchV2Config::default()
        },
        options.action_preview_limit,
    );
    Some(CombatReviewFocusPriorRerun {
        selected_review: focus.selected_review,
        witness_replayed_actions: replay.replayed_actions,
        witness_action_count: focus.progress.action_count,
        witness_terminal: replay.terminal,
        prior_states,
        duplicate_prior_hints,
        rerun,
    })
}
