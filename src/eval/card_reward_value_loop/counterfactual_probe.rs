use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::{
    CardRewardValueEstimateV1, CardRewardValueSourceV1, CardRewardValueStatusV1,
};

pub const CARD_REWARD_COUNTERFACTUAL_PROBE_ESTIMATE_SET_SCHEMA_NAME: &str =
    "CardRewardCounterfactualProbeEstimateSetV1";
pub const CARD_REWARD_COUNTERFACTUAL_PROBE_ESTIMATE_SET_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardCounterfactualProbeEstimateSetV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub estimator_kind: String,
    pub estimates: Vec<CardRewardValueEstimateV1>,
}

impl CardRewardCounterfactualProbeEstimateSetV1 {
    pub fn from_estimates(estimates: Vec<CardRewardValueEstimateV1>) -> Self {
        Self {
            schema_name: CARD_REWARD_COUNTERFACTUAL_PROBE_ESTIMATE_SET_SCHEMA_NAME.to_string(),
            schema_version: CARD_REWARD_COUNTERFACTUAL_PROBE_ESTIMATE_SET_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            estimator_kind: "external_counterfactual_probe_estimates_v1".to_string(),
            estimates,
        }
    }

    pub fn valid_estimates(&self) -> Vec<CardRewardValueEstimateV1> {
        self.estimates
            .iter()
            .filter(|estimate| is_counterfactual_probe_estimate_v1(estimate))
            .cloned()
            .collect()
    }
}

pub fn is_counterfactual_probe_estimate_v1(estimate: &CardRewardValueEstimateV1) -> bool {
    estimate.source == CardRewardValueSourceV1::CombatProbe
        && estimate.status == CardRewardValueStatusV1::CounterfactualProbe
        && estimate.eligibility.usable_for_value_estimate
}
