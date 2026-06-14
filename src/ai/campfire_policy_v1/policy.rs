use crate::ai::deck_mutation_compiler_v1::{
    compile_deck_mutation_decision_v1, DeckMutationCompilerModeV1, DeckMutationKindV1,
    DeckMutationPlanCandidateV1, DeckMutationPlanRoleV1,
};
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::state::core::{
    CampfireChoice, EngineState, RunPendingChoiceReason, RunPendingChoiceState,
};
use crate::state::run::RunState;

use super::approvals::approved_action;
use super::types::{
    candidate_id, CampfireCandidateEvidenceV1, CampfireDecisionContextV1, CampfireDecisionV1,
    CampfirePolicyActionV1, CampfirePolicyClassV1, CampfirePolicyConfigV1,
};

pub fn build_campfire_decision_context_v1(
    run_state: &RunState,
    available_choices: Vec<CampfireChoice>,
) -> CampfireDecisionContextV1 {
    let strategy = build_run_strategy_snapshot_from_run_state_v2(run_state);
    let candidates = available_choices
        .into_iter()
        .flat_map(|choice| expand_choice_targets(run_state, choice))
        .map(|choice| candidate_evidence(choice, &strategy, run_state))
        .collect();

    CampfireDecisionContextV1 {
        strategy,
        current_hp: run_state.current_hp,
        max_hp: run_state.max_hp,
        candidates,
    }
}

#[derive(Clone, Debug)]
struct ExpandedCampfireChoice {
    choice: CampfireChoice,
    deck_mutation_plan: Option<DeckMutationPlanCandidateV1>,
}

fn expand_choice_targets(
    run_state: &RunState,
    choice: CampfireChoice,
) -> Vec<ExpandedCampfireChoice> {
    match choice {
        CampfireChoice::Smith(_) => deck_mutation_campfire_targets(
            run_state,
            RunPendingChoiceReason::Upgrade,
            DeckMutationKindV1::Upgrade,
            CampfireChoice::Smith,
        ),
        CampfireChoice::Toke(_) => deck_mutation_campfire_targets(
            run_state,
            RunPendingChoiceReason::PurgeNonBottled,
            DeckMutationKindV1::Remove,
            CampfireChoice::Toke,
        ),
        _ => vec![ExpandedCampfireChoice {
            choice,
            deck_mutation_plan: None,
        }],
    }
}

fn deck_mutation_campfire_targets(
    run_state: &RunState,
    reason: RunPendingChoiceReason,
    expected_kind: DeckMutationKindV1,
    choice_for_index: fn(usize) -> CampfireChoice,
) -> Vec<ExpandedCampfireChoice> {
    let choice = RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason,
        return_state: Box::new(EngineState::Campfire),
    };
    let decision = compile_deck_mutation_decision_v1(
        run_state,
        &choice,
        DeckMutationCompilerModeV1::BranchTopK {
            max_active: usize::MAX,
        },
    );

    decision
        .candidate_plans
        .into_iter()
        .filter_map(|plan| {
            if plan.step.kind != expected_kind || plan.step.deck_indices.len() != 1 {
                return None;
            }
            let deck_index = *plan.step.deck_indices.first()?;
            Some(ExpandedCampfireChoice {
                choice: choice_for_index(deck_index),
                deck_mutation_plan: Some(plan),
            })
        })
        .collect()
}

pub fn plan_campfire_decision_v1(
    context: &CampfireDecisionContextV1,
    config: &CampfirePolicyConfigV1,
) -> CampfireDecisionV1 {
    let action = approved_action(context, config).unwrap_or_else(|| CampfirePolicyActionV1::Stop {
        reason: stop_reason(context),
    });

    CampfireDecisionV1 {
        action,
        label_role: "behavior_policy_not_teacher",
        context: context.clone(),
    }
}

