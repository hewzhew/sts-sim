use sts_simulator::eval::run_control::{
    reward_policy_has_claimable_step, DecisionCandidateKey, DecisionSurface, RunControlSession,
};

use super::owner_model::{OwnerDecision, OwnerRoutine};

pub(super) fn reward_tiny_owner_decision(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> OwnerDecision {
    match reward_policy_has_claimable_step(session) {
        Ok(true) => return OwnerDecision::Routine(OwnerRoutine::RewardPolicyStep),
        Ok(false) => {}
        Err(error) => return OwnerDecision::Gap(error),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::owner_audit::owner_routines::apply_owner_routine;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::content::relics::RelicId;
    use sts_simulator::eval::run_control::{
        build_decision_surface, RunControlConfig, RunControlSession, RunDecisionAction,
    };
    use sts_simulator::state::core::{ClientInput, EngineState};
    use sts_simulator::state::rewards::{RewardCard, RewardItem, RewardState};

    #[test]
    fn reward_owner_claims_safe_relic_before_opening_card_boundary() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.items = vec![
            RewardItem::Card {
                cards: vec![RewardCard::new(CardId::DarkEmbrace, 0)],
            },
            RewardItem::Relic {
                relic_id: RelicId::MummifiedHand,
            },
        ];
        session.engine_state = EngineState::RewardScreen(reward);

        let surface = build_decision_surface(&session);
        assert!(matches!(
            reward_tiny_owner_decision(&session, &surface),
            OwnerDecision::Routine(OwnerRoutine::RewardPolicyStep)
        ));
        let current_order =
            crate::runtime::branch::owner_audit::current_oracle_candidate_order_v1(&session);
        let current_first = surface
            .view
            .candidates
            .iter()
            .find(|candidate| Some(&candidate.id) == current_order.first())
            .expect("production prior should select one exact candidate");
        assert!(matches!(
            current_first.action.executable_action_ref(),
            Some(RunDecisionAction::Input(ClientInput::ClaimReward(1)))
        ));
        apply_owner_routine(&mut session, OwnerRoutine::RewardPolicyStep)
            .expect("safe relic claim should execute as one owner step");
        assert!(session
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::MummifiedHand));

        let surface = build_decision_surface(&session);
        let OwnerDecision::Routine(OwnerRoutine::Candidate { candidate_id, .. }) =
            reward_tiny_owner_decision(&session, &surface)
        else {
            panic!("card reward should become the next boundary");
        };
        assert!(surface.view.candidates.iter().any(|candidate| {
            candidate.id == candidate_id
                && matches!(
                    candidate.key,
                    Some(DecisionCandidateKey::CardRewardOpen { .. })
                )
        }));
    }
}
