// action_handlers/cards.rs — Card pile management domain
//
// Handles: DrawCards, EmptyDeckShuffle, DiscardCard, ExhaustCard, MoveCard,
//          MakeTempCard*, MakeCopy*, MakeRandom*, PlayCardDirect, PlayTopCard,
//          UseCardDone, UpgradeCard, UpgradeRandomCard, UpgradeAllInHand, UpgradeAllBurns,
//          ReduceAllHandCosts, RandomizeHandCosts, ModifyCardMisc, MummifiedHandEffect,
//          UsePotion, DiscardPotion, ObtainPotion, ObtainSpecificPotion, Scry,
//          EndTurnTrigger, StartTurnTrigger, PostDrawTrigger, BattleStartTrigger, ClearCardQueue,
//          AddCardToMasterDeck, MakeTempCardInDiscardAndDeck, SuspendForCardReward

use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use crate::content::cards::CardId;
use crate::content::powers::store;
use crate::content::powers::PowerId;
use crate::engine::targeting;

fn queue_exhaust_triggers(card: &crate::combat::CombatCard, state: &mut CombatState) {
    let mut after_actions = crate::content::relics::hooks::on_exhaust(state);
    let card_hooks = crate::content::cards::resolve_card_on_exhaust(card, state);
    after_actions.extend(card_hooks);

    for (owner, powers) in store::powers_snapshot_all(state) {
        for power in powers {
            let actions = crate::content::powers::resolve_power_on_exhaust(
                power.power_type,
                state,
                owner,
                power.amount,
                card.uuid,
                card.id,
            );
            for action in actions {
                after_actions.push(ActionInfo {
                    action,
                    insertion_mode: AddTo::Bottom,
                });
            }
        }
    }

    crate::engine::core::queue_actions(&mut state.engine.action_queue, after_actions);
}

pub fn move_card_to_exhaust_pile(card: crate::combat::CombatCard, state: &mut CombatState) {
    queue_exhaust_triggers(&card, state);
    state.zones.exhaust_pile.push(card);
}

pub fn handle_draw_cards(amount: u32, state: &mut CombatState) {
    let has_no_draw = store::has_power(state, 0, PowerId::NoDraw);
    if has_no_draw {
        return;
    }
    for _ in 0..amount {
        if state.zones.hand.len() >= 10 {
            break;
        }
        if state.zones.draw_pile.is_empty() && !state.zones.discard_pile.is_empty() {
            state.zones.draw_pile.append(&mut state.zones.discard_pile);
            crate::rng::shuffle_with_random_long(
                &mut state.zones.draw_pile,
                &mut state.rng.shuffle_rng,
            );
            state.zones.draw_pile.reverse();
            let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
            crate::engine::core::queue_actions(&mut state.engine.action_queue, shuffle_actions);
        }
        if state.zones.draw_pile.is_empty() {
            break;
        }
        let mut card = state.zones.draw_pile.remove(0);

        if card.id == CardId::Void {
            let void_actions = crate::content::cards::status::void::on_drawn(state);
            crate::engine::core::queue_actions(&mut state.engine.action_queue, void_actions);
        }

        // Apply pre-draw powers (like Corruption, Confusion)
        for power in &store::powers_snapshot_for(state, 0) {
            crate::content::powers::resolve_power_on_card_draw(power.power_type, state, &mut card);
        }

        state.zones.hand.push(card.clone());

        // Post-draw actions for powers and specific card hooks
        for power in &store::powers_snapshot_for(state, 0) {
            let actions = crate::content::powers::resolve_power_on_card_drawn(
                power.power_type,
                state,
                0,
                power.amount,
                card.uuid,
            );
            for a in actions {
                state.engine.action_queue.push_back(a);
            }
        }
    }
}

pub fn handle_empty_deck_shuffle(state: &mut CombatState) {
    if state.zones.draw_pile.is_empty() && !state.zones.discard_pile.is_empty() {
        state.zones.draw_pile.append(&mut state.zones.discard_pile);
        crate::rng::shuffle_with_random_long(
            &mut state.zones.draw_pile,
            &mut state.rng.shuffle_rng,
        );
        state.zones.draw_pile.reverse();
        let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
        crate::engine::core::queue_actions(&mut state.engine.action_queue, shuffle_actions);
    }
}

pub fn handle_shuffle_discard_into_draw(state: &mut CombatState) {
    if state.zones.discard_pile.is_empty() {
        return;
    }
    state.zones.draw_pile.append(&mut state.zones.discard_pile);
    crate::rng::shuffle_with_random_long(&mut state.zones.draw_pile, &mut state.rng.shuffle_rng);
    state.zones.draw_pile.reverse();
    let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
    crate::engine::core::queue_actions(&mut state.engine.action_queue, shuffle_actions);
}

pub fn handle_discard_card(card_uuid: u32, state: &mut CombatState) {
    if let Some(pos) = state.zones.hand.iter().position(|c| c.uuid == card_uuid) {
        let card = state.zones.hand.remove(pos);
        state.zones.discard_pile.push(card);
        let discard_actions = crate::content::relics::hooks::on_discard(state);
        crate::engine::core::queue_actions(&mut state.engine.action_queue, discard_actions);
    }
}

pub fn handle_exhaust_card(
    card_uuid: u32,
    source_pile: crate::state::PileType,
    state: &mut CombatState,
) {
    let mut removed_card = None;
    match source_pile {
        crate::state::PileType::Hand => {
            if let Some(pos) = state.zones.hand.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.zones.hand.remove(pos));
            }
        }
        crate::state::PileType::Draw => {
            if let Some(pos) = state
                .zones
                .draw_pile
                .iter()
                .position(|c| c.uuid == card_uuid)
            {
                removed_card = Some(state.zones.draw_pile.remove(pos));
            }
        }
        crate::state::PileType::Discard => {
            if let Some(pos) = state
                .zones
                .discard_pile
                .iter()
                .position(|c| c.uuid == card_uuid)
            {
                removed_card = Some(state.zones.discard_pile.remove(pos));
            }
        }
        crate::state::PileType::Limbo => {
            if let Some(pos) = state.zones.limbo.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.zones.limbo.remove(pos));
            }
        }
        _ => {}
    }
    if let Some(card) = removed_card {
        move_card_to_exhaust_pile(card, state);
    }
}

