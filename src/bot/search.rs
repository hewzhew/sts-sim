use crate::state::core::{ClientInput, PendingChoice};
use crate::state::EngineState;
use crate::combat::CombatState;

struct MctsNode {
    visits: u32,
    total_score: f32, // Based on evaluations of children/leaves
    unexplored_moves: Vec<ClientInput>,
    children: Vec<(ClientInput, usize)>, // Child node indexes in Arena
}

struct MctsArena {
    nodes: Vec<MctsNode>,
}

impl MctsArena {
    fn new() -> Self {
        Self { nodes: Vec::with_capacity(4096) }
    }
    
    fn new_node(&mut self, engine: &EngineState, combat: &CombatState) -> usize {
        let node = MctsNode {
            visits: 0,
            total_score: 0.0,
            unexplored_moves: get_legal_moves(engine, combat),
            children: Vec::new(),
        };
        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }
}

/// Finds the optimum full-turn sequential moves via Heuristic MCTS with Smart Playout Rollouts.
pub fn find_best_move(engine: &EngineState, combat: &CombatState, _depth_limit: u32, verbose: bool, db: &crate::bot::coverage::CoverageDb) -> ClientInput {
    let mut arena = MctsArena::new();
    let root_idx = arena.new_node(engine, combat);
    
    // Fallback if no moves
    if arena.nodes[root_idx].unexplored_moves.is_empty() {
        return ClientInput::EndTurn;
    }

    let num_simulations = 4000;
    for _ in 0..num_simulations {
        let mut sim_engine = engine.clone();
        let mut sim_combat = combat.clone();
        mcts_iteration(&mut arena, root_idx, &mut sim_engine, &mut sim_combat, db);
    }

    // Pick highest visit child (most robust path)
    let mut best_move = None;
    let mut best_visits = 0;
    
    if verbose {
        println!("MCTS Root stats ({} sims):", num_simulations);
    }
    for &(ref m, child_idx) in &arena.nodes[root_idx].children {
        let child = &arena.nodes[child_idx];
        if verbose {
            println!("  Move: {:?} | Visits: {} | Avg Score: {}", m, child.visits, child.total_score / (child.visits as f32).max(1.0));
        }
        if child.visits > best_visits {
            best_visits = child.visits;
            best_move = Some(m.clone());
        }
    }
    
    best_move.unwrap_or_else(|| {
        let legal = get_legal_moves(engine, combat);
        // pop() would give the last element which is the most desirable
        legal.last().cloned().unwrap_or(ClientInput::EndTurn)
    })
}



