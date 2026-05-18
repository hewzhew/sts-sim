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
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
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

fn apply_card_trigger_when_drawn(
    card: &mut crate::runtime::combat::CombatCard,
    state: &mut CombatState,
) {
    if card.id == CardId::Eviscerate {
        card.set_cost_for_turn_java(
            card.combat_cost_without_turn_override_java()
                - state.turn.counters.cards_discarded_this_turn as i32,
        );
    } else if card.id == CardId::EndlessAgony {
        let constructed =
            crate::content::cards::prepare_make_temp_card_in_hand_constructor(card.clone(), state);
        state.queue_action_front(Action::MakeConstructedCopyInHand {
            original: Box::new(constructed),
            amount: 1,
        });
    } else if card.id == CardId::DeusExMachina {
        let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
        let amount = evaluated.base_magic_num_mut.max(0).min(u8::MAX as i32) as u8;
        state.queue_action_front(
            crate::content::cards::make_constructed_temp_card_in_hand_action(
                CardId::Miracle,
                amount,
                false,
                state,
            ),
        );
        state.queue_action_front(Action::ExhaustCard {
            card_uuid: card.uuid,
            source_pile: crate::state::PileType::Hand,
        });
    }
}

fn move_hand_card_to_discard_at(pos: usize, hook_order: DiscardHookOrder, state: &mut CombatState) {
    let card = state.zones.hand.remove(pos);
    state.add_card_to_discard_pile_top(card.clone());
    queue_manual_discard_hooks(&card, hook_order, state);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DrawHistoryMode {
    Untouched,
    Track { clear_history: bool },
}

pub fn handle_draw_cards(amount: u32, state: &mut CombatState) {
    handle_draw_cards_inner(amount, DrawHistoryMode::Untouched, state);
}

pub fn handle_draw_for_unique_orb_types(amount_per_orb_type: u32, state: &mut CombatState) {
    if amount_per_orb_type == 0 {
        return;
    }

    let mut seen = Vec::new();
    for orb in &state.entities.player.orbs {
        if orb.id != crate::runtime::combat::OrbId::Empty && !seen.contains(&orb.id) {
            seen.push(orb.id);
        }
    }

    let to_draw = seen.len() as u32 * amount_per_orb_type;
    if to_draw > 0 {
        state.queue_action_front(Action::DrawCards(to_draw));
    }
}

pub fn handle_barrage(damage: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    let count = state
        .entities
        .player
        .orbs
        .iter()
        .filter(|orb| orb.id != crate::runtime::combat::OrbId::Empty)
        .count();

    for _ in 0..count {
        state.queue_action_front(Action::Damage(damage.clone()));
    }
}

pub fn handle_draw_cards_with_history(amount: u32, clear_history: bool, state: &mut CombatState) {
    handle_draw_cards_inner(amount, DrawHistoryMode::Track { clear_history }, state);
}

fn queue_split_draw_action_front(amount: u32, mode: DrawHistoryMode, state: &mut CombatState) {
    match mode {
        DrawHistoryMode::Untouched => state.queue_action_front(Action::DrawCards(amount)),
        DrawHistoryMode::Track { .. } => state.queue_action_front(Action::DrawCardsWithHistory {
            amount,
            clear_history: false,
        }),
    }
}

fn handle_draw_cards_inner(amount: u32, mode: DrawHistoryMode, state: &mut CombatState) {
    if matches!(
        mode,
        DrawHistoryMode::Track {
            clear_history: true
        }
    ) {
        state.runtime.last_drawn_cards.clear();
    }

    let has_no_draw = store::has_power(state, 0, PowerId::NoDraw);
    if has_no_draw {
        return;
    }

    if amount == 0 {
        return;
    }

    let deck_size = state.zones.draw_pile.len();
    let discard_size = state.zones.discard_pile.len();
    if deck_size + discard_size == 0 {
        return;
    }

    let hand_size = state.zones.hand.len();
    if hand_size >= 10 {
        return;
    }

    let amount = amount.min((10 - hand_size) as u32);
    if amount == 0 {
        return;
    }

    if amount as usize > deck_size {
        let remaining = amount - deck_size as u32;
        queue_split_draw_action_front(remaining, mode, state);
        state.queue_action_front(Action::EmptyDeckShuffle);
        if deck_size != 0 {
            queue_split_draw_action_front(deck_size as u32, mode, state);
        }
        return;
    }

    for _ in 0..amount {
        if state.zones.draw_pile.is_empty() {
            return;
        }
        let mut card = state
            .draw_top_card()
            .expect("draw pile was checked non-empty before drawing");

        apply_card_trigger_when_drawn(&mut card, state);

        if card.id == CardId::Void {
            let void_actions = crate::content::cards::status::void::on_drawn(state);
            state.queue_actions(void_actions);
        }

        if matches!(mode, DrawHistoryMode::Track { .. }) {
            state
                .runtime
                .last_drawn_cards
                .push(crate::runtime::combat::DrawnCardRecord {
                    card_uuid: card.uuid,
                    card_id: card.id,
                });
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

pub fn handle_escape_plan_block_if_skill(block: i32, state: &mut CombatState) {
    if state.runtime.last_drawn_cards.iter().any(|record| {
        crate::content::cards::get_card_definition(record.card_id).card_type
            == crate::content::cards::CardType::Skill
    }) {
        state.queue_action_front(Action::GainBlock {
            target: 0,
            amount: block,
        });
    }
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

pub fn handle_blade_fury(upgraded: bool, state: &mut CombatState) {
    let count = state.zones.hand.len() as u8;
    state.queue_action_front(
        crate::content::cards::make_constructed_temp_card_in_hand_action(
            CardId::Shiv,
            count,
            upgraded,
            state,
        ),
    );
    state.queue_action_front(Action::DiscardFromHand {
        amount: count as i32,
        random: false,
        end_turn: false,
    });
}

pub fn handle_apply_bullet_time(state: &mut CombatState) {
    for card in &mut state.zones.hand {
        card.set_cost_for_turn_java(0);
    }
}

pub fn handle_unload_non_attack(state: &mut CombatState) {
    let non_attacks: Vec<u32> = state
        .zones
        .hand
        .iter()
        .filter(|card| {
            crate::content::cards::get_card_definition(card.id).card_type
                != crate::content::cards::CardType::Attack
        })
        .map(|card| card.uuid)
        .collect();

    for uuid in non_attacks {
        state.queue_action_front(Action::DiscardCard { card_uuid: uuid });
    }
}

pub fn handle_expertise_draw(target_hand_size: i32, state: &mut CombatState) {
    let to_draw = target_hand_size - state.zones.hand.len() as i32;
    if to_draw > 0 {
        state.queue_action_front(Action::DrawCards(to_draw as u32));
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

pub fn handle_forethought(upgraded: bool, state: &mut CombatState) {
    if state.zones.hand.is_empty() {
        return;
    }

    if !upgraded && state.zones.hand.len() == 1 {
        let mut card = state
            .zones
            .hand
            .pop()
            .expect("checked non-empty hand before Forethought auto move");
        if card.combat_cost_without_turn_override_java() > 0 {
            card.free_to_play_once = true;
        }
        state.add_card_to_draw_pile_bottom(card);
        return;
    }

    state.queue_action_front(Action::SuspendForHandSelect {
        min: if upgraded { 0 } else { 1 },
        max: if upgraded { 99 } else { 1 },
        can_cancel: upgraded,
        filter: crate::state::HandSelectFilter::Any,
        reason: crate::state::HandSelectReason::PutToBottomOfDraw,
    });
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

pub fn handle_shuffle_all_into_draw(state: &mut CombatState) {
    let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
    state.queue_actions(shuffle_actions);

    state.queue_action_front(Action::PutOnDeck {
        amount: 99,
        random: true,
    });

    if state.zones.discard_pile.is_empty() {
        return;
    }

    crate::runtime::rng::shuffle_with_random_long(
        &mut state.zones.discard_pile,
        &mut state.rng.shuffle_rng,
    );
    let mut moved = std::mem::take(&mut state.zones.discard_pile);
    moved.reverse();
    moved.append(&mut state.zones.draw_pile);
    state.zones.draw_pile = moved;
}

pub fn handle_shuffle_draw_pile(trigger_relics: bool, state: &mut CombatState) {
    if trigger_relics {
        let shuffle_actions = crate::content::relics::hooks::on_shuffle(state);
        state.queue_actions(shuffle_actions);
    }
    if state.zones.draw_pile.len() <= 1 {
        return;
    }
    state.zones.draw_pile.reverse();
    crate::runtime::rng::shuffle_with_random_long(
        &mut state.zones.draw_pile,
        &mut state.rng.shuffle_rng,
    );
    state.zones.draw_pile.reverse();
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

pub fn handle_all_cost_to_hand(cost_target: i32, state: &mut CombatState) {
    let matching_uuids: Vec<u32> = state
        .zones
        .discard_pile
        .iter()
        .filter(|card| {
            card.combat_cost_without_turn_override_java() == cost_target || card.free_to_play_once
        })
        .map(|card| card.uuid)
        .collect();

    for uuid in matching_uuids {
        state.queue_action_back(Action::DiscardToHand {
            card_uuid: uuid,
            cost_for_turn: None,
        });
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
    state: &CombatState,
) -> crate::runtime::combat::CombatCard {
    let mut card = crate::runtime::combat::CombatCard::new(card_id, uuid);
    if upgraded {
        card.upgrades = 1;
    }
    crate::content::cards::configure_costs_on_new_card(&mut card, state);
    card
}

fn make_random_pool_card_from_id(
    card_id: CardId,
    uuid: u32,
    state: &CombatState,
) -> crate::runtime::combat::CombatCard {
    crate::content::cards::make_fresh_card_copy_for_combat(card_id, uuid, state)
}

fn apply_make_random_draw_pile_cost_override(
    card: &mut crate::runtime::combat::CombatCard,
    cost_for_turn: Option<u8>,
) {
    let Some(cost) = cost_for_turn else {
        return;
    };
    if cost == 0 && card.combat_cost_without_turn_override_java() > 0 {
        card.set_combat_and_turn_cost_java(0);
    } else {
        card.set_cost_for_turn_java(cost as i32);
    }
}

fn materialize_random_class_card_in_hand_action(action: &mut Action, state: &mut CombatState) {
    let (card_type, cost_for_turn) = match action {
        Action::MakeRandomCardInHand {
            card_type,
            cost_for_turn,
        } => (*card_type, *cost_for_turn),
        _ => return,
    };

    let pool = class_card_pool_for_type(state.meta.player_class, card_type);
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

fn materialize_random_class_card_in_draw_pile_action(action: &mut Action, state: &mut CombatState) {
    let (card_type, cost_for_turn, random_spot) = match action {
        Action::MakeRandomCardInDrawPile {
            card_type,
            cost_for_turn,
            random_spot,
        } => (*card_type, *cost_for_turn, *random_spot),
        _ => return,
    };

    let pool = class_card_pool_for_type(state.meta.player_class, card_type);
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

fn materialize_random_colorless_card_in_hand_action(action: &mut Action, state: &mut CombatState) {
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
    let mut card = crate::runtime::combat::CombatCard::new(pool[idx], 0);
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
    if upgrade {
        crate::content::cards::upgrade_card_once_java(&mut card);
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

fn add_constructed_card_to_hand_or_discard(
    mut card: crate::runtime::combat::CombatCard,
    state: &mut CombatState,
) {
    if state.zones.hand.len() < 10 {
        apply_master_reality_to_generated_card(&mut card, state, 1);
        apply_generated_card_entering_hand_mechanics(&mut card, state);
        state.zones.hand.push(card);
    } else {
        state.add_card_to_discard_pile_top(card);
    }
}

pub fn handle_make_constructed_copy_in_hand(
    original: Box<crate::runtime::combat::CombatCard>,
    amount: u8,
    state: &mut CombatState,
) {
    for _ in 0..amount {
        let card = original.make_stat_equivalent_copy_with_uuid(state.next_card_uuid());
        add_constructed_card_to_hand_or_discard(card, state);
    }
}

pub fn handle_make_copy_in_draw_pile(
    original: Box<crate::runtime::combat::CombatCard>,
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

fn nightmare_payload_copy(
    card: &crate::runtime::combat::CombatCard,
) -> crate::runtime::combat::CombatCard {
    let mut copy = card.make_stat_equivalent_copy_with_uuid(card.uuid);
    copy.reset_attributes_java();
    copy
}

pub fn queue_nightmare_power_front(
    card: &crate::runtime::combat::CombatCard,
    amount: u8,
    state: &mut CombatState,
) {
    let payload = crate::runtime::combat::PowerPayload::Card(nightmare_payload_copy(card));
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

pub fn handle_halt(block: i32, additional: i32, state: &mut CombatState) {
    let amount = if state.entities.player.stance == crate::runtime::combat::StanceId::Wrath {
        block + additional
    } else {
        block
    };
    state.queue_action_front(Action::GainBlock { target: 0, amount });
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

fn class_card_pool_for_type(
    player_class: &str,
    card_type: Option<crate::content::cards::CardType>,
) -> Vec<CardId> {
    crate::content::cards::class_combat_card_pool_for_type(player_class, card_type)
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
        apply_make_random_draw_pile_cost_override(&mut card, cost_for_turn);
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

pub fn handle_aggregate_energy(divide_amount: i32, state: &mut CombatState) {
    if divide_amount <= 0 {
        return;
    }
    let amount = state.zones.draw_pile.len() as i32 / divide_amount;
    if amount > 0 {
        state.turn.adjust_energy(amount);
    }
}

pub fn handle_tempest(
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
    let mut effect = crate::content::relics::hooks::on_calculate_x_cost(state, base_effect);
    if upgraded {
        effect += 1;
    }

    if effect > 0 {
        for _ in 0..effect {
            state.queue_action_back(Action::ChannelOrb(crate::runtime::combat::OrbId::Lightning));
        }
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_multicast(
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    if !state
        .entities
        .player
        .orbs
        .first()
        .is_some_and(|orb| orb.id != crate::runtime::combat::OrbId::Empty)
    {
        return;
    }

    let base_effect = if energy_on_use != -1 {
        energy_on_use
    } else {
        state.turn.energy as i32
    };
    let mut effect = crate::content::relics::hooks::on_calculate_x_cost(state, base_effect);
    if upgraded {
        effect += 1;
    }

    if effect > 0 {
        for _ in 0..effect - 1 {
            state.queue_action_back(Action::EvokeOrbWithoutRemoving);
        }
        state.queue_action_back(Action::EvokeOrb);
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

pub fn handle_meditate(amount: u8, state: &mut CombatState) {
    if amount == 0 || state.zones.discard_pile.is_empty() {
        return;
    }

    if state.zones.discard_pile.len() <= amount as usize {
        let uuids: Vec<u32> = state
            .zones
            .discard_pile
            .iter()
            .map(|card| card.uuid)
            .collect();
        for uuid in uuids {
            if let Some(pos) = state
                .zones
                .discard_pile
                .iter()
                .position(|card| card.uuid == uuid)
            {
                state.zones.discard_pile[pos].retain_override = Some(true);
                if state.zones.hand.len() < 10 {
                    let card = state.zones.discard_pile.remove(pos);
                    state.zones.hand.push(card);
                }
            }
        }
        return;
    }

    state.queue_action_front(Action::SuspendForGridSelect {
        source_pile: crate::state::PileType::Discard,
        min: amount,
        max: amount,
        can_cancel: false,
        filter: crate::state::GridSelectFilter::Any,
        reason: crate::state::GridSelectReason::DiscardToHandRetain,
    });
}

pub fn handle_reinforced_body(
    block_amount: i32,
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
            state.queue_action_back(Action::GainBlock {
                target: 0,
                amount: block_amount,
            });
        }
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct UseCardPlacementOverrides {
    rebound: bool,
}

fn apply_use_card_after_use_hooks(
    card: &crate::runtime::combat::CombatCard,
    state: &mut CombatState,
) -> UseCardPlacementOverrides {
    let mut overrides = UseCardPlacementOverrides::default();
    let player_powers = crate::content::powers::store::powers_snapshot_for(state, 0);
    for power in player_powers {
        if power.power_type == PowerId::Rebound {
            overrides.rebound |=
                crate::content::powers::defect::rebound::on_after_use_card(state, card);
        }
    }
    overrides
}

pub fn handle_use_card_after_use_hooks(
    mut card: crate::runtime::combat::CombatCard,
    state: &mut CombatState,
) {
    card.free_to_play_once = false;
    apply_use_card_after_use_hooks(&card, state);
}

pub fn handle_use_card_done(
    should_exhaust: bool,
    trigger_after_use_hooks: bool,
    state: &mut CombatState,
) {
    if let Some(mut card) = state.zones.limbo.pop() {
        let placement_overrides = if trigger_after_use_hooks {
            apply_use_card_after_use_hooks(&card, state)
        } else {
            UseCardPlacementOverrides::default()
        };

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
            if placement_overrides.rebound {
                state.add_card_to_draw_pile_top(card);
            } else if crate::content::cards::shuffle_back_into_draw_pile_when_played(&card) {
                state.add_card_to_draw_pile_random_spot(card);
            } else {
                state.add_card_to_discard_pile_top(card);
            }
        }
    }

    if state.turn.counters.early_end_turn_pending {
        state.turn.clear_early_end_turn_pending();
        state.begin_turn_transition();
        state.queue_action_back(Action::EndTurnTrigger);
    }
}

pub fn handle_queue_early_end_turn(state: &mut CombatState) {
    let queued_cards: Vec<_> = state.zones.queued_cards.drain(..).collect();
    for queued in queued_cards.into_iter().rev() {
        if queued.autoplay && !queued.purge_on_use {
            let should_exhaust = queued
                .card
                .exhaust_override
                .unwrap_or(crate::content::cards::exhausts_when_played(&queued.card));
            state.zones.limbo.push(queued.card);
            state.queue_action_back(Action::UseCardDone {
                should_exhaust,
                trigger_after_use_hooks: false,
            });
        }
    }
    state.turn.mark_early_end_turn_pending();
}

pub fn handle_skip_enemies_turn(state: &mut CombatState) {
    state.turn.mark_skip_monster_turn_pending();
}

pub fn handle_retain_non_ethereal_hand_cards(state: &mut CombatState) {
    for card in &mut state.zones.hand {
        if !crate::content::cards::is_ethereal(card) {
            card.retain_override = Some(true);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CardPlaySource {
    Hand,
    Direct,
}

fn execute_played_card(
    mut played_card: crate::runtime::combat::CombatCard,
    target: Option<usize>,
    purge: bool,
    source: CardPlaySource,
    state: &mut CombatState,
) {
    let card_id = played_card.id;
    let def = crate::content::cards::get_card_definition(card_id);

    crate::content::cards::evaluate_card(&mut played_card, state, target);

    let mut card_actions = crate::content::cards::resolve_card_play_with_context(
        card_id,
        state,
        &played_card,
        target,
        crate::content::cards::CardUseContext {
            played_from_hand: source == CardPlaySource::Hand,
        },
    );
    if card_id == CardId::Havoc || card_id == CardId::BouncingFlask {
        for action in &mut card_actions {
            match &mut action.action {
                Action::PlayTopCard { target, .. } | Action::BouncingFlask { target, .. }
                    if target.is_none() =>
                {
                    *target = targeting::pick_random_target(
                        state,
                        crate::state::TargetValidation::AnyEnemy,
                    );
                }
                _ => {}
            }
        }
    }
    for action in &mut card_actions {
        materialize_random_class_card_in_hand_action(&mut action.action, state);
        materialize_random_class_card_in_draw_pile_action(&mut action.action, state);
        materialize_random_colorless_card_in_hand_action(&mut action.action, state);
    }
    state.queue_actions(card_actions);

    let passive_card_actions = crate::content::cards::on_play_card(&played_card, state);
    state.queue_actions(passive_card_actions);
    crate::content::cards::trigger_cards_on_card_played(&played_card, state);

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
                | PowerId::Amplify
                | PowerId::EchoForm
                | PowerId::Burst
                | PowerId::Corruption
                | PowerId::Heatsink
                | PowerId::PenNibPower
                | PowerId::Storm
                | PowerId::Vigor
                | PowerId::FreeAttackPower => {
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
        state.queue_action_back(Action::UseCardDone {
            should_exhaust,
            trigger_after_use_hooks: true,
        });
    } else {
        state.queue_action_back(Action::UseCardAfterUseHooks {
            card: Box::new(played_card),
        });
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
    let free_attack_power_applies = def.card_type == crate::content::cards::CardType::Attack
        && crate::content::powers::store::power_amount(
            state,
            0,
            crate::content::powers::PowerId::FreeAttackPower,
        ) > 0;
    let effective_cost = if card.free_to_play_once || free_attack_power_applies {
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

    crate::content::powers::core::surrounded::face_target_for_surrounded_if_needed(state, target);

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
    execute_played_card(played_card, target, false, CardPlaySource::Hand, state);
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
            state.queue_action_front(Action::UseCardDone {
                should_exhaust,
                trigger_after_use_hooks: false,
            });
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
    execute_played_card(played_card, target, purge, CardPlaySource::Direct, state);
}

pub fn handle_use_potion(slot: usize, target: Option<usize>, state: &mut CombatState) {
    if let Some(Some(potion)) = state.entities.potions.get(slot).cloned() {
        if !crate::content::potions::potion_can_use_in_combat_like_java(&potion, state) {
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
        if potion.id == crate::content::potions::PotionId::EntropicBrew {
            let potion_class = potion_class_for_combat(state);
            let potion_slots = state.entities.potions.len();
            let mut actions = smallvec::SmallVec::<[ActionInfo; 4]>::new();
            for _ in 0..potion_slots {
                let potion_id = crate::content::potions::random_potion(
                    &mut state.rng.potion_rng,
                    potion_class,
                    true,
                );
                actions.push(ActionInfo {
                    action: Action::ObtainSpecificPotion(potion_id),
                    insertion_mode: AddTo::Bottom,
                });
            }
            actions.extend(crate::content::relics::hooks::on_use_potion(state, 0));
            state.queue_actions(actions);
            state.entities.potions[slot] = None;
            return;
        }
        if potion.id == crate::content::potions::PotionId::DistilledChaosPotion {
            let mut actions = smallvec::SmallVec::<[ActionInfo; 4]>::new();
            for _ in 0..potency.max(0) {
                actions.push(ActionInfo {
                    action: Action::PlayTopCard {
                        target: targeting::pick_random_target(
                            state,
                            crate::state::TargetValidation::AnyEnemy,
                        ),
                        exhaust: false,
                    },
                    insertion_mode: AddTo::Bottom,
                });
            }
            actions.extend(crate::content::relics::hooks::on_use_potion(state, 0));
            state.queue_actions(actions);
            state.entities.potions[slot] = None;
            return;
        }
        if potion.id == crate::content::potions::PotionId::EssenceOfDarkness {
            let orb_slots =
                (state.entities.player.max_orbs as usize).max(state.entities.player.orbs.len());
            let mut actions = smallvec::SmallVec::<[ActionInfo; 4]>::new();
            for _ in 0..orb_slots {
                for _ in 0..potency.max(0) {
                    actions.push(ActionInfo {
                        action: Action::ChannelOrb(crate::runtime::combat::OrbId::Dark),
                        insertion_mode: AddTo::Bottom,
                    });
                }
            }
            actions.extend(crate::content::relics::hooks::on_use_potion(state, 0));
            state.queue_actions(actions);
            state.entities.potions[slot] = None;
            return;
        }
        if potion.id == crate::content::potions::PotionId::LiquidMemories
            && state.zones.discard_pile.len() <= potency.max(0) as usize
        {
            let uuids: Vec<u32> = state.zones.discard_pile.iter().map(|c| c.uuid).collect();
            for uuid in uuids {
                if state.zones.hand.len() >= 10 {
                    break;
                }
                if let Some(pos) = state.zones.discard_pile.iter().position(|c| c.uuid == uuid) {
                    let mut card = state.zones.discard_pile.remove(pos);
                    card.set_cost_for_turn_java(0);
                    state.zones.hand.push(card);
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
        if potion.id == crate::content::potions::PotionId::FirePotion {
            let Some(target_id) = resolved_target else {
                return;
            };
            let mut output = potency.max(0);
            for power in crate::content::powers::store::powers_snapshot_for(state, target_id) {
                output = crate::content::powers::resolve_power_at_damage_final_receive(
                    power.power_type,
                    output,
                    power.amount,
                    DamageType::Thorns,
                );
            }
            let mut actions = smallvec::smallvec![ActionInfo {
                action: Action::Damage(DamageInfo {
                    source: 0,
                    target: target_id,
                    base: potency,
                    output,
                    damage_type: DamageType::Thorns,
                    is_modified: output != potency,
                }),
                insertion_mode: AddTo::Bottom,
            }];
            actions.extend(crate::content::relics::hooks::on_use_potion(state, 0));
            state.queue_actions(actions);
            state.entities.potions[slot] = None;
            return;
        }
        let actions = crate::content::potions::potion_effects::get_potion_actions(
            state,
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

fn potion_class_for_combat(state: &CombatState) -> crate::content::potions::PotionClass {
    match state.meta.player_class {
        "Silent" => crate::content::potions::PotionClass::Silent,
        "Defect" => crate::content::potions::PotionClass::Defect,
        "Watcher" => crate::content::potions::PotionClass::Watcher,
        _ => crate::content::potions::PotionClass::Ironclad,
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
    let resolved_target = target
        .or_else(|| targeting::pick_random_target(state, crate::state::TargetValidation::AnyEnemy));
    state.queue_action_back(Action::PlayTopCard {
        target: resolved_target,
        exhaust,
    });
}

pub fn handle_obtain_potion(state: &mut CombatState) {
    let potion_class = match state.meta.player_class {
        "Silent" => crate::content::potions::PotionClass::Silent,
        "Defect" => crate::content::potions::PotionClass::Defect,
        "Watcher" => crate::content::potions::PotionClass::Watcher,
        _ => crate::content::potions::PotionClass::Ironclad,
    };
    let potion_id =
        crate::content::potions::random_potion(&mut state.rng.potion_rng, potion_class, true);

    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::Sozu)
    {
        return;
    }
    if let Some(slot) = state.entities.potions.iter().position(|p| p.is_none()) {
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
        // Java DiscardAtEndOfTurnAction moves retain/selfRetain cards out of
        // hand before calling triggerOnEndOfPlayerTurn(), so explicit retain
        // wins over ethereal. Runic Pyramid does not set per-card retain and
        // therefore still allows ethereal cards to exhaust.
        if card.retain_override != Some(true)
            && !crate::content::cards::is_self_retain(card)
            && crate::content::cards::is_ethereal(card)
        {
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
        class_card_pool_for_type, handle_discard_pile_to_top_of_deck, handle_draw_cards,
        handle_draw_pile_to_hand_by_type, handle_end_turn_trigger,
        handle_make_constructed_copy_in_hand, handle_make_copy_in_discard,
        handle_make_random_card_in_draw_pile, handle_make_random_card_in_hand,
        handle_make_temp_card_in_discard, handle_make_temp_card_in_discard_and_deck,
        handle_make_temp_card_in_draw_pile, handle_make_temp_card_in_hand, handle_play_card_direct,
        handle_pre_battle_trigger, handle_queue_early_end_turn, handle_randomize_hand_costs,
        handle_return_stasis_card, handle_upgrade_all_burns, handle_upgrade_all_cards_in_combat,
        handle_upgrade_all_in_hand, handle_use_card_done, handle_use_potion,
        obtain_specific_potion_if_allowed,
    };
    use crate::content::cards::{CardId, CardType};
    use crate::content::monsters::EnemyId;
    use crate::content::potions::PotionId;
    use crate::content::powers::store;
    use crate::content::powers::PowerId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::action::Action;
    use crate::runtime::combat::{CombatCard, Power, QueuedCardPlay, QueuedCardSource};
    use crate::runtime::rng::StsRng;
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn draw_cards_splits_shuffle_like_java_draw_card_action() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 11),
        ];
        state.zones.discard_pile = vec![CombatCard::new(CardId::Bash, 12)];
        state.queue_action_back(Action::GainEnergy { amount: 1 });

        handle_draw_cards(3, &mut state);

        assert!(
            state.zones.hand.is_empty(),
            "Java DrawCardAction does not draw immediately when amount exceeds draw pile; it splits into top-queued actions"
        );
        assert_eq!(state.pop_next_action(), Some(Action::DrawCards(2)));
        assert_eq!(state.pop_next_action(), Some(Action::EmptyDeckShuffle));
        assert_eq!(state.pop_next_action(), Some(Action::DrawCards(1)));
        assert_eq!(
            state.pop_next_action(),
            Some(Action::GainEnergy { amount: 1 }),
            "split draw actions are addToTop-style and must run before previously queued actions"
        );
    }

    #[test]
    fn monster_group_pre_battle_uses_monster_hp_rng_for_louse_curl_up_like_java() {
        let mut state = blank_test_combat();
        state.rng.monster_hp_rng = StsRng::new(41);
        state.rng.misc_rng = StsRng::new(41);
        state.entities.monsters = vec![test_monster(EnemyId::LouseNormal)];

        let mut expected_hp_rng = state.rng.monster_hp_rng.clone();
        let expected_curl_up = expected_hp_rng.random_range(3, 7);

        handle_pre_battle_trigger(&mut state);

        assert_eq!(state.rng.monster_hp_rng, expected_hp_rng);
        assert_eq!(
            state.rng.misc_rng.counter, 0,
            "Java Louse.usePreBattleAction consumes AbstractDungeon.monsterHpRng, not miscRng"
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::ApplyPower {
                source: 1,
                target: 1,
                power_id: PowerId::CurlUp,
                amount: expected_curl_up,
            })
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::BattleStartPreDrawTrigger)
        );
    }

    #[test]
    fn draw_cards_caps_amount_to_available_hand_space_before_split() {
        let mut state = blank_test_combat();
        state.zones.hand = (0..9)
            .map(|idx| CombatCard::new(CardId::Defend, 100 + idx))
            .collect();
        state.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 200),
            CombatCard::new(CardId::Strike, 201),
        ];

        handle_draw_cards(5, &mut state);

        assert_eq!(state.zones.hand.len(), 9);
        assert_eq!(state.pop_next_action(), Some(Action::EmptyDeckShuffle));
        assert_eq!(state.pop_next_action(), Some(Action::DrawCards(1)));
        assert_eq!(state.pop_next_action(), None);
    }

    #[test]
    fn snecko_oil_randomize_updates_combat_cost_and_turn_cost_like_java() {
        let mut state = blank_test_combat();
        let mut modified = CombatCard::new(CardId::Strike, 10);
        modified.set_combat_and_turn_cost_java(3);
        modified.set_cost_for_turn_java(0);
        while {
            let mut probe = state.rng.card_random_rng.clone();
            probe.random(3) == modified.combat_cost_without_turn_override_java()
        } {
            state.rng.card_random_rng.random(3);
        }
        let mut expected_rng = state.rng.card_random_rng.clone();
        let expected_cost = expected_rng.random(3);
        assert_ne!(
            expected_cost,
            modified.combat_cost_without_turn_override_java()
        );
        state.zones.hand = vec![modified, CombatCard::new(CardId::Whirlwind, 11)];

        handle_randomize_hand_costs(&mut state);

        assert_eq!(
            state.zones.hand[0].combat_cost_without_turn_override_java(),
            expected_cost,
            "Java RandomizeHandCostAction mutates AbstractCard.cost, not only costForTurn"
        );
        assert_eq!(state.zones.hand[0].cost_for_turn_java(), expected_cost);
        assert_eq!(
            state.zones.hand[1].combat_cost_without_turn_override_java(),
            -1,
            "X-cost cards short-circuit before consuming a random cost roll"
        );
        assert_eq!(state.rng.card_random_rng.counter, expected_rng.counter);
    }

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
    fn fire_potion_applies_enemy_final_receive_before_damage_action_like_java() {
        let mut state = blank_test_combat();
        let mut nemesis_like = test_monster(EnemyId::JawWorm);
        nemesis_like.id = 1;
        nemesis_like.current_hp = 40;
        state.entities.monsters = vec![nemesis_like];
        crate::content::powers::store::set_powers_for(
            &mut state,
            1,
            vec![Power {
                power_type: PowerId::Intangible,
                instance_id: None,
                amount: 1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::FirePotion,
            1,
        ))];

        handle_use_potion(0, Some(1), &mut state);

        let Some(Action::Damage(info)) = state.pop_next_action() else {
            panic!("Fire Potion should queue one DamageAction");
        };
        assert_eq!(info.base, 20);
        assert_eq!(
            info.output, 1,
            "Java FirePotion.use calls DamageInfo.applyEnemyPowersOnly(target), so target IntangiblePower caps the queued THORNS damage before DamageAction runs"
        );
        assert!(info.is_modified);
        assert_eq!(state.entities.potions[0], None);
    }

    #[test]
    fn blood_potion_queues_fixed_use_time_heal_amount_without_minimum_one() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.player.max_hp = 1;
        state.entities.player.current_hp = 1;
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::BloodPotion,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        let Some(Action::Heal { target, amount }) = state.pop_next_action() else {
            panic!("Blood Potion should queue a fixed HealAction");
        };
        assert_eq!(target, 0);
        assert_eq!(
            amount, 0,
            "Java BloodPotion computes (int)(maxHealth * potencyPercent) directly and does not apply Fairy Potion's minimum-one revive rule"
        );
        assert_eq!(state.entities.potions[0], None);
    }

    #[test]
    fn blood_potion_heal_amount_is_computed_when_used_not_when_heal_executes() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.player.max_hp = 10;
        state.entities.player.current_hp = 1;
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::BloodPotion,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        state.entities.player.max_hp = 100;
        let Some(action) = state.pop_next_action() else {
            panic!("Blood Potion should queue a HealAction");
        };
        crate::engine::action_handlers::execute_action(action, &mut state);

        assert_eq!(
            state.entities.player.current_hp, 3,
            "Java BloodPotion.use computes the HealAction amount before it is queued; later max HP changes do not recalculate the potion heal"
        );
    }

    #[test]
    fn entropic_brew_generates_concrete_limited_potions_before_obtain_actions() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.potions = vec![
            Some(crate::content::potions::Potion::new(
                PotionId::EntropicBrew,
                1,
            )),
            Some(crate::content::potions::Potion::new(
                PotionId::FirePotion,
                2,
            )),
            None,
        ];
        let potion_rng_before = state.rng.potion_rng.counter;

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_none());
        assert_eq!(
            state.action_queue_len(),
            3,
            "Java Entropic Brew queues one ObtainPotionAction per potion slot"
        );
        assert!(
            state.rng.potion_rng.counter >= potion_rng_before + 9,
            "Java Entropic Brew calls returnRandomPotion(true) once per potion slot while the potion is used; each call consumes one rarity roll, discards one initial flat potion roll, then consumes at least one accepted/rejected flat roll"
        );
        while let Some(action) = state.pop_next_action() {
            crate::engine::action_handlers::execute_action(action, &mut state);
        }

        let filled = state
            .entities
            .potions
            .iter()
            .filter(|slot| slot.is_some())
            .count();
        assert_eq!(
            filled, 3,
            "after Entropic Brew is destroyed, queued concrete potion obtains fill the newly empty slot and existing empty slots"
        );
        assert!(
            state
                .entities
                .potions
                .iter()
                .flatten()
                .all(|potion| potion.id != PotionId::FruitJuice),
            "Java returnRandomPotion(true) excludes Fruit Juice for Entropic Brew"
        );
    }

    #[test]
    fn distilled_chaos_rolls_random_targets_when_potion_is_used() {
        let mut state = blank_test_combat();
        let mut first = test_monster(EnemyId::JawWorm);
        first.id = 11;
        let mut second = test_monster(EnemyId::Cultist);
        second.id = 12;
        state.entities.monsters = vec![first, second];
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::DistilledChaosPotion,
            1,
        ))];
        let card_random_before = state.rng.card_random_rng.counter;

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_none());
        assert_eq!(state.action_queue_len(), 3);
        assert!(
            state.rng.card_random_rng.counter >= card_random_before + 3,
            "Java DistilledChaosPotion calls getRandomMonster once per PlayTopCardAction while the potion is used"
        );
        for _ in 0..3 {
            let Some(Action::PlayTopCard {
                target: Some(target),
                exhaust: false,
            }) = state.pop_next_action()
            else {
                panic!("Distilled Chaos should queue targeted PlayTopCard actions");
            };
            assert!(
                target == 11 || target == 12,
                "queued Java target should be one of the use-time random monster choices"
            );
        }
    }

    #[test]
    fn essence_of_darkness_channels_for_each_orb_slot_and_sacred_bark_potency() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.player.max_orbs = 3;
        state.entities.player.orbs = vec![
            crate::runtime::combat::OrbEntity::new(crate::runtime::combat::OrbId::Empty),
            crate::runtime::combat::OrbEntity::new(crate::runtime::combat::OrbId::Lightning),
            crate::runtime::combat::OrbEntity::new(crate::runtime::combat::OrbId::Empty),
        ];
        state
            .entities
            .player
            .relics
            .push(RelicState::new(RelicId::SacredBark));
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::EssenceOfDarkness,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_none());
        assert_eq!(
            state.action_queue_len(),
            6,
            "Java EssenceOfDarknessAction channels potency Dark orbs for each orb slot"
        );
        while let Some(action) = state.pop_next_action() {
            assert_eq!(
                action,
                Action::ChannelOrb(crate::runtime::combat::OrbId::Dark)
            );
        }
    }

    #[test]
    fn smoke_bomb_is_blocked_by_spire_shield_back_attack_power() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::SpireShield);
        monster.id = 7;
        state.entities.monsters = vec![monster];
        state.entities.power_db.insert(
            7,
            vec![Power {
                power_type: PowerId::BackAttack,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::SmokeBomb,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_some());
        assert_eq!(
            state.action_queue_len(),
            0,
            "Java SmokeBomb.canUse returns false when any monster has BackAttack"
        );
    }

    #[test]
    fn smoke_bomb_is_blocked_by_boss_monster_type_even_without_room_flag() {
        let mut state = blank_test_combat();
        state.meta.is_boss_fight = false;
        state.entities.monsters = vec![test_monster(EnemyId::SlimeBoss)];
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::SmokeBomb,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_some());
        assert_eq!(
            state.action_queue_len(),
            0,
            "Java SmokeBomb.canUse walks monsters and blocks EnemyType.BOSS, not only room boss flags"
        );
    }

    #[test]
    fn combat_potion_execution_respects_java_can_use_gate() {
        let mut disabled = blank_test_combat();
        disabled.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        disabled.entities.potions = vec![Some(
            crate::content::potions::Potion::with_affordance_truth(
                PotionId::FirePotion,
                1,
                false,
                true,
                true,
            ),
        )];
        handle_use_potion(0, Some(disabled.entities.monsters[0].id), &mut disabled);
        assert!(disabled.entities.potions[0].is_some());
        assert_eq!(
            disabled.action_queue_len(),
            0,
            "Java PotionPopUp checks potion.canUse before calling use()"
        );

        let mut dead_monsters = blank_test_combat();
        dead_monsters.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        dead_monsters.entities.monsters[0].current_hp = 0;
        dead_monsters.entities.monsters[0].is_dying = true;
        dead_monsters.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::FirePotion,
            2,
        ))];
        handle_use_potion(
            0,
            Some(dead_monsters.entities.monsters[0].id),
            &mut dead_monsters,
        );
        assert!(dead_monsters.entities.potions[0].is_some());
        assert_eq!(
            dead_monsters.action_queue_len(),
            0,
            "Java AbstractPotion.canUse blocks when the room monsters are basically dead"
        );
    }

    #[test]
    fn liquid_memories_auto_move_does_not_drop_cards_when_hand_fills() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::LiquidMemories,
            1,
        ))];
        state
            .entities
            .player
            .relics
            .push(RelicState::new(RelicId::SacredBark));
        state.zones.hand = (0..9)
            .map(|idx| CombatCard::new(CardId::Defend, 100 + idx))
            .collect();
        state.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 201),
            CombatCard::new(CardId::Bash, 202),
        ];

        handle_use_potion(0, None, &mut state);

        assert_eq!(state.zones.hand.len(), 10);
        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(
            state.zones.discard_pile[0].id,
            CardId::Bash,
            "Java BetterDiscardPileToHandAction leaves remaining discard cards in place once hand is full"
        );
        assert_eq!(state.zones.hand[9].id, CardId::Strike);
        assert_eq!(state.zones.hand[9].cost_for_turn_java(), 0);
    }

    #[test]
    fn liquid_memories_sacred_bark_grid_select_requires_exact_potency() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::LiquidMemories,
            1,
        ))];
        state
            .entities
            .player
            .relics
            .push(RelicState::new(RelicId::SacredBark));
        state.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 201),
            CombatCard::new(CardId::Bash, 202),
            CombatCard::new(CardId::Defend, 203),
        ];

        handle_use_potion(0, None, &mut state);

        let Some(Action::SuspendForGridSelect {
            source_pile,
            min,
            max,
            can_cancel,
            reason,
            ..
        }) = state.pop_next_action()
        else {
            panic!("Liquid Memories should queue a discard grid select when discard has more cards than potency");
        };
        assert_eq!(source_pile, crate::state::PileType::Discard);
        assert_eq!(min, 2);
        assert_eq!(max, 2);
        assert!(!can_cancel);
        assert_eq!(reason, crate::state::GridSelectReason::DiscardToHand);
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
                payload: crate::runtime::combat::PowerPayload::None,
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
                payload: crate::runtime::combat::PowerPayload::None,
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
                payload: crate::runtime::combat::PowerPayload::None,
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
                payload: crate::runtime::combat::PowerPayload::None,
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
    fn make_temp_card_in_draw_pile_large_amount_uses_java_src_card_path() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_make_temp_card_in_draw_pile(CardId::SearingBlow, 6, true, false, false, &mut state);

        assert_eq!(state.zones.draw_pile.len(), 6);
        assert!(state
            .zones
            .draw_pile
            .iter()
            .all(|card| card.id == CardId::SearingBlow && card.upgrades == 1));
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
                payload: crate::runtime::combat::PowerPayload::None,
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
    fn constructed_make_copy_in_hand_separates_constructor_and_effect_reality_calls() {
        let mut hand_state = blank_test_combat();
        hand_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        let mut constructed = CombatCard::new(CardId::SearingBlow, 200);
        crate::content::cards::apply_master_reality_to_generated_card(
            &mut constructed,
            &hand_state,
            1,
        );
        handle_make_constructed_copy_in_hand(Box::new(constructed.clone()), 1, &mut hand_state);
        assert_eq!(
            hand_state.zones.hand[0].upgrades, 2,
            "hand path gets Java constructor and ShowCardAndAddToHandEffect Master Reality calls"
        );

        let mut delayed_state = blank_test_combat();
        delayed_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        let mut delayed_constructed = CombatCard::new(CardId::SearingBlow, 201);
        crate::content::cards::apply_master_reality_to_generated_card(
            &mut delayed_constructed,
            &delayed_state,
            1,
        );
        store::set_powers_for(&mut delayed_state, 0, vec![]);
        handle_make_constructed_copy_in_hand(Box::new(delayed_constructed), 1, &mut delayed_state);
        assert_eq!(
            delayed_state.zones.hand[0].upgrades, 1,
            "constructor-time Master Reality persists even if the power is gone when the queued action executes"
        );

        let mut overflow_state = blank_test_combat();
        overflow_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        for uuid in 1..=10 {
            overflow_state
                .zones
                .hand
                .push(CombatCard::new(CardId::Strike, uuid));
        }
        let mut overflow_constructed = CombatCard::new(CardId::SearingBlow, 202);
        crate::content::cards::apply_master_reality_to_generated_card(
            &mut overflow_constructed,
            &overflow_state,
            1,
        );
        handle_make_constructed_copy_in_hand(
            Box::new(overflow_constructed),
            1,
            &mut overflow_state,
        );
        assert_eq!(
            overflow_state.zones.discard_pile[0].upgrades, 1,
            "Java overflow discard receives the constructor-upgraded srcCard, not the visually upgraded discard-effect copy"
        );
    }

    #[test]
    fn stasis_return_preserves_same_uuid_and_java_master_reality_counts() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state
            .zones
            .limbo
            .push(CombatCard::new(CardId::SearingBlow, 77));

        handle_return_stasis_card(77, true, &mut state);

        assert!(state.zones.limbo.is_empty());
        assert_eq!(state.zones.hand.len(), 1);
        assert_eq!(state.zones.hand[0].uuid, 77);
        assert_eq!(
            state.zones.hand[0].upgrades, 2,
            "Java Stasis hand path uses MakeTempCardInHandAction(card, false, true), so sameUUID still receives constructor + hand-effect Master Reality calls"
        );

        let mut overflow_state = blank_test_combat();
        overflow_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        for uuid in 1..=10 {
            overflow_state
                .zones
                .hand
                .push(CombatCard::new(CardId::Strike, uuid));
        }
        overflow_state
            .zones
            .limbo
            .push(CombatCard::new(CardId::SearingBlow, 88));

        handle_return_stasis_card(88, false, &mut overflow_state);

        assert!(overflow_state.zones.limbo.is_empty());
        assert_eq!(overflow_state.zones.discard_pile.len(), 1);
        assert_eq!(overflow_state.zones.discard_pile[0].uuid, 88);
        assert_eq!(
            overflow_state.zones.discard_pile[0].upgrades, 0,
            "Java Stasis full-hand path uses MakeTempCardInDiscardAction(card, true), whose sameUUID constructor skips Master Reality"
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
        assert_eq!(
            state.zones.draw_pile[0].uuid, 31,
            "Java creates the draw-pile copy before the discard copy"
        );
        assert_eq!(state.zones.discard_pile[0].uuid, 32);
        assert_ne!(
            state.zones.discard_pile[0].uuid, state.zones.draw_pile[0].uuid,
            "Java MakeTempCardInDiscardAndDeckAction uses separate stat-equivalent copies"
        );
    }

    #[test]
    fn burn_increase_upgrades_only_draw_and_discard_like_java() {
        let mut state = blank_test_combat();
        state.zones.hand = vec![CombatCard::new(CardId::Burn, 1)];
        state.zones.draw_pile = vec![CombatCard::new(CardId::Burn, 2)];
        state.zones.discard_pile = vec![CombatCard::new(CardId::Burn, 3)];
        state.zones.exhaust_pile = vec![CombatCard::new(CardId::Burn, 4)];

        handle_upgrade_all_burns(&mut state);

        assert_eq!(
            state.zones.hand[0].upgrades, 0,
            "Java BurnIncreaseAction does not iterate the hand"
        );
        assert_eq!(state.zones.draw_pile[0].upgrades, 1);
        assert_eq!(state.zones.discard_pile[0].upgrades, 1);
        assert_eq!(
            state.zones.exhaust_pile[0].upgrades, 0,
            "Java BurnIncreaseAction does not iterate the exhaust pile"
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
        let generated_def = crate::content::cards::get_card_definition(generated.id);
        if generated_def.cost >= 0 {
            assert_eq!(generated.cost_for_turn, Some(0));
        } else {
            assert_eq!(
                generated.cost_for_turn, None,
                "Java setCostForTurn(0) does not make unplayable cards playable"
            );
        }
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

        handle_use_card_done(false, true, &mut discarded);

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

        handle_use_card_done(true, true, &mut exhausted);

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

            handle_use_card_done(true, true, &mut state);

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

        handle_use_card_done(true, true, &mut no_spoon);

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

        handle_use_card_done(false, true, &mut not_exhausting);

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
                should_exhaust: false,
                trigger_after_use_hooks: false,
            }),
        ));
        assert!(
            state.zones.limbo.iter().any(|card| card.uuid == 711),
            "Java failed autoplay canUse path still routes the card through UseCardAction"
        );
    }

    #[test]
    fn early_end_turn_clears_card_queue_and_only_cleans_up_autoplay_cards_like_java() {
        let mut state = blank_test_combat();
        crate::content::powers::store::set_powers_for(
            &mut state,
            0,
            vec![Power {
                power_type: PowerId::Rebound,
                instance_id: None,
                amount: 1,
                extra_data: 1,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state.zones.queued_cards = std::collections::VecDeque::from([
            QueuedCardPlay {
                card: CombatCard::new(CardId::Strike, 712),
                target: Some(7),
                energy_on_use: 0,
                ignore_energy_total: true,
                autoplay: false,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: QueuedCardSource::Normal,
            },
            QueuedCardPlay {
                card: CombatCard::new(CardId::Defend, 713),
                target: None,
                energy_on_use: 0,
                ignore_energy_total: true,
                autoplay: true,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: QueuedCardSource::Normal,
            },
        ]);

        handle_queue_early_end_turn(&mut state);

        assert!(state.zones.queued_cards.is_empty());
        assert_eq!(
            state
                .zones
                .limbo
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![713]
        );
        let cleanup = state
            .pop_next_action()
            .expect("Java early-end sequence adds UseCardAction only for autoplay queued cards");
        assert_eq!(
            cleanup,
            Action::UseCardDone {
                should_exhaust: false,
                trigger_after_use_hooks: false,
            }
        );

        crate::engine::action_handlers::execute_action(cleanup, &mut state);
        assert_eq!(
            state
                .zones
                .discard_pile
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![713],
            "dontTriggerOnUseCard cleanup must skip Rebound-style after-use hooks"
        );
        assert!(state.zones.draw_pile.is_empty());
        assert!(state.zones.hand.iter().all(|card| card.uuid != 712));
        assert!(state.zones.discard_pile.iter().all(|card| card.uuid != 712));
        assert!(state.zones.exhaust_pile.iter().all(|card| card.uuid != 712));
    }

    #[test]
    fn upgrade_all_in_hand_matches_armaments_plus_can_upgrade_filter() {
        let mut state = blank_test_combat();
        let mut upgraded_defend = CombatCard::new(CardId::Defend, 801);
        upgraded_defend.upgrades = 1;
        let mut searing = CombatCard::new(CardId::SearingBlow, 804);
        searing.upgrades = 2;
        state.zones.hand = vec![
            CombatCard::new(CardId::Strike, 800),
            upgraded_defend,
            CombatCard::new(CardId::Wound, 802),
            CombatCard::new(CardId::Injury, 803),
            searing,
        ];

        handle_upgrade_all_in_hand(&mut state);

        assert_eq!(state.zones.hand[0].upgrades, 1);
        assert_eq!(
            state.zones.hand[1].upgrades, 1,
            "Java canUpgrade() rejects already-upgraded normal cards"
        );
        assert_eq!(
            state.zones.hand[2].upgrades, 0,
            "Java canUpgrade() rejects Status cards"
        );
        assert_eq!(
            state.zones.hand[3].upgrades, 0,
            "Java canUpgrade() rejects Curse cards"
        );
        assert_eq!(
            state.zones.hand[4].upgrades, 3,
            "Searing Blow remains repeatedly upgradeable through its override"
        );
    }

    #[test]
    fn upgrade_all_cards_in_combat_matches_apotheosis_groups() {
        let mut state = blank_test_combat();
        state.zones.hand = vec![
            CombatCard::new(CardId::Strike, 810),
            CombatCard::new(CardId::Wound, 811),
        ];
        state.zones.draw_pile = vec![CombatCard::new(CardId::Defend, 812)];
        state.zones.discard_pile = vec![CombatCard::new(CardId::Bash, 813)];
        state.zones.exhaust_pile = vec![CombatCard::new(CardId::ShrugItOff, 814)];
        state.zones.limbo = vec![CombatCard::new(CardId::Strike, 815)];

        handle_upgrade_all_cards_in_combat(&mut state);

        assert_eq!(state.zones.hand[0].upgrades, 1);
        assert_eq!(state.zones.hand[1].upgrades, 0);
        assert_eq!(state.zones.draw_pile[0].upgrades, 1);
        assert_eq!(state.zones.discard_pile[0].upgrades, 1);
        assert_eq!(state.zones.exhaust_pile[0].upgrades, 1);
        assert_eq!(
            state.zones.limbo[0].upgrades, 0,
            "Java ApotheosisAction upgrades hand/draw/discard/exhaust, not limbo/cardInUse"
        );
    }

    #[test]
    fn end_turn_ethereal_exhaust_respects_explicit_retain_like_java_discard_at_end() {
        let mut state = blank_test_combat();
        let mut retained_ethereal = CombatCard::new(CardId::GhostlyArmor, 830);
        retained_ethereal.retain_override = Some(true);
        state.zones.hand = vec![
            retained_ethereal,
            CombatCard::new(CardId::Carnage, 831),
            CombatCard::new(CardId::Strike, 832),
        ];

        handle_end_turn_trigger(&mut state);
        let queued: Vec<_> = std::iter::from_fn(|| state.pop_next_action()).collect();

        assert!(
            !queued.iter().any(|action| matches!(
                action,
                Action::ExhaustCard {
                    card_uuid: 830,
                    source_pile: crate::state::PileType::Hand
                }
            )),
            "Java removes retained/selfRetain cards from hand before triggerOnEndOfPlayerTurn, so explicit retain prevents ethereal exhaust"
        );
        assert!(
            queued.iter().any(|action| matches!(
                action,
                Action::ExhaustCard {
                    card_uuid: 831,
                    source_pile: crate::state::PileType::Hand
                }
            )),
            "non-retained ethereal cards still exhaust at end of turn"
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
                crate::content::monsters::PreBattleLegacyRng::MonsterHp,
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

    // Java AbstractRoom.update() constructs the whole opening queue before
    // actionManager drains it:
    //   atBattleStartPreDraw hooks
    //   DrawCardAction
    //   atBattleStart hooks
    //   atTurnStart relics
    //   atTurnStartPostDraw relics
    //   card / power / orb atTurnStart hooks
    //
    // Therefore these hook methods must run synchronously here. Queuing a later
    // synthetic BattleStartTrigger would incorrectly let the initial draw execute
    // before atBattleStart / atTurnStart hooks have had a chance to enqueue.
    let draw_amount = crate::engine::core::compute_player_turn_start_draw_count(state);
    if draw_amount > 0 {
        state.queue_action_back(crate::runtime::action::Action::DrawCards(
            draw_amount as u32,
        ));
    }

    queue_initial_battle_start_hooks_after_draw_is_queued(state);
}

pub fn handle_battle_start_trigger(state: &mut CombatState) {
    // Relic battle-start hooks (e.g. Akabeko, Marbles)
    let battle_start_actions = crate::content::relics::hooks::at_battle_start(state);
    state.queue_actions(battle_start_actions);
}

fn queue_initial_battle_start_hooks_after_draw_is_queued(state: &mut CombatState) {
    let battle_start_actions = crate::content::relics::hooks::at_battle_start(state);
    state.queue_actions(battle_start_actions);

    // Java AbstractPlayer.applyStartOfTurnRelics() calls stance.atStartOfTurn()
    // before relic atTurnStart hooks. Divinity queues a return to Neutral here.
    if state.entities.player.stance == crate::runtime::combat::StanceId::Divinity {
        state.queue_action_back(crate::runtime::action::Action::EnterStance(
            "Neutral".to_string(),
        ));
    }

    let turn_start_actions = crate::content::relics::hooks::at_turn_start(state);
    state.queue_actions(turn_start_actions);

    // Initial combat is special: AbstractRoom.update() calls only relic
    // atTurnStartPostDraw here, not power atStartOfTurnPostDraw.
    let post_draw_relic_actions = crate::content::relics::hooks::at_turn_start_post_draw(state);
    state.queue_actions(post_draw_relic_actions);

    let card_actions = crate::content::cards::hooks::at_turn_start_in_hand(state);
    state.queue_actions(card_actions);

    for power in &crate::content::powers::store::powers_snapshot_for(state, 0) {
        let power_actions =
            crate::content::powers::resolve_power_instance_at_turn_start(power, state, 0);
        for action in power_actions {
            state.queue_action_back(action);
        }
    }

    let orb_actions = crate::content::orbs::hooks::at_turn_start(state);
    state.queue_actions(orb_actions);
}
