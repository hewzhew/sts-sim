use crate::content::cards::{CardId, CardType};
use crate::content::powers::{store, PowerId};
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState, PowerPayload};

fn apply_master_reality_to_generated_card(
    card: &mut CombatCard,
    state: &CombatState,
    upgrade_call_sites: u8,
) {
    crate::content::cards::apply_master_reality_to_generated_card(card, state, upgrade_call_sites);
}

fn make_generated_card_from_id(
    card_id: CardId,
    uuid: u32,
    upgraded: bool,
    state: &CombatState,
) -> CombatCard {
    let mut card = CombatCard::new(card_id, uuid);
    if upgraded {
        card.upgrades = 1;
    }
    crate::content::cards::configure_costs_on_new_card(&mut card, state);
    card
}

fn make_random_pool_card_from_id(card_id: CardId, uuid: u32, state: &CombatState) -> CombatCard {
    crate::content::cards::make_fresh_card_copy_for_combat(card_id, uuid, state)
}

fn apply_make_random_draw_pile_cost_override(card: &mut CombatCard, cost_for_turn: Option<u8>) {
    let Some(cost) = cost_for_turn else {
        return;
    };
    if cost == 0 && card.combat_cost_without_turn_override_java() > 0 {
        card.set_combat_and_turn_cost_java(0);
    } else {
        card.set_cost_for_turn_java(cost as i32);
    }
}

pub(super) fn materialize_random_class_card_in_hand_action(
    action: &mut Action,
    state: &mut CombatState,
) {
    let (card_type, cost_for_turn) = match action {
        Action::MakeRandomCardInHand {
            card_type,
            cost_for_turn,
        } => (*card_type, *cost_for_turn),
        _ => return,
    };

    let pool = class_card_pool_for_type(state.meta.player_class.as_str(), card_type);
    if pool.is_empty() {
        return;
    }

    let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
    let mut card = make_random_pool_card_from_id(pool[idx], 0, state);
    if let Some(cost) = cost_for_turn {
        card.set_cost_for_turn_java(cost as i32);
    }

    let constructed =
        crate::content::cards::prepare_make_temp_card_in_hand_constructor(card, state);

    *action = Action::MakeConstructedCopyInHand {
        original: Box::new(constructed),
        amount: 1,
    };
}

pub(super) fn materialize_random_class_card_in_draw_pile_action(
    action: &mut Action,
    state: &mut CombatState,
) {
    let (card_type, cost_for_turn, random_spot) = match action {
        Action::MakeRandomCardInDrawPile {
            card_type,
            cost_for_turn,
            random_spot,
        } => (*card_type, *cost_for_turn, *random_spot),
        _ => return,
    };

    let pool = class_card_pool_for_type(state.meta.player_class.as_str(), card_type);
    if pool.is_empty() {
        return;
    }

    let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
    let mut card = make_random_pool_card_from_id(pool[idx], 0, state);
    apply_make_random_draw_pile_cost_override(&mut card, cost_for_turn);

    *action = Action::MakeCopyInDrawPile {
        original: Box::new(card),
        amount: 1,
        random_spot,
        to_bottom: false,
    };
}

pub(super) fn materialize_random_colorless_card_in_hand_action(
    action: &mut Action,
    state: &mut CombatState,
) {
    let (cost_for_turn, upgraded) = match action {
        Action::MakeRandomColorlessCardInHand {
            cost_for_turn,
            upgraded,
        } => (*cost_for_turn, *upgraded),
        _ => return,
    };

    let pool = state.colorless_combat_pool();
    if pool.is_empty() {
        return;
    }

    let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
    let mut card = CombatCard::new(pool[idx], 0);
    if upgraded {
        card.upgrades = 1;
    }
    if let Some(cost) = cost_for_turn {
        card.set_cost_for_turn_java(cost as i32);
    }

    let constructed =
        crate::content::cards::prepare_make_temp_card_in_hand_constructor(card, state);

    *action = Action::MakeConstructedCopyInHand {
        original: Box::new(constructed),
        amount: 1,
    };
}

