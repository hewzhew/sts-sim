use crate::ai::deck_mutation_compiler_v1::{
    best_duplicate_target_for_shop_v1, compile_deck_mutation_decision_v1,
    deck_removal_target_snapshots_v1, render_compiled_deck_mutation_decision_v1,
    DeckMutationCompilerRequestV1, DeckMutationPlanRoleV1, DeckMutationTargetLossTierV1,
    DuplicateTargetRoleV1, TransformRandomAdditionBandV1, TransformVarianceRiskV1,
};
use crate::ai::upgrade_planner_v1::{
    upgrade_candidate_for_deck_index_v1, upgrade_candidate_score_hint_v1,
};
use crate::content::cards::CardId;
use crate::runtime::combat::CombatCard;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;

#[test]
fn removal_snapshots_preserve_redundant_functional_loss_tier() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.master_deck = vec![
        CombatCard::new(CardId::Flex, 1),
        CombatCard::new(CardId::Flex, 2),
    ];

    let snapshots = deck_removal_target_snapshots_v1(&run);

    assert_eq!(snapshots.len(), 2);
    assert!(snapshots.iter().all(|snapshot| {
        snapshot.target_loss.tier == DeckMutationTargetLossTierV1::RedundantFunctional
    }));
}

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
        DeckMutationCompilerRequestV1::optional_branch_top_k(12),
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
        DeckMutationCompilerRequestV1::optional_branch_top_k(12),
    );

    let commands = decision
        .branch_active_plans
        .iter()
        .map(|plan| (plan.step.command.as_str(), plan.step.cards[0].card))
        .collect::<Vec<_>>();
    assert_eq!(
        commands,
        vec![("select 0", CardId::Strike), ("select 5", CardId::Defend),]
    );
    assert!(!decision.branch_active_plans.iter().any(|plan| plan
        .step
        .cards
        .iter()
        .any(|card| card.card == CardId::Bash)));
}

#[test]
fn compiler_execute_one_selects_evaluated_executable_plan() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = choice(RunPendingChoiceReason::PurgeNonBottled, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerRequestV1::optional_execute_one(),
    );
    let selected = decision.selected_plan.expect("selected plan");

    assert_eq!(selected.step.command, "select 0");
    assert!(selected.allowed_consumers.execute_autopilot);
    assert!(decision
        .candidate_plans
        .iter()
        .any(|candidate| candidate.plan_id == selected.plan_id));
}

#[test]
fn transform_compiler_keeps_low_value_targets_branchable_without_autopilot_execution() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = choice(RunPendingChoiceReason::TransformNonBottled, 1);

    let branch_decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerRequestV1::optional_branch_top_k(12),
    );

    let strike_transform = branch_decision
        .branch_active_plans
        .iter()
        .find(|plan| plan.step.cards[0].card == CardId::Strike)
        .expect("starter Strike transform should stay branchable");
    assert_eq!(
        strike_transform.step.cards[0]
            .transform
            .random_addition_band,
        TransformRandomAdditionBandV1::LikelyBetterThanTarget
    );
    assert_eq!(
        strike_transform.step.cards[0].transform.variance_risk,
        TransformVarianceRiskV1::Medium
    );
    assert!(!strike_transform.allowed_consumers.execute_autopilot);
    assert!(strike_transform
        .reasons
        .iter()
        .any(|reason| reason.contains("transform_random_addition_band")));

    let execute_decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerRequestV1::optional_execute_one(),
    );
    assert!(
        execute_decision.selected_plan.is_none(),
        "transform is a random deck mutation and should not share purge's autopilot gate"
    );
}

#[test]
fn committed_forced_transform_selects_least_bad_legal_target() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = choice(RunPendingChoiceReason::TransformNonBottled, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerRequestV1::committed_forced_execute_one(),
    );
    let selected = decision
        .selected_plan
        .expect("committed forced transform should select a legal target");

    assert_eq!(selected.step.cards[0].card, CardId::Strike);
    assert!(selected
        .reasons
        .iter()
        .any(|reason| reason == "commitment_mode=CommittedForced"));
    assert!(selected
        .risks
        .iter()
        .any(|risk| risk == "committed_forced_selected_non_autopilot_target"));
}

#[test]
fn transform_compiler_marks_functional_targets_as_high_variance() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.add_card_to_deck(CardId::TrueGrit);
    let choice = choice(RunPendingChoiceReason::TransformNonBottled, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerRequestV1::optional_branch_top_k(12),
    );
    let true_grit = decision
        .candidate_plans
        .iter()
        .find(|plan| plan.step.cards[0].card == CardId::TrueGrit)
        .expect("True Grit transform candidate should exist");

    assert_eq!(
        true_grit.step.cards[0].transform.random_addition_band,
        TransformRandomAdditionBandV1::Mixed
    );
    assert_eq!(
        true_grit.step.cards[0].transform.variance_risk,
        TransformVarianceRiskV1::High
    );
    assert!(matches!(
        true_grit.role,
        DeckMutationPlanRoleV1::InspectOnly | DeckMutationPlanRoleV1::RiskyExploration
    ));
    assert!(true_grit
        .risks
        .iter()
        .any(|risk| risk == "transform_variance_risk=High"));
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
        DeckMutationCompilerRequestV1::optional_branch_top_k(12),
    );

    let rendered = render_compiled_deck_mutation_decision_v1(&decision);

    assert!(rendered.contains("Deck mutation evidence:"));
    assert!(rendered.contains("execution: head="));
    assert!(rendered.contains("scheduler: branch_active:"));
    assert!(rendered.contains("candidate_pool: inspect_only:"));
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
        DeckMutationCompilerRequestV1::optional_branch_top_k(16),
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

