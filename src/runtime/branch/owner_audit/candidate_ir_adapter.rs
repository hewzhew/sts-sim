use sts_simulator::ai::strategy::decision_pipeline::DecisionCandidateKind;
use sts_simulator::eval::run_control::DecisionCandidateKey;

pub(super) fn boss_relic_kind(key: &Option<DecisionCandidateKey>) -> DecisionCandidateKind {
    match key {
        Some(DecisionCandidateKey::BossRelicPick { relic, .. }) => {
            DecisionCandidateKind::BossRelicPick { relic: *relic }
        }
        Some(DecisionCandidateKey::BossRelicSkip) => DecisionCandidateKind::BossRelicSkip,
        _ => DecisionCandidateKind::Unsupported,
    }
}

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

pub(super) fn is_boss_relic_key(key: &Option<DecisionCandidateKey>) -> bool {
    matches!(
        key,
        Some(DecisionCandidateKey::BossRelicPick { .. } | DecisionCandidateKey::BossRelicSkip)
    )
}
