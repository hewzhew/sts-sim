use crate::diff::replay::tick_until_stable;
use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::legal_moves::get_legal_moves;
use super::ordering::end_turn_last;
use super::terminal::{survives, terminal_kind};
use super::types::CombatCandidate;
use super::value::{
    compare_values, diagnostic_score, projected_frontier, projected_unblocked, total_enemy_hp,
    CombatValue,
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
}

#[derive(Clone)]
struct FrontierOutcome {
    frontier_engine: EngineState,
    frontier_combat: CombatState,
    frontier_value: CombatValue,
    tail: Vec<ClientInput>,
    explored_nodes: u32,
}

pub(super) fn plan_candidate(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
    branch_width: usize,
    max_engine_steps: usize,
) -> CombatCandidate {
    let (next_engine, next_combat) = simulate_input(engine, combat, input);
    let outcome = continue_local_plan(
        &next_engine,
        &next_combat,
        LOCAL_PLAN_DEPTH.saturating_sub(1),
        branch_width,
        max_engine_steps,
    );

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

fn continue_local_plan(
    engine: &EngineState,
    combat: &CombatState,
    remaining_actions: usize,
    branch_width: usize,
    max_engine_steps: usize,
) -> FrontierOutcome {
    let (frontier_engine, frontier_combat, frontier_value) =
        projected_frontier(engine, combat, max_engine_steps);
    let mut best = FrontierOutcome {
        frontier_engine,
        frontier_combat,
        frontier_value,
        tail: Vec::new(),
        explored_nodes: 1,
    };

    if remaining_actions == 0 {
        return best;
    }

    let mut beam = vec![PlanNode {
        engine: engine.clone(),
        combat: combat.clone(),
        tail: Vec::new(),
        frontier_engine: best.frontier_engine.clone(),
        frontier_combat: best.frontier_combat.clone(),
        frontier_value: best.frontier_value,
    }];

    for _ in 0..remaining_actions {
        let mut next_beam = Vec::new();

        for node in beam {
            let mut best_for_node = FrontierOutcome {
                frontier_engine: node.frontier_engine.clone(),
                frontier_combat: node.frontier_combat.clone(),
                frontier_value: node.frontier_value,
                tail: node.tail.clone(),
                explored_nodes: 1,
            };
            promote_outcome(&mut best, &best_for_node);

            if !can_continue_same_turn(&node.engine, &node.combat) {
                continue;
            }

            let mut children = continuation_children(&node, branch_width, max_engine_steps);
            for child in &children {
                let child_outcome = FrontierOutcome {
                    frontier_engine: child.frontier_engine.clone(),
                    frontier_combat: child.frontier_combat.clone(),
                    frontier_value: child.frontier_value,
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
        .map(|input| {
            let (next_engine, next_combat) = simulate_input(&node.engine, &node.combat, &input);
            let (frontier_engine, frontier_combat, frontier_value) =
                projected_frontier(&next_engine, &next_combat, max_engine_steps);
            let mut tail = node.tail.clone();
            tail.push(input);
            PlanNode {
                engine: next_engine,
                combat: next_combat,
                tail,
                frontier_engine,
                frontier_combat,
                frontier_value,
            }
        })
        .collect::<Vec<_>>();
    children.sort_by(compare_plan_nodes);
    children.truncate(branch_width.max(1));
    children
}

fn simulate_input(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> (EngineState, CombatState) {
    let mut next_engine = engine.clone();
    let mut next_combat = combat.clone();
    let _ = tick_until_stable(&mut next_engine, &mut next_combat, input.clone());
    (next_engine, next_combat)
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
    if compare_values(&best.frontier_value, &candidate.frontier_value).is_gt() {
        *best = candidate.clone();
    } else {
        best.explored_nodes = best.explored_nodes.max(candidate.explored_nodes);
    }
}

fn compare_plan_nodes(left: &PlanNode, right: &PlanNode) -> std::cmp::Ordering {
    compare_values(&left.frontier_value, &right.frontier_value).then_with(|| {
        let left_last = left.tail.last().cloned().unwrap_or(ClientInput::EndTurn);
        let right_last = right.tail.last().cloned().unwrap_or(ClientInput::EndTurn);
        end_turn_last(&left_last, &right_last)
    })
}
