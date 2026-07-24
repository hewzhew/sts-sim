use sts_simulator::eval::run_control::DecisionCandidateKey;

pub(super) fn is_card_reward_key(key: &Option<DecisionCandidateKey>) -> bool {
    matches!(
        key,
        Some(
            DecisionCandidateKey::CardRewardOpen { .. }
                | DecisionCandidateKey::CardRewardPick { .. }
                | DecisionCandidateKey::CardRewardSingingBowl { .. }
                | DecisionCandidateKey::CardRewardSkip { .. }
        )
    )
}
