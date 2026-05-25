use std::collections::BTreeMap;

use super::super::action_effects::PlayCardEffectDiagnostics;
use super::super::action_priority::ActionOrderingRole;
use super::super::{
    CombatSearchV2DiagnosticsActionEffectSample, CombatSearchV2DiagnosticsActionRoleCount,
    CombatSearchV2DiagnosticsOrdering, CombatSearchV2DiagnosticsOrderingSample,
};
use super::ActionOrderingSummary;

const LARGEST_REORDER_SAMPLE_LIMIT: usize = 8;
pub(super) const ACTION_EFFECT_SAMPLE_LIMIT: usize = 12;

#[derive(Default)]
pub(in crate::ai::combat_search_v2) struct ActionOrderingDiagnosticsCollector {
    states_observed: u64,
    states_reordered: u64,
    total_actions_observed: u64,
    total_position_shift: u64,
    max_position_shift: usize,
    role_counts: BTreeMap<ActionOrderingRole, MutableOrderingRoleCount>,
    largest_reorders: Vec<ActionOrderingObservation>,
    action_effect_actions: u64,
    phase_action_hint_actions: u64,
    action_effect_samples: Vec<ActionOrderingActionEffectObservation>,
}

#[derive(Clone, Debug, Default)]
struct MutableOrderingRoleCount {
    actions: u64,
    first_actions: u64,
}

#[derive(Clone, Debug)]
struct ActionOrderingObservation {
    observed_at_state_query: u64,
    action_count: usize,
    max_position_shift: usize,
    first_role: ActionOrderingRole,
    first_original_action_id: usize,
    first_action_key: String,
}

#[derive(Clone, Debug)]
struct ActionOrderingActionEffectObservation {
    observed_at_state_query: u64,
    original_action_id: usize,
    ordered_index: usize,
    role: ActionOrderingRole,
    action_key: String,
    effects: PlayCardEffectDiagnostics,
}

impl ActionOrderingDiagnosticsCollector {
    pub(in crate::ai::combat_search_v2) fn observe(&mut self, summary: &ActionOrderingSummary) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.total_actions_observed = self
            .total_actions_observed
            .saturating_add(summary.action_count as u64);
        self.total_position_shift = self
            .total_position_shift
            .saturating_add(summary.max_position_shift as u64);
        self.max_position_shift = self.max_position_shift.max(summary.max_position_shift);
        if summary.max_position_shift > 0 {
            self.states_reordered = self.states_reordered.saturating_add(1);
        }
        self.phase_action_hint_actions = self
            .phase_action_hint_actions
            .saturating_add(summary.phase_signal_actions as u64);

        for (role, count) in &summary.role_counts {
            let mutable = self.role_counts.entry(*role).or_default();
            mutable.actions = mutable.actions.saturating_add(*count as u64);
        }
        if let Some(first_role) = summary.first_role {
            self.role_counts
                .entry(first_role)
                .or_default()
                .first_actions += 1;
        }

