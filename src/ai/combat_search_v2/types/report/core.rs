use serde::Serialize;

use super::super::{
    CombatSearchV2DiagnosticsReport, CombatSearchV2TrajectoryReport, SearchCoverageStatus,
};
use super::evidence::CombatSearchV2PolicyEvidenceReport;
use super::frontier::CombatSearchV2FrontierReport;
use super::rollout::CombatSearchV2RolloutReport;

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2Report {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub input_label: Option<String>,
    pub information_boundary: &'static str,
    pub policy_evidence: CombatSearchV2PolicyEvidenceReport,
    pub search_policy: CombatSearchV2PolicyReport,
    pub budget: CombatSearchV2BudgetReport,
    pub outcome: CombatSearchV2OutcomeReport,
    pub best_complete_trajectory: Option<CombatSearchV2TrajectoryReport>,
    pub best_win_trajectory: Option<CombatSearchV2TrajectoryReport>,
    #[serde(skip_serializing)]
    pub win_candidate_trajectories: Vec<CombatSearchV2TrajectoryReport>,
    pub best_frontier_trajectory: Option<CombatSearchV2TrajectoryReport>,
    pub frontier: CombatSearchV2FrontierReport,
    pub rollout: CombatSearchV2RolloutReport,
    pub diagnostics: CombatSearchV2DiagnosticsReport,
    pub stats: CombatSearchV2Stats,
    pub performance: CombatSearchV2PerformanceReport,
    pub evidence_reliability: CombatSearchV2EvidenceReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2PolicyReport {
    pub kind: &'static str,
    pub terminal_policy: &'static str,
    pub expansion_order: &'static str,
    pub frontier_value: &'static str,
    pub frontier_policy: &'static str,
    pub turn_branching: &'static str,
    pub turn_plan_policy: &'static str,
    pub potion_policy: &'static str,
    pub transposition_table: &'static str,
    pub dominance_pruning: &'static str,
    pub rollout_value: &'static str,
    pub child_rollout_policy: &'static str,
    pub llm_authority: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2BudgetReport {
    pub max_nodes: usize,
    pub max_actions_per_line: usize,
    pub max_engine_steps_per_action: usize,
    pub wall_time_ms: Option<u128>,
    pub stop_on_win_hp_loss_at_most: Option<u32>,
    pub min_win_candidates_before_stop: usize,
    pub max_potions_used: Option<u32>,
    pub rollout_max_evaluations: usize,
    pub rollout_max_actions: usize,
    pub rollout_beam_width: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2OutcomeReport {
    pub coverage_status: SearchCoverageStatus,
    pub coverage_reason: String,
    pub complete_trajectory_found: bool,
    pub complete_win_found: bool,
    pub exhaustive: bool,
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

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatSearchV2PerformanceReport {
    pub total_elapsed_us: u128,
    pub unattributed_elapsed_us: u128,
    pub rollout_estimate_calls: u64,
    pub root_rollout_estimate_calls: u64,
    pub child_rollout_estimate_calls: u64,
    pub deferred_child_rollout_estimate_calls: u64,
    pub turn_plan_seed_rollout_estimate_calls: u64,
    pub deferred_child_rollout_nodes: u64,
    pub deferred_child_rollout_requeues: u64,
    pub terminal_child_rollout_skips: u64,
    pub terminal_turn_plan_seed_rollout_skips: u64,
    pub turn_local_dominance_rollout_skips: u64,
    pub rollout_estimate_elapsed_us: u128,
    pub engine_step_calls: u64,
    pub engine_step_elapsed_us: u128,
    pub frontier_pop_calls: u64,
    pub frontier_pop_elapsed_us: u128,
    pub pre_expand_elapsed_us: u128,
    pub expansion_elapsed_us: u128,
    pub child_bookkeeping_elapsed_us: u128,
    pub turn_plan_frontier_seed_calls: u64,
    pub turn_plan_frontier_seed_elapsed_us: u128,
    pub shadow_audit_elapsed_us: u128,
    pub root_turn_plan_diagnostics_elapsed_us: u128,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2EvidenceReport {
    pub hidden_info_policy: &'static str,
    pub random_policy: &'static str,
    pub estimate_policy: &'static str,
    pub reliability: &'static str,
    pub warnings: Vec<&'static str>,
}
