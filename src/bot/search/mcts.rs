use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;
use std::time::Instant;

use super::equivalence::{SearchEquivalenceKind, SearchEquivalenceMode};
use super::profile::{
    SearchProfileBreakdown, SearchProfileCollector, SearchProfilePhase, SearchProfilingLevel,
};
use super::root_policy::StatePressureFeatures;
use super::root_policy::{
    motif_transition_bonus, sequencing_order_bonus, tactical_hint_bonus, total_incoming_damage,
};
use super::root_prior::RootPriorConfig;
use super::root_rollout::{is_terminal, project_turn_close_state_profiled, total_enemy_hp};
use super::sequence_judge::{SequenceCandidate, SequenceJudge};
use super::{
    curiosity_archetype_move_bonus, default_equivalence_mode, get_legal_moves, reduce_search_moves,
    tactical_move_bonus,
};

#[derive(Clone, Debug)]
pub struct SearchMoveStat {
    pub input: ClientInput,
    pub visits: u32,
    pub avg_score: f32,
    pub base_order_score: f32,
    pub order_score: f32,
    pub root_prior_score: f32,
    pub root_prior_hit: bool,
    pub leaf_score: f32,
    pub policy_bonus: f32,
    pub sequence_bonus: f32,
    pub sequence_survival_bonus: f32,
    pub sequence_exhaust_bonus: f32,
    pub sequence_frontload_bonus: f32,
    pub sequence_defer_bonus: f32,
    pub sequence_branch_bonus: f32,
    pub sequence_downside_penalty: f32,
    pub projected_hp: i32,
    pub projected_block: i32,
    pub projected_enemy_total: i32,
    pub projected_unblocked: i32,
    pub survives: bool,
    pub realized_exhaust_block: i32,
    pub realized_exhaust_draw: i32,
    pub immediate_hp: i32,
    pub immediate_block: i32,
    pub immediate_energy: u8,
    pub immediate_hand_len: usize,
    pub immediate_draw_len: usize,
    pub immediate_discard_len: usize,
    pub immediate_exhaust_len: usize,
    pub immediate_incoming: i32,
    pub immediate_enemy_total: i32,
    pub cluster_id: String,
    pub cluster_size: usize,
    pub collapsed_inputs: Vec<ClientInput>,
    pub equivalence_kind: Option<SearchEquivalenceKind>,
}

#[derive(Clone, Debug)]
pub struct SearchDiagnostics {
    pub chosen_move: ClientInput,
    pub legal_moves: usize,
    pub reduced_legal_moves: usize,
    pub simulations: u32,
    pub elapsed_ms: u128,
    pub depth_limit: u32,
    pub max_decision_depth: usize,
    pub root_width: usize,
    pub branch_width: usize,
    pub max_engine_steps: usize,
    pub equivalence_mode: SearchEquivalenceMode,
    pub root_prior_enabled: bool,
    pub root_prior_key: Option<String>,
    pub root_prior_weight: f32,
    pub root_prior_hits: usize,
    pub root_prior_reordered: bool,
    pub top_moves: Vec<SearchMoveStat>,
    pub profile: SearchProfileBreakdown,
}

#[derive(Clone)]
struct RankedTransition {
    input: ClientInput,
    next_engine: EngineState,
    next_combat: CombatState,
    base_order_score: f32,
    order_score: f32,
    root_prior_score: f32,
    root_prior_hit: bool,
    leaf_score: f32,
    cluster_id: String,
    cluster_size: usize,
    collapsed_inputs: Vec<ClientInput>,
    equivalence_kind: Option<SearchEquivalenceKind>,
}

struct RankedTransitionsResult {
    transitions: Vec<RankedTransition>,
    root_prior_hits: usize,
    root_prior_reordered: bool,
}

pub fn find_best_move(
    engine: &EngineState,
    combat: &CombatState,
    depth_limit: u32,
    _verbose: bool,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
) -> ClientInput {
    find_best_move_with_mode_and_profiling(
        engine,
        combat,
        default_equivalence_mode(),
        depth_limit,
        _verbose,
        db,
        coverage_mode,
        curiosity_target,
        SearchProfilingLevel::Summary,
    )
}

