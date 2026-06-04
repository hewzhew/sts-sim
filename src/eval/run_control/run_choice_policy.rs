use crate::content::cards::{get_card_definition, CardType};
use crate::state::core::{ClientInput, EngineState, RunPendingChoiceReason, RunPendingChoiceState};

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_run_choice_policy_purge_curse(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let Some((indices, labels)) = run_choice_curse_purge_selection(session) else {
        return Ok(None);
    };
    let summary = format!("run choice policy: purge {}", labels.join(", "));
    let outcome = session.apply_input(ClientInput::SubmitDeckSelect(indices))?;
    Ok(Some((outcome, summary)))
}

fn run_choice_curse_purge_selection(
    session: &RunControlSession,
) -> Option<(Vec<usize>, Vec<&'static str>)> {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return None;
    };
    if !is_purge_choice(choice) {
        return None;
    }

    let mut curse_indices = session
        .run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| {
            crate::state::core::run_pending_choice_allows_card_for_run(
                &choice.reason,
                card,
                &session.run_state,
            ) && get_card_definition(card.id).card_type == CardType::Curse
        })
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();

    if curse_indices.len() < choice.min_choices {
        return None;
    }
    curse_indices.truncate(choice.max_choices);
    if curse_indices.len() < choice.min_choices || curse_indices.is_empty() {
        return None;
    }

    let labels = curse_indices
        .iter()
        .filter_map(|idx| session.run_state.master_deck.get(*idx))
        .map(|card| get_card_definition(card.id).name)
        .collect::<Vec<_>>();
    Some((curse_indices, labels))
}

fn is_purge_choice(choice: &RunPendingChoiceState) -> bool {
    matches!(
        choice.reason,
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled
    )
}
