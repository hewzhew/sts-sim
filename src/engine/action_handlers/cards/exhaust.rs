use crate::content::powers::store;
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

pub fn handle_exhaust_from_hand(
    amount: usize,
    random: bool,
    any_number: bool,
    can_pick_zero: bool,
    state: &mut CombatState,
) {
    if state.zones.hand.is_empty() {
        return;
    }

    if !any_number && state.zones.hand.len() <= amount {
        while !state.zones.hand.is_empty() {
            let card = state
                .zones
                .hand
                .pop()
                .expect("checked non-empty hand before ExhaustAction auto move");
            move_card_to_exhaust_pile(card, state);
        }
        return;
    }

    if random {
        for _ in 0..amount {
            if state.zones.hand.is_empty() {
                break;
            }
            let idx = state
                .rng
                .card_random_rng
                .random(state.zones.hand.len() as i32 - 1) as usize;
            let card = state.zones.hand.remove(idx);
            move_card_to_exhaust_pile(card, state);
        }
        return;
    }

    state.queue_action_front(Action::SuspendForHandSelect {
        min: if can_pick_zero { 0 } else { 1 },
        max: amount.min(u8::MAX as usize) as u8,
        can_cancel: can_pick_zero,
        filter: crate::state::HandSelectFilter::Any,
        reason: crate::state::HandSelectReason::Exhaust,
    });
}

pub fn handle_recycle(state: &mut CombatState) {
    if state.zones.hand.is_empty() {
        return;
    }

    if state.zones.hand.len() == 1 {
        let card_uuid = state.zones.hand[0].uuid;
        handle_recycle_selected_card(card_uuid, state);
        return;
    }

    state.queue_action_front(Action::SuspendForHandSelect {
        min: 1,
        max: 1,
        can_cancel: false,
        filter: crate::state::HandSelectFilter::Any,
        reason: crate::state::HandSelectReason::Recycle,
    });
}

pub fn handle_recycle_selected_card(card_uuid: u32, state: &mut CombatState) {
    let Some(card) = state.zones.hand.iter().find(|card| card.uuid == card_uuid) else {
        return;
    };
    let cost_for_turn = card.cost_for_turn_java();
    let energy_gain = if cost_for_turn == -1 {
        state.turn.energy as i32
    } else if cost_for_turn > 0 {
        cost_for_turn
    } else {
        0
    };

    if energy_gain > 0 {
        state.queue_action_front(Action::GainEnergy {
            amount: energy_gain,
        });
    }
    handle_exhaust_card(card_uuid, crate::state::PileType::Hand, state);
}