pub fn find_best_move_with_mode(
    engine: &EngineState,
    combat: &CombatState,
    equivalence_mode: SearchEquivalenceMode,
    depth_limit: u32,
    _verbose: bool,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
) -> ClientInput {
    find_best_move_with_mode_and_profiling(
        engine,
        combat,
        equivalence_mode,
        depth_limit,
        _verbose,
        db,
        coverage_mode,
        curiosity_target,
        SearchProfilingLevel::Summary,
    )
}

pub fn find_best_move_with_mode_and_profiling(
    engine: &EngineState,
    combat: &CombatState,
    equivalence_mode: SearchEquivalenceMode,
    depth_limit: u32,
    _verbose: bool,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    profiling_level: SearchProfilingLevel,
) -> ClientInput {
    run_root_search(
        engine,
        combat,
        equivalence_mode,
        db,
        coverage_mode,
        curiosity_target,
        depth_limit,
        profiling_level,
        None,
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
    diagnose_root_search_with_mode_and_profiling(
        engine,
        combat,
        db,
        coverage_mode,
        curiosity_target,
        num_simulations,
        default_equivalence_mode(),
        SearchProfilingLevel::Summary,
    )
}

pub fn diagnose_root_search_with_mode(
    engine: &EngineState,
    combat: &CombatState,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    num_simulations: u32,
    equivalence_mode: SearchEquivalenceMode,
) -> SearchDiagnostics {
    diagnose_root_search_with_mode_and_profiling(
        engine,
        combat,
        db,
        coverage_mode,
        curiosity_target,
        num_simulations,
        equivalence_mode,
        SearchProfilingLevel::Summary,
    )
}

pub fn diagnose_root_search_with_mode_and_profiling(
    engine: &EngineState,
    combat: &CombatState,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    num_simulations: u32,
    equivalence_mode: SearchEquivalenceMode,
    profiling_level: SearchProfilingLevel,
) -> SearchDiagnostics {
    let depth_limit = search_depth_for_budget(num_simulations);
    run_root_search(
        engine,
        combat,
        equivalence_mode,
        db,
        coverage_mode,
        curiosity_target,
        depth_limit,
        profiling_level,
        None,
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
    diagnose_root_search_with_depth_and_mode_and_profiling(
        engine,
        combat,
        db,
        coverage_mode,
        curiosity_target,
        depth_limit,
        _num_simulations,
        default_equivalence_mode(),
        SearchProfilingLevel::Summary,
    )
}

pub fn diagnose_root_search_with_depth_and_mode(
    engine: &EngineState,
    combat: &CombatState,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    depth_limit: u32,
    _num_simulations: u32,
    equivalence_mode: SearchEquivalenceMode,
) -> SearchDiagnostics {
    diagnose_root_search_with_depth_and_mode_and_profiling(
        engine,
        combat,
        db,
        coverage_mode,
        curiosity_target,
        depth_limit,
        _num_simulations,
        equivalence_mode,
        SearchProfilingLevel::Summary,
    )
}

pub fn diagnose_root_search_with_depth_and_mode_and_profiling(
    engine: &EngineState,
    combat: &CombatState,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    depth_limit: u32,
    _num_simulations: u32,
    equivalence_mode: SearchEquivalenceMode,
    profiling_level: SearchProfilingLevel,
) -> SearchDiagnostics {
    run_root_search(
        engine,
        combat,
        equivalence_mode,
        db,
        coverage_mode,
        curiosity_target,
        depth_limit,
        profiling_level,
        None,
    )
}

pub fn diagnose_root_search_with_depth_and_mode_and_root_prior(
    engine: &EngineState,
    combat: &CombatState,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    depth_limit: u32,
    _num_simulations: u32,
    equivalence_mode: SearchEquivalenceMode,
    profiling_level: SearchProfilingLevel,
    root_prior: Option<&RootPriorConfig>,
) -> SearchDiagnostics {
    run_root_search(
        engine,
        combat,
        equivalence_mode,
        db,
        coverage_mode,
        curiosity_target,
        depth_limit,
        profiling_level,
        root_prior,
    )
}

fn run_root_search(
    engine: &EngineState,
    combat: &CombatState,
    equivalence_mode: SearchEquivalenceMode,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    depth_limit: u32,
    profiling_level: SearchProfilingLevel,
    root_prior: Option<&RootPriorConfig>,
) -> SearchDiagnostics {
    let started = Instant::now();
    let mut profiler = SearchProfileCollector::new(profiling_level);
    let legal_started = Instant::now();
    let legal_moves = get_legal_moves(engine, combat);
    profiler.record_legal_move_gen(
        SearchProfilePhase::Root,
        legal_started.elapsed().as_millis(),
        legal_moves.len(),
    );
    let (max_decision_depth, root_width, branch_width, max_engine_steps) =
        search_limits(depth_limit);
    if legal_moves.is_empty() {
        profiler.record_search_total_ms(started.elapsed().as_millis());
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
            profile: profiler.finish(),
        };
    }

    let ranked = rank_transitions(
        engine,
        combat,
        &legal_moves,
        equivalence_mode,
        db,
        coverage_mode,
        curiosity_target,
        max_engine_steps,
        &mut profiler,
        SearchProfilePhase::Root,
        root_prior,
    );
    let root_assessments =
        assess_root_moves(combat, &ranked.transitions, max_engine_steps, &mut profiler);

    let root_cut = root_width.max(1).min(ranked.transitions.len());
    let mut explored_nodes = 0;
    let mut top_moves = Vec::with_capacity(root_cut);

    for transition in ranked.transitions.iter().take(root_cut) {
        let leaf_after_move = transition.leaf_score;
        let policy_bonus = transition.order_score - leaf_after_move;
        let (score, explored) = search_from_state(
            &transition.next_engine,
            &transition.next_combat,
            max_decision_depth.saturating_sub(1),
            branch_width,
            max_engine_steps,
            db,
            coverage_mode,
            curiosity_target,
            equivalence_mode,
            &mut profiler,
        );
        explored_nodes += explored + 1;
        let assessment = root_assessments
            .iter()
            .find(|assessment| assessment.input == transition.input)
            .expect("root assessment for ranked transition");
        top_moves.push(SearchMoveStat {
            input: transition.input.clone(),
            visits: explored + 1,
            avg_score: score + policy_bonus,
            base_order_score: transition.base_order_score,
            order_score: transition.order_score,
            root_prior_score: transition.root_prior_score,
            root_prior_hit: transition.root_prior_hit,
            leaf_score: leaf_after_move,
            policy_bonus,
            sequence_bonus: 0.0,
            sequence_survival_bonus: 0.0,
            sequence_exhaust_bonus: 0.0,
            sequence_frontload_bonus: 0.0,
            sequence_defer_bonus: 0.0,
            sequence_branch_bonus: 0.0,
            sequence_downside_penalty: 0.0,
            projected_hp: assessment.projected_hp,
            projected_block: assessment.projected_block,
            projected_enemy_total: assessment.projected_enemy_total,
            projected_unblocked: assessment.projected_unblocked,
            survives: assessment.survives,
            realized_exhaust_block: 0,
            realized_exhaust_draw: 0,
            immediate_hp: transition.next_combat.entities.player.current_hp,
            immediate_block: transition.next_combat.entities.player.block,
            immediate_energy: transition.next_combat.turn.energy,
            immediate_hand_len: transition.next_combat.zones.hand.len(),
            immediate_draw_len: transition.next_combat.zones.draw_pile.len(),
            immediate_discard_len: transition.next_combat.zones.discard_pile.len(),
            immediate_exhaust_len: transition.next_combat.zones.exhaust_pile.len(),
            immediate_incoming: total_incoming_damage(&transition.next_combat),
            immediate_enemy_total: total_enemy_hp(&transition.next_combat),
            cluster_id: transition.cluster_id.clone(),
            cluster_size: transition.cluster_size,
            collapsed_inputs: transition.collapsed_inputs.clone(),
            equivalence_kind: transition.equivalence_kind,
        });
    }

    apply_sequence_judge(combat, &root_assessments, &mut top_moves, &mut profiler);

    top_moves.sort_by(|a, b| {
        b.avg_score
            .total_cmp(&a.avg_score)
            .then_with(|| b.visits.cmp(&a.visits))
            .then_with(|| end_turn_last(&a.input, &b.input))
    });

    let chosen_move = top_moves
        .first()
        .map(|stat| stat.input.clone())
        .unwrap_or_else(|| ranked.transitions[0].input.clone());

    profiler.record_search_total_ms(started.elapsed().as_millis());
    SearchDiagnostics {
        chosen_move,
        legal_moves: legal_moves.len(),
        reduced_legal_moves: ranked.transitions.len(),
        simulations: explored_nodes,
        elapsed_ms: started.elapsed().as_millis(),
        depth_limit,
        max_decision_depth,
        root_width,
        branch_width,
        max_engine_steps,
        equivalence_mode,
        root_prior_enabled: root_prior.is_some(),
        root_prior_key: root_prior.map(|config| config.key.as_string()),
        root_prior_weight: root_prior.map(|config| config.weight).unwrap_or(0.0),
        root_prior_hits: ranked.root_prior_hits,
        root_prior_reordered: ranked.root_prior_reordered,
        top_moves,
        profile: profiler.finish(),
    }
}

fn search_from_state(
    engine: &EngineState,
    combat: &CombatState,
    depth_left: usize,
    branch_width: usize,
    max_engine_steps: usize,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    equivalence_mode: SearchEquivalenceMode,
    profiler: &mut SearchProfileCollector,
) -> (f32, u32) {
    profiler.record_node_expanded();
    if is_terminal(engine, combat) {
        profiler.record_terminal_node();
        return (
            evaluate_leaf_profiled(engine, combat, profiler, SearchProfilePhase::Recursive),
            1,
        );
    }
    if depth_left == 0 {
        return (
            evaluate_leaf_profiled(engine, combat, profiler, SearchProfilePhase::Recursive),
            1,
        );
    }

    let legal_started = Instant::now();
    let legal_moves = get_legal_moves(engine, combat);
    profiler.record_legal_move_gen(
        SearchProfilePhase::Recursive,
        legal_started.elapsed().as_millis(),
        legal_moves.len(),
    );
    if legal_moves.is_empty() {
        return (
            evaluate_leaf_profiled(engine, combat, profiler, SearchProfilePhase::Recursive),
            1,
        );
    }

    let ranked = rank_transitions(
        engine,
        combat,
        &legal_moves,
        equivalence_mode,
        db,
        coverage_mode,
        curiosity_target,
        max_engine_steps,
        profiler,
        SearchProfilePhase::Recursive,
        None,
    );

    let mut best_score = f32::MIN;
    let mut explored = 0;
    let selective_branch_width = recursive_branch_width(combat, branch_width);

    for transition in ranked
        .transitions
        .iter()
        .take(selective_branch_width.max(1))
    {
        if should_skip_recursive_transition(combat, transition) {
            continue;
        }
        let (child_score, child_explored) = search_from_state(
            &transition.next_engine,
            &transition.next_combat,
            depth_left.saturating_sub(1),
            branch_width,
            max_engine_steps,
            db,
            coverage_mode,
            curiosity_target,
            equivalence_mode,
            profiler,
        );
        best_score = best_score.max(child_score);
        explored += child_explored + 1;
    }

    if best_score == f32::MIN {
        (
            evaluate_leaf_profiled(engine, combat, profiler, SearchProfilePhase::Recursive),
            explored.max(1),
        )
    } else {
        (best_score, explored.max(1))
    }
}

fn rank_transitions(
    engine: &EngineState,
    combat: &CombatState,
    legal_moves: &[ClientInput],
    equivalence_mode: SearchEquivalenceMode,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    max_engine_steps: usize,
    profiler: &mut SearchProfileCollector,
    phase: SearchProfilePhase,
    root_prior: Option<&RootPriorConfig>,
) -> RankedTransitionsResult {
    let tactical_hint = super::tactical_override(engine, combat).map(|choice| choice.input);
    let reduce_started = Instant::now();
    let reduced = reduce_search_moves(
        engine,
        combat,
        legal_moves,
        max_engine_steps,
        equivalence_mode,
        profiler,
        phase,
    );
    profiler.record_transition_reduce(
        phase,
        reduce_started.elapsed().as_millis(),
        legal_moves.len(),
        reduced.len(),
    );
    let mut ranked = reduced
        .into_iter()
        .map(|reduced| {
            let leaf_score =
                evaluate_leaf_profiled(&reduced.next_engine, &reduced.next_combat, profiler, phase);
            let mut base_order_score = leaf_score
                + transition_bonus(
                    engine,
                    combat,
                    &reduced.input,
                    &reduced.next_engine,
                    &reduced.next_combat,
                    db,
                    coverage_mode,
                    curiosity_target,
                );
            base_order_score += sequencing_order_bonus(
                combat,
                &reduced.input,
                StatePressureFeatures::from_combat(combat).value_unblocked <= 0,
            );
            if tactical_hint
                .as_ref()
                .is_some_and(|hint| *hint == reduced.input)
            {
                base_order_score += tactical_hint_bonus(combat, &reduced.input);
            }
            if matches!(reduced.input, ClientInput::EndTurn) {
                base_order_score -= 8_000.0;
            }
            let move_label = describe_root_prior_move_label(combat, &reduced.input);
            let root_prior_score = root_prior
                .and_then(|config| config.provider.score(&config.key, &move_label))
                .map(|score| score.aggregate_score)
                .unwrap_or(0.0);
            let root_prior_hit = root_prior
                .is_some_and(|config| config.provider.score(&config.key, &move_label).is_some());
            let order_score = base_order_score
                + root_prior.map(|config| config.weight).unwrap_or(0.0) * root_prior_score;

            RankedTransition {
                input: reduced.input,
                next_engine: reduced.next_engine,
                next_combat: reduced.next_combat,
                base_order_score,
                order_score,
                root_prior_score,
                root_prior_hit,
                leaf_score,
                cluster_id: reduced.cluster_id,
                cluster_size: reduced.cluster_size,
                collapsed_inputs: reduced.collapsed_inputs,
                equivalence_kind: reduced.equivalence_kind,
            }
        })
        .collect::<Vec<_>>();

    let mut base_sorted = ranked
        .iter()
        .map(|transition| (transition.input.clone(), transition.base_order_score))
        .collect::<Vec<_>>();
    base_sorted.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| end_turn_last(&a.0, &b.0)));
    ranked.sort_by(|a, b| {
        b.order_score
            .total_cmp(&a.order_score)
            .then_with(|| end_turn_last(&a.input, &b.input))
    });
    let root_prior_reordered = base_sorted
        .iter()
        .map(|(input, _)| input)
        .zip(ranked.iter().map(|transition| &transition.input))
        .any(|(left, right)| left != right);
    let root_prior_hits = ranked
        .iter()
        .filter(|transition| transition.root_prior_hit)
        .count();
    RankedTransitionsResult {
        transitions: ranked,
        root_prior_hits,
        root_prior_reordered,
    }
}

