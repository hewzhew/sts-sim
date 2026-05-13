// action_handlers/cards.rs — Card pile management domain
//
// Handles: DrawCards, EmptyDeckShuffle, DiscardCard, ExhaustCard, MoveCard, PutOnDeck,
//          MakeTempCard*, MakeCopy*, MakeRandom*, PlayCardDirect, PlayTopCard,
//          UseCardDone, UpgradeCard, UpgradeRandomCard, UpgradeAllInHand, UpgradeAllBurns,
//          ReduceAllHandCosts, RandomizeHandCosts, ModifyCardMisc,
//          UsePotion, DiscardPotion, ObtainPotion, ObtainSpecificPotion, Scry,
//          EndTurnTrigger, StartTurnTrigger, PostDrawTrigger, BattleStartTrigger, ClearCardQueue,
//          AddCardToMasterDeck, MakeTempCardInDiscardAndDeck, SuspendForCardReward

use crate::content::cards::CardId;
use crate::content::powers::store;
use crate::content::powers::PowerId;
use crate::engine::targeting;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;

fn queue_exhaust_triggers(card: &crate::runtime::combat::CombatCard, state: &mut CombatState) {
    let mut after_actions = crate::content::relics::hooks::on_exhaust(state);

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

    let card_hooks = crate::content::cards::resolve_card_on_exhaust(card, state);
    after_actions.extend(card_hooks);

    state.queue_actions(after_actions);
}

pub fn move_card_to_exhaust_pile(
    card: crate::runtime::combat::CombatCard,
    state: &mut CombatState,
) {
    queue_exhaust_triggers(&card, state);
    state.add_card_to_exhaust_pile_top(card);
}

fn move_random_hand_card_to_draw_top(state: &mut CombatState) {
    if state.zones.hand.is_empty() {
        return;
    }
    let idx = state
        .rng
        .card_random_rng
        .random(state.zones.hand.len() as i32 - 1) as usize;
    let card = state.zones.hand.remove(idx);
    state.add_card_to_draw_pile_top(card);
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
            state.shuffle_discard_pile_into_draw_pile();
            let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
            state.queue_actions(shuffle_actions);
        }
        if state.zones.draw_pile.is_empty() {
            break;
        }
        let mut card = state
            .draw_top_card()
            .expect("draw pile was checked non-empty before drawing");

        if card.id == CardId::Void {
            let void_actions = crate::content::cards::status::void::on_drawn(state);
            state.queue_actions(void_actions);
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
                state.queue_action_back(a);
            }
        }
    }
}

pub fn handle_put_on_deck(amount: usize, random: bool, state: &mut CombatState) {
    let amount = amount.min(state.zones.hand.len());
    if amount == 0 {
        return;
    }

    if random {
        for _ in 0..amount {
            move_random_hand_card_to_draw_top(state);
        }
        return;
    }

    if state.zones.hand.len() > amount {
        state.queue_action_front(Action::SuspendForHandSelect {
            min: amount as u8,
            max: amount as u8,
            can_cancel: false,
            filter: crate::state::HandSelectFilter::Any,
            reason: crate::state::HandSelectReason::PutOnDrawPile,
        });
        return;
    }

    let mut i = 0;
    while i < state.zones.hand.len() {
        move_random_hand_card_to_draw_top(state);
        i += 1;
    }
}

pub fn handle_empty_deck_shuffle(state: &mut CombatState) {
    if state.zones.draw_pile.is_empty() && !state.zones.discard_pile.is_empty() {
        state.shuffle_discard_pile_into_draw_pile();
        let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
        state.queue_actions(shuffle_actions);
    }
}

pub fn handle_shuffle_discard_into_draw(state: &mut CombatState) {
    if state.zones.discard_pile.is_empty() {
        return;
    }
    state.shuffle_discard_pile_into_draw_pile();
    let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
    state.queue_actions(shuffle_actions);
}

pub fn handle_discard_card(card_uuid: u32, state: &mut CombatState) {
    if let Some(pos) = state.zones.hand.iter().position(|c| c.uuid == card_uuid) {
        let card = state.zones.hand.remove(pos);
        state.add_card_to_discard_pile_top(card);
        let discard_actions = crate::content::relics::hooks::on_discard(state);
        state.queue_actions(discard_actions);
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
                    state.add_card_to_discard_pile_top(card);
                }
            }
            crate::state::PileType::Draw => state.add_card_to_draw_pile_top(card),
            crate::state::PileType::Discard => state.add_card_to_discard_pile_top(card),
            crate::state::PileType::Exhaust => {
                if matches!(from, crate::state::PileType::Exhaust) {
                    state.add_card_to_exhaust_pile_top(card);
                } else {
                    move_card_to_exhaust_pile(card, state);
                }
            }
            _ => {}
        }
    }
}

fn monsters_are_basically_dead(state: &CombatState) -> bool {
    state.are_monsters_basically_dead_java()
}

pub fn handle_discard_pile_to_top_of_deck(state: &mut CombatState) {
    if monsters_are_basically_dead(state) {
        return;
    }

    match state.zones.discard_pile.len() {
        0 => {}
        1 => {
            let card_uuid = state.zones.discard_pile[0].uuid;
            handle_move_card(
                card_uuid,
                crate::state::PileType::Discard,
                crate::state::PileType::Draw,
                state,
            );
        }
        _ => state.queue_action_front(Action::SuspendForGridSelect {
            source_pile: crate::state::PileType::Discard,
            min: 1,
            max: 1,
            can_cancel: false,
            filter: crate::state::GridSelectFilter::Any,
            reason: crate::state::GridSelectReason::MoveToDrawPile,
        }),
    }
}

fn apply_master_reality_to_generated_card(
    card: &mut crate::runtime::combat::CombatCard,
    state: &CombatState,
    upgrade_call_sites: u8,
) {
    crate::content::cards::apply_master_reality_to_generated_card(card, state, upgrade_call_sites);
}

fn make_generated_card_from_id(
    card_id: CardId,
    uuid: u32,
    upgraded: bool,
) -> crate::runtime::combat::CombatCard {
    let mut card = crate::runtime::combat::CombatCard::new(card_id, uuid);
    if upgraded {
        card.upgrades = 1;
    }
    card
}

fn make_random_pool_card_from_id(
    card_id: CardId,
    uuid: u32,
    state: &CombatState,
) -> crate::runtime::combat::CombatCard {
    crate::content::cards::make_fresh_card_copy_for_combat(card_id, uuid, state)
}

