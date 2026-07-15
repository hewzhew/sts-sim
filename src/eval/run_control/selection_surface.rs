use crate::content::cards::CardId;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::core::{ClientInput, EngineState, PendingChoice, PileType};
use crate::state::selection::{
    SelectionReason, SelectionResolution, SelectionScope, SelectionTargetRef,
};

use super::session::RunControlSession;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SelectionSurface {
    pub scope: SelectionScope,
    pub reason: SelectionReason,
    pub min_choices: usize,
    pub max_choices: usize,
    pub can_cancel: bool,
    pub item_count: usize,
    pub items: Vec<SelectionSurfaceItem>,
    pub submit_hint: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SelectionSurfaceItem {
    pub visible_index: usize,
    pub location: SelectionItemLocation,
    pub target: SelectionTargetRef,
    pub card: CardId,
    pub upgrades: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectionItemLocation {
    Hand { index: usize },
    Grid { pile: PileType, index: usize },
    Scry { index: usize },
    MasterDeck { index: usize },
}

pub(super) fn active_selection_surface(session: &RunControlSession) -> Option<SelectionSurface> {
    match &session.engine_state {
        EngineState::PendingChoice(choice) => match choice {
            PendingChoice::HandSelect {
                candidate_uuids,
                reason,
                min_cards,
                max_cards,
                can_cancel,
            } => Some(SelectionSurface {
                scope: SelectionScope::Hand,
                reason: (*reason).into(),
                min_choices: *min_cards as usize,
                max_choices: (*max_cards as usize).min(candidate_uuids.len()),
                can_cancel: *can_cancel,
                item_count: candidate_uuids.len(),
                items: selection_items_for_combat_cards(
                    combat_hand_cards(session),
                    candidate_uuids,
                    CombatSelectionLocation::Hand,
                ),
                submit_hint: "select <idx...>",
            }),
            PendingChoice::GridSelect {
                source_pile,
                candidate_uuids,
                reason,
                min_cards,
                max_cards,
                can_cancel,
            } => Some(SelectionSurface {
                scope: SelectionScope::Grid,
                reason: (*reason).into(),
                min_choices: *min_cards as usize,
                max_choices: (*max_cards as usize).min(candidate_uuids.len()),
                can_cancel: *can_cancel,
                item_count: candidate_uuids.len(),
                items: selection_items_for_combat_cards(
                    combat_cards_for_pile(session, *source_pile),
                    candidate_uuids,
                    CombatSelectionLocation::Grid(*source_pile),
                ),
                submit_hint: "select <idx...>",
            }),
            PendingChoice::ScrySelect { cards, card_uuids } => Some(SelectionSurface {
                scope: SelectionScope::Grid,
                reason: SelectionReason::PutToBottomOfDraw,
                min_choices: 0,
                max_choices: cards.len(),
                can_cancel: false,
                item_count: cards.len(),
                items: cards
                    .iter()
                    .zip(card_uuids.iter())
                    .enumerate()
                    .map(|(index, (card, uuid))| SelectionSurfaceItem {
                        visible_index: index,
                        location: SelectionItemLocation::Scry { index },
                        target: SelectionTargetRef::CardUuid(*uuid),
                        card: *card,
                        upgrades: 0,
                    })
                    .collect(),
                submit_hint: "select <idx...>",
            }),
            _ => None,
        },
        EngineState::RunPendingChoice(choice) => {
            let request = choice.selection_request(&session.run_state);
            Some(SelectionSurface {
                scope: request.scope,
                reason: request.reason,
                min_choices: choice.min_choices,
                max_choices: choice.max_choices,
                can_cancel: request.can_cancel,
                item_count: request.targets.len(),
                items: selection_items_for_master_deck_targets(session, &request.targets),
                submit_hint: "select <idx...>",
            })
        }
        _ => None,
    }
}

fn combat_hand_cards(session: &RunControlSession) -> &[CombatCard] {
    session
        .active_combat
        .as_ref()
        .map(|active| active.combat_state.zones.hand.as_slice())
        .unwrap_or(&[])
}

fn combat_cards_for_pile(session: &RunControlSession, pile: PileType) -> &[CombatCard] {
    let Some(combat) = session
        .active_combat
        .as_ref()
        .map(|active| &active.combat_state)
    else {
        return &[];
    };
    grid_source_cards(combat, pile)
}

#[derive(Clone, Copy)]
enum CombatSelectionLocation {
    Hand,
    Grid(PileType),
}

fn selection_items_for_combat_cards(
    cards: &[CombatCard],
    candidate_uuids: &[u32],
    source: CombatSelectionLocation,
) -> Vec<SelectionSurfaceItem> {
    candidate_uuids
        .iter()
        .enumerate()
        .filter_map(|(visible_index, uuid)| {
            let card_index = cards.iter().position(|card| card.uuid == *uuid)?;
            let card = &cards[card_index];
            let location = match source {
                CombatSelectionLocation::Hand => SelectionItemLocation::Hand { index: card_index },
                CombatSelectionLocation::Grid(pile) => SelectionItemLocation::Grid {
                    pile,
                    index: card_index,
                },
            };
            Some(SelectionSurfaceItem {
                visible_index,
                location,
                target: SelectionTargetRef::CardUuid(*uuid),
                card: card.id,
                upgrades: card.upgrades,
            })
        })
        .collect()
}

fn selection_items_for_master_deck_targets(
    session: &RunControlSession,
    targets: &[SelectionTargetRef],
) -> Vec<SelectionSurfaceItem> {
    targets
        .iter()
        .enumerate()
        .filter_map(|(visible_index, target)| match target {
            SelectionTargetRef::CardUuid(uuid) => {
                let deck_index = session
                    .run_state
                    .master_deck
                    .iter()
                    .position(|card| card.uuid == *uuid)?;
                let card = &session.run_state.master_deck[deck_index];
                Some(SelectionSurfaceItem {
                    visible_index,
                    location: SelectionItemLocation::MasterDeck { index: deck_index },
                    target: *target,
                    card: card.id,
                    upgrades: card.upgrades,
                })
            }
        })
        .collect()
}

pub(super) fn current_selection_input_is_allowed(
    session: &RunControlSession,
    input: &ClientInput,
) -> Option<bool> {
    match &session.engine_state {
        EngineState::PendingChoice(choice) => {
            Some(pending_choice_input_is_allowed(session, choice, input))
        }
        EngineState::RunPendingChoice(choice) => match input {
            ClientInput::SubmitSelection(resolution) => Some(run_pending_resolution_is_allowed(
                session, choice, resolution,
            )),
            ClientInput::Cancel => Some(choice.min_choices == 0),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn pending_choice_input_is_allowed(
    session: &RunControlSession,
    choice: &PendingChoice,
    input: &ClientInput,
) -> bool {
    match (choice, input) {
        (
            PendingChoice::HandSelect {
                candidate_uuids,
                min_cards,
                max_cards,
                ..
            },
            ClientInput::SubmitSelection(resolution),
        ) => {
            if resolution.scope != SelectionScope::Hand {
                return false;
            }
            let uuids = resolution.selected_card_uuids();
            uuid_selection_is_allowed(
                &uuids,
                candidate_uuids,
                *min_cards as usize,
                *max_cards as usize,
            ) && hand_contains_all(session, &uuids)
        }
        (
            PendingChoice::GridSelect {
                candidate_uuids,
                min_cards,
                max_cards,
                source_pile,
                ..
            },
            ClientInput::SubmitSelection(resolution),
        ) => {
            if resolution.scope != SelectionScope::Grid {
                return false;
            }
            let uuids = resolution.selected_card_uuids();
            uuid_selection_is_allowed(
                &uuids,
                candidate_uuids,
                *min_cards as usize,
                *max_cards as usize,
            ) && grid_source_contains_all(session, *source_pile, &uuids)
        }
        (
            PendingChoice::HandSelect { can_cancel, .. }
            | PendingChoice::GridSelect { can_cancel, .. },
            ClientInput::Cancel,
        ) => *can_cancel,
        (PendingChoice::ScrySelect { cards, .. }, ClientInput::SubmitScryDiscard(indices)) => {
            validate_indices_in_range(cards.len(), indices).is_ok()
                && reject_duplicate_indices(indices).is_ok()
        }
        (PendingChoice::DiscoverySelect(choice), ClientInput::SubmitDiscoverChoice(idx)) => {
            *idx < choice.cards.len()
        }
        (PendingChoice::DiscoverySelect(choice), ClientInput::Cancel) => choice.can_skip,
        (PendingChoice::CardRewardSelect { cards, .. }, ClientInput::SubmitDiscoverChoice(idx)) => {
            *idx < cards.len()
        }
        (PendingChoice::CardRewardSelect { can_skip, .. }, ClientInput::Cancel) => *can_skip,
        (
            PendingChoice::ForeignInfluenceSelect { cards, .. },
            ClientInput::SubmitDiscoverChoice(idx),
        ) => *idx < cards.len(),
        (PendingChoice::ChooseOneSelect { choices }, ClientInput::SubmitDiscoverChoice(idx)) => {
            *idx < choices.len()
        }
        (PendingChoice::StanceChoice, ClientInput::SubmitDiscoverChoice(idx)) => *idx < 2,
        _ => false,
    }
}

fn run_pending_resolution_is_allowed(
    session: &RunControlSession,
    choice: &crate::state::core::RunPendingChoiceState,
    resolution: &SelectionResolution,
) -> bool {
    if resolution.scope != SelectionScope::Deck {
        return false;
    }
    let indices = resolution
        .selected
        .iter()
        .filter_map(|target| match target {
            SelectionTargetRef::CardUuid(uuid) => session
                .run_state
                .master_deck
                .iter()
                .position(|card| card.uuid == *uuid),
        })
        .collect::<Vec<_>>();
    indices.len() == resolution.selected.len()
        && session.run_pending_selection_is_allowed(choice, &indices)
}

fn validate_indices_in_range(item_count: usize, indices: &[usize]) -> Result<(), String> {
    for idx in indices {
        if *idx >= item_count {
            return Err(format!(
                "selection index {idx} out of range; visible indices are 0..{}",
                item_count.saturating_sub(1)
            ));
        }
    }
    Ok(())
}

fn reject_duplicate_indices(indices: &[usize]) -> Result<(), String> {
    let mut sorted = indices.to_vec();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("selection indices must be unique".to_string());
    }
    Ok(())
}

fn uuid_selection_is_allowed(
    uuids: &[u32],
    candidate_uuids: &[u32],
    min_choices: usize,
    max_choices: usize,
) -> bool {
    uuids.len() >= min_choices
        && uuids.len() <= max_choices
        && all_unique(uuids)
        && uuids.iter().all(|uuid| candidate_uuids.contains(uuid))
}

fn all_unique(values: &[u32]) -> bool {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    !sorted.windows(2).any(|pair| pair[0] == pair[1])
}

fn hand_contains_all(session: &RunControlSession, uuids: &[u32]) -> bool {
    let Some(combat) = session
        .active_combat
        .as_ref()
        .map(|active| &active.combat_state)
    else {
        return false;
    };
    pile_contains_all(&combat.zones.hand, uuids)
}

fn grid_source_contains_all(
    session: &RunControlSession,
    source_pile: PileType,
    uuids: &[u32],
) -> bool {
    let Some(combat) = session
        .active_combat
        .as_ref()
        .map(|active| &active.combat_state)
    else {
        return false;
    };
    pile_contains_all(grid_source_cards(combat, source_pile), uuids)
}

fn pile_contains_all(cards: &[CombatCard], uuids: &[u32]) -> bool {
    uuids
        .iter()
        .all(|uuid| cards.iter().any(|card| card.uuid == *uuid))
}

fn grid_source_cards(combat: &CombatState, source_pile: PileType) -> &[CombatCard] {
    match source_pile {
        PileType::Draw => &combat.zones.draw_pile,
        PileType::Discard => &combat.zones.discard_pile,
        PileType::Exhaust => &combat.zones.exhaust_pile,
        PileType::Hand => &combat.zones.hand,
        PileType::Limbo => &combat.zones.limbo,
        PileType::MasterDeck => &combat.meta.master_deck_snapshot,
    }
}
