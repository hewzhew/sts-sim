use smallvec::SmallVec;
use crate::state::core::{EngineState, ClientInput, RunResult, PendingChoice};
use crate::combat::{CombatState, CombatPhase};
use crate::action::{Action, ActionInfo};

use super::pending_choices;

pub fn tick_engine(engine_state: &mut EngineState, combat_state: &mut CombatState, input: Option<ClientInput>) -> bool {
    // Phase 1: pending choice overrides
    if let EngineState::PendingChoice(_) = engine_state {
        if let Some(cmd) = input {
            if resolve_pending_choice(engine_state, combat_state, cmd).is_ok() {
                if !matches!(engine_state, EngineState::PendingChoice(_)) {
                    *engine_state = EngineState::CombatProcessing;
                }
            }
        }
        return true; 
    }

    // Phase 2: process input
    if *engine_state == EngineState::CombatPlayerTurn {
        if let Some(cmd) = input {
            if handle_player_turn_input(engine_state, combat_state, cmd).is_ok() {
                // After a card play, actions (damage, block, etc.) are queued.
                // Transition to CombatProcessing to drain the queue.
                if !combat_state.action_queue.is_empty() && *engine_state == EngineState::CombatPlayerTurn {
                    *engine_state = EngineState::CombatProcessing;
                }
            } else {
                return true; 
            }
        } else {
            return true;
        }
    }

    // Phase 3: execute action queue
    if *engine_state == EngineState::CombatProcessing {
        if !combat_state.action_queue.is_empty() {
            let next_action = combat_state.action_queue.pop_front().unwrap();

            // Intercept SuspendFor* actions → transition to PendingChoice
            match next_action {
                Action::SuspendForHandSelect { min, max, reason } => {
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::HandSelect {
                        min_cards: min, max_cards: max, can_cancel: false, reason,
                    });
                    return true;
                },
                Action::SuspendForGridSelect { source_pile, min, max, can_cancel, reason } => {
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::GridSelect {
                        source_pile, min_cards: min, max_cards: max, can_cancel, reason,
                    });
                    return true;
                },
                Action::SuspendForDiscovery { card_type, cost_for_turn } => {
                    // Generate 3 unique random cards from pool, filtered by card_type
                    // Java: DiscoveryAction.generateCardChoices(type) — 3 unique cards
                    let mut pool: Vec<crate::content::cards::CardId> = Vec::new();
                    let is_colorless = card_type.is_none();
                    for &rarity in &[
                        crate::content::cards::CardRarity::Common,
                        crate::content::cards::CardRarity::Uncommon,
                        crate::content::cards::CardRarity::Rare,
                    ] {
                        let current_pool = if is_colorless {
                            crate::content::cards::colorless_pool_for_rarity(rarity)
                        } else {
                            crate::content::cards::ironclad_pool_for_rarity(rarity)
                        };

                        for &id in current_pool {
                            if let Some(ct) = card_type {
                                let def = crate::content::cards::get_card_definition(id);
                                if def.card_type != ct { continue; }
                            }
                            pool.push(id);
                        }
                    }
                    let mut cards = Vec::new();
                    while cards.len() < 3 && !pool.is_empty() {
                        let idx = combat_state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
                        let id = pool[idx];
                        if !cards.contains(&id) {
                            cards.push(id);
                        }
                    }
                    // Store cost_for_turn in the first element of limbo as a signal
                    // (it will be applied when the choice is resolved)
                    combat_state.counters.discovery_cost_for_turn = cost_for_turn;
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::DiscoverySelect(cards));
                    return true;
                },
                Action::SuspendForCardReward { pool, destination, can_skip } => {
                    // Generate 3 unique random cards from pool
                    use crate::action::CardRewardPool;
                    let mut card_pool: Vec<crate::content::cards::CardId> = Vec::new();
                    match pool {
                        CardRewardPool::ClassAll => {
                            // Java: returnTrulyRandomCardInCombat() — all class cards
                            for &rarity in &[
                                crate::content::cards::CardRarity::Common,
                                crate::content::cards::CardRarity::Uncommon,
                                crate::content::cards::CardRarity::Rare,
                            ] {
                                for &id in crate::content::cards::ironclad_pool_for_rarity(rarity) {
                                    card_pool.push(id);
                                }
                            }
                        },
                        CardRewardPool::Colorless => {
                            // Java: returnTrulyRandomColorlessCardInCombat()
                            for &id in crate::content::cards::COLORLESS_UNCOMMON_POOL {
                                let def = crate::content::cards::get_card_definition(id);
                                if !def.tags.contains(&crate::content::cards::CardTag::Healing) {
                                    card_pool.push(id);
                                }
                            }
                            for &id in crate::content::cards::COLORLESS_RARE_POOL {
                                let def = crate::content::cards::get_card_definition(id);
                                if !def.tags.contains(&crate::content::cards::CardTag::Healing) {
                                    card_pool.push(id);
                                }
                            }
                        },
                    }
                    let mut cards = Vec::new();
                    while cards.len() < 3 && !card_pool.is_empty() {
                        let idx = combat_state.rng.card_random_rng.random(card_pool.len() as i32 - 1) as usize;
                        let id = card_pool[idx];
                        if !cards.contains(&id) {
                            cards.push(id);
                        }
                    }
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::CardRewardSelect {
                        cards,
                        destination,
                        can_skip,
                    });
                    return true;
                },
                Action::SuspendForStanceChoice => {
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::StanceChoice);
                    return true;
                },

                Action::FleeCombat => {
                    // Escape combat immediately (SmokeBomb). No rewards.
                    // Java: marks current room as smoked, sets player.isEscaping = true, then combat loop exits.
                    // Returning false from tick_engine ends combat execution.
                    *engine_state = EngineState::MapNavigation;
                    return false;
                },
                _ => {
                    super::action_handlers::execute_action(next_action, combat_state);
                }
            }
            if matches!(engine_state, EngineState::PendingChoice(_)) {
                return true;
            }
        } else {
            // Queue is empty — decide next state based on combat phase
            match combat_state.current_phase {
                CombatPhase::PlayerTurn => {
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::CombatPlayerTurn;
                },
                CombatPhase::TurnTransition => {
                    // === TURN TRANSITION: End of player turn → enemy turn → new player turn ===
                    
                    // 1. Discard hand (respecting Retain and RunicPyramid)
                    let has_runic_pyramid = combat_state.player.has_relic(crate::content::relics::RelicId::RunicPyramid);
                    if has_runic_pyramid {
                        // RunicPyramid: retain ALL cards — skip discard entirely
                    } else {
                        let mut retained = Vec::new();
                        let mut discarded = Vec::new();
                        for card in combat_state.hand.drain(..) {
                            // Check for actual retain: card.retain_override
                            if card.retain_override == Some(true) {
                                retained.push(card);
                            } else {
                                discarded.push(card);
                            }
                        }
                        combat_state.discard_pile.extend(discarded);
                        combat_state.hand = retained;
                    }

                    // 1.5 === MONSTER PRE-TURN LOGIC ===
                    // Java: MonsterStartTurnAction calls MonsterGroup.applyPreTurnLogic() -> clears block, etc.
                    let alive_for_pre: Vec<_> = combat_state.monsters.iter()
                        .filter(|m| !m.is_dying && !m.is_escaped)
                        .map(|m| m.id)
                        .collect();

                    for mid in &alive_for_pre {
                        // 1. Clear block
                        if let Some(monster) = combat_state.monsters.iter_mut().find(|m| m.id == *mid) {
                            let has_barricade = combat_state.power_db.get(mid).map_or(false, |powers| {
                                powers.iter().any(|p| p.power_type == crate::content::powers::PowerId::Barricade)
                            });
                            if !has_barricade {
                                monster.block = 0;
                            }
                        }
                        // 2. Fire Start of Turn Powers (e.g. Poison tick, Flight regain)
                        if let Some(powers) = combat_state.power_db.get(mid).cloned() {
                            for power in &powers {
                                let hook_actions = crate::content::powers::resolve_power_at_turn_start(
                                    power.power_type, combat_state, *mid, power.amount
                                );
                                for a in hook_actions {
                                    combat_state.action_queue.push_back(a);
                                }
                            }
                        }
                    }
                    // 3. Drain pre-turn actions instantly
                    while let Some(action) = combat_state.action_queue.pop_front() {
                        super::action_handlers::execute_action(action, combat_state);
                        if combat_state.player.current_hp <= 0 {
                            combat_state.action_queue.clear();
                            *engine_state = EngineState::GameOver(RunResult::Defeat);
                            return false;
                        }
                    }

                    // 2. Execute each alive monster's turn (player block absorbs damage)
                    combat_state.current_phase = CombatPhase::MonsterTurn;
                    let mut monster_snapshots = Vec::new();
                    let mut dead_ids = Vec::new();
                    for m in &combat_state.monsters {
                        if m.is_dying || m.is_escaped {
                            dead_ids.push(m.id);
                        } else {
                            monster_snapshots.push(m.clone());
                        }
                    }
                    for id in dead_ids {
                        combat_state.power_db.remove(&id);
                    }
                    for monster in &monster_snapshots {
                        // Reset monster Invincible limit
                        if let Some(powers) = combat_state.power_db.get_mut(&monster.id) {
                            if let Some(inv) = powers.iter_mut().find(|p| p.power_type == crate::content::powers::PowerId::Invincible) {
                                inv.amount = inv.extra_data;
                            }
                        }
                        let actions = crate::content::monsters::resolve_monster_turn(combat_state, monster);
                        for action in actions {
                            combat_state.action_queue.push_back(action);
                        }
                        // Drain this monster's turn actions
                        while let Some(action) = combat_state.action_queue.pop_front() {
                            super::action_handlers::execute_action(action, combat_state);
                            if combat_state.player.current_hp <= 0 {
                                combat_state.action_queue.clear();
                                *engine_state = EngineState::GameOver(RunResult::Defeat);
                                return false;
                            }
                        }
                    }
                    // (Monster actions now drained per-monster inside the for-loop above)

                    // 2.3 === COLLECTIVE END OF TURN ===
                    // Java: MonsterGroup.applyEndOfTurnPowers() calls p.atEndOfTurn(false) across all alive monsters.
                    let alive_monsters_for_end_turn: Vec<_> = combat_state.monsters.iter()
                        .filter(|m| !m.is_dying && !m.is_escaped)
                        .map(|m| m.id)
                        .collect();
                    for mid in &alive_monsters_for_end_turn {
                        if let Some(powers) = combat_state.power_db.get(mid).cloned() {
                            for power in &powers {
                                let hook_actions = crate::content::powers::resolve_power_at_end_of_turn(
                                    power.power_type, combat_state, *mid, power.amount
                                );
                                for a in hook_actions {
                                    combat_state.action_queue.push_back(a);
                                }
                            }
                        }
                    }
                    // Drain atEndOfTurn collective actions
                    while let Some(action) = combat_state.action_queue.pop_front() {
                        super::action_handlers::execute_action(action, combat_state);
                        if combat_state.player.current_hp <= 0 {
                            combat_state.action_queue.clear();
                            *engine_state = EngineState::GameOver(RunResult::Defeat);
                            return false;
                        }
                    }

                    // 2.5 === FULL ROUND END ===
                    // Java: applyEndOfTurnPowers() calls p.atEndOfRound() on player and all monsters
                    // Player powers:
                    if let Some(powers) = combat_state.power_db.get(&0).cloned() {
                        for power in &powers {
                            let hook_actions = crate::content::powers::resolve_power_at_end_of_round(
                                power.power_type, combat_state, 0, power.amount, power.just_applied
                            );
                            for a in hook_actions {
                                combat_state.action_queue.push_back(a);
                            }
                        }
                    }
                    // Monster powers:
                    let alive_monsters: Vec<_> = combat_state.monsters.iter()
                        .filter(|m| !m.is_dying && !m.is_escaped)
                        .map(|m| m.id)
                        .collect();
                    for mid in alive_monsters {
                        if let Some(powers) = combat_state.power_db.get(&mid).cloned() {
                            for power in &powers {
                                let hook_actions = crate::content::powers::resolve_power_at_end_of_round(
                                    power.power_type, combat_state, mid, power.amount, power.just_applied
                                );
                                for a in hook_actions {
                                    combat_state.action_queue.push_back(a);
                                }
                            }
                        }
                    }
                    // Drain at_end_of_round actions
                    while let Some(action) = combat_state.action_queue.pop_front() {
                        super::action_handlers::execute_action(action, combat_state);
                    }
                    
                    // Clear all just_applied flags globally at the end of the round!
                    for powers in combat_state.power_db.values_mut() {
                        for p in powers.iter_mut() {
                            p.just_applied = false;
                        }
                    }

                    // If player died during monster turn, immediate game over
                    if combat_state.player.current_hp <= 0 {
                        combat_state.action_queue.clear();
                        *engine_state = EngineState::GameOver(RunResult::Defeat);
                        return false;
                    }

                    // 3. (Intent rolling is handled by Action::RollMonsterMove in the queue)

                    // === NEW PLAYER TURN START ===
                    // 4. Clear player block (Barricade: keep all, Calipers: retain up to 15)
                    let has_barricade = combat_state.power_db.get(&0).map_or(false, |powers| {
                        powers.iter().any(|p| p.power_type == crate::content::powers::PowerId::Barricade)
                    });
                    if !has_barricade {
                        let has_calipers = !combat_state.player.relic_buses.on_calculate_block_retained.is_empty();
                        if has_calipers {
                            let retained = crate::content::relics::hooks::on_calculate_block_retained(combat_state, combat_state.player.block);
                            combat_state.player.block = retained;
                        } else {
                            combat_state.player.block = 0;
                        }
                    }

                    // (Monster blocks are cleared per-monster at the start of each monster's turn above)

                    combat_state.turn_count += 1;
                    combat_state.current_phase = CombatPhase::PlayerTurn;

                    // 6. Reset energy — Java: EnergyManager.recharge() → this.energy = this.energyMaster
                    combat_state.energy = combat_state.player.energy_master;

                    // 7. Reset per-turn counters
                    combat_state.counters.cards_played_this_turn = 0;
                    combat_state.counters.attacks_played_this_turn = 0;
                    // Reset per-turn relic counters (Necronomicon, OrangePellets, Pocketwatch)
                    for relic in combat_state.player.relics.iter_mut() {
                        match relic.id {
                            crate::content::relics::RelicId::Necronomicon => relic.counter = 0,
                            crate::content::relics::RelicId::OrangePellets => relic.counter = 0,
                            crate::content::relics::RelicId::Pocketwatch => relic.counter = 0,
                            _ => {}
                        }
                    }

                    // Reset player Invincible limit
                    if let Some(powers) = combat_state.power_db.get_mut(&0) {
                        if let Some(inv) = powers.iter_mut().find(|p| p.power_type == crate::content::powers::PowerId::Invincible) {
                            inv.amount = inv.extra_data;
                        }
                    }

                    // 8. at_turn_start relic hooks (AncientTeaSet, HappyFlower, etc.)
                    // Java: relics fire atTurnStart BEFORE draw cards
                    let turn_start_actions = crate::content::relics::hooks::at_turn_start(combat_state);
                    queue_actions(&mut combat_state.action_queue, turn_start_actions);

                    // 8.1. at_turn_start power hooks (Player)
                    // Java: player.applyStartOfTurnPowers()
                    if let Some(player_powers) = combat_state.power_db.get(&0).cloned() {
                        for power in &player_powers {
                            let pa = crate::content::powers::resolve_power_at_turn_start(
                                power.power_type, combat_state, 0, power.amount
                            );
                            for a in pa {
                                combat_state.action_queue.push_back(a);
                            }
                        }
                    }

                    // 8.2. applyStartOfTurnOrbs
                    let orb_actions = crate::content::orbs::hooks::at_turn_start(combat_state);
                    queue_actions(&mut combat_state.action_queue, orb_actions);

                    // 8.3. applyStartOfTurnCards (For Curses in hand)
                    let card_actions = crate::content::cards::hooks::at_turn_start_in_hand(combat_state);
                    queue_actions(&mut combat_state.action_queue, card_actions);

                    // 9. Draw cards (default 5, reduced by DrawReduction power)
                    // Java: GameActionManager checks DrawReductionPower.amount
                    let mut draw_count: i32 = 5;
                    if combat_state.player.has_relic(crate::content::relics::RelicId::SneckoEye) {
                        draw_count += 2;
                    }
                    if let Some(powers) = combat_state.power_db.get(&0) {
                        if let Some(dr) = powers.iter().find(|p| p.power_type == crate::content::powers::PowerId::DrawReduction) {
                            draw_count -= dr.amount;
                        }
                    }
                    if draw_count > 0 {
                        combat_state.action_queue.push_back(Action::DrawCards(draw_count as u32));
                    }
                    // Java: DrawReductionPower.atEndOfRound() calls removePowerAction
                    // We remove it at turn start after applying the reduction
                    if let Some(powers) = combat_state.power_db.get_mut(&0) {
                        powers.retain(|p| p.power_type != crate::content::powers::PowerId::DrawReduction);
                    }

                    *engine_state = EngineState::CombatProcessing;
                },
                CombatPhase::MonsterTurn => {
                    // Monster actions drained, transition to player turn start
                    combat_state.current_phase = CombatPhase::PlayerTurn;
                    *engine_state = EngineState::CombatProcessing;
                },
            }
            if combat_state.player.current_hp <= 0 {
                *engine_state = EngineState::GameOver(RunResult::Defeat);
                return false;
            }
            return true;
        }
    }

    if combat_state.monsters.iter().all(|m| m.current_hp <= 0 || m.is_escaped) {
        if !combat_state.counters.victory_triggered {
            combat_state.counters.victory_triggered = true;
            combat_state.action_queue.clear();
            
            // Generate basic reward stub
            *engine_state = EngineState::RewardScreen(crate::state::reward::RewardState::new());
            return false;
        }
        *engine_state = EngineState::CombatProcessing;
    }
    
    true
}

