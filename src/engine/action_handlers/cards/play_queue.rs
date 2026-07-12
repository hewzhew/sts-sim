use super::exhaust::move_card_to_exhaust_pile;
use super::generated::{
    materialize_random_class_card_in_draw_pile_action,
    materialize_random_class_card_in_hand_action, materialize_random_colorless_card_in_hand_action,
};
use crate::content::cards::CardId;
use crate::content::powers::{store, PowerId};
use crate::engine::targeting;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState, QueuedCardPlay, QueuedCardSource};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct UseCardPlacementOverrides {
    rebound: bool,
}

fn apply_use_card_after_use_hooks(
    card: &CombatCard,
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

pub fn handle_use_card_after_use_hooks(mut card: CombatCard, state: &mut CombatState) {
    card.free_to_play_once = false;
    apply_use_card_after_use_hooks(&card, state);
    resolve_early_end_turn_pending_after_card_use(state);
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

    resolve_early_end_turn_pending_after_card_use(state);
}

fn resolve_early_end_turn_pending_after_card_use(state: &mut CombatState) {
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
    mut played_card: CombatCard,
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
        effective_cost.max(0)
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

pub fn handle_enqueue_card_play(item: QueuedCardPlay, in_front: bool, state: &mut CombatState) {
    state.enqueue_card_play(item, in_front);
}

fn queued_card_target_fails_java_can_use(
    card: &CombatCard,
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
    card: &CombatCard,
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
    card: Box<CombatCard>,
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
        item: Box::new(QueuedCardPlay {
            card: *card,
            target: resolved_target,
            energy_on_use: state.turn.energy as i32,
            ignore_energy_total: true,
            autoplay: true,
            random_target: false,
            is_end_turn_autoplay: false,
            purge_on_use: false,
            source: QueuedCardSource::Normal,
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