        if let (Some(first_role), Some(first_original_action_id), Some(first_action_key)) = (
            summary.first_role,
            summary.first_original_action_id,
            summary.first_action_key.as_ref(),
        ) {
            self.remember_largest_reorder(ActionOrderingObservation {
                observed_at_state_query: self.states_observed,
                action_count: summary.action_count,
                max_position_shift: summary.max_position_shift,
                first_role,
                first_original_action_id,
                first_action_key: first_action_key.clone(),
            });
        }
        for sample in &summary.action_effect_samples {
            self.action_effect_actions = self.action_effect_actions.saturating_add(1);
            self.remember_action_effect(ActionOrderingActionEffectObservation {
                observed_at_state_query: self.states_observed,
                original_action_id: sample.original_action_id,
                ordered_index: sample.ordered_index,
                role: sample.role,
                action_key: sample.action_key.clone(),
                effects: sample.effects,
            });
        }
    }

    pub(in crate::ai::combat_search_v2) fn finish(&self) -> CombatSearchV2DiagnosticsOrdering {
        CombatSearchV2DiagnosticsOrdering {
            ordering_policy:
                "semantic_role_ordering_for_player_turn_and_pending_choice_boundaries",
            behavioral_effect: "child_generation_order_only_no_prune_no_merge",
            states_observed: self.states_observed,
            states_reordered: self.states_reordered,
            reordered_state_ratio: rounded_ratio(self.states_reordered, self.states_observed),
            total_actions_observed: self.total_actions_observed,
            action_effect_actions: self.action_effect_actions,
            phase_action_hint_actions: self.phase_action_hint_actions,
            max_position_shift: self.max_position_shift,
            avg_position_shift: rounded_ratio(self.total_position_shift, self.states_observed),
            action_role_counts: self.action_role_counts(),
            largest_reorders: self.largest_reorder_samples(),
            action_effect_samples: self.action_effect_samples(),
            notes: vec![
                "ordering diagnostics summarize which semantic roles are explored first",
                "original action ids are preserved in action traces after ordering",
                "a reorder sample is kept only when action order changed",
                "ordering does not remove legal actions or prove action equivalence",
                "reactive power risk is derived from simulator power hooks, not monster-name policy",
                "enemy phase transition hints only reorder children and never suppress phase-triggering actions",
                "phase action hints reuse phase_profile and only add ordering tiebreaks",
                "pending choice ordering uses typed selection facts and never drops alternatives",
            ],
        }
    }

    fn remember_largest_reorder(&mut self, observation: ActionOrderingObservation) {
        if observation.max_position_shift == 0 {
            return;
        }
        self.largest_reorders.push(observation);
        self.largest_reorders.sort_by(|left, right| {
            right
                .max_position_shift
                .cmp(&left.max_position_shift)
                .then_with(|| right.action_count.cmp(&left.action_count))
                .then_with(|| {
                    left.observed_at_state_query
                        .cmp(&right.observed_at_state_query)
                })
        });
        self.largest_reorders.truncate(LARGEST_REORDER_SAMPLE_LIMIT);
    }

    fn remember_action_effect(&mut self, observation: ActionOrderingActionEffectObservation) {
        self.action_effect_samples.push(observation);
        self.action_effect_samples.sort_by(|left, right| {
            right
                .effects
                .reactive_risk_score
                .cmp(&left.effects.reactive_risk_score)
                .then_with(|| {
                    right
                        .effects
                        .mitigation_score
                        .cmp(&left.effects.mitigation_score)
                })
                .then_with(|| {
                    right
                        .effects
                        .reactive_enemy_damage
                        .cmp(&left.effects.reactive_enemy_damage)
                })
                .then_with(|| {
                    left.observed_at_state_query
                        .cmp(&right.observed_at_state_query)
                })
                .then_with(|| left.ordered_index.cmp(&right.ordered_index))
        });
        self.action_effect_samples
            .truncate(ACTION_EFFECT_SAMPLE_LIMIT);
    }

    fn action_role_counts(&self) -> Vec<CombatSearchV2DiagnosticsActionRoleCount> {
        self.role_counts
            .iter()
            .map(|(role, count)| CombatSearchV2DiagnosticsActionRoleCount {
                role: role.label().to_string(),
                actions: count.actions,
                first_actions: count.first_actions,
            })
            .collect()
    }

    fn largest_reorder_samples(&self) -> Vec<CombatSearchV2DiagnosticsOrderingSample> {
        self.largest_reorders
            .iter()
            .map(|sample| CombatSearchV2DiagnosticsOrderingSample {
                observed_at_state_query: sample.observed_at_state_query,
                action_count: sample.action_count,
                max_position_shift: sample.max_position_shift,
                first_role: sample.first_role.label().to_string(),
                first_original_action_id: sample.first_original_action_id,
                first_action_key: sample.first_action_key.clone(),
            })
            .collect()
    }

    fn action_effect_samples(&self) -> Vec<CombatSearchV2DiagnosticsActionEffectSample> {
        self.action_effect_samples
            .iter()
            .map(|sample| CombatSearchV2DiagnosticsActionEffectSample {
                observed_at_state_query: sample.observed_at_state_query,
                original_action_id: sample.original_action_id,
                ordered_index: sample.ordered_index,
                role: sample.role.label().to_string(),
                action_key: sample.action_key.clone(),
                mitigation_score: sample.effects.mitigation_score,
                reactive_risk_score: sample.effects.reactive_risk_score,
                enemy_strength_gain: sample.effects.enemy_strength_gain,
                visible_attack_pressure_hint: sample.effects.visible_attack_pressure_hint,
                reactive_player_hp_loss: sample.effects.reactive_player_hp_loss,
                reactive_player_block: sample.effects.reactive_player_block,
                reactive_enemy_damage: sample.effects.reactive_enemy_damage,
                reactive_bad_draw_cards: sample.effects.reactive_bad_draw_cards,
                reactive_forced_turn_end: sample.effects.reactive_forced_turn_end,
            })
            .collect()
    }
}

fn rounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let value = numerator as f64 / denominator as f64;
    (value * 100.0).round() / 100.0
}
