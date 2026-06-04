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
    } else {
        None
    }
}
