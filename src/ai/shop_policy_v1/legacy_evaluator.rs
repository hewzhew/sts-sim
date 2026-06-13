use crate::ai::noncombat_strategy_v1::StrategyPlanSupportV1;
use crate::ai::strategic::{CandidateAction, StrategicDecisionTrace};

use super::component_scorer::score_shop_plan_components_v1;
use super::types::{
    ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopPlanCandidateRoleV1, ShopPlanCandidateV1,
    ShopPlanComponentKindV1, ShopPlanComponentV1, ShopPlanEvaluationV1, ShopPlanKindV1,
    ShopPlanSourceV1, ShopPlanStepV1, ShopPolicyClassV1, ShopPolicyConfigV1, ShopPurchaseTargetV1,
};

pub(crate) fn evaluate_shop_plan_candidate_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
    strategic_trace: &StrategicDecisionTrace,
    candidate_plan: &ShopPlanCandidateV1,
) -> ShopPlanEvaluationV1 {
    if candidate_plan.plan.kind == ShopPlanKindV1::Stop
        || candidate_plan.role == ShopPlanCandidateRoleV1::StopFallback
        || candidate_plan.plan.steps.is_empty()
    {
        return attach_components_and_score_v1(
            ShopPlanEvaluationV1::stop(candidate_plan.plan.reason.clone()),
            candidate_plan,
            None,
        );
    }

    if candidate_plan.role == ShopPlanCandidateRoleV1::PortfolioAlternative
        || candidate_plan.plan.source == ShopPlanSourceV1::LegacyShopPortfolioSource
    {
        return attach_components_and_score_v1(
            evaluate_legacy_portfolio_plan_v1(candidate_plan),
            candidate_plan,
            None,
        );
    }

    let Some(candidate_id) = candidate_plan.plan.candidate_ids.first() else {
        return attach_components_and_score_v1(
            ShopPlanEvaluationV1::block(
                candidate_plan.plan.legacy_priority,
                "shop plan has no candidate id",
            ),
            candidate_plan,
            None,
        );
    };
    let Some(candidate) = context
        .candidates
        .iter()
        .find(|candidate| &candidate.candidate_id == candidate_id)
    else {
        return attach_components_and_score_v1(
            ShopPlanEvaluationV1::block(
                candidate_plan.plan.legacy_priority,
                format!("shop plan candidate id {candidate_id} is no longer visible"),
            ),
            candidate_plan,
            None,
        );
    };

    attach_components_and_score_v1(
        evaluate_single_candidate_v1(context, config, strategic_trace, candidate),
        candidate_plan,
        Some(candidate),
    )
}

fn attach_components_and_score_v1(
    mut evaluation: ShopPlanEvaluationV1,
    candidate_plan: &ShopPlanCandidateV1,
    candidate: Option<&ShopCandidateEvidenceV1>,
) -> ShopPlanEvaluationV1 {
    evaluation.components = plan_components_v1(candidate_plan, candidate);
    evaluation.component_score = score_shop_plan_components_v1(&evaluation.components);
    evaluation
}

fn evaluate_single_candidate_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
    strategic_trace: &StrategicDecisionTrace,
    candidate: &ShopCandidateEvidenceV1,
) -> ShopPlanEvaluationV1 {
    match candidate.class {
        ShopPolicyClassV1::CursePurge => evaluate_curse_purge_v1(candidate, config),
        ShopPolicyClassV1::StarterStrikePurge => {
            evaluate_starter_strike_purge_v1(context, candidate, config)
        }
        ShopPolicyClassV1::PurchaseOpportunity => {
            evaluate_purchase_v1(candidate, context, config, strategic_trace)
        }
        ShopPolicyClassV1::Leave => ShopPlanEvaluationV1::stop("legacy shop leave candidate"),
        ShopPolicyClassV1::Unknown => ShopPlanEvaluationV1::block(
            candidate.purchase_priority,
            "legacy evaluator does not approve unknown shop candidate",
        ),
    }
}

fn evaluate_curse_purge_v1(
    candidate: &ShopCandidateEvidenceV1,
    config: &ShopPolicyConfigV1,
) -> ShopPlanEvaluationV1 {
    if !config.allow_curse_purge {
        return ShopPlanEvaluationV1::block(None, "curse purge disabled by shop policy config");
    }
    if candidate.deck_index.is_none() || candidate.card.is_none() {
        return ShopPlanEvaluationV1::block(None, "curse purge candidate lacks deck/card identity");
    }
    ShopPlanEvaluationV1::allow(400, 1000, 0.92, None, "legacy approval: curse cleanup")
}

