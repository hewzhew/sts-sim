use crate::ai::noncombat_strategy_v1::{StrategyPackageIdV2, StrategyPlanSupportV1};
use crate::ai::upgrade_planner_v1::RestVsSmithVerdictV1;
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
            let reason = if context.rest_vs_smith.verdict == RestVsSmithVerdictV1::RestFavored {
                format!(
                    "Rest favored by rest-vs-smith plan: effective_heal={} hp={}/{}",
                    context.rest_vs_smith.effective_rest_heal, context.current_hp, context.max_hp
                )
            } else if imminent_boss_recovery_is_required(context, config) {
                format!(
                    "Known boss is next and recovery is required: effective_heal={} hp={}/{}",
                    context.rest_vs_smith.effective_rest_heal, context.current_hp, context.max_hp
                )
            } else if ordinary_smith_health_floor_requires_rest(context, config) {
                format!(
                    "HP is below the ordinary Smith health floor: effective_heal={} hp={}/{} floor={}pct",
                    context.rest_vs_smith.effective_rest_heal,
                    context.current_hp,
                    context.max_hp,
                    config.ordinary_smith_min_hp_percent
                )
            } else {
                "RecoveryPressure Strong and Rest is available while HP is missing".to_string()
            };
            CampfirePolicyActionV1::Rest {
                confidence: 0.86,
                reason,
            }
        });
    }

    if smith_is_autopilot_allowed(context, config, candidate) {
        if let CampfireChoice::Smith(deck_index) = candidate.choice {
            return Some(CampfirePolicyActionV1::Smith {
                deck_index,
                confidence: 0.78,
                reason: format!(
                    "Smith clears campfire upgrade gate: tag={} score={} hp={}/{}",
                    candidate.strategy_tag.as_deref().unwrap_or("none"),
                    candidate.upgrade_plan_score_hint.unwrap_or_default(),
                    context.current_hp,
                    context.max_hp
                ),
            });
        }
    }

    None
}

fn smith_is_autopilot_allowed(
    context: &CampfireDecisionContextV1,
    config: &CampfirePolicyConfigV1,
    candidate: &CampfireCandidateEvidenceV1,
) -> bool {
    if !matches!(candidate.choice, CampfireChoice::Smith(_)) {
        return false;
    }
    if candidate.deck_mutation_execute_allowed == Some(false) {
        return false;
    }
    if ordinary_smith_health_floor_requires_rest(context, config) {
        return false;
    }
    if context
        .strategy
        .support(StrategyPackageIdV2::RecoveryPressure)
        == StrategyPlanSupportV1::Strong
    {
        return false;
    }
    if context.rest_vs_smith.verdict == RestVsSmithVerdictV1::RestFavored {
        return false;
    }
    if imminent_boss_recovery_is_required(context, config) {
        return false;
    }

    let score = candidate.upgrade_plan_score_hint.unwrap_or_default();
    let tag = candidate.strategy_tag.as_deref().unwrap_or_default();
    if config.allow_clear_core_smith_when_healthy
        && score >= config.clear_core_smith_priority_threshold
        && clear_core_upgrade_tag(tag)
    {
        return true;
    }

    let hp_percent = hp_percent(context.current_hp, context.max_hp);
    config.allow_combat_patch_smith_when_safe
        && hp_percent >= config.combat_patch_smith_min_hp_percent
        && score >= config.combat_patch_smith_priority_threshold
        && combat_patch_upgrade_tag(tag)
}

fn clear_core_upgrade_tag(tag: &str) -> bool {
    matches!(
        tag,
        "upgrade_role:core_mechanic"
            | "deck_repair:needed_function"
            | "upgrade_role:engine_enabler"
            | "upgrade_role:consistency"
            | "upgrade_role:scaling"
            | "upgrade_debt:controlled_exhaust"
            | "upgrade_debt:access_recovery"
            | "upgrade_debt:scaling_setup"
    )
}

fn combat_patch_upgrade_tag(tag: &str) -> bool {
    matches!(
        tag,
        "upgrade_role:defensive_survival"
            | "deck_repair:reliability"
            | "upgrade_role:phase_burst"
            | "upgrade_role:debuff_coverage"
            | "upgrade_debt:stasis_recovery"
            | "upgrade_debt:hyperbeam_block"
            | "upgrade_debt:phase_burst"
            | "upgrade_debt:execute_block"
            | "upgrade_debt:debuff_coverage"
            | "upgrade_debt:transitional_frontload"
    )
}

fn hp_percent(current_hp: i32, max_hp: i32) -> i32 {
    if max_hp <= 0 {
        return 0;
    }
    current_hp.saturating_mul(100) / max_hp
}

fn imminent_boss_recovery_is_required(
    context: &CampfireDecisionContextV1,
    config: &CampfirePolicyConfigV1,
) -> bool {
    context.known_boss_is_next
        && context.current_hp < context.max_hp
        && context.rest_vs_smith.effective_rest_heal > 0
        && hp_percent(context.current_hp, context.max_hp)
            <= config.imminent_boss_rest_max_hp_percent
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
        && (context.rest_vs_smith.verdict == RestVsSmithVerdictV1::RestFavored
            || imminent_boss_recovery_is_required(context, config)
            || ordinary_smith_health_floor_requires_rest(context, config)
            || context
                .strategy
                .support(StrategyPackageIdV2::RecoveryPressure)
                == StrategyPlanSupportV1::Strong)
}

fn ordinary_smith_health_floor_requires_rest(
    context: &CampfireDecisionContextV1,
    config: &CampfirePolicyConfigV1,
) -> bool {
    context.current_hp < context.max_hp
        && context.rest_vs_smith.effective_rest_heal > 0
        && context
            .candidates
            .iter()
            .any(|candidate| candidate.choice == CampfireChoice::Rest)
        && hp_percent(context.current_hp, context.max_hp) < config.ordinary_smith_min_hp_percent
}