pub fn handle_move_card(
    card_uuid: u32,
    from: crate::state::PileType,
    to: crate::state::PileType,
    state: &mut CombatState,
) {
    let mut removed_card = None;
    match from {
        crate::state::PileType::Hand => {
            if let Some(pos) = state.zones.hand.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.zones.hand.remove(pos));
            }
        }
        crate::state::PileType::Draw => {
            if let Some(pos) = state
                .zones
                .draw_pile
                .iter()
                .position(|c| c.uuid == card_uuid)
            {
                removed_card = Some(state.zones.draw_pile.remove(pos));
            }
        }
        crate::state::PileType::Discard => {
            if let Some(pos) = state
                .zones
                .discard_pile
                .iter()
                .position(|c| c.uuid == card_uuid)
            {
                removed_card = Some(state.zones.discard_pile.remove(pos));
            }
        }
        crate::state::PileType::Exhaust => {
            if let Some(pos) = state
                .zones
                .exhaust_pile
                .iter()
                .position(|c| c.uuid == card_uuid)
            {
                removed_card = Some(state.zones.exhaust_pile.remove(pos));
            }
        }
        crate::state::PileType::Limbo => {
            if let Some(pos) = state.zones.limbo.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.zones.limbo.remove(pos));
            }
        }
        _ => {}
    }
    if let Some(card) = removed_card {
        match to {
            crate::state::PileType::Hand => {
                if state.zones.hand.len() < 10 {
                    state.zones.hand.push(card);
                } else {
                    state.zones.discard_pile.push(card);
                }
            }
            crate::state::PileType::Draw => state.zones.draw_pile.insert(0, card),
            crate::state::PileType::Discard => state.zones.discard_pile.push(card),
            crate::state::PileType::Exhaust => {
                if matches!(from, crate::state::PileType::Exhaust) {
                    state.zones.exhaust_pile.push(card);
                } else {
                    move_card_to_exhaust_pile(card, state);
                }
            }
            _ => {}
        }
    }
}

pub fn handle_remove_card_from_pile(
    card_uuid: u32,
    from: crate::state::PileType,
    state: &mut CombatState,
) {
    let source = match from {
        crate::state::PileType::Hand => &mut state.zones.hand,
        crate::state::PileType::Draw => &mut state.zones.draw_pile,
        crate::state::PileType::Discard => &mut state.zones.discard_pile,
        crate::state::PileType::Exhaust => &mut state.zones.exhaust_pile,
        crate::state::PileType::Limbo => &mut state.zones.limbo,
        crate::state::PileType::MasterDeck => return,
    };
    if let Some(pos) = source.iter().position(|c| c.uuid == card_uuid) {
        source.remove(pos);
    }
}

pub fn handle_make_temp_card_in_hand(
    card_id: CardId,
    amount: u8,
    upgraded: bool,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        state.zones.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.zones.card_uuid_counter);
        if upgraded {
            card.upgrades = 1;
        }
        if state.zones.hand.len() < 10 {
            state.zones.hand.push(card);
        } else {
            state.zones.discard_pile.push(card);
        }
    }
}

pub fn handle_make_temp_card_in_discard(
    card_id: CardId,
    amount: u8,
    upgraded: bool,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        state.zones.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.zones.card_uuid_counter);
        if upgraded {
            card.upgrades = 1;
        }
        state.zones.discard_pile.push(card);
    }
}

pub fn handle_make_temp_card_in_draw_pile(
    card_id: CardId,
    amount: u8,
    random_spot: bool,
    upgraded: bool,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        state.zones.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.zones.card_uuid_counter);
        if upgraded {
            card.upgrades = 1;
        }
        if random_spot && !state.zones.draw_pile.is_empty() {
            let idx = state
                .rng
                .card_random_rng
                .random(state.zones.draw_pile.len() as i32) as usize;
            state.zones.draw_pile.insert(idx, card);
        } else {
            state.zones.draw_pile.push(card);
        }
    }
}

pub fn handle_make_copy_in_hand(
    original: Box<crate::combat::CombatCard>,
    amount: u8,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        state.zones.card_uuid_counter += 1;
        let mut card = (*original).clone();
        card.uuid = state.zones.card_uuid_counter;
        if state.zones.hand.len() < 10 {
            state.zones.hand.push(card);
        } else {
            state.zones.discard_pile.push(card);
        }
    }
}

pub fn handle_make_copy_in_discard(
    original: Box<crate::combat::CombatCard>,
    amount: u8,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        state.zones.card_uuid_counter += 1;
        let mut card = (*original).clone();
        card.uuid = state.zones.card_uuid_counter;
        state.zones.discard_pile.push(card);
    }
}

pub fn handle_make_temp_card_in_discard_and_deck(
    card_id: CardId,
    amount: u8,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        state.zones.card_uuid_counter += 1;
        let card = crate::combat::CombatCard::new(card_id, state.zones.card_uuid_counter);
        state.zones.discard_pile.push(card.clone());
        let pos = state
            .rng
            .card_random_rng
            .random(state.zones.draw_pile.len() as i32) as usize;
        state.zones.draw_pile.insert(pos, card);
    }
}

pub fn handle_reduce_all_hand_costs(amount: u8, state: &mut CombatState) {
    for card in state.zones.hand.iter_mut() {
        let def = crate::content::cards::get_card_definition(card.id);
        if def.cost >= 0 {
            let current = card.cost_for_turn.unwrap_or(def.cost as u8);
            card.cost_for_turn = Some(current.saturating_sub(amount));
        }
    }
}

pub fn handle_enlightenment(permanent: bool, state: &mut CombatState) {
    for card in state.zones.hand.iter_mut() {
        let def = crate::content::cards::get_card_definition(card.id);
        if def.cost < 0 {
            continue;
        }

        let current = card.cost_for_turn.unwrap_or(def.cost as u8);
        if current > 1 {
            card.cost_for_turn = Some(1);
        }

        if permanent && def.cost > 1 {
            card.cost_modifier = 1 - def.cost;
        }
    }
}

