use crate::content::cards::CardId;
use crate::runtime::combat::CombatState;

pub fn handle_apply_bullet_time(state: &mut CombatState) {
    for card in &mut state.zones.hand {
        card.set_cost_for_turn_java(0);
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

pub fn handle_reduce_retained_hand_costs(amount: i32, state: &mut CombatState) {
    if amount <= 0 {
        return;
    }
    for card in state.zones.hand.iter_mut() {
        if card.retain_override == Some(true) || crate::content::cards::is_self_retain(card) {
            card.modify_cost_for_combat_java(-amount);
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
        crate::content::cards::upgrade_card_once_java(card);
    }
}

pub fn handle_upgrade_all_cards_in_combat(state: &mut CombatState) {
    for card in state
        .zones
        .hand
        .iter_mut()
        .chain(state.zones.draw_pile.iter_mut())
        .chain(state.zones.discard_pile.iter_mut())
        .chain(state.zones.exhaust_pile.iter_mut())
    {
        crate::content::cards::upgrade_card_once_java(card);
    }
}

pub fn handle_upgrade_all_burns(state: &mut CombatState) {
    for card in state
        .zones
        .draw_pile
        .iter_mut()
        .chain(state.zones.discard_pile.iter_mut())
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
            crate::content::cards::upgrade_card_once_java(card);
            break;
        }
    }
}

pub fn handle_upgrade_random_card(state: &mut CombatState) {
    let upgradeable_uuids: Vec<u32> = state
        .zones
        .hand
        .iter()
        .filter(|c| crate::content::cards::can_upgrade_card_once(c))
        .map(|c| c.uuid)
        .collect();
    if !upgradeable_uuids.is_empty() {
        let mut shuffled = upgradeable_uuids;
        crate::runtime::rng::shuffle_with_random_long(&mut shuffled, &mut state.rng.shuffle_rng);
        let target_uuid = shuffled[0];
        if let Some(card) = state.zones.hand.iter_mut().find(|c| c.uuid == target_uuid) {
            crate::content::cards::upgrade_card_once_java(card);
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

pub fn handle_gash(card_uuid: u32, amount: i32, state: &mut CombatState) {
    let apply = |card: &mut crate::runtime::combat::CombatCard| {
        let def = crate::content::cards::get_card_definition(card.id);
        let upgraded_base = def.base_damage + (card.upgrades as i32) * def.upgrade_damage;
        let current = card.base_damage_override.unwrap_or(upgraded_base);
        card.base_damage_override = Some((current + amount).max(0));
    };

    for card in state
        .zones
        .hand
        .iter_mut()
        .chain(state.zones.draw_pile.iter_mut())
        .chain(state.zones.discard_pile.iter_mut())
        .chain(state.zones.limbo.iter_mut())
    {
        if card.id == CardId::Claw || card.uuid == card_uuid {
            apply(card);
        }
    }
}

pub fn handle_modify_card_block(card_uuid: u32, amount: i32, state: &mut CombatState) {
    state
        .zones
        .for_each_java_battle_instance_mut_by_uuid(card_uuid, |card| {
            let def = crate::content::cards::get_card_definition(card.id);
            let upgraded_base = def.base_block + (card.upgrades as i32) * def.upgrade_block;
            let current = card.base_block_override.unwrap_or(upgraded_base);
            card.base_block_override = Some(current + amount);
        });
}

pub fn handle_reduce_card_cost_for_combat(card_uuid: u32, amount: i32, state: &mut CombatState) {
    if amount <= 0 {
        return;
    }
    let delta = -amount;
    state
        .zones
        .for_each_java_battle_instance_mut_by_uuid(card_uuid, |card| {
            card.modify_cost_for_combat_java(delta);
        });
    state
        .zones
        .for_each_queued_instance_mut_by_uuid(card_uuid, |card| {
            card.modify_cost_for_combat_java(delta);
        });
}

pub fn handle_randomize_hand_costs(state: &mut CombatState) {
    for card in state.zones.hand.iter_mut() {
        let current_cost = card.combat_cost_without_turn_override_java();
        if current_cost >= 0 {
            let new_cost = state.rng.card_random_rng.random(3);
            if current_cost != new_cost {
                card.set_combat_and_turn_cost_java(new_cost);
            }
        }
    }
}
