use std::time::Duration;

use crate::state::core::ClientInput;
use serde::{Deserialize, Serialize};

use super::state_abstraction::{
    StateAbstractionBoundaryId, StateAbstractionConsumer, StateAbstractionRevealGate,
    StateAbstractionSoundnessLevel, StateDivergenceKind,
};

#[derive(Clone, Debug)]
pub struct CombatSearchV2Config {
    pub max_nodes: usize,
    pub max_actions_per_line: usize,
    pub max_engine_steps_per_action: usize,
    pub wall_time: Option<Duration>,
    pub input_label: Option<String>,
    pub potion_policy: CombatSearchV2PotionPolicy,
    pub max_potions_used: Option<u32>,
    pub rollout_policy: CombatSearchV2RolloutPolicy,
    pub rollout_max_evaluations: usize,
    pub rollout_max_actions: usize,
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
            max_potions_used: None,
            rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            rollout_max_evaluations: super::rollout::DEFAULT_ROLLOUT_MAX_EVALUATIONS,
            rollout_max_actions: super::rollout::DEFAULT_ROLLOUT_MAX_ACTIONS,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2PotionPolicy {
    Never,
    #[serde(alias = "all_legal_potion_actions")]
    All,
    #[serde(alias = "semantic_budgeted_potion_actions")]
    SemanticBudgeted,
}

impl CombatSearchV2PotionPolicy {
    pub(super) fn label(self) -> &'static str {
        match self {
            CombatSearchV2PotionPolicy::Never => "never",
            CombatSearchV2PotionPolicy::All => "all_legal_potion_actions",
            CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic_budgeted_potion_actions",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2RolloutPolicy {
    Disabled,
    ConservativeNoPotion,
}

impl Default for CombatSearchV2RolloutPolicy {
    fn default() -> Self {
        Self::ConservativeNoPotion
    }
}

impl CombatSearchV2RolloutPolicy {
    pub(super) fn label(self) -> &'static str {
        match self {
            CombatSearchV2RolloutPolicy::Disabled => "disabled",
            CombatSearchV2RolloutPolicy::ConservativeNoPotion => "conservative_no_potion",
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
    pub high_fanout_pending_choice: bool,
    pub pending_choice_estimated_action_fanout: usize,
    pub survival_margin: i32,
    pub actions_simulated: usize,
    pub truncated: bool,
    pub stop_reason: &'static str,
    pub last_action_reason: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsReport {
    pub schema_version: u32,
    pub mode: &'static str,
    pub tables: CombatSearchV2DiagnosticsTables,
    pub branching: CombatSearchV2DiagnosticsBranching,
    pub expansion: CombatSearchV2DiagnosticsExpansion,
    pub target_fanout: CombatSearchV2DiagnosticsTargetFanout,
    pub equivalence: CombatSearchV2DiagnosticsEquivalence,
    pub ordering: CombatSearchV2DiagnosticsOrdering,
    pub turn_branching: CombatSearchV2DiagnosticsTurnBranching,
    pub pending_choice: CombatSearchV2DiagnosticsPendingChoice,
    pub turn_prefix: CombatSearchV2DiagnosticsTurnPrefix,
    pub turn_sequence: CombatSearchV2DiagnosticsTurnSequence,
    pub card_identity: CombatSearchV2DiagnosticsCardIdentity,
    pub turn_local_dominance: CombatSearchV2DiagnosticsTurnLocalDominance,
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
pub struct CombatSearchV2DiagnosticsTargetFanout {
    pub grouping_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub targeted_actions_total: u64,
    pub target_fanout_groups_total: u64,
    pub multi_target_fanout_groups: u64,
    pub avg_targets_per_group: f64,
    pub max_targets_per_group: usize,
    pub lethal_target_groups: u64,
    pub unique_lethal_target_groups: u64,
    pub uniform_damage_groups: u64,
    pub max_target_hp_span: i32,
    pub group_kind_counts: Vec<CombatSearchV2DiagnosticsTargetFanoutKindCount>,
    pub largest_target_fanouts: Vec<CombatSearchV2DiagnosticsTargetFanoutSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTargetFanoutKindCount {
    pub kind: String,
    pub groups: u64,
    pub actions: u64,
    pub multi_target_groups: u64,
    pub lethal_target_groups: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTargetFanoutSample {
    pub observed_at_state_query: u64,
    pub kind: String,
    pub source_key: String,
    pub target_count: usize,
    pub lethal_targets: usize,
    pub min_target_hp_with_block: i32,
    pub max_target_hp_with_block: i32,
    pub target_hp_span: i32,
    pub min_damage_hint: i32,
    pub max_damage_hint: i32,
    pub first_action_key: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsEquivalence {
    pub equivalence_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub states_compressed: u64,
    pub atomic_actions_in: u64,
    pub representative_actions_out: u64,
    pub actions_removed: u64,
    pub removed_action_ratio: f64,
    pub max_group_size: usize,
    pub group_kind_counts: Vec<CombatSearchV2DiagnosticsEquivalenceKindCount>,
    pub largest_groups: Vec<CombatSearchV2DiagnosticsEquivalenceGroupSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsEquivalenceKindCount {
    pub kind: String,
    pub boundary_id: StateAbstractionBoundaryId,
    pub soundness: StateAbstractionSoundnessLevel,
    pub allowed_consumers: Vec<StateAbstractionConsumer>,
    pub groups: u64,
    pub actions_in: u64,
    pub actions_removed: u64,
    pub max_group_size: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsEquivalenceGroupSample {
    pub observed_at_state_query: u64,
    pub kind: String,
    pub boundary_id: StateAbstractionBoundaryId,
    pub soundness: StateAbstractionSoundnessLevel,
    pub allowed_consumers: Vec<StateAbstractionConsumer>,
    pub equivalence_key: String,
    pub representative_original_action_id: usize,
    pub removed_original_action_ids: Vec<usize>,
    pub group_size: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsOrdering {
    pub ordering_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub states_reordered: u64,
    pub reordered_state_ratio: f64,
    pub total_actions_observed: u64,
    pub action_effect_actions: u64,
    pub phase_action_hint_actions: u64,
    pub max_position_shift: usize,
    pub avg_position_shift: f64,
    pub action_role_counts: Vec<CombatSearchV2DiagnosticsActionRoleCount>,
    pub largest_reorders: Vec<CombatSearchV2DiagnosticsOrderingSample>,
    pub action_effect_samples: Vec<CombatSearchV2DiagnosticsActionEffectSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsActionRoleCount {
    pub role: String,
    pub actions: u64,
    pub first_actions: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsOrderingSample {
    pub observed_at_state_query: u64,
    pub action_count: usize,
    pub max_position_shift: usize,
    pub first_role: String,
    pub first_original_action_id: usize,
    pub first_action_key: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsActionEffectSample {
    pub observed_at_state_query: u64,
    pub original_action_id: usize,
    pub ordered_index: usize,
    pub role: String,
    pub action_key: String,
    pub mitigation_score: i32,
    pub reactive_risk_score: i32,
    pub enemy_strength_gain: i32,
    pub visible_attack_pressure_hint: i32,
    pub reactive_player_hp_loss: i32,
    pub reactive_player_block: i32,
    pub reactive_enemy_damage: i32,
    pub reactive_bad_draw_cards: i32,
    pub reactive_forced_turn_end: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnBranching {
    pub organization_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub total_legal_actions: u64,
    pub total_generated_children: u64,
    pub generated_children_per_state: f64,
    pub same_turn_children: u64,
    pub next_turn_children: u64,
    pub pending_choice_children: u64,
    pub terminal_children: u64,
    pub other_children: u64,
    pub end_turn_children: u64,
    pub same_turn_child_ratio: f64,
    pub next_turn_child_ratio: f64,
    pub transition_counts: Vec<CombatSearchV2DiagnosticsTurnTransitionCount>,
    pub largest_turn_fanouts: Vec<CombatSearchV2DiagnosticsTurnFanoutSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnTransitionCount {
    pub action_kind: String,
    pub transition_kind: String,
    pub children: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnFanoutSample {
    pub parent_turn_count: u32,
    pub parent_energy: u8,
    pub legal_actions: usize,
    pub generated_children: usize,
    pub same_turn_children: usize,
    pub next_turn_children: usize,
    pub pending_choice_children: usize,
    pub terminal_children: usize,
    pub end_turn_children: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsPendingChoice {
    pub profiling_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub pending_choice_states: u64,
    pub high_fanout_states: u64,
    pub max_candidate_count: usize,
    pub kind_counts: Vec<CombatSearchV2DiagnosticsPendingChoiceKindCount>,
    pub largest_pending_choices: Vec<CombatSearchV2DiagnosticsPendingChoiceSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsPendingChoiceKindCount {
    pub kind: String,
    pub states: u64,
    pub max_candidate_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsPendingChoiceSample {
    pub observed_at_state_query: u64,
    pub kind: String,
    pub reason: Option<String>,
    pub source_pile: Option<String>,
    pub candidate_count: usize,
    pub min_cards: usize,
    pub max_cards: usize,
    pub can_cancel: bool,
    pub fanout_class: &'static str,
    pub search_risk: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnPrefix {
    pub tracking_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub non_empty_prefix_states: u64,
    pub empty_prefix_states: u64,
    pub avg_prefix_length: f64,
    pub max_prefix_length: usize,
    pub max_legal_actions_after_non_empty_prefix: usize,
    pub total_cards_played_in_prefix: u64,
    pub total_potions_used_in_prefix: u64,
    pub total_potions_discarded_in_prefix: u64,
    pub total_other_actions_in_prefix: u64,
    pub prefix_length_counts: Vec<CombatSearchV2DiagnosticsTurnPrefixLengthCount>,
    pub prefix_kind_counts: Vec<CombatSearchV2DiagnosticsTurnPrefixKindCount>,
    pub largest_prefix_fanouts: Vec<CombatSearchV2DiagnosticsTurnPrefixFanoutSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnPrefixLengthCount {
    pub prefix_length: usize,
    pub states: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnPrefixKindCount {
    pub kind: String,
    pub states: u64,
    pub legal_actions_total: u64,
    pub max_prefix_length: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnPrefixFanoutSample {
    pub observed_at_state_query: u64,
    pub prefix_length: usize,
    pub kind: String,
    pub cards_played: usize,
    pub potions_used: usize,
    pub potions_discarded: usize,
    pub other_actions: usize,
    pub legal_actions: usize,
    pub signature_preview: String,
    pub signature_truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnSequence {
    pub grouping_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub non_empty_prefix_states: u64,
    pub grouped_prefix_states: u64,
    pub unordered_sequence_groups: usize,
    pub groups_with_order_variants: usize,
    pub same_effect_order_variant_groups: usize,
    pub order_sensitive_groups: usize,
    pub max_ordered_variants_per_group: usize,
    pub max_effect_variants_per_group: usize,
    pub max_prefix_length: usize,
    pub max_legal_actions_after_prefix: usize,
    pub order_sensitive_divergence_histogram:
        Vec<CombatSearchV2DiagnosticsTurnSequenceDivergenceCount>,
    pub discard_order_shadow_audit: CombatSearchV2DiagnosticsDiscardOrderShadowAudit,
    pub largest_groups: Vec<CombatSearchV2DiagnosticsTurnSequenceGroupSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsDiscardOrderShadowAudit {
    pub audit_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub candidate_groups: usize,
    pub candidate_states: u64,
    pub static_immediate_safe_groups: usize,
    pub static_immediate_safe_states: u64,
    pub exact_rollout_verified_groups: usize,
    pub proof_pruning_enabled: bool,
    pub reveal_gate: StateAbstractionRevealGate,
    pub sample_limit: usize,
    pub samples: Vec<CombatSearchV2DiagnosticsDiscardOrderShadowAuditSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsDiscardOrderShadowAuditSample {
    pub origin_key: String,
    pub unordered_key_preview: String,
    pub states: u64,
    pub max_prefix_length: usize,
    pub ordered_variants: usize,
    pub effect_variants: usize,
    pub max_legal_actions: usize,
    pub first_divergence_path: Option<&'static str>,
    pub reveal_gate: StateAbstractionRevealGate,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnSequenceDivergenceCount {
    pub kind: StateDivergenceKind,
    pub first_divergence_path: Option<&'static str>,
    pub guessed_reveal_gate: StateAbstractionRevealGate,
    pub groups: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnSequenceGroupSample {
    pub group_class: String,
    pub origin_key: String,
    pub unordered_key_preview: String,
    pub states: u64,
    pub max_prefix_length: usize,
    pub ordered_variants: usize,
    pub effect_variants: usize,
    pub max_legal_actions: usize,
    pub divergence_kind: StateDivergenceKind,
    pub first_divergence_path: Option<&'static str>,
    pub guessed_reveal_gate: StateAbstractionRevealGate,
    pub ordered_samples: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsCardIdentity {
    pub audit_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub active_cards_observed: u64,
    pub action_payload_cards_observed: u64,
    pub action_payload_placeholder_cards: u64,
    pub states_with_duplicate_active_uuid: u64,
    pub duplicate_active_uuid_observations: u64,
    pub states_with_uuid_card_id_conflict: u64,
    pub uuid_card_id_conflict_observations: u64,
    pub max_duplicate_group_size: usize,
    pub largest_duplicate_groups: Vec<CombatSearchV2DiagnosticsCardIdentitySample>,
    pub largest_conflict_groups: Vec<CombatSearchV2DiagnosticsCardIdentitySample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsCardIdentitySample {
    pub observed_at_state_query: u64,
    pub uuid: u32,
    pub occurrence_count: usize,
    pub distinct_card_labels: Vec<String>,
    pub locations: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnLocalDominance {
    pub pruning_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub parent_states_observed: u64,
    pub enabled_parent_states: u64,
    pub eligible_child_states: u64,
    pub accepted_child_states: u64,
    pub pruned_child_states: u64,
    pub prune_ratio: f64,
    pub max_parent_dominance_buckets: usize,
    pub max_parent_resource_vectors: usize,
    pub max_bucket_width: usize,
    pub largest_parent_samples: Vec<CombatSearchV2DiagnosticsTurnLocalDominanceSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsTurnLocalDominanceSample {
    pub observed_at_parent_state: u64,
    pub parent_turn_count: u32,
    pub legal_actions: usize,
    pub eligible_child_states: usize,
    pub accepted_child_states: usize,
    pub pruned_child_states: usize,
    pub dominance_buckets: usize,
    pub resource_vectors: usize,
    pub max_bucket_width: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsPruning {
    pub transposition_prunes: u64,
    pub dominance_prunes: u64,
    pub turn_local_dominance_prunes: u64,
    pub terminal_wins: u64,
    pub terminal_losses: u64,
    pub unresolved_leaf_count: u64,
    pub max_actions_cut_count: u64,
    pub engine_step_limit_count: u64,
    pub potion_budget_cut_count: u64,
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
