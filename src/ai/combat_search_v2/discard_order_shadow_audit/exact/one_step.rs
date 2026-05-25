use std::collections::{BTreeMap, BTreeSet};

use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};
use crate::sim::combat_action::CombatActionChoice;

use super::super::super::frontier::SearchNode;
use super::super::super::state_abstraction::StateDivergenceKind;
use super::super::super::transition::filtered_legal_actions;
use super::super::super::turn_branching::classify_turn_branch_transition;
use super::super::super::turn_sequence_effect::{
    effect_fingerprint, effect_key, TurnSequenceDivergence, TurnSequenceEffectAggregate,
    TurnSequenceEffectFingerprint,
};
use super::super::super::types::{CombatSearchV2ActionTrace, CombatSearchV2Config};
use super::super::is_static_discard_order_candidate;
use super::types::{
    DiscardOrderShadowAuditExactGroupResult, DiscardOrderShadowAuditGroup,
    EXACT_SHADOW_ACTIONS_PER_GROUP,
};

pub(super) fn audit_group_one_step(
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
) -> BTreeMap<String, CombatActionChoice> {
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
    choice: &CombatActionChoice,
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
