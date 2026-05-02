mod audit;
mod card_knowledge;
mod decision;
mod diag;
mod dominance;
mod equivalence;
pub mod exact_turn_solver;
mod frontier_eval;
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
mod stepping;
mod terminal;
pub(crate) mod turn_plan_probe;
mod turn_state_key;
mod types;
mod value;

use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;
use serde_json::json;
use std::time::Instant;

use self::decision::{
    build_decision_trace, build_exact_turn_verdict, classify_proposal_class, classify_regime,
    exact_turn_takeover_policy, frontier_outcome_from_candidate, CombatRegime, DecisionOutcome,
    DecisionTrace, DominanceClaim, ExactTurnVerdict, ExactnessLevel, ProposalTrace,
    ScreenRejection, TakeoverPolicy,
};
use self::equivalence::{default_equivalence_mode, reduce_equivalent_inputs};
use self::exact_turn_solver::{solve_exact_turn_with_config, ExactTurnConfig};
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
pub use turn_plan_probe::{
    probe_turn_plans, CombatPlanProbeLimits, CombatPlanReport, CombatPlanRiskNote,
    CombatPlanSequenceClass, CombatPlanStateSummary, CombatTurnPlanProbeConfig,
    CombatTurnPlanProbeReport, PlanScoreBreakdown,
};

