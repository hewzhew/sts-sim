mod partial_frontier;
mod plan;
mod ranking;
mod selection;
mod work;

use std::collections::HashSet;
use std::time::Instant;

use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use super::super::*;
use super::types::{
    TurnPlanEnumeration, TurnPlanStepStateV1, TurnPlanStopReason, TurnPlannerConfigV1,
};
use partial_frontier::select_partial_frontier;
use plan::{plan_from_node, root_eval, stop_reason_for_transition};
use ranking::count_turn_plan_prior_scored_plans;
use selection::{bucket_counts, first_action_summaries, select_bucketed_plans};
use work::TurnPlanWorkNode;

pub(in crate::ai::combat_search_v2) fn enumerate_turn_plans(
    root: &SearchNode,
    stepper: &impl CombatStepper,
    config: &TurnPlannerConfigV1,
    deadline: Option<Instant>,
) -> TurnPlanEnumeration {
    enumerate_turn_plans_impl(root, stepper, config, deadline, false)
}

pub(in crate::ai::combat_search_v2) fn enumerate_turn_plans_across_pending_choices(
    root: &SearchNode,
    stepper: &impl CombatStepper,
    config: &TurnPlannerConfigV1,
    deadline: Option<Instant>,
) -> TurnPlanEnumeration {
    enumerate_turn_plans_impl(root, stepper, config, deadline, true)
}

