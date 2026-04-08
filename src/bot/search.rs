use crate::combat::CombatState;
use crate::combat::PowerId;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::monsters::EnemyId;
use crate::engine::targeting;
use crate::state::core::{ClientInput, PendingChoice};
use crate::state::EngineState;

struct MctsNode {
    visits: u32,
    total_score: f32, // Based on evaluations of children/leaves
    unexplored_moves: Vec<ClientInput>,
    children: Vec<(ClientInput, usize)>, // Child node indexes in Arena
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

/// Finds the optimum full-turn sequential moves via Heuristic MCTS with Smart Playout Rollouts.
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

    // Fallback if no moves
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

    // Pick highest visit child (most robust path)
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
        // pop() would give the last element which is the most desirable
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
    // 1. Check Death / Victory directly to halt early
    if combat.player.current_hp <= 0
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
    if combat.monsters.iter().all(|m| m.is_dying || m.is_escaped)
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

    // 2. Expand
    if let Some(move_to_try) = arena.nodes[node_idx].unexplored_moves.pop() {
        let before_engine = engine.clone();
        let before_combat = combat.clone();
        // Execute move and fast-forward to next decision point
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

        // --- STATIC HEURISTIC EVALUATION ---
        let score = crate::bot::evaluator::evaluate_state(engine, combat);

        // Add child link
        arena.nodes[node_idx]
            .children
            .push((move_to_try, new_child_idx));

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
                    if let Some(card) = combat.hand.get(*card_index) {
                        let def = crate::content::cards::get_card_definition(card.id);
                        if db.is_card_untested(def.name) {
                            coverage_boost += 5_000.0;
                        }
                    }
                }
                ClientInput::UsePotion { potion_index, .. } => {
                    if let Some(Some(p)) = combat.potions.get(*potion_index) {
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

    // Apply selected move
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

    // Recursively evaluate
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
                        crate::combat::Intent::Attack { hits, .. }
                        | crate::combat::Intent::AttackBuff { hits, .. }
                        | crate::combat::Intent::AttackDebuff { hits, .. }
                        | crate::combat::Intent::AttackDefend { hits, .. } => {
                            expected_inc_damage += (m.intent_dmg * (hits as i32)).max(0);
                        }
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
                        if let Some(validation) =
                            targeting::validation_for_potion_target(def.target_required)
                        {
                            for target in targeting::candidate_targets(combat, validation) {
                                moves.push(ClientInput::UsePotion {
                                    potion_index: i,
                                    target: Some(target),
                                });
                            }
                        } else {
                            moves.push(ClientInput::UsePotion {
                                potion_index: i,
                                target: None,
                            });
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
                    if let Some(validation) = targeting::validation_for_card_target(target_type) {
                        for target in targeting::candidate_targets(combat, validation) {
                            moves.push(ClientInput::PlayCard {
                                card_index: i,
                                target: Some(target),
                            });
                        }
                    } else {
                        moves.push(ClientInput::PlayCard {
                            card_index: i,
                            target: None,
                        });
                    }
                }
            }
        }
        EngineState::PendingChoice(choice) => {
            match choice {
                PendingChoice::HandSelect {
                    min_cards,
                    candidate_uuids,
                    ..
                } => {
                    // Combinations
                    if *min_cards == 1 {
                        for &uuid in candidate_uuids {
                            moves.push(ClientInput::SubmitHandSelect(vec![uuid]));
                        }
                    } else if *min_cards > 1 {
                        let mut selected_uuids = Vec::new();
                        for i in 0..*min_cards {
                            if let Some(&uuid) = candidate_uuids.get(i as usize) {
                                selected_uuids.push(uuid);
                            }
                        }
                        moves.push(ClientInput::SubmitHandSelect(selected_uuids));
                    } else {
                        moves.push(ClientInput::SubmitHandSelect(Vec::new()));
                    }
                }
                PendingChoice::GridSelect {
                    min_cards,
                    candidate_uuids,
                    ..
                } => {
                    // Try empty first
                    if *min_cards == 0 {
                        moves.push(ClientInput::SubmitGridSelect(Vec::new()));
                    }
                    // Generic fallback
                    let mut selected = Vec::new();
                    for i in 0..*min_cards {
                        if let Some(&uuid) = candidate_uuids.get(i as usize) {
                            selected.push(uuid);
                        }
                    }
                    moves.push(ClientInput::SubmitGridSelect(selected));
                }
                PendingChoice::DiscoverySelect(_) => {
                    moves.push(ClientInput::SubmitDiscoverChoice(0));
                    moves.push(ClientInput::SubmitDiscoverChoice(1));
                    moves.push(ClientInput::SubmitDiscoverChoice(2));
                }
                PendingChoice::CardRewardSelect { .. } => {
                    moves.push(ClientInput::SubmitCardChoice(vec![0]));
                }
                PendingChoice::StanceChoice => {
                    moves.push(ClientInput::Proceed);
                }
                _ => {
                    moves.push(ClientInput::Proceed);
                }
            }
        }
        _ => {
            moves.push(ClientInput::Proceed);
        }
    }

    moves
}