pub fn handle_madness(state: &mut CombatState) {
    let better_possible: Vec<usize> = state
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(_, card)| card.get_cost() > 0)
        .map(|(idx, _)| idx)
        .collect();

    let possible: Vec<usize> = if better_possible.is_empty() {
        state
            .zones
            .hand
            .iter()
            .enumerate()
            .filter(|(_, card)| {
                let def = crate::content::cards::get_card_definition(card.id);
                def.cost > 0
            })
            .map(|(idx, _)| idx)
            .collect()
    } else {
        Vec::new()
    };

    let pool = if !better_possible.is_empty() {
        better_possible
    } else {
        possible
    };

    if pool.is_empty() {
        return;
    }

    let pick = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
    let card = &mut state.zones.hand[pool[pick]];
    let def = crate::content::cards::get_card_definition(card.id);
    let base_cost = crate::content::cards::upgraded_base_cost_override(card).unwrap_or(def.cost);
    if base_cost >= 0 {
        card.cost_modifier = -base_cost;
        card.cost_for_turn = Some(0);
    }
}

pub fn handle_upgrade_all_in_hand(state: &mut CombatState) {
    for card in state.zones.hand.iter_mut() {
        card.upgrades += 1;
    }
}

pub fn handle_upgrade_all_burns(state: &mut CombatState) {
    for card in state
        .zones
        .draw_pile
        .iter_mut()
        .chain(state.zones.discard_pile.iter_mut())
        .chain(state.zones.hand.iter_mut())
    {
        if card.id == CardId::Burn {
            card.upgrades += 1;
        }
    }
}

pub fn handle_upgrade_card(card_uuid: u32, state: &mut CombatState) {
    for card in state
        .zones
        .hand
        .iter_mut()
        .chain(state.zones.draw_pile.iter_mut())
        .chain(state.zones.discard_pile.iter_mut())
    {
        if card.uuid == card_uuid {
            card.upgrades += 1;
            break;
        }
    }
}

pub fn handle_upgrade_random_card(state: &mut CombatState) {
    let upgradeable_uuids: Vec<u32> = state
        .zones
        .hand
        .iter()
        .filter(|c| {
            c.upgrades == 0
                && crate::content::cards::get_card_definition(c.id).card_type
                    != crate::content::cards::CardType::Status
        })
        .map(|c| c.uuid)
        .collect();
    if !upgradeable_uuids.is_empty() {
        let mut shuffled = upgradeable_uuids;
        crate::rng::shuffle_with_random_long(&mut shuffled, &mut state.rng.shuffle_rng);
        let target_uuid = shuffled[0];
        if let Some(card) = state.zones.hand.iter_mut().find(|c| c.uuid == target_uuid) {
            card.upgrades += 1;
        }
    }
}

pub fn handle_modify_card_misc(card_uuid: u32, amount: i32, state: &mut CombatState) {
    for card in state
        .zones
        .hand
        .iter_mut()
        .chain(state.zones.draw_pile.iter_mut())
        .chain(state.zones.discard_pile.iter_mut())
        .chain(state.zones.exhaust_pile.iter_mut())
        .chain(state.zones.limbo.iter_mut())
    {
        if card.uuid == card_uuid {
            card.misc_value += amount;
            break;
        }
    }
}

pub fn handle_modify_card_damage(card_uuid: u32, amount: i32, state: &mut CombatState) {
    for card in state
        .zones
        .hand
        .iter_mut()
        .chain(state.zones.draw_pile.iter_mut())
        .chain(state.zones.discard_pile.iter_mut())
        .chain(state.zones.exhaust_pile.iter_mut())
        .chain(state.zones.limbo.iter_mut())
        .chain(
            state
                .zones
                .queued_cards
                .iter_mut()
                .map(|queued| &mut queued.card),
        )
    {
        if card.uuid == card_uuid {
            let def = crate::content::cards::get_card_definition(card.id);
            let upgraded_base = def.base_damage + (card.upgrades as i32) * def.upgrade_damage;
            let current = card.base_damage_override.unwrap_or(upgraded_base);
            card.base_damage_override = Some((current + amount).max(0));
        }
    }
}

pub fn handle_randomize_hand_costs(state: &mut CombatState) {
    for card in state.zones.hand.iter_mut() {
        let base_cost = crate::content::cards::get_card_definition(card.id).cost;
        if base_cost >= 0 {
            let new_cost = state.rng.card_random_rng.random(3) as u8;
            card.cost_for_turn = Some(new_cost);
        }
    }
}

pub fn handle_mummified_hand_effect(state: &mut CombatState) {
    let reserved = state.reserved_card_uuids_for_queue_sensitive_effects();
    let eligible: Vec<usize> = state
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            let def = crate::content::cards::get_card_definition(c.id);
            let current = c.cost_for_turn.unwrap_or(def.cost as u8);
            def.cost > 0 && current > 0 && !c.free_to_play_once && !reserved.contains(&c.uuid)
        })
        .map(|(i, _)| i)
        .collect();
    if !eligible.is_empty() {
        let idx = state.rng.card_random_rng.random(eligible.len() as i32 - 1) as usize;
        let card_idx = eligible[idx];
        let card = &mut state.zones.hand[card_idx];
        // Java Mummified Hand sets costForTurn directly to 0.
        card.cost_for_turn = Some(0);
    }
}

pub fn handle_make_random_card_in_hand(
    card_type: Option<crate::content::cards::CardType>,
    cost_for_turn: Option<u8>,
    state: &mut CombatState,
) {
    let mut pool: Vec<CardId> = Vec::new();
    for &rarity in &[
        crate::content::cards::CardRarity::Common,
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Rare,
    ] {
        for &id in crate::content::cards::ironclad_pool_for_rarity(rarity) {
            if let Some(ct) = card_type {
                let def = crate::content::cards::get_card_definition(id);
                if def.card_type != ct {
                    continue;
                }
            }
            pool.push(id);
        }
    }
    if !pool.is_empty() {
        let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        state.zones.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.zones.card_uuid_counter);
        if let Some(cost) = cost_for_turn {
            card.cost_for_turn = Some(cost);
        }
        if state.zones.hand.len() < 10 {
            state.zones.hand.push(card);
        } else {
            state.zones.discard_pile.push(card);
        }
    }
}

