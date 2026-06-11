use crate::ai::noncombat_strategy_v1::StrategyPlanSupportV1;

use super::types::{
    ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopPolicyActionV1, ShopPolicyClassV1,
    ShopPolicyConfigV1, ShopPurchaseTargetV1,
};

pub(crate) fn certified_action(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
) -> Option<ShopPolicyActionV1> {
    if config.allow_curse_purge {
        if let Some(action) = context
            .candidates
            .iter()
            .find(|candidate| candidate.class == ShopPolicyClassV1::CursePurge)
            .and_then(|candidate| purge_action(candidate, 0.92, "curse cleanup"))
        {
            return Some(action);
        }
    }

    if config.allow_high_impact_purchase {
        if let Some(action) = context
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.class == ShopPolicyClassV1::PurchaseOpportunity
                    && candidate.support_gate == StrategyPlanSupportV1::Strong
            })
            .filter_map(|candidate| {
                let target = candidate.purchase_target?;
                let priority = candidate.purchase_priority?;
                let threshold = purchase_priority_threshold(target, config);
                (priority >= threshold).then_some((candidate, priority, threshold))
            })
            .max_by_key(|(_, priority, _)| *priority)
            .and_then(|(candidate, priority, threshold)| {
                purchase_action(
                    candidate,
                    0.76,
                    format!(
                        "high-impact shop purchase priority {priority} clears threshold {threshold}"
                    ),
                )
            })
        {
            return Some(action);
        }
    }

    if !config.allow_starter_strike_purge_when_core_plan_protected {
        return None;
    }

    context
        .candidates
        .iter()
        .find(|candidate| {
            candidate.class == ShopPolicyClassV1::StarterStrikePurge
                && candidate.support_gate == StrategyPlanSupportV1::Strong
                && !context.affordable_purchase_exists
        })
        .and_then(|candidate| {
            purge_action(
                candidate,
                0.74,
                "CorePlanProtection Strong and no affordable purchase competes",
            )
        })
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

fn purge_action(
    candidate: &ShopCandidateEvidenceV1,
    confidence: f32,
    reason: &'static str,
) -> Option<ShopPolicyActionV1> {
    Some(ShopPolicyActionV1::Purge {
        deck_index: candidate.deck_index?,
        card: candidate.card?,
        confidence,
        reason: reason.to_string(),
    })
}

fn purchase_action(
    candidate: &ShopCandidateEvidenceV1,
    confidence: f32,
    reason: String,
) -> Option<ShopPolicyActionV1> {
    Some(ShopPolicyActionV1::Purchase {
        target: candidate.purchase_target?,
        confidence,
        reason,
    })
}
