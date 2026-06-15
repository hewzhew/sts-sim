use crate::ai::noncombat_strategy_v1::StrategyPlanSupportV1;
use crate::content::relics::RelicId;

use super::types::{
    BossRelicCandidateEvidenceV1, BossRelicDecisionContextV1, BossRelicPolicyClassV1,
    BossRelicPolicyConfigV1,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AutopilotPick {
    pub(crate) index: usize,
    pub(crate) relic: RelicId,
    pub(crate) confidence: f32,
    pub(crate) reason: String,
}

pub(crate) fn autopilot_picks(
    context: &BossRelicDecisionContextV1,
    config: &BossRelicPolicyConfigV1,
) -> Vec<AutopilotPick> {
    context
        .candidates
        .iter()
        .filter_map(|candidate| autopilot_pick(candidate, context, config))
        .collect()
}

fn autopilot_pick(
    candidate: &BossRelicCandidateEvidenceV1,
    context: &BossRelicDecisionContextV1,
    config: &BossRelicPolicyConfigV1,
) -> Option<AutopilotPick> {
    match candidate.class {
        BossRelicPolicyClassV1::StarterRelicUpgrade if config.allow_starter_upgrade => {
            Some(AutopilotPick {
                index: candidate.index,
                relic: candidate.relic,
                confidence: 0.95,
                reason: format!(
                    "{:?} upgrades the starter relic with no visible downside",
                    candidate.relic
                ),
            })
        }
        BossRelicPolicyClassV1::DeckCleanup
            if config.allow_empty_cage_when_cleanup_supported
                && support_gate_at_least(candidate, StrategyPlanSupportV1::Plausible)
                && no_higher_agency_competitor(context, candidate.index) =>
        {
            Some(AutopilotPick {
                index: candidate.index,
                relic: candidate.relic,
                confidence: 0.82,
                reason: format!(
                    "{:?} matches cleanup pressure and avoids higher-agency boss relic uncertainty",
                    candidate.relic
                ),
            })
        }
        BossRelicPolicyClassV1::BroadSafeValue
            if config.allow_tiny_house_as_safe_fallback
                && candidate.relic == RelicId::TinyHouse
                && all_other_candidates_are_constrained(context, candidate.index) =>
        {
            Some(AutopilotPick {
                index: candidate.index,
                relic: candidate.relic,
                confidence: 0.78,
                reason:
                    "TinyHouse is the only broad low-downside option against constrained alternatives"
                        .to_string(),
            })
        }
        _ => None,
    }
}

fn no_higher_agency_competitor(
    context: &BossRelicDecisionContextV1,
    selected_index: usize,
) -> bool {
    context
        .candidates
        .iter()
        .filter(|candidate| candidate.index != selected_index)
        .all(|candidate| {
            matches!(
                candidate.class,
                BossRelicPolicyClassV1::EnergyWithConstraint
                    | BossRelicPolicyClassV1::CurseDebt
                    | BossRelicPolicyClassV1::TransformAgency
                    | BossRelicPolicyClassV1::Unknown
            )
        })
}

fn all_other_candidates_are_constrained(
    context: &BossRelicDecisionContextV1,
    selected_index: usize,
) -> bool {
    context
        .candidates
        .iter()
        .filter(|candidate| candidate.index != selected_index)
        .all(|candidate| {
            matches!(
                candidate.class,
                BossRelicPolicyClassV1::EnergyWithConstraint
                    | BossRelicPolicyClassV1::CurseDebt
                    | BossRelicPolicyClassV1::TransformAgency
                    | BossRelicPolicyClassV1::Unknown
            )
        })
}

fn support_gate_at_least(
    candidate: &BossRelicCandidateEvidenceV1,
    minimum: StrategyPlanSupportV1,
) -> bool {
    support_rank(candidate.support_gate) >= support_rank(minimum)
}

fn support_rank(support: StrategyPlanSupportV1) -> u8 {
    match support {
        StrategyPlanSupportV1::Blocked => 0,
        StrategyPlanSupportV1::Weak => 1,
        StrategyPlanSupportV1::Plausible => 2,
        StrategyPlanSupportV1::Strong => 3,
    }
}
