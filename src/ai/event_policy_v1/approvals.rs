use super::types::{
    EventCandidateEvidenceV1, EventDecisionContextV1, EventPolicyClassV1, EventPolicyConfigV1,
};
use crate::state::events::EventId;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PickApproval {
    pub(crate) index: usize,
    pub(crate) label: String,
    pub(crate) confidence: f32,
    pub(crate) reason: String,
}

pub(crate) fn pick_approvals(
    context: &EventDecisionContextV1,
    config: &EventPolicyConfigV1,
) -> Vec<PickApproval> {
    context
        .candidates
        .iter()
        .filter_map(|candidate| pick_approval(candidate, context, config))
        .collect()
}

fn pick_approval(
    candidate: &EventCandidateEvidenceV1,
    context: &EventDecisionContextV1,
    config: &EventPolicyConfigV1,
) -> Option<PickApproval> {
    if candidate.disabled {
        return None;
    }
    if let Some(approval) = winding_halls_mark_of_bloom_approval(candidate, context) {
        return Some(approval);
    }
    match candidate.class {
        EventPolicyClassV1::FreeKnownBenefit if config.allow_free_known_benefit => {
            Some(PickApproval {
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
            Some(PickApproval {
                index: candidate.index,
                label: candidate.label.clone(),
                confidence: 0.72,
                reason: "declined event because every other visible option has cost, uncertainty, combat, or deck mutation".to_string(),
            })
        }
        EventPolicyClassV1::MaxHpForHpCost
            if config.allow_max_hp_for_safe_hp_cost
                && max_hp_for_hp_cost_is_safe(context, candidate, config) =>
        {
            Some(PickApproval {
                index: candidate.index,
                label: candidate.label.clone(),
                confidence: 0.74,
                reason: format!(
                    "gain {} max HP for {} HP while keeping a safe health buffer",
                    candidate.max_hp_gain, candidate.hp_cost
                ),
            })
        }
        _ => None,
    }
}

fn winding_halls_mark_of_bloom_approval(
    candidate: &EventCandidateEvidenceV1,
    context: &EventDecisionContextV1,
) -> Option<PickApproval> {
    if context.event_id != EventId::WindingHalls || !context.has_mark_of_the_bloom {
        return None;
    }
    if candidate.max_hp_loss <= 0
        || candidate.hp_cost > 0
        || candidate.heal_amount > 0
        || candidate.curse_count > 0
        || candidate.obtained_card_count > 0
    {
        return None;
    }
    Some(PickApproval {
        index: candidate.index,
        label: candidate.label.clone(),
        confidence: 0.82,
        reason: "Winding Halls: Mark of the Bloom blocks the heal option, so prefer the structured max-HP loss option over curse or deck growth".to_string(),
    })
}

fn max_hp_for_hp_cost_is_safe(
    context: &EventDecisionContextV1,
    candidate: &EventCandidateEvidenceV1,
    config: &EventPolicyConfigV1,
) -> bool {
    if candidate.hp_cost <= 0 || candidate.max_hp_gain <= 0 {
        return false;
    }
    let hp_after = context.current_hp.saturating_sub(candidate.hp_cost);
    if hp_after < config.min_hp_after_safe_hp_cost {
        return false;
    }
    if context.max_hp <= 0 {
        return false;
    }
    let ratio_after = hp_after as f32 / context.max_hp as f32;
    ratio_after >= config.min_hp_ratio_after_safe_hp_cost
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