fn describe_root_prior_move_label(combat: &CombatState, input: &ClientInput) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card = combat
                .zones
                .hand
                .get(*card_index)
                .map(format_root_prior_card)
                .unwrap_or_else(|| format!("hand[{card_index}]"));
            match target {
                Some(target) => format!("Play #{} {card} @{target}", card_index + 1),
                None => format!("Play #{} {card}", card_index + 1),
            }
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => match target {
            Some(target) => format!("UsePotion#{potion_index} @{target}"),
            None => format!("UsePotion#{potion_index}"),
        },
        ClientInput::EndTurn => "EndTurn".to_string(),
        other => format!("{other:?}"),
    }
}

fn format_root_prior_card(card: &crate::runtime::combat::CombatCard) -> String {
    let mut label = crate::content::cards::get_card_definition(card.id)
        .name
        .to_string();
    if card.upgrades > 0 {
        label.push_str(&"+".repeat(card.upgrades as usize));
    }
    label
}

fn transition_bonus(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
    next_engine: &EngineState,
    next_combat: &CombatState,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
) -> f32 {
    let signature = crate::bot::coverage_signatures::signature_from_transition_with_archetypes(
        engine,
        combat,
        input,
        next_engine,
        next_combat,
        crate::bot::coverage::archetype_tags_for_combat(combat),
    );

    let mut bonus = tactical_move_bonus(combat, input);
    bonus += crate::bot::coverage::novelty_bonus(
        Some(signature.canonical_key().as_str()),
        Some(signature.source_combo_key().as_str()),
        db,
        coverage_mode,
    );
    bonus += crate::bot::coverage::curiosity_bonus(Some(&signature), curiosity_target);
    bonus += curiosity_archetype_move_bonus(combat, input, curiosity_target);
    bonus += motif_transition_bonus(combat, input, next_combat);

    if coverage_mode != crate::bot::coverage::CoverageMode::Off {
        match input {
            ClientInput::PlayCard { card_index, .. } => {
                if let Some(card) = combat.zones.hand.get(*card_index) {
                    let def = crate::content::cards::get_card_definition(card.id);
                    if db.is_card_untested(def.name) {
                        bonus += 5_000.0;
                    }
                }
            }
            ClientInput::UsePotion { potion_index, .. } => {
                if let Some(Some(potion)) = combat.entities.potions.get(*potion_index) {
                    let def = crate::content::potions::get_potion_definition(potion.id);
                    if db.is_potion_untested(def.name) {
                        bonus += 5_000.0;
                    }
                }
            }
            ClientInput::SubmitDiscoverChoice(_) | ClientInput::SubmitCardChoice(_) => {
                bonus += 2_500.0;
            }
            _ => {}
        }
    }

    bonus
}