fn curiosity_archetype_move_bonus(
    combat: &CombatState,
    chosen_move: &ClientInput,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
) -> f32 {
    let target = match curiosity_target {
        Some(crate::bot::coverage::CuriosityTarget::Archetype(target)) => normalize_name(target),
        _ => return 0.0,
    };
    let profile = crate::bot::evaluator::CardEvaluator::combat_profile(combat);

    match chosen_move {
        ClientInput::PlayCard { card_index, .. } => {
            let Some(card) = combat.hand.get(*card_index) else {
                return 0.0;
            };
            let card_id = card.id;
            match target.as_str() {
                "strength" => match card_id {
                    crate::content::cards::CardId::LimitBreak => {
                        if profile.strength_enablers > 0 {
                            12_000.0
                        } else {
                            2_000.0
                        }
                    }
                    crate::content::cards::CardId::Inflame
                    | crate::content::cards::CardId::SpotWeakness
                    | crate::content::cards::CardId::DemonForm => 9_000.0,
                    crate::content::cards::CardId::HeavyBlade
                    | crate::content::cards::CardId::SwordBoomerang
                    | crate::content::cards::CardId::Pummel
                    | crate::content::cards::CardId::Whirlwind
                    | crate::content::cards::CardId::TwinStrike => {
                        if profile.strength_enablers > 0 {
                            6_500.0
                        } else {
                            2_500.0
                        }
                    }
                    crate::content::cards::CardId::Flex => 5_000.0,
                    crate::content::cards::CardId::Panacea => 3_000.0,
                    _ => 0.0,
                },
                "exhaust" => match card_id {
                    crate::content::cards::CardId::Corruption
                    | crate::content::cards::CardId::FeelNoPain
                    | crate::content::cards::CardId::DarkEmbrace => 10_000.0,
                    crate::content::cards::CardId::SecondWind
                    | crate::content::cards::CardId::BurningPact
                    | crate::content::cards::CardId::TrueGrit
                    | crate::content::cards::CardId::SeverSoul
                    | crate::content::cards::CardId::FiendFire => {
                        if profile.exhaust_engines > 0 {
                            7_500.0
                        } else {
                            3_000.0
                        }
                    }
                    _ => 0.0,
                },
                "block" => match card_id {
                    crate::content::cards::CardId::Barricade
                    | crate::content::cards::CardId::Entrench => 10_000.0,
                    crate::content::cards::CardId::BodySlam
                    | crate::content::cards::CardId::FlameBarrier
                    | crate::content::cards::CardId::Impervious
                    | crate::content::cards::CardId::ShrugItOff => {
                        if profile.block_core > 0 {
                            7_000.0
                        } else {
                            3_000.0
                        }
                    }
                    crate::content::cards::CardId::Juggernaut => 5_000.0,
                    _ => 0.0,
                },
                "selfdamage" => match card_id {
                    crate::content::cards::CardId::Rupture => 9_000.0,
                    crate::content::cards::CardId::Offering
                    | crate::content::cards::CardId::Bloodletting
                    | crate::content::cards::CardId::Hemokinesis
                    | crate::content::cards::CardId::Combust
                    | crate::content::cards::CardId::Brutality => 6_500.0,
                    _ => 0.0,
                },
                "drawcycle" => match card_id {
                    crate::content::cards::CardId::BattleTrance
                    | crate::content::cards::CardId::PommelStrike
                    | crate::content::cards::CardId::ShrugItOff
                    | crate::content::cards::CardId::Finesse
                    | crate::content::cards::CardId::FlashOfSteel
                    | crate::content::cards::CardId::MasterOfStrategy => 7_000.0,
                    crate::content::cards::CardId::BurningPact
                    | crate::content::cards::CardId::Offering
                    | crate::content::cards::CardId::Brutality => 5_000.0,
                    _ => 0.0,
                },
                "powerscaling" => match card_id {
                    crate::content::cards::CardId::DemonForm
                    | crate::content::cards::CardId::Corruption
                    | crate::content::cards::CardId::FeelNoPain
                    | crate::content::cards::CardId::DarkEmbrace
                    | crate::content::cards::CardId::Barricade
                    | crate::content::cards::CardId::Juggernaut
                    | crate::content::cards::CardId::Evolve
                    | crate::content::cards::CardId::FireBreathing
                    | crate::content::cards::CardId::Panache
                    | crate::content::cards::CardId::Mayhem
                    | crate::content::cards::CardId::Magnetism => 8_500.0,
                    _ => 0.0,
                },
                "status" => match card_id {
                    crate::content::cards::CardId::Evolve
                    | crate::content::cards::CardId::FireBreathing => 9_000.0,
                    crate::content::cards::CardId::WildStrike
                    | crate::content::cards::CardId::RecklessCharge
                    | crate::content::cards::CardId::PowerThrough => 4_500.0,
                    _ => 0.0,
                },
                _ => 0.0,
            }
        }
        ClientInput::UsePotion { potion_index, .. } => {
            let Some(Some(potion)) = combat.potions.get(*potion_index) else {
                return 0.0;
            };
            match target.as_str() {
                "strength" => match potion.id {
                    crate::content::potions::PotionId::StrengthPotion
                    | crate::content::potions::PotionId::SteroidPotion
                    | crate::content::potions::PotionId::AncientPotion => 6_000.0,
                    _ => 0.0,
                },
                "exhaust" => match potion.id {
                    crate::content::potions::PotionId::PowerPotion
                    | crate::content::potions::PotionId::ColorlessPotion
                    | crate::content::potions::PotionId::DuplicationPotion => 4_500.0,
                    _ => 0.0,
                },
                "block" => match potion.id {
                    crate::content::potions::PotionId::BlockPotion
                    | crate::content::potions::PotionId::DexterityPotion
                    | crate::content::potions::PotionId::EssenceOfSteel
                    | crate::content::potions::PotionId::RegenPotion => 5_500.0,
                    _ => 0.0,
                },
                "selfdamage" => match potion.id {
                    crate::content::potions::PotionId::AncientPotion
                    | crate::content::potions::PotionId::RegenPotion => 4_500.0,
                    _ => 0.0,
                },
                "drawcycle" => match potion.id {
                    crate::content::potions::PotionId::SwiftPotion
                    | crate::content::potions::PotionId::EnergyPotion
                    | crate::content::potions::PotionId::GamblersBrew => 5_000.0,
                    _ => 0.0,
                },
                "powerscaling" => match potion.id {
                    crate::content::potions::PotionId::PowerPotion
                    | crate::content::potions::PotionId::ColorlessPotion
                    | crate::content::potions::PotionId::AncientPotion => 5_000.0,
                    _ => 0.0,
                },
                "status" => match potion.id {
                    crate::content::potions::PotionId::PowerPotion
                    | crate::content::potions::PotionId::ColorlessPotion => 4_000.0,
                    _ => 0.0,
                },
                _ => 0.0,
            }
        }
        _ => 0.0,
    }
}

