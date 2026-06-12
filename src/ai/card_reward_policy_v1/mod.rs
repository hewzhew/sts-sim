mod arbitration;
mod behavior_gate;
mod combat_probe;
mod context;
mod facts;
mod gate;
mod impact;
mod policy;
mod profile;
mod replay;
mod route_risk;
mod semantics;
mod strategy_package_threat_alignment;
mod strategy_package_value;
mod threat_response;
mod types;
mod value;

pub use arbitration::arbitrate_card_reward_value_estimates_v1;
pub use context::build_card_reward_decision_context_v1;
pub use policy::{
    plan_card_reward_decision_v1, plan_card_reward_decision_with_estimator_inputs_v1,
};
pub use replay::{
    replay_card_reward_decision_v1, replay_card_reward_decision_with_estimator_inputs_v1,
    CardRewardDecisionReplayV1, PublicRewardDecisionPacketV1,
};
pub use semantics::card_reward_semantic_profile_v1;
pub(crate) use threat_response::candidate_response_threat_tags_v1;
pub use types::{
    CardRewardAutopilotGateReportV1, CardRewardCandidateEvidenceV1, CardRewardDecisionApprovalV1,
    CardRewardDecisionContextV1, CardRewardDecisionV1, CardRewardEstimatorArbitrationV1,
    CardRewardEstimatorCandidateArbitrationV1, CardRewardEstimatorInputsV1,
    CardRewardEvidenceGapV1, CardRewardFactsV1, CardRewardPickDependencyV1, CardRewardPlanEffectV1,
    CardRewardPolicyActionV1, CardRewardPolicyConfigV1, CardRewardRouteEvidenceV1,
    CardRewardSelectedRouteV1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
    CardRewardStopDispositionV1, CardRewardValueComponentV1, CardRewardValueEligibilityReasonV1,
    CardRewardValueEligibilityV1, CardRewardValueEstimateV1, CardRewardValueHorizonV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1,
};

#[cfg(test)]
mod tests;
