use crate::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::{curiosity_archetype_move_bonus, get_legal_moves, tactical_move_bonus};

struct MctsNode {
    visits: u32,
    total_score: f32,
    unexplored_moves: Vec<ClientInput>,
    children: Vec<(ClientInput, usize)>,
    signature: Option<crate::interaction_coverage::InteractionSignature>,
    signature_key: Option<String>,
    source_combo_key: Option<String>,
}

struct MctsArena {
    nodes: Vec<MctsNode>,
}

impl MctsArena {
    fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(4096),
        }
    }

    fn new_node(
        &mut self,
        engine: &EngineState,
        combat: &CombatState,
        signature: Option<crate::interaction_coverage::InteractionSignature>,
        signature_key: Option<String>,
        source_combo_key: Option<String>,
    ) -> usize {
        let node = MctsNode {
            visits: 0,
            total_score: 0.0,
            unexplored_moves: get_legal_moves(engine, combat),
            children: Vec::new(),
            signature,
            signature_key,
            source_combo_key,
        };
        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }
}

pub fn find_best_move(
    engine: &EngineState,
    combat: &CombatState,
    _depth_limit: u32,
    verbose: bool,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
) -> ClientInput {
    let mut arena = MctsArena::new();
    let root_idx = arena.new_node(engine, combat, None, None, None);

    if arena.nodes[root_idx].unexplored_moves.is_empty() {
        return ClientInput::EndTurn;
    }

    let num_simulations = 4000;
    for _ in 0..num_simulations {
        let mut sim_engine = engine.clone();
        let mut sim_combat = combat.clone();
        mcts_iteration(
            &mut arena,
            root_idx,
            &mut sim_engine,
            &mut sim_combat,
            db,
            coverage_mode,
            curiosity_target,
        );
    }

    let mut best_move = None;
    let mut best_visits = 0;

    if verbose {
        eprintln!("MCTS Root stats ({} sims):", num_simulations);
    }
    for &(ref m, child_idx) in &arena.nodes[root_idx].children {
        let child = &arena.nodes[child_idx];
        if verbose {
            eprintln!(
                "  Move: {:?} | Visits: {} | Avg Score: {}",
                m,
                child.visits,
                child.total_score / (child.visits as f32).max(1.0)
            );
        }
        if child.visits > best_visits {
            best_visits = child.visits;
            best_move = Some(m.clone());
        }
    }

    best_move.unwrap_or_else(|| {
        let legal = get_legal_moves(engine, combat);
        legal.last().cloned().unwrap_or(ClientInput::EndTurn)
    })
}

