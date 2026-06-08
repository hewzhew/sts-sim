use crate::ai::reward_policy_v1::{
    build_reward_decision_context_v1, plan_reward_decision_v1, RewardCandidateEvidenceV1,
    RewardDecisionContextV1, RewardPolicyActionV1, RewardPolicyClassV1, RewardPolicyConfigV1,
};
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;
use crate::state::rewards::RewardState;

pub(crate) struct RewardBranchOption {
    pub(crate) kind: &'static str,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) effect_kind: String,
    pub(crate) effect_key: String,
    pub(crate) effect_label: String,
}

pub(crate) fn reward_branch_options(
    session: &RunControlSession,
) -> Option<Vec<RewardBranchOption>> {
    let reward = active_reward_state(&session.engine_state)?;
    if reward.pending_card_choice.is_some() {
        return None;
    }

    let context = build_reward_decision_context_v1(&session.run_state, reward);
    if matches!(
        plan_reward_decision_v1(&context, &RewardPolicyConfigV1::default()).action,
        RewardPolicyActionV1::Claim { .. }
    ) {
        return None;
    }

    let mut options = context
        .candidates
        .iter()
        .filter_map(reward_claim_branch_option)
        .collect::<Vec<_>>();
    if let Some(option) = full_slot_potion_skip_branch_option(&session.engine_state, &context) {
        options.push(option);
    }
    if options.is_empty() {
        return None;
    }
    Some(options)
}

fn active_reward_state(engine: &EngineState) -> Option<&RewardState> {
    match engine {
        EngineState::RewardScreen(reward) => Some(reward),
        EngineState::RewardOverlay { reward_state, .. } => Some(reward_state),
        _ => None,
    }
}

fn reward_claim_branch_option(candidate: &RewardCandidateEvidenceV1) -> Option<RewardBranchOption> {
    let note = match candidate.class {
        RewardPolicyClassV1::RelicWithSapphireKeyConflict => "competes with Sapphire key",
        RewardPolicyClassV1::SapphireKey => "takes Sapphire key instead of the visible relic",
        RewardPolicyClassV1::EmeraldKey => "takes Emerald key route objective reward",
        _ => return None,
    };

    Some(RewardBranchOption {
        kind: "reward_claim",
        label: format!("Claim {}", candidate.label),
        command: format!("claim {}", candidate.index),
        effect_kind: "reward_claim".to_string(),
        effect_key: candidate.candidate_id.clone(),
        effect_label: format!("Claim {} | {note}", candidate.label),
    })
}

fn full_slot_potion_skip_branch_option(
    engine: &EngineState,
    context: &RewardDecisionContextV1,
) -> Option<RewardBranchOption> {
    if !matches!(engine, EngineState::RewardScreen(_)) {
        return None;
    }
    if context.candidates.is_empty()
        || !context
            .candidates
            .iter()
            .all(|candidate| candidate.class == RewardPolicyClassV1::PotionNoEmptySlot)
    {
        return None;
    }

    let labels = context
        .candidates
        .iter()
        .map(|candidate| candidate.label.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Some(RewardBranchOption {
        kind: "reward_skip",
        label: format!("Skip potion reward: {labels}"),
        command: "skip".to_string(),
        effect_kind: "reward_skip_full_potion".to_string(),
        effect_key: "reward:skip_full_slot_potion".to_string(),
        effect_label: format!("Skip potion reward: {labels} | full potion slots"),
    })
}
