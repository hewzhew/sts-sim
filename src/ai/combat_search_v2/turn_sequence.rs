use super::discard_order_shadow_audit::{
    is_static_discard_order_candidate, summarize_discard_order_shadow_audit,
    DiscardOrderShadowAuditCollector, DiscardOrderShadowAuditKey,
    DiscardOrderShadowAuditObservation,
};
use super::turn_sequence_effect::{
    effect_fingerprint, TurnSequenceDivergence, TurnSequenceEffectAggregate,
    TurnSequenceEffectFingerprint,
};
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const LARGEST_SEQUENCE_GROUP_SAMPLE_LIMIT: usize = 8;
const PREVIEW_LIMIT: usize = 180;

#[derive(Clone, Debug)]
pub(super) struct TurnSequenceSummary {
    prefix_length: usize,
    legal_actions: usize,
    origin_key: Option<String>,
    ordered_key: Option<String>,
    unordered_key: Option<String>,
    effect_key: Option<String>,
    effect_fingerprint: Option<TurnSequenceEffectFingerprint>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TurnSequenceGroupKey {
    origin_key: String,
    unordered_key: String,
}

#[derive(Clone, Debug, Default)]
struct TurnSequenceGroupAggregate {
    states: u64,
    max_prefix_length: usize,
    max_legal_actions: usize,
    ordered_variants: BTreeSet<String>,
    effect_variants: BTreeSet<String>,
    effect_components: TurnSequenceEffectAggregate,
}

#[derive(Default)]
pub(super) struct TurnSequenceDiagnosticsCollector {
    states_observed: u64,
    non_empty_prefix_states: u64,
    grouped_prefix_states: u64,
    max_prefix_length: usize,
    max_legal_actions_after_prefix: usize,
    groups: BTreeMap<TurnSequenceGroupKey, TurnSequenceGroupAggregate>,
    discard_order_shadow_audit: DiscardOrderShadowAuditCollector,
}

pub(super) fn summarize_turn_sequence(
    node: &SearchNode,
    legal_actions: usize,
) -> TurnSequenceSummary {
    let prefix_length = node.turn_prefix.prefix_length();
    if prefix_length == 0 || !matches!(node.engine, EngineState::CombatPlayerTurn) {
        return TurnSequenceSummary {
            prefix_length,
            legal_actions,
            origin_key: None,
            ordered_key: None,
            unordered_key: None,
            effect_key: None,
            effect_fingerprint: None,
        };
    }

    let effect_fingerprint = effect_fingerprint(node, legal_actions);
    TurnSequenceSummary {
        prefix_length,
        legal_actions,
        origin_key: node.turn_prefix.origin_key().map(str::to_string),
        ordered_key: node.turn_prefix.ordered_sequence_key(),
        unordered_key: node.turn_prefix.unordered_sequence_key(),
        effect_key: Some(turn_sequence_effect::effect_key(&effect_fingerprint)),
        effect_fingerprint: Some(effect_fingerprint),
    }
}

impl TurnSequenceDiagnosticsCollector {
    #[cfg(test)]
    pub(super) fn observe(&mut self, summary: &TurnSequenceSummary) {
        self.observe_inner(summary, None);
    }

    pub(super) fn observe_with_node(&mut self, summary: &TurnSequenceSummary, node: &SearchNode) {
        self.observe_inner(summary, Some(node));
    }

    fn observe_inner(&mut self, summary: &TurnSequenceSummary, node: Option<&SearchNode>) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.max_prefix_length = self.max_prefix_length.max(summary.prefix_length);
        if summary.prefix_length == 0 {
            return;
        }

        self.non_empty_prefix_states = self.non_empty_prefix_states.saturating_add(1);
        self.max_legal_actions_after_prefix = self
            .max_legal_actions_after_prefix
            .max(summary.legal_actions);

        let (Some(origin_key), Some(ordered_key), Some(unordered_key), Some(effect_key)) = (
            summary.origin_key.as_ref(),
            summary.ordered_key.as_ref(),
            summary.unordered_key.as_ref(),
            summary.effect_key.as_ref(),
        ) else {
            return;
        };

