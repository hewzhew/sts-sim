use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;
use std::time::Instant;

use super::equivalence::{reduce_equivalent_inputs, SearchEquivalenceMode};
use super::legal_moves::get_legal_moves;
use super::ordering::{compare_candidates, end_turn_tiebreak};
use super::planner::plan_candidate;
use super::profile::SearchProfileBreakdown;
use super::terminal::terminal_outcome;
use super::types::CombatCandidate;
use super::value::{compare_values, projected_frontier, CombatValue};

#[derive(Clone)]
pub(super) struct ExploredCandidate {
    pub(super) candidate: CombatCandidate,
    pub(super) search_value: CombatValue,
    pub(super) explored_nodes: u32,
}

pub(super) struct RootExploreResult {
    pub(super) explored: Vec<ExploredCandidate>,
    pub(super) timed_out: bool,
}

#[derive(Clone, Copy)]
struct SearchOutcome {
    value: CombatValue,
    explored_nodes: u32,
    timed_out: bool,
}

pub(super) fn explore_root(
    engine: &EngineState,
    combat: &CombatState,
    max_decision_depth: usize,
    root_width: usize,
    branch_width: usize,
    max_engine_steps: usize,
    root_node_budget: usize,
    deadline: Option<Instant>,
    equivalence_mode: SearchEquivalenceMode,
    profile: &mut SearchProfileBreakdown,
) -> RootExploreResult {
    let legal_moves = get_legal_moves(engine, combat);
    explore_root_with_inputs(
        engine,
        combat,
        legal_moves,
        max_decision_depth,
        root_width,
        branch_width,
        max_engine_steps,
        root_node_budget,
        deadline,
        equivalence_mode,
        profile,
    )
}

pub(super) fn explore_root_with_inputs(
    engine: &EngineState,
    combat: &CombatState,
    legal_moves: Vec<ClientInput>,
    max_decision_depth: usize,
    root_width: usize,
    branch_width: usize,
    max_engine_steps: usize,
    root_node_budget: usize,
    deadline: Option<Instant>,
    equivalence_mode: SearchEquivalenceMode,
    profile: &mut SearchProfileBreakdown,
) -> RootExploreResult {
    if legal_moves.is_empty() {
        return RootExploreResult {
            explored: Vec::new(),
            timed_out: false,
        };
    }

    let clusters = reduce_equivalent_inputs(combat, legal_moves, equivalence_mode);
    let mut candidates = clusters
        .iter()
        .map(|cluster| {
            let mut candidate = {
                plan_candidate(
                    engine,
                    combat,
                    &cluster.representative,
                    branch_width,
                    max_engine_steps,
                    deadline,
                    profile,
                )
            };
            candidate.cluster_size = cluster.collapsed_inputs.len() + 1;
            candidate.collapsed_inputs = cluster.collapsed_inputs.clone();
            candidate
        })
        .collect::<Vec<_>>();
    candidates.sort_by(compare_candidates);

    let mut timed_out = false;
    let mut explored = Vec::new();
    let mut consumed_nodes = 0usize;
    for candidate in candidates.into_iter().take(root_width.max(1)) {
        if consumed_nodes >= root_node_budget && !explored.is_empty() {
            timed_out = true;
            profile.note_timeout_source("root_node_budget");
            break;
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) && !explored.is_empty() {
            timed_out = true;
            profile.note_timeout_source("wall_clock_deadline");
            break;
        }
        let outcome = evaluate_state(
            &candidate.frontier_engine,
            &candidate.frontier_combat,
            max_decision_depth.saturating_sub(1),
            branch_width,
            max_engine_steps,
            root_node_budget.saturating_sub(consumed_nodes),
            deadline,
            equivalence_mode,
            profile,
        );
        timed_out |= outcome.timed_out;
        consumed_nodes = consumed_nodes
            .saturating_add(candidate.planner_nodes as usize)
            .saturating_add(outcome.explored_nodes as usize);
        explored.push(ExploredCandidate {
            explored_nodes: outcome.explored_nodes + candidate.planner_nodes,
            candidate,
            search_value: outcome.value,
        });
    }

    explored.sort_by(|left, right| {
        compare_values(&left.search_value, &right.search_value).then_with(|| {
            end_turn_tiebreak(
                &left.candidate.input,
                &right.candidate.input,
                &left.search_value,
            )
        })
    });
    RootExploreResult {
        explored,
        timed_out,
    }
}