fn tactical_move_bonus(combat: &CombatState, chosen_move: &ClientInput) -> f32 {
    match chosen_move {
        ClientInput::PlayCard { card_index, target } => {
            let Some(card) = combat.hand.get(*card_index) else {
                return 0.0;
            };
            armaments_move_bonus(combat, *card_index)
                + double_tap_move_bonus(combat, *card_index)
                + feel_no_pain_move_bonus(combat, *card_index)
                + limit_break_move_bonus(combat, *card_index)
                + gremlin_nob_skill_penalty(combat, card)
                + sharp_hide_attack_penalty(combat, card, *target)
        }
        _ => 0.0,
    }
}

fn armaments_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::Armaments {
        return 0.0;
    }

    let candidates: Vec<_> = combat
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, c)| {
            *idx != card_index
                && c.upgrades == 0
                && !matches!(
                    get_card_definition(c.id).card_type,
                    CardType::Status | CardType::Curse
                )
        })
        .collect();

    if candidates.is_empty() {
        return 0.0;
    }

    if card.upgrades > 0 {
        return 7_500.0 + candidates.len() as f32 * 1_500.0;
    }

    let best_upgrade_value = candidates
        .iter()
        .map(|(_, c)| armaments_upgrade_value(c.id))
        .max()
        .unwrap_or(0);

    3_500.0 + best_upgrade_value as f32
}

