use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::state::core::{RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;
use crate::state::selection::SelectionTargetRef;

use super::types::NeowRunSelectionDecisionV1;

pub fn neow_followup_selection_v1(
    run_state: &RunState,
    choice: &RunPendingChoiceState,
    player_class: &str,
) -> Option<NeowRunSelectionDecisionV1> {
    if choice.min_choices == 0 || choice.min_choices != choice.max_choices {
        return None;
    }
    if !matches!(
        choice.reason,
        RunPendingChoiceReason::Purge
            | RunPendingChoiceReason::PurgeNonBottled
            | RunPendingChoiceReason::Transform
            | RunPendingChoiceReason::TransformNonBottled
            | RunPendingChoiceReason::TransformUpgraded
            | RunPendingChoiceReason::Upgrade
    ) {
        return None;
    }

    let candidates = neow_selection_candidates(run_state, choice);
    let selected = select_neow_followup_candidates(
        &candidates,
        choice.reason,
        choice.min_choices,
        player_class,
    )?;
    let selected_deck_indices = selected
        .iter()
        .map(|candidate| candidate.deck_idx)
        .collect::<Vec<_>>();
    Some(NeowRunSelectionDecisionV1 {
        command: format_select_command(&selected_deck_indices),
        selected_deck_indices,
        selected_cards: selected
            .iter()
            .map(|candidate| (candidate.card_id, candidate.upgrades))
            .collect(),
        selection_mode: "neow_followup_starter_baseline_v1",
    })
}

#[derive(Clone, Copy, Debug)]
struct NeowSelectionCandidate {
    deck_idx: usize,
    card_id: CardId,
    upgrades: u8,
    card_type: CardType,
}

fn neow_selection_candidates(
    run_state: &RunState,
    choice: &RunPendingChoiceState,
) -> Vec<NeowSelectionCandidate> {
    let request = choice.selection_request(run_state);
    let target_uuids = request
        .targets
        .iter()
        .map(|target| match target {
            SelectionTargetRef::CardUuid(uuid) => *uuid,
        })
        .collect::<std::collections::BTreeSet<_>>();
    run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| target_uuids.contains(&card.uuid))
        .map(|(deck_idx, card)| NeowSelectionCandidate {
            deck_idx,
            card_id: card.id,
            upgrades: card.upgrades,
            card_type: get_card_definition(card.id).card_type,
        })
        .collect()
}

fn select_neow_followup_candidates<'a>(
    candidates: &'a [NeowSelectionCandidate],
    reason: RunPendingChoiceReason,
    count: usize,
    player_class: &str,
) -> Option<Vec<&'a NeowSelectionCandidate>> {
    if candidates.len() < count {
        return None;
    }
    let mut selected = Vec::new();
    select_curses_first(candidates, &mut selected, count);
    if selected.len() >= count {
        return Some(selected);
    }

    match reason {
        RunPendingChoiceReason::Upgrade => {
            select_upgrade_targets(candidates, &mut selected, count, player_class);
        }
        RunPendingChoiceReason::Purge
        | RunPendingChoiceReason::PurgeNonBottled
        | RunPendingChoiceReason::Transform
        | RunPendingChoiceReason::TransformNonBottled
        | RunPendingChoiceReason::TransformUpgraded => {
            select_remove_or_transform_targets(candidates, &mut selected, count, player_class);
        }
        _ => return None,
    }

    fill_remaining_stable(candidates, &mut selected, count);
    (selected.len() == count).then_some(selected)
}

fn select_curses_first<'a>(
    candidates: &'a [NeowSelectionCandidate],
    selected: &mut Vec<&'a NeowSelectionCandidate>,
    count: usize,
) {
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.card_type == CardType::Curse)
    {
        push_if_room(selected, candidate, count);
    }
}

fn select_remove_or_transform_targets<'a>(
    candidates: &'a [NeowSelectionCandidate],
    selected: &mut Vec<&'a NeowSelectionCandidate>,
    count: usize,
    player_class: &str,
) {
    let order = remove_transform_card_order(player_class, count);
    for card_id in &order {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| starter_group_matches(candidate.card_id, *card_id))
        {
            push_if_room(selected, candidate, count);
        }
    }
    for card_id in order {
        for candidate in candidates {
            if !starter_group_matches(candidate.card_id, card_id) {
                continue;
            }
            let group_already_selected = selected
                .iter()
                .any(|selected| starter_group_matches(selected.card_id, candidate.card_id));
            if group_already_selected {
                continue;
            }
            push_if_room(selected, candidate, count);
        }
    }
}

fn select_upgrade_targets<'a>(
    candidates: &'a [NeowSelectionCandidate],
    selected: &mut Vec<&'a NeowSelectionCandidate>,
    count: usize,
    player_class: &str,
) {
    for card_id in upgrade_card_order(player_class) {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| starter_group_matches(candidate.card_id, card_id))
        {
            push_if_room(selected, candidate, count);
        }
    }
}

fn fill_remaining_stable<'a>(
    candidates: &'a [NeowSelectionCandidate],
    selected: &mut Vec<&'a NeowSelectionCandidate>,
    count: usize,
) {
    for candidate in candidates {
        push_if_room(selected, candidate, count);
    }
}

fn push_if_room<'a>(
    selected: &mut Vec<&'a NeowSelectionCandidate>,
    candidate: &'a NeowSelectionCandidate,
    count: usize,
) {
    if selected.len() >= count
        || selected
            .iter()
            .any(|existing| existing.deck_idx == candidate.deck_idx)
    {
        return;
    }
    selected.push(candidate);
}

fn remove_transform_card_order(player_class: &str, count: usize) -> Vec<CardId> {
    match player_class.to_ascii_lowercase().as_str() {
        "watcher" | "purple" => vec![CardId::DefendP, CardId::StrikeP, CardId::Eruption],
        "silent" | "green" => vec![CardId::StrikeG, CardId::DefendG, CardId::Survivor],
        "defect" | "blue" => vec![CardId::StrikeB, CardId::DefendB, CardId::Dualcast],
        _ if count >= 2 => vec![CardId::Strike, CardId::Defend, CardId::Bash],
        _ => vec![CardId::Strike, CardId::Defend, CardId::Bash],
    }
}

fn upgrade_card_order(player_class: &str) -> Vec<CardId> {
    match player_class.to_ascii_lowercase().as_str() {
        "watcher" | "purple" => vec![CardId::Eruption, CardId::Vigilance],
        "silent" | "green" => vec![CardId::Neutralize, CardId::Survivor],
        "defect" | "blue" => vec![CardId::Zap, CardId::Dualcast],
        _ => vec![CardId::Bash],
    }
}

fn starter_group_matches(actual: CardId, desired: CardId) -> bool {
    actual == desired
        || (is_strike(actual) && is_strike(desired))
        || (is_defend(actual) && is_defend(desired))
}

fn is_strike(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Strike | CardId::StrikeG | CardId::StrikeB | CardId::StrikeP
    )
}

fn is_defend(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Defend | CardId::DefendG | CardId::DefendB | CardId::DefendP
    )
}

fn format_select_command(indices: &[usize]) -> String {
    format!(
        "select {}",
        indices
            .iter()
            .map(|idx| idx.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    )
}
