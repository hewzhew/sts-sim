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
use crate::content::powers::PowerId;

pub fn handle_draw_cards(amount: u32, state: &mut CombatState) {
    let has_no_draw = state.power_db.get(&0).map_or(false, |powers| {
        powers.iter().any(|p| p.power_type == PowerId::NoDraw)
    });
    if has_no_draw {
        return;
    }
    for _ in 0..amount {
        if state.draw_pile.is_empty() && !state.discard_pile.is_empty() {
            state.draw_pile.append(&mut state.discard_pile);
            crate::rng::shuffle_with_random_long(&mut state.draw_pile, &mut state.rng.shuffle_rng);
            let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
            crate::engine::core::queue_actions(&mut state.action_queue, shuffle_actions);
        }
        if state.draw_pile.is_empty() {
            break;
        }
        let mut card = state.draw_pile.remove(0);

        if card.id == CardId::Void {
            let void_actions = crate::content::cards::status::void::on_drawn(state);
            crate::engine::core::queue_actions(&mut state.action_queue, void_actions);
        }

        let has_corruption = state.power_db.get(&0).map_or(false, |powers| {
            powers.iter().any(|p| p.power_type == PowerId::Corruption)
        });
        if has_corruption {
            crate::content::cards::ironclad::corruption::corruption_on_card_draw(state, &mut card);
        }

        if state.hand.len() < 10 {
            state.hand.push(card);
        } else {
            state.discard_pile.push(card);
        }
    }
}

pub fn handle_empty_deck_shuffle(state: &mut CombatState) {
    if state.draw_pile.is_empty() && !state.discard_pile.is_empty() {
        state.draw_pile.append(&mut state.discard_pile);
        crate::rng::shuffle_with_random_long(&mut state.draw_pile, &mut state.rng.shuffle_rng);
        let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
        crate::engine::core::queue_actions(&mut state.action_queue, shuffle_actions);
    }
}

pub fn handle_discard_card(card_uuid: u32, state: &mut CombatState) {
    if let Some(pos) = state.hand.iter().position(|c| c.uuid == card_uuid) {
        let card = state.hand.remove(pos);
        state.discard_pile.push(card);
        let discard_actions = crate::content::relics::hooks::on_discard(state);
        crate::engine::core::queue_actions(&mut state.action_queue, discard_actions);
    }
}

pub fn handle_exhaust_card(card_uuid: u32, source_pile: crate::state::PileType, state: &mut CombatState) {
    let mut removed_card = None;
    match source_pile {
        crate::state::PileType::Hand => {
            if let Some(pos) = state.hand.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.hand.remove(pos));
            }
        },
        crate::state::PileType::Draw => {
            if let Some(pos) = state.draw_pile.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.draw_pile.remove(pos));
            }
        },
        crate::state::PileType::Discard => {
            if let Some(pos) = state.discard_pile.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.discard_pile.remove(pos));
            }
        },
        crate::state::PileType::Limbo => {
            if let Some(pos) = state.limbo.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.limbo.remove(pos));
            }
        },
        _ => {}
    }
    if let Some(card) = removed_card {
        let mut after_actions = crate::content::relics::hooks::on_exhaust(state);
        let card_hooks = crate::content::cards::resolve_card_on_exhaust(&card, state);
        after_actions.extend(card_hooks);

        state.exhaust_pile.push(card);
        crate::engine::core::queue_actions(&mut state.action_queue, after_actions);
    }
}