#[test]
fn upgrade_compiler_scores_targets_from_upgrade_planner() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let bash_index = run_state
        .master_deck
        .iter()
        .position(|card| card.id == CardId::Bash)
        .expect("starter deck should contain Bash");
    let choice = choice(RunPendingChoiceReason::Upgrade, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerRequestV1::optional_branch_top_k(12),
    );
    let bash_plan = decision
        .candidate_plans
        .iter()
        .find(|plan| plan.step.cards[0].deck_index == bash_index)
        .expect("Bash upgrade candidate");
    let upgrade_candidate =
        upgrade_candidate_for_deck_index_v1(&run_state, bash_index).expect("planner candidate");

    assert_eq!(
        bash_plan.score_hint,
        upgrade_candidate_score_hint_v1(&upgrade_candidate),
        "deck mutation upgrade score must be sourced from UpgradePlanner"
    );
    assert!(
        bash_plan
            .reasons
            .iter()
            .any(|reason| reason.starts_with("upgrade_plan: ")),
        "upgrade candidates should expose UpgradePlanner evidence"
    );
}

#[test]
fn duplicate_shop_target_requires_premium_duplicate_role() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");

    assert!(
        best_duplicate_target_for_shop_v1(&run_state).is_none(),
        "starter deck should not make Dolly's Mirror a high-impact purchase"
    );

    run_state.add_card_to_deck(CardId::Offering);
    let target = best_duplicate_target_for_shop_v1(&run_state).expect("premium duplicate target");

    assert_eq!(target.card, CardId::Offering);
    assert_eq!(target.role, DuplicateTargetRoleV1::SetupAccelerator);
    assert!(target.premium);
    assert!(target.priority >= 760);
}

#[test]
fn duplicate_compiler_exposes_target_role_evidence() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.add_card_to_deck(CardId::Offering);
    run_state.add_card_to_deck(CardId::ShrugItOff);
    let choice = choice(RunPendingChoiceReason::Duplicate, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerRequestV1::optional_branch_top_k(4),
    );
    let offering = decision
        .branch_active_plans
        .iter()
        .find(|plan| plan.step.cards[0].card == CardId::Offering)
        .expect("Offering duplicate candidate should remain visible");

    assert!(offering
        .reasons
        .iter()
        .any(|reason| reason == "duplicate_role=setup_accelerator"));
    assert!(offering
        .reasons
        .iter()
        .any(|reason| reason == "duplicate_premium=true"));
    assert!(
        decision
            .candidate_plans
            .iter()
            .find(|plan| plan.step.cards[0].card == CardId::ShrugItOff)
            .is_some_and(|plan| plan.risks.iter().any(|risk| {
                risk == "duplicate_good_reward_card_but_not_premium_mirror_target"
            })),
        "ordinary good reward cards should be explainable as non-premium mirror targets"
    );
}

#[test]
fn bottle_flame_compiler_does_not_rank_starter_strike_above_real_opening_target() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let choice = choice(RunPendingChoiceReason::BottleFlame, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerRequestV1::optional_branch_top_k(3),
    );

    let first = decision
        .branch_active_plans
        .first()
        .expect("bottle target option");
    assert_eq!(first.step.cards[0].card, CardId::Bash);
    assert!(first
        .reasons
        .iter()
        .any(|reason| reason.contains("opening_hand_target_verdict")));
}

#[test]
fn bottle_compiler_keeps_best_bad_target_branchable_when_no_clean_target_exists() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.master_deck.retain(|card| card.id != CardId::Bash);
    let choice = choice(RunPendingChoiceReason::BottleFlame, 1);

    let decision = compile_deck_mutation_decision_v1(
        &run_state,
        &choice,
        DeckMutationCompilerRequestV1::optional_branch_top_k(3),
    );

    assert!(
        !decision.branch_active_plans.is_empty(),
        "forced bottle choice should keep a risky fallback branch when every target is bad"
    );
    assert_eq!(
        decision.branch_active_plans[0].role,
        DeckMutationPlanRoleV1::RiskyExploration
    );
    assert!(decision.branch_active_plans[0]
        .risks
        .iter()
        .any(|risk| risk.contains("opening-hand debt")));
}

fn choice(reason: RunPendingChoiceReason, count: usize) -> RunPendingChoiceState {
    RunPendingChoiceState {
        min_choices: count,
        max_choices: count,
        reason,
        source: crate::state::selection::DomainEventSource::Selection(reason.into()),
        return_state: Box::new(EngineState::MapNavigation),
    }
}