fn mcts_iteration(arena: &mut MctsArena, node_idx: usize, engine: &mut EngineState, combat: &mut CombatState, db: &crate::bot::coverage::CoverageDb) -> f32 {
    // 1. Check Death / Victory directly to halt early
    if combat.player.current_hp <= 0 || matches!(engine, EngineState::GameOver(crate::state::core::RunResult::Defeat)) {
        let score = -999999.0;
        arena.nodes[node_idx].visits += 1;
        arena.nodes[node_idx].total_score += score;
        return score;
    }
    if combat.monsters.iter().all(|m| m.is_dying || m.is_escaped) || matches!(engine, EngineState::GameOver(crate::state::core::RunResult::Victory)) {
        let base_eval = crate::bot::evaluator::evaluate_state(engine, combat);
        let score = base_eval + 50000.0;
        arena.nodes[node_idx].visits += 1;
        arena.nodes[node_idx].total_score += score;
        return score;
    }

    // 2. Expand
    if let Some(move_to_try) = arena.nodes[node_idx].unexplored_moves.pop() {
        // Execute move and fast-forward to next decision point
        let mut first = true;
        let mut safety_counter = 0;
        loop {
            safety_counter += 1;
            if safety_counter > 100 { break; }
            let input_val = if first { Some(move_to_try.clone()) } else { None };
            first = false;
            let alive = crate::engine::core::tick_engine(engine, combat, input_val);
            if !alive || matches!(engine, EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) | EngineState::GameOver(_)) {
                break;
            }
        }
        
        let new_child_idx = arena.new_node(engine, combat);
        
        // --- STATIC HEURISTIC EVALUATION ---
        let score = crate::bot::evaluator::evaluate_state(engine, combat);
        
        // Add child link
        arena.nodes[node_idx].children.push((move_to_try, new_child_idx));
        
        // Update leaf
        arena.nodes[new_child_idx].visits += 1;
        arena.nodes[new_child_idx].total_score += score;

        // Update current
        arena.nodes[node_idx].visits += 1;
        arena.nodes[node_idx].total_score += score;
        return score;
    }

    // 3. Selection
    if arena.nodes[node_idx].children.is_empty() {
        // Fully expanded but no children? Terminal state lacking moves
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
            f32::MAX // explore unvisited
        } else {
            q + exploration_c * f32::sqrt(ln_n / child_visits)
        };
        
        let mut coverage_boost = 0.0;
        let chosen_move = &arena.nodes[node_idx].children[i].0;
        match chosen_move {
            ClientInput::PlayCard { card_index, .. } => {
                if let Some(card) = combat.hand.get(*card_index) {
                    let def = crate::content::cards::get_card_definition(card.id);
                    if db.is_card_untested(def.name) {
                        coverage_boost += 100000.0;
                    }
                }
            },
            ClientInput::UsePotion { potion_index, .. } => {
                if let Some(Some(p)) = combat.potions.get(*potion_index) {
                    let def = crate::content::potions::get_potion_definition(p.id);
                    if db.is_potion_untested(def.name) {
                        coverage_boost += 100000.0;
                    }
                }
            },
            ClientInput::SubmitDiscoverChoice(_) | ClientInput::SubmitCardChoice(_) => {
                coverage_boost += 50000.0; // highly encourage digging into pending choices
            },
            _ => {}
        }

        let final_ucb = if ucb == f32::MAX { f32::MAX } else { ucb + coverage_boost };

        if final_ucb > best_ucb {
            best_ucb = final_ucb;
            best_idx = i;
        }
    }

    let (chosen_move, child_idx) = arena.nodes[node_idx].children[best_idx].clone();
    
    // Apply selected move
    let mut first = true;
    let mut safety_counter = 0;
    loop {
        safety_counter += 1;
        if safety_counter > 100 { break; }
        
        let input_val = if first { Some(chosen_move.clone()) } else { None };
        first = false;
        let alive = crate::engine::core::tick_engine(engine, combat, input_val);
        if !alive || matches!(engine, EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) | EngineState::GameOver(_)) {
            break;
        }
    }

    // Recursively evaluate
    let score = mcts_iteration(arena, child_idx, engine, combat, db);
    
    arena.nodes[node_idx].visits += 1;
    arena.nodes[node_idx].total_score += score;
    score
}