        self.grouped_prefix_states = self.grouped_prefix_states.saturating_add(1);
        let aggregate = self
            .groups
            .entry(TurnSequenceGroupKey {
                origin_key: origin_key.clone(),
                unordered_key: unordered_key.clone(),
            })
            .or_default();
        aggregate.states = aggregate.states.saturating_add(1);
        aggregate.max_prefix_length = aggregate.max_prefix_length.max(summary.prefix_length);
        aggregate.max_legal_actions = aggregate.max_legal_actions.max(summary.legal_actions);
        aggregate.ordered_variants.insert(ordered_key.clone());
        aggregate.effect_variants.insert(effect_key.clone());
        if let Some(effect_fingerprint) = summary.effect_fingerprint.as_ref() {
            aggregate.effect_components.observe(effect_fingerprint);
            if let Some(node) = node {
                self.discard_order_shadow_audit.observe_state(
                    DiscardOrderShadowAuditKey {
                        origin_key: origin_key.clone(),
                        unordered_key: unordered_key.clone(),
                    },
                    ordered_key,
                    effect_key,
                    effect_fingerprint,
                    node,
                );
            }
        }
    }

    pub(super) fn run_discard_order_exact_shadow_audit(
        &mut self,
        stepper: &impl CombatStepper,
        config: &CombatSearchV2Config,
    ) {
        let candidate_keys = self.discard_order_shadow_audit_candidate_keys();
        self.discard_order_shadow_audit
            .run_one_step_exact_shadow_audit(stepper, config, &candidate_keys);
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsTurnSequence {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn collector_detects_same_effect_order_variants() {
        let mut collector = TurnSequenceDiagnosticsCollector::default();

        collector.observe(&summary(
            "origin",
            "card:Strike_R#1>card:Defend_R#2",
            "card:Defend_R#2>card:Strike_R#1",
            "effect",
        ));
        collector.observe(&summary(
            "origin",
            "card:Defend_R#2>card:Strike_R#1",
            "card:Defend_R#2>card:Strike_R#1",
            "effect",
        ));

        let report = collector.finish();

        assert_eq!(report.states_observed, 2);
        assert_eq!(report.groups_with_order_variants, 1);
        assert_eq!(report.same_effect_order_variant_groups, 1);
        assert_eq!(report.order_sensitive_groups, 0);
        assert_eq!(
            report.largest_groups[0].group_class,
            "same_effect_order_variants"
        );
    }

    #[test]
    fn collector_detects_order_sensitive_groups() {
        let mut collector = TurnSequenceDiagnosticsCollector::default();

        collector.observe(&summary("origin", "A>B", "A>B", "effect_1"));
        collector.observe(&summary("origin", "B>A", "A>B", "effect_2"));

        let report = collector.finish();

        assert_eq!(report.groups_with_order_variants, 1);
        assert_eq!(report.same_effect_order_variant_groups, 0);
        assert_eq!(report.order_sensitive_groups, 1);
        assert_eq!(report.max_effect_variants_per_group, 2);
    }

    #[test]
    fn summarize_turn_sequence_uses_non_empty_combat_prefix() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        let mut node = test_node(combat.clone());
        let transition = TurnBranchTransition::test_same_turn_play_card();
        node.note_turn_prefix(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
            transition,
        );

        let summary = summarize_turn_sequence(&node, 3);

        assert_eq!(summary.prefix_length, 1);
        assert_eq!(summary.legal_actions, 3);
        assert!(summary.origin_key.is_some());
        assert!(summary
            .ordered_key
            .as_deref()
            .is_some_and(|key| key.contains("Strike_R")));
        assert!(summary.unordered_key.is_some());
        assert!(summary.effect_key.is_some());
    }

    fn summary(
        origin_key: &str,
        ordered_key: &str,
        unordered_key: &str,
        effect_key: &str,
    ) -> TurnSequenceSummary {
        TurnSequenceSummary {
            prefix_length: 2,
            legal_actions: 5,
            origin_key: Some(origin_key.to_string()),
            ordered_key: Some(ordered_key.to_string()),
            unordered_key: Some(unordered_key.to_string()),
            effect_key: Some(effect_key.to_string()),
            effect_fingerprint: None,
        }
    }

    fn test_node(combat: CombatState) -> SearchNode {
        SearchNode {
            engine: EngineState::CombatPlayerTurn,
            combat,
            actions: Vec::new(),
            turn_prefix: TurnPrefixState::default(),
            initial_hp: 80,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 0,
            potion_tactical_priority: 0,
            last_turn_branch_priority: 0,
            rollout_estimate: RolloutNodeEstimate::unevaluated(),
        }
    }
}
