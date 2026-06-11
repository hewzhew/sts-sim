use crate::ai::run_choice_policy_v1::{
    build_run_choice_decision_context_v1, plan_run_choice_decision_v1, RunChoicePolicyActionV1,
    RunChoicePolicyClassV1, RunChoicePolicyConfigV1,
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

#[test]
fn run_choice_policy_purges_visible_starter_strike_when_no_curse_exists() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = purge_choice();
    let context = build_run_choice_decision_context_v1(&run_state, &choice);

    let decision = plan_run_choice_decision_v1(&context, &RunChoicePolicyConfigV1::default());

    let RunChoicePolicyActionV1::SelectDeckIndices {
        indices, labels, ..
    } = decision.action
    else {
        panic!("expected low-value starter purge selection");
    };
    assert_eq!(indices.len(), 1);
    assert_eq!(labels, vec!["Strike".to_string()]);
}

#[test]
fn run_choice_policy_transform_upgraded_prefers_starter_shell_targets() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = RunPendingChoiceState {
        min_choices: 3,
        max_choices: 3,
        reason: RunPendingChoiceReason::TransformUpgraded,
        return_state: Box::new(EngineState::MapNavigation),
    };
    let context = build_run_choice_decision_context_v1(&run_state, &choice);

    let decision = plan_run_choice_decision_v1(&context, &RunChoicePolicyConfigV1::default());

    let RunChoicePolicyActionV1::SelectDeckIndices { labels, .. } = decision.action else {
        panic!("expected low-value transform selection");
    };
    assert_eq!(
        labels,
        vec![
            "Strike".to_string(),
            "Defend".to_string(),
            "Strike".to_string()
        ]
    );
}

#[test]
fn run_choice_policy_upgrades_high_priority_card_with_smith_priority() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::Upgrade,
        return_state: Box::new(EngineState::MapNavigation),
    };
    let context = build_run_choice_decision_context_v1(&run_state, &choice);

    let decision = plan_run_choice_decision_v1(&context, &RunChoicePolicyConfigV1::default());

    let RunChoicePolicyActionV1::SelectDeckIndices {
        indices, labels, ..
    } = decision.action
    else {
        panic!("expected high-priority upgrade selection");
    };
    assert_eq!(indices, vec![9]);
    assert_eq!(labels, vec!["Bash".to_string()]);
}

fn purge_choice() -> RunPendingChoiceState {
    RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::PurgeNonBottled,
        return_state: Box::new(EngineState::MapNavigation),
    }
}