fn enumerate_turn_plans_impl(
    root: &SearchNode,
    stepper: &impl CombatStepper,
    config: &TurnPlannerConfigV1,
    deadline: Option<Instant>,
    continue_pending_choices: bool,
) -> TurnPlanEnumeration {
    let mut enumeration = TurnPlanEnumeration::default();
    if !matches!(root.engine, EngineState::CombatPlayerTurn) {
        return enumeration;
    }

    let root_action_len = root.actions.len();
    let root_eval = root_eval(root);
    let mut seen = HashSet::new();
    seen.insert(combat_exact_state_key(&root.engine, &root.combat));

    let mut frontier = vec![TurnPlanWorkNode {
        node: root.clone(),
        action_facts: Vec::new(),
        step_states: Vec::new(),
    }];
    let mut candidates = Vec::new();
    while !frontier.is_empty() && enumeration.nodes_expanded < config.max_inner_nodes {
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            break;
        }
        let mut next = Vec::new();
        for work in std::mem::take(&mut frontier) {
            if enumeration.nodes_expanded >= config.max_inner_nodes
                || deadline.is_some_and(|limit| Instant::now() >= limit)
            {
                break;
            }
            let TurnPlanWorkNode {
                node,
                action_facts,
                step_states,
            } = work;

            if terminal_label(&node.engine, &node.combat) != SearchTerminalLabel::Unresolved {
                candidates.push(plan_from_node(
                    TurnPlanWorkNode {
                        node,
                        action_facts,
                        step_states,
                    },
                    root_action_len,
                    TurnPlanStopReason::Terminal,
                    root_eval,
                ));
                continue;
            }

            let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
            let legal = filtered_legal_actions(
                legal_action_choices(
                    stepper,
                    &position,
                    continue_pending_choices,
                    config.max_inner_nodes,
                ),
                config.potion_policy,
                &node.combat,
            );
            if legal.is_empty() {
                candidates.push(plan_from_node(
                    TurnPlanWorkNode {
                        node,
                        action_facts,
                        step_states,
                    },
                    root_action_len,
                    TurnPlanStopReason::NoLegalActions,
                    root_eval,
                ));
                continue;
            }

            enumeration.nodes_expanded = enumeration.nodes_expanded.saturating_add(1);
            let equivalence = compress_equivalent_actions(&node.engine, &node.combat, legal);
            let ordered =
                order_indexed_action_choices(&node.engine, &node.combat, equivalence.choices);
            let before_step_trace = config.capture_step_trace.then(|| {
                (
                    combat_exact_state_hash_v1(&node.engine, &node.combat),
                    summarize_state(&node.engine, &node.combat),
                )
            });
            for ordered_choice in ordered.choices {
                if deadline.is_some_and(|limit| Instant::now() >= limit) {
                    break;
                }

                let step = stepper.apply_to_stable(
                    &position,
                    ordered_choice.choice.input.clone(),
                    CombatStepLimits {
                        max_engine_steps: config.max_engine_steps_per_action,
                        deadline,
                    },
                );
                enumeration.nodes_generated = enumeration.nodes_generated.saturating_add(1);

                let mut child_action_facts = action_facts.clone();
                child_action_facts.push(summarize_action_facts_from_step(
                    &node.combat,
                    &ordered_choice.choice.input,
                    &step,
                ));
                let truncated = step.truncated;
                let CombatPosition {
                    engine: child_engine,
                    combat: child_combat,
                } = step.position;
                let mut child = node.clone_for_child(child_engine, child_combat);
                let mut child_step_states = step_states.clone();
                if let Some((before_exact_state_hash, state_before)) = before_step_trace.as_ref() {
                    child_step_states.push(TurnPlanStepStateV1 {
                        before_exact_state_hash: before_exact_state_hash.clone(),
                        before: state_before.clone(),
                        after_exact_state_hash: combat_exact_state_hash_v1(
                            &child.engine,
                            &child.combat,
                        ),
                        after: summarize_state(&child.engine, &child.combat),
                    });
                }
                let transition = classify_turn_branch_transition(
                    &node.engine,
                    &node.combat,
                    &ordered_choice.choice.input,
                    &child.engine,
                    &child.combat,
                );
                child.note_turn_prefix(&node.combat, &ordered_choice.choice.input, transition);
                child.note_input(&ordered_choice.choice.input);
                child.note_turn_branch_priority(transition.frontier_priority_hint());
                child.push_action(CombatSearchV2ActionTrace {
                    step_index: node.actions.len(),
                    action_id: ordered_choice.original_action_id,
                    action_key: ordered_choice.choice.action_key,
                    action_debug: ordered_choice.choice.action_debug,
                    input: ordered_choice.choice.input,
                });

                if truncated {
                    enumeration.truncated_children =
                        enumeration.truncated_children.saturating_add(1);
                    candidates.push(plan_from_node(
                        TurnPlanWorkNode {
                            node: child,
                            action_facts: child_action_facts,
                            step_states: child_step_states,
                        },
                        root_action_len,
                        TurnPlanStopReason::EngineStepLimit,
                        root_eval,
                    ));
                } else if transition.is_same_turn()
                    || (continue_pending_choices
                        && child.combat.turn.turn_count == root.combat.turn.turn_count
                        && matches!(
                            child.engine,
                            EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
                        ))
                {
                    let key = combat_exact_state_key(&child.engine, &child.combat);
                    if seen.insert(key) {
                        next.push(TurnPlanWorkNode {
                            node: child,
                            action_facts: child_action_facts,
                            step_states: child_step_states,
                        });
                    } else {
                        enumeration.exact_state_skips =
                            enumeration.exact_state_skips.saturating_add(1);
                    }
                } else {
                    candidates.push(plan_from_node(
                        TurnPlanWorkNode {
                            node: child,
                            action_facts: child_action_facts,
                            step_states: child_step_states,
                        },
                        root_action_len,
                        stop_reason_for_transition(transition),
                        root_eval,
                    ));
                }
            }
        }
        frontier = select_partial_frontier(next, config, root_action_len);
    }

    enumeration.preselection_plan_count = candidates.len();
    enumeration.preselection_first_action_summaries = first_action_summaries(&candidates);
    enumeration.preselection_first_actions = enumeration
        .preselection_first_action_summaries
        .iter()
        .map(|summary| summary.action.clone())
        .collect();
    enumeration.preselection_bucket_counts = bucket_counts(&candidates);
    let prior_state_hash = config
        .turn_plan_prior
        .as_ref()
        .filter(|prior| !prior.is_empty())
        .map(|_| combat_exact_state_hash_v1(&root.engine, &root.combat));
    enumeration.turn_plan_prior_scored_plans = count_turn_plan_prior_scored_plans(
        &candidates,
        prior_state_hash.as_deref(),
        config.turn_plan_prior.as_ref(),
    );
    let (selected_plans, selection_audit) =
        select_bucketed_plans(candidates, config, prior_state_hash.as_deref());
    enumeration.plans = selected_plans;
    enumeration.selection_audit = selection_audit;
    enumeration
}

fn legal_action_choices(
    stepper: &impl CombatStepper,
    position: &CombatPosition,
    continue_pending_choices: bool,
    structured_choice_cap: usize,
) -> Vec<crate::sim::combat_action::CombatActionChoice> {
    let mut choices = stepper.atomic_action_choices(position);
    if !continue_pending_choices || !stepper.supports_canonical_pending_choice_actions() {
        return choices;
    }
    let EngineState::PendingChoice(choice) = &position.engine else {
        return choices;
    };
    let Some(inputs) =
        crate::ai::combat_search_v2::pending_choice_action_prefix::canonical_pending_choice_inputs(
            choice,
        )
    else {
        return choices;
    };

    for input in inputs.take(structured_choice_cap.max(1)) {
        if choices.iter().any(|choice| choice.input == input) {
            continue;
        }
        if let Some(choice) = stepper.choice_for_legal_input(position, &input) {
            choices.push(choice);
        }
    }
    choices
}