pub fn handle_exhume_card(card_uuid: u32, upgrade: bool, state: &mut CombatState) {
    if state.zones.hand.len() >= 10 {
        return;
    }

    let Some(pos) = state
        .zones
        .exhaust_pile
        .iter()
        .position(|c| c.uuid == card_uuid && c.id != CardId::Exhume)
    else {
        return;
    };

    let mut card = state.zones.exhaust_pile.remove(pos);
    if upgrade && crate::content::cards::can_upgrade_card_once(&card) {
        card.upgrades += 1;
    }
    if store::has_power(state, 0, PowerId::Corruption)
        && crate::content::cards::get_card_definition(card.id).card_type
            == crate::content::cards::CardType::Skill
    {
        card.set_cost_for_turn_java(0);
    }
    state.zones.hand.push(card);
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

fn apply_generated_card_entering_hand_mechanics(
    card: &mut crate::runtime::combat::CombatCard,
    state: &CombatState,
) {
    if store::has_power(state, 0, PowerId::Corruption) {
        crate::content::cards::ironclad::corruption::corruption_on_card_draw(state, card);
    }
    crate::content::cards::evaluate_card(card, state, None);
}

fn add_generated_card_to_hand_or_discard(
    mut card: crate::runtime::combat::CombatCard,
    state: &mut CombatState,
) {
    if state.zones.hand.len() < 10 {
        apply_master_reality_to_generated_card(&mut card, state, 2);
        apply_generated_card_entering_hand_mechanics(&mut card, state);
        state.zones.hand.push(card);
    } else {
        apply_master_reality_to_generated_card(&mut card, state, 1);
        state.add_card_to_discard_pile_top(card);
    }
}

pub fn handle_make_temp_card_in_hand(
    card_id: CardId,
    amount: u8,
    upgraded: bool,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        let card = make_generated_card_from_id(card_id, state.next_card_uuid(), upgraded);
        add_generated_card_to_hand_or_discard(card, state);
    }
}

pub fn handle_make_temp_card_in_discard(
    card_id: CardId,
    amount: u8,
    upgraded: bool,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        let mut card = make_generated_card_from_id(card_id, state.next_card_uuid(), upgraded);
        apply_master_reality_to_generated_card(&mut card, state, 1);
        state.add_card_to_discard_pile_top(card);
    }
}

pub fn handle_make_temp_card_in_draw_pile(
    card_id: CardId,
    amount: u8,
    random_spot: bool,
    to_bottom: bool,
    upgraded: bool,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        let mut card = make_generated_card_from_id(card_id, state.next_card_uuid(), upgraded);
        let upgrade_call_sites = if amount < 6 { 2 } else { 1 };
        apply_master_reality_to_generated_card(&mut card, state, upgrade_call_sites);
        if to_bottom {
            state.add_card_to_draw_pile_bottom(card);
        } else if random_spot {
            state.add_card_to_draw_pile_random_spot(card);
        } else {
            state.add_card_to_draw_pile_top(card);
        }
    }
}

pub fn handle_make_copy_in_hand(
    original: Box<crate::runtime::combat::CombatCard>,
    amount: u8,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        let card = original.make_stat_equivalent_copy_with_uuid(state.next_card_uuid());
        add_generated_card_to_hand_or_discard(card, state);
    }
}

pub fn handle_make_copy_in_discard(
    original: Box<crate::runtime::combat::CombatCard>,
    amount: u8,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        let mut card = original.make_stat_equivalent_copy_with_uuid(state.next_card_uuid());
        apply_master_reality_to_generated_card(&mut card, state, 1);
        state.add_card_to_discard_pile_top(card);
    }
}

pub fn handle_make_temp_card_in_discard_and_deck(
    card_id: CardId,
    amount: u8,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        let mut discard_card = make_generated_card_from_id(card_id, state.next_card_uuid(), false);
        apply_master_reality_to_generated_card(&mut discard_card, state, 1);
        state.add_card_to_discard_pile_top(discard_card);

        let mut draw_card = make_generated_card_from_id(card_id, state.next_card_uuid(), false);
        apply_master_reality_to_generated_card(&mut draw_card, state, 1);
        state.add_card_to_draw_pile_random_spot(draw_card);
    }
}

pub fn handle_reduce_all_hand_costs(amount: u8, state: &mut CombatState) {
    for card in state.zones.hand.iter_mut() {
        if card.cost_for_turn_java() > 0 {
            let current = card.cost_for_turn_java();
            card.set_cost_for_turn_java(current - amount as i32);
        }
    }
}

