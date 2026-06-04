use super::types::{
    EventCandidateEvidenceV1, EventDecisionContextV1, EventPolicyClassV1, EventPolicyConfigV1,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PickCertificate {
    pub(crate) index: usize,
    pub(crate) label: String,
    pub(crate) confidence: f32,
    pub(crate) reason: String,
}

pub(crate) fn pick_certificates(
    context: &EventDecisionContextV1,
    config: &EventPolicyConfigV1,
) -> Vec<PickCertificate> {
    context
        .candidates
        .iter()
        .filter_map(|candidate| pick_certificate(candidate, context, config))
        .collect()
}

fn pick_certificate(
    candidate: &EventCandidateEvidenceV1,
    context: &EventDecisionContextV1,
    config: &EventPolicyConfigV1,
) -> Option<PickCertificate> {
    if candidate.disabled {
        return None;
    }
    match candidate.class {
        EventPolicyClassV1::FreeKnownBenefit if config.allow_free_known_benefit => {
            Some(PickCertificate {
                index: candidate.index,
                label: candidate.label.clone(),
                confidence: 0.84,
                reason: "free known public event benefit with no visible downside".to_string(),
            })
        }
        EventPolicyClassV1::SafeExit
            if config.allow_safe_exit_from_risky_event
                && all_other_enabled_candidates_are_risky(context, candidate.index) =>
        {
            Some(PickCertificate {
                index: candidate.index,
                label: candidate.label.clone(),
                confidence: 0.72,
                reason: "declined event because every other visible option has cost, uncertainty, combat, or deck mutation".to_string(),
            })
        }
        _ => None,
    }
}

fn all_other_enabled_candidates_are_risky(
    context: &EventDecisionContextV1,
    selected_index: usize,
) -> bool {
    context
        .candidates
        .iter()
        .filter(|candidate| candidate.index != selected_index && !candidate.disabled)
        .all(|candidate| {
            matches!(
                candidate.class,
                EventPolicyClassV1::ResourceCost
                    | EventPolicyClassV1::CurseDebt
                    | EventPolicyClassV1::SelectionOrDeckMutation
                    | EventPolicyClassV1::CombatStart
                    | EventPolicyClassV1::UncertainReward
                    | EventPolicyClassV1::Unknown
            )
        })
}
