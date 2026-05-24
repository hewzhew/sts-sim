use std::collections::{BTreeMap, BTreeSet};

use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use super::frontier::SearchNode;
use super::state_abstraction::{StateAbstractionRevealGate, StateDivergenceKind};
use super::transition::filtered_legal_actions;
use super::turn_branching::classify_turn_branch_transition;
use super::turn_sequence_effect::{
    effect_fingerprint, effect_key, TurnSequenceDivergence, TurnSequenceEffectAggregate,
    TurnSequenceEffectFingerprint,
};
use super::types::{
    CombatSearchV2ActionTrace, CombatSearchV2Config,
    CombatSearchV2DiagnosticsDiscardOrderShadowAudit,
    CombatSearchV2DiagnosticsDiscardOrderShadowAuditSample,
};

const DISCARD_ORDER_SHADOW_AUDIT_SAMPLE_LIMIT: usize = 8;
const EXACT_SHADOW_STORED_GROUP_LIMIT: usize = 1024;
const EXACT_SHADOW_GROUP_SAMPLE_LIMIT: usize = 16;
const EXACT_SHADOW_REPRESENTATIVES_PER_GROUP: usize = 2;
const EXACT_SHADOW_ACTIONS_PER_GROUP: usize = 8;

#[derive(Clone, Debug)]
pub(super) struct DiscardOrderShadowAuditObservation {
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

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct DiscardOrderShadowAuditKey {
    pub origin_key: String,
    pub unordered_key: String,
}

#[derive(Clone)]
struct DiscardOrderShadowAuditRepresentative {
    ordered_key: String,
    effect_key: String,
    effect_fingerprint: TurnSequenceEffectFingerprint,
    node: SearchNode,
}

#[derive(Clone, Default)]
struct DiscardOrderShadowAuditGroup {
    representatives: Vec<DiscardOrderShadowAuditRepresentative>,
}

#[derive(Default)]
pub(super) struct DiscardOrderShadowAuditCollector {
    groups: BTreeMap<DiscardOrderShadowAuditKey, DiscardOrderShadowAuditGroup>,
    exact: DiscardOrderShadowAuditExactSummary,
}

#[derive(Clone, Debug, Default)]
struct DiscardOrderShadowAuditExactSummary {
    checked_groups: usize,
    sample_verified_groups: usize,
    blocked_groups: usize,
    checked_actions: usize,
    verified_actions: usize,
    blocked_actions: usize,
    group_results: BTreeMap<DiscardOrderShadowAuditKey, DiscardOrderShadowAuditExactGroupResult>,
}

#[derive(Clone, Debug)]
struct DiscardOrderShadowAuditExactGroupResult {
    status: &'static str,
    checked_actions: usize,
    verified_actions: usize,
    blocked_actions: usize,
    blocking_action_key: Option<String>,
    blocking_divergence_kind: Option<StateDivergenceKind>,
    blocking_path: Option<&'static str>,
}

pub(super) fn is_static_discard_order_candidate(
    kind: StateDivergenceKind,
    first_divergence_path: Option<&'static str>,
    reveal_gate: StateAbstractionRevealGate,
) -> bool {
    matches!(kind, StateDivergenceKind::DiscardOrderDelta)
        && first_divergence_path == Some("combat.zones.discard_pile")
        && matches!(reveal_gate, StateAbstractionRevealGate::NextShuffle)
}

impl DiscardOrderShadowAuditCollector {
    pub(super) fn observe_state(
        &mut self,
        key: DiscardOrderShadowAuditKey,
        ordered_key: &str,
        effect_key: &str,
        effect_fingerprint: &TurnSequenceEffectFingerprint,
        node: &SearchNode,
    ) {
        if !self.groups.contains_key(&key) && self.groups.len() >= EXACT_SHADOW_STORED_GROUP_LIMIT {
            return;
        }

        let group = self.groups.entry(key).or_default();
        if group
            .representatives
            .iter()
            .any(|representative| representative.ordered_key == ordered_key)
        {
            return;
        }
        if group.representatives.len() >= EXACT_SHADOW_REPRESENTATIVES_PER_GROUP {
            return;
        }

        group
            .representatives
            .push(DiscardOrderShadowAuditRepresentative {
                ordered_key: ordered_key.to_string(),
                effect_key: effect_key.to_string(),
                effect_fingerprint: effect_fingerprint.clone(),
                node: node.clone(),
            });
    }

