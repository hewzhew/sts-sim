use crate::ai::run_choice_policy_v1::{
    build_run_choice_decision_context_v1, RunChoicePolicyClassV1,
};
use crate::content::cards::CardId;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;

#[test]
fn run_choice_context_exposes_visible_curse_candidate() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.add_card_to_deck_without_interception_from(
        CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
    let choice = purge_choice();

    let context = build_run_choice_decision_context_v1(&run_state, &choice);

    assert!(context
        .candidates
        .iter()
        .any(|candidate| candidate.class == RunChoicePolicyClassV1::CursePurge));
}

#[test]
fn run_choice_context_has_no_visible_curse_candidate_without_curse() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = purge_choice();

    let context = build_run_choice_decision_context_v1(&run_state, &choice);

    assert!(!context
        .candidates
        .iter()
        .any(|candidate| candidate.class == RunChoicePolicyClassV1::CursePurge));
}

fn purge_choice() -> RunPendingChoiceState {
    RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::PurgeNonBottled,
        return_state: Box::new(EngineState::MapNavigation),
    }
}
