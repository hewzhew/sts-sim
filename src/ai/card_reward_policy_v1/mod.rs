mod context;
mod facts;
mod gate;
mod impact;
mod policy;
mod profile;
mod replay;
mod types;
mod value;

pub use context::build_card_reward_decision_context_v1;
pub use policy::plan_card_reward_decision_v1;
pub use replay::{replay_card_reward_decision_v1, PublicRewardDecisionPacketV1};
pub use types::{
    CardRewardAutopilotGateReportV1, CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1,
    CardRewardDecisionV1, CardRewardEvidenceGapV1, CardRewardFactsV1, CardRewardPickCertificateV1,
    CardRewardPickDependencyV1, CardRewardPlanEffectV1, CardRewardPolicyActionV1,
    CardRewardPolicyConfigV1, CardRewardStopDispositionV1, CardRewardValueEstimateV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1,
};

#[cfg(test)]
mod tests;