fn evaluate_leaf(engine: &EngineState, combat: &CombatState) -> f32 {
    match engine {
        EngineState::GameOver(crate::state::core::RunResult::Defeat) => return -1_000_000.0,
        EngineState::GameOver(crate::state::core::RunResult::Victory) => return 1_000_000.0,
        _ => {}
    }

    let heuristic = crate::bot::combat_heuristic::evaluate_combat_state(combat) as f32;
    let strategic = crate::bot::evaluator::evaluate_state(engine, combat);
    heuristic + strategic * 0.15
}

fn evaluate_leaf_profiled(
    engine: &EngineState,
    combat: &CombatState,
    profiler: &mut SearchProfileCollector,
    phase: SearchProfilePhase,
) -> f32 {
    let started = Instant::now();
    let score = evaluate_leaf(engine, combat);
    profiler.record_leaf_eval(phase, started.elapsed().as_millis());
    score
}

fn assess_root_moves(
    combat: &CombatState,
    ranked: &[RankedTransition],
    max_engine_steps: usize,
    profiler: &mut SearchProfileCollector,
) -> Vec<SequenceCandidate> {
    ranked
        .iter()
        .map(|transition| {
            let passive = is_passive_transition(combat, transition);
            profiler.record_clone_calls(SearchProfilePhase::Root, 2);
            let (projected_engine, projected_combat) = project_turn_close_state_profiled(
                &transition.next_engine,
                &transition.next_combat,
                max_engine_steps,
                Some(profiler),
            );
            SequenceCandidate {
                input: transition.input.clone(),
                after_engine: transition.next_engine.clone(),
                after_combat: transition.next_combat.clone(),
                passive,
                projected_hp: projected_combat.entities.player.current_hp,
                projected_block: projected_combat.entities.player.block,
                projected_enemy_total: total_enemy_hp(&projected_combat),
                projected_unblocked: (total_incoming_damage(&projected_combat)
                    - projected_combat.entities.player.block)
                    .max(0),
                survives: !matches!(
                    projected_engine,
                    EngineState::GameOver(crate::state::core::RunResult::Defeat)
                ) && projected_combat.entities.player.current_hp > 0,
            }
        })
        .collect()
}

