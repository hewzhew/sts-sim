use crate::ai::noncombat_strategy_v1::{StrategyPackageIdV2, StrategyPlanSupportV1};
use crate::state::core::CampfireChoice;

use super::types::{CampfireDecisionContextV1, CampfirePolicyActionV1, CampfirePolicyConfigV1};

pub(crate) fn certified_action(
    context: &CampfireDecisionContextV1,
    config: &CampfirePolicyConfigV1,
) -> Option<CampfirePolicyActionV1> {
    if config.allow_rest_under_recovery_pressure
        && context.current_hp < context.max_hp
        && context
            .candidates
            .iter()
            .any(|candidate| candidate.choice == CampfireChoice::Rest)
        && context
            .strategy
            .support(StrategyPackageIdV2::RecoveryPressure)
            == StrategyPlanSupportV1::Strong
    {
        Some(CampfirePolicyActionV1::Rest {
            confidence: 0.86,
            reason: "RecoveryPressure Strong and Rest is available while HP is missing".to_string(),
        })
    } else if config.allow_clear_core_smith_when_healthy && context.current_hp >= context.max_hp {
        context
            .candidates
            .iter()
            .filter_map(|candidate| match candidate.choice {
                CampfireChoice::Smith(deck_index) => candidate
                    .upgrade_priority
                    .filter(|priority| *priority >= config.clear_core_smith_priority_threshold)
                    .map(|priority| (deck_index, priority)),
                _ => None,
            })
            .max_by_key(|(_, priority)| *priority)
            .map(|(deck_index, priority)| CampfirePolicyActionV1::Smith {
                deck_index,
                confidence: 0.72,
                reason: format!(
                    "HP is full and smith priority {priority} clears clear-core threshold {}",
                    config.clear_core_smith_priority_threshold
                ),
            })
    } else {
        None
    }
}