pub fn handle_make_random_card_in_draw_pile(
    card_type: Option<crate::content::cards::CardType>,
    cost_for_turn: Option<u8>,
    random_spot: bool,
    state: &mut CombatState,
) {
    let mut pool: Vec<CardId> = Vec::new();
    for &rarity in &[
        crate::content::cards::CardRarity::Common,
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Rare,
    ] {
        for &id in crate::content::cards::ironclad_pool_for_rarity(rarity) {
            if let Some(ct) = card_type {
                let def = crate::content::cards::get_card_definition(id);
                if def.card_type != ct {
                    continue;
                }
            }
            pool.push(id);
        }
    }
    if !pool.is_empty() {
        let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        state.zones.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.zones.card_uuid_counter);
        if let Some(cost) = cost_for_turn {
            card.cost_for_turn = Some(cost);
        }
        if random_spot && !state.zones.draw_pile.is_empty() {
            let idx = state
                .rng
                .card_random_rng
                .random(state.zones.draw_pile.len() as i32) as usize;
            state.zones.draw_pile.insert(idx, card);
        } else {
            state.zones.draw_pile.push(card);
        }
    }
}

pub fn handle_draw_pile_to_hand_by_type(
    amount: u8,
    card_type: crate::content::cards::CardType,
    state: &mut CombatState,
) {
    let mut candidates: Vec<u32> = state
        .zones
        .draw_pile
        .iter()
        .filter(|card| crate::content::cards::get_card_definition(card.id).card_type == card_type)
        .map(|card| card.uuid)
        .collect();

    for _ in 0..amount {
        if candidates.is_empty() {
            break;
        }
        let idx = state
            .rng
            .card_random_rng
            .random(candidates.len() as i32 - 1) as usize;
        let chosen_uuid = candidates.swap_remove(idx);
        if let Some(pos) = state
            .zones
            .draw_pile
            .iter()
            .position(|card| card.uuid == chosen_uuid)
        {
            let card = state.zones.draw_pile.remove(pos);
            if state.zones.hand.len() < 10 {
                state.zones.hand.push(card);
            } else {
                state.zones.discard_pile.push(card);
            }
        }
    }
}

pub fn handle_make_random_colorless_card_in_hand(
    cost_for_turn: Option<u8>,
    upgraded: bool,
    state: &mut CombatState,
) {
    let pool = state.colorless_combat_pool();
    if !pool.is_empty() {
        let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        state.zones.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.zones.card_uuid_counter);
        if upgraded {
            card.upgrades = 1;
        }
        if let Some(cost) = cost_for_turn {
            card.cost_for_turn = Some(cost);
        }
        if state.zones.hand.len() < 10 {
            state.zones.hand.push(card);
        } else {
            state.zones.discard_pile.push(card);
        }
    }
}

pub fn handle_use_card_done(should_exhaust: bool, state: &mut CombatState) {
    if let Some(card) = state.zones.limbo.pop() {
        if should_exhaust {
            move_card_to_exhaust_pile(card, state);
        } else {
            state.zones.discard_pile.push(card);
        }
    }

    if state.turn.counters.early_end_turn_pending {
        state.turn.counters.early_end_turn_pending = false;
        state.turn.current_phase = crate::combat::CombatPhase::TurnTransition;
        state.engine.action_queue.push_back(Action::EndTurnTrigger);
    }
}

pub fn handle_queue_early_end_turn(state: &mut CombatState) {
    state.turn.counters.early_end_turn_pending = true;
}

fn execute_played_card(
    mut played_card: crate::combat::CombatCard,
    target: Option<usize>,
    purge: bool,
    state: &mut CombatState,
) {
    let card_id = played_card.id;
    let def = crate::content::cards::get_card_definition(card_id);

    crate::content::cards::evaluate_card(&mut played_card, state, target);

    let card_actions =
        crate::content::cards::resolve_card_play(card_id, state, &played_card, target);
    crate::engine::core::queue_actions(&mut state.engine.action_queue, card_actions);

    let passive_card_actions = crate::content::cards::on_play_card(&played_card, state);
    crate::engine::core::queue_actions(&mut state.engine.action_queue, passive_card_actions);

    let relic_actions = crate::content::relics::hooks::on_use_card(state, &played_card, target);
    crate::engine::core::queue_actions(&mut state.engine.action_queue, relic_actions);

    for entity_id in std::iter::once(0usize).chain(state.entities.monsters.iter().map(|m| m.id)) {
        for power in &store::powers_snapshot_for(state, entity_id) {
            let hook_actions = crate::content::powers::resolve_power_on_card_played(
                power.power_type,
                state,
                entity_id,
                &played_card,
                power.amount,
            );
            for a in hook_actions {
                state.engine.action_queue.push_back(a);
            }
        }
    }

    {
        let player_powers = crate::content::powers::store::powers_snapshot_for(state, 0);
        let mut exhaust_override = false;
        for power in &player_powers {
            use crate::content::powers::PowerId;
            match power.power_type {
                PowerId::DoubleTap
                | PowerId::DuplicationPower
                | PowerId::Burst
                | PowerId::Corruption
                | PowerId::PenNibPower
                | PowerId::Vigor => {
                    crate::content::powers::resolve_power_on_use_card(
                        power.power_type,
                        state,
                        &played_card,
                        &mut exhaust_override,
                        purge,
                        target,
                    );
                }
                _ => {}
            }
        }
        if exhaust_override {
            played_card.exhaust_override = Some(true);
        }
    }

    state.turn.counters.cards_played_this_turn += 1;
    if def.card_type == crate::content::cards::CardType::Attack {
        state.turn.counters.attacks_played_this_turn += 1;
    }

    let mut should_exhaust = played_card
        .exhaust_override
        .unwrap_or(crate::content::cards::exhausts_when_played(&played_card))
        || (def.card_type == crate::content::cards::CardType::Status
            && state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::MedicalKit))
        || (def.card_type == crate::content::cards::CardType::Curse
            && state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::BlueCandle));
    crate::content::cards::ironclad::corruption::corruption_on_use_card(
        state,
        &played_card,
        &mut should_exhaust,
    );

    if def.card_type != crate::content::cards::CardType::Power && !purge {
        state.zones.limbo.push(played_card);
        state
            .engine
            .action_queue
            .push_back(Action::UseCardDone { should_exhaust });
    }
}

