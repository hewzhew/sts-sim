use super::*;
use crate::content::cards::CardId;
use crate::runtime::combat::CombatCard;
use crate::state::core::{GridSelectReason, HandSelectReason, PendingChoice, PileType};

mod card_selection;
use card_selection::{aggregate_card_facts, CardSelectionFacts};

const RECYCLE_ENERGY_FACTOR: i32 = 10;
const SETUP_EXPENSIVE_CARD_BONUS: i32 = 25;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PendingChoiceOrderingRole {
    ValueSelection,
    RemovalSelection,
    NeutralSelection,
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PendingChoiceOrderingHint {
    pub(super) primary: i32,
    pub(super) secondary: i32,
    pub(super) selected_count_tiebreak: i32,
    pub(super) role: PendingChoiceOrderingRole,
}

pub(super) fn pending_choice_ordering_hint(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> Option<PendingChoiceOrderingHint> {
    let EngineState::PendingChoice(choice) = engine else {
        return None;
    };

    match (choice, input) {
        (_, ClientInput::Cancel) => Some(PendingChoiceOrderingHint {
            role: PendingChoiceOrderingRole::Cancel,
            primary: 0,
            secondary: 0,
            selected_count_tiebreak: 0,
        }),
        (
            PendingChoice::HandSelect {
                candidate_uuids,
                reason,
                ..
            },
            ClientInput::SubmitHandSelect(uuids),
        ) if selection_is_subset(uuids, candidate_uuids) => {
            let cards = uuids
                .iter()
                .filter_map(|uuid| find_card_by_uuid(&combat.zones.hand, *uuid))
                .collect::<Vec<_>>();
            Some(selection_hint_for_hand_reason(*reason, &cards, uuids.len()))
        }
        (
            PendingChoice::GridSelect {
                source_pile,
                candidate_uuids,
                reason,
                ..
            },
            ClientInput::SubmitGridSelect(uuids),
        ) if selection_is_subset(uuids, candidate_uuids) => {
            let cards = uuids
                .iter()
                .filter_map(|uuid| find_card_by_uuid(pile_cards(combat, *source_pile), *uuid))
                .collect::<Vec<_>>();
            Some(selection_hint_for_grid_reason(*reason, &cards, uuids.len()))
        }
        (PendingChoice::ScrySelect { cards, .. }, ClientInput::SubmitScryDiscard(indices)) => {
            let selected_cards = indices
                .iter()
                .filter_map(|idx| cards.get(*idx).copied())
                .collect::<Vec<_>>();
            Some(removal_selection_hint_from_card_ids(
                &selected_cards,
                indices.len(),
            ))
        }
        (PendingChoice::DiscoverySelect(choice), ClientInput::SubmitDiscoverChoice(idx))
            if *idx < choice.cards.len() =>
        {
            Some(value_selection_hint_from_card_id(choice.cards[*idx], 1))
        }
        (PendingChoice::CardRewardSelect { cards, .. }, ClientInput::SubmitDiscoverChoice(idx))
            if *idx < cards.len() =>
        {
            Some(value_selection_hint_from_card_id(cards[*idx], 1))
        }
        (
            PendingChoice::ForeignInfluenceSelect { cards, .. },
            ClientInput::SubmitDiscoverChoice(idx),
        ) if *idx < cards.len() => Some(value_selection_hint_from_card_id(cards[*idx], 1)),
        (PendingChoice::ChooseOneSelect { choices }, ClientInput::SubmitDiscoverChoice(idx))
            if *idx < choices.len() =>
        {
            Some(value_selection_hint_from_card_id(choices[*idx].card_id, 1))
        }
        (PendingChoice::StanceChoice, ClientInput::SubmitDiscoverChoice(idx)) if *idx <= 1 => {
            Some(PendingChoiceOrderingHint {
                role: PendingChoiceOrderingRole::NeutralSelection,
                primary: -(*idx as i32),
                secondary: 0,
                selected_count_tiebreak: -1,
            })
        }
        _ => None,
    }
}

fn selection_hint_for_hand_reason(
    reason: HandSelectReason,
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    match reason {
        HandSelectReason::Discard | HandSelectReason::Exhaust | HandSelectReason::GamblingChip => {
            removal_selection_hint(cards, selected_count)
        }
        HandSelectReason::Recycle => recycle_selection_hint(cards, selected_count),
        HandSelectReason::Upgrade => upgrade_selection_hint(cards, selected_count),
        HandSelectReason::Copy { amount } | HandSelectReason::Nightmare { amount } => {
            repeated_value_selection_hint(cards, selected_count, amount)
        }
        HandSelectReason::Retain => value_selection_hint(cards, selected_count),
        HandSelectReason::PutOnDrawPile
        | HandSelectReason::PutToBottomOfDraw
        | HandSelectReason::Setup => draw_pile_setup_selection_hint(cards, selected_count),
    }
}

fn selection_hint_for_grid_reason(
    reason: GridSelectReason,
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    match reason {
        GridSelectReason::MoveToDrawPile
        | GridSelectReason::DrawPileToHand
        | GridSelectReason::SkillFromDeckToHand
        | GridSelectReason::AttackFromDeckToHand
        | GridSelectReason::DiscardToHand
        | GridSelectReason::DiscardToHandNoCostChange
        | GridSelectReason::DiscardToHandRetain => value_selection_hint(cards, selected_count),
        GridSelectReason::Exhume { upgrade } => {
            exhume_selection_hint(cards, selected_count, upgrade)
        }
        GridSelectReason::Omniscience { play_amount } => {
            repeated_value_selection_hint(cards, selected_count, play_amount)
        }
    }
}

fn value_selection_hint(cards: &[&CombatCard], selected_count: usize) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::ValueSelection,
        primary: facts.keep_value,
        secondary: -facts.removal_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn value_selection_hint_from_card_id(
    card_id: CardId,
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = CardSelectionFacts::from_card_id(card_id);
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::ValueSelection,
        primary: facts.keep_value,
        secondary: -facts.removal_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn repeated_value_selection_hint(
    cards: &[&CombatCard],
    selected_count: usize,
    repeat_count: u8,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    let repeat_count = i32::from(repeat_count.max(1));
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::ValueSelection,
        primary: facts.keep_value.saturating_mul(repeat_count),
        secondary: facts.upgrade_value.saturating_sub(facts.removal_value),
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn upgrade_selection_hint(
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::ValueSelection,
        primary: facts.upgrade_value,
        secondary: facts.keep_value.saturating_sub(facts.removal_value),
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn exhume_selection_hint(
    cards: &[&CombatCard],
    selected_count: usize,
    upgrade: bool,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    let upgrade_bonus = if upgrade { facts.upgrade_value } else { 0 };
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::ValueSelection,
        primary: facts.keep_value.saturating_add(upgrade_bonus),
        secondary: -facts.removal_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn removal_selection_hint(
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::RemovalSelection,
        primary: facts.removal_value,
        secondary: -facts.keep_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn removal_selection_hint_from_card_ids(
    card_ids: &[CardId],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(
        card_ids
            .iter()
            .copied()
            .map(CardSelectionFacts::from_card_id),
    );
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::RemovalSelection,
        primary: facts.removal_value,
        secondary: -facts.keep_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn recycle_selection_hint(
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    let energy_return = cards
        .iter()
        .map(|card| card.combat_cost_without_turn_override_java().max(0))
        .sum::<i32>();
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::RemovalSelection,
        primary: energy_return
            .saturating_mul(RECYCLE_ENERGY_FACTOR)
            .saturating_add(facts.removal_value),
        secondary: -facts.keep_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn draw_pile_setup_selection_hint(
    cards: &[&CombatCard],
    selected_count: usize,
) -> PendingChoiceOrderingHint {
    let facts = aggregate_card_facts(cards.iter().copied().map(CardSelectionFacts::from_card));
    let currently_expensive = cards
        .iter()
        .filter(|card| card.cost_for_turn_java() > 0)
        .count() as i32;
    PendingChoiceOrderingHint {
        role: PendingChoiceOrderingRole::ValueSelection,
        primary: facts
            .keep_value
            .saturating_add(currently_expensive.saturating_mul(SETUP_EXPENSIVE_CARD_BONUS)),
        secondary: -facts.removal_value,
        selected_count_tiebreak: -(selected_count as i32),
    }
}

fn selection_is_subset(selected: &[u32], candidates: &[u32]) -> bool {
    selected.iter().all(|uuid| candidates.contains(uuid))
}

fn find_card_by_uuid(cards: &[CombatCard], uuid: u32) -> Option<&CombatCard> {
    cards.iter().find(|card| card.uuid == uuid)
}

fn pile_cards(combat: &CombatState, pile: PileType) -> &[CombatCard] {
    match pile {
        PileType::Draw => &combat.zones.draw_pile,
        PileType::Discard => &combat.zones.discard_pile,
        PileType::Exhaust => &combat.zones.exhaust_pile,
        PileType::Hand => &combat.zones.hand,
        PileType::Limbo => &combat.zones.limbo,
        PileType::MasterDeck => &[],
    }
}

#[cfg(test)]
mod tests;
