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
    if rest_is_routine_exit_allowed(context, candidate) {
        return Some(CampfirePolicyActionV1::Rest {
            confidence: 0.90,
            reason: "Rest is the only available campfire action and functions as the campfire exit"
                .to_string(),
        });
    }

    if rest_is_autopilot_allowed(context, config) {
        return matches!(candidate.choice, CampfireChoice::Rest).then(|| {
            CampfirePolicyActionV1::Rest {
                confidence: 0.86,
                reason: "RecoveryPressure Strong and Rest is available while HP is missing"
                    .to_string(),
            }
        });
    }

    let _ = (config, candidate);
    None
}

fn rest_is_routine_exit_allowed(
    context: &CampfireDecisionContextV1,
    candidate: &CampfireCandidateEvidenceV1,
) -> bool {
    candidate.choice == CampfireChoice::Rest
        && context.current_hp >= context.max_hp
        && context.candidates.iter().all(|other| {
            matches!(other.choice, CampfireChoice::Rest)
                || other.deck_mutation_execute_allowed == Some(false)
        })
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
