use crate::runtime::combat::{CardPileView, CombatState};
use crate::state::core::{PendingChoice, PileType};

use super::stable::stable_card_signature;
use super::types::{StableChoiceCandidateKey, StablePendingChoiceKey};

pub(super) fn pending_choice_key(
    choice: &PendingChoice,
    combat: &CombatState,
) -> StablePendingChoiceKey {
    match choice {
        PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => StablePendingChoiceKey::GridSelect {
            source_pile: pile_label(*source_pile),
            min_cards: *min_cards,
            max_cards: *max_cards,
            can_cancel: *can_cancel,
            reason: format!("{reason:?}"),
            candidates: candidate_cards_key(
                combat,
                candidate_uuids,
                pile_cards(combat, *source_pile),
                matches!(
                    source_pile,
                    PileType::Draw | PileType::Discard | PileType::Exhaust
                ),
                matches!(
                    source_pile,
                    PileType::MasterDeck | PileType::Draw | PileType::Discard | PileType::Exhaust
                ),
                if matches!(source_pile, PileType::MasterDeck) {
                    "master_ref"
                } else {
                    "unknown_ref"
                },
            ),
        },
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => StablePendingChoiceKey::HandSelect {
            min_cards: *min_cards,
            max_cards: *max_cards,
            can_cancel: *can_cancel,
            reason: format!("{reason:?}"),
            candidates: candidate_cards_key(
                combat,
                candidate_uuids,
                Some(CardPileView::Contiguous(&combat.zones.hand)),
                false,
                true,
                "unknown_ref",
            ),
        },
        PendingChoice::DiscoverySelect(choice) => {
            StablePendingChoiceKey::Discovery(sorted_debug_values(&choice.cards))
        }
        PendingChoice::ScrySelect { card_uuids, .. } => {
            StablePendingChoiceKey::Scry(scry_candidates_key(combat, card_uuids))
        }
        PendingChoice::CardRewardSelect {
            cards,
            destination,
            can_skip,
        } => StablePendingChoiceKey::CardRewardSelect {
            destination: format!("{destination:?}"),
            can_skip: *can_skip,
            cards: sorted_debug_values(cards),
        },
        PendingChoice::ForeignInfluenceSelect { cards, upgraded } => {
            StablePendingChoiceKey::ForeignInfluence {
                upgraded: *upgraded,
                cards: sorted_debug_values(cards),
            }
        }
        PendingChoice::ChooseOneSelect { choices } => {
            StablePendingChoiceKey::ChooseOne(sorted_debug_values(choices))
        }
        PendingChoice::StanceChoice => StablePendingChoiceKey::StanceChoice,
    }
}

fn pile_cards(combat: &CombatState, pile: PileType) -> Option<CardPileView<'_>> {
    match pile {
        PileType::Draw => Some(CardPileView::Contiguous(combat.zones.draw_pile.as_ref())),
        PileType::Discard => Some(CardPileView::Discard(&combat.zones.discard_pile)),
        PileType::Exhaust => Some(CardPileView::Contiguous(
            combat.zones.exhaust_pile.as_slice(),
        )),
        PileType::Hand => Some(CardPileView::Contiguous(&combat.zones.hand)),
        PileType::Limbo => Some(CardPileView::Contiguous(&combat.zones.limbo)),
        PileType::MasterDeck => None,
    }
}

fn candidate_cards_key(
    combat: &CombatState,
    candidate_uuids: &[u32],
    cards: Option<CardPileView<'_>>,
    allow_visible_fallback: bool,
    normalize_order: bool,
    unresolved_prefix: &'static str,
) -> Vec<StableChoiceCandidateKey> {
    let mut candidates = candidate_uuids
        .iter()
        .map(|uuid| {
            stable_candidate_card_key(combat, cards, *uuid, allow_visible_fallback).unwrap_or(
                StableChoiceCandidateKey::Ref {
                    prefix: unresolved_prefix,
                    uuid: *uuid,
                },
            )
        })
        .collect::<Vec<_>>();
    if normalize_order {
        candidates.sort();
    }
    candidates
}

fn stable_candidate_card_key(
    combat: &CombatState,
    cards: Option<CardPileView<'_>>,
    uuid: u32,
    allow_visible_fallback: bool,
) -> Option<StableChoiceCandidateKey> {
    cards
        .and_then(|cards| cards.iter().find(|card| card.uuid == uuid))
        .or_else(|| {
            allow_visible_fallback.then(|| {
                visible_card_zones(combat)
                    .iter()
                    .find_map(|zone| zone.iter().find(|card| card.uuid == uuid))
            })?
        })
        .map(|card| StableChoiceCandidateKey::Card(stable_card_signature(card)))
}

fn visible_card_zones(combat: &CombatState) -> [CardPileView<'_>; 5] {
    [
        CardPileView::Contiguous(&combat.zones.hand),
        CardPileView::Contiguous(combat.zones.draw_pile.as_ref()),
        CardPileView::Discard(&combat.zones.discard_pile),
        CardPileView::Contiguous(combat.zones.exhaust_pile.as_slice()),
        CardPileView::Contiguous(&combat.zones.limbo),
    ]
}

fn scry_candidates_key(combat: &CombatState, card_uuids: &[u32]) -> Vec<StableChoiceCandidateKey> {
    card_uuids
        .iter()
        .map(|uuid| {
            stable_candidate_card_key(
                combat,
                Some(CardPileView::Contiguous(combat.zones.draw_pile.as_ref())),
                *uuid,
                false,
            )
            .unwrap_or(StableChoiceCandidateKey::Ref {
                prefix: "scry_ref",
                uuid: *uuid,
            })
        })
        .collect()
}

fn pile_label(pile: PileType) -> &'static str {
    match pile {
        PileType::Draw => "Draw",
        PileType::Discard => "Discard",
        PileType::Exhaust => "Exhaust",
        PileType::Hand => "Hand",
        PileType::Limbo => "Limbo",
        PileType::MasterDeck => "MasterDeck",
    }
}

fn debug_values<T: std::fmt::Debug>(values: &[T]) -> Vec<String> {
    values.iter().map(|value| format!("{value:?}")).collect()
}

fn sorted_debug_values<T: std::fmt::Debug>(values: &[T]) -> Vec<String> {
    let mut values = debug_values(values);
    values.sort();
    values
}
