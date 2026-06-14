use crate::ai::deck_mutation_compiler_v1::{
    compile_deck_mutation_decision_v1, DeckMutationCompilerModeV1,
};
use crate::state::core::{RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;

use super::types::NeowRunSelectionDecisionV1;

pub fn neow_followup_selection_v1(
    run_state: &RunState,
    choice: &RunPendingChoiceState,
    _player_class: &str,
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

    let decision = compile_deck_mutation_decision_v1(
        run_state,
        choice,
        DeckMutationCompilerModeV1::ExecuteOne,
    );
    let selected = decision.selected_plan?;
    if selected.step.deck_indices.len() != choice.min_choices
        || selected.step.cards.len() != choice.min_choices
    {
        return None;
    }

    Some(NeowRunSelectionDecisionV1 {
        command: selected.step.command,
        selected_deck_indices: selected.step.deck_indices,
        selected_cards: selected
            .step
            .cards
            .into_iter()
            .map(|card| (card.card, card.upgrades))
            .collect(),
        selection_mode: "deck_mutation_compiler_v1",
    })
}
