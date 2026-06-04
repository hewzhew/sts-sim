use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::state::core::CampfireChoice;
use crate::state::run::RunState;

use super::certificates::certified_action;
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
        .map(|choice| candidate_evidence(choice, &strategy))
        .collect();

    CampfireDecisionContextV1 {
        strategy,
        current_hp: run_state.current_hp,
        max_hp: run_state.max_hp,
        candidates,
    }
}

pub fn plan_campfire_decision_v1(
    context: &CampfireDecisionContextV1,
    config: &CampfirePolicyConfigV1,
) -> CampfireDecisionV1 {
    let action =
        certified_action(context, config).unwrap_or_else(|| CampfirePolicyActionV1::Stop {
            reason: stop_reason(context),
        });

    CampfireDecisionV1 {
        action,
        label_role: "behavior_policy_not_teacher",
        context: context.clone(),
    }
}

fn candidate_evidence(
    choice: CampfireChoice,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> CampfireCandidateEvidenceV1 {
    let class = class_for_choice(choice);
    let support_gate = support_gate_for_choice(choice, strategy);
    let mut evidence = vec![format!("campfire choice is {choice:?}")];
    let mut risks = Vec::new();

    match class {
        CampfirePolicyClassV1::RestRecovery => {
            evidence.push(format!(
                "RecoveryPressure support is {:?}",
                strategy.support(StrategyPackageIdV2::RecoveryPressure)
            ));
        }
        CampfirePolicyClassV1::UpgradeAgency => {
            risks.push("smith choice changes upgrade plan and remains human".to_string());
        }
        CampfirePolicyClassV1::RelicAction => {
            risks.push("campfire relic action is route/deck dependent".to_string());
        }
        CampfirePolicyClassV1::KeyRecall => {
            risks.push("ruby key timing is a high-level route objective".to_string());
        }
        CampfirePolicyClassV1::Unknown => {
            risks.push("campfire policy has no safe certificate for this option".to_string());
        }
    }

    CampfireCandidateEvidenceV1 {
        candidate_id: candidate_id(choice),
        label: label_for_choice(choice),
        choice,
        class,
        support_gate,
        evidence,
        risks,
    }
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
