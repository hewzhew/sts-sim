use super::arbitration::arbitrate_card_reward_value_estimates_v1;
use super::gate::pick_gate;
use super::types::{CardRewardDecisionContextV1, CardRewardDecisionV1, CardRewardPolicyConfigV1};
use super::value::estimate_card_reward_values;

pub fn plan_card_reward_decision_v1(
    context: &CardRewardDecisionContextV1,
    config: &CardRewardPolicyConfigV1,
) -> CardRewardDecisionV1 {
    let value_estimates = estimate_card_reward_values(context);
    let value_arbitration = arbitrate_card_reward_value_estimates_v1(context, &value_estimates);
    let (action, autopilot_gate, evidence_gaps, pick_certificate) =
        pick_gate(context, &value_arbitration.gate_value_estimates, config);

    CardRewardDecisionV1 {
        action,
        context: context.clone(),
        candidates: context.candidates.clone(),
        value_estimates,
        value_arbitration,
        autopilot_gate,
        evidence_gaps,
        pick_certificate,
        label_role: "behavior_policy_not_teacher",
    }
}