fn handle_player_turn_input(engine_state: &mut EngineState, combat_state: &mut CombatState, cmd: ClientInput) -> Result<(), &'static str> {
    match cmd {
        ClientInput::PlayCard { card_index, mut target } => {
            // 1. Validate card in hand
            if card_index >= combat_state.hand.len() {
                return Err("Card index out of range");
            }

            // VelvetChoker: cannot play more than 6 cards per turn (Java: canPlay returns false if counter >= 6)
            if combat_state.player.has_relic(crate::content::relics::RelicId::VelvetChoker)
                && combat_state.counters.cards_played_this_turn >= 6
            {
                return Err("VelvetChoker: card play limit reached (6)");
            }

            let card = &combat_state.hand[card_index];
            let card_id = card.id;
            let _card_uuid = card.uuid;
            let def = crate::content::cards::get_card_definition(card_id);

            // 1.2 Card-Specific Play Conditions (Clash, Normality, etc.)
            crate::content::cards::can_play_card(card, combat_state)?;

            // 1.5 Target Validation and Auto-Selection
            use crate::content::cards::CardTarget;
            if def.target == CardTarget::Enemy {
                let targetable: Vec<_> = combat_state.monsters.iter()
                    .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
                    .map(|m| m.id)
                    .collect();

                if let Some(t_id) = target {
                    if !targetable.contains(&t_id) {
                        return Err("Invalid or untargetable monster selected.");
                    }
                } else {
                    if targetable.len() == 1 {
                        // Auto-select the only valid target
                        target = Some(targetable[0]);
                    } else if targetable.is_empty() {
                        return Err("No valid targets available.");
                    } else {
                        return Err("Multiple targets available. Must specify a target.");
                    }
                }
            } else {
                target = None;
            }

            // 2. Compute effective cost
            let effective_cost = if card.free_to_play_once {
                0
            } else if let Some(cft) = card.cost_for_turn {
                cft as i32
            } else {
                (def.cost as i32 + card.cost_modifier as i32).max(0)
            };

            // X-cost cards: cost = all remaining energy (Java: cost == -1)
            let is_x_cost = def.cost == -1;
            let energy_to_spend = if is_x_cost {
                combat_state.energy as i32
            } else {
                effective_cost
            };

            // 3. Energy check (skip for X-cost, they spend whatever is available)
            if !is_x_cost && energy_to_spend > combat_state.energy as i32 {
                return Err("Not enough energy");
            }

            // 4. Spend energy
            combat_state.energy = (combat_state.energy as i32 - energy_to_spend).max(0) as u8;

            // 5. Store energy_on_use for X-cost cards (e.g. Whirlwind)
            let card_mut = &mut combat_state.hand[card_index];
            if is_x_cost {
                card_mut.energy_on_use = energy_to_spend;
            }

            // 5b. Re-evaluate card with target so Vulnerable/etc apply to base_damage_mut
            {
                let mut card_copy = combat_state.hand[card_index].clone();
                crate::content::cards::evaluate_card(&mut card_copy, combat_state, target);
                combat_state.hand[card_index] = card_copy;
            }

            // 6. Remove card from hand
            let mut played_card = combat_state.hand.remove(card_index);

            // 7. Generate card play actions
            let card_actions = crate::content::cards::resolve_card_play(card_id, combat_state, &played_card, target);
            queue_actions(&mut combat_state.action_queue, card_actions);

            // 8. on_use_card relic hooks (Kunai, Nunchaku, PenNib, etc.)
            let relic_actions = crate::content::relics::hooks::on_use_card(combat_state, card_id);
            queue_actions(&mut combat_state.action_queue, relic_actions);

            // 8b. on_card_played power hooks for ALL creatures (Java: UseCardAction triggers onUseCard)
            // This dispatches powers like Sharp Hide (Guardian), Anger, Hex on monsters,
            // and any player powers that respond to card plays.
            for entity_id in std::iter::once(0usize).chain(combat_state.monsters.iter().map(|m| m.id)) {
                if let Some(powers) = combat_state.power_db.get(&entity_id).cloned() {
                    for power in &powers {
                        let hook_actions = crate::content::powers::resolve_power_on_card_played(
                            power.power_type, combat_state, entity_id, &played_card, power.amount
                        );
                        for a in hook_actions {
                            combat_state.action_queue.push_back(a);
                        }
                    }
                }
            }

            // 8c. on_use_card power hooks that need &mut state (DoubleTap, DuplicationPower, Corruption)
            // These powers clone/modify cards directly, requiring mutable access.
            {
                let player_powers = combat_state.power_db.get(&0).cloned().unwrap_or_default();
                let mut exhaust_override = false;
                for power in &player_powers {
                    use crate::content::powers::PowerId;
                    match power.power_type {
                        PowerId::DoubleTap | PowerId::DuplicationPower | PowerId::Corruption => {
                            crate::content::powers::resolve_power_on_use_card(
                                power.power_type, combat_state, &played_card, &mut exhaust_override, false, target,
                            );
                        }
                        _ => {}
                    }
                }
                if exhaust_override {
                    // Corruption forces skills to exhaust
                    played_card.exhaust_override = Some(true);
                }
            }

            // 9. Update counters
            combat_state.counters.cards_played_this_turn += 1;
            if def.card_type == crate::content::cards::CardType::Attack {
                combat_state.counters.attacks_played_this_turn += 1;
            }

            // 10. Determine exhaust/discard, but DEFER the actual card-to-pile move
            //     Java's UseCardAction moves the card AFTER all sub-actions complete.
            //     If we move it now, DrawCards shuffle would sweep it from discard to draw.
            let mut should_exhaust = played_card.exhaust_override.unwrap_or(def.exhaust)
                // MedicalKit: Status cards exhaust when played
                || (def.card_type == crate::content::cards::CardType::Status
                    && combat_state.player.has_relic(crate::content::relics::RelicId::MedicalKit))
                // BlueCandle: Curses exhaust when played
                || (def.card_type == crate::content::cards::CardType::Curse
                    && combat_state.player.has_relic(crate::content::relics::RelicId::BlueCandle));
            // Corruption power: Skill cards exhaust when played
            crate::content::cards::ironclad::corruption::corruption_on_use_card(
                combat_state, &played_card, &mut should_exhaust
            );
            if def.card_type == crate::content::cards::CardType::Power {
                // Power cards are purged after play (removed from game)
                // Card is dropped — not added to any pile or limbo
            } else {
                // Hold card in limbo until UseCardDone fires
                combat_state.limbo.push(played_card);
                combat_state.action_queue.push_back(Action::UseCardDone { should_exhaust });
            }

            Ok(())
        },

        ClientInput::UsePotion { potion_index, target } => {
            // Queue UsePotion action — handler at action_handlers.rs does the work
            combat_state.action_queue.push_back(Action::UsePotion {
                slot: potion_index,
                target: target.map(|t| t as usize),
            });
            Ok(())
        },

        ClientInput::DiscardPotion(slot) => {
            combat_state.action_queue.push_back(Action::DiscardPotion { slot });
            Ok(())
        },

        ClientInput::EndTurn => {
            // Queue end-of-turn processing
            // 1. EndTurnTrigger handles in-hand card effects (Burn, Decay, ethereal exhaust, etc.)
            combat_state.action_queue.push_back(Action::EndTurnTrigger);
            // 2. Relic at_end_of_turn hooks (Orichalcum, CloakClasp, ArtOfWar, etc.)
            let end_turn_relic_actions = crate::content::relics::hooks::at_end_of_turn(combat_state);
            queue_actions(&mut combat_state.action_queue, end_turn_relic_actions);
            // 3. Transition: the engine loop will detect CombatProcessing and handle
            //    discarding hand, applying power at_end_of_turn, enemy turns, draw, etc.
            *engine_state = EngineState::CombatProcessing;
            combat_state.current_phase = CombatPhase::TurnTransition;
            Ok(())
        },

        _ => Err("Invalid input for player turn"),
    }
}

