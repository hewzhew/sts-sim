mod ordering;
mod terminal;
mod types;
mod value;

use crate::bot::search::{default_equivalence_mode, get_legal_moves, SearchProfileBreakdown};
use crate::diff::replay::tick_until_stable;
use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;
use serde_json::json;
use std::time::Instant;

use ordering::compare_candidates;
use terminal::{survives, terminal_kind};
use types::CombatCandidate;
use value::{
    display_score, incoming_damage, project_turn_close_state, projected_unblocked, total_enemy_hp,
};

pub use crate::bot::search::{SearchDiagnostics, SearchMoveStat};

pub fn find_best_move(
    engine: &EngineState,
    combat: &CombatState,
    depth_limit: u32,
    _verbose: bool,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
) -> ClientInput {
    let _ = (db, coverage_mode, curiosity_target);
    diagnose_root_search_with_depth(
        engine,
        combat,
        db,
        coverage_mode,
        curiosity_target,
        depth_limit,
        0,
    )
    .chosen_move
}

pub fn diagnose_root_search(
    engine: &EngineState,
    combat: &CombatState,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    num_simulations: u32,
) -> SearchDiagnostics {
    let depth_limit = search_depth_for_budget(num_simulations);
    diagnose_root_search_with_depth(
        engine,
        combat,
        db,
        coverage_mode,
        curiosity_target,
        depth_limit,
        num_simulations,
    )
}

pub fn diagnose_root_search_with_depth(
    engine: &EngineState,
    combat: &CombatState,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    depth_limit: u32,
    _num_simulations: u32,
) -> SearchDiagnostics {
    let _ = (db, coverage_mode, curiosity_target);
    let started = Instant::now();
    let legal_moves = get_legal_moves(engine, combat);
    let (max_decision_depth, root_width, branch_width, max_engine_steps) =
        search_limits(depth_limit);
    let equivalence_mode = default_equivalence_mode();

    if legal_moves.is_empty() {
        return SearchDiagnostics {
            chosen_move: ClientInput::EndTurn,
            legal_moves: 0,
            reduced_legal_moves: 0,
            simulations: 0,
            elapsed_ms: started.elapsed().as_millis(),
            depth_limit,
            max_decision_depth,
            root_width,
            branch_width,
            max_engine_steps,
            equivalence_mode,
            root_prior_enabled: false,
            root_prior_key: None,
            root_prior_weight: 0.0,
            root_prior_hits: 0,
            root_prior_reordered: false,
            top_moves: Vec::new(),
            decision_audit: json!({}),
            profile: SearchProfileBreakdown::default(),
        };
    }

    let mut candidates = legal_moves
        .iter()
        .map(|input| simulate_candidate(engine, combat, input, max_engine_steps))
        .collect::<Vec<_>>();

    candidates.sort_by(compare_candidates);

    let top_moves = candidates
        .iter()
        .take(root_width.max(1))
        .enumerate()
        .map(|(idx, candidate)| build_move_stat(candidate, idx))
        .collect::<Vec<_>>();

    let chosen_move = candidates
        .first()
        .map(|candidate| candidate.input.clone())
        .unwrap_or(ClientInput::EndTurn);

    let mut profile = SearchProfileBreakdown::default();
    profile.search_total_ms = started.elapsed().as_millis();
    profile.root.legal_move_gen_calls = 1;
    profile.root.transition_reduce_inputs = legal_moves.len() as u32;
    profile.root.transition_reduce_outputs = legal_moves.len() as u32;
    profile.root.avg_branch_before_reduce = legal_moves.len() as f32;
    profile.root.avg_branch_after_reduce = legal_moves.len() as f32;
    profile.nodes.nodes_expanded = legal_moves.len() as u32;

    SearchDiagnostics {
        chosen_move,
        legal_moves: legal_moves.len(),
        reduced_legal_moves: legal_moves.len(),
        simulations: legal_moves.len() as u32,
        elapsed_ms: started.elapsed().as_millis(),
        depth_limit,
        max_decision_depth,
        root_width,
        branch_width,
        max_engine_steps,
        equivalence_mode,
        root_prior_enabled: false,
        root_prior_key: None,
        root_prior_weight: 0.0,
        root_prior_hits: 0,
        root_prior_reordered: false,
        top_moves,
        decision_audit: json!({
            "planner": "combat_baseline",
            "kind": "projected_turn_close"
        }),
        profile,
    }
}