pub fn handle_play_card_from_hand(
    card_index: usize,
    target: Option<usize>,
    state: &mut CombatState,
) -> Result<(), &'static str> {
    if card_index >= state.zones.hand.len() {
        return Err("Card index out of range");
    }

    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::VelvetChoker)
        && state.turn.counters.cards_played_this_turn >= 6
    {
        return Err("VelvetChoker: card play limit reached (6)");
    }

    let card = &state.zones.hand[card_index];
    let card_id = card.id;
    let def = crate::content::cards::get_card_definition(card_id);

    crate::content::cards::can_play_card(card, state)?;

    let target = targeting::resolve_target_request(
        state,
        targeting::validation_for_card_target(crate::content::cards::effective_target(card)),
        target,
    )?;

    let effective_cost = if card.free_to_play_once {
        0
    } else if let Some(cft) = card.cost_for_turn {
        cft as i32
    } else {
        (def.cost as i32 + card.cost_modifier as i32).max(0)
    };

    let is_x_cost = def.cost == -1;
    let energy_to_spend = if is_x_cost {
        state.turn.energy as i32
    } else {
        effective_cost
    };
    let x_effect = if is_x_cost {
        crate::content::relics::hooks::on_calculate_x_cost(state, energy_to_spend)
    } else {
        energy_to_spend
    };

    if !is_x_cost && energy_to_spend > state.turn.energy as i32 {
        return Err("Not enough energy");
    }

    state.turn.energy = (state.turn.energy as i32 - energy_to_spend).max(0) as u8;

    let card_mut = &mut state.zones.hand[card_index];
    if is_x_cost {
        card_mut.energy_on_use = x_effect;
    }

    {
        let mut card_copy = state.zones.hand[card_index].clone();
        crate::content::cards::evaluate_card(&mut card_copy, state, target);
        state.zones.hand[card_index] = card_copy;
    }

    let played_card = state.zones.hand.remove(card_index);
    execute_played_card(played_card, target, false, state);
    Ok(())
}

pub fn handle_enqueue_card_play(
    item: crate::combat::QueuedCardPlay,
    in_front: bool,
    state: &mut CombatState,
) {
    state.enqueue_card_play(item, in_front);
}

pub fn handle_flush_next_queued_card(state: &mut CombatState) {
    let Some(mut queued) = state.zones.queued_cards.pop_front() else {
        return;
    };

    queued.card.energy_on_use = queued.energy_on_use;
    if queued.autoplay {
        queued.card.free_to_play_once = true;
    }
    let target = if queued.random_target {
        targeting::validation_for_card_target(crate::content::cards::effective_target(&queued.card))
            .and_then(|validation| targeting::pick_random_target(state, validation))
    } else {
        queued.target
    };

    if !state.zones.queued_cards.is_empty() {
        state
            .engine
            .action_queue
            .push_back(Action::FlushNextQueuedCard);
    }
    state
        .engine
        .action_queue
        .push_front(Action::PlayCardDirect {
            card: Box::new(queued.card),
            target,
            purge: queued.purge_on_use,
        });
}

pub fn handle_play_card_direct(
    card: Box<crate::combat::CombatCard>,
    target: Option<usize>,
    purge: bool,
    state: &mut CombatState,
) {
    let played_card = *card;
    let target = targeting::resolve_target_request(
        state,
        targeting::validation_for_card_target(crate::content::cards::effective_target(
            &played_card,
        )),
        target,
    )
    .ok()
    .flatten();
    if targeting::validation_for_card_target(crate::content::cards::effective_target(&played_card))
        .is_some()
        && target.is_none()
    {
        return;
    }
    execute_played_card(played_card, target, purge, state);
}

pub fn handle_use_potion(slot: usize, target: Option<usize>, state: &mut CombatState) {
    if let Some(Some(potion)) = state.entities.potions.get(slot).cloned() {
        if potion.id == crate::content::potions::PotionId::FairyPotion {
            return;
        }
        if potion.id == crate::content::potions::PotionId::SmokeBomb && state.meta.is_boss_fight {
            return;
        }
        let def = crate::content::potions::get_potion_definition(potion.id);
        let mut potency = def.base_potency;
        if state
            .entities
            .player
            .has_relic(crate::content::relics::RelicId::SacredBark)
        {
            potency *= 2;
        }
        if potion.id == crate::content::potions::PotionId::LiquidMemories
            && !state.zones.discard_pile.is_empty()
            && state.zones.discard_pile.len() <= potency as usize
        {
            let uuids: Vec<u32> = state.zones.discard_pile.iter().map(|c| c.uuid).collect();
            for uuid in uuids {
                if let Some(pos) = state.zones.discard_pile.iter().position(|c| c.uuid == uuid) {
                    let mut card = state.zones.discard_pile.remove(pos);
                    card.cost_for_turn = Some(0);
                    if state.zones.hand.len() < 10 {
                        state.zones.hand.push(card);
                    }
                }
            }
            let relic_actions = crate::content::relics::hooks::on_use_potion(state, 0);
            crate::engine::core::queue_actions(&mut state.engine.action_queue, relic_actions);
            state.entities.potions[slot] = None;
            return;
        }
        let resolved_target = match targeting::resolve_target_request(
            state,
            targeting::validation_for_potion_target(def.target_required),
            target,
        ) {
            Ok(target) => target,
            Err(_) => return,
        };
        let actions = crate::content::potions::potion_effects::get_potion_actions(
            state.entities.monsters.len(),
            potion.id,
            resolved_target,
            potency,
        );
        let relic_actions = crate::content::relics::hooks::on_use_potion(state, 0);
        let mut combined = actions;
        combined.extend(relic_actions);
        crate::engine::core::queue_actions(&mut state.engine.action_queue, combined);
        state.entities.potions[slot] = None;
    }
}

