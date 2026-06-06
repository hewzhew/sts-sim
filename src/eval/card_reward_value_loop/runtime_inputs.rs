use crate::ai::card_reward_policy_v1::{
    CardRewardDecisionContextV1, CardRewardEstimatorInputsV1, CardRewardValueEstimateV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1,
};

use super::{
    estimate_card_reward_values_from_calibration_v1,
    estimate_card_reward_values_from_route_risk_calibration_v1,
    estimate_card_reward_values_from_strategy_package_calibration_v1,
    CardRewardOutcomeCalibrationV1, CardRewardRouteRiskCalibrationV1,
    CardRewardStrategyPackageCalibrationV1,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct CardRewardRuntimeEstimatorCalibrationsV1<'a> {
    pub outcome: Option<&'a CardRewardOutcomeCalibrationV1>,
    pub route_risk: Option<&'a CardRewardRouteRiskCalibrationV1>,
    pub strategy_package: Option<&'a CardRewardStrategyPackageCalibrationV1>,
}

#[derive(Clone, Copy, Debug)]
pub struct CardRewardRuntimeEstimatorSourcesV1<'a> {
    pub calibrations: CardRewardRuntimeEstimatorCalibrationsV1<'a>,
    pub counterfactual_probe_estimates: &'a [CardRewardValueEstimateV1],
}

impl Default for CardRewardRuntimeEstimatorSourcesV1<'_> {
    fn default() -> Self {
        Self {
            calibrations: CardRewardRuntimeEstimatorCalibrationsV1::default(),
            counterfactual_probe_estimates: &[],
        }
    }
}

impl<'a> From<CardRewardRuntimeEstimatorCalibrationsV1<'a>>
    for CardRewardRuntimeEstimatorSourcesV1<'a>
{
    fn from(calibrations: CardRewardRuntimeEstimatorCalibrationsV1<'a>) -> Self {
        Self {
            calibrations,
            counterfactual_probe_estimates: &[],
        }
    }
}

pub fn build_card_reward_runtime_estimator_inputs_v1<'a>(
    context: &CardRewardDecisionContextV1,
    sources: impl Into<CardRewardRuntimeEstimatorSourcesV1<'a>>,
) -> CardRewardEstimatorInputsV1 {
    let sources = sources.into();
    let calibrations = sources.calibrations;
    let mut external_value_estimates = calibrations
        .outcome
        .map(|calibration| estimate_card_reward_values_from_calibration_v1(context, calibration))
        .unwrap_or_default();

    if let Some(calibration) = calibrations.route_risk {
        external_value_estimates.extend(
            estimate_card_reward_values_from_route_risk_calibration_v1(context, calibration),
        );
    }

    if let Some(calibration) = calibrations.strategy_package {
        external_value_estimates.extend(
            estimate_card_reward_values_from_strategy_package_calibration_v1(context, calibration),
        );
    }

    external_value_estimates.extend(
        sources
            .counterfactual_probe_estimates
            .iter()
            .filter(|estimate| is_valid_counterfactual_probe_estimate(context, estimate))
            .cloned(),
    );

    CardRewardEstimatorInputsV1 {
        external_value_estimates,
    }
}

fn is_valid_counterfactual_probe_estimate(
    context: &CardRewardDecisionContextV1,
    estimate: &CardRewardValueEstimateV1,
) -> bool {
    estimate.source == CardRewardValueSourceV1::CombatProbe
        && estimate.status == CardRewardValueStatusV1::CounterfactualProbe
        && estimate.eligibility.usable_for_value_estimate
        && context
            .candidates
            .iter()
            .any(|candidate| candidate.index == estimate.index && candidate.card == estimate.card)
}