fn evaluate_purchase_v1(
    candidate: &ShopCandidateEvidenceV1,
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
    strategic_trace: &StrategicDecisionTrace,
) -> ShopPlanEvaluationV1 {
    if candidate.support_gate != StrategyPlanSupportV1::Strong {
        return ShopPlanEvaluationV1::block(
            candidate.purchase_priority,
            format!(
                "purchase support gate {:?} is not Strong",
                candidate.support_gate
            ),
        );
    }
    let Some(target) = candidate.purchase_target else {
        return ShopPlanEvaluationV1::block(candidate.purchase_priority, "purchase target missing");
    };
    let Some(priority) = candidate.purchase_priority else {
        return ShopPlanEvaluationV1::block(None, "purchase legacy priority missing");
    };
    if !purchase_allowed_by_strategic_trace(target, strategic_trace) {
        return ShopPlanEvaluationV1::block(
            Some(priority),
            "strategic trace blocks shop purchase behavior acquisition",
        );
    }

    let threshold = purchase_priority_threshold(target, config);
    if config.allow_high_impact_purchase && priority >= threshold {
        return ShopPlanEvaluationV1::allow(
            300,
            priority,
            0.76,
            Some(priority),
            format!(
                "legacy approval: high-impact shop purchase priority {priority} clears threshold {threshold}; strategic verdict allows purchase"
            ),
        );
    }

    if context.conversion_pressure && priority > 0 {
        return ShopPlanEvaluationV1::allow(
            200,
            priority.saturating_mul(10)
                .saturating_add(purchase_tiebreaker(target)),
            0.64,
            Some(priority),
            format!(
                "legacy approval: shop conversion pressure selected affordable purchase priority {priority}; strategic verdict allows purchase"
            ),
        );
    }

    ShopPlanEvaluationV1::block(
        Some(priority),
        format!("purchase priority {priority} does not clear legacy shop approval gates"),
    )
}

fn evaluate_starter_strike_purge_v1(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
    config: &ShopPolicyConfigV1,
) -> ShopPlanEvaluationV1 {
    if !config.allow_starter_strike_purge_when_core_plan_protected {
        return ShopPlanEvaluationV1::block(
            None,
            "starter strike purge disabled by shop policy config",
        );
    }
    if candidate.support_gate != StrategyPlanSupportV1::Strong {
        return ShopPlanEvaluationV1::block(
            None,
            format!(
                "starter strike purge support gate {:?} is not Strong",
                candidate.support_gate
            ),
        );
    }
    if context.affordable_purchase_exists {
        return ShopPlanEvaluationV1::block(
            None,
            "starter strike purge blocked because affordable purchase competes",
        );
    }
    ShopPlanEvaluationV1::allow(
        100,
        700,
        0.74,
        None,
        "legacy approval: CorePlanProtection Strong and no affordable purchase competes",
    )
}

fn evaluate_legacy_portfolio_plan_v1(candidate_plan: &ShopPlanCandidateV1) -> ShopPlanEvaluationV1 {
    if candidate_plan.plan.steps.is_empty() {
        return ShopPlanEvaluationV1::stop(candidate_plan.plan.reason.clone());
    }
    if !candidate_plan
        .plan
        .steps
        .iter()
        .all(|step| !matches!(step, ShopPlanStepV1::LeaveShop))
    {
        return ShopPlanEvaluationV1::stop(candidate_plan.plan.reason.clone());
    }
    let priority = candidate_plan.plan.legacy_priority.unwrap_or_default();
    if priority <= 0 {
        return ShopPlanEvaluationV1::block(
            candidate_plan.plan.legacy_priority,
            "legacy portfolio plan has no positive priority",
        );
    }
    ShopPlanEvaluationV1::allow(
        150,
        priority,
        0.50,
        Some(priority),
        "legacy portfolio alternative retained for branch exploration",
    )
}

fn purchase_allowed_by_strategic_trace(
    target: ShopPurchaseTargetV1,
    strategic_trace: &StrategicDecisionTrace,
) -> bool {
    let ShopPurchaseTargetV1::Card { index, card } = target else {
        return true;
    };
    let action = CandidateAction::BuyCard {
        shop_index: index,
        card,
        gold: 0,
    };
    strategic_trace
        .compiled_for_action(&action)
        .map(|decision| decision.verdict.allows_behavior_acquisition())
        .unwrap_or(false)
}