pub fn handle_play_top_card(target: Option<usize>, exhaust: bool, state: &mut CombatState) {
    if state.zones.draw_pile.is_empty() {
        if state.zones.discard_pile.is_empty() {
            return;
        }
        state
            .engine
            .action_queue
            .push_front(Action::PlayTopCard { target, exhaust });
        state
            .engine
            .action_queue
            .push_front(Action::EmptyDeckShuffle);
        return;
    }

    let mut card = Box::new(state.zones.draw_pile.remove(0));
    card.free_to_play_once = true;
    if crate::content::cards::get_card_definition(card.id).cost == -1 {
        card.energy_on_use = state.turn.energy as i32;
    }
    let queued_random_target = target
        .or_else(|| targeting::pick_random_target(state, crate::state::TargetValidation::AnyEnemy));
    let resolved_target = if let Some(validation) =
        targeting::validation_for_card_target(crate::content::cards::effective_target(&card))
    {
        match queued_random_target {
            Some(explicit) => {
                targeting::resolve_target_request(state, Some(validation), Some(explicit))
                    .ok()
                    .flatten()
                    .or_else(|| targeting::pick_random_target(state, validation))
            }
            None => targeting::pick_random_target(state, validation),
        }
    } else {
        queued_random_target
    };

    if exhaust {
        card.exhaust_override = Some(true);
    }
    state
        .engine
        .action_queue
        .push_front(Action::EnqueueCardPlay {
            item: Box::new(crate::combat::QueuedCardPlay {
                card: *card,
                target: resolved_target,
                energy_on_use: state.turn.energy as i32,
                ignore_energy_total: true,
                autoplay: true,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: crate::combat::QueuedCardSource::Normal,
            }),
            in_front: true,
        });
}

pub fn handle_play_top_cards_buffered(
    count: u8,
    target: Option<usize>,
    exhaust: bool,
    state: &mut CombatState,
) {
    let mut buffered: Vec<(Box<crate::combat::CombatCard>, Option<usize>)> = Vec::new();

    for _ in 0..count {
        if state.zones.draw_pile.is_empty() {
            if state.zones.discard_pile.is_empty() {
                break;
            }
            handle_empty_deck_shuffle(state);
            if state.zones.draw_pile.is_empty() {
                break;
            }
        }

        let mut card = Box::new(state.zones.draw_pile.remove(0));
        card.free_to_play_once = true;
        if crate::content::cards::get_card_definition(card.id).cost == -1 {
            card.energy_on_use = state.turn.energy as i32;
        }
        if exhaust {
            card.exhaust_override = Some(true);
        }
        let queued_random_target = target.or_else(|| {
            targeting::pick_random_target(state, crate::state::TargetValidation::AnyEnemy)
        });
        let resolved_target = if let Some(validation) =
            targeting::validation_for_card_target(crate::content::cards::effective_target(&card))
        {
            match queued_random_target {
                Some(explicit) => {
                    targeting::resolve_target_request(state, Some(validation), Some(explicit))
                        .ok()
                        .flatten()
                        .or_else(|| targeting::pick_random_target(state, validation))
                }
                None => targeting::pick_random_target(state, validation),
            }
        } else {
            queued_random_target
        };
        buffered.push((card, resolved_target));
    }

    for (card, resolved_target) in buffered.into_iter().rev() {
        state
            .engine
            .action_queue
            .push_front(Action::EnqueueCardPlay {
                item: Box::new(crate::combat::QueuedCardPlay {
                    card: *card,
                    target: resolved_target,
                    energy_on_use: state.turn.energy as i32,
                    ignore_energy_total: true,
                    autoplay: true,
                    random_target: false,
                    is_end_turn_autoplay: false,
                    purge_on_use: false,
                    source: crate::combat::QueuedCardSource::Normal,
                }),
                in_front: true,
            });
    }
}

pub fn handle_obtain_potion(state: &mut CombatState) {
    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::Sozu)
    {
        return;
    }
    if let Some(slot) = state.entities.potions.iter().position(|p| p.is_none()) {
        let potion_class = match state.meta.player_class {
            "Silent" => crate::content::potions::PotionClass::Silent,
            "Defect" => crate::content::potions::PotionClass::Defect,
            "Watcher" => crate::content::potions::PotionClass::Watcher,
            _ => crate::content::potions::PotionClass::Ironclad,
        };
        let potion_id =
            crate::content::potions::random_potion(&mut state.rng.potion_rng, potion_class, true);
        state.entities.potions[slot] = Some(crate::content::potions::Potion::new(
            potion_id,
            40000 + slot as u32,
        ));
    }
}

