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

use super::approvals::legacy_approved_action;
use super::types::{
    candidate_id, CampfireCandidateEvidenceV1, CampfireDecisionContextV1, CampfireDecisionV1,
    CampfirePlanCandidateV1, CampfirePlanRoleV1, CampfirePolicyActionV1, CampfirePolicyClassV1,
    CampfirePolicyConfigV1,
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
    let legacy_action = legacy_approved_action(context, config);
    let mut candidate_plans = campfire_candidate_plans(context, legacy_action.as_ref());
    candidate_plans.push(stop_candidate_plan(context, legacy_action.is_none()));
    candidate_plans.sort_by(compare_campfire_plan_candidates_v1);

    let selected_plan = candidate_plans
        .iter()
        .find(|candidate| candidate.execute_autopilot)
        .cloned()
        .unwrap_or_else(|| CampfirePlanCandidateV1 {
            plan_id: "campfire:stop:fallback".to_string(),
            choice: None,
            action: CampfirePolicyActionV1::Stop {
                reason: "campfire compiler found no executable plan".to_string(),
            },
            role: CampfirePlanRoleV1::StopFallback,
            score_hint: 0,
            confidence: 0.0,
            reasons: vec!["campfire compiler found no executable plan".to_string()],
            execute_autopilot: true,
        });

    CampfireDecisionV1 {
        action: selected_plan.action.clone(),
        selected_plan,
        candidate_plans,
        label_role: "behavior_policy_not_teacher",
        context: context.clone(),
    }
}

fn campfire_candidate_plans(
    context: &CampfireDecisionContextV1,
    legacy_action: Option<&CampfirePolicyActionV1>,
) -> Vec<CampfirePlanCandidateV1> {
    context
        .candidates
        .iter()
        .map(|candidate| campfire_candidate_plan(candidate, legacy_action))
        .collect()
}

fn campfire_candidate_plan(
    candidate: &CampfireCandidateEvidenceV1,
    legacy_action: Option<&CampfirePolicyActionV1>,
) -> CampfirePlanCandidateV1 {
    let candidate_action = action_for_candidate(candidate);
    let legacy_selected =
        legacy_action.is_some_and(|legacy| action_matches(legacy, &candidate_action));
    let action = if legacy_selected {
        legacy_action.cloned().unwrap_or(candidate_action)
    } else {
        candidate_action
    };
    let (confidence, reasons) = if legacy_selected {
        action_confidence_and_reason(&action)
    } else {
        (
            0.0,
            candidate
                .risks
                .iter()
                .cloned()
                .chain(candidate.evidence.iter().take(1).cloned())
                .collect(),
        )
    };

    CampfirePlanCandidateV1 {
        plan_id: candidate.candidate_id.clone(),
        choice: Some(candidate.choice),
        action,
        role: if legacy_selected {
            CampfirePlanRoleV1::PolicyPreferred
        } else {
            CampfirePlanRoleV1::InspectOnly
        },
        score_hint: candidate.upgrade_priority.unwrap_or_default(),
        confidence,
        reasons,
        execute_autopilot: legacy_selected,
    }
}

fn stop_candidate_plan(
    context: &CampfireDecisionContextV1,
    legacy_selected: bool,
) -> CampfirePlanCandidateV1 {
    let reason = stop_reason(context);
    CampfirePlanCandidateV1 {
        plan_id: "campfire:stop".to_string(),
        choice: None,
        action: CampfirePolicyActionV1::Stop {
            reason: reason.clone(),
        },
        role: if legacy_selected {
            CampfirePlanRoleV1::PolicyPreferred
        } else {
            CampfirePlanRoleV1::StopFallback
        },
        score_hint: 0,
        confidence: 0.0,
        reasons: vec![reason],
        execute_autopilot: legacy_selected,
    }
}

fn action_for_candidate(candidate: &CampfireCandidateEvidenceV1) -> CampfirePolicyActionV1 {
    match candidate.choice {
        CampfireChoice::Rest => CampfirePolicyActionV1::Rest {
            confidence: 0.0,
            reason: "campfire candidate plan: rest".to_string(),
        },
        CampfireChoice::Smith(deck_index) => CampfirePolicyActionV1::Smith {
            deck_index,
            confidence: 0.0,
            reason: "campfire candidate plan: smith".to_string(),
        },
        _ => CampfirePolicyActionV1::Stop {
            reason: format!(
                "campfire candidate {:?} is inspect-only in policy v1",
                candidate.choice
            ),
        },
    }
}

fn action_matches(left: &CampfirePolicyActionV1, right: &CampfirePolicyActionV1) -> bool {
    match (left, right) {
        (CampfirePolicyActionV1::Rest { .. }, CampfirePolicyActionV1::Rest { .. }) => true,
        (
            CampfirePolicyActionV1::Smith {
                deck_index: left, ..
            },
            CampfirePolicyActionV1::Smith {
                deck_index: right, ..
            },
        ) => left == right,
        (CampfirePolicyActionV1::Stop { .. }, CampfirePolicyActionV1::Stop { .. }) => true,
        _ => false,
    }
}

fn action_confidence_and_reason(action: &CampfirePolicyActionV1) -> (f32, Vec<String>) {
    match action {
        CampfirePolicyActionV1::Rest { confidence, reason }
        | CampfirePolicyActionV1::Smith {
            confidence, reason, ..
        } => (*confidence, vec![reason.clone()]),
        CampfirePolicyActionV1::Stop { reason } => (0.0, vec![reason.clone()]),
    }
}

fn compare_campfire_plan_candidates_v1(
    left: &CampfirePlanCandidateV1,
    right: &CampfirePlanCandidateV1,
) -> std::cmp::Ordering {
    campfire_plan_role_rank(left.role)
        .cmp(&campfire_plan_role_rank(right.role))
        .then_with(|| right.score_hint.cmp(&left.score_hint))
        .then_with(|| right.confidence.total_cmp(&left.confidence))
        .then_with(|| left.plan_id.cmp(&right.plan_id))
}

fn campfire_plan_role_rank(role: CampfirePlanRoleV1) -> u8 {
    match role {
        CampfirePlanRoleV1::PolicyPreferred => 0,
        CampfirePlanRoleV1::InspectOnly => 1,
        CampfirePlanRoleV1::StopFallback => 2,
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
            if let CampfireChoice::Smith(idx) = choice {
                if let Some(card) = run_state.master_deck.get(idx) {
                    if let Some(tag) =
                        crate::ai::campfire_policy_v1::campfire_smith_upgrade_strategy_tag_v1(
                            card, run_state,
                        )
                    {
                        evidence.push(format!("smith strategy tag is {tag}"));
                    }
                }
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
