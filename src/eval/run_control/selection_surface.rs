use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::core::{ClientInput, EngineState, PendingChoice, PileType};
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};

use super::session::RunControlSession;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SelectionSurface {
    pub min_choices: usize,
    pub max_choices: usize,
    pub can_cancel: bool,
    pub item_count: usize,
    pub submit_hint: &'static str,
}

pub(super) fn active_selection_surface(session: &RunControlSession) -> Option<SelectionSurface> {
    match &session.engine_state {
        EngineState::PendingChoice(choice) => match choice {
            PendingChoice::HandSelect {
                candidate_uuids,
                min_cards,
                max_cards,
                can_cancel,
                ..
            }
            | PendingChoice::GridSelect {
                candidate_uuids,
                min_cards,
                max_cards,
                can_cancel,
                ..
            } => Some(SelectionSurface {
                min_choices: *min_cards as usize,
                max_choices: (*max_cards as usize).min(candidate_uuids.len()),
                can_cancel: *can_cancel,
                item_count: candidate_uuids.len(),
                submit_hint: "select <idx...>",
            }),
            PendingChoice::ScrySelect { cards, .. } => Some(SelectionSurface {
                min_choices: 0,
                max_choices: cards.len(),
                can_cancel: false,
                item_count: cards.len(),
                submit_hint: "select <idx...>",
            }),
            _ => None,
        },
        EngineState::RunPendingChoice(choice) => Some(SelectionSurface {
            min_choices: choice.min_choices,
            max_choices: choice.max_choices,
            can_cancel: choice.min_choices == 0,
            item_count: choice.selection_request(&session.run_state).targets.len(),
            submit_hint: "select <deck_idx...>",
        }),
        _ => None,
    }
}

pub(super) fn resolve_selection_indices(
    session: &RunControlSession,
    indices: Vec<usize>,
) -> Result<ClientInput, String> {
    match &session.engine_state {
        EngineState::PendingChoice(choice) => {
            resolve_combat_selection_indices(session, choice, indices)
        }
        EngineState::RunPendingChoice(choice) => {
            resolve_run_pending_selection_indices(session, choice, indices)
        }
        _ => Err("select <idx...> is only valid on a selection screen".to_string()),
    }
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

fn resolve_combat_selection_indices(
    session: &RunControlSession,
    choice: &PendingChoice,
    indices: Vec<usize>,
) -> Result<ClientInput, String> {
    match choice {
        PendingChoice::HandSelect {
            candidate_uuids, ..
        } => Ok(selection_input(
            SelectionScope::Hand,
            indices_to_uuids(candidate_uuids, &indices)?,
        )),
        PendingChoice::GridSelect {
            candidate_uuids, ..
        } => Ok(selection_input(
            SelectionScope::Grid,
            indices_to_uuids(candidate_uuids, &indices)?,
        )),
        PendingChoice::ScrySelect { cards, .. } => {
            validate_indices_in_range(cards.len(), &indices)?;
            reject_duplicate_indices(&indices)?;
            Ok(ClientInput::SubmitScryDiscard(indices))
        }
        _ => Err(
            "select <idx...> is not used for this pending choice; choose a visible id".to_string(),
        ),
    }
    .and_then(|input| {
        if pending_choice_input_is_allowed(session, choice, &input) {
            Ok(input)
        } else {
            Err(format!(
                "selection `{}` is not valid for the current bounds",
                crate::eval::run_control::view_model::client_input_hint(&input)
            ))
        }
    })
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

fn resolve_run_pending_selection_indices(
    session: &RunControlSession,
    choice: &crate::state::core::RunPendingChoiceState,
    indices: Vec<usize>,
) -> Result<ClientInput, String> {
    if !session.run_pending_selection_is_allowed(choice, &indices) {
        return Err("selection is not valid for the current deck choice".to_string());
    }
    let uuids = indices
        .into_iter()
        .filter_map(|idx| session.run_state.master_deck.get(idx).map(|card| card.uuid))
        .collect::<Vec<_>>();
    Ok(selection_input(SelectionScope::Deck, uuids))
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

fn indices_to_uuids(candidate_uuids: &[u32], indices: &[usize]) -> Result<Vec<u32>, String> {
    validate_indices_in_range(candidate_uuids.len(), indices)?;
    reject_duplicate_indices(indices)?;
    Ok(indices.iter().map(|idx| candidate_uuids[*idx]).collect())
}

fn selection_input(scope: SelectionScope, uuids: Vec<u32>) -> ClientInput {
    ClientInput::SubmitSelection(SelectionResolution::card_uuids(scope, uuids))
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