pub fn handle_end_turn_trigger(state: &mut CombatState) {
    let mut actions = smallvec::SmallVec::new();

    // 1. Player Powers
    for power in store::powers_snapshot_for(state, 0) {
        actions.extend(
            crate::content::powers::resolve_power_at_end_of_turn(&power, state, 0)
                .into_iter()
                .map(|a| ActionInfo {
                    action: a,
                    insertion_mode: AddTo::Bottom,
                }),
        );
    }

    // 2. Ethereal exhaust
    for card in &state.zones.hand {
        if crate::content::cards::is_ethereal(card) {
            actions.push(ActionInfo {
                action: Action::ExhaustCard {
                    card_uuid: card.uuid,
                    source_pile: crate::state::PileType::Hand,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
    }

    // 3. Relics
    actions.extend(crate::content::relics::hooks::at_end_of_turn(state));

    // 4. Orbs
    actions.extend(crate::content::orbs::hooks::trigger_end_of_turn_orbs(state));

    // 5. Curses and Burns in hand
    for card in &state.zones.hand {
        if card.id == CardId::Burn {
            actions.extend(crate::content::cards::status::burn::on_end_turn_in_hand(
                state, card,
            ));
        }
        if card.id == CardId::Regret {
            actions.extend(crate::content::cards::curses::regret::on_end_turn_in_hand(
                state,
            ));
        }
        if card.id == CardId::Decay {
            actions.extend(crate::content::cards::curses::decay::on_end_turn_in_hand(
                state,
            ));
        }
        if card.id == CardId::Doubt {
            actions.extend(crate::content::cards::curses::doubt::on_end_turn_in_hand(
                state,
            ));
        }
        if card.id == CardId::Pride {
            actions.extend(crate::content::cards::curses::pride::on_end_turn_in_hand(
                state,
            ));
        }
        if card.id == CardId::Shame {
            actions.extend(crate::content::cards::curses::shame::on_end_turn_in_hand(
                state,
            ));
        }
    }

    // 6. Stances
    actions.extend(crate::content::stances::hooks::on_end_of_turn(state));

    crate::engine::core::queue_actions(&mut state.engine.action_queue, actions);
}

pub fn handle_post_draw_trigger(state: &mut CombatState) {
    let mut actions = smallvec::SmallVec::new();

    actions.extend(crate::content::relics::hooks::at_turn_start_post_draw(
        state,
    ));

    for power in &store::powers_snapshot_for(state, 0) {
        for action in crate::content::powers::resolve_power_on_post_draw(
            power.power_type,
            state,
            0,
            power.amount,
        ) {
            actions.push(ActionInfo {
                action,
                insertion_mode: AddTo::Bottom,
            });
        }
    }

    crate::engine::core::queue_actions(&mut state.engine.action_queue, actions);
}

pub fn handle_clear_card_queue(state: &mut CombatState) {
    state.zones.queued_cards.clear();
    state.engine.action_queue.retain(|a| {
        !matches!(
            a,
            Action::EnqueueCardPlay { .. }
                | Action::PlayCardDirect { .. }
                | Action::FlushNextQueuedCard
        )
    });
}

pub fn handle_add_card_to_master_deck(card_id: CardId, state: &mut CombatState) {
    state
        .meta
        .meta_changes
        .push(crate::combat::MetaChange::AddCardToMasterDeck(card_id));
}

pub fn handle_pre_battle_trigger(state: &mut CombatState) {
    // 1. Monster pre-battle actions (CurlUp, ModeShift, etc.)
    // Java: AbstractRoom.initializeBattle() calls usePreBattleAction() for each monster
    let monsters_snapshot: Vec<_> = state
        .entities
        .monsters
        .iter()
        .filter_map(|m| {
            crate::content::monsters::EnemyId::from_id(m.monster_type).map(|eid| (eid, m.id))
        })
        .collect();
    for (enemy_id, monster_id) in &monsters_snapshot {
        if let Some(entity) = state.entities.monsters.iter().find(|m| m.id == *monster_id) {
            let entity_clone = entity.clone();
            let pre_actions = crate::content::monsters::resolve_pre_battle_action(
                *enemy_id,
                &entity_clone,
                &mut state.rng.misc_rng,
                state.meta.ascension_level,
            );
            for action in pre_actions {
                state.engine.action_queue.push_back(action);
            }
        }
    }

    // 2. Relic pre-battle hooks (e.g. Snecko Eye applying Confusion)
    let pre_battle_actions = crate::content::relics::hooks::at_pre_battle(state);
    crate::engine::core::queue_actions(&mut state.engine.action_queue, pre_battle_actions);

    // Auto-chain Phase 2
    state
        .engine
        .action_queue
        .push_back(crate::action::Action::BattleStartPreDrawTrigger);
}

pub fn handle_battle_start_pre_draw_trigger(state: &mut CombatState) {
    let pre_draw_actions = crate::content::relics::hooks::at_battle_start_pre_draw(state);
    crate::engine::core::queue_actions(&mut state.engine.action_queue, pre_draw_actions);

    // Auto-chain Phase 3 Initial Draw
    let mut draw_amount = 5;
    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::SneckoEye)
    {
        draw_amount += 2;
    }
    state
        .engine
        .action_queue
        .push_back(crate::action::Action::DrawCards(draw_amount));

    // Auto-chain Phase 4
    state
        .engine
        .action_queue
        .push_back(crate::action::Action::BattleStartTrigger);
}

pub fn handle_battle_start_trigger(state: &mut CombatState) {
    // Relic battle-start hooks (e.g. Akabeko, Marbles)
    let battle_start_actions = crate::content::relics::hooks::at_battle_start(state);
    crate::engine::core::queue_actions(&mut state.engine.action_queue, battle_start_actions);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::CombatCard;
    use crate::content::cards::{self, CardId};
    use crate::content::potions::{get_potion_definition, PotionClass};
    use crate::engine::test_support::basic_combat;

    #[test]
    fn modify_card_damage_persists_rampage_damage_growth() {
        let mut combat = basic_combat();
        let mut rampage = CombatCard::new(CardId::Rampage, 99);
        cards::evaluate_card(&mut rampage, &combat, Some(1));
        assert_eq!(rampage.base_damage_mut, 8);

        combat.zones.discard_pile.push(rampage);
        handle_modify_card_damage(99, 5, &mut combat);

        let mut grown = combat.zones.discard_pile[0].clone();
        cards::evaluate_card(&mut grown, &combat, Some(1));
        assert_eq!(grown.base_damage_override, Some(13));
        assert_eq!(grown.base_damage_mut, 13);
    }

    #[test]
    fn modify_card_damage_updates_matching_queued_copy() {
        let mut combat = basic_combat();
        combat
            .zones
            .queued_cards
            .push_back(crate::combat::QueuedCardPlay {
                card: CombatCard::new(CardId::Rampage, 77),
                target: Some(1),
                energy_on_use: 1,
                ignore_energy_total: true,
                autoplay: true,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: true,
                source: crate::combat::QueuedCardSource::DoubleTap,
            });

        handle_modify_card_damage(77, 5, &mut combat);

        assert_eq!(
            combat.zones.queued_cards[0].card.base_damage_override,
            Some(13)
        );
    }

    #[test]
    fn draw_pile_to_hand_by_type_only_moves_matching_cards() {
        let mut combat = basic_combat();
        combat
            .zones
            .draw_pile
            .push(CombatCard::new(CardId::Strike, 1));
        combat
            .zones
            .draw_pile
            .push(CombatCard::new(CardId::Defend, 2));
        combat
            .zones
            .draw_pile
            .push(CombatCard::new(CardId::Bash, 3));

        handle_draw_pile_to_hand_by_type(2, crate::content::cards::CardType::Attack, &mut combat);

        assert_eq!(combat.zones.hand.len(), 2);
        assert!(combat.zones.hand.iter().all(|c| {
            matches!(
                crate::content::cards::get_card_definition(c.id).card_type,
                crate::content::cards::CardType::Attack
            )
        }));
        assert_eq!(combat.zones.draw_pile.len(), 1);
        assert_eq!(combat.zones.draw_pile[0].id, CardId::Defend);
    }

    #[test]
    fn enlightenment_caps_hand_costs_and_upgraded_sets_permanent_discount() {
        let mut combat = basic_combat();
        let strike = CombatCard::new(CardId::Strike, 1);
        let mut bash = CombatCard::new(CardId::Bash, 2);
        bash.cost_for_turn = Some(2);
        let mut bludgeon = CombatCard::new(CardId::Bludgeon, 3);
        bludgeon.cost_for_turn = Some(4);
        combat.zones.hand = vec![strike, bash, bludgeon];

        handle_enlightenment(true, &mut combat);

        assert_eq!(combat.zones.hand[0].cost_for_turn, None);
        assert_eq!(combat.zones.hand[0].cost_modifier, 0);
        assert_eq!(combat.zones.hand[1].cost_for_turn, Some(1));
        assert_eq!(combat.zones.hand[1].cost_modifier, -1);
        assert_eq!(combat.zones.hand[2].cost_for_turn, Some(1));
        assert_eq!(combat.zones.hand[2].cost_modifier, -2);
    }

    #[test]
    fn shuffle_discard_into_draw_merges_even_when_draw_not_empty() {
        let mut combat = basic_combat();
        combat
            .zones
            .draw_pile
            .push(CombatCard::new(CardId::Strike, 1));
        combat
            .zones
            .discard_pile
            .push(CombatCard::new(CardId::Defend, 2));
        combat
            .zones
            .discard_pile
            .push(CombatCard::new(CardId::Bash, 3));

        handle_shuffle_discard_into_draw(&mut combat);

        assert!(combat.zones.discard_pile.is_empty());
        assert_eq!(combat.zones.draw_pile.len(), 3);
        let mut ids = combat
            .zones
            .draw_pile
            .iter()
            .map(|c| c.id)
            .collect::<Vec<_>>();
        ids.sort_by_key(|id| *id as i32);
        assert_eq!(ids, vec![CardId::Strike, CardId::Defend, CardId::Bash]);
    }

    #[test]
    fn madness_prefers_cards_with_positive_cost_for_turn() {
        let mut combat = basic_combat();
        let mut strike = CombatCard::new(CardId::Strike, 1);
        strike.cost_for_turn = Some(0);
        let bash = CombatCard::new(CardId::Bash, 2);
        combat.zones.hand = vec![strike, bash];

        handle_madness(&mut combat);

        assert_eq!(combat.zones.hand[0].cost_for_turn, Some(0));
        assert_eq!(combat.zones.hand[0].cost_modifier, 0);
        assert_eq!(combat.zones.hand[1].cost_for_turn, Some(0));
        assert_eq!(combat.zones.hand[1].cost_modifier, -2);
    }

    #[test]
    fn madness_can_fix_temporarily_inflated_zero_cost_card() {
        let mut combat = basic_combat();
        let mut finesse = CombatCard::new(CardId::Finesse, 1);
        finesse.cost_for_turn = Some(2);
        combat.zones.hand = vec![finesse];

        handle_madness(&mut combat);

        assert_eq!(combat.zones.hand[0].cost_for_turn, Some(0));
        assert_eq!(combat.zones.hand[0].cost_modifier, 0);
    }

    #[test]
    fn madness_overwrites_existing_permanent_cost_reduction_with_true_zero_base() {
        let mut combat = basic_combat();
        let mut bash = CombatCard::new(CardId::Bash, 2);
        bash.cost_modifier = -1;
        bash.cost_for_turn = Some(1);
        combat.zones.hand = vec![bash];

        handle_madness(&mut combat);

        assert_eq!(combat.zones.hand[0].cost_for_turn, Some(0));
        assert_eq!(combat.zones.hand[0].cost_modifier, -2);
    }

    #[test]
    fn make_random_colorless_card_in_hand_uses_combat_pool_and_preserves_flags() {
        let mut combat = basic_combat();
        combat.runtime.colorless_combat_pool = vec![CardId::GoodInstincts];

        handle_make_random_colorless_card_in_hand(Some(0), true, &mut combat);

        assert_eq!(combat.zones.hand.len(), 1);
        let card = &combat.zones.hand[0];
        assert_eq!(card.id, CardId::GoodInstincts);
        assert_eq!(card.upgrades, 1);
        assert_eq!(card.cost_for_turn, Some(0));
    }

    #[test]
    fn obtain_potion_respects_combat_player_class() {
        let mut combat = basic_combat();
        combat.meta.player_class = "Silent";
        let mut seen_silent_only = false;

        for _ in 0..256 {
            combat.entities.potions[0] = None;
            handle_obtain_potion(&mut combat);
            let potion = combat.entities.potions[0].take().expect("potion obtained");
            let class = get_potion_definition(potion.id).class;
            assert!(
                matches!(class, PotionClass::Any | PotionClass::Silent),
                "expected Silent/Any potion, got {:?}",
                potion.id
            );
            seen_silent_only |= class == PotionClass::Silent;
        }

        assert!(
            seen_silent_only,
            "expected to see at least one Silent-only potion"
        );
    }
}
