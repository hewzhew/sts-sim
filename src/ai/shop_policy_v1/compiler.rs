use super::approvals::approved_action;
use super::policy::stop_reason;
use super::portfolio::legacy_shop_portfolio_plans_v1;
use super::types::{
    purge_candidate_id, CompiledShopDecisionV1, ShopCandidateEvidenceV1, ShopCompileModeV1,
    ShopDecisionContextV1, ShopDecisionSourceV1, ShopPlanCandidateRoleV1, ShopPlanCandidateV1,
    ShopPlanKindV1, ShopPlanSourceV1, ShopPlanStepV1, ShopPlanV1, ShopPolicyActionV1,
    ShopPolicyClassV1, ShopPolicyConfigV1, ShopPurchaseTargetV1,
};

pub fn compile_shop_decision_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
    mode: ShopCompileModeV1,
) -> CompiledShopDecisionV1 {
    let strategic_trace = crate::ai::strategic::strategic_trace_for_shop(context);
    let action = approved_action(context, config, &strategic_trace).unwrap_or_else(|| {
        ShopPolicyActionV1::Stop {
            reason: stop_reason(context),
        }
    });
    let mut candidate_plans = enumerate_single_action_plan_candidates_v1(context);
    candidate_plans.push(stop_candidate_plan_v1(stop_reason(context)));
    let alternatives =
        match mode {
            ShopCompileModeV1::ExecuteOne => Vec::new(),
            ShopCompileModeV1::BranchTopK { max_plans } => {
                let alternatives = legacy_shop_portfolio_plans_v1(context, max_plans);
                candidate_plans.extend(alternatives.iter().cloned().map(|plan| {
                    ShopPlanCandidateV1 {
                        plan,
                        role: ShopPlanCandidateRoleV1::PortfolioAlternative,
                    }
                }));
                alternatives
            }
        };
    let selected_plan = select_legacy_approved_plan_v1(context, &action, &candidate_plans);

    CompiledShopDecisionV1 {
        selected_plan,
        alternatives,
        candidate_plans,
        strategic_trace,
        source: ShopDecisionSourceV1::LegacyWrapped,
    }
}

pub fn shop_policy_action_from_plan_v1(plan: &ShopPlanV1) -> Option<ShopPolicyActionV1> {
    let confidence = plan.legacy_confidence.unwrap_or(0.0);
    let reason = plan.reason.clone();
    match plan.steps.first()? {
        ShopPlanStepV1::RemoveCard {
            deck_index, card, ..
        } => Some(ShopPolicyActionV1::Purge {
            deck_index: *deck_index,
            card: *card,
            confidence,
            reason,
        }),
        ShopPlanStepV1::BuyCard { index, card, .. } => Some(ShopPolicyActionV1::Purchase {
            target: ShopPurchaseTargetV1::Card {
                index: *index,
                card: *card,
            },
            confidence,
            reason,
        }),
        ShopPlanStepV1::BuyRelic { index, relic, .. } => Some(ShopPolicyActionV1::Purchase {
            target: ShopPurchaseTargetV1::Relic {
                index: *index,
                relic: *relic,
            },
            confidence,
            reason,
        }),
        ShopPlanStepV1::BuyPotion { index, potion, .. } => Some(ShopPolicyActionV1::Purchase {
            target: ShopPurchaseTargetV1::Potion {
                index: *index,
                potion: *potion,
            },
            confidence,
            reason,
        }),
        ShopPlanStepV1::LeaveShop => None,
    }
}

fn select_legacy_approved_plan_v1(
    context: &ShopDecisionContextV1,
    action: &ShopPolicyActionV1,
    candidates: &[ShopPlanCandidateV1],
) -> ShopPlanV1 {
    match action {
        ShopPolicyActionV1::Purge {
            deck_index,
            confidence,
            reason,
            ..
        } => context
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == purge_candidate_id(*deck_index))
            .and_then(|candidate| find_candidate_plan_by_id(candidates, &candidate.candidate_id))
            .map(|mut plan| {
                plan.legacy_confidence = Some(*confidence);
                plan.reason = reason.clone();
                plan
            })
            .unwrap_or_else(|| stop_candidate_plan_v1(reason.clone()).plan),
        ShopPolicyActionV1::Purchase {
            target,
            confidence,
            reason,
        } => context
            .candidates
            .iter()
            .find(|candidate| {
                candidate
                    .purchase_target
                    .is_some_and(|candidate_target| candidate_target == *target)
            })
            .and_then(|candidate| find_candidate_plan_by_id(candidates, &candidate.candidate_id))
            .map(|mut plan| {
                plan.legacy_confidence = Some(*confidence);
                plan.reason = reason.clone();
                plan
            })
            .unwrap_or_else(|| stop_candidate_plan_v1(reason.clone()).plan),
        ShopPolicyActionV1::Stop { reason } => candidates
            .iter()
            .find(|candidate| candidate.role == ShopPlanCandidateRoleV1::StopFallback)
            .map(|candidate| {
                let mut plan = candidate.plan.clone();
                plan.reason = reason.clone();
                plan
            })
            .unwrap_or_else(|| stop_candidate_plan_v1(reason.clone()).plan),
    }
}