fn candidate_evidence(
    expanded: ExpandedCampfireChoice,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
    run_state: &RunState,
) -> CampfireCandidateEvidenceV1 {
    let choice = expanded.choice;
    let class = class_for_choice(choice);
    let support_gate = support_gate_for_choice(choice, strategy);
    let mut evidence = vec![format!("campfire choice is {choice:?}")];
    let mut risks = Vec::new();
    let upgrade_priority = match choice {
        CampfireChoice::Smith(idx) => run_state.master_deck.get(idx).map(|card| {
            crate::ai::campfire_policy_v1::campfire_smith_upgrade_priority_v1(card, run_state)
        }),
        _ => None,
    };
    if let Some(plan) = &expanded.deck_mutation_plan {
        evidence.extend(deck_mutation_plan_evidence(plan));
        risks.extend(plan.risks.iter().cloned());
        if matches!(
            plan.role,
            DeckMutationPlanRoleV1::InspectOnly | DeckMutationPlanRoleV1::Blocked
        ) {
            risks.push(format!(
                "deck mutation compiler did not approve this target for automatic execution: {:?}",
                plan.role
            ));
        }
    }

    match class {
        CampfirePolicyClassV1::RestRecovery => {
            evidence.push(format!(
                "RecoveryPressure support is {:?}",
                strategy.support(StrategyPackageIdV2::RecoveryPressure)
            ));
        }
        CampfirePolicyClassV1::UpgradeAgency => {
            if let Some(priority) = upgrade_priority {
                evidence.push(format!("smith upgrade priority is {priority}"));
            }
            risks.push("smith choice changes upgrade plan unless priority clears gate".to_string());
        }
        CampfirePolicyClassV1::RelicAction => {
            risks.push("campfire relic action is route/deck dependent".to_string());
        }
        CampfirePolicyClassV1::KeyRecall => {
            risks.push("ruby key timing is a high-level route objective".to_string());
        }
        CampfirePolicyClassV1::Unknown => {
            risks.push("campfire policy has no safe approval for this option".to_string());
        }
    }

    CampfireCandidateEvidenceV1 {
        candidate_id: candidate_id(choice),
        label: label_for_choice(choice),
        choice,
        class,
        upgrade_priority,
        support_gate,
        evidence,
        risks,
    }
}

fn deck_mutation_plan_evidence(plan: &DeckMutationPlanCandidateV1) -> Vec<String> {
    let mut evidence = vec![
        format!("DeckMutationCompilerV1 plan_id={}", plan.plan_id),
        format!("deck mutation role={:?}", plan.role),
        format!(
            "deck mutation allowed execute={} branch={} inspect={}",
            plan.allowed_consumers.execute_autopilot,
            plan.allowed_consumers.branch_active,
            plan.allowed_consumers.inspect
        ),
        format!("deck mutation effect={}", plan.step.effect_label),
        format!(
            "deck mutation representative_count={} suppressed_count={}",
            plan.representative_count, plan.suppressed_count
        ),
    ];
    evidence.extend(plan.reasons.iter().cloned());
    evidence
}

fn class_for_choice(choice: CampfireChoice) -> CampfirePolicyClassV1 {
    match choice {
        CampfireChoice::Rest => CampfirePolicyClassV1::RestRecovery,
        CampfireChoice::Smith(_) => CampfirePolicyClassV1::UpgradeAgency,
        CampfireChoice::Dig | CampfireChoice::Lift | CampfireChoice::Toke(_) => {
            CampfirePolicyClassV1::RelicAction
        }
        CampfireChoice::Recall => CampfirePolicyClassV1::KeyRecall,
    }
}

fn support_gate_for_choice(
    choice: CampfireChoice,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> StrategyPlanSupportV1 {
    match choice {
        CampfireChoice::Rest => strategy.support(StrategyPackageIdV2::RecoveryPressure),
        _ => StrategyPlanSupportV1::Blocked,
    }
}

fn label_for_choice(choice: CampfireChoice) -> String {
    match choice {
        CampfireChoice::Rest => "Rest".to_string(),
        CampfireChoice::Smith(idx) => format!("Smith card {idx}"),
        CampfireChoice::Dig => "Dig".to_string(),
        CampfireChoice::Lift => "Lift".to_string(),
        CampfireChoice::Toke(idx) => format!("Toke card {idx}"),
        CampfireChoice::Recall => "Recall ruby key".to_string(),
    }
}

fn stop_reason(context: &CampfireDecisionContextV1) -> String {
    if context.current_hp >= context.max_hp {
        return "campfire policy stopped because HP is full".to_string();
    }
    let recovery = context
        .strategy
        .support(StrategyPackageIdV2::RecoveryPressure);
    format!("campfire policy stopped because RecoveryPressure is {recovery:?}")
}
