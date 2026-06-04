use crate::ai::run_choice_policy_v1::{
    build_run_choice_decision_context_v1, plan_run_choice_decision_v1, RunChoicePolicyActionV1,
    RunChoicePolicyConfigV1,
};
use crate::content::cards::CardId;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;

#[test]
fn run_choice_policy_purges_visible_curse() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.add_card_to_deck_without_interception_from(
        CardId::Doubt,
        0,
        crate::state::selection::DomainEventSource::DeckMutation,
    );
    let choice = purge_choice();

    let context = build_run_choice_decision_context_v1(&run_state, &choice);
    let decision = plan_run_choice_decision_v1(&context, &RunChoicePolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        RunChoicePolicyActionV1::SelectDeckIndices { ref labels, .. }
            if labels.iter().any(|label| label == "Doubt")
    ));
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(
        &decision.to_noncombat_decision_record_v1(),
    )
    .expect("run choice policy record should validate");
}

#[test]
fn run_choice_policy_stops_without_visible_curse() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = purge_choice();

    let context = build_run_choice_decision_context_v1(&run_state, &choice);
    let decision = plan_run_choice_decision_v1(&context, &RunChoicePolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        RunChoicePolicyActionV1::Stop { .. }
    ));
}

fn purge_choice() -> RunPendingChoiceState {
    RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::PurgeNonBottled,
        return_state: Box::new(EngineState::MapNavigation),
    }
}