pub fn handle_move_card(card_uuid: u32, from: crate::state::PileType, to: crate::state::PileType, state: &mut CombatState) {
    let mut removed_card = None;
    match from {
        crate::state::PileType::Hand => {
            if let Some(pos) = state.hand.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.hand.remove(pos));
            }
        },
        crate::state::PileType::Draw => {
            if let Some(pos) = state.draw_pile.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.draw_pile.remove(pos));
            }
        },
        crate::state::PileType::Discard => {
            if let Some(pos) = state.discard_pile.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.discard_pile.remove(pos));
            }
        },
        crate::state::PileType::Exhaust => {
            if let Some(pos) = state.exhaust_pile.iter().position(|c| c.uuid == card_uuid) {
                removed_card = Some(state.exhaust_pile.remove(pos));
            }
        },
        _ => {}
    }
    if let Some(card) = removed_card {
        match to {
            crate::state::PileType::Hand => {
                if state.hand.len() < 10 { state.hand.push(card); }
                else { state.discard_pile.push(card); }
            },
            crate::state::PileType::Draw => state.draw_pile.insert(0, card),
            crate::state::PileType::Discard => state.discard_pile.push(card),
            crate::state::PileType::Exhaust => state.exhaust_pile.push(card),
            _ => {}
        }
    }
}

pub fn handle_make_temp_card_in_hand(card_id: CardId, amount: u8, upgraded: bool, state: &mut CombatState) {
    for _ in 0..amount {
        state.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
        if upgraded { card.upgrades = 1; }
        if state.hand.len() < 10 {
            state.hand.push(card);
        } else {
            state.discard_pile.push(card);
        }
    }
}

pub fn handle_make_temp_card_in_discard(card_id: CardId, amount: u8, upgraded: bool, state: &mut CombatState) {
    for _ in 0..amount {
        state.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
        if upgraded { card.upgrades = 1; }
        state.discard_pile.push(card);
    }
}

pub fn handle_make_temp_card_in_draw_pile(card_id: CardId, amount: u8, random_spot: bool, upgraded: bool, state: &mut CombatState) {
    for _ in 0..amount {
        state.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
        if upgraded { card.upgrades = 1; }
        if random_spot && !state.draw_pile.is_empty() {
            let idx = state.rng.card_random_rng.random(state.draw_pile.len() as i32) as usize;
            state.draw_pile.insert(idx, card);
        } else {
            state.draw_pile.push(card);
        }
    }
}

pub fn handle_make_copy_in_hand(original: Box<crate::combat::CombatCard>, amount: u8, state: &mut CombatState) {
    for _ in 0..amount {
        state.card_uuid_counter += 1;
        let mut card = (*original).clone();
        card.uuid = state.card_uuid_counter;
        if state.hand.len() < 10 {
            state.hand.push(card);
        } else {
            state.discard_pile.push(card);
        }
    }
}

pub fn handle_make_copy_in_discard(original: Box<crate::combat::CombatCard>, amount: u8, state: &mut CombatState) {
    for _ in 0..amount {
        state.card_uuid_counter += 1;
        let mut card = (*original).clone();
        card.uuid = state.card_uuid_counter;
        state.discard_pile.push(card);
    }
}

pub fn handle_make_temp_card_in_discard_and_deck(card_id: CardId, amount: u8, state: &mut CombatState) {
    for _ in 0..amount {
        state.card_uuid_counter += 1;
        let card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
        state.discard_pile.push(card.clone());
        let pos = state.rng.card_random_rng.random(state.draw_pile.len() as i32) as usize;
        state.draw_pile.insert(pos, card);
    }
}

pub fn handle_reduce_all_hand_costs(amount: u8, state: &mut CombatState) {
    for card in state.hand.iter_mut() {
        let def = crate::content::cards::get_card_definition(card.id);
        if def.cost >= 0 {
            let current = card.cost_for_turn.unwrap_or(def.cost as u8);
            card.cost_for_turn = Some(current.saturating_sub(amount));
        }
    }
}

pub fn handle_upgrade_all_in_hand(state: &mut CombatState) {
    for card in state.hand.iter_mut() {
        card.upgrades += 1;
    }
}

pub fn handle_upgrade_all_burns(state: &mut CombatState) {
    for card in state.draw_pile.iter_mut()
        .chain(state.discard_pile.iter_mut())
        .chain(state.hand.iter_mut())
    {
        if card.id == CardId::Burn {
            card.upgrades += 1;
        }
    }
}

