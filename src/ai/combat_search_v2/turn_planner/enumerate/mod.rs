mod plan;
mod ranking;
mod selection;
mod work;

use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use super::super::*;
use super::types::{
    TurnPlanEnumeration, TurnPlanStepStateV1, TurnPlanStopReason, TurnPlannerConfigV1,
};
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
    let mut enumeration = TurnPlanEnumeration::default();
    if !matches!(root.engine, EngineState::CombatPlayerTurn) {
        return enumeration;
    }

    let root_action_len = root.actions.len();
    let root_eval = root_eval(root);
    let mut seen = HashSet::new();
    seen.insert(combat_exact_state_key(&root.engine, &root.combat));

    let mut queue = VecDeque::from([TurnPlanWorkNode {
        node: root.clone(),
        action_facts: Vec::new(),
        step_states: Vec::new(),
    }]);
    let mut candidates = Vec::new();
    while let Some(work) = queue.pop_front() {
        if enumeration.nodes_expanded >= config.max_inner_nodes {
            break;
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
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
            stepper.legal_action_choices(&position),
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
        let ordered = order_indexed_action_choices(&node.engine, &node.combat, equivalence.choices);
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

            let before_exact_state_hash = combat_exact_state_hash_v1(&node.engine, &node.combat);
            let state_before = summarize_state(&node.engine, &node.combat);
            let after_exact_state_hash =
                combat_exact_state_hash_v1(&step.position.engine, &step.position.combat);
            let state_after = summarize_state(&step.position.engine, &step.position.combat);
            let mut child =
                node.clone_for_child(step.position.engine.clone(), step.position.combat.clone());
            let mut child_action_facts = action_facts.clone();
            child_action_facts.push(summarize_action_facts_from_step(
                &node.combat,
                &ordered_choice.choice.input,
                &step,
            ));
            let mut child_step_states = step_states.clone();
            child_step_states.push(TurnPlanStepStateV1 {
                before_exact_state_hash,
                before: state_before,
                after_exact_state_hash,
                after: state_after,
            });
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
            child.actions.push(CombatSearchV2ActionTrace {
                step_index: node.actions.len(),
                action_id: ordered_choice.original_action_id,
                action_key: ordered_choice.choice.action_key,
                action_debug: ordered_choice.choice.action_debug,
                input: ordered_choice.choice.input,
            });

            if step.truncated {
                enumeration.truncated_children = enumeration.truncated_children.saturating_add(1);
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
            } else if transition.is_same_turn() {
                let key = combat_exact_state_key(&child.engine, &child.combat);
                if seen.insert(key) {
                    queue.push_back(TurnPlanWorkNode {
                        node: child,
                        action_facts: child_action_facts,
                        step_states: child_step_states,
                    });
                } else {
                    enumeration.exact_state_skips = enumeration.exact_state_skips.saturating_add(1);
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