fn apply_sequence_judge(
    combat: &CombatState,
    candidates: &[SequenceCandidate],
    top_moves: &mut [SearchMoveStat],
    profiler: &mut SearchProfileCollector,
) {
    let started = Instant::now();
    for adjustment in SequenceJudge::new(combat, candidates).adjustments() {
        let Some(stat) = top_moves
            .iter_mut()
            .find(|stat| stat.input == adjustment.input)
        else {
            continue;
        };
        stat.sequence_bonus += adjustment.total_delta;
        stat.sequence_survival_bonus += adjustment.survival_window_delta;
        stat.sequence_exhaust_bonus += adjustment.exhaust_evidence_delta;
        stat.sequence_frontload_bonus += adjustment.frontload_delta;
        stat.sequence_defer_bonus += adjustment.defer_delta;
        stat.sequence_branch_bonus += adjustment.branch_opening_delta;
        stat.sequence_downside_penalty += adjustment.downside_penalty_delta;
        stat.avg_score += adjustment.total_delta;
        stat.realized_exhaust_block = adjustment.realized_exhaust_block;
        stat.realized_exhaust_draw = adjustment.realized_exhaust_draw;
    }
    profiler.record_sequence_judge_ms(started.elapsed().as_millis());
}

fn is_passive_transition(combat: &CombatState, transition: &RankedTransition) -> bool {
    if matches!(transition.input, ClientInput::EndTurn) {
        return true;
    }

    let before_enemy_total = total_enemy_hp(combat);
    let after_enemy_total = total_enemy_hp(&transition.next_combat);
    let player = &combat.entities.player;
    let next_player = &transition.next_combat.entities.player;

    after_enemy_total >= before_enemy_total
        && next_player.block <= player.block
        && next_player.current_hp <= player.current_hp
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

fn end_turn_last(left: &ClientInput, right: &ClientInput) -> std::cmp::Ordering {
    match (
        matches!(left, ClientInput::EndTurn),
        matches!(right, ClientInput::EndTurn),
    ) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        _ => std::cmp::Ordering::Equal,
    }
}

