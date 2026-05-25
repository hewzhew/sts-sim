use super::*;
#[cfg(test)]
use crate::content::cards::CardId;
use crate::runtime::combat::CombatCard;
#[cfg(test)]
use crate::state::core::{GridSelectReason, HandSelectReason};
use crate::state::core::{PendingChoice, PileType};

mod card_selection;
mod selection_hints;
mod types;

use selection_hints::{
    removal_selection_hint_from_card_ids, selection_hint_for_grid_reason,
    selection_hint_for_hand_reason, value_selection_hint_from_card_id,
};
pub(super) use types::{PendingChoiceOrderingHint, PendingChoiceOrderingRole};

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
