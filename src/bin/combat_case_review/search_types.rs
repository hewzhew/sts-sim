use serde::Serialize;
use sts_simulator::ai::combat_search_v2::SearchTerminalLabel;
use sts_simulator::state::core::ClientInput;

#[derive(Serialize)]
pub(super) struct SearchReview {
    pub(super) label: &'static str,
    pub(super) nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) rollout_policy: &'static str,
    pub(super) turn_plan_policy: &'static str,
    pub(super) phase_guard_policy: &'static str,
    pub(super) child_rollout_policy: &'static str,
    pub(super) potion_policy: &'static str,
    pub(super) max_potions_used: Option<u32>,
    pub(super) complete_win: bool,
    pub(super) hp_loss: Option<i32>,
    pub(super) final_hp: Option<i32>,
    pub(super) turns: Option<u32>,
    pub(super) potions_used: Option<u32>,
    pub(super) nodes_expanded: u64,
    pub(super) nodes_generated: u64,
    pub(super) nodes_to_first_win: Option<u64>,
    pub(super) terminal_wins: u64,
    pub(super) elapsed_ms: u128,
    pub(super) deadline_hit: bool,
    pub(super) node_budget_hit: bool,
    pub(super) performance: SearchPerformanceReview,
    pub(super) facts: SearchReviewFacts,
}

#[derive(Serialize)]
pub(super) struct SearchPerformanceReview {
    pub(super) total_us: u128,
    pub(super) rollout_us: u128,
    pub(super) rollout_calls: u64,
    pub(super) root_rollout_calls: u64,
    pub(super) child_rollout_calls: u64,
    pub(super) deferred_child_rollout_calls: u64,
    pub(super) turn_plan_seed_rollout_calls: u64,
    pub(super) rollout_evaluations: u64,
    pub(super) rollout_budget_skips: u64,
    pub(super) rollout_max_evaluation_budget_skips: u64,
    pub(super) rollout_deadline_budget_skips: u64,
    pub(super) deferred_child_rollout_admitted_signal: u64,
    pub(super) deferred_child_rollout_admitted_periodic: u64,
    pub(super) deferred_child_rollout_skipped_low_signal: u64,
    pub(super) deferred_child_rollout_skipped_budget_share: u64,
    pub(super) turn_plan_seed_us: u128,
    pub(super) engine_step_us: u128,
    pub(super) frontier_pop_us: u128,
    pub(super) expansion_us: u128,
    pub(super) child_bookkeeping_us: u128,
    pub(super) rollout_profile: SearchRolloutPerformanceReview,
}

#[derive(Serialize)]
pub(super) struct SearchRolloutPerformanceReview {
    pub(super) cache_queries: u64,
    pub(super) cache_hits: u64,
    pub(super) cache_misses: u64,
    pub(super) cache_lookup_us: u128,
    pub(super) policy_dispatch_us: u128,
    pub(super) no_potion_iterations: u64,
    pub(super) no_potion_phase_profile_us: u128,
    pub(super) no_potion_legal_actions_us: u128,
    pub(super) no_potion_choose_action_us: u128,
    pub(super) no_potion_choose_ordering_us: u128,
    pub(super) no_potion_probe_us: u128,
    pub(super) no_potion_probe_score_calls: u64,
    pub(super) no_potion_probe_actions_evaluated: u64,
    pub(super) no_potion_probe_step_reuses: u64,
    pub(super) no_potion_probe_engine_step_us: u128,
    pub(super) no_potion_probe_phase_profile_us: u128,
    pub(super) no_potion_probe_action_facts_us: u128,
    pub(super) no_potion_engine_step_us: u128,
    pub(super) no_potion_child_build_us: u128,
}

#[derive(Serialize)]
pub(super) struct SearchReviewFacts {
    pub(super) diagnostic_progress: Option<SearchDiagnosticProgressFacts>,
}

#[derive(Clone, Serialize)]
pub(super) struct SearchDiagnosticProgressFacts {
    pub(super) source: &'static str,
    pub(super) terminal: SearchTerminalLabel,
    pub(super) estimated: bool,
    pub(super) final_hp: i32,
    pub(super) hp_loss: i32,
    pub(super) turns: u32,
    pub(super) potions_used: u32,
    pub(super) cards_played: u32,
    pub(super) living_enemy_count: usize,
    pub(super) total_enemy_hp: i32,
    pub(super) visible_incoming_damage: Option<i32>,
    pub(super) action_count: Option<usize>,
    pub(super) exact_prefix_action_count: Option<usize>,
    pub(super) action_key_preview: Vec<String>,
    pub(super) input_preview: Vec<ClientInput>,
}
