mod audit;
mod card_knowledge;
mod diag;
mod equivalence;
mod hand_select;
pub(crate) mod legal_moves;
pub(crate) mod monster_belief;
mod ordering;
mod planner;
pub(crate) mod posture;
pub(crate) mod pressure;
mod profile;
mod root_prior;
mod search;
mod terminal;
mod types;
mod value;

use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;
use serde_json::json;
use std::time::Instant;

use self::equivalence::default_equivalence_mode;
use self::legal_moves::get_legal_moves;
use search::ExploredCandidate;
use value::{diagnostic_score, incoming_damage, total_enemy_hp};

pub use audit::{
    audit_fixture, audit_state, build_fixture_from_reconstructed_step, extract_preference_samples,
    load_fixture_path, render_text_report, write_fixture_path, CombatPreferenceSample,
    CombatPreferenceState, DecisionAuditConfig, DecisionAuditEngineState, DecisionAuditFixture,
    DecisionAuditReport, ScoreBreakdown, TrajectoryOutcomeKind, TrajectoryReport,
};
pub use card_knowledge::{branch_family_for_card, BranchFamily};
pub use diag::{CombatDiagnostics, CombatMoveStat};
pub use equivalence::{SearchEquivalenceKind, SearchEquivalenceMode};
pub use legal_moves::legal_moves_for_audit;
pub use profile::{
    SearchNodeCounters, SearchPhaseProfile, SearchProfileBreakdown, SearchProfilingLevel,
};
pub use root_prior::{LookupRootPriorProvider, RootPriorConfig, RootPriorQueryKey};

pub fn find_best_move(
    engine: &EngineState,
    combat: &CombatState,
    depth_limit: u32,
    _verbose: bool,
) -> ClientInput {
    diagnose_root_search_with_depth(engine, combat, depth_limit, 0).chosen_move
}

pub fn describe_end_turn_options(combat: &CombatState) -> Vec<String> {
    let engine = EngineState::CombatPlayerTurn;
    let diagnostics = diagnose_root_search_with_depth(&engine, combat, 2, 0);
    let legal_plays = get_legal_moves(&engine, combat)
        .into_iter()
        .filter(|input| !matches!(input, ClientInput::EndTurn))
        .count();

    let end_score = diagnostics
        .top_moves
        .iter()
        .find(|stat| matches!(stat.input, ClientInput::EndTurn))
        .map(|stat| stat.avg_score);
    if legal_plays == 0 {
        return vec![format!(
            "END score={:.1} no_legal_plays",
            end_score.unwrap_or(0.0)
        )];
    }

    let mut lines = vec![format!(
        "END score={:.1} legal_plays={}",
        end_score.unwrap_or(0.0),
        legal_plays
    )];
    for stat in diagnostics
        .top_moves
        .iter()
        .filter(|stat| !matches!(stat.input, ClientInput::EndTurn))
        .take(8)
    {
        lines.push(format!(
            "play move={:?} score={:.1} visits={}",
            stat.input, stat.avg_score, stat.visits
        ));
    }
    lines
}

pub fn diagnose_root_search(
    engine: &EngineState,
    combat: &CombatState,
    num_simulations: u32,
) -> CombatDiagnostics {
    let depth_limit = search_depth_for_budget(num_simulations);
    diagnose_root_search_with_depth(engine, combat, depth_limit, num_simulations)
}

pub fn diagnose_root_search_with_depth(
    engine: &EngineState,
    combat: &CombatState,
    depth_limit: u32,
    _num_simulations: u32,
) -> CombatDiagnostics {
    let started = Instant::now();
    let legal_moves = get_legal_moves(engine, combat);
    let (max_decision_depth, root_width, branch_width, max_engine_steps) =
        search_limits(depth_limit);
    let equivalence_mode = default_equivalence_mode();

    if legal_moves.is_empty() {
        return CombatDiagnostics {
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

    let explored = search::explore_root(
        engine,
        combat,
        max_decision_depth,
        root_width,
        branch_width,
        max_engine_steps,
    );

    let top_moves = explored
        .iter()
        .enumerate()
        .map(|(idx, candidate)| build_move_stat(candidate, idx))
        .collect::<Vec<_>>();

    let chosen_move = explored
        .first()
        .map(|candidate| candidate.candidate.input.clone())
        .unwrap_or(ClientInput::EndTurn);
    let simulations = explored
        .iter()
        .map(|candidate| candidate.explored_nodes)
        .sum::<u32>()
        .max(legal_moves.len() as u32);

    let mut profile = SearchProfileBreakdown::default();
    profile.search_total_ms = started.elapsed().as_millis();
    profile.root.legal_move_gen_calls = 1;
    profile.root.transition_reduce_inputs = legal_moves.len() as u32;
    profile.root.transition_reduce_outputs = explored.len() as u32;
    profile.root.avg_branch_before_reduce = legal_moves.len() as f32;
    profile.root.avg_branch_after_reduce = explored.len().max(1) as f32;
    profile.nodes.nodes_expanded = simulations;

    CombatDiagnostics {
        chosen_move,
        legal_moves: legal_moves.len(),
        reduced_legal_moves: explored.len(),
        simulations,
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
            "kind": "frontier_driven_best_line"
        }),
        profile,
    }
}

pub fn diagnose_root_search_with_depth_and_mode(
    engine: &EngineState,
    combat: &CombatState,
    depth_limit: u32,
    num_simulations: u32,
    _equivalence_mode: SearchEquivalenceMode,
) -> CombatDiagnostics {
    diagnose_root_search_with_depth(engine, combat, depth_limit, num_simulations)
}

pub fn diagnose_root_search_with_depth_and_mode_and_root_prior(
    engine: &EngineState,
    combat: &CombatState,
    depth_limit: u32,
    num_simulations: u32,
    _equivalence_mode: SearchEquivalenceMode,
    _profiling_level: SearchProfilingLevel,
    _root_prior: Option<&RootPriorConfig>,
) -> CombatDiagnostics {
    diagnose_root_search_with_depth(engine, combat, depth_limit, num_simulations)
}

fn build_move_stat(explored: &ExploredCandidate, idx: usize) -> CombatMoveStat {
    let candidate = &explored.candidate;
    let immediate_incoming = incoming_damage(&candidate.next_combat);
    let search_score = diagnostic_score(explored.search_value, &candidate.input);
    CombatMoveStat {
        input: candidate.input.clone(),
        visits: explored.explored_nodes.max(1),
        avg_score: search_score,
        base_order_score: candidate.diagnostic_score,
        order_score: search_score,
        root_prior_score: 0.0,
        root_prior_hit: false,
        leaf_score: candidate.diagnostic_score,
        policy_bonus: search_score - candidate.diagnostic_score,
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
        cluster_id: format!("frontier-{idx}-len{}", candidate.local_plan.len()),
        cluster_size: 1,
        collapsed_inputs: candidate.local_plan.iter().skip(1).cloned().collect(),
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
