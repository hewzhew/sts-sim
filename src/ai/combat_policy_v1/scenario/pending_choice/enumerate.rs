use crate::state::core::{ClientInput, PendingChoice};
use crate::state::selection::{SelectionResolution, SelectionScope};

use super::super::types::{CombatScenarioPolicyErrorV1, ExactActionInputs};
use super::{pending_choice_kind, CombatPublicPendingChoiceKindV1};

pub(super) const MAX_PENDING_CHOICE_EXACT_ACTIONS: usize = 4_096;

pub(in crate::ai::combat_policy_v1::scenario) fn exact_pending_choice_inputs(
    scenario_id: &str,
    choice: &PendingChoice,
) -> Result<ExactActionInputs, CombatScenarioPolicyErrorV1> {
    let inputs = match choice {
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            ..
        } => {
            let mut inputs = selection_inputs(
                scenario_id,
                CombatPublicPendingChoiceKindV1::HandSelect,
                SelectionScope::Hand,
                candidate_uuids,
                *min_cards as usize,
                *max_cards as usize,
            )?;
            if *can_cancel {
                inputs.push(ClientInput::Cancel);
            }
            inputs
        }
        PendingChoice::GridSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            ..
        } => {
            let mut inputs = selection_inputs(
                scenario_id,
                CombatPublicPendingChoiceKindV1::GridSelect,
                SelectionScope::Grid,
                candidate_uuids,
                *min_cards as usize,
                *max_cards as usize,
            )?;
            if *can_cancel {
                inputs.push(ClientInput::Cancel);
            }
            inputs
        }
        PendingChoice::DiscoverySelect(choice) => {
            let mut inputs = indexed_choices(choice.cards.len());
            if choice.can_skip {
                inputs.push(ClientInput::Cancel);
            }
            inputs
        }
        PendingChoice::ScrySelect { cards, card_uuids } => {
            if cards.len() != card_uuids.len() {
                return Err(CombatScenarioPolicyErrorV1::InvalidPendingChoice {
                    scenario_id: scenario_id.to_string(),
                    choice_kind: CombatPublicPendingChoiceKindV1::ScrySelect,
                    detail: "scry card and UUID counts differ".to_string(),
                });
            }
            index_subsets(
                scenario_id,
                CombatPublicPendingChoiceKindV1::ScrySelect,
                cards.len(),
                0,
                cards.len(),
            )?
            .into_iter()
            .map(ClientInput::SubmitScryDiscard)
            .collect()
        }
        PendingChoice::CardRewardSelect {
            cards, can_skip, ..
        } => {
            let mut inputs = indexed_choices(cards.len());
            if *can_skip {
                inputs.push(ClientInput::Cancel);
            }
            inputs
        }
        PendingChoice::ForeignInfluenceSelect { cards, .. } => indexed_choices(cards.len()),
        PendingChoice::ChooseOneSelect { choices } => indexed_choices(choices.len()),
        PendingChoice::StanceChoice => indexed_choices(2),
    };

    if inputs.len() > MAX_PENDING_CHOICE_EXACT_ACTIONS {
        return Err(CombatScenarioPolicyErrorV1::CandidateSpaceTooLarge {
            scenario_id: scenario_id.to_string(),
            choice_kind: pending_choice_kind(choice),
            candidate_count: pending_candidate_count(choice),
            action_count: inputs.len(),
            cap: MAX_PENDING_CHOICE_EXACT_ACTIONS,
        });
    }
    if inputs.is_empty() {
        return Err(CombatScenarioPolicyErrorV1::InvalidPendingChoice {
            scenario_id: scenario_id.to_string(),
            choice_kind: pending_choice_kind(choice),
            detail: "pending choice exposes no legal action".to_string(),
        });
    }
    Ok(inputs)
}

