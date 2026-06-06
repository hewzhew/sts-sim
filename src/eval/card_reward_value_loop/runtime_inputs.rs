use crate::ai::card_reward_policy_v1::{CardRewardDecisionContextV1, CardRewardEstimatorInputsV1};

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

pub fn build_card_reward_runtime_estimator_inputs_v1(
    context: &CardRewardDecisionContextV1,
    calibrations: CardRewardRuntimeEstimatorCalibrationsV1<'_>,
) -> CardRewardEstimatorInputsV1 {
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

    CardRewardEstimatorInputsV1 {
        external_value_estimates,
    }
}
