use std::collections::HashMap;
use std::time::Instant;

use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::dominance::{strictly_dominates, TurnResourceSummary};
use super::equivalence::{reduce_equivalent_inputs, SearchEquivalenceMode};
use super::frontier_eval::{compare_frontier_eval, eval_frontier_state, FrontierEval};
use super::legal_moves::get_legal_moves;
use super::profile::SearchProfileBreakdown;
use super::stepping::{
    pending_choice_is_same_turn_frontier, project_turn_close_state_bounded, simulate_input_bounded,
};
use super::turn_state_key::{
    stable_dominance_bucket_key, turn_state_key, StableOutcomeKey, TurnStateKey,
};

#[derive(Clone, Debug)]
pub struct ExactTurnConfig {
    pub max_nodes: usize,
    pub max_engine_steps: usize,
    pub deadline: Option<Instant>,
    pub root_inputs: Option<Vec<ClientInput>>,
}

impl Default for ExactTurnConfig {
    fn default() -> Self {
        Self {
            max_nodes: 10_000,
            max_engine_steps: 200,
            deadline: None,
            root_inputs: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TurnEndState {
    pub frontier_engine: EngineState,
    pub frontier_combat: CombatState,
    frontier_eval: FrontierEval,
    stable_key: Option<StableOutcomeKey>,
    pub line: Vec<ClientInput>,
    pub resources: TurnResourceSummary,
}

#[derive(Clone, Debug)]
pub struct ExactTurnSolution {
    pub best_first_input: Option<ClientInput>,
    pub best_line: Vec<ClientInput>,
    pub nondominated_end_states: Vec<TurnEndState>,
    pub elapsed_ms: u128,
    pub explored_nodes: u32,
    pub dominance_prunes: u32,
    pub cycle_cuts: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub truncated: bool,
}

#[derive(Clone, Debug)]
enum MemoEntry {
    InProgress,
    Done(Vec<TurnEndState>),
}

struct SearchCtx {
    config: ExactTurnConfig,
    explored_nodes: u32,
    dominance_prunes: u32,
    cycle_cuts: u32,
    cache_hits: u32,
    cache_misses: u32,
    truncated: bool,
    memo: HashMap<TurnStateKey, MemoEntry>,
    profile: SearchProfileBreakdown,
}

pub fn solve_exact_turn(engine: &EngineState, combat: &CombatState) -> ExactTurnSolution {
    solve_exact_turn_with_config(engine, combat, ExactTurnConfig::default())
}

pub fn solve_exact_turn_with_config(
    engine: &EngineState,
    combat: &CombatState,
    config: ExactTurnConfig,
) -> ExactTurnSolution {
    let started = Instant::now();
    let mut ctx = SearchCtx {
        config,
        explored_nodes: 0,
        dominance_prunes: 0,
        cycle_cuts: 0,
        cache_hits: 0,
        cache_misses: 0,
        truncated: false,
        memo: HashMap::new(),
        profile: SearchProfileBreakdown::default(),
    };
    let root_inputs = ctx.config.root_inputs.clone();
    let mut end_states = enumerate_end_states(engine, combat, &mut ctx, root_inputs.as_deref());
    end_states.sort_by(compare_end_states);

    let best_line = end_states
        .first()
        .map(|state| state.line.clone())
        .unwrap_or_default();
    let best_first_input = best_line.first().cloned();

    ExactTurnSolution {
        best_first_input,
        best_line,
        nondominated_end_states: end_states,
        elapsed_ms: started.elapsed().as_millis(),
        explored_nodes: ctx.explored_nodes.max(1),
        dominance_prunes: ctx.dominance_prunes,
        cycle_cuts: ctx.cycle_cuts,
        cache_hits: ctx.cache_hits,
        cache_misses: ctx.cache_misses,
        truncated: ctx.truncated,
    }
}

fn enumerate_end_states(
    engine: &EngineState,
    combat: &CombatState,
    ctx: &mut SearchCtx,
    root_inputs: Option<&[ClientInput]>,
) -> Vec<TurnEndState> {
    if ctx
        .config
        .deadline
        .is_some_and(|deadline| Instant::now() >= deadline)
    {
        ctx.truncated = true;
        let (end_state, _) = build_turn_close_end_state(
            engine,
            combat,
            ctx.config.max_engine_steps,
            ctx.config.deadline,
            root_inputs,
            &mut ctx.profile,
        );
        return vec![end_state];
    }

    if ctx.explored_nodes as usize >= ctx.config.max_nodes {
        ctx.truncated = true;
        let (end_state, _) = build_turn_close_end_state(
            engine,
            combat,
            ctx.config.max_engine_steps,
            ctx.config.deadline,
            root_inputs,
            &mut ctx.profile,
        );
        return vec![end_state];
    }

    let key = turn_state_key(engine, combat);
    if let Some(cached) = ctx.memo.get(&key) {
        ctx.cache_hits = ctx.cache_hits.saturating_add(1);
        return match cached {
            MemoEntry::Done(end_states) => end_states.clone(),
            MemoEntry::InProgress => {
                ctx.truncated = true;
                ctx.cycle_cuts = ctx.cycle_cuts.saturating_add(1);
                let (end_state, _) = build_turn_close_end_state(
                    engine,
                    combat,
                    ctx.config.max_engine_steps,
                    ctx.config.deadline,
                    root_inputs,
                    &mut ctx.profile,
                );
                vec![end_state]
            }
        };
    }
    ctx.cache_misses = ctx.cache_misses.saturating_add(1);

    ctx.memo.insert(key.clone(), MemoEntry::InProgress);
    ctx.explored_nodes += 1;

    let can_continue = can_continue_same_turn(engine, combat, root_inputs);
    let mut end_states = Vec::new();
    if should_emit_leaf_candidate(engine, combat, can_continue, root_inputs) {
        let (base_end_state, base_truncated) = build_turn_close_end_state(
            engine,
            combat,
            ctx.config.max_engine_steps,
            ctx.config.deadline,
            root_inputs,
            &mut ctx.profile,
        );
        ctx.truncated |= base_truncated;
        end_states.push(base_end_state);
    }

    if can_continue {
        for input in legal_non_endturn_moves(engine, combat, root_inputs) {
            if ctx
                .config
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                ctx.truncated = true;
                break;
            }
            let (next_engine, next_combat, step_outcome) = simulate_input_bounded(
                engine,
                combat,
                &input,
                ctx.config.max_engine_steps,
                ctx.config.deadline,
                &mut ctx.profile,
            );
            ctx.truncated |= step_outcome.truncated || step_outcome.timed_out;
            let suffixes = enumerate_end_states(&next_engine, &next_combat, ctx, None);
            for suffix in suffixes {
                insert_nondominated(
                    &mut end_states,
                    prepend_input(&input, combat, &next_combat, suffix),
                    &mut ctx.dominance_prunes,
                );
            }
        }
    }

    end_states.sort_by(compare_end_states);
    ctx.memo.insert(key, MemoEntry::Done(end_states.clone()));
    end_states
}

fn build_turn_close_end_state(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    root_inputs: Option<&[ClientInput]>,
    profile: &mut SearchProfileBreakdown,
) -> (TurnEndState, bool) {
    let (frontier_engine, frontier_combat, outcome) =
        project_turn_close_state_bounded(engine, combat, max_engine_steps, deadline, profile);
    let frontier_eval = eval_frontier_state(&frontier_engine, &frontier_combat);
    let final_hp = frontier_combat.entities.player.current_hp;
    let final_block = frontier_combat.entities.player.block;
    let line = if end_turn_available(engine, combat, root_inputs) {
        vec![ClientInput::EndTurn]
    } else {
        Vec::new()
    };

    (
        TurnEndState {
            stable_key: stable_dominance_bucket_key(&frontier_engine, &frontier_combat),
            frontier_engine,
            frontier_combat,
            frontier_eval,
            line,
            resources: TurnResourceSummary::at_frontier(final_hp, final_block),
        },
        outcome.truncated || outcome.timed_out,
    )
}

fn prepend_input(
    input: &ClientInput,
    before: &CombatState,
    after: &CombatState,
    mut suffix: TurnEndState,
) -> TurnEndState {
    suffix.line.insert(0, input.clone());
    suffix.resources = suffix.resources.with_transition(
        input,
        before.entities.player.current_hp,
        after.entities.player.current_hp,
        after
            .zones
            .exhaust_pile
            .len()
            .saturating_sub(before.zones.exhaust_pile.len()),
    );
    suffix
}

fn insert_nondominated(
    end_states: &mut Vec<TurnEndState>,
    candidate: TurnEndState,
    dominance_prunes: &mut u32,
) {
    let mut dominated = false;
    let mut to_remove = Vec::new();

    for (idx, existing) in end_states.iter().enumerate() {
        let (Some(existing_key), Some(candidate_key)) =
            (existing.stable_key.as_ref(), candidate.stable_key.as_ref())
        else {
            continue;
        };

        if existing_key != candidate_key {
            continue;
        }

        if strictly_dominates(&existing.resources, &candidate.resources) {
            dominated = true;
            *dominance_prunes = dominance_prunes.saturating_add(1);
            break;
        }

        if strictly_dominates(&candidate.resources, &existing.resources) {
            to_remove.push(idx);
            continue;
        }

        if existing.resources == candidate.resources && candidate.line.len() < existing.line.len() {
            to_remove.push(idx);
        }
    }

    if dominated {
        return;
    }

    for idx in to_remove.into_iter().rev() {
        end_states.remove(idx);
        *dominance_prunes = dominance_prunes.saturating_add(1);
    }

    end_states.push(candidate);
}

fn compare_end_states(left: &TurnEndState, right: &TurnEndState) -> std::cmp::Ordering {
    compare_frontier_eval(&left.frontier_eval, &right.frontier_eval)
        .then_with(|| right.resources.final_hp.cmp(&left.resources.final_hp))
        .then_with(|| right.resources.final_block.cmp(&left.resources.final_block))
        .then_with(|| {
            left.resources
                .spent_potions
                .cmp(&right.resources.spent_potions)
        })
        .then_with(|| left.resources.hp_lost.cmp(&right.resources.hp_lost))
        .then_with(|| {
            left.resources
                .exhausted_cards
                .cmp(&right.resources.exhausted_cards)
        })
        .then_with(|| left.line.len().cmp(&right.line.len()))
}

fn should_emit_leaf_candidate(
    engine: &EngineState,
    combat: &CombatState,
    can_continue_same_turn: bool,
    root_inputs: Option<&[ClientInput]>,
) -> bool {
    end_turn_available(engine, combat, root_inputs) || !can_continue_same_turn
}

fn legal_non_endturn_moves(
    engine: &EngineState,
    combat: &CombatState,
    root_inputs: Option<&[ClientInput]>,
) -> Vec<ClientInput> {
    reduce_equivalent_inputs(
        combat,
        root_inputs
            .map(|inputs| inputs.to_vec())
            .unwrap_or_else(|| get_legal_moves(engine, combat))
            .into_iter()
            .filter(|input| !matches!(input, ClientInput::EndTurn))
            .collect(),
        SearchEquivalenceMode::Safe,
    )
    .into_iter()
    .map(|cluster| cluster.representative)
    .collect()
}

fn end_turn_available(
    engine: &EngineState,
    combat: &CombatState,
    root_inputs: Option<&[ClientInput]>,
) -> bool {
    root_inputs
        .map(|inputs| inputs.to_vec())
        .unwrap_or_else(|| get_legal_moves(engine, combat))
        .into_iter()
        .any(|input| matches!(input, ClientInput::EndTurn))
}

fn can_continue_same_turn(
    engine: &EngineState,
    combat: &CombatState,
    root_inputs: Option<&[ClientInput]>,
) -> bool {
    (matches!(engine, EngineState::CombatPlayerTurn)
        || matches!(
            engine,
            EngineState::PendingChoice(choice) if pending_choice_is_same_turn_frontier(choice)
        ))
        && !legal_non_endturn_moves(engine, combat, root_inputs).is_empty()
}