pub fn handle_upgrade_card(card_uuid: u32, state: &mut CombatState) {
    for card in state.hand.iter_mut()
        .chain(state.draw_pile.iter_mut())
        .chain(state.discard_pile.iter_mut())
    {
        if card.uuid == card_uuid {
            card.upgrades += 1;
            break;
        }
    }
}

pub fn handle_upgrade_random_card(state: &mut CombatState) {
    let upgradeable_uuids: Vec<u32> = state.hand.iter()
        .filter(|c| c.upgrades == 0 && crate::content::cards::get_card_definition(c.id).card_type != crate::content::cards::CardType::Status)
        .map(|c| c.uuid)
        .collect();
    if !upgradeable_uuids.is_empty() {
        let mut shuffled = upgradeable_uuids;
        crate::rng::shuffle_with_random_long(&mut shuffled, &mut state.rng.shuffle_rng);
        let target_uuid = shuffled[0];
        if let Some(card) = state.hand.iter_mut().find(|c| c.uuid == target_uuid) {
            card.upgrades += 1;
        }
    }
}

pub fn handle_modify_card_misc(card_uuid: u32, amount: i32, state: &mut CombatState) {
    for card in state.hand.iter_mut()
        .chain(state.draw_pile.iter_mut())
        .chain(state.discard_pile.iter_mut())
        .chain(state.exhaust_pile.iter_mut())
    {
        if card.uuid == card_uuid {
            card.misc_value = amount;
            break;
        }
    }
}

pub fn handle_randomize_hand_costs(state: &mut CombatState) {
    for card in state.hand.iter_mut() {
        let base_cost = crate::content::cards::get_card_definition(card.id).cost;
        if base_cost >= 0 {
            let new_cost = state.rng.card_random_rng.random(3) as u8;
            card.cost_for_turn = Some(new_cost);
        }
    }
}

pub fn handle_mummified_hand_effect(state: &mut CombatState) {
    let eligible: Vec<usize> = state.hand.iter().enumerate()
        .filter(|(_, c)| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.cost > 0
        })
        .map(|(i, _)| i)
        .collect();
    if !eligible.is_empty() {
        let idx = state.rng.card_random_rng.random(eligible.len() as i32 - 1) as usize;
        let card_idx = eligible[idx];
        let card = &mut state.hand[card_idx];
        let def = crate::content::cards::get_card_definition(card.id);
        let current = card.cost_for_turn.unwrap_or(def.cost as u8);
        card.cost_for_turn = Some(current.saturating_sub(1));
    }
}

pub fn handle_make_random_card_in_hand(card_type: Option<crate::content::cards::CardType>, cost_for_turn: Option<u8>, state: &mut CombatState) {
    let mut pool: Vec<CardId> = Vec::new();
    for &rarity in &[
        crate::content::cards::CardRarity::Common,
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Rare,
    ] {
        for &id in crate::content::cards::ironclad_pool_for_rarity(rarity) {
            if let Some(ct) = card_type {
                let def = crate::content::cards::get_card_definition(id);
                if def.card_type != ct { continue; }
            }
            pool.push(id);
        }
    }
    if !pool.is_empty() {
        let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        state.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
        if let Some(cost) = cost_for_turn {
            card.cost_for_turn = Some(cost);
        }
        if state.hand.len() < 10 {
            state.hand.push(card);
        } else {
            state.discard_pile.push(card);
        }
    }
}

