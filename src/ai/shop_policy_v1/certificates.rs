use crate::ai::noncombat_strategy_v1::StrategyPlanSupportV1;

use super::types::{
    ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopPolicyActionV1, ShopPolicyClassV1,
    ShopPolicyConfigV1,
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
