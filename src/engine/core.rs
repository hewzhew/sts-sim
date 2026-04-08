use crate::action::{Action, ActionInfo};
use crate::combat::{CombatPhase, CombatState};
use crate::state::core::{ClientInput, EngineState, PendingChoice, RunResult};
use smallvec::SmallVec;

use super::pending_choices;
use super::targeting;

pub fn tick_engine(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    input: Option<ClientInput>,
) -> bool {
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
                if !combat_state.action_queue.is_empty()
                    && *engine_state == EngineState::CombatPlayerTurn
                {
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
                Action::SuspendForHandSelect {
                    min,
                    max,
                    can_cancel,
                    filter,
                    reason,
                } => {
                    let candidate_uuids = hand_select_candidates(combat_state, filter);
                    let available = candidate_uuids.len() as u8;
                    if available == 0 {
                        eprintln!(
                            "WARNING: Auto-skipping empty hand select for {:?} with filter {:?} (requested min={}, max={})",
                            reason, filter, min, max
                        );
                        return true;
                    }

                    if available == 1 && min == 1 && max == 1 && !can_cancel {
                        let _ = pending_choices::handle_hand_select(
                            engine_state,
                            combat_state,
                            &candidate_uuids,
                            1,
                            true,
                            false,
                            reason,
                            ClientInput::SubmitHandSelect(vec![candidate_uuids[0]]),
                        );
                        return true;
                    }

                    let min_cards = min.min(available);
                    let max_cards = max.min(available);
                    if min_cards != min || max_cards != max {
                        eprintln!(
                            "WARNING: Clamping hand select for {:?} with filter {:?} from min/max {}/{} to {}/{} because only {} candidates remain",
                            reason, filter, min, max, min_cards, max_cards, available
                        );
                    }

                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::HandSelect {
                        candidate_uuids,
                        min_cards,
                        max_cards,
                        can_cancel,
                        reason,
                    });
                    return true;
                }
                Action::SuspendForGridSelect {
                    source_pile,
                    min,
                    max,
                    can_cancel,
                    filter,
                    reason,
                } => {
                    let candidate_uuids = grid_select_candidates(combat_state, source_pile, filter);
                    let available = candidate_uuids.len() as u8;
                    if available == 0 {
                        eprintln!(
                            "WARNING: Auto-skipping empty grid select for {:?} on {:?} with filter {:?} (requested min={}, max={})",
                            reason, source_pile, filter, min, max
                        );
                        return true;
                    }

                    if available == 1 && min == 1 && max == 1 && !can_cancel {
                        let _ = pending_choices::handle_grid_select(
                            engine_state,
                            combat_state,
                            &candidate_uuids,
                            source_pile,
                            1,
                            1,
                            false,
                            reason,
                            ClientInput::SubmitGridSelect(vec![candidate_uuids[0]]),
                        );
                        return true;
                    }

                    let min_cards = min.min(available);
                    let max_cards = max.min(available);
                    if min_cards != min || max_cards != max {
                        eprintln!(
                            "WARNING: Clamping grid select for {:?} on {:?} with filter {:?} from min/max {}/{} to {}/{} because only {} candidates remain",
                            reason, source_pile, filter, min, max, min_cards, max_cards, available
                        );
                    }

                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::GridSelect {
                        source_pile,
                        candidate_uuids,
                        min_cards,
                        max_cards,
                        can_cancel,
                        reason,
                    });
                    return true;
                }
                Action::SuspendForDiscovery {
                    card_type,
                    cost_for_turn,
                } => {
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
                                if def.card_type != ct {
                                    continue;
                                }
                            }
                            pool.push(id);
                        }
                    }
                    let mut cards = Vec::new();
                    while cards.len() < 3 && !pool.is_empty() {
                        let idx = combat_state
                            .rng
                            .card_random_rng
                            .random(pool.len() as i32 - 1)
                            as usize;
                        let id = pool[idx];
                        if !cards.contains(&id) {
                            cards.push(id);
                        }
                    }
                    // Store cost_for_turn in the first element of limbo as a signal
                    // (it will be applied when the choice is resolved)
                    combat_state.counters.discovery_cost_for_turn = cost_for_turn;
                    update_monster_intents(combat_state);
                    *engine_state =
                        EngineState::PendingChoice(PendingChoice::DiscoverySelect(cards));
                    return true;
                }
                Action::SuspendForCardReward {
                    pool,
                    destination,
                    can_skip,
                } => {
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
                        }
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
                        }
                    }
                    let mut cards = Vec::new();
                    while cards.len() < 3 && !card_pool.is_empty() {
                        let idx = combat_state
                            .rng
                            .card_random_rng
                            .random(card_pool.len() as i32 - 1)
                            as usize;
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
                }
                Action::SuspendForStanceChoice => {
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::StanceChoice);
                    return true;
                }

                Action::FleeCombat => {
                    // Escape combat immediately (SmokeBomb). No rewards.
                    // Java: marks current room as smoked, sets player.isEscaping = true, then combat loop exits.
                    // Returning false from tick_engine ends combat execution.
                    *engine_state = EngineState::MapNavigation;
                    return false;
                }
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
                }
                CombatPhase::TurnTransition => {
                    // === TURN TRANSITION: End of player turn → enemy turn → new player turn ===

                    // 1. Discard hand (respecting Retain and RunicPyramid)
                    let has_runic_pyramid = combat_state
                        .player
                        .has_relic(crate::content::relics::RelicId::RunicPyramid);
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
                    let alive_for_pre: Vec<_> = combat_state
                        .monsters
                        .iter()
                        .filter(|m| !m.is_dying && !m.is_escaped)
                        .map(|m| m.id)
                        .collect();

                    for mid in &alive_for_pre {
                        // 1. Clear block
                        if let Some(monster) =
                            combat_state.monsters.iter_mut().find(|m| m.id == *mid)
                        {
                            let has_barricade =
                                combat_state.power_db.get(mid).map_or(false, |powers| {
                                    powers.iter().any(|p| {
                                        p.power_type == crate::content::powers::PowerId::Barricade
                                    })
                                });
                            if !has_barricade {
                                monster.block = 0;
                            }
                        }
                        // 2. Fire Start of Turn Powers (e.g. Poison tick, Flight regain)
                        if let Some(powers) = combat_state.power_db.get(mid).cloned() {
                            for power in &powers {
                                let hook_actions =
                                    crate::content::powers::resolve_power_at_turn_start(
                                        power.power_type,
                                        combat_state,
                                        *mid,
                                        power.amount,
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
                            if let Some(inv) = powers.iter_mut().find(|p| {
                                p.power_type == crate::content::powers::PowerId::Invincible
                            }) {
                                inv.amount = inv.extra_data;
                            }
                        }
                        let actions =
                            crate::content::monsters::resolve_monster_turn(combat_state, monster);
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
                    let alive_monsters_for_end_turn: Vec<_> = combat_state
                        .monsters
                        .iter()
                        .filter(|m| !m.is_dying && !m.is_escaped)
                        .map(|m| m.id)
                        .collect();
                    for mid in &alive_monsters_for_end_turn {
                        if let Some(powers) = combat_state.power_db.get(mid).cloned() {
                            for power in &powers {
                                let hook_actions =
                                    crate::content::powers::resolve_power_at_end_of_turn(
                                        power.power_type,
                                        combat_state,
                                        *mid,
                                        power.amount,
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
                            let hook_actions =
                                crate::content::powers::resolve_power_at_end_of_round(
                                    power.power_type,
                                    combat_state,
                                    0,
                                    power.amount,
                                    power.just_applied,
                                );
                            for a in hook_actions {
                                combat_state.action_queue.push_back(a);
                            }
                        }
                    }
                    // Monster powers:
                    let alive_monsters: Vec<_> = combat_state
                        .monsters
                        .iter()
                        .filter(|m| !m.is_dying && !m.is_escaped)
                        .map(|m| m.id)
                        .collect();
                    for mid in alive_monsters {
                        if let Some(powers) = combat_state.power_db.get(&mid).cloned() {
                            for power in &powers {
                                let hook_actions =
                                    crate::content::powers::resolve_power_at_end_of_round(
                                        power.power_type,
                                        combat_state,
                                        mid,
                                        power.amount,
                                        power.just_applied,
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
                        powers
                            .iter()
                            .any(|p| p.power_type == crate::content::powers::PowerId::Barricade)
                    });
                    if !has_barricade {
                        let has_calipers = !combat_state
                            .player
                            .relic_buses
                            .on_calculate_block_retained
                            .is_empty();
                        if has_calipers {
                            let retained =
                                crate::content::relics::hooks::on_calculate_block_retained(
                                    combat_state,
                                    combat_state.player.block,
                                );
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
                        if let Some(inv) = powers
                            .iter_mut()
                            .find(|p| p.power_type == crate::content::powers::PowerId::Invincible)
                        {
                            inv.amount = inv.extra_data;
                        }
                    }

                    // 8. at_turn_start relic hooks (AncientTeaSet, HappyFlower, etc.)
                    // Java: relics fire atTurnStart BEFORE draw cards
                    let turn_start_actions =
                        crate::content::relics::hooks::at_turn_start(combat_state);
                    queue_actions(&mut combat_state.action_queue, turn_start_actions);

                    // 8.1. at_turn_start power hooks (Player)
                    // Java: player.applyStartOfTurnPowers()
                    if let Some(player_powers) = combat_state.power_db.get(&0).cloned() {
                        for power in &player_powers {
                            let pa = crate::content::powers::resolve_power_at_turn_start(
                                power.power_type,
                                combat_state,
                                0,
                                power.amount,
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
                    let card_actions =
                        crate::content::cards::hooks::at_turn_start_in_hand(combat_state);
                    queue_actions(&mut combat_state.action_queue, card_actions);

                    // 9. Draw cards (default 5, reduced by DrawReduction power)
                    // Java: GameActionManager checks DrawReductionPower.amount
                    let mut draw_count: i32 = 5;
                    if combat_state
                        .player
                        .has_relic(crate::content::relics::RelicId::SneckoEye)
                    {
                        draw_count += 2;
                    }
                    if let Some(powers) = combat_state.power_db.get(&0) {
                        if let Some(dr) = powers.iter().find(|p| {
                            p.power_type == crate::content::powers::PowerId::DrawReduction
                        }) {
                            draw_count -= dr.amount;
                        }
                    }
                    if draw_count > 0 {
                        combat_state
                            .action_queue
                            .push_back(Action::DrawCards(draw_count as u32));
                    }
                    // Java: DrawReductionPower.atEndOfRound() calls removePowerAction
                    // We remove it at turn start after applying the reduction
                    if let Some(powers) = combat_state.power_db.get_mut(&0) {
                        powers.retain(|p| {
                            p.power_type != crate::content::powers::PowerId::DrawReduction
                        });
                    }

                    *engine_state = EngineState::CombatProcessing;
                }
                CombatPhase::MonsterTurn => {
                    // Monster actions drained, transition to player turn start
                    combat_state.current_phase = CombatPhase::PlayerTurn;
                    *engine_state = EngineState::CombatProcessing;
                }
            }
            if combat_state.player.current_hp <= 0 {
                *engine_state = EngineState::GameOver(RunResult::Defeat);
                return false;
            }
            return true;
        }
    }

    if combat_state.monsters.iter().all(|m| {
        if m.is_escaped {
            return true;
        }
        if m.half_dead {
            return false;
        }
        if m.current_hp > 0 {
            return false;
        }
        let is_pending_rebirth = combat_state.power_db.get(&m.id).is_some_and(|powers| {
            powers.iter().any(|p| {
                matches!(
                    p.power_type,
                    crate::content::powers::PowerId::Regrow
                        | crate::content::powers::PowerId::Unawakened
                )
            })
        });
        !is_pending_rebirth
    }) {
        if !combat_state.counters.victory_triggered {
            combat_state.counters.victory_triggered = true;
        }

        // Java does not cut off queued onUseCard / onDeath aftermath when the last monster dies.
        // Finish draining any already-queued actions (e.g. Rage block, relic hooks, death hooks)
        // before transitioning to rewards.
        if combat_state.action_queue.is_empty() && combat_state.limbo.is_empty() {
            *engine_state = EngineState::RewardScreen(crate::rewards::state::RewardState::new());
            return false;
        }
        *engine_state = EngineState::CombatProcessing;
    }

    true
}

fn handle_player_turn_input(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    cmd: ClientInput,
) -> Result<(), &'static str> {
    match cmd {
        ClientInput::PlayCard {
            card_index,
            mut target,
        } => {
            // 1. Validate card in hand
            if card_index >= combat_state.hand.len() {
                return Err("Card index out of range");
            }

            // VelvetChoker: cannot play more than 6 cards per turn (Java: canPlay returns false if counter >= 6)
            if combat_state
                .player
                .has_relic(crate::content::relics::RelicId::VelvetChoker)
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

            target = targeting::resolve_target_request(
                combat_state,
                targeting::validation_for_card_target(def.target),
                target,
            )?;

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
            let card_actions = crate::content::cards::resolve_card_play(
                card_id,
                combat_state,
                &played_card,
                target,
            );
            queue_actions(&mut combat_state.action_queue, card_actions);

            // 8. on_use_card relic hooks (Kunai, Nunchaku, PenNib, etc.)
            let relic_actions = crate::content::relics::hooks::on_use_card(combat_state, card_id);
            queue_actions(&mut combat_state.action_queue, relic_actions);

            // 8b. on_card_played power hooks for ALL creatures (Java: UseCardAction triggers onUseCard)
            // This dispatches powers like Sharp Hide (Guardian), Anger, Hex on monsters,
            // and any player powers that respond to card plays.
            for entity_id in
                std::iter::once(0usize).chain(combat_state.monsters.iter().map(|m| m.id))
            {
                if let Some(powers) = combat_state.power_db.get(&entity_id).cloned() {
                    for power in &powers {
                        let hook_actions = crate::content::powers::resolve_power_on_card_played(
                            power.power_type,
                            combat_state,
                            entity_id,
                            &played_card,
                            power.amount,
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
                        PowerId::DoubleTap
                        | PowerId::DuplicationPower
                        | PowerId::Corruption
                        | PowerId::PenNibPower
                        | PowerId::Vigor => {
                            crate::content::powers::resolve_power_on_use_card(
                                power.power_type,
                                combat_state,
                                &played_card,
                                &mut exhaust_override,
                                false,
                                target,
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
                combat_state,
                &played_card,
                &mut should_exhaust,
            );
            if def.card_type == crate::content::cards::CardType::Power {
                // Power cards are purged after play (removed from game)
                // Card is dropped — not added to any pile or limbo
            } else {
                // Hold card in limbo until UseCardDone fires
                combat_state.limbo.push(played_card);
                combat_state
                    .action_queue
                    .push_back(Action::UseCardDone { should_exhaust });
            }

            Ok(())
        }

        ClientInput::UsePotion {
            potion_index,
            mut target,
        } => {
            let potion = combat_state
                .potions
                .get(potion_index)
                .and_then(|p| p.as_ref())
                .ok_or("Potion index out of range")?;
            let def = crate::content::potions::get_potion_definition(potion.id);
            target = targeting::resolve_target_request(
                combat_state,
                targeting::validation_for_potion_target(def.target_required),
                target,
            )?;
            // Queue UsePotion action — handler at action_handlers.rs does the work
            combat_state.action_queue.push_back(Action::UsePotion {
                slot: potion_index,
                target: target.map(|t| t as usize),
            });
            Ok(())
        }

        ClientInput::DiscardPotion(slot) => {
            combat_state
                .action_queue
                .push_back(Action::DiscardPotion { slot });
            Ok(())
        }

        ClientInput::EndTurn => {
            // Queue end-of-turn processing
            // 1. EndTurnTrigger handles in-hand card effects (Burn, Decay, ethereal exhaust, etc.)
            combat_state.action_queue.push_back(Action::EndTurnTrigger);
            // 2. Relic at_end_of_turn hooks (Orichalcum, CloakClasp, ArtOfWar, etc.)
            let end_turn_relic_actions =
                crate::content::relics::hooks::at_end_of_turn(combat_state);
            queue_actions(&mut combat_state.action_queue, end_turn_relic_actions);
            // 3. Transition: the engine loop will detect CombatProcessing and handle
            //    discarding hand, applying power at_end_of_turn, enemy turns, draw, etc.
            *engine_state = EngineState::CombatProcessing;
            combat_state.current_phase = CombatPhase::TurnTransition;
            Ok(())
        }

        _ => Err("Invalid input for player turn"),
    }
}

fn resolve_pending_choice(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    input: ClientInput,
) -> Result<(), &'static str> {
    let choice = if let EngineState::PendingChoice(c) = engine_state {
        c.clone()
    } else {
        return Err("Not in a pending choice state");
    };

    match choice {
        PendingChoice::ScrySelect {
            cards,
            card_uuids: _,
        } => pending_choices::handle_scry(engine_state, combat_state, cards.len(), input),
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards: count,
            max_cards: _,
            can_cancel: cancellable,
            reason,
        } => pending_choices::handle_hand_select(
            engine_state,
            combat_state,
            &candidate_uuids,
            count as usize,
            false,
            cancellable,
            reason,
            input,
        ),
        PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => pending_choices::handle_grid_select(
            engine_state,
            combat_state,
            &candidate_uuids,
            source_pile,
            min_cards,
            max_cards,
            can_cancel,
            reason,
            input,
        ),
        PendingChoice::DiscoverySelect(ref cards) => {
            // Player picks one card from the discovery options
            if let ClientInput::SubmitDiscoverChoice(idx) = input {
                if idx < cards.len() {
                    let card_id = cards[idx];
                    let uuid = 50000
                        + combat_state.hand.len() as u32
                        + combat_state.discard_pile.len() as u32;
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
        }
        PendingChoice::CardRewardSelect {
            ref cards,
            destination,
            can_skip,
        } => {
            // Player picks one card from the reward options, or Cancel if can_skip
            match input {
                ClientInput::SubmitDiscoverChoice(idx) => {
                    if idx < cards.len() {
                        let card_id = cards[idx];
                        let uuid = 50000
                            + combat_state.hand.len() as u32
                            + combat_state.discard_pile.len() as u32
                            + combat_state.draw_pile.len() as u32;
                        let card = crate::combat::CombatCard::new(card_id, uuid);
                        match destination {
                            crate::action::CardDestination::Hand => {
                                // Java ChooseOneColorless: hand (or discard if full)
                                if combat_state.hand.len() < 10 {
                                    combat_state.hand.push(card);
                                } else {
                                    combat_state.discard_pile.push(card);
                                }
                            }
                            crate::action::CardDestination::DrawPileRandom => {
                                // Java CodexAction: add to draw pile at random position
                                if combat_state.draw_pile.is_empty() {
                                    combat_state.draw_pile.push(card);
                                } else {
                                    let pos = combat_state
                                        .rng
                                        .card_random_rng
                                        .random(combat_state.draw_pile.len() as i32)
                                        as usize;
                                    combat_state
                                        .draw_pile
                                        .insert(pos.min(combat_state.draw_pile.len()), card);
                                }
                            }
                        }
                        *engine_state = EngineState::CombatProcessing;
                        Ok(())
                    } else {
                        Err("Invalid card reward choice index")
                    }
                }
                ClientInput::Cancel if can_skip => {
                    // Java CodexAction: canSkip=true — player can skip picking
                    *engine_state = EngineState::CombatProcessing;
                    Ok(())
                }
                _ => Err("Invalid input for card reward selection"),
            }
        }
        PendingChoice::StanceChoice => {
            // Player picks 0=Wrath, 1=Calm
            if let ClientInput::SubmitDiscoverChoice(idx) = input {
                let stance = match idx {
                    0 => "Wrath",
                    1 => "Calm",
                    _ => return Err("Invalid stance choice (expected 0=Wrath or 1=Calm)"),
                };
                combat_state
                    .action_queue
                    .push_back(Action::EnterStance(stance.to_string()));
                *engine_state = EngineState::CombatProcessing;
                Ok(())
            } else {
                Err("Expected SubmitDiscoverChoice for stance selection")
            }
        }
    }
}

fn hand_select_candidates(
    combat_state: &CombatState,
    filter: crate::state::HandSelectFilter,
) -> Vec<u32> {
    combat_state
        .hand
        .iter()
        .filter(|card| hand_candidate_matches(card, filter))
        .map(|card| card.uuid)
        .collect()
}

fn hand_candidate_matches(
    card: &crate::combat::CombatCard,
    filter: crate::state::HandSelectFilter,
) -> bool {
    let def = crate::content::cards::get_card_definition(card.id);
    match filter {
        crate::state::HandSelectFilter::Any => true,
        crate::state::HandSelectFilter::Upgradeable => {
            (card.id == crate::content::cards::CardId::SearingBlow || card.upgrades == 0)
                && def.card_type != crate::content::cards::CardType::Status
                && def.card_type != crate::content::cards::CardType::Curse
        }
        crate::state::HandSelectFilter::AttackOrPower => {
            matches!(
                def.card_type,
                crate::content::cards::CardType::Attack | crate::content::cards::CardType::Power
            )
        }
    }
}

fn grid_select_candidates(
    combat_state: &CombatState,
    source_pile: crate::state::PileType,
    filter: crate::state::GridSelectFilter,
) -> Vec<u32> {
    let pile: &[crate::combat::CombatCard] = match source_pile {
        crate::state::PileType::Draw => &combat_state.draw_pile,
        crate::state::PileType::Discard => &combat_state.discard_pile,
        crate::state::PileType::Exhaust => &combat_state.exhaust_pile,
        crate::state::PileType::Hand => &combat_state.hand,
        crate::state::PileType::Limbo => &combat_state.limbo,
        crate::state::PileType::MasterDeck => &[],
    };

    pile.iter()
        .filter(|card| grid_candidate_matches(card, filter))
        .map(|card| card.uuid)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::combat::{
        CombatCard, Intent, MonsterEntity, PlayerEntity, Power, RelicBuses, StanceId,
    };
    use crate::content::cards::CardId;
    use crate::content::powers::PowerId;
    use crate::content::relics::{RelicId, RelicState};
    use std::collections::{HashMap, VecDeque};

    fn test_combat() -> CombatState {
        CombatState {
            ascension_level: 0,
            turn_count: 1,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            draw_pile: Vec::new(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            player: PlayerEntity {
                id: 0,
                current_hp: 80,
                max_hp: 80,
                block: 0,
                gold_delta_this_combat: 0,
                gold: 99,
                max_orbs: 0,
                orbs: Vec::new(),
                stance: StanceId::Neutral,
                relics: Vec::new(),
                relic_buses: RelicBuses::default(),
                energy_master: 3,
            },
            monsters: vec![MonsterEntity {
                id: 1,
                monster_type: crate::content::monsters::EnemyId::JawWorm as usize,
                current_hp: 40,
                max_hp: 40,
                block: 0,
                slot: 0,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Unknown,
                move_history: VecDeque::new(),
                intent_dmg: 0,
                logical_position: 0,
            }],
            potions: vec![None, None, None],
            power_db: HashMap::new(),
            action_queue: VecDeque::new(),
            counters: Default::default(),
            card_uuid_counter: 10,
            rng: crate::rng::RngPool::new(123),
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        }
    }

    #[test]
    fn auto_resolves_single_required_hand_choice() {
        let mut engine_state = EngineState::CombatProcessing;
        let mut combat = test_combat();
        let mut card = CombatCard::new(CardId::Strike, 100);
        card.base_damage_mut = 6;
        combat.hand.push(card);
        combat.action_queue.push_back(Action::SuspendForHandSelect {
            min: 1,
            max: 1,
            can_cancel: false,
            filter: crate::state::HandSelectFilter::Upgradeable,
            reason: crate::state::HandSelectReason::Upgrade,
        });

        assert!(tick_engine(&mut engine_state, &mut combat, None));
        assert!(!matches!(engine_state, EngineState::PendingChoice(_)));
        assert_eq!(combat.hand[0].upgrades, 1);
    }

    #[test]
    fn empty_required_hand_choice_safely_noops() {
        let mut engine_state = EngineState::CombatProcessing;
        let mut combat = test_combat();
        combat.action_queue.push_back(Action::SuspendForHandSelect {
            min: 1,
            max: 1,
            can_cancel: false,
            filter: crate::state::HandSelectFilter::Upgradeable,
            reason: crate::state::HandSelectReason::Upgrade,
        });

        assert!(tick_engine(&mut engine_state, &mut combat, None));
        assert!(!matches!(engine_state, EngineState::PendingChoice(_)));
        assert!(combat.action_queue.is_empty());
    }

    #[test]
    fn player_turn_auto_selects_single_target() {
        let mut engine_state = EngineState::CombatPlayerTurn;
        let mut combat = test_combat();
        combat.hand.push(CombatCard::new(CardId::Strike, 100));

        let result = handle_player_turn_input(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );

        assert!(result.is_ok());
        assert!(!combat.action_queue.is_empty());
    }

    #[test]
    fn player_turn_rejects_missing_target_when_multiple_exist() {
        let mut engine_state = EngineState::CombatPlayerTurn;
        let mut combat = test_combat();
        combat.monsters.push(MonsterEntity {
            id: 2,
            monster_type: crate::content::monsters::EnemyId::JawWorm as usize,
            current_hp: 35,
            max_hp: 35,
            block: 0,
            slot: 1,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            next_move_byte: 0,
            current_intent: Intent::Unknown,
            move_history: VecDeque::new(),
            intent_dmg: 0,
            logical_position: 1,
        });
        combat.hand.push(CombatCard::new(CardId::Strike, 100));

        let result = handle_player_turn_input(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );

        assert_eq!(
            result.unwrap_err(),
            "Multiple targets available. Must specify a target."
        );
    }

    #[test]
    fn strike_with_pen_nib_queues_power_removal() {
        let mut engine_state = EngineState::CombatPlayerTurn;
        let mut combat = test_combat();
        combat.hand.push(CombatCard::new(CardId::Strike, 100));
        combat.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::PenNibPower,
                amount: 1,
                extra_data: 0,
                just_applied: false,
            }],
        );

        let result = handle_player_turn_input(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        );

        assert!(result.is_ok());
        assert!(combat.action_queue.iter().any(|action| {
            matches!(
                action,
                Action::RemovePower {
                    target: 0,
                    power_id: PowerId::PenNibPower
                }
            )
        }));
    }

    #[test]
    fn rage_grants_block_when_attack_is_played_after_it() {
        let mut engine_state = EngineState::CombatPlayerTurn;
        let mut combat = test_combat();
        combat.hand.push(CombatCard::new(CardId::Strike, 100));
        combat.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Rage,
                amount: 3,
                extra_data: 0,
                just_applied: false,
            }],
        );

        let result = handle_player_turn_input(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        );

        assert!(result.is_ok());
        assert!(combat.action_queue.iter().any(|action| {
            matches!(
                action,
                Action::GainBlock {
                    target: 0,
                    amount: 3
                }
            )
        }));
    }

    #[test]
    fn rage_still_grants_block_when_attack_kills_last_monster() {
        let mut engine_state = EngineState::CombatPlayerTurn;
        let mut combat = test_combat();
        combat.monsters[0].current_hp = 6;
        combat.monsters[0].max_hp = 6;
        combat.hand.push(CombatCard::new(CardId::Strike, 100));
        combat.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Rage,
                amount: 3,
                extra_data: 0,
                just_applied: false,
            }],
        );

        let _alive = tick_until_stable_turn(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        );

        assert_eq!(combat.player.block, 3);
        assert!(matches!(engine_state, EngineState::RewardScreen(_)));
    }

    #[test]
    fn wild_strike_adds_wound_to_draw_pile_not_discard() {
        let mut engine_state = EngineState::CombatPlayerTurn;
        let mut combat = test_combat();
        combat.hand.push(CombatCard::new(CardId::WildStrike, 100));

        let result = handle_player_turn_input(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        );

        assert!(result.is_ok());
        assert!(combat.action_queue.iter().any(|action| {
            matches!(
                action,
                Action::MakeTempCardInDrawPile {
                    card_id: CardId::Wound,
                    amount: 1,
                    ..
                }
            )
        }));
        assert!(!combat.action_queue.iter().any(|action| {
            matches!(
                action,
                Action::MakeTempCardInDiscard {
                    card_id: CardId::Wound,
                    amount: 1,
                    ..
                }
            )
        }));
    }

    #[test]
    fn vigor_is_removed_after_playing_attack_card() {
        let mut engine_state = EngineState::CombatPlayerTurn;
        let mut combat = test_combat();
        combat.hand.push(CombatCard::new(CardId::Strike, 100));
        combat.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Vigor,
                amount: 8,
                extra_data: 0,
                just_applied: false,
            }],
        );

        let _alive = tick_until_stable_turn(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        );

        let player_powers = combat.power_db.get(&0).cloned().unwrap_or_default();
        assert!(!player_powers.iter().any(|p| p.power_type == PowerId::Vigor));
    }

    #[test]
    fn lose_hp_triggers_self_forming_clay_and_rupture_hooks() {
        let mut combat = test_combat();
        combat
            .player
            .add_relic(RelicState::new(RelicId::SelfFormingClay));
        combat.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Rupture,
                amount: 1,
                extra_data: 0,
                just_applied: false,
            }],
        );

        crate::engine::action_handlers::damage::handle_lose_hp(0, 3, &mut combat);

        assert_eq!(combat.player.current_hp, 77);
        assert!(combat.action_queue.iter().any(|action| {
            matches!(
                action,
                Action::ApplyPower {
                    target: 0,
                    power_id: PowerId::NextTurnBlock,
                    amount: 3,
                    ..
                }
            )
        }));
        assert!(combat.action_queue.iter().any(|action| {
            matches!(
                action,
                Action::ApplyPower {
                    target: 0,
                    power_id: PowerId::Strength,
                    amount: 3,
                    ..
                }
            )
        }));
    }

    #[test]
    fn bloodletting_applies_next_turn_block_from_self_forming_clay_after_stabilizing() {
        let mut engine_state = EngineState::CombatPlayerTurn;
        let mut combat = test_combat();
        combat
            .player
            .add_relic(RelicState::new(RelicId::SelfFormingClay));
        combat.hand.push(CombatCard::new(CardId::Bloodletting, 100));

        let alive = tick_until_stable_turn(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );

        assert!(alive);
        assert_eq!(combat.player.current_hp, 77);
        assert_eq!(combat.energy, 5);
        let player_powers = combat.power_db.get(&0).cloned().unwrap_or_default();
        assert!(player_powers.iter().any(|p| {
            p.power_type == PowerId::NextTurnBlock && p.amount == 3
        }));
    }

    #[test]
    fn bloodletting_from_live_style_snapshot_keeps_next_turn_block() {
        let snapshot = serde_json::json!({
            "turn": 7,
            "room_type": "MonsterRoomBoss",
            "player": {
                "current_hp": 25,
                "max_hp": 80,
                "block": 0,
                "energy": 3,
                "powers": [
                    {"id": "Dexterity", "amount": 1}
                ]
            },
            "monsters": [
                {
                    "id": "Hexaghost",
                    "current_hp": 134,
                    "max_hp": 250,
                    "block": 12,
                    "intent": "ATTACK",
                    "move_base_damage": 5,
                    "move_adjusted_damage": 7,
                    "move_hits": 2,
                    "move_id": 2,
                    "powers": [
                        {"id": "Strength", "amount": 2}
                    ]
                }
            ],
            "hand": [
                {"id": "Defend_R", "uuid": "h1", "upgrades": 0, "cost": 1},
                {"id": "Heavy Blade", "uuid": "h2", "upgrades": 0, "cost": 2},
                {"id": "Bloodletting", "uuid": "h3", "upgrades": 0, "cost": 0},
                {"id": "Defend_R", "uuid": "h4", "upgrades": 0, "cost": 1},
                {"id": "Defend_R", "uuid": "h5", "upgrades": 0, "cost": 1}
            ],
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "potions": [],
            "relics": [
                {"id": "Burning Blood", "counter": -1},
                {"id": "Kunai", "counter": 0},
                {"id": "Self Forming Clay", "counter": -1}
            ]
        });

        let mut engine_state = EngineState::CombatPlayerTurn;
        let mut combat = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);

        let alive = tick_until_stable_turn(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 2,
                target: None,
            },
        );

        assert!(alive);
        assert_eq!(combat.player.current_hp, 22);
        assert_eq!(combat.energy, 5);
        let player_powers = combat.power_db.get(&0).cloned().unwrap_or_default();
        assert!(player_powers.iter().any(|p| {
            p.power_type == PowerId::NextTurnBlock && p.amount == 3
        }));
    }

    #[test]
    fn cloned_live_style_state_keeps_self_forming_clay_bus_for_bloodletting() {
        let snapshot = serde_json::json!({
            "turn": 7,
            "room_type": "MonsterRoomBoss",
            "player": {
                "current_hp": 25,
                "max_hp": 80,
                "block": 0,
                "energy": 3,
                "powers": [
                    {"id": "Dexterity", "amount": 1}
                ]
            },
            "monsters": [
                {
                    "id": "Hexaghost",
                    "current_hp": 134,
                    "max_hp": 250,
                    "block": 12,
                    "intent": "ATTACK",
                    "move_base_damage": 5,
                    "move_adjusted_damage": 7,
                    "move_hits": 2,
                    "move_id": 2,
                    "powers": [
                        {"id": "Strength", "amount": 2}
                    ]
                }
            ],
            "hand": [
                {"id": "Defend_R", "uuid": "h1", "upgrades": 0, "cost": 1},
                {"id": "Heavy Blade", "uuid": "h2", "upgrades": 0, "cost": 2},
                {"id": "Bloodletting", "uuid": "h3", "upgrades": 0, "cost": 0},
                {"id": "Defend_R", "uuid": "h4", "upgrades": 0, "cost": 1},
                {"id": "Defend_R", "uuid": "h5", "upgrades": 0, "cost": 1}
            ],
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "potions": [],
            "relics": [
                {"id": "Burning Blood", "counter": -1},
                {"id": "Kunai", "counter": 0},
                {"id": "Self Forming Clay", "counter": -1}
            ]
        });

        let truth = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);
        let mut combat = truth.clone();
        let mut engine_state = EngineState::CombatPlayerTurn;

        let alive = tick_until_stable_turn(
            &mut engine_state,
            &mut combat,
            ClientInput::PlayCard {
                card_index: 2,
                target: None,
            },
        );

        assert!(alive);
        let player_powers = combat.power_db.get(&0).cloned().unwrap_or_default();
        assert!(player_powers.iter().any(|p| {
            p.power_type == PowerId::NextTurnBlock && p.amount == 3
        }));
    }

    #[test]
    fn stone_calendar_uses_relic_counter_not_turn_count() {
        let mut combat = test_combat();
        let mut relic = RelicState::new(RelicId::StoneCalendar);
        relic.counter = 6;
        combat.player.add_relic(relic);
        combat.turn_count = 6;

        let no_trigger = crate::content::relics::stone_calendar::at_end_of_turn(&combat, 6);
        assert!(no_trigger.is_empty());

        let turn_start = crate::content::relics::stone_calendar::at_turn_start(6);
        assert_eq!(turn_start.len(), 1);
        match &turn_start[0].action {
            Action::UpdateRelicCounter { relic_id, counter } => {
                assert_eq!(*relic_id, RelicId::StoneCalendar);
                assert_eq!(*counter, 7);
            }
            other => panic!("unexpected action: {other:?}"),
        }

        let trigger = crate::content::relics::stone_calendar::at_end_of_turn(&combat, 7);
        assert!(!trigger.is_empty());
    }

    #[test]
    fn flight_restores_to_stored_amount_at_turn_start() {
        let mut combat = test_combat();
        combat.monsters[0].monster_type = crate::content::monsters::EnemyId::Byrd as usize;
        combat.power_db.insert(
            1,
            vec![Power {
                power_type: PowerId::Flight,
                amount: 1,
                extra_data: 3,
                just_applied: false,
            }],
        );

        let actions =
            crate::content::powers::resolve_power_at_turn_start(PowerId::Flight, &combat, 1, 1);

        assert!(actions.iter().any(|action| {
            matches!(
                action,
                Action::ApplyPower {
                    target: 1,
                    power_id: PowerId::Flight,
                    amount: 2,
                    ..
                }
            )
        }));
    }

    #[test]
    fn flight_does_not_reduce_when_attack_is_lethal() {
        let mut combat = test_combat();
        combat.monsters[0].monster_type = crate::content::monsters::EnemyId::Byrd as usize;
        combat.monsters[0].current_hp = 0;
        combat.monsters[0].is_dying = true;

        let actions = crate::content::powers::core::flight::on_attacked(&combat, 1, 5, 0, 1);
        assert!(actions.is_empty());
    }

    #[test]
    fn modify_card_misc_updates_limbo_card_and_stacks() {
        let mut combat = test_combat();
        let mut rampage = CombatCard::new(CardId::Rampage, 4242);
        rampage.misc_value = 5;
        combat.limbo.push(rampage);

        crate::engine::action_handlers::cards::handle_modify_card_misc(4242, 5, &mut combat);

        assert_eq!(combat.limbo[0].misc_value, 10);
    }

    #[test]
    fn corruption_power_uses_java_negative_sentinel_amount() {
        let mut combat = test_combat();

        crate::engine::action_handlers::powers::handle_apply_power(
            0,
            0,
            PowerId::Corruption,
            1,
            &mut combat,
        );

        assert_eq!(combat.get_power(0, PowerId::Corruption), -1);
    }

    #[test]
    fn use_card_done_exhaust_triggers_dark_embrace_draw() {
        let mut combat = test_combat();
        combat
            .power_db
            .entry(0)
            .or_default()
            .push(Power { power_type: PowerId::DarkEmbrace, amount: 1, extra_data: 0, just_applied: false });
        combat.draw_pile.push(CombatCard::new(CardId::Strike, 500));
        combat.limbo.push(CombatCard::new(CardId::Defend, 400));

        crate::engine::action_handlers::cards::handle_use_card_done(true, &mut combat);

        let mut engine_state = EngineState::CombatProcessing;
        while !combat.action_queue.is_empty() {
            assert!(tick_engine(&mut engine_state, &mut combat, None));
            if matches!(engine_state, EngineState::CombatPlayerTurn) && !combat.action_queue.is_empty() {
                engine_state = EngineState::CombatProcessing;
            }
        }

        assert_eq!(combat.hand.len(), 1);
        assert_eq!(combat.hand[0].id, CardId::Strike);
        assert_eq!(combat.exhaust_pile.len(), 1);
        assert_eq!(combat.exhaust_pile[0].id, CardId::Defend);
    }

    #[test]
    fn sword_boomerang_reduces_plated_armor_once_per_hit() {
        let mut combat = test_combat();
        combat.monsters[0].monster_type =
            crate::content::monsters::EnemyId::ShelledParasite as usize;
        combat.power_db.insert(
            1,
            vec![Power {
                power_type: PowerId::PlatedArmor,
                amount: 6,
                extra_data: 0,
                just_applied: false,
            }],
        );

        for _ in 0..3 {
            crate::engine::action_handlers::execute_action(
                Action::AttackDamageRandomEnemy {
                    base_damage: 3,
                    damage_type: crate::action::DamageType::Normal,
                    applies_target_modifiers: true,
                },
                &mut combat,
            );
            while let Some(action) = combat.action_queue.pop_front() {
                crate::engine::action_handlers::execute_action(action, &mut combat);
            }
        }

        let plated = combat
            .power_db
            .get(&1)
            .and_then(|powers| powers.iter().find(|p| p.power_type == PowerId::PlatedArmor))
            .map(|p| p.amount);
        assert_eq!(plated, Some(3));
    }
}

fn grid_candidate_matches(
    card: &crate::combat::CombatCard,
    filter: crate::state::GridSelectFilter,
) -> bool {
    let def = crate::content::cards::get_card_definition(card.id);
    match filter {
        crate::state::GridSelectFilter::Any => true,
        crate::state::GridSelectFilter::NonExhume => {
            card.id != crate::content::cards::CardId::Exhume
        }
        crate::state::GridSelectFilter::Skill => {
            def.card_type == crate::content::cards::CardType::Skill
        }
        crate::state::GridSelectFilter::Attack => {
            def.card_type == crate::content::cards::CardType::Attack
        }
    }
}

pub fn queue_actions(
    queue: &mut std::collections::VecDeque<Action>,
    actions: SmallVec<[ActionInfo; 4]>,
) {
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
    let alive_monsters: Vec<_> = combat_state
        .monsters
        .iter()
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
                new_intent_dmg =
                    crate::content::powers::calculate_monster_damage(damage, mid, 0, combat_state);
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
pub fn tick_until_stable_turn(
    es: &mut EngineState,
    cs: &mut CombatState,
    input: ClientInput,
) -> bool {
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
            EngineState::CombatProcessing => {}
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
