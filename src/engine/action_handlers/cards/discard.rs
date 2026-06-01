use crate::content::cards::CardId;
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscardHookOrder {
    /// Java end-turn all-card discard path: count the discard, but do not
    /// trigger manual-discard card hooks or relic hooks.
    None,
    /// Java DiscardAction random path with endTurn=true: card hook still fires,
    /// but GameActionManager.incrementDiscard does not fire relic hooks.
    CardOnly,
    /// Java DiscardAction: moveToDiscardPile, triggerOnManualDiscard,
    /// then GameActionManager.incrementDiscard.
    CardThenRelics,
    /// Java DiscardSpecificCardAction/GamblingChipAction: moveToDiscardPile,
    /// GameActionManager.incrementDiscard, then triggerOnManualDiscard.
    RelicsThenCard,
}

fn queue_manual_discard_hooks(
    card: &crate::runtime::combat::CombatCard,
    order: DiscardHookOrder,
    state: &mut CombatState,
) {
    match order {
        DiscardHookOrder::None => {
            state.turn.increment_cards_discarded();
        }
        DiscardHookOrder::CardOnly => {
            let card_actions = crate::content::cards::resolve_card_on_manual_discard(card, state);
            state.queue_actions(card_actions);
            state.turn.increment_cards_discarded();
        }
        DiscardHookOrder::CardThenRelics => {
            let card_actions = crate::content::cards::resolve_card_on_manual_discard(card, state);
            state.queue_actions(card_actions);
            state.turn.increment_cards_discarded();
            apply_player_update_cards_on_discard(state);
            let relic_actions = crate::content::relics::hooks::on_discard(state);
            state.queue_actions(relic_actions);
        }
        DiscardHookOrder::RelicsThenCard => {
            state.turn.increment_cards_discarded();
            apply_player_update_cards_on_discard(state);
            let relic_actions = crate::content::relics::hooks::on_discard(state);
            state.queue_actions(relic_actions);
            let card_actions = crate::content::cards::resolve_card_on_manual_discard(card, state);
            state.queue_actions(card_actions);
        }
    }
}

fn apply_player_update_cards_on_discard(state: &mut CombatState) {
    for card in state
        .zones
        .hand
        .iter_mut()
        .chain(state.zones.discard_pile.iter_mut())
        .chain(state.zones.draw_pile.iter_mut())
    {
        if card.id == CardId::Eviscerate {
            card.set_cost_for_turn_java(card.cost_for_turn_java() - 1);
        }
    }
}

fn move_hand_card_to_discard_at(pos: usize, hook_order: DiscardHookOrder, state: &mut CombatState) {
    let card = state.zones.hand.remove(pos);
    state.add_card_to_discard_pile_top(card.clone());
    queue_manual_discard_hooks(&card, hook_order, state);
}

pub fn handle_scrape_follow_up(state: &mut CombatState) {
    let drawn = state.runtime.last_drawn_cards.clone();
    for record in drawn {
        let Some(pos) = state
            .zones
            .hand
            .iter()
            .position(|card| card.uuid == record.card_uuid)
        else {
            continue;
        };
        let card = &state.zones.hand[pos];
        if card.cost_for_turn_java() == 0 || card.free_to_play_once {
            continue;
        }
        move_hand_card_to_discard_at(pos, DiscardHookOrder::CardThenRelics, state);
    }
}

pub fn handle_calculated_gamble(draw_extra: bool, state: &mut CombatState) {
    let count = state.zones.hand.len() as u32;
    if count == 0 && !draw_extra {
        return;
    }

    let draw_count = count + u32::from(draw_extra);
    state.queue_action_front(Action::DrawCards(draw_count));
    state.queue_action_front(Action::DiscardFromHand {
        amount: count as i32,
        random: true,
        end_turn: false,
    });
}

pub fn handle_discard_card(card_uuid: u32, state: &mut CombatState) {
    handle_discard_card_with_order(card_uuid, DiscardHookOrder::RelicsThenCard, state);
}

pub fn handle_discard_card_with_order(
    card_uuid: u32,
    hook_order: DiscardHookOrder,
    state: &mut CombatState,
) {
    if let Some(pos) = state.zones.hand.iter().position(|c| c.uuid == card_uuid) {
        move_hand_card_to_discard_at(pos, hook_order, state);
    }
}

pub fn handle_discard_from_hand(
    amount: i32,
    random: bool,
    end_turn: bool,
    state: &mut CombatState,
) {
    if state.are_monsters_basically_dead_java() {
        return;
    }

    if state.zones.hand.is_empty() {
        return;
    }

    if amount < 0 && !random {
        state.queue_action_front(Action::SuspendForHandSelect {
            min: 0,
            max: 99,
            can_cancel: true,
            filter: crate::state::HandSelectFilter::Any,
            reason: crate::state::HandSelectReason::Discard,
        });
        return;
    }

    let amount = amount.max(0) as usize;
    if state.zones.hand.len() <= amount {
        let hook_order = if end_turn {
            DiscardHookOrder::None
        } else {
            DiscardHookOrder::CardThenRelics
        };
        while !state.zones.hand.is_empty() {
            let top = state.zones.hand.len() - 1;
            move_hand_card_to_discard_at(top, hook_order, state);
        }
        return;
    }

    if random {
        let hook_order = if end_turn {
            DiscardHookOrder::CardOnly
        } else {
            DiscardHookOrder::CardThenRelics
        };
        for _ in 0..amount {
            if state.zones.hand.is_empty() {
                break;
            }
            let idx = state
                .rng
                .card_random_rng
                .random(state.zones.hand.len() as i32 - 1) as usize;
            move_hand_card_to_discard_at(idx, hook_order, state);
        }
        return;
    }

    state.queue_action_front(Action::SuspendForHandSelect {
        min: amount as u8,
        max: amount as u8,
        can_cancel: false,
        filter: crate::state::HandSelectFilter::Any,
        reason: crate::state::HandSelectReason::Discard,
    });
}

pub fn handle_discard_to_hand(card_uuid: u32, cost_for_turn: Option<u8>, state: &mut CombatState) {
    if state.zones.hand.len() >= 10 {
        return;
    }

    let Some(pos) = state
        .zones
        .discard_pile
        .iter()
        .position(|card| card.uuid == card_uuid)
    else {
        return;
    };

    let mut card = state.zones.discard_pile.remove(pos);
    if let Some(cost) = cost_for_turn {
        card.set_cost_for_turn_java(cost as i32);
    }
    crate::content::cards::evaluate_card(&mut card, state, None);
    state.zones.hand.push(card);
}
