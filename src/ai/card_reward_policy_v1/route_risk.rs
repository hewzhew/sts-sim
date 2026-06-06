use super::types::{
    CardRewardDecisionContextV1, CardRewardValueComponentV1, CardRewardValueEligibilityReasonV1,
    CardRewardValueEligibilityV1, CardRewardValueEstimateV1, CardRewardValueHorizonV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1,
};

const FRONTLOAD_DAMAGE_SCALE: f32 = 14.0;
const BLOCK_SCALE: f32 = 12.0;
const DRAW_SCALE: f32 = 3.0;
const ENERGY_SCALE: f32 = 2.0;
const ROUTE_RISK_BASE_PRESSURE: f32 = 0.25;
const ROUTE_RISK_ELITE_PRESSURE_WEIGHT: f32 = 0.35;
const ROUTE_RISK_AVOID_DAMAGE_WEIGHT: f32 = 0.25;
const ROUTE_RISK_EARLY_PRESSURE_WEIGHT: f32 = 0.25;
const ROUTE_RISK_FUTURE_ELITE_PRESSURE: f32 = 0.2;
const ROUTE_RISK_MAX_PRESSURE: f32 = 1.5;
const ROUTE_RISK_BLOCK_RELIEF_WEIGHT: f32 = 0.8;
const ROUTE_RISK_CONTROL_RELIEF_PER_DEBUFF: f32 = 0.18;
const ROUTE_RISK_MAX_CONTROL_RELIEF: f32 = 0.9;
const ROUTE_RISK_DRAW_RELIEF_WEIGHT: f32 = 0.25;
const ROUTE_RISK_ENERGY_RELIEF_WEIGHT: f32 = 0.25;
const ROUTE_RISK_BASE_DECK_SIZE_DRAG: f32 = 0.12;
const ROUTE_RISK_AVOID_DAMAGE_DECK_DRAG: f32 = 0.08;
const ROUTE_RISK_PROGRESS_FROM_FRONTLOAD: f32 = 0.25;
const ROUTE_RISK_UNCERTAINTY_WITH_SELECTED_ROUTE: f32 = 0.12;
const ROUTE_RISK_UNCERTAINTY_WITHOUT_SELECTED_ROUTE: f32 = 0.24;
const ROUTE_RISK_BASE_UNCERTAINTY: f32 = 0.58;
const ROUTE_RISK_WARNING_UNCERTAINTY: f32 = 0.03;
const ROUTE_RISK_MAX_WARNING_UNCERTAINTY: f32 = 0.15;

