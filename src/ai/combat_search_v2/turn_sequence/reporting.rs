use super::super::discard_order_shadow_audit::{
    is_static_discard_order_candidate, summarize_discard_order_shadow_audit,
    DiscardOrderShadowAuditKey, DiscardOrderShadowAuditObservation,
};
use super::super::turn_sequence_effect::TurnSequenceDivergence;
use super::super::*;
use super::types::TurnSequenceGroupAggregate;
use super::TurnSequenceDiagnosticsCollector;
use std::collections::{BTreeMap, BTreeSet};

const LARGEST_SEQUENCE_GROUP_SAMPLE_LIMIT: usize = 8;
const PREVIEW_LIMIT: usize = 180;

impl TurnSequenceDiagnosticsCollector {
    pub(in crate::ai::combat_search_v2) fn run_discard_order_exact_shadow_audit(
        &mut self,
        stepper: &impl CombatStepper,
        config: &CombatSearchV2Config,
    ) {
        let candidate_keys = self.discard_order_shadow_audit_candidate_keys();
        self.discard_order_shadow_audit
            .run_one_step_exact_shadow_audit(stepper, config, &candidate_keys);
    }

    pub(in crate::ai::combat_search_v2) fn finish(&self) -> CombatSearchV2DiagnosticsTurnSequence {
        let mut groups_with_order_variants = 0usize;
        let mut same_effect_order_variant_groups = 0usize;
        let mut order_sensitive_groups = 0usize;
        let mut max_ordered_variants_per_group = 0usize;
        let mut max_effect_variants_per_group = 0usize;
        let mut divergence_counts = BTreeMap::<TurnSequenceDivergence, usize>::new();

        for aggregate in self.groups.values() {
            let ordered = aggregate.ordered_variants.len();
            let effects = aggregate.effect_variants.len();
            max_ordered_variants_per_group = max_ordered_variants_per_group.max(ordered);
            max_effect_variants_per_group = max_effect_variants_per_group.max(effects);
            if ordered > 1 {
                groups_with_order_variants += 1;
                if effects == 1 {
                    same_effect_order_variant_groups += 1;
                } else if effects > 1 {
                    order_sensitive_groups += 1;
                    *divergence_counts
                        .entry(aggregate.effect_components.classify())
                        .or_default() += 1;
                }
            }
        }

        CombatSearchV2DiagnosticsTurnSequence {
            grouping_policy: "same_turn_origin_plus_unordered_prefix_tokens",
            behavioral_effect: "diagnostic_only_no_sequence_prune_no_commutation_claim",
            states_observed: self.states_observed,
            non_empty_prefix_states: self.non_empty_prefix_states,
            grouped_prefix_states: self.grouped_prefix_states,
            unordered_sequence_groups: self.groups.len(),
            groups_with_order_variants,
            same_effect_order_variant_groups,
            order_sensitive_groups,
            max_ordered_variants_per_group,
            max_effect_variants_per_group,
            max_prefix_length: self.max_prefix_length,
            max_legal_actions_after_prefix: self.max_legal_actions_after_prefix,
            order_sensitive_divergence_histogram: divergence_histogram(divergence_counts),
            discard_order_shadow_audit: self.discard_order_shadow_audit(),
            largest_groups: self.largest_group_samples(),
            notes: vec![
                "groups are scoped by the first action's turn-origin dominance hash",
                "unordered prefix tokens intentionally ignore action order for diagnostics only",
                "effect variants use typed diagnostic components plus dominance/resource fallback hashes",
                "same-effect groups are candidates for later simulator-backed commutation probes, not pruning proof",
                "order-sensitive group divergence is classifier guidance, not proof-safe abstraction",
                "large-choice pending decisions are not handled by this diagnostic",
            ],
        }
    }

