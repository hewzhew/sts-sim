use serde::Serialize;

use super::{
    CombatSearchV2DiagnosticsReport, CombatSearchV2StateSummary, CombatSearchV2TrajectoryReport,
    SearchProofStatus, SearchTerminalLabel,
};
#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2Report {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub input_label: Option<String>,
    pub information_boundary: &'static str,
    pub search_policy: CombatSearchV2PolicyReport,
    pub budget: CombatSearchV2BudgetReport,
    pub outcome: CombatSearchV2OutcomeReport,
    pub best_complete_trajectory: Option<CombatSearchV2TrajectoryReport>,
    pub best_frontier_trajectory: Option<CombatSearchV2TrajectoryReport>,
    pub frontier: CombatSearchV2FrontierReport,
    pub rollout: CombatSearchV2RolloutReport,
    pub diagnostics: CombatSearchV2DiagnosticsReport,
    pub stats: CombatSearchV2Stats,
    pub evidence_reliability: CombatSearchV2EvidenceReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2PolicyReport {
    pub kind: &'static str,
    pub terminal_policy: &'static str,
    pub expansion_order: &'static str,
    pub frontier_value: &'static str,
    pub turn_branching: &'static str,
    pub potion_policy: &'static str,
    pub transposition_table: &'static str,
    pub dominance_pruning: &'static str,
    pub rollout_value: &'static str,
    pub llm_authority: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2BudgetReport {
    pub max_nodes: usize,
    pub max_actions_per_line: usize,
    pub max_engine_steps_per_action: usize,
    pub wall_time_ms: Option<u128>,
    pub max_potions_used: Option<u32>,
    pub rollout_max_evaluations: usize,
    pub rollout_max_actions: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2OutcomeReport {
    pub terminal: SearchTerminalLabel,
    pub proof_status: SearchProofStatus,
    pub reason: String,
    pub complete_trajectory_found: bool,
    pub exhaustive: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2FrontierReport {
    pub remaining_states: usize,
    pub unresolved_leaf_count: u64,
    pub max_actions_cut_count: u64,
    pub engine_step_limit_count: u64,
    pub potion_budget_cut_count: u64,
    pub best_estimated_value: Option<CombatSearchV2FrontierValueReport>,
    pub sample_states: Vec<CombatSearchV2StateSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2FrontierValueReport {
    pub policy: &'static str,
    pub terminal: SearchTerminalLabel,
    pub player_hp: i32,
    pub player_block: i32,
    pub visible_incoming_damage: i32,
    pub survival_margin: i32,
    pub living_enemy_count: usize,
    pub total_enemy_hp: i32,
    pub total_enemy_block: i32,
    pub total_enemy_effort: i32,
    pub phase_adjusted_enemy_hp: i32,
    pub phase_adjusted_enemy_effort: i32,
    pub split_pending_count: usize,
    pub split_debt_hp: i32,
    pub guardian_defensive_count: usize,
    pub guardian_defensive_block: i32,
    pub guardian_mode_shift_pending_count: usize,
    pub lagavulin_waking_count: usize,
    pub gremlin_nob_anger_amount_total: i32,
    pub sentry_dazed_pressure_count: usize,
    pub hexaghost_opening_pressure_count: usize,
    pub phase_profile: CombatSearchV2PhaseProfileReport,
    pub sustained_mitigation: i32,
    pub hand: CombatSearchV2CardPileValueReport,
    pub next_draw: CombatSearchV2CardPileValueReport,
    pub enemy_mechanics: CombatSearchV2EnemyMechanicsReport,
    pub potions_used: u32,
    pub potions_discarded: u32,
    pub cards_played: u32,
    pub actions_taken: usize,
    pub estimated: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2PhaseProfileReport {
    pub profiling_policy: &'static str,
    pub special_enemy_phase_count: usize,
    pub split_pending_count: usize,
    pub split_debt_hp: i32,
    pub guardian_mode_shift_pending_count: usize,
    pub guardian_defensive_count: usize,
    pub lagavulin_sleeping_count: usize,
    pub lagavulin_waking_count: usize,
    pub pending_choice_present: bool,
    pub pending_choice_kind: Option<&'static str>,
    pub pending_choice_candidate_count: usize,
    pub pending_choice_estimated_action_fanout: usize,
    pub high_fanout_pending_choice: bool,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2CardPileValueReport {
    pub damage: i32,
    pub block: i32,
    pub playable_cards: i32,
    pub low_cost: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2EnemyMechanicsReport {
    pub profiling_policy: &'static str,
    pub tracked_monsters: usize,
    pub split_pending_count: usize,
    pub guardian_open_count: usize,
    pub guardian_defensive_count: usize,
    pub guardian_mode_shift_pending_count: usize,
    pub guardian_min_mode_shift_remaining: Option<i32>,
    pub lagavulin_sleeping_count: usize,
    pub lagavulin_waking_count: usize,
    pub gremlin_nob_enrage_count: usize,
    pub gremlin_nob_anger_amount_total: i32,
    pub sentry_dazed_pressure_count: usize,
    pub hexaghost_opening_pressure_count: usize,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2RolloutReport {
    pub policy: &'static str,
    pub behavioral_effect: &'static str,
    pub max_evaluations: usize,
    pub max_actions_per_rollout: usize,
    pub evaluations: u64,
    pub cache_hits: u64,
    pub budget_skips: u64,
    pub truncated_rollouts: u64,
    pub terminal_wins: u64,
    pub terminal_losses: u64,
    pub rollouts_with_pending_choice: u64,
    pub rollouts_stopped_on_high_fanout_pending_choice: u64,
    pub pending_choice_actions_simulated: u64,
    pub max_pending_choice_estimated_action_fanout: usize,
    pub best_frontier_estimate: Option<CombatSearchV2RolloutEstimateReport>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2RolloutEstimateReport {
    pub terminal: SearchTerminalLabel,
    pub estimated: bool,
    pub final_hp: i32,
    pub hp_loss: i32,
    pub turns: u32,
    pub potions_used: u32,
    pub potions_discarded: u32,
    pub cards_played: u32,
    pub living_enemy_count: usize,
    pub total_enemy_hp: i32,
    pub total_enemy_block: i32,
    pub phase_adjusted_enemy_effort: i32,
    pub special_enemy_phase_count: usize,
    pub guardian_mode_shift_pending_count: usize,
    pub lagavulin_waking_count: usize,
    pub gremlin_nob_anger_amount_total: i32,
    pub sentry_dazed_pressure_count: usize,
    pub hexaghost_opening_pressure_count: usize,
    pub high_fanout_pending_choice: bool,
    pub pending_choice_estimated_action_fanout: usize,
    pub pending_choices_seen: usize,
    pub pending_choice_actions_simulated: usize,
    pub max_pending_choice_candidate_count: usize,
    pub max_pending_choice_estimated_action_fanout: usize,
    pub last_pending_choice_kind: Option<&'static str>,
    pub stopped_on_high_fanout_pending_choice: bool,
    pub survival_margin: i32,
    pub actions_simulated: usize,
    pub truncated: bool,
    pub stop_reason: &'static str,
    pub last_action_reason: Option<&'static str>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatSearchV2Stats {
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
    pub nodes_to_first_win: Option<u64>,
    pub terminal_wins: u64,
    pub terminal_losses: u64,
    pub dominance_prunes: u64,
    pub turn_local_dominance_prunes: u64,
    pub transposition_prunes: u64,
    pub deadline_hit: bool,
    pub node_budget_hit: bool,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2EvidenceReport {
    pub hidden_info_policy: &'static str,
    pub random_policy: &'static str,
    pub estimate_policy: &'static str,
    pub reliability: &'static str,
    pub warnings: Vec<&'static str>,
}
