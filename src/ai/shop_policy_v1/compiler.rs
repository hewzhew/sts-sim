use super::approvals::approved_action;
use super::portfolio::legacy_shop_portfolio_plans_v1;
use super::policy::stop_reason;
use super::types::{
    purge_candidate_id, CompiledShopDecisionV1, ShopCandidateEvidenceV1, ShopCompileModeV1,
    ShopDecisionContextV1, ShopDecisionSourceV1, ShopPlanKindV1, ShopPlanSourceV1,
    ShopPlanStepV1, ShopPlanV1, ShopPolicyActionV1, ShopPolicyClassV1, ShopPolicyConfigV1,
    ShopPurchaseTargetV1,
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
    let selected_plan =
        shop_plan_from_action_v1(context, &action, ShopPlanSourceV1::LegacyWrapped);
    let alternatives = match mode {
        ShopCompileModeV1::ExecuteOne => Vec::new(),
        ShopCompileModeV1::BranchTopK { max_plans } => {
            legacy_shop_portfolio_plans_v1(context, max_plans)
        }
    };

    CompiledShopDecisionV1 {
        selected_plan,
        alternatives,
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

fn shop_plan_from_action_v1(
    context: &ShopDecisionContextV1,
    action: &ShopPolicyActionV1,
    source: ShopPlanSourceV1,
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
            .and_then(|candidate| single_candidate_plan_v1(candidate, source))
            .map(|mut plan| {
                plan.legacy_confidence = Some(*confidence);
                plan.reason = reason.clone();
                plan
            })
            .unwrap_or_else(|| stop_plan_v1(reason.clone(), source)),
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
            .and_then(|candidate| single_candidate_plan_v1(candidate, source))
            .map(|mut plan| {
                plan.legacy_confidence = Some(*confidence);
                plan.reason = reason.clone();
                plan
            })
            .unwrap_or_else(|| stop_plan_v1(reason.clone(), source)),
        ShopPolicyActionV1::Stop { reason } => stop_plan_v1(reason.clone(), source),
    }
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
        (_, _, Some(ShopPurchaseTargetV1::Relic { index, relic }), _) => {
            ShopPlanStepV1::BuyRelic {
                index,
                relic,
                cost: candidate.gold_cost.unwrap_or_default(),
            }
        }
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

fn stop_plan_v1(reason: String, source: ShopPlanSourceV1) -> ShopPlanV1 {
    ShopPlanV1 {
        plan_id: "legacy:stop".to_string(),
        label: "stop shop automation".to_string(),
        kind: ShopPlanKindV1::Stop,
        steps: Vec::new(),
        total_gold_spent: 0,
        candidate_ids: Vec::new(),
        source,
        legacy_priority: None,
        legacy_confidence: None,
        suppressed_count: 0,
        reason,
    }
}