    pub(super) fn run_one_step_exact_shadow_audit(
        &mut self,
        stepper: &impl CombatStepper,
        config: &CombatSearchV2Config,
        candidate_keys: &BTreeSet<DiscardOrderShadowAuditKey>,
    ) {
        let mut exact = DiscardOrderShadowAuditExactSummary::default();
        for (key, group) in self
            .groups
            .iter()
            .filter(|(key, _)| candidate_keys.contains(*key))
            .take(EXACT_SHADOW_GROUP_SAMPLE_LIMIT)
        {
            let Some(result) = audit_group_one_step(stepper, config, group) else {
                continue;
            };
            exact.checked_groups += 1;
            exact.checked_actions += result.checked_actions;
            exact.verified_actions += result.verified_actions;
            exact.blocked_actions += result.blocked_actions;
            if result.blocked_actions == 0 {
                exact.sample_verified_groups += 1;
            } else {
                exact.blocked_groups += 1;
            }
            exact.group_results.insert(key.clone(), result);
        }
        self.exact = exact;
    }

    fn exact_result(
        &self,
        origin_key: &str,
        unordered_key_preview: &str,
    ) -> Option<&DiscardOrderShadowAuditExactGroupResult> {
        self.exact
            .group_results
            .iter()
            .find(|(key, _)| {
                key.origin_key == origin_key && preview(&key.unordered_key) == unordered_key_preview
            })
            .map(|(_, result)| result)
    }
}

pub(super) fn summarize_discard_order_shadow_audit(
    mut observations: Vec<DiscardOrderShadowAuditObservation>,
    collector: &DiscardOrderShadowAuditCollector,
) -> CombatSearchV2DiagnosticsDiscardOrderShadowAudit {
    observations.sort_by(|left, right| {
        let left_exact = collector
            .exact_result(&left.origin_key, &left.unordered_key_preview)
            .is_some();
        let right_exact = collector
            .exact_result(&right.origin_key, &right.unordered_key_preview)
            .is_some();
        right_exact
            .cmp(&left_exact)
            .then_with(|| right.states.cmp(&left.states))
            .then_with(|| right.ordered_variants.cmp(&left.ordered_variants))
            .then_with(|| right.effect_variants.cmp(&left.effect_variants))
            .then_with(|| left.origin_key.cmp(&right.origin_key))
            .then_with(|| left.unordered_key_preview.cmp(&right.unordered_key_preview))
    });

    let candidate_groups = observations.len();
    let candidate_states = observations.iter().map(|item| item.states).sum();
    let samples = observations
        .into_iter()
        .take(DISCARD_ORDER_SHADOW_AUDIT_SAMPLE_LIMIT)
        .map(|item| {
            let exact = collector.exact_result(&item.origin_key, &item.unordered_key_preview);
            CombatSearchV2DiagnosticsDiscardOrderShadowAuditSample {
                origin_key: item.origin_key,
                unordered_key_preview: item.unordered_key_preview,
                states: item.states,
                max_prefix_length: item.max_prefix_length,
                ordered_variants: item.ordered_variants,
                effect_variants: item.effect_variants,
                max_legal_actions: item.max_legal_actions,
                first_divergence_path: item.first_divergence_path,
                reveal_gate: item.reveal_gate,
                one_step_exact_status: exact.map(|result| result.status).unwrap_or("not_sampled"),
                one_step_exact_checked_actions: exact
                    .map(|result| result.checked_actions)
                    .unwrap_or(0),
                one_step_exact_verified_actions: exact
                    .map(|result| result.verified_actions)
                    .unwrap_or(0),
                one_step_exact_blocked_actions: exact
                    .map(|result| result.blocked_actions)
                    .unwrap_or(0),
                one_step_exact_blocking_action_key: exact
                    .and_then(|result| result.blocking_action_key.clone()),
                one_step_exact_blocking_divergence_kind: exact
                    .and_then(|result| result.blocking_divergence_kind),
                one_step_exact_blocking_path: exact.and_then(|result| result.blocking_path),
            }
        })
        .collect();

    CombatSearchV2DiagnosticsDiscardOrderShadowAudit {
        audit_policy: "static_discard_order_candidate_plus_bounded_one_step_exact_shadow",
        behavioral_effect: "diagnostic_only_no_prune_no_state_merge",
        candidate_groups,
        candidate_states,
        static_immediate_safe_groups: candidate_groups,
        static_immediate_safe_states: candidate_states,
        exact_rollout_verified_groups: 0,
        proof_pruning_enabled: false,
        reveal_gate: StateAbstractionRevealGate::NextShuffle,
        one_step_exact_policy: "sample_representative_pairs_compare_common_actions_one_step",
        one_step_exact_stored_group_limit: EXACT_SHADOW_STORED_GROUP_LIMIT,
        one_step_exact_sample_limit_groups: EXACT_SHADOW_GROUP_SAMPLE_LIMIT,
        one_step_exact_sample_limit_actions_per_group: EXACT_SHADOW_ACTIONS_PER_GROUP,
        one_step_exact_checked_groups: collector.exact.checked_groups,
        one_step_exact_sample_verified_groups: collector.exact.sample_verified_groups,
        one_step_exact_blocked_groups: collector.exact.blocked_groups,
        one_step_exact_checked_actions: collector.exact.checked_actions,
        one_step_exact_verified_actions: collector.exact.verified_actions,
        one_step_exact_blocked_actions: collector.exact.blocked_actions,
        sample_limit: DISCARD_ORDER_SHADOW_AUDIT_SAMPLE_LIMIT,
        samples,
        notes: vec![
            "static audit only identifies candidate groups; exact shadow audit is bounded and sampled",
            "one-step exact audit applies common legal actions from paired exact states and compares the resulting typed effect boundary",
            "sample-verified groups are not proof-safe pruning because the audit is one-step and action-sampled",
            "exact_rollout_verified_groups stays zero until a simulator-backed until-reveal-gate rollout audit is implemented",
        ],
    }
}

fn audit_group_one_step(
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    group: &DiscardOrderShadowAuditGroup,
) -> Option<DiscardOrderShadowAuditExactGroupResult> {
    let [left, right] = group.representatives.as_slice() else {
        return None;
    };

    let initial_divergence = classify_pair(&left.effect_fingerprint, &right.effect_fingerprint);
    if left.effect_key == right.effect_key
        || !is_static_discard_order_candidate(
            initial_divergence.kind,
            initial_divergence.first_divergence_path,
            initial_divergence.guessed_reveal_gate,
        )
    {
        return None;
    }

    let left_actions = legal_action_map(stepper, config, &left.node);
    let right_actions = legal_action_map(stepper, config, &right.node);
    let left_keys = left_actions.keys().cloned().collect::<BTreeSet<_>>();
    let right_keys = right_actions.keys().cloned().collect::<BTreeSet<_>>();
    if left_keys != right_keys {
        return Some(DiscardOrderShadowAuditExactGroupResult {
            status: "blocked_legal_action_set_delta",
            checked_actions: 0,
            verified_actions: 0,
            blocked_actions: 1,
            blocking_action_key: None,
            blocking_divergence_kind: Some(StateDivergenceKind::LegalActionDelta),
            blocking_path: Some("combat.legal_actions.action_key_set"),
        });
    }

    let mut result = DiscardOrderShadowAuditExactGroupResult {
        status: "sample_verified_one_step",
        checked_actions: 0,
        verified_actions: 0,
        blocked_actions: 0,
        blocking_action_key: None,
        blocking_divergence_kind: None,
        blocking_path: None,
    };

    for action_key in left_keys.into_iter().take(EXACT_SHADOW_ACTIONS_PER_GROUP) {
        let left_choice = left_actions
            .get(&action_key)
            .expect("key collected from left action map");
        let right_choice = right_actions
            .get(&action_key)
            .expect("matching key checked before action audit");
        result.checked_actions += 1;

        let left_child = one_step_effect(stepper, config, &left.node, left_choice);
        let right_child = one_step_effect(stepper, config, &right.node, right_choice);
        let action_status = match (left_child, right_child) {
            (Ok(left_fingerprint), Ok(right_fingerprint)) => {
                let divergence = classify_pair(&left_fingerprint, &right_fingerprint);
                if effect_key(&left_fingerprint) == effect_key(&right_fingerprint)
                    || is_static_discard_order_candidate(
                        divergence.kind,
                        divergence.first_divergence_path,
                        divergence.guessed_reveal_gate,
                    )
                {
                    Ok(())
                } else {
                    Err((
                        divergence.kind,
                        divergence.first_divergence_path,
                        "blocked_one_step_divergence",
                    ))
                }
            }
            _ => Err((
                StateDivergenceKind::EngineRuntimeDelta,
                Some("combat.shadow_audit.apply_to_stable"),
                "blocked_engine_step",
            )),
        };

        match action_status {
            Ok(()) => {
                result.verified_actions += 1;
            }
            Err((kind, path, status)) => {
                result.status = status;
                result.blocked_actions += 1;
                if result.blocking_action_key.is_none() {
                    result.blocking_action_key = Some(action_key);
                    result.blocking_divergence_kind = Some(kind);
                    result.blocking_path = path;
                }
            }
        }
    }

    Some(result)
}

fn legal_action_map(
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    node: &SearchNode,
) -> BTreeMap<String, crate::sim::combat_action::CombatActionChoice> {
    let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
    filtered_legal_actions(
        stepper.legal_action_choices(&position),
        config.potion_policy,
        &node.combat,
    )
    .into_iter()
    .map(|choice| (choice.action_key.clone(), choice))
    .collect()
}

fn one_step_effect(
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    node: &SearchNode,
    choice: &crate::sim::combat_action::CombatActionChoice,
) -> Result<TurnSequenceEffectFingerprint, ()> {
    let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
    let step = stepper.apply_to_stable(
        &position,
        choice.input.clone(),
        CombatStepLimits {
            max_engine_steps: config.max_engine_steps_per_action,
            deadline: None,
        },
    );
    if step.truncated || step.timed_out {
        return Err(());
    }

    let mut child = node.clone_for_child(step.position.engine, step.position.combat);
    let transition = classify_turn_branch_transition(
        &node.engine,
        &node.combat,
        &choice.input,
        &child.engine,
        &child.combat,
    );
    child.note_turn_prefix(&node.combat, &choice.input, transition);
    child.note_input(&choice.input);
    child.actions.push(CombatSearchV2ActionTrace {
        step_index: node.actions.len(),
        action_id: 0,
        action_key: choice.action_key.clone(),
        action_debug: choice.action_debug.clone(),
        input: choice.input.clone(),
    });

    let child_position = CombatPosition::new(child.engine.clone(), child.combat.clone());
    let child_legal_count = filtered_legal_actions(
        stepper.legal_action_choices(&child_position),
        config.potion_policy,
        &child.combat,
    )
    .len();
    Ok(effect_fingerprint(&child, child_legal_count))
}

fn classify_pair(
    left: &TurnSequenceEffectFingerprint,
    right: &TurnSequenceEffectFingerprint,
) -> TurnSequenceDivergence {
    let mut aggregate = TurnSequenceEffectAggregate::default();
    aggregate.observe(left);
    aggregate.observe(right);
    aggregate.classify()
}

fn preview(value: &str) -> String {
    const PREVIEW_LIMIT: usize = 180;
    if value.len() <= PREVIEW_LIMIT {
        value.to_string()
    } else {
        format!("{}...", &value[..PREVIEW_LIMIT])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_candidate_requires_discard_order_until_next_shuffle() {
        assert!(is_static_discard_order_candidate(
            StateDivergenceKind::DiscardOrderDelta,
            Some("combat.zones.discard_pile"),
            StateAbstractionRevealGate::NextShuffle,
        ));
        assert!(!is_static_discard_order_candidate(
            StateDivergenceKind::DiscardOrderDelta,
            Some("combat.zones.draw_pile"),
            StateAbstractionRevealGate::NextShuffle,
        ));
        assert!(!is_static_discard_order_candidate(
            StateDivergenceKind::DiscardOrderDelta,
            Some("combat.zones.discard_pile"),
            StateAbstractionRevealGate::NextDraw,
        ));
        assert!(!is_static_discard_order_candidate(
            StateDivergenceKind::ImmediatePublicDelta,
            Some("combat.zones.discard_pile"),
            StateAbstractionRevealGate::NextShuffle,
        ));
    }

    #[test]
    fn summary_reports_static_candidates_without_pruning() {
        let collector = DiscardOrderShadowAuditCollector::default();
        let report = summarize_discard_order_shadow_audit(
            vec![
                DiscardOrderShadowAuditObservation {
                    origin_key: "origin_b".to_string(),
                    unordered_key_preview: "B>A".to_string(),
                    states: 2,
                    max_prefix_length: 2,
                    ordered_variants: 2,
                    effect_variants: 2,
                    max_legal_actions: 4,
                    first_divergence_path: Some("combat.zones.discard_pile"),
                    reveal_gate: StateAbstractionRevealGate::NextShuffle,
                },
                DiscardOrderShadowAuditObservation {
                    origin_key: "origin_a".to_string(),
                    unordered_key_preview: "A>B".to_string(),
                    states: 3,
                    max_prefix_length: 3,
                    ordered_variants: 2,
                    effect_variants: 2,
                    max_legal_actions: 5,
                    first_divergence_path: Some("combat.zones.discard_pile"),
                    reveal_gate: StateAbstractionRevealGate::NextShuffle,
                },
            ],
            &collector,
        );

        assert_eq!(report.candidate_groups, 2);
        assert_eq!(report.candidate_states, 5);
        assert_eq!(report.static_immediate_safe_groups, 2);
        assert_eq!(report.static_immediate_safe_states, 5);
        assert_eq!(report.exact_rollout_verified_groups, 0);
        assert_eq!(report.one_step_exact_checked_groups, 0);
        assert!(!report.proof_pruning_enabled);
        assert_eq!(report.samples[0].origin_key, "origin_a");
        assert_eq!(report.samples[0].one_step_exact_status, "not_sampled");
    }
}