pub(crate) fn estimate_route_risk_values(
    context: &CardRewardDecisionContextV1,
) -> Vec<CardRewardValueEstimateV1> {
    let Some(route) = context.route.as_ref() else {
        return Vec::new();
    };
    let pressure = route_risk_pressure(context);

    context
        .candidates
        .iter()
        .map(|candidate| {
            let frontload_relief = scaled_positive(
                candidate.impact.frontload_damage_delta,
                FRONTLOAD_DAMAGE_SCALE,
            ) * pressure;
            let block_relief = scaled_positive(candidate.impact.block_delta, BLOCK_SCALE)
                * route.avoid_damage
                * ROUTE_RISK_BLOCK_RELIEF_WEIGHT;
            let control_relief =
                ((candidate.facts.weak.max(0) + candidate.facts.enemy_strength_down.max(0)) as f32
                    * ROUTE_RISK_CONTROL_RELIEF_PER_DEBUFF)
                    .min(ROUTE_RISK_MAX_CONTROL_RELIEF)
                    * pressure;
            let draw_relief = scaled_positive(candidate.impact.draw_delta, DRAW_SCALE)
                * route.need_card_rewards
                * ROUTE_RISK_DRAW_RELIEF_WEIGHT;
            let energy_relief = scaled_positive(candidate.impact.energy_delta, ENERGY_SCALE)
                * pressure
                * ROUTE_RISK_ENERGY_RELIEF_WEIGHT;
            let deck_size_drag = ROUTE_RISK_BASE_DECK_SIZE_DRAG
                + route.avoid_damage.clamp(0.0, 1.0) * ROUTE_RISK_AVOID_DAMAGE_DECK_DRAG;
            let survival_delta =
                frontload_relief + block_relief + control_relief + energy_relief - deck_size_drag;
            let progress_delta = (frontload_relief * ROUTE_RISK_PROGRESS_FROM_FRONTLOAD
                + draw_relief)
                * route.need_card_rewards.clamp(0.0, 1.0);

            CardRewardValueEstimateV1 {
                index: candidate.index,
                card: candidate.card,
                source: CardRewardValueSourceV1::RouteRisk,
                status: CardRewardValueStatusV1::RouteRiskEstimate,
                survival_delta,
                progress_delta,
                deck_consistency_delta: -deck_size_drag,
                uncertainty: route_risk_uncertainty(context),
                eligibility: CardRewardValueEligibilityV1 {
                    usable_for_value_estimate: true,
                    usable_for_autopilot_gate: false,
                    reasons: vec![CardRewardValueEligibilityReasonV1::RouteRiskEstimateNotPromoted],
                    bucket_key: None,
                    horizon: Some(CardRewardValueHorizonV1::VisibleRouteRisk),
                    outcome_sample_count: None,
                },
                components: vec![
                    CardRewardValueComponentV1 {
                        name: "route_risk_pressure".to_string(),
                        value: pressure,
                    },
                    CardRewardValueComponentV1 {
                        name: "route_frontload_relief".to_string(),
                        value: frontload_relief,
                    },
                    CardRewardValueComponentV1 {
                        name: "route_block_relief".to_string(),
                        value: block_relief,
                    },
                    CardRewardValueComponentV1 {
                        name: "route_control_relief".to_string(),
                        value: control_relief,
                    },
                    CardRewardValueComponentV1 {
                        name: "route_draw_relief".to_string(),
                        value: draw_relief,
                    },
                    CardRewardValueComponentV1 {
                        name: "route_deck_size_drag".to_string(),
                        value: deck_size_drag,
                    },
                ],
            }
        })
        .collect()
}

fn route_risk_pressure(context: &CardRewardDecisionContextV1) -> f32 {
    let Some(route) = context.route.as_ref() else {
        return 0.0;
    };
    let elite_pressure = (1.0 - route.can_take_elite).clamp(0.0, 1.0);
    let avoid_damage = route.avoid_damage.clamp(0.0, 1.0);
    let early_pressure = route
        .selected_route
        .as_ref()
        .map(|selected| selected.max_early_pressure as f32 / 4.0)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let future_elite_pressure = route
        .selected_route
        .as_ref()
        .map(|selected| (selected.max_elites > 0) as u8 as f32 * 0.2)
        .map(|elite_ahead| elite_ahead * ROUTE_RISK_FUTURE_ELITE_PRESSURE)
        .unwrap_or(0.0);

    (ROUTE_RISK_BASE_PRESSURE
        + elite_pressure * ROUTE_RISK_ELITE_PRESSURE_WEIGHT
        + avoid_damage * ROUTE_RISK_AVOID_DAMAGE_WEIGHT
        + early_pressure * ROUTE_RISK_EARLY_PRESSURE_WEIGHT
        + future_elite_pressure)
        .clamp(0.0, ROUTE_RISK_MAX_PRESSURE)
}

fn route_risk_uncertainty(context: &CardRewardDecisionContextV1) -> f32 {
    let Some(route) = context.route.as_ref() else {
        return 1.0;
    };
    let route_coverage = if route.selected_route.is_some() {
        ROUTE_RISK_UNCERTAINTY_WITH_SELECTED_ROUTE
    } else {
        ROUTE_RISK_UNCERTAINTY_WITHOUT_SELECTED_ROUTE
    };
    let warning_penalty = (route.warnings.len() as f32 * ROUTE_RISK_WARNING_UNCERTAINTY)
        .min(ROUTE_RISK_MAX_WARNING_UNCERTAINTY);
    (ROUTE_RISK_BASE_UNCERTAINTY + route_coverage + warning_penalty).clamp(0.0, 1.0)
}

fn scaled_positive(value: i32, denominator: f32) -> f32 {
    (value.max(0) as f32 / denominator).clamp(0.0, 1.5)
}
