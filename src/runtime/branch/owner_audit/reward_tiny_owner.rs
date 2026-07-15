use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface};

use super::owner_model::{OwnerDecision, OwnerRoutine};

pub(super) fn reward_tiny_owner_decision(surface: &DecisionSurface) -> OwnerDecision {
    if let Some(action) = surface
        .view
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.key,
                Some(DecisionCandidateKey::CardRewardOpen { .. })
            )
        })
        .and_then(|candidate| candidate.action.executable_action())
    {
        return OwnerDecision::Routine(OwnerRoutine::Action(action));
    }
    OwnerDecision::Routine(OwnerRoutine::RewardTinyAutomation)
}