fn selection_inputs(
    scenario_id: &str,
    choice_kind: CombatPublicPendingChoiceKindV1,
    scope: SelectionScope,
    candidate_uuids: &[u32],
    min_cards: usize,
    max_cards: usize,
) -> Result<ExactActionInputs, CombatScenarioPolicyErrorV1> {
    let selections = index_subsets(
        scenario_id,
        choice_kind,
        candidate_uuids.len(),
        min_cards,
        max_cards,
    )?;
    Ok(selections
        .into_iter()
        .map(|indices| {
            ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                scope,
                indices.into_iter().map(|index| candidate_uuids[index]),
            ))
        })
        .collect())
}

fn index_subsets(
    scenario_id: &str,
    choice_kind: CombatPublicPendingChoiceKindV1,
    candidate_count: usize,
    min_cards: usize,
    max_cards: usize,
) -> Result<Vec<Vec<usize>>, CombatScenarioPolicyErrorV1> {
    let effective_max = max_cards.min(candidate_count);
    if min_cards > effective_max {
        return Err(CombatScenarioPolicyErrorV1::InvalidPendingChoice {
            scenario_id: scenario_id.to_string(),
            choice_kind,
            detail: format!(
                "minimum selection {min_cards} exceeds {candidate_count} available candidates"
            ),
        });
    }
    let action_count = bounded_subset_count(candidate_count, min_cards, effective_max, usize::MAX);
    if action_count > MAX_PENDING_CHOICE_EXACT_ACTIONS {
        return Err(CombatScenarioPolicyErrorV1::CandidateSpaceTooLarge {
            scenario_id: scenario_id.to_string(),
            choice_kind,
            candidate_count,
            action_count,
            cap: MAX_PENDING_CHOICE_EXACT_ACTIONS,
        });
    }

    let mut selections = Vec::with_capacity(action_count);
    let mut current = Vec::new();
    for target_size in min_cards..=effective_max {
        collect_index_combinations(
            candidate_count,
            target_size,
            0,
            &mut current,
            &mut selections,
        );
    }
    Ok(selections)
}

fn bounded_subset_count(
    candidate_count: usize,
    min_cards: usize,
    max_cards: usize,
    limit: usize,
) -> usize {
    let mut total = 0usize;
    for selected in min_cards..=max_cards {
        total = total.saturating_add(bounded_binomial(candidate_count, selected, limit));
        if total > limit {
            return limit.saturating_add(1);
        }
    }
    total
}

fn bounded_binomial(n: usize, k: usize, limit: usize) -> usize {
    let k = k.min(n.saturating_sub(k));
    let mut value = 1u128;
    for index in 0..k {
        value = value
            .saturating_mul((n - index) as u128)
            .checked_div((index + 1) as u128)
            .unwrap_or(u128::MAX);
        if value > limit as u128 {
            return limit.saturating_add(1);
        }
    }
    value.min(usize::MAX as u128) as usize
}

fn collect_index_combinations(
    candidate_count: usize,
    target_size: usize,
    start: usize,
    current: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    if current.len() == target_size {
        out.push(current.clone());
        return;
    }
    let remaining = target_size - current.len();
    if candidate_count.saturating_sub(start) < remaining {
        return;
    }
    let max_start = candidate_count.saturating_sub(remaining);
    for index in start..=max_start {
        current.push(index);
        collect_index_combinations(candidate_count, target_size, index + 1, current, out);
        current.pop();
    }
}

fn indexed_choices(count: usize) -> ExactActionInputs {
    (0..count).map(ClientInput::SubmitDiscoverChoice).collect()
}

fn pending_candidate_count(choice: &PendingChoice) -> usize {
    match choice {
        PendingChoice::HandSelect {
            candidate_uuids, ..
        }
        | PendingChoice::GridSelect {
            candidate_uuids, ..
        } => candidate_uuids.len(),
        PendingChoice::DiscoverySelect(choice) => choice.cards.len(),
        PendingChoice::ScrySelect { cards, .. } => cards.len(),
        PendingChoice::CardRewardSelect { cards, .. }
        | PendingChoice::ForeignInfluenceSelect { cards, .. } => cards.len(),
        PendingChoice::ChooseOneSelect { choices } => choices.len(),
        PendingChoice::StanceChoice => 2,
    }
}