fn simulate_candidate(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
    max_engine_steps: usize,
) -> CombatCandidate {
    let mut next_engine = engine.clone();
    let mut next_combat = combat.clone();
    let _ = tick_until_stable(&mut next_engine, &mut next_combat, input.clone());

    let (projected_engine, projected_combat) =
        project_turn_close_state(&next_engine, &next_combat, max_engine_steps);
    let terminal_kind = terminal_kind(&projected_engine, &projected_combat);
    let projected_hp = projected_combat.entities.player.current_hp;
    let projected_block = projected_combat.entities.player.block;
    let projected_enemy_total = total_enemy_hp(&projected_combat);
    let projected_unblocked = projected_unblocked(&projected_combat);
    let survives = survives(terminal_kind, projected_hp);
    let display_score = display_score(
        terminal_kind,
        projected_unblocked,
        projected_enemy_total,
        projected_hp,
        projected_block,
        input,
    );

    CombatCandidate {
        input: input.clone(),
        next_combat,
        terminal_kind,
        projected_hp,
        projected_block,
        projected_enemy_total,
        projected_unblocked,
        survives,
        display_score,
    }
}

fn build_move_stat(candidate: &CombatCandidate, idx: usize) -> SearchMoveStat {
    let immediate_incoming = incoming_damage(&candidate.next_combat);
    SearchMoveStat {
        input: candidate.input.clone(),
        visits: 1,
        avg_score: candidate.display_score,
        base_order_score: candidate.display_score,
        order_score: candidate.display_score,
        root_prior_score: 0.0,
        root_prior_hit: false,
        leaf_score: candidate.display_score,
        policy_bonus: 0.0,
        sequence_bonus: 0.0,
        sequence_survival_bonus: 0.0,
        sequence_exhaust_bonus: 0.0,
        sequence_frontload_bonus: 0.0,
        sequence_defer_bonus: 0.0,
        sequence_branch_bonus: 0.0,
        sequence_downside_penalty: 0.0,
        projected_hp: candidate.projected_hp,
        projected_block: candidate.projected_block,
        projected_enemy_total: candidate.projected_enemy_total,
        projected_unblocked: candidate.projected_unblocked,
        survives: candidate.survives,
        realized_exhaust_block: 0,
        realized_exhaust_draw: 0,
        immediate_hp: candidate.next_combat.entities.player.current_hp,
        immediate_block: candidate.next_combat.entities.player.block,
        immediate_energy: candidate.next_combat.turn.energy,
        immediate_hand_len: candidate.next_combat.zones.hand.len(),
        immediate_draw_len: candidate.next_combat.zones.draw_pile.len(),
        immediate_discard_len: candidate.next_combat.zones.discard_pile.len(),
        immediate_exhaust_len: candidate.next_combat.zones.exhaust_pile.len(),
        immediate_incoming,
        immediate_enemy_total: total_enemy_hp(&candidate.next_combat),
        cluster_id: format!("direct-{idx}"),
        cluster_size: 1,
        collapsed_inputs: Vec::new(),
        equivalence_kind: None,
    }
}

fn search_limits(depth_limit: u32) -> (usize, usize, usize, usize) {
    let depth = depth_limit.max(2) as usize;
    let root_width = if depth >= 8 { 10 } else { 8 };
    let branch_width = if depth >= 8 { 5 } else { 4 };
    let max_engine_steps = (depth * 40).max(80);
    (depth, root_width, branch_width, max_engine_steps)
}

fn search_depth_for_budget(num_simulations: u32) -> u32 {
    match num_simulations {
        0..=300 => 4,
        301..=900 => 5,
        901..=2000 => 6,
        _ => 7,
    }
}
