use std::collections::{BTreeMap, HashSet, VecDeque};
use std::time::Instant;

use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use super::super::rollout_pending_choice::RolloutPendingChoiceProgress;
use super::super::value::{combat_eval_from_rollout_estimate, CombatEvalV2};
use super::super::*;
use super::types::{
    TurnPlanBucket, TurnPlanEnumeration, TurnPlanStopReason, TurnPlanV1, TurnPlannerConfigV1,
};

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

    let mut queue = VecDeque::from([root.clone()]);
    let mut candidates = Vec::new();
    while let Some(node) = queue.pop_front() {
        if enumeration.nodes_expanded >= config.max_inner_nodes {
            break;
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            break;
        }

        if terminal_label(&node.engine, &node.combat) != SearchTerminalLabel::Unresolved {
            candidates.push(plan_from_node(
                node,
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
                node,
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

            let mut child =
                node.clone_for_child(step.position.engine.clone(), step.position.combat.clone());
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
                    child,
                    root_action_len,
                    TurnPlanStopReason::EngineStepLimit,
                    root_eval,
                ));
            } else if transition.is_same_turn() {
                let key = combat_exact_state_key(&child.engine, &child.combat);
                if seen.insert(key) {
                    queue.push_back(child);
                } else {
                    enumeration.exact_state_skips = enumeration.exact_state_skips.saturating_add(1);
                }
            } else {
                candidates.push(plan_from_node(
                    child,
                    root_action_len,
                    stop_reason_for_transition(transition),
                    root_eval,
                ));
            }
        }
    }

    enumeration.plans = select_bucketed_plans(candidates, config);
    enumeration
}

fn plan_from_node(
    mut node: SearchNode,
    root_action_len: usize,
    stop_reason: TurnPlanStopReason,
    root_eval: CombatEvalV2,
) -> TurnPlanV1 {
    let pending_choice_progress = pending_choice_progress_for_plan(&node, stop_reason);
    let estimate = RolloutNodeEstimate::from_node(
        &node,
        node.actions.len().saturating_sub(root_action_len),
        rollout_stop_reason_for_turn_plan(stop_reason),
        None,
        pending_choice_progress,
    );
    let eval = combat_eval_from_rollout_estimate(estimate);
    node.rollout_estimate = estimate;
    TurnPlanV1 {
        actions: node.actions[root_action_len..].to_vec(),
        end_node: node,
        stop_reason,
        bucket: TurnPlanBucket::from_root_and_eval(root_eval, eval, stop_reason),
        eval,
    }
}

fn root_eval(root: &SearchNode) -> CombatEvalV2 {
    let estimate = RolloutNodeEstimate::from_node(
        root,
        0,
        RolloutStopReason::MaxActions,
        None,
        RolloutPendingChoiceProgress::default(),
    );
    combat_eval_from_rollout_estimate(estimate)
}

fn stop_reason_for_transition(transition: TurnBranchTransition) -> TurnPlanStopReason {
    if transition.is_terminal() {
        TurnPlanStopReason::Terminal
    } else if transition.is_next_turn() {
        TurnPlanStopReason::NextTurn
    } else if transition.is_pending_choice() {
        TurnPlanStopReason::PendingChoice
    } else {
        TurnPlanStopReason::OtherBoundary
    }
}

fn pending_choice_progress_for_plan(
    node: &SearchNode,
    stop_reason: TurnPlanStopReason,
) -> RolloutPendingChoiceProgress {
    let mut progress = RolloutPendingChoiceProgress::default();
    if stop_reason == TurnPlanStopReason::PendingChoice {
        progress.observe_boundary(
            combat_search_phase_profile(&node.engine, &node.combat).pending_choice,
        );
    }
    progress
}

fn rollout_stop_reason_for_turn_plan(stop_reason: TurnPlanStopReason) -> RolloutStopReason {
    match stop_reason {
        TurnPlanStopReason::Terminal => RolloutStopReason::TerminalState,
        TurnPlanStopReason::NextTurn => RolloutStopReason::MaxActions,
        TurnPlanStopReason::PendingChoice => RolloutStopReason::PolicyDeclined,
        TurnPlanStopReason::OtherBoundary => RolloutStopReason::PolicyDeclined,
        TurnPlanStopReason::NoLegalActions => RolloutStopReason::NoLegalActions,
        TurnPlanStopReason::EngineStepLimit => RolloutStopReason::EngineStepLimit,
    }
}

fn select_bucketed_plans(
    mut candidates: Vec<TurnPlanV1>,
    config: &TurnPlannerConfigV1,
) -> Vec<TurnPlanV1> {
    if config.max_end_states == 0 || config.per_bucket_limit == 0 {
        return Vec::new();
    }

    candidates.sort_by(|left, right| right.eval.cmp(&left.eval));
    let mut selected = Vec::new();
    let mut selected_indexes = vec![false; candidates.len()];
    let mut bucket_counts = BTreeMap::<TurnPlanBucket, usize>::new();

    for bucket in TURN_PLAN_BUCKET_DIVERSITY_ORDER {
        if selected.len() >= config.max_end_states {
            break;
        }
        if let Some((index, candidate)) = candidates
            .iter()
            .enumerate()
            .find(|(index, candidate)| !selected_indexes[*index] && candidate.bucket == bucket)
        {
            bucket_counts.insert(candidate.bucket, 1);
            selected_indexes[index] = true;
            selected.push(candidate.clone());
        }
    }

    for (index, candidate) in candidates.into_iter().enumerate() {
        if selected.len() >= config.max_end_states {
            break;
        }
        if !selected_indexes[index] {
            let count = bucket_counts.entry(candidate.bucket).or_default();
            if *count >= config.per_bucket_limit {
                continue;
            }
            *count = count.saturating_add(1);
            selected.push(candidate);
        }
    }

    selected
}

const TURN_PLAN_BUCKET_DIVERSITY_ORDER: [TurnPlanBucket; 7] = [
    TurnPlanBucket::TerminalWin,
    TurnPlanBucket::Progress,
    TurnPlanBucket::Survival,
    TurnPlanBucket::Setup,
    TurnPlanBucket::Balanced,
    TurnPlanBucket::Boundary,
    TurnPlanBucket::TerminalLoss,
];