fn resolve_pending_choice(engine_state: &mut EngineState, combat_state: &mut CombatState, input: ClientInput) -> Result<(), &'static str> {
    let choice = if let EngineState::PendingChoice(c) = engine_state {
        c.clone()
    } else {
        return Err("Not in a pending choice state");
    };

    match choice {
        PendingChoice::ScrySelect { cards, card_uuids: _ } => pending_choices::handle_scry(engine_state, combat_state, cards.len(), input),
        PendingChoice::HandSelect { min_cards: count, max_cards: _, can_cancel: cancellable, reason } => {
            pending_choices::handle_hand_select(engine_state, combat_state, count as usize, false, cancellable, reason, input)
        },
        PendingChoice::GridSelect { source_pile, min_cards, max_cards, can_cancel, reason } => {
            pending_choices::handle_grid_select(engine_state, combat_state, source_pile, min_cards, max_cards, can_cancel, reason, input)
        },
        PendingChoice::DiscoverySelect(ref cards) => {
            // Player picks one card from the discovery options
            if let ClientInput::SubmitDiscoverChoice(idx) = input {
                if idx < cards.len() {
                    let card_id = cards[idx];
                    let uuid = 50000 + combat_state.hand.len() as u32 + combat_state.discard_pile.len() as u32;
                    let mut card = crate::combat::CombatCard::new(card_id, uuid);
                    // Apply cost override from the SuspendForDiscovery action
                    if let Some(cost) = combat_state.counters.discovery_cost_for_turn.take() {
                        card.cost_for_turn = Some(cost);
                    }
                    if combat_state.hand.len() < 10 {
                        combat_state.hand.push(card);
                    } else {
                        combat_state.discard_pile.push(card);
                    }
                    *engine_state = EngineState::CombatProcessing;
                    return Ok(());
                }
            }
            Err("Invalid discovery choice")
        },
        PendingChoice::TargetSelect(_validation) => {
            // Target selection — not currently produced by any action, placeholder handler
            *engine_state = EngineState::CombatProcessing;
            Ok(())
        },
        PendingChoice::CardRewardSelect { ref cards, destination, can_skip } => {
            // Player picks one card from the reward options, or Cancel if can_skip
            match input {
                ClientInput::SubmitDiscoverChoice(idx) => {
                    if idx < cards.len() {
                        let card_id = cards[idx];
                        let uuid = 50000 + combat_state.hand.len() as u32 + combat_state.discard_pile.len() as u32 + combat_state.draw_pile.len() as u32;
                        let card = crate::combat::CombatCard::new(card_id, uuid);
                        match destination {
                            crate::action::CardDestination::Hand => {
                                // Java ChooseOneColorless: hand (or discard if full)
                                if combat_state.hand.len() < 10 {
                                    combat_state.hand.push(card);
                                } else {
                                    combat_state.discard_pile.push(card);
                                }
                            },
                            crate::action::CardDestination::DrawPileRandom => {
                                // Java CodexAction: add to draw pile at random position
                                if combat_state.draw_pile.is_empty() {
                                    combat_state.draw_pile.push(card);
                                } else {
                                    let pos = combat_state.rng.card_random_rng.random(combat_state.draw_pile.len() as i32) as usize;
                                    combat_state.draw_pile.insert(pos.min(combat_state.draw_pile.len()), card);
                                }
                            },
                        }
                        *engine_state = EngineState::CombatProcessing;
                        Ok(())
                    } else {
                        Err("Invalid card reward choice index")
                    }
                },
                ClientInput::Cancel if can_skip => {
                    // Java CodexAction: canSkip=true — player can skip picking
                    *engine_state = EngineState::CombatProcessing;
                    Ok(())
                },
                _ => Err("Invalid input for card reward selection"),
            }
        },
        PendingChoice::StanceChoice => {
            // Player picks 0=Wrath, 1=Calm
            if let ClientInput::SubmitDiscoverChoice(idx) = input {
                let stance = match idx {
                    0 => "Wrath",
                    1 => "Calm",
                    _ => return Err("Invalid stance choice (expected 0=Wrath or 1=Calm)"),
                };
                combat_state.action_queue.push_back(Action::EnterStance(stance.to_string()));
                *engine_state = EngineState::CombatProcessing;
                Ok(())
            } else {
                Err("Expected SubmitDiscoverChoice for stance selection")
            }
        },
    }
}