fn mcts_iteration(
    arena: &mut MctsArena,
    node_idx: usize,
    engine: &mut EngineState,
    combat: &mut CombatState,
    db: &crate::bot::coverage::CoverageDb,
    coverage_mode: crate::bot::coverage::CoverageMode,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
) -> f32 {
    if combat.entities.player.current_hp <= 0
        || matches!(
            engine,
            EngineState::GameOver(crate::state::core::RunResult::Defeat)
        )
    {
        let score = -999999.0;
        arena.nodes[node_idx].visits += 1;
        arena.nodes[node_idx].total_score += score;
        return score;
    }
    if combat
        .entities
        .monsters
        .iter()
        .all(|m| m.is_dying || m.is_escaped)
        || matches!(
            engine,
            EngineState::GameOver(crate::state::core::RunResult::Victory)
        )
    {
        let base_eval = crate::bot::evaluator::evaluate_state(engine, combat);
        let score = base_eval + 50000.0;
        arena.nodes[node_idx].visits += 1;
        arena.nodes[node_idx].total_score += score;
        return score;
    }

    if let Some(move_to_try) = arena.nodes[node_idx].unexplored_moves.pop() {
        let before_engine = engine.clone();
        let before_combat = combat.clone();
        let mut first = true;
        let mut safety_counter = 0;
        loop {
            safety_counter += 1;
            if safety_counter > 100 {
                break;
            }
            let input_val = if first {
                Some(move_to_try.clone())
            } else {
                None
            };
            first = false;
            let alive = crate::engine::core::tick_engine(engine, combat, input_val);
            if !alive
                || matches!(
                    engine,
                    EngineState::CombatPlayerTurn
                        | EngineState::PendingChoice(_)
                        | EngineState::GameOver(_)
                )
            {
                break;
            }
        }

        let signature = crate::interaction_coverage::signature_from_transition(
            &before_engine,
            &before_combat,
            &move_to_try,
            engine,
            combat,
        );
        let new_child_idx = arena.new_node(
            engine,
            combat,
            Some(signature.clone()),
            Some(signature.canonical_key()),
            Some(signature.source_combo_key()),
        );

        let score = crate::bot::evaluator::evaluate_state(engine, combat);

        arena.nodes[node_idx]
            .children
            .push((move_to_try, new_child_idx));

        arena.nodes[new_child_idx].visits += 1;
        arena.nodes[new_child_idx].total_score += score;

        arena.nodes[node_idx].visits += 1;
        arena.nodes[node_idx].total_score += score;
        return score;
    }

    if arena.nodes[node_idx].children.is_empty() {
        let score = crate::bot::evaluator::evaluate_state(engine, combat);
        arena.nodes[node_idx].visits += 1;
        arena.nodes[node_idx].total_score += score;
        return score;
    }

    let mut best_idx = 0;
    let mut best_ucb = f32::MIN;
    let exploration_c = 150.0;

    let n_visits = arena.nodes[node_idx].visits as f32;
    let ln_n = f32::ln(n_visits.max(1.0));

    for (i, &(_, child_node_idx)) in arena.nodes[node_idx].children.iter().enumerate() {
        let child_visits = arena.nodes[child_node_idx].visits as f32;
        let q = arena.nodes[child_node_idx].total_score / child_visits.max(1.0);

        let ucb = if child_visits == 0.0 {
            f32::MAX
        } else {
            q + exploration_c * f32::sqrt(ln_n / child_visits)
        };

        let mut coverage_boost = crate::interaction_coverage::novelty_bonus(
            arena.nodes[child_node_idx].signature_key.as_deref(),
            arena.nodes[child_node_idx].source_combo_key.as_deref(),
            db,
            coverage_mode,
        );
        coverage_boost += crate::interaction_coverage::curiosity_bonus(
            arena.nodes[child_node_idx].signature.as_ref(),
            curiosity_target,
        );
        let chosen_move = &arena.nodes[node_idx].children[i].0;
        coverage_boost += curiosity_archetype_move_bonus(combat, chosen_move, curiosity_target);
        coverage_boost += tactical_move_bonus(combat, chosen_move);
        if coverage_mode != crate::bot::coverage::CoverageMode::Off {
            match chosen_move {
                ClientInput::PlayCard { card_index, .. } => {
                    if let Some(card) = combat.zones.hand.get(*card_index) {
                        let def = crate::content::cards::get_card_definition(card.id);
                        if db.is_card_untested(def.name) {
                            coverage_boost += 5_000.0;
                        }
                    }
                }
                ClientInput::UsePotion { potion_index, .. } => {
                    if let Some(Some(p)) = combat.entities.potions.get(*potion_index) {
                        let def = crate::content::potions::get_potion_definition(p.id);
                        if db.is_potion_untested(def.name) {
                            coverage_boost += 5_000.0;
                        }
                    }
                }
                ClientInput::SubmitDiscoverChoice(_) | ClientInput::SubmitCardChoice(_) => {
                    coverage_boost += 2_500.0;
                }
                _ => {}
            }
        }

        let final_ucb = if ucb == f32::MAX {
            f32::MAX
        } else {
            ucb + coverage_boost
        };

        if final_ucb > best_ucb {
            best_ucb = final_ucb;
            best_idx = i;
        }
    }

    let (chosen_move, child_idx) = arena.nodes[node_idx].children[best_idx].clone();

    let mut first = true;
    let mut safety_counter = 0;
    loop {
        safety_counter += 1;
        if safety_counter > 100 {
            break;
        }

        let input_val = if first {
            Some(chosen_move.clone())
        } else {
            None
        };
        first = false;
        let alive = crate::engine::core::tick_engine(engine, combat, input_val);
        if !alive
            || matches!(
                engine,
                EngineState::CombatPlayerTurn
                    | EngineState::PendingChoice(_)
                    | EngineState::GameOver(_)
            )
        {
            break;
        }
    }

    let score = mcts_iteration(
        arena,
        child_idx,
        engine,
        combat,
        db,
        coverage_mode,
        curiosity_target,
    );

    arena.nodes[node_idx].visits += 1;
    arena.nodes[node_idx].total_score += score;
    score
}
