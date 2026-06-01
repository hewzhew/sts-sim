use crate::content::powers::{store, PowerId};
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;

pub fn handle_limit_break(state: &mut CombatState) {
    if let Some(strength) = store::powers_for(state, 0)
        .and_then(|powers| powers.iter().find(|p| p.power_type == PowerId::Strength))
        .map(|power| power.amount)
    {
        state.queue_action_front(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: strength,
        });
    }
}

pub fn handle_block_per_non_attack(block_per_card: i32, state: &mut CombatState) {
    let non_attacks: Vec<u32> = state
        .zones
        .hand
        .iter()
        .filter(|c| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.card_type != crate::content::cards::CardType::Attack
        })
        .map(|c| c.uuid)
        .collect();

    // Java BlockPerNonAttackAction uses addToTop in two loops: first
    // GainBlockAction for every card, then ExhaustSpecificCardAction for every
    // card. The resulting queue executes all exhausts before all block gains,
    // and each group is reversed relative to hand iteration order.
    for _ in &non_attacks {
        state.queue_action_front(Action::GainBlock {
            target: 0,
            amount: block_per_card,
        });
    }
    for uuid in &non_attacks {
        state.queue_action_front(Action::ExhaustCard {
            card_uuid: *uuid,
            source_pile: crate::state::PileType::Hand,
        });
    }
}

pub fn handle_exhaust_all_non_attack(state: &mut CombatState) {
    let non_attacks: Vec<u32> = state
        .zones
        .hand
        .iter()
        .filter(|c| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.card_type != crate::content::cards::CardType::Attack
        })
        .map(|c| c.uuid)
        .collect();
    for uuid in non_attacks {
        state.queue_action_front(Action::ExhaustCard {
            card_uuid: uuid,
            source_pile: crate::state::PileType::Hand,
        });
    }
}

pub fn handle_exhaust_random_card(amount: usize, state: &mut CombatState) {
    for _ in 0..amount {
        if state.zones.hand.is_empty() {
            break;
        }
        let idx = state
            .rng
            .card_random_rng
            .random(state.zones.hand.len() as i32 - 1) as usize;
        let card_uuid = state.zones.hand[idx].uuid;
        crate::engine::action_handlers::cards::handle_exhaust_card(
            card_uuid,
            crate::state::PileType::Hand,
            state,
        );
    }
}
