use std::time::Duration;

use crate::state::core::ClientInput;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct CombatSearchV2Config {
    pub max_nodes: usize,
    pub max_actions_per_line: usize,
    pub max_engine_steps_per_action: usize,
    pub wall_time: Option<Duration>,
    pub input_label: Option<String>,
    pub potion_policy: CombatSearchV2PotionPolicy,
}

impl Default for CombatSearchV2Config {
    fn default() -> Self {
        Self {
            max_nodes: 50_000,
            max_actions_per_line: 200,
            max_engine_steps_per_action: 250,
            wall_time: None,
            input_label: None,
            potion_policy: CombatSearchV2PotionPolicy::Never,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2PotionPolicy {
    Never,
    #[serde(alias = "all_legal_potion_actions")]
    All,
}

impl CombatSearchV2PotionPolicy {
    pub(super) fn label(self) -> &'static str {
        match self {
            CombatSearchV2PotionPolicy::Never => "never",
            CombatSearchV2PotionPolicy::All => "all_legal_potion_actions",
        }
    }
}

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
    pub diagnostics: CombatSearchV2DiagnosticsReport,
    pub stats: CombatSearchV2Stats,
    pub evidence_reliability: CombatSearchV2EvidenceReport,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2PolicyReport {
    pub kind: &'static str,
    pub terminal_policy: &'static str,
    pub expansion_order: &'static str,
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
    pub sample_states: Vec<CombatSearchV2StateSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsReport {
    pub schema_version: u32,
    pub mode: &'static str,
    pub tables: CombatSearchV2DiagnosticsTables,
    pub branching: CombatSearchV2DiagnosticsBranching,
    pub expansion: CombatSearchV2DiagnosticsExpansion,
    pub pruning: CombatSearchV2DiagnosticsPruning,
    pub frontier: CombatSearchV2DiagnosticsFrontier,
    pub diagnosis: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTables {
    pub exact_keys: usize,
    pub exact_resource_vectors: usize,
    pub dominance_buckets: usize,
    pub dominance_resource_vectors: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsBranching {
    pub states_queried: u64,
    pub states_with_legal_actions: u64,
    pub legal_actions_total: u64,
    pub legal_actions_avg: f64,
    pub legal_actions_max: usize,
    pub nodes_generated_per_expanded: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsExpansion {
    pub grouping_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub total_atomic_actions: u64,
    pub total_fanout_groups: u64,
    pub fanout_groups_avg: f64,
    pub fanout_groups_max: usize,
    pub max_group_size: usize,
    pub action_kind_counts: Vec<CombatSearchV2DiagnosticsActionKindCount>,
    pub largest_groups: Vec<CombatSearchV2DiagnosticsActionGroupSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsActionKindCount {
    pub kind: String,
    pub atomic_actions: u64,
    pub fanout_groups: u64,
    pub max_group_size: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsActionGroupSample {
    pub observed_at_state_query: u64,
    pub kind: String,
    pub group_key: String,
    pub atomic_actions: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsPruning {
    pub transposition_prunes: u64,
    pub dominance_prunes: u64,
    pub terminal_wins: u64,
    pub terminal_losses: u64,
    pub unresolved_leaf_count: u64,
    pub max_actions_cut_count: u64,
    pub engine_step_limit_count: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsFrontier {
    pub remaining_states: usize,
    pub sample_limit: usize,
    pub sampled_states: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatSearchV2Stats {
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
    pub nodes_to_first_win: Option<u64>,
    pub terminal_wins: u64,
    pub terminal_losses: u64,
    pub dominance_prunes: u64,
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

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TrajectoryReport {
    pub terminal: SearchTerminalLabel,
    pub estimated: bool,
    pub actions: Vec<CombatSearchV2ActionTrace>,
    pub final_hp: i32,
    pub final_block: i32,
    pub hp_loss: i32,
    pub turns: u32,
    pub potions_used: u32,
    pub potions_discarded: u32,
    pub cards_played: u32,
    pub enemy_final_state: Vec<CombatSearchV2EnemySummary>,
    pub final_state: CombatSearchV2StateSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2ActionTrace {
    pub step_index: usize,
    pub action_id: usize,
    pub action_key: String,
    pub action_debug: String,
    pub input: ClientInput,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2EnemySummary {
    pub slot: usize,
    pub entity_id: usize,
    pub enemy_id: String,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub escaped: bool,
    pub dying: bool,
    pub half_dead: bool,
    pub planned_move_id: u8,
    pub visible_intent: String,
    pub visible_incoming_damage: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2StateSummary {
    pub engine_state: String,
    pub terminal: SearchTerminalLabel,
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: u8,
    pub turn_count: u32,
    pub living_enemy_count: usize,
    pub total_enemy_hp: i32,
    pub visible_incoming_damage: i32,
    pub hand_count: usize,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub limbo_count: usize,
    pub queued_cards_count: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchTerminalLabel {
    Win,
    Loss,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchProofStatus {
    Exhaustive,
    BudgetExhausted,
    DeadlineHit,
    FrontierUnresolved,
}