pub fn handle_make_random_colorless_card_in_hand(cost_for_turn: Option<u8>, state: &mut CombatState) {
    let mut pool: Vec<CardId> = Vec::new();
    for &id in crate::content::cards::COLORLESS_UNCOMMON_POOL {
        let def = crate::content::cards::get_card_definition(id);
        if !def.tags.contains(&crate::content::cards::CardTag::Healing) {
            pool.push(id);
        }
    }
    for &id in crate::content::cards::COLORLESS_RARE_POOL {
        let def = crate::content::cards::get_card_definition(id);
        if !def.tags.contains(&crate::content::cards::CardTag::Healing) {
            pool.push(id);
        }
    }
    if !pool.is_empty() {
        let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        state.card_uuid_counter += 1;
        let mut card = crate::combat::CombatCard::new(card_id, state.card_uuid_counter);
        if let Some(cost) = cost_for_turn {
            card.cost_for_turn = Some(cost);
        }
        if state.hand.len() < 10 {
            state.hand.push(card);
        } else {
            state.discard_pile.push(card);
        }
    }
}

pub fn handle_use_card_done(should_exhaust: bool, state: &mut CombatState) {
    if let Some(card) = state.limbo.pop() {
        if should_exhaust {
            state.exhaust_pile.push(card);
            let exhaust_actions = crate::content::relics::hooks::on_exhaust(state);
            crate::engine::core::queue_actions(&mut state.action_queue, exhaust_actions);
        } else {
            state.discard_pile.push(card);
        }
    }
}

pub fn handle_play_card_direct(card: Box<crate::combat::CombatCard>, target: Option<usize>, purge: bool, state: &mut CombatState) {
    let card_id = card.id;
    let mut played_card = *card;

    crate::content::cards::evaluate_card(&mut played_card, state, target);

    let card_actions = crate::content::cards::resolve_card_play(card_id, state, &played_card, target);
    crate::engine::core::queue_actions(&mut state.action_queue, card_actions);

    let relic_actions = crate::content::relics::hooks::on_use_card(state, card_id);
    crate::engine::core::queue_actions(&mut state.action_queue, relic_actions);

    for entity_id in std::iter::once(0usize).chain(state.monsters.iter().map(|m| m.id)) {
        if let Some(powers) = state.power_db.get(&entity_id).cloned() {
            for power in &powers {
                let hook_actions = crate::content::powers::resolve_power_on_card_played(
                    power.power_type, state, entity_id, &played_card, power.amount
                );
                for a in hook_actions {
                    state.action_queue.push_back(a);
                }
            }
        }
    }

    state.counters.cards_played_this_turn += 1;
    let def = crate::content::cards::get_card_definition(card_id);
    if def.card_type == crate::content::cards::CardType::Attack {
        state.counters.attacks_played_this_turn += 1;
    }

    if !purge {
        state.discard_pile.push(played_card);
    }
}

pub fn handle_use_potion(slot: usize, target: Option<usize>, state: &mut CombatState) {
    if let Some(Some(potion)) = state.potions.get(slot).cloned() {
        if potion.id == crate::content::potions::PotionId::FairyPotion {
            return;
        }
        if potion.id == crate::content::potions::PotionId::SmokeBomb && state.is_boss_fight {
            return;
        }
        let def = crate::content::potions::get_potion_definition(potion.id);
        let mut potency = def.base_potency;
        if state.player.has_relic(crate::content::relics::RelicId::SacredBark) {
            potency *= 2;
        }
        let actions = crate::content::potions::potion_effects::get_potion_actions(potion.id, target, potency);
        let relic_actions = crate::content::relics::hooks::on_use_potion(state, 0);
        let mut combined = relic_actions;
        combined.extend(actions);
        crate::engine::core::queue_actions(&mut state.action_queue, combined);
        state.potions[slot] = None;
    }
}

pub fn handle_obtain_potion(state: &mut CombatState) {
    if state.player.has_relic(crate::content::relics::RelicId::Sozu) {
        return;
    }
    if let Some(slot) = state.potions.iter().position(|p| p.is_none()) {
        let potion_id = crate::content::potions::random_potion(
            &mut state.rng.potion_rng,
            crate::content::potions::PotionClass::Ironclad,
            true,
        );
        state.potions[slot] = Some(crate::content::potions::Potion::new(potion_id, 40000 + slot as u32));
    }
}

