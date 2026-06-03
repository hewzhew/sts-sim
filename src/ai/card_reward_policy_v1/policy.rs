use super::gate::pick_gate;
use super::types::{CardRewardDecisionContextV1, CardRewardDecisionV1, CardRewardPolicyConfigV1};

pub fn plan_card_reward_decision_v1(
    context: &CardRewardDecisionContextV1,
    config: &CardRewardPolicyConfigV1,
) -> CardRewardDecisionV1 {
    let (action, evidence_gaps, pick_certificate) = pick_gate(context, config);

    CardRewardDecisionV1 {
        action,
        context: context.clone(),
        candidates: context.candidates.clone(),
        evidence_gaps,
        pick_certificate,
        label_role: "behavior_policy_not_teacher",
    }
}
