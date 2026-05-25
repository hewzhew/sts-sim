use super::*;
use std::collections::HashMap;

const LARGEST_PARENT_SAMPLE_LIMIT: usize = 8;

#[derive(Debug)]
pub(super) struct TurnLocalDominanceStateObservation {
    enabled: bool,
    parent_turn_count: u32,
    legal_actions: usize,
    eligible_child_states: usize,
    accepted_child_states: usize,
    pruned_child_states: usize,
    dominance_buckets: HashMap<CombatDominanceKey, Vec<ResourceVector>>,
    max_bucket_width: usize,
}

#[derive(Default)]
pub(super) struct TurnLocalDominanceDiagnosticsCollector {
    parent_states_observed: u64,
    enabled_parent_states: u64,
    eligible_child_states: u64,
    accepted_child_states: u64,
    pruned_child_states: u64,
    max_parent_dominance_buckets: usize,
    max_parent_resource_vectors: usize,
    max_bucket_width: usize,
    largest_parent_samples: Vec<TurnLocalDominanceParentObservation>,
}

#[derive(Clone, Debug)]
struct TurnLocalDominanceParentObservation {
    observed_at_parent_state: u64,
    parent_turn_count: u32,
    legal_actions: usize,
    eligible_child_states: usize,
    accepted_child_states: usize,
    pruned_child_states: usize,
    dominance_buckets: usize,
    resource_vectors: usize,
    max_bucket_width: usize,
}

impl TurnLocalDominanceStateObservation {
    pub(super) fn new(
        parent_engine: &EngineState,
        parent_combat: &CombatState,
        legal_actions: usize,
    ) -> Self {
        Self {
            enabled: matches!(parent_engine, EngineState::CombatPlayerTurn),
            parent_turn_count: parent_combat.turn.turn_count,
            legal_actions,
            eligible_child_states: 0,
            accepted_child_states: 0,
            pruned_child_states: 0,
            dominance_buckets: HashMap::new(),
            max_bucket_width: 0,
        }
    }

    pub(super) fn observe_child(&mut self, child: &SearchNode) -> bool {
        if !self.enabled || !self.is_same_turn_player_child(child) {
            return false;
        }

        self.eligible_child_states = self.eligible_child_states.saturating_add(1);
        let dominance_key = combat_dominance_key(&child.engine, &child.combat);
        if is_resource_covered(
            &mut self.dominance_buckets,
            dominance_key,
            child.resource_vector(),
        ) {
            self.pruned_child_states = self.pruned_child_states.saturating_add(1);
            true
        } else {
            self.accepted_child_states = self.accepted_child_states.saturating_add(1);
            self.max_bucket_width = self.max_bucket_width.max(
                self.dominance_buckets
                    .values()
                    .map(Vec::len)
                    .max()
                    .unwrap_or_default(),
            );
            false
        }
    }

    fn is_same_turn_player_child(&self, child: &SearchNode) -> bool {
        matches!(child.engine, EngineState::CombatPlayerTurn)
            && child.combat.turn.turn_count == self.parent_turn_count
            && terminal_label(&child.engine, &child.combat) == SearchTerminalLabel::Unresolved
    }

    fn resource_vector_count(&self) -> usize {
        self.dominance_buckets.values().map(Vec::len).sum()
    }
}

impl TurnLocalDominanceDiagnosticsCollector {
    pub(super) fn observe(&mut self, observation: &TurnLocalDominanceStateObservation) {
        self.parent_states_observed = self.parent_states_observed.saturating_add(1);
        if observation.enabled {
            self.enabled_parent_states = self.enabled_parent_states.saturating_add(1);
        }
        self.eligible_child_states = self
            .eligible_child_states
            .saturating_add(observation.eligible_child_states as u64);
        self.accepted_child_states = self
            .accepted_child_states
            .saturating_add(observation.accepted_child_states as u64);
        self.pruned_child_states = self
            .pruned_child_states
            .saturating_add(observation.pruned_child_states as u64);
        self.max_parent_dominance_buckets = self
            .max_parent_dominance_buckets
            .max(observation.dominance_buckets.len());
        self.max_parent_resource_vectors = self
            .max_parent_resource_vectors
            .max(observation.resource_vector_count());
        self.max_bucket_width = self.max_bucket_width.max(observation.max_bucket_width);
        self.remember_largest_parent(observation);
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsTurnLocalDominance {
        CombatSearchV2DiagnosticsTurnLocalDominance {
            pruning_policy: "same_parent_same_turn_dominance_key_resource_coverage",
            behavioral_effect:
                "safe_sibling_child_prune_only_no_cross_parent_no_next_turn_no_terminal_prune",
            parent_states_observed: self.parent_states_observed,
            enabled_parent_states: self.enabled_parent_states,
            eligible_child_states: self.eligible_child_states,
            accepted_child_states: self.accepted_child_states,
            pruned_child_states: self.pruned_child_states,
            prune_ratio: rounded_ratio(self.pruned_child_states, self.eligible_child_states),
            max_parent_dominance_buckets: self.max_parent_dominance_buckets,
            max_parent_resource_vectors: self.max_parent_resource_vectors,
            max_bucket_width: self.max_bucket_width,
            largest_parent_samples: self.largest_parent_samples(),
            notes: vec![
                "v1 only compares children generated from the same expanded parent state",
                "v1 only applies to same-turn CombatPlayerTurn children",
                "next-turn, terminal, pending-choice, and truncated children are not pruned here",
                "coverage uses CombatDominanceKey plus ResourceVector, matching the global dominance boundary",
                "one-pass pruning only removes children covered by an already accepted sibling",
            ],
        }
    }

    fn remember_largest_parent(&mut self, observation: &TurnLocalDominanceStateObservation) {
        if observation.eligible_child_states == 0 {
            return;
        }
        self.largest_parent_samples
            .push(TurnLocalDominanceParentObservation {
                observed_at_parent_state: self.parent_states_observed,
                parent_turn_count: observation.parent_turn_count,
                legal_actions: observation.legal_actions,
                eligible_child_states: observation.eligible_child_states,
                accepted_child_states: observation.accepted_child_states,
                pruned_child_states: observation.pruned_child_states,
                dominance_buckets: observation.dominance_buckets.len(),
                resource_vectors: observation.resource_vector_count(),
                max_bucket_width: observation.max_bucket_width,
            });
        self.largest_parent_samples.sort_by(|left, right| {
            right
                .pruned_child_states
                .cmp(&left.pruned_child_states)
                .then_with(|| right.eligible_child_states.cmp(&left.eligible_child_states))
                .then_with(|| right.legal_actions.cmp(&left.legal_actions))
                .then_with(|| {
                    left.observed_at_parent_state
                        .cmp(&right.observed_at_parent_state)
                })
        });
        self.largest_parent_samples
            .truncate(LARGEST_PARENT_SAMPLE_LIMIT);
    }

    fn largest_parent_samples(&self) -> Vec<CombatSearchV2DiagnosticsTurnLocalDominanceSample> {
        self.largest_parent_samples
            .iter()
            .map(|sample| CombatSearchV2DiagnosticsTurnLocalDominanceSample {
                observed_at_parent_state: sample.observed_at_parent_state,
                parent_turn_count: sample.parent_turn_count,
                legal_actions: sample.legal_actions,
                eligible_child_states: sample.eligible_child_states,
                accepted_child_states: sample.accepted_child_states,
                pruned_child_states: sample.pruned_child_states,
                dominance_buckets: sample.dominance_buckets,
                resource_vectors: sample.resource_vectors,
                max_bucket_width: sample.max_bucket_width,
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

#[cfg(test)]
mod tests;
