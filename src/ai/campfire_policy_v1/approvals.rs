use crate::ai::noncombat_strategy_v1::{StrategyPackageIdV2, StrategyPlanSupportV1};
use crate::state::core::CampfireChoice;

use super::types::{CampfireDecisionContextV1, CampfirePolicyActionV1, CampfirePolicyConfigV1};

pub(crate) fn approved_action(
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
    } else if config.allow_combat_patch_smith_when_safe
        && hp_percent_at_least(context, config.combat_patch_smith_min_hp_percent)
        && matches!(
            context
                .strategy
                .support(StrategyPackageIdV2::CombatPatchWindow),
            StrategyPlanSupportV1::Strong | StrategyPlanSupportV1::Plausible
        )
    {
        best_smith(context, config.combat_patch_smith_priority_threshold).map(
            |(deck_index, priority)| CampfirePolicyActionV1::Smith {
                deck_index,
                confidence: 0.70,
                reason: format!(
                    "CombatPatchWindow active and smith priority {priority} clears prep threshold {}",
                    config.combat_patch_smith_priority_threshold
                ),
            },
        )
    } else if config.allow_clear_core_smith_when_healthy && context.current_hp >= context.max_hp {
        best_smith(context, config.clear_core_smith_priority_threshold).map(
            |(deck_index, priority)| CampfirePolicyActionV1::Smith {
                deck_index,
                confidence: 0.72,
                reason: format!(
                    "HP is full and smith priority {priority} clears clear-core threshold {}",
                    config.clear_core_smith_priority_threshold
                ),
            },
        )
    } else {
        None
    }
}

fn best_smith(context: &CampfireDecisionContextV1, threshold: i32) -> Option<(usize, i32)> {
    context
        .candidates
        .iter()
        .filter_map(|candidate| match candidate.choice {
            CampfireChoice::Smith(deck_index) => candidate
                .upgrade_priority
                .filter(|priority| *priority >= threshold)
                .map(|priority| (deck_index, priority)),
            _ => None,
        })
        .max_by_key(|(_, priority)| *priority)
}

fn hp_percent_at_least(context: &CampfireDecisionContextV1, threshold: i32) -> bool {
    context.max_hp > 0 && context.current_hp.saturating_mul(100) >= context.max_hp * threshold
}
