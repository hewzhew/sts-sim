use serde::Serialize;

use super::super::super::state_abstraction::{StateAbstractionRevealGate, StateDivergenceKind};
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
    pub one_step_exact_policy: &'static str,
    pub one_step_exact_stored_group_limit: usize,
    pub one_step_exact_sample_limit_groups: usize,
    pub one_step_exact_sample_limit_actions_per_group: usize,
    pub one_step_exact_checked_groups: usize,
    pub one_step_exact_sample_verified_groups: usize,
    pub one_step_exact_blocked_groups: usize,
    pub one_step_exact_checked_actions: usize,
    pub one_step_exact_verified_actions: usize,
    pub one_step_exact_blocked_actions: usize,
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
    pub one_step_exact_status: &'static str,
    pub one_step_exact_checked_actions: usize,
    pub one_step_exact_verified_actions: usize,
    pub one_step_exact_blocked_actions: usize,
    pub one_step_exact_blocking_action_key: Option<String>,
    pub one_step_exact_blocking_divergence_kind: Option<StateDivergenceKind>,
    pub one_step_exact_blocking_path: Option<&'static str>,
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