    fn largest_group_samples(&self) -> Vec<CombatSearchV2DiagnosticsTurnSequenceGroupSample> {
        let mut samples = self
            .groups
            .iter()
            .filter(|(_, aggregate)| aggregate.ordered_variants.len() > 1 || aggregate.states > 1)
            .collect::<Vec<_>>();
        samples.sort_by(|(left_key, left), (right_key, right)| {
            right
                .effect_variants
                .len()
                .cmp(&left.effect_variants.len())
                .then_with(|| {
                    right
                        .ordered_variants
                        .len()
                        .cmp(&left.ordered_variants.len())
                })
                .then_with(|| right.states.cmp(&left.states))
                .then_with(|| left_key.origin_key.cmp(&right_key.origin_key))
                .then_with(|| left_key.unordered_key.cmp(&right_key.unordered_key))
        });
        samples
            .into_iter()
            .take(LARGEST_SEQUENCE_GROUP_SAMPLE_LIMIT)
            .map(|(key, aggregate)| {
                let divergence = aggregate.effect_components.classify();
                CombatSearchV2DiagnosticsTurnSequenceGroupSample {
                    group_class: group_class(aggregate).to_string(),
                    origin_key: key.origin_key.clone(),
                    unordered_key_preview: preview(&key.unordered_key),
                    states: aggregate.states,
                    max_prefix_length: aggregate.max_prefix_length,
                    ordered_variants: aggregate.ordered_variants.len(),
                    effect_variants: aggregate.effect_variants.len(),
                    max_legal_actions: aggregate.max_legal_actions,
                    divergence_kind: divergence.kind,
                    first_divergence_path: divergence.first_divergence_path,
                    guessed_reveal_gate: divergence.guessed_reveal_gate,
                    ordered_samples: aggregate
                        .ordered_variants
                        .iter()
                        .take(3)
                        .map(|ordered| preview(ordered))
                        .collect(),
                }
            })
            .collect()
    }

    fn discard_order_shadow_audit(&self) -> CombatSearchV2DiagnosticsDiscardOrderShadowAudit {
        let observations = self
            .groups
            .iter()
            .filter_map(|(key, aggregate)| {
                if aggregate.ordered_variants.len() <= 1 || aggregate.effect_variants.len() <= 1 {
                    return None;
                }

                let divergence = aggregate.effect_components.classify();
                if !is_static_discard_order_candidate(
                    divergence.kind,
                    divergence.first_divergence_path,
                    divergence.guessed_reveal_gate,
                ) {
                    return None;
                }

                Some(DiscardOrderShadowAuditObservation {
                    origin_key: key.origin_key.clone(),
                    unordered_key_preview: preview(&key.unordered_key),
                    states: aggregate.states,
                    max_prefix_length: aggregate.max_prefix_length,
                    ordered_variants: aggregate.ordered_variants.len(),
                    effect_variants: aggregate.effect_variants.len(),
                    max_legal_actions: aggregate.max_legal_actions,
                    first_divergence_path: divergence.first_divergence_path,
                    reveal_gate: divergence.guessed_reveal_gate,
                })
            })
            .collect();
        summarize_discard_order_shadow_audit(observations, &self.discard_order_shadow_audit)
    }

    fn discard_order_shadow_audit_candidate_keys(&self) -> BTreeSet<DiscardOrderShadowAuditKey> {
        self.groups
            .iter()
            .filter_map(|(key, aggregate)| {
                if aggregate.ordered_variants.len() <= 1 || aggregate.effect_variants.len() <= 1 {
                    return None;
                }

                let divergence = aggregate.effect_components.classify();
                if !is_static_discard_order_candidate(
                    divergence.kind,
                    divergence.first_divergence_path,
                    divergence.guessed_reveal_gate,
                ) {
                    return None;
                }

                Some(DiscardOrderShadowAuditKey {
                    origin_key: key.origin_key.clone(),
                    unordered_key: key.unordered_key.clone(),
                })
            })
            .collect()
    }
}

fn group_class(aggregate: &TurnSequenceGroupAggregate) -> &'static str {
    match (
        aggregate.ordered_variants.len() > 1,
        aggregate.effect_variants.len() > 1,
    ) {
        (true, true) => "order_sensitive_observed",
        (true, false) => "same_effect_order_variants",
        (false, true) => "same_order_effect_variants",
        (false, false) => "single_order_observed",
    }
}

fn preview(value: &str) -> String {
    if value.len() <= PREVIEW_LIMIT {
        value.to_string()
    } else {
        format!("{}...", &value[..PREVIEW_LIMIT])
    }
}

fn divergence_histogram(
    counts: BTreeMap<TurnSequenceDivergence, usize>,
) -> Vec<CombatSearchV2DiagnosticsTurnSequenceDivergenceCount> {
    let mut entries = counts
        .into_iter()
        .map(
            |(divergence, groups)| CombatSearchV2DiagnosticsTurnSequenceDivergenceCount {
                kind: divergence.kind,
                first_divergence_path: divergence.first_divergence_path,
                guessed_reveal_gate: divergence.guessed_reveal_gate,
                groups,
            },
        )
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .groups
            .cmp(&left.groups)
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.first_divergence_path.cmp(&right.first_divergence_path))
    });
    entries
}
