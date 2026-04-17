use crate::runtime::combat::CombatState;
use crate::state::EngineState;

use super::legal_moves::get_legal_moves;
use super::ordering::{compare_candidates, end_turn_last};
use super::planner::plan_candidate;
use super::terminal::terminal_outcome;
use super::types::CombatCandidate;
use super::value::{compare_values, projected_frontier, CombatValue};

#[derive(Clone)]
pub(super) struct ExploredCandidate {
    pub(super) candidate: CombatCandidate,
    pub(super) search_value: CombatValue,
    pub(super) explored_nodes: u32,
}

#[derive(Clone, Copy)]
struct SearchOutcome {
    value: CombatValue,
    explored_nodes: u32,
}

pub(super) fn explore_root(
    engine: &EngineState,
    combat: &CombatState,
    max_decision_depth: usize,
    root_width: usize,
    branch_width: usize,
    max_engine_steps: usize,
) -> Vec<ExploredCandidate> {
    let legal_moves = get_legal_moves(engine, combat);
    if legal_moves.is_empty() {
        return Vec::new();
    }

    let mut candidates = legal_moves
        .iter()
        .map(|input| plan_candidate(engine, combat, input, branch_width, max_engine_steps))
        .collect::<Vec<_>>();
    candidates.sort_by(compare_candidates);

    let mut explored = candidates
        .into_iter()
        .take(root_width.max(1))
        .map(|candidate| {
            let outcome = evaluate_state(
                &candidate.frontier_engine,
                &candidate.frontier_combat,
                max_decision_depth.saturating_sub(1),
                branch_width,
                max_engine_steps,
            );
            ExploredCandidate {
                explored_nodes: outcome.explored_nodes + candidate.planner_nodes,
                candidate,
                search_value: outcome.value,
            }
        })
        .collect::<Vec<_>>();

    explored.sort_by(|left, right| {
        compare_values(&left.search_value, &right.search_value)
            .then_with(|| end_turn_last(&left.candidate.input, &right.candidate.input))
    });
    explored
}

fn evaluate_state(
    engine: &EngineState,
    combat: &CombatState,
    depth_left: usize,
    branch_width: usize,
    max_engine_steps: usize,
) -> SearchOutcome {
    if let Some(outcome) = terminal_outcome(engine, combat) {
        return SearchOutcome {
            value: CombatValue::Terminal(outcome),
            explored_nodes: 1,
        };
    }

    if depth_left == 0 {
        return SearchOutcome {
            value: evaluate_projected_value(engine, combat, max_engine_steps),
            explored_nodes: 1,
        };
    }

    let legal_moves = get_legal_moves(engine, combat);
    if legal_moves.is_empty() {
        return SearchOutcome {
            value: evaluate_projected_value(engine, combat, max_engine_steps),
            explored_nodes: 1,
        };
    }

    let mut candidates = legal_moves
        .iter()
        .map(|input| plan_candidate(engine, combat, input, branch_width, max_engine_steps))
        .collect::<Vec<_>>();
    candidates.sort_by(compare_candidates);

    let mut best: Option<SearchOutcome> = None;
    let mut explored_nodes = 0;
    for candidate in candidates.into_iter().take(branch_width.max(1)) {
        let child = evaluate_state(
            &candidate.frontier_engine,
            &candidate.frontier_combat,
            depth_left.saturating_sub(1),
            branch_width,
            max_engine_steps,
        );
        explored_nodes += child.explored_nodes + candidate.planner_nodes;
        match best {
            Some(current) if compare_values(&current.value, &child.value).is_le() => {}
            _ => {
                best = Some(child);
            }
        }
    }

    best.unwrap_or(SearchOutcome {
        value: evaluate_projected_value(engine, combat, max_engine_steps),
        explored_nodes: explored_nodes.max(1),
    })
}

fn evaluate_projected_value(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
) -> CombatValue {
    projected_frontier(engine, combat, max_engine_steps).2
}