pub fn handle_enlightenment(permanent: bool, state: &mut CombatState) {
    for card in state.zones.hand.iter_mut() {
        let base_cost = card.base_cost_for_combat_java();
        if base_cost < 0 {
            continue;
        }

        let current = card.cost_for_turn_java();
        if current > 1 {
            card.set_cost_for_turn_java(1);
        }

        if permanent && base_cost > 1 {
            card.set_combat_cost_preserving_turn_java(1);
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
    card.modify_cost_for_combat_java(-99);
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
        crate::runtime::rng::shuffle_with_random_long(&mut shuffled, &mut state.rng.shuffle_rng);
        let target_uuid = shuffled[0];
        if let Some(card) = state.zones.hand.iter_mut().find(|c| c.uuid == target_uuid) {
            card.upgrades += 1;
        }
    }
}

pub fn handle_modify_card_misc(card_uuid: u32, amount: i32, state: &mut CombatState) {
    state
        .meta
        .meta_changes
        .push(crate::runtime::combat::MetaChange::ModifyCardMisc { card_uuid, amount });

    state
        .zones
        .for_each_java_battle_instance_mut_by_uuid(card_uuid, |card| {
            card.misc_value += amount;
        });
}

pub fn handle_modify_card_damage(card_uuid: u32, amount: i32, state: &mut CombatState) {
    state
        .zones
        .for_each_java_battle_instance_mut_by_uuid(card_uuid, |card| {
            let def = crate::content::cards::get_card_definition(card.id);
            let upgraded_base = def.base_damage + (card.upgrades as i32) * def.upgrade_damage;
            let current = card.base_damage_override.unwrap_or(upgraded_base);
            card.base_damage_override = Some((current + amount).max(0));
        });
    state
        .zones
        .for_each_queued_instance_mut_by_uuid(card_uuid, |card| {
            let def = crate::content::cards::get_card_definition(card.id);
            let upgraded_base = def.base_damage + (card.upgrades as i32) * def.upgrade_damage;
            let current = card.base_damage_override.unwrap_or(upgraded_base);
            card.base_damage_override = Some((current + amount).max(0));
        });
}

pub fn handle_randomize_hand_costs(state: &mut CombatState) {
    for card in state.zones.hand.iter_mut() {
        let base_cost = crate::content::cards::get_card_definition(card.id).cost;
        if base_cost >= 0 {
            let new_cost = state.rng.card_random_rng.random(3) as u8;
            card.set_cost_for_turn_java(new_cost as i32);
        }
    }
}

fn class_card_pool_for_type(
    player_class: &str,
    card_type: Option<crate::content::cards::CardType>,
) -> Vec<CardId> {
    let mut pool = Vec::new();
    for &rarity in &[
        crate::content::cards::CardRarity::Common,
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Rare,
    ] {
        for &id in crate::engine::campfire_handler::card_pool_for_class(player_class, rarity) {
            let def = crate::content::cards::get_card_definition(id);
            if def.tags.contains(&crate::content::cards::CardTag::Healing) {
                continue;
            }
            if let Some(ct) = card_type {
                if def.card_type != ct {
                    continue;
                }
            }
            pool.push(id);
        }
    }
    pool
}

pub fn handle_make_random_card_in_hand(
    card_type: Option<crate::content::cards::CardType>,
    cost_for_turn: Option<u8>,
    state: &mut CombatState,
) {
    let pool = class_card_pool_for_type(state.meta.player_class, card_type);
    if !pool.is_empty() {
        let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        let mut card = make_random_pool_card_from_id(card_id, state.next_card_uuid(), state);
        if let Some(cost) = cost_for_turn {
            card.set_cost_for_turn_java(cost as i32);
        }
        add_generated_card_to_hand_or_discard(card, state);
    }
}

pub fn handle_make_random_card_in_draw_pile(
    card_type: Option<crate::content::cards::CardType>,
    cost_for_turn: Option<u8>,
    random_spot: bool,
    state: &mut CombatState,
) {
    let pool = class_card_pool_for_type(state.meta.player_class, card_type);
    if !pool.is_empty() {
        let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        let mut card = make_random_pool_card_from_id(card_id, state.next_card_uuid(), state);
        if let Some(cost) = cost_for_turn {
            card.set_cost_for_turn_java(cost as i32);
        }
        apply_master_reality_to_generated_card(&mut card, state, 2);
        if random_spot {
            state.add_card_to_draw_pile_random_spot(card);
        } else {
            state.add_card_to_draw_pile_top(card);
        }
    }
}

pub fn handle_draw_pile_to_hand_by_type(
    amount: u8,
    card_type: crate::content::cards::CardType,
    state: &mut CombatState,
) {
    let mut candidates: Vec<u32> = Vec::new();
    let matching_uuids: Vec<u32> = state
        .zones
        .draw_pile
        .iter()
        .rev()
        .filter(|card| crate::content::cards::get_card_definition(card.id).card_type == card_type)
        .map(|card| card.uuid)
        .collect();
    for uuid in matching_uuids {
        if candidates.is_empty() {
            candidates.push(uuid);
        } else {
            let index = state
                .rng
                .card_random_rng
                .random(candidates.len() as i32 - 1) as usize;
            candidates.insert(index, uuid);
        }
    }

    for _ in 0..amount {
        if candidates.is_empty() {
            break;
        }
        crate::runtime::rng::shuffle_with_random_long(&mut candidates, &mut state.rng.shuffle_rng);
        let chosen_uuid = candidates.remove(0);
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
                state.add_card_to_discard_pile_top(card);
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
        let mut card = crate::runtime::combat::CombatCard::new(card_id, state.next_card_uuid());
        if upgraded {
            card.upgrades = 1;
        }
        if let Some(cost) = cost_for_turn {
            card.set_cost_for_turn_java(cost as i32);
        }
        add_generated_card_to_hand_or_discard(card, state);
    }
}

pub fn handle_transmutation(
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let base_effect = if energy_on_use != -1 {
        energy_on_use
    } else {
        state.turn.energy as i32
    };
    let effect = crate::content::relics::hooks::on_calculate_x_cost(state, base_effect);

    if effect > 0 {
        for _ in 0..effect {
            state.queue_action_back(Action::MakeRandomColorlessCardInHand {
                cost_for_turn: Some(0),
                upgraded,
            });
        }
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_use_card_done(should_exhaust: bool, state: &mut CombatState) {
    if let Some(mut card) = state.zones.limbo.pop() {
        // Java UseCardAction clears this before moving the card to discard or
        // exhaust. Keeping it on a saved/discarded card makes later draws free.
        card.free_to_play_once = false;

        let def = crate::content::cards::get_card_definition(card.id);
        let spoon_saves_exhaust = should_exhaust
            && def.card_type != crate::content::cards::CardType::Power
            && state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::StrangeSpoon)
            && state.rng.card_random_rng.random_boolean();

        if should_exhaust && !spoon_saves_exhaust {
            move_card_to_exhaust_pile(card, state);
        } else {
            if spoon_saves_exhaust {
                card.exhaust_override = None;
            }
            state.add_card_to_discard_pile_top(card);
        }
    }

    if state.turn.counters.early_end_turn_pending {
        state.turn.clear_early_end_turn_pending();
        state.begin_turn_transition();
        state.queue_action_back(Action::EndTurnTrigger);
    }
}

pub fn handle_queue_early_end_turn(state: &mut CombatState) {
    state.turn.mark_early_end_turn_pending();
}

fn execute_played_card(
    mut played_card: crate::runtime::combat::CombatCard,
    target: Option<usize>,
    purge: bool,
    state: &mut CombatState,
) {
    let card_id = played_card.id;
    let def = crate::content::cards::get_card_definition(card_id);

    crate::content::cards::evaluate_card(&mut played_card, state, target);

    let mut card_actions =
        crate::content::cards::resolve_card_play(card_id, state, &played_card, target);
    if card_id == CardId::Havoc {
        for action in &mut card_actions {
            if let Action::PlayTopCard { target, .. } = &mut action.action {
                if target.is_none() {
                    *target = targeting::pick_random_target(
                        state,
                        crate::state::TargetValidation::AnyEnemy,
                    );
                }
            }
        }
    }
    state.queue_actions(card_actions);

    let passive_card_actions = crate::content::cards::on_play_card(&played_card, state);
    state.queue_actions(passive_card_actions);

    let relic_actions = crate::content::relics::hooks::on_use_card(state, &played_card, target);
    state.queue_actions(relic_actions);

    let trigger_owners: Vec<_> = std::iter::once(0usize)
        .chain(state.entities.monsters.iter().map(|m| m.id))
        .collect();
    for entity_id in trigger_owners {
        for power in &store::powers_snapshot_for(state, entity_id) {
            let hook_actions = crate::content::powers::resolve_power_on_card_played(
                power.power_type,
                state,
                entity_id,
                &played_card,
                power.amount,
            );
            for a in hook_actions {
                state.queue_action_back(a);
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

    state.turn.record_card_played(card_id);
    if def.card_type == crate::content::cards::CardType::Attack {
        state.turn.increment_attacks_played();
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
        state.queue_action_back(Action::UseCardDone { should_exhaust });
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

    let base_cost = crate::content::cards::upgraded_base_cost_override(card).unwrap_or(def.cost);
    let effective_cost = if card.free_to_play_once {
        0
    } else if let Some(cft) = card.cost_for_turn {
        cft as i32
    } else {
        card.get_cost() as i32
    };

    let is_x_cost = base_cost == -1;
    let energy_on_use = if is_x_cost {
        state.turn.energy as i32
    } else {
        effective_cost
    };

    if !is_x_cost && energy_on_use > state.turn.energy as i32 {
        return Err("Not enough energy");
    }

    if !is_x_cost {
        state.turn.spend_energy(energy_on_use);
    }

    let card_mut = &mut state.zones.hand[card_index];
    if is_x_cost {
        card_mut.energy_on_use = energy_on_use;
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
    item: crate::runtime::combat::QueuedCardPlay,
    in_front: bool,
    state: &mut CombatState,
) {
    state.enqueue_card_play(item, in_front);
}

fn queued_card_target_fails_java_can_use(
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
    state: &CombatState,
) -> bool {
    if targeting::validation_for_card_target(crate::content::cards::effective_target(card))
        .is_none()
    {
        return false;
    }

    if state.are_monsters_basically_dead_java() {
        return true;
    }

    target.is_some_and(|target_id| {
        state
            .entities
            .monsters
            .iter()
            .find(|m| m.id == target_id)
            .is_some_and(|m| m.is_dying)
    })
}

fn queued_card_target_allows_java_use_card(
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
    state: &CombatState,
) -> bool {
    if targeting::validation_for_card_target(crate::content::cards::effective_target(card))
        .is_none()
    {
        return true;
    }

    let Some(target_id) = target else {
        return false;
    };

    state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == target_id)
        .is_some_and(|m| !m.is_dead_or_escaped())
}

pub fn handle_flush_next_queued_card(state: &mut CombatState) {
    if state.zones.queued_cards.len() == 1
        && state
            .zones
            .queued_cards
            .front()
            .is_some_and(|queued| queued.is_end_turn_autoplay)
    {
        for relic in &mut state.entities.player.relics {
            if relic.id == crate::content::relics::RelicId::UnceasingTop {
                crate::content::relics::unceasing_top::disable_until_turn_ends(relic);
            }
        }
    }

    let Some(mut queued) = state.zones.queued_cards.pop_front() else {
        return;
    };

    queued.card.energy_on_use = queued.energy_on_use;
    let target = if queued.random_target {
        targeting::validation_for_card_target(crate::content::cards::effective_target(&queued.card))
            .and_then(|validation| targeting::pick_random_target(state, validation))
    } else {
        queued.target
    };

    let has_more_queued_cards = !state.zones.queued_cards.is_empty();
    if crate::content::cards::can_play_card_ignoring_energy(&queued.card, state).is_err()
        || queued_card_target_fails_java_can_use(&queued.card, target, state)
    {
        if queued.autoplay && !queued.purge_on_use {
            let should_exhaust = queued
                .card
                .exhaust_override
                .unwrap_or(crate::content::cards::exhausts_when_played(&queued.card));
            state.zones.limbo.push(queued.card);
            state.queue_action_front(Action::UseCardDone { should_exhaust });
        }
        if has_more_queued_cards {
            state.queue_action_back(Action::FlushNextQueuedCard);
        }
        return;
    }

    if has_more_queued_cards {
        state.queue_action_back(Action::FlushNextQueuedCard);
    }
    state.queue_action_front(Action::PlayCardDirect {
        card: Box::new(queued.card),
        target,
        purge: queued.purge_on_use,
    });
}

pub fn handle_play_card_direct(
    card: Box<crate::runtime::combat::CombatCard>,
    target: Option<usize>,
    purge: bool,
    state: &mut CombatState,
) {
    let played_card = *card;
    if !queued_card_target_allows_java_use_card(&played_card, target, state) {
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
                    card.set_cost_for_turn_java(0);
                    if state.zones.hand.len() < 10 {
                        state.zones.hand.push(card);
                    }
                }
            }
            let relic_actions = crate::content::relics::hooks::on_use_potion(state, 0);
            state.queue_actions(relic_actions);
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
        state.queue_actions(combined);
        state.entities.potions[slot] = None;
    }
}

pub fn handle_play_top_card(target: Option<usize>, exhaust: bool, state: &mut CombatState) {
    let queued_random_target = target
        .or_else(|| targeting::pick_random_target(state, crate::state::TargetValidation::AnyEnemy));

    if state.zones.draw_pile.is_empty() {
        if state.zones.discard_pile.is_empty() {
            return;
        }
        state.queue_action_front(Action::PlayTopCard {
            target: queued_random_target,
            exhaust,
        });
        state.queue_action_front(Action::EmptyDeckShuffle);
        return;
    }

    let mut card = Box::new(
        state
            .draw_top_card()
            .expect("draw pile was checked non-empty before PlayTopCard"),
    );
    if crate::content::cards::get_card_definition(card.id).cost == -1 {
        card.energy_on_use = state.turn.energy as i32;
    }
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
    state.queue_action_front(Action::EnqueueCardPlay {
        item: Box::new(crate::runtime::combat::QueuedCardPlay {
            card: *card,
            target: resolved_target,
            energy_on_use: state.turn.energy as i32,
            ignore_energy_total: true,
            autoplay: true,
            random_target: false,
            is_end_turn_autoplay: false,
            purge_on_use: false,
            source: crate::runtime::combat::QueuedCardSource::Normal,
        }),
        in_front: false,
    });
}

pub fn handle_queue_play_top_card_to_bottom(
    target: Option<usize>,
    exhaust: bool,
    state: &mut CombatState,
) {
    state.queue_action_back(Action::PlayTopCard { target, exhaust });
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

/// Java source evidence:
/// `actions/common/ObtainPotionAction.java` stores one concrete `AbstractPotion`
/// and on first update performs:
///   if Sozu: flash only
///   else: AbstractDungeon.player.obtainPotion(this.potion)
/// `AbstractPlayer.obtainPotion` places into the first empty potion slot and
/// does nothing if all slots are full. Rust models only the mechanical state
/// transition; sound/flash/UI effects are intentionally excluded.
pub fn obtain_specific_potion_if_allowed(
    state: &mut CombatState,
    potion_id: crate::content::potions::PotionId,
) -> bool {
    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::Sozu)
    {
        return false;
    }
    let Some(slot) = state.entities.potions.iter().position(|p| p.is_none()) else {
        return false;
    };
    state.entities.potions[slot] = Some(crate::content::potions::Potion::new(
        potion_id,
        40000 + slot as u32,
    ));
    true
}

pub fn handle_end_turn_trigger(state: &mut CombatState) {
    let mut actions = smallvec::SmallVec::new();

    // 1. Relics
    actions.extend(crate::content::relics::hooks::at_end_of_turn(state));

    // 2. Player powers
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

    // 3. Orbs
    actions.extend(crate::content::orbs::hooks::trigger_end_of_turn_orbs(state));

    // 4. Ethereal exhaust and status/curse in-hand triggers
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

    // 5. Stances
    actions.extend(crate::content::stances::hooks::on_end_of_turn(state));

    state.queue_actions(actions);
}

#[cfg(test)]
mod tests {
    use super::{
        class_card_pool_for_type, handle_discard_pile_to_top_of_deck,
        handle_draw_pile_to_hand_by_type, handle_make_copy_in_discard,
        handle_make_random_card_in_draw_pile, handle_make_random_card_in_hand,
        handle_make_temp_card_in_discard, handle_make_temp_card_in_discard_and_deck,
        handle_make_temp_card_in_draw_pile, handle_make_temp_card_in_hand, handle_play_card_direct,
        handle_use_card_done, obtain_specific_potion_if_allowed,
    };
    use crate::content::cards::{CardId, CardType};
    use crate::content::monsters::EnemyId;
    use crate::content::potions::PotionId;
    use crate::content::powers::PowerId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::action::Action;
    use crate::runtime::combat::{CombatCard, Power};
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn obtain_specific_potion_fills_first_empty_slot() {
        let mut state = blank_test_combat();
        state.entities.potions = vec![
            Some(crate::content::potions::Potion::new(
                PotionId::FirePotion,
                1,
            )),
            None,
            None,
        ];

        assert!(obtain_specific_potion_if_allowed(
            &mut state,
            PotionId::EnergyPotion
        ));

        assert_eq!(
            state.entities.potions[1].as_ref().map(|p| p.id),
            Some(PotionId::EnergyPotion)
        );
        assert!(state.entities.potions[2].is_none());
    }

    #[test]
    fn obtain_specific_potion_is_blocked_by_sozu() {
        let mut state = blank_test_combat();
        state.entities.potions = vec![None, None, None];
        state
            .entities
            .player
            .relics
            .push(RelicState::new(RelicId::Sozu));

        assert!(!obtain_specific_potion_if_allowed(
            &mut state,
            PotionId::EnergyPotion
        ));

        assert!(state.entities.potions.iter().all(Option::is_none));
    }

    #[test]
    fn obtain_specific_potion_does_nothing_when_slots_are_full() {
        let mut state = blank_test_combat();
        state.entities.potions = vec![
            Some(crate::content::potions::Potion::new(
                PotionId::FirePotion,
                1,
            )),
            Some(crate::content::potions::Potion::new(
                PotionId::BlockPotion,
                2,
            )),
        ];
        let before = state.entities.potions.clone();

        assert!(!obtain_specific_potion_if_allowed(
            &mut state,
            PotionId::EnergyPotion
        ));

        assert_eq!(state.entities.potions, before);
    }

    #[test]
    fn random_class_card_in_combat_pool_excludes_healing_cards_like_java() {
        let all = class_card_pool_for_type("Ironclad", None);
        assert!(!all.contains(&CardId::Feed));
        assert!(!all.contains(&CardId::Reaper));

        let attacks = class_card_pool_for_type("Ironclad", Some(CardType::Attack));
        assert!(!attacks.contains(&CardId::Feed));
        assert!(!attacks.contains(&CardId::Reaper));
        assert!(!attacks.contains(&CardId::InfernalBlade));
        assert!(attacks.contains(&CardId::Pummel));
    }

    #[test]
    fn discard_pile_to_top_uses_java_basically_dead_guard() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 900;
        monster.current_hp = 0;
        monster.is_dying = false;
        monster.is_escaped = false;
        state.entities.monsters = vec![monster];
        state.zones.discard_pile = vec![CombatCard::new(CardId::Strike, 901)];

        handle_discard_pile_to_top_of_deck(&mut state);

        assert!(state.zones.discard_pile.is_empty());
        assert_eq!(state.zones.draw_pile[0].uuid, 901);
    }

    #[test]
    fn generated_skill_entering_hand_obeys_corruption_cost_override() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Corruption,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                just_applied: false,
            }],
        );

        handle_make_temp_card_in_hand(CardId::Defend, 1, false, &mut state);

        assert_eq!(state.zones.hand.len(), 1);
        assert_eq!(state.zones.hand[0].id, CardId::Defend);
        assert_eq!(state.zones.hand[0].cost_for_turn, Some(0));
    }

    #[test]
    fn generated_skill_overflowing_to_discard_does_not_apply_hand_only_corruption_hook() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Corruption,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                just_applied: false,
            }],
        );
        for uuid in 1..=10 {
            state.zones.hand.push(CombatCard::new(CardId::Strike, uuid));
        }
        state.zones.card_uuid_counter = 10;

        handle_make_temp_card_in_hand(CardId::Defend, 1, false, &mut state);

        assert_eq!(state.zones.hand.len(), 10);
        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(state.zones.discard_pile[0].id, CardId::Defend);
        assert_eq!(state.zones.discard_pile[0].cost_for_turn, None);
    }

    #[test]
    fn generated_cards_apply_master_reality_before_entering_zones() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                just_applied: false,
            }],
        );

        handle_make_temp_card_in_hand(CardId::Anger, 1, false, &mut state);
        handle_make_temp_card_in_discard(CardId::Anger, 1, false, &mut state);
        handle_make_temp_card_in_draw_pile(CardId::Anger, 1, false, false, false, &mut state);
        handle_make_temp_card_in_hand(CardId::Wound, 1, false, &mut state);

        assert_eq!(state.zones.hand[0].id, CardId::Anger);
        assert_eq!(state.zones.hand[0].upgrades, 1);
        assert_eq!(state.zones.discard_pile[0].id, CardId::Anger);
        assert_eq!(state.zones.discard_pile[0].upgrades, 1);
        assert_eq!(state.zones.draw_pile[0].id, CardId::Anger);
        assert_eq!(state.zones.draw_pile[0].upgrades, 1);
        assert_eq!(state.zones.hand[1].id, CardId::Wound);
        assert_eq!(state.zones.hand[1].upgrades, 0);
    }

    #[test]
    fn searing_blow_preserves_java_master_reality_effect_call_counts() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                just_applied: false,
            }],
        );
        state.zones.card_uuid_counter = 30;

        handle_make_temp_card_in_hand(CardId::SearingBlow, 1, false, &mut state);
        handle_make_temp_card_in_discard(CardId::SearingBlow, 1, false, &mut state);
        handle_make_temp_card_in_draw_pile(CardId::SearingBlow, 1, false, false, false, &mut state);

        assert_eq!(
            state.zones.hand[0].upgrades, 2,
            "Java MakeTempCardInHandAction plus ShowCardAndAddToHandEffect both call Master Reality"
        );
        assert_eq!(
            state.zones.discard_pile[0].upgrades, 1,
            "Java MakeTempCardInDiscardAction(card, amount) only upgrades through the discard effect"
        );
        assert_eq!(
            state.zones.draw_pile[0].upgrades, 2,
            "Java MakeTempCardInDrawPileAction amount<6 and the draw-pile effect both call Master Reality"
        );
    }

    #[test]
    fn make_temp_card_in_hand_overflow_uses_java_discard_effect_upgrade_count() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                just_applied: false,
            }],
        );
        for uuid in 1..=10 {
            state.zones.hand.push(CombatCard::new(CardId::Strike, uuid));
        }
        state.zones.card_uuid_counter = 10;

        handle_make_temp_card_in_hand(CardId::SearingBlow, 1, false, &mut state);

        assert_eq!(state.zones.hand.len(), 10);
        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(state.zones.discard_pile[0].id, CardId::SearingBlow);
        assert_eq!(
            state.zones.discard_pile[0].upgrades, 1,
            "Java MakeTempCardInHandAction overflow adds srcCard to discard, so only the action constructor Master Reality call affects the actual card"
        );
    }

    #[test]
    fn random_pool_blood_for_blood_copy_uses_java_make_copy_damage_discount() {
        let mut state = blank_test_combat();
        state.turn.counters.times_damaged_this_combat = 3;

        let card = crate::content::cards::make_fresh_card_copy_for_combat(
            CardId::BloodForBlood,
            90,
            &state,
        );

        assert_eq!(card.cost_modifier, -3);
        assert_eq!(
            card.get_cost(),
            1,
            "Java BloodForBlood.makeCopy() applies damagedThisCombat before random generated copies enter combat"
        );
    }

    #[test]
    fn make_copy_in_discard_uses_java_stat_equivalent_copy_not_transient_evaluation() {
        let mut state = blank_test_combat();
        state.zones.card_uuid_counter = 20;
        let mut original = CombatCard::new(CardId::Anger, 10);
        original.upgrades = 1;
        original.misc_value = 3;
        original.base_damage_override = Some(17);
        original.cost_modifier = -1;
        original.cost_for_turn = Some(0);
        original.free_to_play_once = true;
        original.base_damage_mut = 99;
        original.base_block_mut = 88;
        original.base_magic_num_mut = 77;
        original.multi_damage = smallvec::smallvec![1, 2, 3];
        original.exhaust_override = Some(true);
        original.retain_override = Some(true);
        original.energy_on_use = 5;

        handle_make_copy_in_discard(Box::new(original), 1, &mut state);

        let copied = &state.zones.discard_pile[0];
        assert_eq!(copied.uuid, 21);
        assert_eq!(copied.id, CardId::Anger);
        assert_eq!(copied.upgrades, 1);
        assert_eq!(copied.misc_value, 3);
        assert_eq!(copied.base_damage_override, Some(17));
        assert_eq!(copied.cost_modifier, -1);
        assert_eq!(copied.cost_for_turn, Some(0));
        assert!(copied.free_to_play_once);
        assert_eq!(copied.base_damage_mut, 0);
        assert_eq!(copied.base_block_mut, 0);
        assert_eq!(copied.base_magic_num_mut, 0);
        assert!(copied.multi_damage.is_empty());
        assert_eq!(copied.exhaust_override, None);
        assert_eq!(copied.retain_override, None);
        assert_eq!(copied.energy_on_use, 0);
    }

    #[test]
    fn make_temp_card_in_discard_and_deck_creates_distinct_instances() {
        let mut state = blank_test_combat();
        state.zones.card_uuid_counter = 30;

        handle_make_temp_card_in_discard_and_deck(CardId::Burn, 1, &mut state);

        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(state.zones.draw_pile.len(), 1);
        assert_eq!(state.zones.discard_pile[0].id, CardId::Burn);
        assert_eq!(state.zones.draw_pile[0].id, CardId::Burn);
        assert_ne!(
            state.zones.discard_pile[0].uuid, state.zones.draw_pile[0].uuid,
            "Java MakeTempCardInDiscardAndDeckAction uses separate stat-equivalent copies"
        );
    }

    #[test]
    fn make_random_card_in_hand_uses_current_player_class_pool() {
        let mut state = blank_test_combat();
        state.meta.player_class = "Silent";

        handle_make_random_card_in_hand(Some(CardType::Attack), Some(0), &mut state);

        assert_eq!(state.zones.hand.len(), 1);
        let generated = &state.zones.hand[0];
        assert_eq!(generated.cost_for_turn, Some(0));
        assert!(
            crate::content::cards::silent_pool_for_type(CardType::Attack).contains(&generated.id),
            "random generated combat cards must come from the current character pool"
        );
        assert!(
            !crate::content::cards::ironclad_pool_for_type(CardType::Attack)
                .contains(&generated.id),
            "Silent random generated combat cards must not leak Ironclad cards"
        );
    }

    #[test]
    fn make_random_card_in_draw_pile_uses_current_player_class_pool() {
        let mut state = blank_test_combat();
        state.meta.player_class = "Silent";
        state.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 1)];
        state.zones.card_uuid_counter = 1;

        handle_make_random_card_in_draw_pile(Some(CardType::Skill), Some(0), false, &mut state);

        assert_eq!(state.zones.draw_pile.len(), 2);
        let generated = &state.zones.draw_pile[0];
        assert_eq!(generated.cost_for_turn, Some(0));
        assert!(
            crate::content::cards::silent_pool_for_type(CardType::Skill).contains(&generated.id),
            "random generated draw-pile cards must come from the current character pool"
        );
        assert!(
            !crate::content::cards::ironclad_pool_for_type(CardType::Skill).contains(&generated.id),
            "Silent random generated draw-pile cards must not leak Ironclad cards"
        );
        assert_eq!(state.zones.draw_pile[1].id, CardId::Strike);
    }

    #[test]
    fn make_temp_card_in_draw_pile_non_random_goes_to_top() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];
        state.zones.card_uuid_counter = 2;

        handle_make_temp_card_in_draw_pile(CardId::Wound, 1, false, false, false, &mut state);

        assert_eq!(state.zones.draw_pile[0].id, CardId::Wound);
        assert_eq!(state.zones.draw_pile[1].id, CardId::Strike);
        assert_eq!(state.zones.draw_pile[2].id, CardId::Defend);
    }

    #[test]
    fn make_temp_card_in_draw_pile_to_bottom_goes_under_existing_cards() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];
        state.zones.card_uuid_counter = 2;

        handle_make_temp_card_in_draw_pile(CardId::Wound, 1, false, true, false, &mut state);

        assert_eq!(state.zones.draw_pile[0].id, CardId::Strike);
        assert_eq!(state.zones.draw_pile[1].id, CardId::Defend);
        assert_eq!(state.zones.draw_pile[2].id, CardId::Wound);
    }

    #[test]
    fn random_draw_pile_insert_does_not_put_card_on_top_when_pile_is_nonempty() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];

        state.add_card_to_draw_pile_random_spot(CombatCard::new(CardId::Wound, 3));

        assert_eq!(state.zones.draw_pile[0].id, CardId::Strike);
        assert!(state
            .zones
            .draw_pile
            .iter()
            .any(|card| card.id == CardId::Wound));
    }

    #[test]
    fn random_draw_pile_insert_maps_java_bottom_to_top_order() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
            CombatCard::new(CardId::Bash, 3),
        ];
        let java_insert_index = state
            .rng
            .card_random_rng
            .clone()
            .random(state.zones.draw_pile.len() as i32 - 1)
            as usize;
        let expected_rust_index = state.zones.draw_pile.len() - java_insert_index;

        state.add_card_to_draw_pile_random_spot(CombatCard::new(CardId::Wound, 4));

        assert_eq!(state.zones.draw_pile[expected_rust_index].id, CardId::Wound);
    }

    #[test]
    fn draw_pile_to_hand_by_type_matches_java_temp_group_rng_sequence() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
            CombatCard::new(CardId::Bash, 3),
            CombatCard::new(CardId::Strike, 4),
        ];
        let mut expected_rng = state.rng.clone();
        let mut expected_candidates = Vec::new();
        for uuid in [4_u32, 3, 1] {
            if expected_candidates.is_empty() {
                expected_candidates.push(uuid);
            } else {
                let index = expected_rng
                    .card_random_rng
                    .random(expected_candidates.len() as i32 - 1)
                    as usize;
                expected_candidates.insert(index, uuid);
            }
        }
        crate::runtime::rng::shuffle_with_random_long(
            &mut expected_candidates,
            &mut expected_rng.shuffle_rng,
        );
        let expected_uuid = expected_candidates[0];

        handle_draw_pile_to_hand_by_type(1, CardType::Attack, &mut state);

        assert_eq!(state.zones.hand.len(), 1);
        assert_eq!(state.zones.hand[0].uuid, expected_uuid);
        assert!(!state
            .zones
            .draw_pile
            .iter()
            .any(|card| card.uuid == expected_uuid));
        assert_eq!(
            state.rng.card_random_rng.counter,
            expected_rng.card_random_rng.counter
        );
        assert_eq!(
            state.rng.shuffle_rng.counter,
            expected_rng.shuffle_rng.counter
        );
    }

    #[test]
    fn draw_pile_to_hand_by_type_overflow_discards_selected_card() {
        let mut state = blank_test_combat();
        for uuid in 10..20 {
            state.zones.hand.push(CombatCard::new(CardId::Defend, uuid));
        }
        state.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 1)];

        handle_draw_pile_to_hand_by_type(1, CardType::Attack, &mut state);

        assert_eq!(state.zones.hand.len(), 10);
        assert!(state.zones.draw_pile.is_empty());
        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(state.zones.discard_pile[0].uuid, 1);
    }

    fn seed_with_next_card_random_boolean(desired: bool) -> u64 {
        for seed in 1..10_000 {
            let mut rng = crate::runtime::rng::StsRng::new(seed);
            if rng.random_boolean() == desired {
                return seed;
            }
        }
        panic!("failed to find cardRandomRng seed with next randomBoolean={desired}");
    }

    #[test]
    fn use_card_done_resets_free_to_play_once_before_zone_move() {
        let mut discarded = blank_test_combat();
        let mut free_strike = CombatCard::new(CardId::Strike, 90);
        free_strike.free_to_play_once = true;
        discarded.zones.limbo = vec![free_strike];

        handle_use_card_done(false, &mut discarded);

        assert_eq!(discarded.zones.discard_pile.len(), 1);
        assert!(
            !discarded.zones.discard_pile[0].free_to_play_once,
            "Java UseCardAction clears freeToPlayOnce before discarding the used card"
        );

        let mut exhausted = blank_test_combat();
        let mut free_havoc_target = CombatCard::new(CardId::Strike, 91);
        free_havoc_target.free_to_play_once = true;
        free_havoc_target.exhaust_override = Some(true);
        exhausted.zones.limbo = vec![free_havoc_target];

        handle_use_card_done(true, &mut exhausted);

        assert_eq!(exhausted.zones.exhaust_pile.len(), 1);
        assert!(
            !exhausted.zones.exhaust_pile[0].free_to_play_once,
            "Java UseCardAction clears freeToPlayOnce before exhausting the used card"
        );
    }

    #[test]
    fn use_card_done_applies_strange_spoon_to_exhaust_on_use_once_cards() {
        for expected_saved in [true, false] {
            let mut state = blank_test_combat();
            state
                .entities
                .player
                .add_relic(RelicState::new(RelicId::StrangeSpoon));
            state.rng.card_random_rng = crate::runtime::rng::StsRng::new(
                seed_with_next_card_random_boolean(expected_saved),
            );

            let mut havoc_target = CombatCard::new(CardId::Strike, 92);
            havoc_target.free_to_play_once = true;
            havoc_target.exhaust_override = Some(true);
            state.zones.limbo = vec![havoc_target];

            handle_use_card_done(true, &mut state);

            assert_eq!(
                state.rng.card_random_rng.counter, 1,
                "Java Strange Spoon uses cardRandomRng.randomBoolean() when exhaustCard is true"
            );
            if expected_saved {
                assert!(state.zones.exhaust_pile.is_empty());
                assert_eq!(state.zones.discard_pile.len(), 1);
                assert_eq!(state.zones.discard_pile[0].id, CardId::Strike);
                assert_eq!(
                    state.zones.discard_pile[0].exhaust_override, None,
                    "Java clears exhaustOnUseOnce after UseCardAction resolves"
                );
                assert!(!state.zones.discard_pile[0].free_to_play_once);
            } else {
                assert!(state.zones.discard_pile.is_empty());
                assert_eq!(state.zones.exhaust_pile.len(), 1);
                assert_eq!(state.zones.exhaust_pile[0].id, CardId::Strike);
                assert!(!state.zones.exhaust_pile[0].free_to_play_once);
            }
        }
    }

    #[test]
    fn use_card_done_does_not_consume_spoon_rng_without_spoon_or_exhaust() {
        let mut no_spoon = blank_test_combat();
        no_spoon.rng.card_random_rng = crate::runtime::rng::StsRng::new(7);
        let before_counter = no_spoon.rng.card_random_rng.counter;
        no_spoon.zones.limbo = vec![CombatCard::new(CardId::Strike, 93)];

        handle_use_card_done(true, &mut no_spoon);

        assert_eq!(no_spoon.rng.card_random_rng.counter, before_counter);
        assert_eq!(no_spoon.zones.exhaust_pile.len(), 1);

        let mut not_exhausting = blank_test_combat();
        not_exhausting
            .entities
            .player
            .add_relic(RelicState::new(RelicId::StrangeSpoon));
        not_exhausting.rng.card_random_rng = crate::runtime::rng::StsRng::new(7);
        let before_counter = not_exhausting.rng.card_random_rng.counter;
        not_exhausting.zones.limbo = vec![CombatCard::new(CardId::Strike, 94)];

        handle_use_card_done(false, &mut not_exhausting);

        assert_eq!(not_exhausting.rng.card_random_rng.counter, before_counter);
        assert_eq!(not_exhausting.zones.discard_pile.len(), 1);
    }

    #[test]
    fn queued_direct_card_accepts_zero_hp_target_if_java_dead_flags_are_clear() {
        let mut state = blank_test_combat();
        let mut zero_hp = test_monster(EnemyId::JawWorm);
        zero_hp.id = 700;
        zero_hp.current_hp = 0;
        zero_hp.is_dying = false;
        zero_hp.half_dead = false;
        zero_hp.is_escaped = false;
        state.entities.monsters = vec![zero_hp];

        handle_play_card_direct(
            Box::new(CombatCard::new(CardId::Strike, 701)),
            Some(700),
            false,
            &mut state,
        );

        assert_eq!(
            state.zones.limbo.len(),
            1,
            "Java GameActionManager checks isDeadOrEscaped(), not currentHealth, before useCard"
        );
        assert!(matches!(
            state.pop_next_action(),
            Some(Action::Damage(crate::runtime::action::DamageInfo {
                target: 700,
                ..
            }))
        ));
    }

    #[test]
    fn failed_autoplay_target_cleanup_matches_java_can_use_path() {
        let mut state = blank_test_combat();
        let mut dying = test_monster(EnemyId::JawWorm);
        dying.id = 710;
        dying.current_hp = 0;
        dying.is_dying = true;
        state.entities.monsters = vec![dying];
        state.enqueue_card_play(
            crate::runtime::combat::QueuedCardPlay {
                card: CombatCard::new(CardId::Strike, 711),
                target: Some(710),
                energy_on_use: 0,
                ignore_energy_total: true,
                autoplay: true,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: crate::runtime::combat::QueuedCardSource::Normal,
            },
            false,
        );

        let flush = state
            .pop_next_action()
            .expect("queued autoplay card should schedule flush");
        crate::engine::action_handlers::execute_action(flush, &mut state);

        assert!(matches!(
            state.pop_next_action(),
            Some(Action::UseCardDone {
                should_exhaust: false
            }),
        ));
        assert!(
            state.zones.limbo.iter().any(|card| card.uuid == 711),
            "Java failed autoplay canUse path still routes the card through UseCardAction"
        );
    }
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

    state.queue_actions(actions);
}