pub fn handle_end_turn_trigger(state: &mut CombatState) {
    let mut actions = smallvec::SmallVec::new();

    // 1. Player Powers
    if let Some(player_powers) = state.power_db.get(&0) {
        for power in player_powers.clone() {
            actions.extend(crate::content::powers::resolve_power_at_end_of_turn(
                power.power_type, state, 0, power.amount
            ).into_iter().map(|a| ActionInfo { action: a, insertion_mode: AddTo::Bottom }));
        }
    }

    // 2. Ethereal exhaust
    for card in &state.hand {
        let def = crate::content::cards::get_card_definition(card.id);
        if def.ethereal {
            actions.push(ActionInfo {
                action: Action::ExhaustCard { card_uuid: card.uuid, source_pile: crate::state::PileType::Hand },
                insertion_mode: AddTo::Bottom,
            });
        }
    }

    // 3. Relics
    actions.extend(crate::content::relics::hooks::at_end_of_turn(state));

    // 4. Orbs
    actions.extend(crate::content::orbs::hooks::trigger_end_of_turn_orbs(state));

    // 5. Curses and Burns in hand
    for card in &state.hand {
        if card.id == CardId::Burn {
            actions.extend(crate::content::cards::status::burn::on_end_turn_in_hand(state, card));
        }
        if card.id == CardId::Regret {
            actions.extend(crate::content::cards::curses::regret::on_end_turn_in_hand(state));
        }
        if card.id == CardId::Decay {
            actions.extend(crate::content::cards::curses::decay::on_end_turn_in_hand(state));
        }
        if card.id == CardId::Doubt {
            actions.extend(crate::content::cards::curses::doubt::on_end_turn_in_hand(state));
        }
        if card.id == CardId::Pride {
            actions.extend(crate::content::cards::curses::pride::on_end_turn_in_hand(state));
        }
        if card.id == CardId::Shame {
            actions.extend(crate::content::cards::curses::shame::on_end_turn_in_hand(state));
        }
    }

    // 6. Stances
    actions.extend(crate::content::stances::hooks::on_end_of_turn(state));

    crate::engine::core::queue_actions(&mut state.action_queue, actions);
}

pub fn handle_clear_card_queue(state: &mut CombatState) {
    state.action_queue.retain(|a| {
        if let Action::PlayCardDirect { .. } = a { false }
        else if let Action::UseCardDone { .. } = a { false }
        else { true }
    });
    state.limbo.clear();
}

pub fn handle_add_card_to_master_deck(card_id: CardId, state: &mut CombatState) {
    state.meta_changes.push(crate::combat::MetaChange::AddCardToMasterDeck(card_id));
}

pub fn handle_battle_start_trigger(state: &mut CombatState) {
    // 1. Monster pre-battle actions (CurlUp, ModeShift, etc.)
    // Java: AbstractRoom.initializeBattle() calls usePreBattleAction() for each monster
    let monsters_snapshot: Vec<_> = state.monsters.iter()
        .filter_map(|m| crate::content::monsters::EnemyId::from_id(m.monster_type).map(|eid| (eid, m.id)))
        .collect();
    for (enemy_id, monster_id) in &monsters_snapshot {
        if let Some(entity) = state.monsters.iter().find(|m| m.id == *monster_id) {
            let entity_clone = entity.clone();
            let pre_actions = crate::content::monsters::resolve_pre_battle_action(
                *enemy_id, &entity_clone, &mut state.rng.misc_rng, state.ascension_level
            );
            for action in pre_actions {
                state.action_queue.push_back(action);
            }
        }
    }

    // 2. Relic battle-start hooks
    let battle_start_actions = crate::content::relics::hooks::at_battle_start(state);
    crate::engine::core::queue_actions(&mut state.action_queue, battle_start_actions);
}