struct ExactTurnShadowDecision {
    audit: serde_json::Value,
    regime: CombatRegime,
    frontier_outcome: DecisionOutcome,
    exact_turn_verdict: ExactTurnVerdict,
    takeover_policy: TakeoverPolicy,
    decision_trace: DecisionTrace,
    takeover_move: Option<ClientInput>,
    timed_out: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchExactTurnMode {
    Off,
    Auto,
    Force,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SearchExperimentFlags {
    pub contested_strict_dominance_takeover: bool,
    pub advantage_strict_dominance_takeover: bool,
    pub forbid_idle_end_turn_when_exact_prefers_play: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct SearchRuntimeBudget {
    pub wall_clock_deadline: Option<Instant>,
    pub root_node_budget: usize,
    pub engine_step_budget: usize,
    pub exact_turn_node_budget: usize,
    pub audit_budget: usize,
    pub exact_turn_mode: SearchExactTurnMode,
    pub experiment_flags: SearchExperimentFlags,
}

impl Default for SearchRuntimeBudget {
    fn default() -> Self {
        Self {
            wall_clock_deadline: None,
            root_node_budget: 64,
            engine_step_budget: 160,
            exact_turn_node_budget: 8_000,
            audit_budget: 16,
            exact_turn_mode: SearchExactTurnMode::Auto,
            experiment_flags: SearchExperimentFlags::default(),
        }
    }
}

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
    diagnose_root_search_with_runtime(
        engine,
        combat,
        num_simulations,
        SearchRuntimeBudget::default(),
    )
}

pub fn diagnose_root_search_with_runtime(
    engine: &EngineState,
    combat: &CombatState,
    num_simulations: u32,
    runtime: SearchRuntimeBudget,
) -> CombatDiagnostics {
    diagnose_root_search_with_runtime_and_root_inputs(
        engine,
        combat,
        num_simulations,
        runtime,
        None,
    )
}

pub fn diagnose_root_search_with_runtime_and_root_inputs(
    engine: &EngineState,
    combat: &CombatState,
    num_simulations: u32,
    runtime: SearchRuntimeBudget,
    root_inputs: Option<Vec<ClientInput>>,
) -> CombatDiagnostics {
    let depth_limit = search_depth_for_budget(num_simulations);
    diagnose_root_search_with_depth_and_runtime_and_root_inputs(
        engine,
        combat,
        depth_limit,
        num_simulations,
        runtime,
        root_inputs,
    )
}

pub fn diagnose_root_search_with_depth(
    engine: &EngineState,
    combat: &CombatState,
    depth_limit: u32,
    num_simulations: u32,
) -> CombatDiagnostics {
    diagnose_root_search_with_depth_and_runtime(
        engine,
        combat,
        depth_limit,
        num_simulations,
        SearchRuntimeBudget::default(),
    )
}

pub fn diagnose_root_search_with_depth_and_runtime(
    engine: &EngineState,
    combat: &CombatState,
    depth_limit: u32,
    _num_simulations: u32,
    runtime: SearchRuntimeBudget,
) -> CombatDiagnostics {
    diagnose_root_search_with_depth_and_runtime_and_root_inputs(
        engine,
        combat,
        depth_limit,
        _num_simulations,
        runtime,
        None,
    )
}

pub fn diagnose_root_search_with_depth_and_runtime_and_root_inputs(
    engine: &EngineState,
    combat: &CombatState,
    depth_limit: u32,
    _num_simulations: u32,
    runtime: SearchRuntimeBudget,
    root_inputs: Option<Vec<ClientInput>>,
) -> CombatDiagnostics {
    let started = Instant::now();
    let legal_moves = root_inputs.unwrap_or_else(|| get_legal_moves(engine, combat));
    let (max_decision_depth, root_width, branch_width, max_engine_steps, root_node_budget) =
        search_limits(depth_limit, runtime);
    let equivalence_mode = default_equivalence_mode();

    if legal_moves.is_empty() {
        return CombatDiagnostics {
            chosen_move: ClientInput::EndTurn,
            legal_moves: 0,
            reduced_legal_moves: 0,
            simulations: 0,
            elapsed_ms: started.elapsed().as_millis(),
            timed_out: false,
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
            decision_audit: json!({
                "regime": serde_json::Value::Null,
                "frontier_outcome": serde_json::Value::Null,
                "exact_turn_verdict": serde_json::Value::Null,
                "takeover_policy": serde_json::Value::Null,
                "decision_trace": serde_json::Value::Null,
                "exact_turn_shadow": serde_json::Value::Null,
            }),
            profile: SearchProfileBreakdown::default(),
        };
    }

    let mut profile = SearchProfileBreakdown::default();
    let chooser_started = Instant::now();
    let explored = search::explore_root_with_inputs(
        engine,
        combat,
        legal_moves.clone(),
        max_decision_depth,
        root_width,
        branch_width,
        max_engine_steps,
        root_node_budget,
        runtime.wall_clock_deadline,
        equivalence_mode,
        &mut profile,
    );
    profile.chooser_ms = chooser_started.elapsed().as_millis();

    let top_moves = explored
        .explored
        .iter()
        .enumerate()
        .map(|(idx, candidate)| build_move_stat(candidate, idx))
        .collect::<Vec<_>>();

    let frontier_candidate = explored
        .explored
        .first()
        .expect("non-empty legal move set should produce a frontier candidate");
    let frontier_chosen_move = frontier_candidate.candidate.input.clone();
    let exact_turn_shadow = if matches!(runtime.exact_turn_mode, SearchExactTurnMode::Off) {
        let regime = classify_regime(combat);
        let frontier_outcome =
            frontier_outcome_from_candidate(combat, &frontier_candidate.candidate);
        let exact_turn_verdict = ExactTurnVerdict {
            best_first_input: None,
            best_outcome: None,
            survival: frontier_outcome.survival,
            dominance: DominanceClaim::Incomparable,
            lethal_window: None,
            confidence: ExactnessLevel::Unavailable,
            truncated: false,
        };
        let (takeover_move, takeover_policy, chosen_by, rejection_reasons) =
            exact_turn_takeover_policy(
                engine,
                &frontier_chosen_move,
                regime,
                &frontier_outcome,
                &exact_turn_verdict,
                &exact_turn_solver::ExactTurnSolution {
                    best_first_input: None,
                    best_line: Vec::new(),
                    nondominated_end_states: Vec::new(),
                    elapsed_ms: 0,
                    explored_nodes: 0,
                    dominance_prunes: 0,
                    cycle_cuts: 0,
                    cache_hits: 0,
                    cache_misses: 0,
                    truncated: false,
                },
                runtime.experiment_flags,
            );
        ExactTurnShadowDecision {
            audit: json!({
                "frontier_chosen_move": format!("{:?}", frontier_chosen_move),
                "disabled": true,
                "reason": "exact_turn_off",
            }),
            regime,
            frontier_outcome: frontier_outcome.clone(),
            exact_turn_verdict: exact_turn_verdict.clone(),
            takeover_policy,
            decision_trace: build_decision_trace(
                &frontier_chosen_move,
                chosen_by,
                regime,
                classify_proposal_class(combat, &frontier_chosen_move),
                frontier_outcome,
                exact_turn_verdict,
                rejection_reasons,
                explored.screened_out.clone(),
                explored.proposal_trace.clone(),
            ),
            takeover_move,
            timed_out: false,
        }
    } else {
        build_exact_turn_shadow(
            engine,
            combat,
            max_engine_steps,
            &legal_moves,
            &frontier_chosen_move,
            &frontier_candidate.candidate,
            legal_moves.len(),
            explored.screened_out.clone(),
            explored.proposal_trace.clone(),
            runtime.exact_turn_node_budget,
            runtime.wall_clock_deadline,
            runtime.exact_turn_mode,
            runtime.experiment_flags,
            &mut profile,
        )
    };
    let chosen_move = exact_turn_shadow
        .takeover_move
        .clone()
        .unwrap_or_else(|| frontier_chosen_move.clone());
    let simulations = explored
        .explored
        .iter()
        .map(|candidate| candidate.explored_nodes)
        .sum::<u32>()
        .max(legal_moves.len() as u32);

    profile.search_total_ms = started.elapsed().as_millis();
    profile.root.legal_move_gen_calls = 1;
    profile.root.transition_reduce_inputs = legal_moves.len() as u32;
    profile.root.transition_reduce_outputs = explored.explored.len() as u32;
    profile.root.avg_branch_before_reduce = legal_moves.len() as f32;
    profile.root.avg_branch_after_reduce = explored.explored.len().max(1) as f32;
    profile.nodes.nodes_expanded = simulations;
    profile.finalize_samples();

    CombatDiagnostics {
        chosen_move,
        legal_moves: legal_moves.len(),
        reduced_legal_moves: explored.explored.len(),
        simulations,
        elapsed_ms: started.elapsed().as_millis(),
        timed_out: explored.timed_out || exact_turn_shadow.timed_out,
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
            "kind": "frontier_driven_best_line",
            "root_pipeline": {
                "regime": format!("{:?}", explored.regime).to_ascii_lowercase(),
                "proposal_count": explored.proposal_count,
                "screened_count": explored.screened_count,
                "exact_adjudicated_count": explored.exact_adjudicated_count,
                "proposal_class_counts": explored.proposal_class_counts,
                "screened_out": explored.screened_out,
            },
            "regime": exact_turn_shadow.regime,
            "frontier_outcome": exact_turn_shadow.frontier_outcome,
            "exact_turn_verdict": exact_turn_shadow.exact_turn_verdict,
            "takeover_policy": exact_turn_shadow.takeover_policy,
            "decision_trace": exact_turn_shadow.decision_trace,
            "exact_turn_shadow": exact_turn_shadow.audit,
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
    diagnose_root_search_with_depth_and_runtime(
        engine,
        combat,
        depth_limit,
        num_simulations,
        SearchRuntimeBudget::default(),
    )
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
    diagnose_root_search_with_depth_and_runtime(
        engine,
        combat,
        depth_limit,
        num_simulations,
        SearchRuntimeBudget::default(),
    )
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
        cluster_size: candidate.cluster_size,
        collapsed_inputs: candidate.collapsed_inputs.clone(),
        equivalence_kind: if candidate.cluster_size > 1 {
            Some(SearchEquivalenceKind::Exact)
        } else {
            None
        },
    }
}

fn build_exact_turn_shadow(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
    root_inputs: &[ClientInput],
    chosen_move: &ClientInput,
    frontier_candidate: &types::CombatCandidate,
    legal_move_count: usize,
    screened_out: Vec<ScreenRejection>,
    proposal_trace: Vec<ProposalTrace>,
    max_nodes: usize,
    deadline: Option<Instant>,
    exact_turn_mode: SearchExactTurnMode,
    experiment_flags: SearchExperimentFlags,
    profile: &mut SearchProfileBreakdown,
) -> ExactTurnShadowDecision {
    let regime = classify_regime(combat);
    let frontier_outcome = frontier_outcome_from_candidate(combat, frontier_candidate);

    if !matches!(
        engine,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        let exact_turn_verdict = ExactTurnVerdict {
            best_first_input: None,
            best_outcome: None,
            survival: frontier_outcome.survival,
            dominance: DominanceClaim::Incomparable,
            lethal_window: None,
            confidence: ExactnessLevel::Unavailable,
            truncated: false,
        };
        let (_, takeover_policy, chosen_by, rejection_reasons) = exact_turn_takeover_policy(
            engine,
            chosen_move,
            regime,
            &frontier_outcome,
            &exact_turn_verdict,
            &exact_turn_solver::ExactTurnSolution {
                best_first_input: None,
                best_line: Vec::new(),
                nondominated_end_states: Vec::new(),
                elapsed_ms: 0,
                explored_nodes: 0,
                dominance_prunes: 0,
                cycle_cuts: 0,
                cache_hits: 0,
                cache_misses: 0,
                truncated: false,
            },
            experiment_flags,
        );
        return ExactTurnShadowDecision {
            audit: serde_json::Value::Null,
            regime,
            frontier_outcome: frontier_outcome.clone(),
            exact_turn_verdict: exact_turn_verdict.clone(),
            takeover_policy,
            decision_trace: build_decision_trace(
                chosen_move,
                chosen_by,
                regime,
                classify_proposal_class(combat, chosen_move),
                frontier_outcome,
                exact_turn_verdict,
                rejection_reasons,
                screened_out.clone(),
                proposal_trace.clone(),
            ),
            takeover_move: None,
            timed_out: false,
        };
    }

    if !matches!(exact_turn_mode, SearchExactTurnMode::Force) {
        if let Some(skip_reason) = exact_turn_shadow_skip_reason(combat, legal_move_count) {
            let exact_turn_verdict = ExactTurnVerdict {
                best_first_input: None,
                best_outcome: None,
                survival: frontier_outcome.survival,
                dominance: DominanceClaim::Incomparable,
                lethal_window: None,
                confidence: ExactnessLevel::Unavailable,
                truncated: false,
            };
            let (_, takeover_policy, chosen_by, mut rejection_reasons) = exact_turn_takeover_policy(
                engine,
                chosen_move,
                regime,
                &frontier_outcome,
                &exact_turn_verdict,
                &exact_turn_solver::ExactTurnSolution {
                    best_first_input: None,
                    best_line: Vec::new(),
                    nondominated_end_states: Vec::new(),
                    elapsed_ms: 0,
                    explored_nodes: 0,
                    dominance_prunes: 0,
                    cycle_cuts: 0,
                    cache_hits: 0,
                    cache_misses: 0,
                    truncated: false,
                },
                experiment_flags,
            );
            rejection_reasons.push(skip_reason.to_string());
            return ExactTurnShadowDecision {
                takeover_move: None,
                audit: json!({
                    "frontier_chosen_move": format!("{:?}", chosen_move),
                    "skipped": true,
                    "skip_reason": skip_reason,
                    "legal_moves": legal_move_count,
                    "living_monsters": living_monster_count(combat),
                    "filled_potions": combat.entities.potions.iter().flatten().count(),
                }),
                regime,
                frontier_outcome: frontier_outcome.clone(),
                exact_turn_verdict: exact_turn_verdict.clone(),
                takeover_policy,
                decision_trace: build_decision_trace(
                    chosen_move,
                    chosen_by,
                    regime,
                    classify_proposal_class(combat, chosen_move),
                    frontier_outcome,
                    exact_turn_verdict,
                    rejection_reasons,
                    screened_out.clone(),
                    proposal_trace.clone(),
                ),
                timed_out: false,
            };
        }
    }

    let exact_turn_started = Instant::now();
    let solution = solve_exact_turn_with_config(
        engine,
        combat,
        ExactTurnConfig {
            max_nodes,
            max_engine_steps,
            deadline,
            root_inputs: Some(root_inputs.to_vec()),
        },
    );
    profile.record_exact_turn_call(exact_turn_started.elapsed().as_millis());
    if solution.truncated {
        profile.note_timeout_source("exact_turn");
    }
    for _ in 0..solution.cache_hits {
        profile.record_cache_hit();
    }
    for _ in 0..solution.cache_misses {
        profile.record_cache_miss();
    }
    let best_state = solution.nondominated_end_states.first();
    let exact_turn_verdict = build_exact_turn_verdict(chosen_move, &frontier_outcome, &solution);
    let (takeover_move, takeover_policy, chosen_by, rejection_reasons) = exact_turn_takeover_policy(
        engine,
        chosen_move,
        regime,
        &frontier_outcome,
        &exact_turn_verdict,
        &solution,
        experiment_flags,
    );
    let takeover_applied = takeover_move.is_some();
    let takeover_move_audit = takeover_move.as_ref().map(|input| format!("{:?}", input));

    ExactTurnShadowDecision {
        takeover_move,
        timed_out: solution.truncated,
        audit: json!({
            "frontier_chosen_move": format!("{:?}", chosen_move),
            "best_first_input": solution.best_first_input.as_ref().map(|input| format!("{:?}", input)),
            "best_line": solution
                .best_line
                .iter()
                .map(|input| format!("{:?}", input))
                .collect::<Vec<_>>(),
            "best_line_len": solution.best_line.len(),
            "elapsed_ms": solution.elapsed_ms,
            "nondominated_end_states": solution.nondominated_end_states.len(),
            "explored_nodes": solution.explored_nodes,
            "dominance_prunes": solution.dominance_prunes,
            "cycle_cuts": solution.cycle_cuts,
            "cache_hits": solution.cache_hits,
            "cache_misses": solution.cache_misses,
            "truncated": solution.truncated,
            "agrees_with_frontier": solution.best_first_input.as_ref() == Some(chosen_move),
            "takeover_eligible": takeover_policy.takeover_eligible,
            "takeover_applied": takeover_applied,
            "takeover_reason": takeover_policy.takeover_reason,
            "takeover_move": takeover_move_audit,
            "best_resources": best_state.map(|state| {
                json!({
                    "spent_potions": state.resources.spent_potions,
                    "hp_lost": state.resources.hp_lost,
                    "exhausted_cards": state.resources.exhausted_cards,
                    "final_hp": state.resources.final_hp,
                    "final_block": state.resources.final_block,
                })
            }),
        }),
        regime,
        frontier_outcome: frontier_outcome.clone(),
        exact_turn_verdict: exact_turn_verdict.clone(),
        takeover_policy,
        decision_trace: build_decision_trace(
            chosen_move,
            chosen_by,
            regime,
            classify_proposal_class(combat, chosen_move),
            frontier_outcome,
            exact_turn_verdict,
            rejection_reasons,
            screened_out,
            proposal_trace,
        ),
    }
}

fn exact_turn_shadow_skip_reason(
    combat: &CombatState,
    legal_move_count: usize,
) -> Option<&'static str> {
    let living_monsters = living_monster_count(combat);
    let filled_potions = combat.entities.potions.iter().flatten().count();
    let hand_len = combat.zones.hand.len();
    let has_confusion = combat.entities.power_db.get(&0).is_some_and(|powers| {
        powers
            .iter()
            .any(|power| power.power_type == crate::content::powers::PowerId::Confusion)
    });
    let duplicate_card_groups = duplicate_hand_card_groups(combat);

    if has_confusion && hand_len >= 6 {
        Some("confusion_high_entropy")
    } else if duplicate_card_groups >= 3 && hand_len >= 6 {
        Some("duplicate_card_permutations")
    } else if legal_move_count >= 12 {
        Some("high_root_branching")
    } else if living_monsters >= 3 && legal_move_count >= 10 {
        Some("multi_monster_branching")
    } else if filled_potions >= 2 && legal_move_count >= 8 {
        Some("potion_branching")
    } else {
        None
    }
}

fn duplicate_hand_card_groups(combat: &CombatState) -> usize {
    reduce_equivalent_inputs(
        combat,
        combat
            .zones
            .hand
            .iter()
            .enumerate()
            .map(|(card_index, card)| ClientInput::PlayCard {
                card_index,
                target: if crate::engine::targeting::validation_for_card_target(
                    crate::content::cards::effective_target(card),
                )
                .is_some()
                {
                    Some(0)
                } else {
                    None
                },
            })
            .collect(),
        SearchEquivalenceMode::Safe,
    )
    .into_iter()
    .filter(|cluster| !cluster.collapsed_inputs.is_empty())
    .count()
}

fn living_monster_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.half_dead && !monster.is_escaped && monster.current_hp > 0
        })
        .count()
}