pub fn queue_actions(queue: &mut std::collections::VecDeque<Action>, actions: SmallVec<[ActionInfo; 4]>) {
    let mut to_bottom = vec![];
    let mut to_front = vec![];

    for a in actions {
        match a.insertion_mode {
            crate::action::AddTo::Top => to_front.push(a.action),
            crate::action::AddTo::Bottom => to_bottom.push(a.action),
        }
    }

    // Top actions: push in reverse so first item ends up at front
    for action in to_front.into_iter().rev() {
        queue.push_front(action);
    }
    for action in to_bottom {
        queue.push_back(action);
    }
}

/// Java: AbstractMonster.applyPowers()
/// Interrogates each living monster, extracts base Damage from its `current_intent`,
/// runs it through `calculate_monster_damage()`, and stores the mutated result in `intent_dmg`.
/// This is purely for updating the UI visually before user interaction.
pub fn update_monster_intents(combat_state: &mut CombatState) {
    let alive_monsters: Vec<_> = combat_state.monsters.iter()
        .filter(|m| !m.is_dying && !m.is_escaped)
        .map(|m| m.id)
        .collect();

    for mid in alive_monsters {
        let mut new_intent_dmg = 0;
        
        // Temporarily extract current intent (cannot borrow mutably directly since we need state)
        if let Some(monster) = combat_state.monsters.iter().find(|m| m.id == mid) {
            if let crate::combat::Intent::Attack { damage, .. } 
                | crate::combat::Intent::AttackBuff { damage, .. }
                | crate::combat::Intent::AttackDebuff { damage, .. }
                | crate::combat::Intent::AttackDefend { damage, .. } = monster.current_intent 
            {
                // `damage` in the enum represents the pure base damage
                new_intent_dmg = crate::content::powers::calculate_monster_damage(damage, mid, 0, combat_state);
            } else {
                new_intent_dmg = -1; // Not an attack intent
            }
        }

        // Apply it back
        if let Some(monster) = combat_state.monsters.iter_mut().find(|m| m.id == mid) {
            if new_intent_dmg != -1 {
                monster.intent_dmg = new_intent_dmg;
            } else {
                monster.intent_dmg = 0;
            }
        }
    }
}

/// Run tick_engine until it returns to CombatPlayerTurn or game over.
pub fn tick_until_stable_turn(es: &mut EngineState, cs: &mut CombatState, input: ClientInput) -> bool {
    // First tick with input
    let alive = tick_engine(es, cs, Some(input));
    if !alive {
        return false;
    }
    
    // After any input: engine stays at CombatPlayerTurn but actions may be queued.
    // We need to force CombatProcessing to drain the action queue.
    if *es == EngineState::CombatPlayerTurn && !cs.action_queue.is_empty() {
        *es = EngineState::CombatProcessing;
    }
    
    // Keep ticking until we're back at PlayerTurn (waiting for input), or we're in a PendingChoice
    let mut iterations = 0;
    loop {
        match es {
            EngineState::CombatPlayerTurn => break,
            EngineState::CombatProcessing => {},
            EngineState::PendingChoice(_) => break, // Would need user input
            EngineState::GameOver(_) => return false,
            _ => break, // RewardScreen, etc.
        }
        let alive = tick_engine(es, cs, None);
        if !alive {
            return false;
        }
        iterations += 1;
        if iterations > 1000 {
            eprintln!("  WARNING: tick loop exceeded 1000 iterations");
            break;
        }
    }
    true
}
