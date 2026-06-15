use crate::ai::deck_mutation_compiler_v1::{
    compile_deck_mutation_decision_v1, render_compiled_deck_mutation_decision_v1,
    DeckMutationCompilerModeV1, DeckMutationPlanRoleV1, DeckMutationTargetLossTierV1,
};
use crate::content::cards::CardId;
use crate::runtime::combat::CombatCard;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;

#[test]
fn compiler_marks_functional_purge_inspect_only_when_low_value_targets_exist() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state
        .master_deck
        .push(CombatCard::new(CardId::TrueGrit, 99));
    let choice = choice(RunPendingChoiceReason::Purge, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerModeV1::BranchTopK { max_active: 12 },
    );

    assert!(decision.inspect_only_plans.iter().any(|plan| plan
        .step
        .cards
        .iter()
        .any(|card| card.card == CardId::TrueGrit)));
    assert!(!decision.branch_active_plans.iter().any(|plan| plan
        .step
        .cards
        .iter()
        .any(|card| card.card == CardId::TrueGrit)));
}

#[test]
fn compiler_keeps_low_value_purge_targets_branch_active() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = choice(RunPendingChoiceReason::Purge, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerModeV1::BranchTopK { max_active: 12 },
    );

    let commands = decision
        .branch_active_plans
        .iter()
        .map(|plan| (plan.step.command.as_str(), plan.step.cards[0].card))
        .collect::<Vec<_>>();
    assert_eq!(
        commands,
        vec![
            ("select 0", CardId::Strike),
            ("select 5", CardId::Defend),
            ("select 9", CardId::Bash),
        ]
    );
}

#[test]
fn compiler_execute_one_selects_evaluated_policy_preferred_plan() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = choice(RunPendingChoiceReason::PurgeNonBottled, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerModeV1::ExecuteOne,
    );
    let selected = decision.selected_plan.expect("selected plan");

    assert_eq!(selected.step.command, "select 0");
    assert_eq!(selected.role, DeckMutationPlanRoleV1::PolicyPreferred);
    assert!(selected.allowed_consumers.execute_autopilot);
    assert!(decision
        .candidate_plans
        .iter()
        .any(|candidate| candidate.plan_id == selected.plan_id));
}

#[test]
fn compiler_render_exposes_active_and_inspect_only_plan_groups() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state
        .master_deck
        .push(CombatCard::new(CardId::TrueGrit, 99));
    let choice = choice(RunPendingChoiceReason::Purge, 1);
    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerModeV1::BranchTopK { max_active: 12 },
    );

    let rendered = render_compiled_deck_mutation_decision_v1(&decision);

    assert!(rendered.contains("DeckMutationCompilerV1"));
    assert!(rendered.contains("selected_plan:"));
    assert!(rendered.contains("branch_active:"));
    assert!(rendered.contains("inspect_only:"));
    assert!(rendered.contains("True Grit"));
    assert!(rendered.contains("role=InspectOnly"));
}

#[test]
fn compiler_exposes_target_loss_for_functional_mutation_targets() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state
        .master_deck
        .push(CombatCard::new(CardId::BurningPact, 90));
    run_state
        .master_deck
        .push(CombatCard::new(CardId::TrueGrit, 91));
    run_state
        .master_deck
        .push(CombatCard::new(CardId::TrueGrit, 92));
    let choice = choice(RunPendingChoiceReason::Purge, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerModeV1::BranchTopK { max_active: 16 },
    );

    let burning_pact = decision
        .candidate_plans
        .iter()
        .find(|plan| plan.step.cards[0].card == CardId::BurningPact)
        .expect("Burning Pact candidate");
    let true_grit = decision
        .candidate_plans
        .iter()
        .find(|plan| plan.step.cards[0].card == CardId::TrueGrit)
        .expect("True Grit candidate");

    assert_eq!(
        burning_pact.step.cards[0].target_loss.tier,
        DeckMutationTargetLossTierV1::CoreFunctional
    );
    assert_eq!(
        true_grit.step.cards[0].target_loss.tier,
        DeckMutationTargetLossTierV1::Functional
    );
    assert!(
        true_grit.score_hint > burning_pact.score_hint,
        "redundant functional target should be ranked before singleton core functional target"
    );
    assert!(burning_pact
        .reasons
        .iter()
        .any(|reason| reason.contains("target_loss=CoreFunctional")));
}

fn choice(reason: RunPendingChoiceReason, count: usize) -> RunPendingChoiceState {
    RunPendingChoiceState {
        min_choices: count,
        max_choices: count,
        reason,
        return_state: Box::new(EngineState::MapNavigation),
    }
}