fn search_limits(
    depth_limit: u32,
    runtime: SearchRuntimeBudget,
) -> (usize, usize, usize, usize, usize) {
    let depth = depth_limit.max(2) as usize;
    let legacy_root_width = if depth >= 8 { 10 } else { 8 };
    let legacy_branch_width = if depth >= 8 { 5 } else { 4 };
    let root_width = legacy_root_width.min(runtime.root_node_budget.max(1));
    let branch_width = legacy_branch_width.min(runtime.root_node_budget.max(1));
    let max_engine_steps = runtime.engine_step_budget.max((depth * 20).max(80));
    (
        depth,
        root_width,
        branch_width,
        max_engine_steps,
        runtime.root_node_budget.max(root_width.max(branch_width)),
    )
}

fn search_depth_for_budget(num_simulations: u32) -> u32 {
    match num_simulations {
        0..=300 => 4,
        301..=900 => 5,
        901..=2000 => 6,
        _ => 7,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        diagnose_root_search_with_runtime, exact_turn_shadow_skip_reason,
        exact_turn_takeover_policy, CombatRegime, DominanceClaim, ExactTurnVerdict, ExactnessLevel,
        SearchExperimentFlags, SearchRuntimeBudget,
    };
    use crate::bot::combat::decision::SurvivalJudgement;
    use crate::bot::combat::exact_turn_solver::ExactTurnSolution;
    use crate::content::cards::CardId;
    use crate::content::powers::PowerId;
    use crate::runtime::combat::{CombatCard, Power};
    use crate::state::core::{ClientInput, PendingChoice};
    use crate::state::EngineState;
    use crate::test_support::{blank_test_combat, planned_monster};
    use crate::{bot::combat::decision::DecisionOutcome, content::monsters::EnemyId};

    fn solution(best_first_input: Option<ClientInput>) -> ExactTurnSolution {
        ExactTurnSolution {
            best_first_input,
            best_line: Vec::new(),
            nondominated_end_states: Vec::new(),
            elapsed_ms: 0,
            explored_nodes: 1,
            dominance_prunes: 0,
            cycle_cuts: 0,
            cache_hits: 0,
            cache_misses: 0,
            truncated: false,
        }
    }

    #[test]
    fn crisis_regime_allows_exact_turn_strict_dominance_takeover() {
        let frontier = DecisionOutcome {
            survival: SurvivalJudgement::SevereRisk,
            position: crate::bot::combat::decision::PositionClass::Collapsing,
            terminality: crate::bot::combat::decision::TerminalForecast::SurvivesWindow,
            resource_delta: crate::bot::combat::decision::ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 2,
                exhausted_cards: 0,
                final_hp: 4,
                final_block: 0,
            },
            efficiency_score: 0.0,
        };
        let verdict = ExactTurnVerdict {
            best_first_input: Some("PlayCard { card_index: 1, target: None }".to_string()),
            best_outcome: None,
            survival: SurvivalJudgement::Stable,
            dominance: DominanceClaim::StrictlyBetterInWindow,
            lethal_window: None,
            confidence: ExactnessLevel::Exact,
            truncated: false,
        };
        let (takeover, policy, authority, _) = exact_turn_takeover_policy(
            &EngineState::CombatPlayerTurn,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            CombatRegime::Crisis,
            &frontier,
            &verdict,
            &solution(Some(ClientInput::PlayCard {
                card_index: 1,
                target: None,
            })),
            SearchExperimentFlags::default(),
        );

        assert_eq!(
            takeover,
            Some(ClientInput::PlayCard {
                card_index: 1,
                target: None,
            })
        );
        assert_eq!(policy.takeover_reason, "crisis_strict_dominance");
        assert!(matches!(
            authority,
            crate::bot::combat::decision::DecisionAuthority::ExactTurnTakeover
        ));
    }

    #[test]
    fn fragile_regime_takeover_requires_survival_upgrade() {
        let frontier = DecisionOutcome {
            survival: SurvivalJudgement::RiskyButPlayable,
            position: crate::bot::combat::decision::PositionClass::TempoNeutral,
            terminality: crate::bot::combat::decision::TerminalForecast::SurvivesWindow,
            resource_delta: crate::bot::combat::decision::ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 1,
                exhausted_cards: 0,
                final_hp: 10,
                final_block: 2,
            },
            efficiency_score: 1.0,
        };
        let verdict = ExactTurnVerdict {
            best_first_input: Some("SubmitDiscoverChoice(1)".to_string()),
            best_outcome: None,
            survival: SurvivalJudgement::Stable,
            dominance: DominanceClaim::Incomparable,
            lethal_window: None,
            confidence: ExactnessLevel::Exact,
            truncated: false,
        };
        let (takeover, policy, _, _) = exact_turn_takeover_policy(
            &EngineState::PendingChoice(PendingChoice::StanceChoice),
            &ClientInput::SubmitDiscoverChoice(0),
            CombatRegime::Fragile,
            &frontier,
            &verdict,
            &solution(Some(ClientInput::SubmitDiscoverChoice(1))),
            SearchExperimentFlags::default(),
        );

        assert_eq!(takeover, Some(ClientInput::SubmitDiscoverChoice(1)));
        assert_eq!(policy.takeover_reason, "override_pending_choice");
    }

    #[test]
    fn contested_regime_records_disagreement_without_takeover() {
        let frontier = DecisionOutcome {
            survival: SurvivalJudgement::Stable,
            position: crate::bot::combat::decision::PositionClass::TempoNeutral,
            terminality: crate::bot::combat::decision::TerminalForecast::SurvivesWindow,
            resource_delta: crate::bot::combat::decision::ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 0,
                exhausted_cards: 0,
                final_hp: 30,
                final_block: 3,
            },
            efficiency_score: 4.0,
        };
        let verdict = ExactTurnVerdict {
            best_first_input: Some("PlayCard { card_index: 1, target: None }".to_string()),
            best_outcome: None,
            survival: SurvivalJudgement::Stable,
            dominance: DominanceClaim::StrictlyBetterInWindow,
            lethal_window: None,
            confidence: ExactnessLevel::Exact,
            truncated: false,
        };
        let (takeover, policy, authority, reasons) = exact_turn_takeover_policy(
            &EngineState::CombatPlayerTurn,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            CombatRegime::Contested,
            &frontier,
            &verdict,
            &solution(Some(ClientInput::PlayCard {
                card_index: 1,
                target: None,
            })),
            SearchExperimentFlags::default(),
        );

        assert_eq!(takeover, None);
        assert_eq!(policy.takeover_reason, "regime_not_takeover");
        assert!(matches!(
            authority,
            crate::bot::combat::decision::DecisionAuthority::Frontier
        ));
        assert!(reasons.iter().any(|reason| reason == "regime_not_takeover"));
    }

    #[test]
    fn contested_regime_can_takeover_with_experiment_flag() {
        let frontier = DecisionOutcome {
            survival: SurvivalJudgement::Stable,
            position: crate::bot::combat::decision::PositionClass::TempoNeutral,
            terminality: crate::bot::combat::decision::TerminalForecast::SurvivesWindow,
            resource_delta: crate::bot::combat::decision::ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 0,
                exhausted_cards: 0,
                final_hp: 30,
                final_block: 2,
            },
            efficiency_score: 3.0,
        };
        let verdict = ExactTurnVerdict {
            best_first_input: Some("PlayCard { card_index: 1, target: None }".to_string()),
            best_outcome: None,
            survival: SurvivalJudgement::Stable,
            dominance: DominanceClaim::StrictlyBetterInWindow,
            lethal_window: None,
            confidence: ExactnessLevel::Exact,
            truncated: false,
        };
        let (takeover, policy, authority, reasons) = exact_turn_takeover_policy(
            &EngineState::CombatPlayerTurn,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            CombatRegime::Contested,
            &frontier,
            &verdict,
            &solution(Some(ClientInput::PlayCard {
                card_index: 1,
                target: None,
            })),
            SearchExperimentFlags {
                contested_strict_dominance_takeover: true,
                ..SearchExperimentFlags::default()
            },
        );

        assert_eq!(
            takeover,
            Some(ClientInput::PlayCard {
                card_index: 1,
                target: None,
            })
        );
        assert_eq!(policy.takeover_reason, "contested_strict_dominance");
        assert!(matches!(
            authority,
            crate::bot::combat::decision::DecisionAuthority::ExactTurnTakeover
        ));
        assert!(reasons
            .iter()
            .any(|reason| reason == "contested_strict_dominance"));
    }

    #[test]
    fn idle_end_turn_guardrail_can_force_exact_turn_play() {
        let frontier = DecisionOutcome {
            survival: SurvivalJudgement::Stable,
            position: crate::bot::combat::decision::PositionClass::TempoNeutral,
            terminality: crate::bot::combat::decision::TerminalForecast::SurvivesWindow,
            resource_delta: crate::bot::combat::decision::ResourceDeltaSummary {
                spent_potions: 0,
                hp_lost: 0,
                exhausted_cards: 0,
                final_hp: 40,
                final_block: 0,
            },
            efficiency_score: 0.0,
        };
        let verdict = ExactTurnVerdict {
            best_first_input: Some("PlayCard { card_index: 2, target: None }".to_string()),
            best_outcome: None,
            survival: SurvivalJudgement::Stable,
            dominance: DominanceClaim::StrictlyBetterInWindow,
            lethal_window: None,
            confidence: ExactnessLevel::Exact,
            truncated: false,
        };
        let (takeover, policy, authority, reasons) = exact_turn_takeover_policy(
            &EngineState::CombatPlayerTurn,
            &ClientInput::EndTurn,
            CombatRegime::Advantage,
            &frontier,
            &verdict,
            &solution(Some(ClientInput::PlayCard {
                card_index: 2,
                target: None,
            })),
            SearchExperimentFlags {
                forbid_idle_end_turn_when_exact_prefers_play: true,
                ..SearchExperimentFlags::default()
            },
        );

        assert_eq!(
            takeover,
            Some(ClientInput::PlayCard {
                card_index: 2,
                target: None,
            })
        );
        assert_eq!(policy.takeover_reason, "idle_end_turn_strict_dominance");
        assert!(matches!(
            authority,
            crate::bot::combat::decision::DecisionAuthority::ExactTurnTakeover
        ));
        assert!(reasons
            .iter()
            .any(|reason| reason == "idle_end_turn_guardrail"));
    }

    #[test]
    fn exact_turn_skip_reason_avoids_confusion_high_entropy_hands() {
        let mut combat = blank_test_combat();
        combat.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Confusion,
                instance_id: None,
                amount: 0,
                extra_data: 0,
                just_applied: false,
            }],
        );
        for uuid in 0..6 {
            combat
                .zones
                .hand
                .push(CombatCard::new(CardId::Defend, uuid + 1));
        }

        assert_eq!(
            exact_turn_shadow_skip_reason(&combat, 9),
            Some("confusion_high_entropy")
        );
    }

    #[test]
    fn decision_audit_keeps_legacy_shadow_and_adds_phase1_fields() {
        let mut combat = blank_test_combat();
        combat.turn.energy = 1;
        combat.entities.player.current_hp = 5;
        combat
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));
        combat.zones.hand.push(CombatCard::new(CardId::Defend, 1));

        let diagnostics = diagnose_root_search_with_runtime(
            &EngineState::CombatPlayerTurn,
            &combat,
            300,
            SearchRuntimeBudget {
                exact_turn_node_budget: 128,
                ..SearchRuntimeBudget::default()
            },
        );

        assert!(diagnostics
            .decision_audit
            .get("exact_turn_shadow")
            .is_some());
        assert!(diagnostics.decision_audit.get("regime").is_some());
        assert!(diagnostics.decision_audit.get("frontier_outcome").is_some());
        assert!(diagnostics
            .decision_audit
            .get("exact_turn_verdict")
            .is_some());
        assert!(diagnostics.decision_audit.get("takeover_policy").is_some());
        assert!(diagnostics.decision_audit.get("decision_trace").is_some());
    }
}