pub fn handle_clear_card_queue(state: &mut CombatState) {
    state.zones.queued_cards.clear();
    state.engine.retain(|a| {
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
        .push(crate::runtime::combat::MetaChange::AddCardToMasterDeck(
            card_id,
        ));
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
            let pre_actions = crate::content::monsters::resolve_pre_battle_actions(
                state,
                *enemy_id,
                &entity_clone,
                crate::content::monsters::PreBattleLegacyRng::Misc,
            );
            for action in pre_actions {
                state.queue_action_back(action);
            }
        }
    }

    // 2. Relic pre-battle hooks (e.g. Snecko Eye applying Confusion)
    let pre_battle_actions = crate::content::relics::hooks::at_pre_battle(state);
    state.queue_actions(pre_battle_actions);

    // Auto-chain Phase 2
    state.queue_action_back(crate::runtime::action::Action::BattleStartPreDrawTrigger);
}

pub fn handle_battle_start_pre_draw_trigger(state: &mut CombatState) {
    let pre_draw_actions = crate::content::relics::hooks::at_battle_start_pre_draw(state);
    state.queue_actions(pre_draw_actions);

    // Auto-chain Phase 3 Initial Draw
    let draw_amount = crate::engine::core::compute_player_turn_start_draw_count(state);
    if draw_amount > 0 {
        state.queue_action_back(crate::runtime::action::Action::DrawCards(
            draw_amount as u32,
        ));
    }

    // Auto-chain Phase 4
    state.queue_action_back(crate::runtime::action::Action::BattleStartTrigger);
}

pub fn handle_battle_start_trigger(state: &mut CombatState) {
    // Relic battle-start hooks (e.g. Akabeko, Marbles)
    let battle_start_actions = crate::content::relics::hooks::at_battle_start(state);
    state.queue_actions(battle_start_actions);
}
