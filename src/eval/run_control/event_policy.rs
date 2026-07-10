use crate::content::cards::{get_card_definition, CardId, CardRarity, CardType};
use crate::state::core::ClientInput;
use crate::state::events::{EventId, EventState};
use crate::state::run::RunState;

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_match_and_keep_policy_choice(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let Some(event_state) = session.run_state.event_state.as_ref() else {
        return Ok(None);
    };
    if event_state.id != EventId::MatchAndKeep {
        return Ok(None);
    }

    let board_ready = match_and_keep_board_ready(event_state);
    let choice = match event_state.current_screen {
        0 => Some(0),
        3 | 4 if board_ready => Some(0),
        1 if board_ready => match_and_keep_first_flip_choice_index(&session.run_state, event_state),
        2 if board_ready => {
            match_and_keep_second_flip_choice_index(&session.run_state, event_state)
        }
        _ => None,
    };
    let Some(index) = choice else {
        return Ok(None);
    };

    let label = crate::engine::event_handler::get_event_options(&session.run_state)
        .get(index)
        .map(|option| option.ui.text.clone())
        .unwrap_or_else(|| format!("choice {index}"));
    let outcome = session.apply_input(ClientInput::EventChoice(index))?;
    Ok(Some((
        outcome,
        format!(
            "event policy: Match and Keep {label} reason=board-pair strategy avoids curse/status pairs label_role=behavior_policy_not_teacher"
        ),
    )))
}

pub(super) fn apply_note_for_yourself_policy_choice(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let Some(event_state) = session.run_state.event_state.as_ref() else {
        return Ok(None);
    };
    if event_state.id != EventId::NoteForYourself || event_state.current_screen != 1 {
        return Ok(None);
    }
    if !crate::content::events::note_for_yourself::default_note_is_ignorable(&session.run_state) {
        return Ok(None);
    }

    let ignore_index = 1usize;
    let label = crate::engine::event_handler::get_event_options(&session.run_state)
        .get(ignore_index)
        .map(|option| option.ui.text.clone())
        .unwrap_or_else(|| "Ignore".to_string());
    let outcome = session.apply_input(ClientInput::EventChoice(ignore_index))?;
    Ok(Some((
        outcome,
        format!(
            "event policy: Note For Yourself {label} reason=ignore default low-value note card; take+remove needs deck mutation compiler label_role=behavior_policy_not_teacher"
        ),
    )))
}

fn match_and_keep_board_ready(event_state: &EventState) -> bool {
    event_state.extra_data.len() > 28
}

fn match_and_keep_first_flip_choice_index(
    run_state: &RunState,
    event_state: &EventState,
) -> Option<usize> {
    let positions = match_and_keep_available_positions(event_state);
    let pos = positions
        .iter()
        .filter_map(|&pos| {
            let (card_id, upgrades) = crate::content::events::match_and_keep::card_entry_at(
                run_state,
                &event_state.extra_data,
                pos,
            )?;
            let has_pair = positions.iter().any(|&other| {
                other != pos
                    && crate::content::events::match_and_keep::card_entry_at(
                        run_state,
                        &event_state.extra_data,
                        other,
                    )
                    .is_some_and(|(other_id, _)| other_id == card_id)
            });
            has_pair.then_some((match_and_keep_card_rank(card_id, upgrades), pos))
        })
        .max_by_key(|&(rank, pos)| (rank, std::cmp::Reverse(pos)))?
        .1;
    match_and_keep_choice_index_for_position(event_state, pos)
}

fn match_and_keep_second_flip_choice_index(
    run_state: &RunState,
    event_state: &EventState,
) -> Option<usize> {
    let first = crate::content::events::match_and_keep::first_flipped(&event_state.extra_data);
    if first < 0 {
        return None;
    }
    let first = first as usize;
    let positions = match_and_keep_available_positions(event_state);
    let first_card = crate::content::events::match_and_keep::card_entry_at(
        run_state,
        &event_state.extra_data,
        first,
    );

    if let Some((first_card_id, _)) = first_card {
        if match_and_keep_card_is_keepable(first_card_id) {
            if let Some(pair_pos) = positions.iter().copied().find(|&pos| {
                crate::content::events::match_and_keep::card_entry_at(
                    run_state,
                    &event_state.extra_data,
                    pos,
                )
                .is_some_and(|(card_id, _)| card_id == first_card_id)
            }) {
                return match_and_keep_choice_index_for_position(event_state, pair_pos);
            }
        }
    }

    let fallback = positions
        .iter()
        .filter_map(|&pos| {
            let (card_id, upgrades) = crate::content::events::match_and_keep::card_entry_at(
                run_state,
                &event_state.extra_data,
                pos,
            )?;
            let mismatches_first =
                first_card.is_none_or(|(first_card_id, _)| card_id != first_card_id);
            (mismatches_first && match_and_keep_card_is_keepable(card_id))
                .then_some((match_and_keep_card_rank(card_id, upgrades), pos))
        })
        .max_by_key(|&(rank, pos)| (rank, std::cmp::Reverse(pos)))
        .map(|(_, pos)| pos)
        .or_else(|| positions.first().copied())?;
    match_and_keep_choice_index_for_position(event_state, fallback)
}

fn match_and_keep_available_positions(event_state: &EventState) -> Vec<usize> {
    let first = crate::content::events::match_and_keep::first_flipped(&event_state.extra_data);
    (0..12usize)
        .filter(|&pos| {
            !crate::content::events::match_and_keep::is_matched(&event_state.extra_data, pos)
                && !(event_state.current_screen == 2 && first == pos as i32)
        })
        .collect()
}

fn match_and_keep_choice_index_for_position(
    event_state: &EventState,
    target_pos: usize,
) -> Option<usize> {
    match_and_keep_available_positions(event_state)
        .iter()
        .position(|&pos| pos == target_pos)
}

fn match_and_keep_card_is_keepable(card_id: CardId) -> bool {
    !matches!(
        get_card_definition(card_id).card_type,
        CardType::Curse | CardType::Status
    )
}

fn match_and_keep_card_rank(card_id: CardId, upgrades: u8) -> i32 {
    let def = get_card_definition(card_id);
    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        return -10_000;
    }

    let rarity = match def.rarity {
        CardRarity::Rare => 600,
        CardRarity::Uncommon => 450,
        CardRarity::Common => 300,
        CardRarity::Basic => 120,
        CardRarity::Special => 180,
        CardRarity::Curse => -10_000,
    };
    let type_bonus = match def.card_type {
        CardType::Power => 35,
        CardType::Attack | CardType::Skill => 20,
        CardType::Status | CardType::Curse => -10_000,
    };
    let mechanics = (def.base_damage + def.base_block + def.base_magic * 2).clamp(0, 60);

    rarity + type_bonus + mechanics + upgrades as i32 * 40
}
