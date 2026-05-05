use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;
use std::time::Instant;

use super::legal_moves::get_legal_moves;
use super::ordering::end_turn_last;
use super::profile::SearchProfileBreakdown;
use super::stepping::simulate_input_bounded;
use super::terminal::{survives, terminal_kind};
use super::types::CombatCandidate;
use super::value::{
    compare_values, diagnostic_score, frontier_value_at_state, projected_frontier,
    projected_unblocked, total_enemy_hp, CombatValue,
};

const LOCAL_PLAN_DEPTH: usize = 3;
const LOCAL_BEAM_WIDTH: usize = 3;

#[derive(Clone)]
struct PlanNode {
    engine: EngineState,
    combat: CombatState,
    tail: Vec<ClientInput>,
    frontier_engine: EngineState,
    frontier_combat: CombatState,
    frontier_value: CombatValue,
    truncated: bool,
}

#[derive(Clone)]
struct FrontierOutcome {
    frontier_engine: EngineState,
    frontier_combat: CombatState,
    frontier_value: CombatValue,
    truncated: bool,
    tail: Vec<ClientInput>,
    explored_nodes: u32,
}

pub(super) fn plan_candidate(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
    branch_width: usize,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> CombatCandidate {
    let planner_started = Instant::now();
    let (next_engine, next_combat, next_step) =
        simulate_input_bounded(engine, combat, input, max_engine_steps, deadline, profile);
    let outcome = if matches!(input, ClientInput::EndTurn) {
        frontier_state_outcome(
            &next_engine,
            &next_combat,
            next_step.truncated || next_step.timed_out,
        )
    } else if deadline.is_some_and(|limit| Instant::now() >= limit) {
        frontier_state_outcome(
            &next_engine,
            &next_combat,
            next_step.truncated || next_step.timed_out,
        )
    } else {
        continue_local_plan(
            &next_engine,
            &next_combat,
            LOCAL_PLAN_DEPTH.saturating_sub(1),
            branch_width,
            max_engine_steps,
            deadline,
            profile,
        )
    };
    profile.record_planner_call(planner_started.elapsed().as_millis());

    let mut local_plan = Vec::with_capacity(outcome.tail.len() + 1);
    local_plan.push(input.clone());
    local_plan.extend(outcome.tail);

    let diagnostic_score = diagnostic_score(outcome.frontier_value, input);
    CombatCandidate {
        input: input.clone(),
        next_combat,
        frontier_engine: outcome.frontier_engine.clone(),
        frontier_combat: outcome.frontier_combat.clone(),
        local_plan,
        planner_nodes: outcome.explored_nodes.max(1),
        value: outcome.frontier_value,
        projection_truncated: outcome.truncated || next_step.truncated || next_step.timed_out,
        cluster_size: 1,
        collapsed_inputs: Vec::new(),
        projected_hp: outcome.frontier_combat.entities.player.current_hp,
        projected_block: outcome.frontier_combat.entities.player.block,
        projected_enemy_total: total_enemy_hp(&outcome.frontier_combat),
        projected_unblocked: projected_unblocked(&outcome.frontier_combat),
        survives: survives(
            terminal_kind(&outcome.frontier_engine, &outcome.frontier_combat),
            outcome.frontier_combat.entities.player.current_hp,
        ),
        diagnostic_score,
    }
}

fn frontier_state_outcome(
    engine: &EngineState,
    combat: &CombatState,
    truncated: bool,
) -> FrontierOutcome {
    FrontierOutcome {
        frontier_engine: engine.clone(),
        frontier_combat: combat.clone(),
        frontier_value: frontier_value_at_state(engine, combat),
        truncated,
        tail: Vec::new(),
        explored_nodes: 1,
    }
}

fn continue_local_plan(
    engine: &EngineState,
    combat: &CombatState,
    remaining_actions: usize,
    branch_width: usize,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> FrontierOutcome {
    let (frontier_engine, frontier_combat, frontier_value, truncated) =
        projected_frontier(engine, combat, max_engine_steps, deadline, profile);
    let mut best = FrontierOutcome {
        frontier_engine,
        frontier_combat,
        frontier_value,
        truncated,
        tail: Vec::new(),
        explored_nodes: 1,
    };

    if remaining_actions == 0 {
        return best;
    }

    if deadline.is_some_and(|limit| Instant::now() >= limit) {
        return best;
    }

    let mut beam = vec![PlanNode {
        engine: engine.clone(),
        combat: combat.clone(),
        tail: Vec::new(),
        frontier_engine: best.frontier_engine.clone(),
        frontier_combat: best.frontier_combat.clone(),
        frontier_value: best.frontier_value,
        truncated: best.truncated,
    }];

    for _ in 0..remaining_actions {
        let mut next_beam = Vec::new();

        for node in beam {
            if deadline.is_some_and(|limit| Instant::now() >= limit) {
                return best;
            }
            let mut best_for_node = FrontierOutcome {
                frontier_engine: node.frontier_engine.clone(),
                frontier_combat: node.frontier_combat.clone(),
                frontier_value: node.frontier_value,
                truncated: node.truncated,
                tail: node.tail.clone(),
                explored_nodes: 1,
            };
            promote_outcome(&mut best, &best_for_node);

            if !can_continue_same_turn(&node.engine, &node.combat) {
                continue;
            }

            let mut children =
                continuation_children(&node, branch_width, max_engine_steps, deadline, profile);
            for child in &children {
                let child_outcome = FrontierOutcome {
                    frontier_engine: child.frontier_engine.clone(),
                    frontier_combat: child.frontier_combat.clone(),
                    frontier_value: child.frontier_value,
                    truncated: child.truncated,
                    tail: child.tail.clone(),
                    explored_nodes: 1,
                };
                promote_outcome(&mut best_for_node, &child_outcome);
                promote_outcome(&mut best, &child_outcome);
            }
            best.explored_nodes += children.len() as u32;

            children.sort_by(compare_plan_nodes);
            next_beam.extend(children.into_iter().take(branch_width.max(1)));
        }

        if next_beam.is_empty() {
            break;
        }

        next_beam.sort_by(compare_plan_nodes);
        next_beam.truncate(LOCAL_BEAM_WIDTH.max(1));
        beam = next_beam;
    }

    best
}

fn continuation_children(
    node: &PlanNode,
    branch_width: usize,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> Vec<PlanNode> {
    let mut moves = get_legal_moves(&node.engine, &node.combat)
        .into_iter()
        .filter(|input| !matches!(input, ClientInput::EndTurn))
        .collect::<Vec<_>>();
    if moves.is_empty() {
        return Vec::new();
    }

    let mut children = moves
        .drain(..)
        .take_while(|_| !deadline.is_some_and(|limit| Instant::now() >= limit))
        .map(|input| {
            let (next_engine, next_combat, next_step) = simulate_input_bounded(
                &node.engine,
                &node.combat,
                &input,
                max_engine_steps,
                deadline,
                profile,
            );
            let (frontier_engine, frontier_combat, frontier_value, truncated) = projected_frontier(
                &next_engine,
                &next_combat,
                max_engine_steps,
                deadline,
                profile,
            );
            let mut tail = node.tail.clone();
            tail.push(input);
            PlanNode {
                engine: next_engine,
                combat: next_combat,
                tail,
                frontier_engine,
                frontier_combat,
                frontier_value,
                truncated: truncated || next_step.truncated || next_step.timed_out,
            }
        })
        .collect::<Vec<_>>();
    children.sort_by(compare_plan_nodes);
    children.truncate(branch_width.max(1));
    children
}

fn can_continue_same_turn(engine: &EngineState, combat: &CombatState) -> bool {
    matches!(
        engine,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) && get_legal_moves(engine, combat)
        .into_iter()
        .any(|input| !matches!(input, ClientInput::EndTurn))
}

fn promote_outcome(best: &mut FrontierOutcome, candidate: &FrontierOutcome) {
    if best.truncated && !candidate.truncated {
        *best = candidate.clone();
    } else if !best.truncated
        && !candidate.truncated
        && compare_values(&best.frontier_value, &candidate.frontier_value).is_gt()
    {
        *best = candidate.clone();
    } else if best.truncated == candidate.truncated
        && compare_values(&best.frontier_value, &candidate.frontier_value).is_gt()
    {
        *best = candidate.clone();
    } else {
        best.explored_nodes = best.explored_nodes.max(candidate.explored_nodes);
    }
}

fn compare_plan_nodes(left: &PlanNode, right: &PlanNode) -> std::cmp::Ordering {
    match (left.truncated, right.truncated) {
        (false, true) => return std::cmp::Ordering::Less,
        (true, false) => return std::cmp::Ordering::Greater,
        _ => {}
    }
    compare_values(&left.frontier_value, &right.frontier_value).then_with(|| {
        let left_last = left.tail.last().cloned().unwrap_or(ClientInput::EndTurn);
        let right_last = right.tail.last().cloned().unwrap_or(ClientInput::EndTurn);
        end_turn_last(&left_last, &right_last)
    })
}
