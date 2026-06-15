use crate::ai::noncombat_strategy_v1::{StrategyPackageIdV2, StrategyPlanSupportV1};
use crate::state::core::CampfireChoice;

use super::types::{
    CampfireCandidateEvidenceV1, CampfireDecisionContextV1, CampfirePolicyActionV1,
    CampfirePolicyConfigV1,
};

pub(crate) fn candidate_autopilot_action(
    context: &CampfireDecisionContextV1,
    config: &CampfirePolicyConfigV1,
    candidate: &CampfireCandidateEvidenceV1,
) -> Option<CampfirePolicyActionV1> {
    if rest_is_autopilot_allowed(context, config) {
        return matches!(candidate.choice, CampfireChoice::Rest).then(|| {
            CampfirePolicyActionV1::Rest {
                confidence: 0.86,
                reason: "RecoveryPressure Strong and Rest is available while HP is missing"
                    .to_string(),
            }
        });
    }

    smith_autopilot_window(context, config).and_then(|window| {
        autopilot_smith_candidate(context, candidate, window.threshold).map(
            |(deck_index, priority)| CampfirePolicyActionV1::Smith {
                deck_index,
                confidence: window.confidence,
                reason: format!(
                    "{} and smith priority {priority} clears threshold {}",
                    window.reason_prefix, window.threshold
                ),
            },
        )
    })
}

#[derive(Clone, Copy)]
struct SmithAutopilotWindow {
    threshold: i32,
    confidence: f32,
    reason_prefix: &'static str,
}

fn rest_is_autopilot_allowed(
    context: &CampfireDecisionContextV1,
    config: &CampfirePolicyConfigV1,
) -> bool {
    config.allow_rest_under_recovery_pressure
        && context.current_hp < context.max_hp
        && context
            .candidates
            .iter()
            .any(|candidate| candidate.choice == CampfireChoice::Rest)
        && context
            .strategy
            .support(StrategyPackageIdV2::RecoveryPressure)
            == StrategyPlanSupportV1::Strong
}

fn smith_autopilot_window(
    context: &CampfireDecisionContextV1,
    config: &CampfirePolicyConfigV1,
) -> Option<SmithAutopilotWindow> {
    if config.allow_combat_patch_smith_when_safe
        && hp_percent_at_least(context, config.combat_patch_smith_min_hp_percent)
        && matches!(
            context
                .strategy
                .support(StrategyPackageIdV2::CombatPatchWindow),
            StrategyPlanSupportV1::Strong | StrategyPlanSupportV1::Plausible
        )
    {
        Some(SmithAutopilotWindow {
            threshold: config.combat_patch_smith_priority_threshold,
            confidence: 0.70,
            reason_prefix: "CombatPatchWindow active",
        })
    } else if config.allow_clear_core_smith_when_healthy && context.current_hp >= context.max_hp {
        Some(SmithAutopilotWindow {
            threshold: config.clear_core_smith_priority_threshold,
            confidence: 0.72,
            reason_prefix: "HP is full",
        })
    } else {
        None
    }
}

fn autopilot_smith_candidate(
    context: &CampfireDecisionContextV1,
    candidate: &CampfireCandidateEvidenceV1,
    threshold: i32,
) -> Option<(usize, i32)> {
    let CampfireChoice::Smith(deck_index) = candidate.choice else {
        return None;
    };
    if candidate.deck_mutation_execute_allowed == Some(false) {
        return None;
    }
    let priority = candidate.upgrade_priority?;
    if priority < threshold {
        return None;
    }
    let best = best_executable_smith(context, threshold)?;
    (best == (deck_index, priority)).then_some(best)
}

fn best_executable_smith(
    context: &CampfireDecisionContextV1,
    threshold: i32,
) -> Option<(usize, i32)> {
    context
        .candidates
        .iter()
        .filter_map(|candidate| match candidate.choice {
            CampfireChoice::Smith(deck_index)
                if candidate.deck_mutation_execute_allowed != Some(false) =>
            {
                candidate
                    .upgrade_priority
                    .filter(|priority| *priority >= threshold)
                    .map(|priority| (deck_index, priority))
            }
            _ => None,
        })
        .max_by_key(|(_, priority)| *priority)
}

fn hp_percent_at_least(context: &CampfireDecisionContextV1, threshold: i32) -> bool {
    context.max_hp > 0 && context.current_hp.saturating_mul(100) >= context.max_hp * threshold
}
