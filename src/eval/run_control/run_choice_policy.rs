use crate::state::core::{ClientInput, EngineState};
use crate::state::selection::{SelectionResolution, SelectionScope};

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_run_choice_policy_deck_selection(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return Ok(None);
    };
    let decision = crate::ai::deck_mutation_compiler_v1::compile_deck_mutation_decision_v1(
        &session.run_state,
        choice,
        crate::ai::deck_mutation_compiler_v1::DeckMutationCompilerModeV1::ExecuteOne,
    );
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let Some(selected_plan) = decision.selected_plan else {
        return Ok(None);
    };
    let uuids = selected_plan
        .step
        .deck_indices
        .iter()
        .filter_map(|idx| {
            session
                .run_state
                .master_deck
                .get(*idx)
                .map(|card| card.uuid)
        })
        .collect::<Vec<_>>();
    let labels = selected_plan
        .step
        .cards
        .iter()
        .map(|card| card.label.clone())
        .collect::<Vec<_>>();
    let confidence = selected_plan.confidence;
    let reason = selected_plan.reasons.join("; ");

    let outcome = session
        .apply_input(ClientInput::SubmitSelection(
            SelectionResolution::card_uuids(SelectionScope::Deck, uuids),
        ))?
        .with_trace_annotations(vec![
            super::noncombat_policy_annotation::noncombat_policy_annotation(
                "run choice policy",
                noncombat_record,
            )?,
        ]);
    Ok(Some((
        outcome,
        format!(
            "deck mutation compiler: select {} confidence={confidence:.2} reason={reason} label_role={}",
            labels.join(", "),
            decision.label_role
        ),
    )))
}