fn enumerate_single_action_plan_candidates_v1(
    context: &ShopDecisionContextV1,
) -> Vec<ShopPlanCandidateV1> {
    context
        .candidates
        .iter()
        .filter_map(|candidate| {
            single_candidate_plan_v1(candidate, ShopPlanSourceV1::LegacyWrapped).map(|plan| {
                ShopPlanCandidateV1 {
                    plan,
                    role: ShopPlanCandidateRoleV1::SingleAction,
                }
            })
        })
        .collect()
}

fn find_candidate_plan_by_id(
    candidates: &[ShopPlanCandidateV1],
    candidate_id: &str,
) -> Option<ShopPlanV1> {
    candidates
        .iter()
        .find(|candidate| {
            candidate
                .plan
                .candidate_ids
                .iter()
                .any(|id| id == candidate_id)
        })
        .map(|candidate| candidate.plan.clone())
}

pub(super) fn single_candidate_plan_v1(
    candidate: &ShopCandidateEvidenceV1,
    source: ShopPlanSourceV1,
) -> Option<ShopPlanV1> {
    let step = match (
        candidate.deck_index,
        candidate.card,
        candidate.purchase_target,
        candidate.class,
    ) {
        (Some(deck_index), Some(card), None, _) => ShopPlanStepV1::RemoveCard {
            deck_index,
            card,
            cost: candidate.gold_cost.unwrap_or_default(),
        },
        (_, _, Some(ShopPurchaseTargetV1::Card { index, card }), _) => ShopPlanStepV1::BuyCard {
            index,
            card,
            cost: candidate.gold_cost.unwrap_or_default(),
        },
        (_, _, Some(ShopPurchaseTargetV1::Relic { index, relic }), _) => ShopPlanStepV1::BuyRelic {
            index,
            relic,
            cost: candidate.gold_cost.unwrap_or_default(),
        },
        (_, _, Some(ShopPurchaseTargetV1::Potion { index, potion }), _) => {
            ShopPlanStepV1::BuyPotion {
                index,
                potion,
                cost: candidate.gold_cost.unwrap_or_default(),
            }
        }
        (_, _, None, ShopPolicyClassV1::Leave) => ShopPlanStepV1::LeaveShop,
        _ => return None,
    };
    let total_gold_spent = match step {
        ShopPlanStepV1::LeaveShop => 0,
        _ => candidate.gold_cost.unwrap_or_default(),
    };
    Some(ShopPlanV1 {
        plan_id: format!("legacy:{}", candidate.candidate_id),
        label: candidate.label.clone(),
        kind: ShopPlanKindV1::Execute,
        steps: vec![step],
        total_gold_spent,
        candidate_ids: vec![candidate.candidate_id.clone()],
        source,
        legacy_priority: candidate.purchase_priority,
        legacy_confidence: None,
        suppressed_count: 0,
        reason: format!("legacy shop plan from {}", candidate.candidate_id),
    })
}

fn stop_candidate_plan_v1(reason: String) -> ShopPlanCandidateV1 {
    ShopPlanCandidateV1 {
        plan: ShopPlanV1 {
            plan_id: "legacy:stop".to_string(),
            label: "stop shop automation".to_string(),
            kind: ShopPlanKindV1::Stop,
            steps: Vec::new(),
            total_gold_spent: 0,
            candidate_ids: Vec::new(),
            source: ShopPlanSourceV1::LegacyWrapped,
            legacy_priority: None,
            legacy_confidence: None,
            suppressed_count: 0,
            reason,
        },
        role: ShopPlanCandidateRoleV1::StopFallback,
    }
}