fn evaluate_state(
    engine: &EngineState,
    combat: &CombatState,
    depth_left: usize,
    branch_width: usize,
    max_engine_steps: usize,
    node_budget: usize,
    deadline: Option<Instant>,
    equivalence_mode: SearchEquivalenceMode,
    profile: &mut SearchProfileBreakdown,
) -> SearchOutcome {
    if deadline.is_some_and(|limit| Instant::now() >= limit) {
        profile.note_timeout_source("wall_clock_deadline");
        return SearchOutcome {
            value: evaluate_projected_value(engine, combat, max_engine_steps, deadline, profile),
            explored_nodes: 1,
            timed_out: true,
        };
    }

    if let Some(outcome) = terminal_outcome(engine, combat) {
        return SearchOutcome {
            value: CombatValue::Terminal(outcome),
            explored_nodes: 1,
            timed_out: false,
        };
    }

    if depth_left == 0 {
        return SearchOutcome {
            value: evaluate_projected_value(engine, combat, max_engine_steps, deadline, profile),
            explored_nodes: 1,
            timed_out: false,
        };
    }

    let legal_moves = get_legal_moves(engine, combat);
    if legal_moves.is_empty() {
        return SearchOutcome {
            value: evaluate_projected_value(engine, combat, max_engine_steps, deadline, profile),
            explored_nodes: 1,
            timed_out: false,
        };
    }

    let clusters = reduce_equivalent_inputs(combat, legal_moves, equivalence_mode);
    let mut candidates = clusters
        .iter()
        .map(|cluster| {
            let mut candidate = {
                plan_candidate(
                    engine,
                    combat,
                    &cluster.representative,
                    branch_width,
                    max_engine_steps,
                    deadline,
                    profile,
                )
            };
            candidate.cluster_size = cluster.collapsed_inputs.len() + 1;
            candidate.collapsed_inputs = cluster.collapsed_inputs.clone();
            candidate
        })
        .collect::<Vec<_>>();
    candidates.sort_by(compare_candidates);

    let mut best: Option<SearchOutcome> = None;
    let mut explored_nodes = 0;
    let mut timed_out = false;
    for candidate in candidates.into_iter().take(branch_width.max(1)) {
        if explored_nodes as usize >= node_budget && best.is_some() {
            timed_out = true;
            profile.note_timeout_source("recursive_node_budget");
            break;
        }
        if deadline.is_some_and(|limit| Instant::now() >= limit) && best.is_some() {
            timed_out = true;
            profile.note_timeout_source("wall_clock_deadline");
            break;
        }
        let child = evaluate_state(
            &candidate.frontier_engine,
            &candidate.frontier_combat,
            depth_left.saturating_sub(1),
            branch_width,
            max_engine_steps,
            node_budget.saturating_sub(explored_nodes as usize),
            deadline,
            equivalence_mode,
            profile,
        );
        explored_nodes += child.explored_nodes + candidate.planner_nodes;
        timed_out |= child.timed_out;
        match best {
            Some(current) if compare_values(&current.value, &child.value).is_le() => {}
            _ => {
                best = Some(child);
            }
        }
    }

    best.unwrap_or(SearchOutcome {
        value: evaluate_projected_value(engine, combat, max_engine_steps, deadline, profile),
        explored_nodes: explored_nodes.max(1),
        timed_out,
    })
}

fn evaluate_projected_value(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> CombatValue {
    projected_frontier(engine, combat, max_engine_steps, deadline, profile).2
}