fn double_tap_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::DoubleTap {
        return 0.0;
    }
    if combat.get_power(0, PowerId::DoubleTap) > 0 {
        return -7_000.0;
    }

    let energy_left = combat.energy as i32 - card.get_cost() as i32;
    if energy_left <= 0 {
        return -4_000.0;
    }

    let best_followup = combat
        .hand
        .iter()
        .enumerate()
        .filter(|(idx, other)| {
            *idx != card_index
                && get_card_definition(other.id).card_type == CardType::Attack
                && other.get_cost() as i32 >= 0
                && other.get_cost() as i32 <= energy_left
        })
        .map(|(_, other)| double_tap_followup_value(combat, other))
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    if best_followup <= 0.0 {
        -4_500.0
    } else {
        2_500.0 + best_followup
    }
}

fn feel_no_pain_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::FeelNoPain {
        return 0.0;
    }
    if combat.get_power(0, PowerId::FeelNoPain) > 0 {
        return -8_000.0;
    }

    let exhaust_sources = combat
        .hand
        .iter()
        .filter(|card| {
            matches!(
                card.id,
                CardId::SecondWind
                    | CardId::SeverSoul
                    | CardId::FiendFire
                    | CardId::BurningPact
                    | CardId::TrueGrit
                    | CardId::Offering
                    | CardId::SeeingRed
            )
        })
        .count() as f32;
    let future_status_cards = combat
        .draw_pile
        .iter()
        .chain(combat.discard_pile.iter())
        .chain(combat.hand.iter())
        .filter(|card| {
            matches!(
                get_card_definition(card.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .count() as f32;
    let sentries = combat
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && m.monster_type == EnemyId::Sentry as usize)
        .count() as f32;

    2_500.0 + exhaust_sources * 1_800.0 + future_status_cards * 800.0 + sentries * 2_500.0
}

fn limit_break_move_bonus(combat: &CombatState, card_index: usize) -> f32 {
    let Some(card) = combat.hand.get(card_index) else {
        return 0.0;
    };
    if card.id != CardId::LimitBreak {
        return 0.0;
    }

    let strength = combat.get_power(0, PowerId::Strength);
    if strength <= 0 {
        return -7_000.0;
    }

    let energy_left = combat.energy as i32 - card.get_cost() as i32;
    let best_followup = if energy_left > 0 {
        combat
            .hand
            .iter()
            .enumerate()
            .filter(|(idx, other)| {
                *idx != card_index
                    && get_card_definition(other.id).card_type == CardType::Attack
                    && other.get_cost() as i32 >= 0
                    && other.get_cost() as i32 <= energy_left
            })
            .map(|(_, other)| double_tap_followup_value(combat, other))
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0)
    } else {
        0.0
    };

    let mut bonus = strength as f32 * 1_800.0;
    if strength >= 3 {
        bonus += 5_000.0;
    } else if strength == 2 {
        bonus += 1_500.0;
    } else {
        bonus -= 1_000.0;
    }
    bonus += best_followup * 0.5;
    if best_followup <= 0.0 && strength <= 2 {
        bonus -= 2_500.0;
    }
    bonus
}

fn double_tap_followup_value(combat: &CombatState, card: &crate::combat::CombatCard) -> f32 {
    let base = estimated_attack_damage(combat, card) * card_hits(card.id);
    (base as f32 * 300.0)
        + match card.id {
            CardId::Bash
            | CardId::Uppercut
            | CardId::Hemokinesis
            | CardId::BloodForBlood
            | CardId::SwordBoomerang
            | CardId::Pummel
            | CardId::Rampage
            | CardId::HeavyBlade
            | CardId::Whirlwind => 2_000.0,
            _ => 0.0,
        }
}

fn armaments_upgrade_value(card_id: CardId) -> i32 {
    let def = get_card_definition(card_id);
    let mut score = def.upgrade_damage * 180 + def.upgrade_block * 130 + def.upgrade_magic * 210;

    score += match card_id {
        CardId::Bash
        | CardId::Uppercut
        | CardId::Shockwave
        | CardId::BattleTrance
        | CardId::GhostlyArmor
        | CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::FlameBarrier
        | CardId::BodySlam
        | CardId::HeavyBlade
        | CardId::TrueGrit
        | CardId::SecondWind
        | CardId::BurningPact
        | CardId::LimitBreak
        | CardId::SeeingRed
        | CardId::Havoc
        | CardId::BloodForBlood
        | CardId::Exhume => 1_500,
        _ => 0,
    };

    score
}

fn gremlin_nob_skill_penalty(combat: &CombatState, card: &crate::combat::CombatCard) -> f32 {
    if get_card_definition(card.id).card_type != CardType::Skill {
        return 0.0;
    }

    let nob_is_enraged = combat
        .monsters
        .iter()
        .any(|m| !m.is_dying && !m.is_escaped && combat.get_power(m.id, PowerId::Anger) != 0);
    if !nob_is_enraged {
        return 0.0;
    }

    let mut penalty = -9_000.0;
    let incoming_damage: i32 = combat
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped)
        .map(|m| match m.current_intent {
            crate::combat::Intent::Attack { hits, .. }
            | crate::combat::Intent::AttackBuff { hits, .. }
            | crate::combat::Intent::AttackDebuff { hits, .. }
            | crate::combat::Intent::AttackDefend { hits, .. } => m.intent_dmg * hits as i32,
            _ => 0,
        })
        .sum();
    let threatened = incoming_damage > combat.player.block;

    penalty += match card.id {
        CardId::GhostlyArmor | CardId::ShrugItOff | CardId::FlameBarrier | CardId::Impervious
            if threatened =>
        {
            6_500.0
        }
        CardId::Shockwave | CardId::Disarm if threatened => 4_500.0,
        CardId::Armaments if threatened => 2_500.0,
        _ => 0.0,
    };

    penalty
}