/// Enumerates all currently legal inputs based on game state.
fn get_legal_moves(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    let mut moves = Vec::new();

    match engine {
        EngineState::CombatPlayerTurn => {
            // Push EndTurn FIRST so it is popped LAST.
            moves.push(ClientInput::EndTurn);
            
            // 2. Use Potions
            let hp_percent = (combat.player.current_hp * 100) / combat.player.max_hp.max(1);
            let mut expected_inc_damage = 0;
            for m in &combat.monsters {
                if !m.is_dying && !m.is_escaped && !m.half_dead {
                    match m.current_intent {
                        crate::combat::Intent::Attack { hits, .. } |
                        crate::combat::Intent::AttackBuff { hits, .. } |
                        crate::combat::Intent::AttackDebuff { hits, .. } |
                        crate::combat::Intent::AttackDefend { hits, .. } => {
                            expected_inc_damage += (m.intent_dmg * (hits as i32)).max(0);
                        },
                        _ => {}
                    }
                }
            }
            let unblocked = expected_inc_damage - combat.player.block as i32;
            
            // --- POTION HEURISTICS (based on bottled_ai) ---
            let mut potion_allowed = false;
            
            // 1. Bottled AI specific logic: Boss fights Turn 1, Elite/Event Turn 1 if HP <= 50%
            if combat.is_boss_fight && combat.turn_count == 1 {
                potion_allowed = true;
            } else if combat.is_elite_fight && hp_percent <= 50 && combat.turn_count == 1 {
                potion_allowed = true;
            }
            
            // 2. Overarching Survival Guard: Taking heavy damage
            if unblocked >= 15 || (hp_percent <= 30 && expected_inc_damage > 0) {
                potion_allowed = true;
            }

            if potion_allowed {
                for (i, opt_p) in combat.potions.iter().enumerate() {
                    if let Some(p) = opt_p {
                        use crate::content::potions::PotionId;
                        if p.id == PotionId::SmokeBomb && combat.is_boss_fight {
                            continue; // Unusable
                        }
                        let def = crate::content::potions::get_potion_definition(p.id);
                        if def.target_required {
                            for m in &combat.monsters {
                                if !m.is_dying && !m.is_escaped && !m.half_dead {
                                    moves.push(ClientInput::UsePotion { potion_index: i, target: Some(m.id) });
                                }
                            }
                        } else {
                            moves.push(ClientInput::UsePotion { potion_index: i, target: None });
                        }
                    }
                }
            }
            
            // 1. Play Cards
            // Passed in reverse so that "good" cards get pushed last and popped first theoretically 
            // but for now any card is popped before EndTurn.
            for (i, card) in combat.hand.iter().enumerate() {
                if crate::content::cards::can_play_card(card, combat).is_ok() {
                    let target_type = crate::content::cards::get_card_definition(card.id).target;
                    if target_type == crate::content::cards::CardTarget::Enemy {
                        for m in &combat.monsters {
                            if !m.is_dying && !m.is_escaped && !m.half_dead {
                                moves.push(ClientInput::PlayCard { card_index: i, target: Some(m.id) });
                            }
                        }
                    } else {
                        moves.push(ClientInput::PlayCard { card_index: i, target: None });
                    }
                }
            }
        },
        EngineState::PendingChoice(choice) => {
            match choice {
                PendingChoice::HandSelect { min_cards, .. } => {
                    // Combinations
                    if *min_cards == 1 {
                        for c in &combat.hand {
                            moves.push(ClientInput::SubmitHandSelect(vec![c.uuid]));
                        }
                    } else if *min_cards > 1 {
                        let mut selected_uuids = Vec::new();
                        for i in 0..*min_cards {
                            if (i as usize) < combat.hand.len() {
                                selected_uuids.push(combat.hand[i as usize].uuid);
                            }
                        }
                        moves.push(ClientInput::SubmitHandSelect(selected_uuids));
                    } else {
                        moves.push(ClientInput::SubmitHandSelect(Vec::new()));
                    }
                },
                PendingChoice::GridSelect { min_cards, .. } => {
                    // Try empty first
                    if *min_cards == 0 {
                        moves.push(ClientInput::SubmitGridSelect(Vec::new()));
                    }
                    // Generic fallback
                    let mut selected = Vec::new();
                    for i in 0..*min_cards {
                        if (i as usize) < combat.discard_pile.len() {
                            selected.push(combat.discard_pile[i as usize].uuid);
                        } else if (i as usize) < combat.draw_pile.len() {
                            selected.push(combat.draw_pile[i as usize].uuid);
                        }
                    }
                    moves.push(ClientInput::SubmitGridSelect(selected));
                },
                PendingChoice::DiscoverySelect(_) => {
                    moves.push(ClientInput::SubmitDiscoverChoice(0));
                    moves.push(ClientInput::SubmitDiscoverChoice(1));
                    moves.push(ClientInput::SubmitDiscoverChoice(2));
                },
                PendingChoice::CardRewardSelect { .. } => {
                    moves.push(ClientInput::SubmitCardChoice(vec![0]));
                },
                PendingChoice::StanceChoice => {
                    moves.push(ClientInput::Proceed);
                },
                _ => {
                    moves.push(ClientInput::Proceed);
                }
            }
        },
        _ => {
            moves.push(ClientInput::Proceed);
        }
    }

    moves
}