fn purchase_tiebreaker(target: ShopPurchaseTargetV1) -> i32 {
    match target {
        ShopPurchaseTargetV1::Relic { .. } => 3,
        ShopPurchaseTargetV1::Potion { .. } => 2,
        ShopPurchaseTargetV1::Card { .. } => 1,
    }
}

fn purchase_priority_threshold(target: ShopPurchaseTargetV1, config: &ShopPolicyConfigV1) -> i32 {
    match target {
        ShopPurchaseTargetV1::Card { .. } => config.high_impact_card_purchase_priority_threshold,
        ShopPurchaseTargetV1::Relic { .. } => config.high_impact_relic_purchase_priority_threshold,
        ShopPurchaseTargetV1::Potion { .. } => {
            config.high_impact_potion_purchase_priority_threshold
        }
    }
}

fn plan_components_v1(
    candidate_plan: &ShopPlanCandidateV1,
    candidate: Option<&ShopCandidateEvidenceV1>,
) -> Vec<ShopPlanComponentV1> {
    let mut components = Vec::new();
    for step in &candidate_plan.plan.steps {
        match *step {
            ShopPlanStepV1::RemoveCard { cost, .. } => {
                if cost > 0 {
                    components.push(component_v1(
                        ShopPlanComponentKindV1::GoldSpend,
                        cost as f32,
                        "shop purge spends gold",
                    ));
                }
                components.push(component_v1(
                    ShopPlanComponentKindV1::DeckCleanup,
                    1.0,
                    "shop purge removes a deck card",
                ));
            }
            ShopPlanStepV1::BuyCard { cost, .. } => {
                if cost > 0 {
                    components.push(component_v1(
                        ShopPlanComponentKindV1::GoldSpend,
                        cost as f32,
                        "card purchase spends gold",
                    ));
                }
                components.push(component_v1(
                    ShopPlanComponentKindV1::DeckBloatCost,
                    1.0,
                    "card purchase adds one deck card",
                ));
            }
            ShopPlanStepV1::BuyRelic { cost, .. } => {
                if cost > 0 {
                    components.push(component_v1(
                        ShopPlanComponentKindV1::GoldSpend,
                        cost as f32,
                        "relic purchase spends gold",
                    ));
                }
                components.push(component_v1(
                    ShopPlanComponentKindV1::RelicValue,
                    1.0,
                    "shop relic adds persistent power",
                ));
            }
            ShopPlanStepV1::BuyPotion { cost, .. } => {
                if cost > 0 {
                    components.push(component_v1(
                        ShopPlanComponentKindV1::GoldSpend,
                        cost as f32,
                        "potion purchase spends gold",
                    ));
                }
                components.push(component_v1(
                    ShopPlanComponentKindV1::PotionFill,
                    1.0,
                    "shop potion fills a potion slot",
                ));
            }
            ShopPlanStepV1::LeaveShop => components.push(component_v1(
                ShopPlanComponentKindV1::StopReason,
                1.0,
                "leave shop is a non-purchase plan",
            )),
        }
    }

    if let Some(priority) = candidate_plan.plan.legacy_priority {
        components.push(component_v1(
            ShopPlanComponentKindV1::LegacyEstimate,
            priority as f32,
            "legacy purchase priority retained as an estimate component",
        ));
    }
    if candidate_plan.role == ShopPlanCandidateRoleV1::PortfolioAlternative {
        components.push(component_v1(
            ShopPlanComponentKindV1::BranchExploration,
            1.0,
            "portfolio plan is retained for branch exploration",
        ));
    }
    if candidate.is_some_and(|candidate| {
        candidate
            .evidence
            .iter()
            .any(|evidence| evidence.contains("answer"))
    }) {
        components.push(component_v1(
            ShopPlanComponentKindV1::BossAnswer,
            1.0,
            "candidate evidence marks this as a combat answer",
        ));
    }
    if components.is_empty() {
        components.push(component_v1(
            ShopPlanComponentKindV1::StopReason,
            1.0,
            "shop plan has no executable purchase component",
        ));
    }
    components
}

fn component_v1(
    kind: ShopPlanComponentKindV1,
    amount: f32,
    reason: &'static str,
) -> ShopPlanComponentV1 {
    ShopPlanComponentV1 {
        kind,
        amount,
        reason: reason.to_string(),
    }
}