fn sharp_hide_attack_penalty(
    combat: &CombatState,
    card: &crate::combat::CombatCard,
    target: Option<usize>,
) -> f32 {
    if get_card_definition(card.id).card_type != CardType::Attack {
        return 0.0;
    }

    let Some((sharp_hide_owner, thorns_damage, owner_effective_hp)) = combat
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .find_map(|m| {
            let amount = combat.get_power(m.id, PowerId::SharpHide);
            (amount > 0).then_some((m.id, amount, m.current_hp + m.block))
        })
    else {
        return 0.0;
    };

    let incoming_damage: i32 = combat
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
        .map(|m| match m.current_intent {
            crate::combat::Intent::Attack { hits, .. }
            | crate::combat::Intent::AttackBuff { hits, .. }
            | crate::combat::Intent::AttackDebuff { hits, .. }
            | crate::combat::Intent::AttackDefend { hits, .. } => m.intent_dmg * hits as i32,
            _ => 0,
        })
        .sum();
    let lethal = target
        .filter(|&tid| tid == sharp_hide_owner)
        .map(|_| estimated_attack_damage(combat, card) * card_hits(card.id) >= owner_effective_hp)
        .unwrap_or(false);
    let projected_hp =
        combat.player.current_hp - thorns_damage - (incoming_damage - combat.player.block).max(0);

    let mut penalty = -6_000.0;
    if lethal {
        penalty += 5_000.0;
    }
    if projected_hp <= 0 && !lethal {
        penalty -= 20_000.0;
    } else if projected_hp <= 8 && !lethal {
        penalty -= 10_000.0;
    } else if combat.player.current_hp <= thorns_damage + 6 && !lethal {
        penalty -= 6_000.0;
    }

    penalty
}