fn recursive_branch_width(combat: &CombatState, branch_width: usize) -> usize {
    let incoming = total_incoming_damage(combat);
    let unblocked = (incoming - combat.entities.player.block).max(0);
    if unblocked > 0 || combat.meta.is_elite_fight {
        branch_width.max(1)
    } else {
        branch_width.saturating_sub(1).max(1)
    }
}

fn should_skip_recursive_transition(combat: &CombatState, transition: &RankedTransition) -> bool {
    if matches!(transition.input, ClientInput::EndTurn) {
        return false;
    }
    let tags = super::root_policy::action_semantic_tags(combat, &transition.input);
    if tags.persistent_setup
        || tags.defensive_potion
        || tags.block_core
        || tags.resource_bridge
        || tags.attack_like
    {
        return false;
    }

    let before_enemy_total = total_enemy_hp(combat);
    let after_enemy_total = total_enemy_hp(&transition.next_combat);
    let before_incoming = total_incoming_damage(combat);
    let after_incoming = total_incoming_damage(&transition.next_combat);
    let player = &combat.entities.player;
    let next_player = &transition.next_combat.entities.player;

    after_enemy_total >= before_enemy_total
        && after_incoming >= before_incoming
        && next_player.block <= player.block
        && next_player.current_hp <= player.current_hp
}
