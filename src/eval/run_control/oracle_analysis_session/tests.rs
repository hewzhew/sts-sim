use super::*;

use crate::eval::run_control::{
    seed_oracle_run_explorer_from_session_v1, RunControlConfig, RunControlSearchCombatOptions,
    RunControlSession,
};
use crate::runtime::combat::CombatCard;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::selection::DomainEventSource;

fn parameterized_selection_analysis() -> OracleAnalysisSessionV1 {
    let mut run = RunControlSession::new(RunControlConfig::default());
    run.run_state.master_deck = (0..3)
        .map(|uuid| CombatCard::new(crate::content::cards::CardId::Strike, uuid))
        .collect();
    run.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
        min_choices: 2,
        max_choices: 2,
        reason: RunPendingChoiceReason::PurgeNonBottled,
        source: DomainEventSource::Selection(RunPendingChoiceReason::PurgeNonBottled.into()),
        return_state: Box::new(EngineState::MapNavigation),
    });
    let combat_budgets =
        OracleRunCombatBudgetsV1::uniform(RunControlSearchCombatOptions::default());
    let explorer = seed_oracle_run_explorer_from_session_v1(
        run,
        RunProgressJournalV1::default(),
        &combat_budgets,
        None,
    )
    .expect("seed parameterized analysis");
    OracleAnalysisSessionV1::from_explorer(explorer, Some(0), combat_budgets, None, None)
        .expect("parameterized analysis")
}

#[test]
fn failed_choice_does_not_widen_or_mutate_the_analysis_session() {
    let mut analysis = parameterized_selection_analysis();
    let work = analysis
        .explorer
        .pending_decisions
        .front_mut()
        .expect("one lazy selection member");
    let requested_ref = choice_ref(work);
    work.parent_state_fingerprint = "stale-parent-state".to_string();
    let before = serde_json::to_value(analysis.checkpoint().expect("checkpoint before failure"))
        .expect("serialize before checkpoint");

    let error = analysis
        .try_choice(&requested_ref)
        .expect_err("stale exact work must fail");

    assert!(error.contains("parent fingerprint changed"), "{error}");
    let after = serde_json::to_value(analysis.checkpoint().expect("checkpoint after failure"))
        .expect("serialize after checkpoint");
    assert_eq!(
        after, before,
        "a rejected user operation must not widen selection supply or mutate navigation"
    );
}

#[test]
fn successful_choice_commits_exactly_one_selection_widening_step() {
    let mut analysis = parameterized_selection_analysis();
    let original_ref = choice_ref(
        analysis
            .explorer
            .pending_decisions
            .front()
            .expect("one lazy selection member"),
    );

    let child_node_id = analysis
        .try_choice(&original_ref)
        .expect("exact selection materializes");

    assert_eq!(analysis.cursor_node_id(), child_node_id);
    let parent = analysis.view_node(0).expect("parent remains inspectable");
    assert_eq!(
        parent.choices.len(),
        2,
        "one successful service preserves the tried member and exposes one sibling"
    );
    assert!(parent
        .choices
        .iter()
        .any(|choice| choice.choice_ref == original_ref));
    assert_eq!(parent.children.len(), 1);
    assert_eq!(parent.children[0].child_node_id, child_node_id);
}