fn estimated_attack_damage(combat: &CombatState, card: &crate::combat::CombatCard) -> i32 {
    let def = get_card_definition(card.id);
    let mut damage = match card.id {
        CardId::BodySlam => combat.player.block,
        CardId::HeavyBlade => {
            let mult = card.base_magic_num_mut.max(def.base_magic);
            card.base_damage_mut + combat.get_power(0, PowerId::Strength) * mult
        }
        _ => card.base_damage_mut + combat.get_power(0, PowerId::Strength),
    };
    if combat.get_power(0, PowerId::Weak) > 0 {
        damage = (damage as f32 * 0.75).floor() as i32;
    }
    damage.max(0)
}

fn card_hits(card_id: CardId) -> i32 {
    match card_id {
        CardId::TwinStrike => 2,
        CardId::Pummel => 4,
        _ => 1,
    }
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{curiosity_archetype_move_bonus, tactical_move_bonus};
    use crate::combat::PowerId;
    use crate::combat::{
        CombatCard, CombatPhase, CombatState, EphemeralCounters, Intent, MonsterEntity,
    };
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::potions::Potion;
    use crate::state::core::ClientInput;
    use crate::state::run::RunState;
    use std::collections::{HashMap, VecDeque};

    fn combat_with_hand_and_deck(hand: &[CardId], deck: &[CardId]) -> CombatState {
        let mut rs = RunState::new(7, 0, false, "Ironclad");
        rs.master_deck = deck
            .iter()
            .enumerate()
            .map(|(idx, &id)| CombatCard::new(id, idx as u32))
            .collect();
        CombatState {
            ascension_level: 0,
            turn_count: 1,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            draw_pile: rs.master_deck.clone(),
            hand: hand
                .iter()
                .enumerate()
                .map(|(idx, &id)| CombatCard::new(id, (100 + idx) as u32))
                .collect(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            player: rs.build_combat_player(0),
            monsters: vec![MonsterEntity {
                id: 1,
                monster_type: 0,
                current_hp: 30,
                max_hp: 30,
                block: 0,
                slot: 0,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Attack {
                    damage: 10,
                    hits: 1,
                },
                move_history: VecDeque::new(),
                intent_dmg: 10,
                logical_position: 0,
            }],
            potions: vec![None::<Potion>; 3],
            power_db: HashMap::new(),
            action_queue: VecDeque::new(),
            counters: EphemeralCounters::default(),
            card_uuid_counter: 500,
            rng: crate::rng::RngPool::new(123),
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        }
    }

    #[test]
    fn strength_curiosity_prefers_inflame_play() {
        let combat = combat_with_hand_and_deck(
            &[CardId::Inflame, CardId::Strike],
            &[CardId::Inflame, CardId::HeavyBlade, CardId::TwinStrike],
        );
        let bonus = curiosity_archetype_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            Some(&crate::bot::coverage::CuriosityTarget::archetype(
                "strength",
            )),
        );
        assert!(bonus > 0.0);
    }

    #[test]
    fn non_matching_archetype_play_gets_no_bonus() {
        let combat = combat_with_hand_and_deck(
            &[CardId::ShrugItOff],
            &[CardId::ShrugItOff, CardId::BodySlam],
        );
        let bonus = curiosity_archetype_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            Some(&crate::bot::coverage::CuriosityTarget::archetype(
                "strength",
            )),
        );
        assert_eq!(bonus, 0.0);
    }

    #[test]
    fn nob_enrage_penalizes_skill_moves() {
        let mut combat =
            combat_with_hand_and_deck(&[CardId::ShrugItOff], &[CardId::ShrugItOff, CardId::Strike]);
        combat.power_db.insert(
            1,
            vec![crate::combat::Power {
                power_type: PowerId::Anger,
                amount: -1,
                extra_data: 0,
                just_applied: false,
            }],
        );

        let bonus = tactical_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );
        assert!(bonus < 0.0);
    }

    #[test]
    fn armaments_gets_positive_tactical_bonus_with_good_targets() {
        let combat = combat_with_hand_and_deck(
            &[CardId::Armaments, CardId::Bash, CardId::ShrugItOff],
            &[CardId::Armaments, CardId::Bash, CardId::ShrugItOff],
        );
        let bonus = tactical_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );
        assert!(bonus > 0.0);
    }

    #[test]
    fn double_tap_gets_positive_bonus_with_good_followup() {
        let combat = combat_with_hand_and_deck(
            &[CardId::DoubleTap, CardId::Bash, CardId::Strike],
            &[CardId::DoubleTap, CardId::Bash, CardId::Strike],
        );
        let bonus = tactical_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );
        assert!(bonus > 0.0);
    }

    #[test]
    fn feel_no_pain_gets_sentry_specific_bonus() {
        let mut combat = combat_with_hand_and_deck(
            &[CardId::FeelNoPain, CardId::Strike],
            &[
                CardId::FeelNoPain,
                CardId::Strike,
                CardId::Dazed,
                CardId::Dazed,
            ],
        );
        combat.monsters[0].monster_type = EnemyId::Sentry as usize;
        let bonus = tactical_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );
        assert!(bonus > 5_000.0);
    }

    #[test]
    fn limit_break_bonus_scales_with_strength() {
        let mut combat = combat_with_hand_and_deck(
            &[CardId::LimitBreak, CardId::HeavyBlade],
            &[CardId::LimitBreak, CardId::HeavyBlade],
        );
        let low = tactical_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );
        combat.power_db.insert(
            0,
            vec![crate::combat::Power {
                power_type: PowerId::Strength,
                amount: 3,
                extra_data: 0,
                just_applied: false,
            }],
        );
        let high = tactical_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );
        assert!(high > low);
    }

    #[test]
    fn sharp_hide_penalizes_non_lethal_attack_when_low_hp() {
        let mut combat = combat_with_hand_and_deck(
            &[CardId::WildStrike, CardId::FlameBarrier],
            &[CardId::WildStrike, CardId::FlameBarrier],
        );
        combat.player.current_hp = 12;
        combat.monsters[0].current_hp = 30;
        combat.monsters[0].intent_dmg = 9;
        combat.monsters[0].current_intent = Intent::Attack { damage: 9, hits: 1 };
        combat.power_db.insert(
            1,
            vec![crate::combat::Power {
                power_type: PowerId::SharpHide,
                amount: 3,
                extra_data: 0,
                just_applied: false,
            }],
        );

        let wild_strike_bonus = tactical_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        );
        let flame_barrier_bonus = tactical_move_bonus(
            &combat,
            &ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
        );

        assert!(wild_strike_bonus < flame_barrier_bonus);
        assert!(wild_strike_bonus < -10_000.0);
    }
}
