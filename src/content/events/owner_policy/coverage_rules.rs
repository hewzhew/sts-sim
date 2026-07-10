use crate::state::events::EventActionKind;
use crate::state::run::RunState;

use super::{action, event_screen, option_index, EventOwnerOptionSelector};

use crate::ai::deck_mutation_compiler_v1::best_duplicate_target_for_shop_v1;
use crate::content::cards::CardId;
use crate::state::events::{EventCardKind, EventEffect};

use super::{effect, has_omamori_charge, has_safe_purge_target};

pub(super) fn gremlin_wheel_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Special),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn lab_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Gain),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn colosseum_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 => action(EventActionKind::Fight),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn the_joust_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 | 2 | 3 => action(EventActionKind::Continue),
        1 => option_index(0),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn golden_shrine_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if has_omamori_charge(run_state) {
        return effect(EventEffect::ObtainCurse {
            count: 1,
            kind: EventCardKind::Specific(CardId::Regret),
        });
    }
    let gold = if run_state.ascension_level >= 15 { 50 } else { 100 };
    effect(EventEffect::GainGold(gold))
}

pub(super) fn fountain_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if crate::content::events::fountain::removable_curse_count(run_state) > 0 => {
            action(EventActionKind::DeckOperation)
        }
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn upgrade_shrine_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if run_state
            .master_deck
            .iter()
            .any(crate::state::core::master_deck_card_can_upgrade) =>
        {
            action(EventActionKind::DeckOperation)
        }
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn accursed_blacksmith_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    if event_screen(run_state) != 0 {
        return action(EventActionKind::Leave);
    }
    if run_state
        .master_deck
        .iter()
        .any(crate::state::core::master_deck_card_can_upgrade)
    {
        action(EventActionKind::DeckOperation)
    } else if has_omamori_charge(run_state) {
        action(EventActionKind::Trade)
    } else {
        action(EventActionKind::Leave)
    }
}

pub(super) fn duplicator_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 if best_duplicate_target_for_shop_v1(run_state).is_some() => {
            action(EventActionKind::DeckOperation)
        }
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn note_for_yourself_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 if !crate::content::events::note_for_yourself::default_note_is_ignorable(run_state)
            && has_safe_purge_target(run_state) =>
        {
            action(EventActionKind::DeckOperation)
        }
        1 => action(EventActionKind::Decline),
        _ => action(EventActionKind::Leave),
    }
}