fn apply_generated_card_entering_hand_mechanics(card: &mut CombatCard, state: &CombatState) {
    if store::has_power(state, 0, PowerId::Corruption) {
        crate::content::cards::ironclad::corruption::corruption_on_card_draw(state, card);
    }
    crate::content::cards::evaluate_card(card, state, None);
}

fn add_generated_card_to_hand_or_discard(mut card: CombatCard, state: &mut CombatState) {
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
        let card = make_generated_card_from_id(card_id, state.next_card_uuid(), upgraded, state);
        add_generated_card_to_hand_or_discard(card, state);
    }
}

pub fn handle_make_temp_card_in_discard(
    card_id: CardId,
    amount: u8,
    upgraded: bool,
    state: &mut CombatState,
) {
    if amount >= 6 {
        return;
    }
    for _ in 0..amount {
        let mut card =
            make_generated_card_from_id(card_id, state.next_card_uuid(), upgraded, state);
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
        let mut card =
            make_generated_card_from_id(card_id, state.next_card_uuid(), upgraded, state);
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

pub fn handle_make_copy_in_hand(original: Box<CombatCard>, amount: u8, state: &mut CombatState) {
    for _ in 0..amount {
        let card = original.make_stat_equivalent_copy_with_uuid(state.next_card_uuid());
        add_generated_card_to_hand_or_discard(card, state);
    }
}

fn add_constructed_card_to_hand_or_discard(mut card: CombatCard, state: &mut CombatState) {
    if state.zones.hand.len() < 10 {
        apply_master_reality_to_generated_card(&mut card, state, 1);
        apply_generated_card_entering_hand_mechanics(&mut card, state);
        state.zones.hand.push(card);
    } else {
        state.add_card_to_discard_pile_top(card);
    }
}

pub fn handle_make_constructed_copy_in_hand(
    original: Box<CombatCard>,
    amount: u8,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        let card = original.make_stat_equivalent_copy_with_uuid(state.next_card_uuid());
        add_constructed_card_to_hand_or_discard(card, state);
    }
}

pub fn handle_make_copy_in_draw_pile(
    original: Box<CombatCard>,
    amount: u8,
    random_spot: bool,
    to_bottom: bool,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        let mut card = original.make_stat_equivalent_copy_with_uuid(state.next_card_uuid());
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

fn nightmare_payload_copy(card: &CombatCard) -> CombatCard {
    let mut copy = card.make_stat_equivalent_copy_with_uuid(card.uuid);
    copy.reset_attributes_java();
    copy
}

pub fn queue_nightmare_power_front(card: &CombatCard, amount: u8, state: &mut CombatState) {
    let payload = PowerPayload::Card(nightmare_payload_copy(card));
    let instance_id = state.next_power_instance_id();
    state.queue_action_front(Action::ApplyPowerWithPayload {
        source: 0,
        target: 0,
        power_id: PowerId::Nightmare,
        amount: amount as i32,
        instance_id: Some(instance_id),
        extra_data: None,
        payload,
    });
}

pub fn handle_nightmare(amount: u8, state: &mut CombatState) {
    if state.zones.hand.is_empty() {
        return;
    }

    if state.zones.hand.len() == 1 {
        let card = state.zones.hand[0].clone();
        queue_nightmare_power_front(&card, amount, state);
        return;
    }

    state.queue_action_front(Action::SuspendForHandSelect {
        min: 1,
        max: 1,
        can_cancel: false,
        filter: crate::state::HandSelectFilter::Any,
        reason: crate::state::HandSelectReason::Nightmare { amount },
    });
}

pub fn handle_make_copy_in_discard(original: Box<CombatCard>, amount: u8, state: &mut CombatState) {
    if amount >= 6 {
        return;
    }
    for _ in 0..amount {
        let mut card = original.make_stat_equivalent_copy_with_uuid(state.next_card_uuid());
        apply_master_reality_to_generated_card(&mut card, state, 1);
        state.add_card_to_discard_pile_top(card);
    }
}

pub fn handle_return_stasis_card(card_uuid: u32, to_hand: bool, state: &mut CombatState) {
    let Some(pos) = state
        .zones
        .limbo
        .iter()
        .position(|card| card.uuid == card_uuid)
    else {
        return;
    };
    let held = state.zones.limbo.remove(pos);
    let mut card = held.make_same_instance_of_java();

    if to_hand {
        // Java StasisPower uses MakeTempCardInHandAction(card, false, true).
        // The action constructor upgrades once under Master Reality. If the
        // card actually enters hand, ShowCardAndAddToHandEffect upgrades again;
        // if hand overflow sends it to discard, only the constructor upgrade
        // has affected the same-UUID copy.
        apply_master_reality_to_generated_card(&mut card, state, 1);
        if state.zones.hand.len() < 10 {
            apply_master_reality_to_generated_card(&mut card, state, 1);
            apply_generated_card_entering_hand_mechanics(&mut card, state);
            state.zones.hand.push(card);
        } else {
            state.add_card_to_discard_pile_top(card);
        }
    } else {
        // Java full-hand Stasis path uses MakeTempCardInDiscardAction(card, true);
        // that sameUUID constructor deliberately skips Master Reality.
        state.add_card_to_discard_pile_top(card);
    }
}

pub fn handle_make_temp_card_in_discard_and_deck(
    card_id: CardId,
    amount: u8,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        // Java MakeTempCardInDiscardAndDeckAction queues the draw-pile effect
        // before the discard effect. Preserve that order for UUID/replay parity.
        let mut draw_card =
            make_generated_card_from_id(card_id, state.next_card_uuid(), false, state);
        apply_master_reality_to_generated_card(&mut draw_card, state, 1);
        state.add_card_to_draw_pile_random_spot(draw_card);

        let mut discard_card =
            make_generated_card_from_id(card_id, state.next_card_uuid(), false, state);
        apply_master_reality_to_generated_card(&mut discard_card, state, 1);
        state.add_card_to_discard_pile_top(discard_card);
    }
}

pub(crate) fn class_card_pool_for_type(
    player_class: &str,
    card_type: Option<CardType>,
) -> Vec<CardId> {
    crate::content::cards::class_combat_card_pool_for_type(player_class, card_type)
}

pub fn handle_make_random_card_in_hand(
    card_type: Option<CardType>,
    cost_for_turn: Option<u8>,
    state: &mut CombatState,
) {
    let pool = class_card_pool_for_type(state.meta.player_class.as_str(), card_type);
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
    card_type: Option<CardType>,
    cost_for_turn: Option<u8>,
    random_spot: bool,
    state: &mut CombatState,
) {
    let pool = class_card_pool_for_type(state.meta.player_class.as_str(), card_type);
    if !pool.is_empty() {
        let idx = state.rng.card_random_rng.random(pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        let mut card = make_random_pool_card_from_id(card_id, state.next_card_uuid(), state);
        apply_make_random_draw_pile_cost_override(&mut card, cost_for_turn);
        apply_master_reality_to_generated_card(&mut card, state, 2);
        if random_spot {
            state.add_card_to_draw_pile_random_spot(card);
        } else {
            state.add_card_to_draw_pile_top(card);
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
        let mut card = CombatCard::new(card_id, state.next_card_uuid());
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
            let mut action = Action::MakeRandomColorlessCardInHand {
                cost_for_turn: Some(0),
                upgraded,
            };
            materialize_random_colorless_card_in_hand_action(&mut action, state);
            state.queue_action_back(action);
        }
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_conjure_blade(free_to_play_once: bool, energy_on_use: i32, state: &mut CombatState) {
    let mut effect = if energy_on_use != -1 {
        energy_on_use
    } else {
        state.turn.energy as i32
    };
    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::ChemicalX)
    {
        effect += 2;
    }

    let mut card =
        make_generated_card_from_id(CardId::Expunger, state.next_card_uuid(), false, state);
    card.misc_value = effect.max(0);
    apply_master_reality_to_generated_card(&mut card, state, 2);
    state.add_card_to_draw_pile_random_spot(card);

    if !free_to_play_once {
        state.turn.spend_energy(state.turn.energy as i32);
    }
}
