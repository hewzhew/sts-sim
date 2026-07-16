use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface};

use super::owner_model::{OwnerDecision, OwnerRoutine};

pub(super) fn reward_tiny_owner_decision(surface: &DecisionSurface) -> OwnerDecision {
    if let Some((candidate_id, action)) = surface
        .view
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.key,
                Some(DecisionCandidateKey::CardRewardOpen { .. })
            )
        })
        .and_then(|candidate| {
            candidate
                .action
                .executable_action()
                .map(|action| (candidate.id.clone(), action))
        })
    {
        return OwnerDecision::Routine(OwnerRoutine::Candidate {
            candidate_id,
            action,
        });
    }
    OwnerDecision::Routine(OwnerRoutine::RewardPolicyStep)
}
