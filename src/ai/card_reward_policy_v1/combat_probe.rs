use super::types::{
    CardRewardDecisionContextV1, CardRewardValueComponentV1, CardRewardValueEligibilityReasonV1,
    CardRewardValueEligibilityV1, CardRewardValueEstimateV1, CardRewardValueHorizonV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1, DeckProfileV1,
};

const FRONTLOAD_DENSITY_SCALE: f32 = 8.0;
const BLOCK_DENSITY_SCALE: f32 = 7.0;
const CONTROL_SCALE: f32 = 3.0;
const PROGRESS_FROM_FRONTLOAD: f32 = 0.35;
const PROGRESS_FROM_VULNERABLE: f32 = 0.08;
const BASE_DECK_SIZE_DRAG: f32 = 0.10;
const BASE_PUBLIC_PROBE_UNCERTAINTY: f32 = 0.58;
const ROUTE_EVIDENCE_UNCERTAINTY_RELIEF: f32 = 0.08;
const ROUTE_WARNING_UNCERTAINTY: f32 = 0.03;
const MAX_WARNING_UNCERTAINTY: f32 = 0.15;

pub(crate) fn estimate_combat_probe_values(
    context: &CardRewardDecisionContextV1,
) -> Vec<CardRewardValueEstimateV1> {
    let pressure = combat_pressure(context);
    context
        .candidates
        .iter()
        .map(|candidate| {
            let frontload_density_delta = density_delta(
                context.deck.total_attack_damage,
                candidate.impact.frontload_damage_delta.max(0),
                &context.deck,
            );
            let block_density_delta = density_delta(
                context.deck.total_block,
                candidate.impact.block_delta.max(0),
                &context.deck,
            );
            let survival_control_delta =
                (candidate.facts.weak.max(0) + candidate.facts.enemy_strength_down.max(0)) as f32
                    / CONTROL_SCALE;
            let progress_control_delta =
                candidate.facts.vulnerable.max(0) as f32 * PROGRESS_FROM_VULNERABLE;
            let deck_size_drag = BASE_DECK_SIZE_DRAG / deck_size_after_pick(&context.deck);
            let survival_delta = scaled(frontload_density_delta, FRONTLOAD_DENSITY_SCALE)
                * pressure
                + scaled(block_density_delta, BLOCK_DENSITY_SCALE) * route_avoid_damage(context)
                + survival_control_delta
                - deck_size_drag;
            let progress_delta = scaled(frontload_density_delta, FRONTLOAD_DENSITY_SCALE)
                * PROGRESS_FROM_FRONTLOAD
                + progress_control_delta;

            CardRewardValueEstimateV1 {
                index: candidate.index,
                card: candidate.card,
                source: CardRewardValueSourceV1::CombatProbe,
                status: CardRewardValueStatusV1::PublicCombatHeuristic,
                survival_delta,
                progress_delta,
                deck_consistency_delta: -deck_size_drag,
                uncertainty: public_probe_uncertainty(context),
                eligibility: CardRewardValueEligibilityV1 {
                    usable_for_value_estimate: true,
                    usable_for_autopilot_gate: false,
                    reasons: vec![
                        CardRewardValueEligibilityReasonV1::PublicCombatHeuristicNotGateEligible,
                    ],
                    bucket_key: None,
                    horizon: Some(CardRewardValueHorizonV1::NextCombatPublicProbe),
                    outcome_sample_count: None,
                },
                components: vec![
                    CardRewardValueComponentV1 {
                        name: "public_probe_combat_pressure".to_string(),
                        value: pressure,
                    },
                    CardRewardValueComponentV1 {
                        name: "frontload_density_delta".to_string(),
                        value: frontload_density_delta,
                    },
                    CardRewardValueComponentV1 {
                        name: "block_density_delta".to_string(),
                        value: block_density_delta,
                    },
                    CardRewardValueComponentV1 {
                        name: "survival_control_delta".to_string(),
                        value: survival_control_delta,
                    },
                    CardRewardValueComponentV1 {
                        name: "progress_control_delta".to_string(),
                        value: progress_control_delta,
                    },
                    CardRewardValueComponentV1 {
                        name: "public_probe_deck_size_drag".to_string(),
                        value: deck_size_drag,
                    },
                ],
            }
        })
        .collect()
}

fn density_delta(current_total: i32, added: i32, deck: &DeckProfileV1) -> f32 {
    let before = (current_total.max(0) as f32) / deck_size_before_pick(deck);
    let after = (current_total.max(0) + added.max(0)) as f32 / deck_size_after_pick(deck);
    after - before
}

fn deck_size_before_pick(deck: &DeckProfileV1) -> f32 {
    deck.deck_size.max(1) as f32
}

fn deck_size_after_pick(deck: &DeckProfileV1) -> f32 {
    deck.deck_size.saturating_add(1).max(1) as f32
}

fn combat_pressure(context: &CardRewardDecisionContextV1) -> f32 {
    let Some(route) = context.route.as_ref() else {
        return 0.45;
    };
    let elite_pressure = (1.0 - route.can_take_elite).clamp(0.0, 1.0);
    let early_pressure = route
        .selected_route
        .as_ref()
        .map(|selected| selected.max_early_pressure as f32 / 4.0)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    (0.35
        + route.avoid_damage.clamp(0.0, 1.0) * 0.25
        + elite_pressure * 0.25
        + early_pressure * 0.15)
        .clamp(0.0, 1.0)
}

fn route_avoid_damage(context: &CardRewardDecisionContextV1) -> f32 {
    context
        .route
        .as_ref()
        .map(|route| route.avoid_damage.clamp(0.0, 1.0))
        .unwrap_or(0.5)
}

fn public_probe_uncertainty(context: &CardRewardDecisionContextV1) -> f32 {
    let route_relief = context
        .route
        .as_ref()
        .map(|_| ROUTE_EVIDENCE_UNCERTAINTY_RELIEF)
        .unwrap_or(0.0);
    let warning_penalty = context
        .route
        .as_ref()
        .map(|route| {
            (route.warnings.len() as f32 * ROUTE_WARNING_UNCERTAINTY).min(MAX_WARNING_UNCERTAINTY)
        })
        .unwrap_or(0.0);
    (BASE_PUBLIC_PROBE_UNCERTAINTY - route_relief + warning_penalty).clamp(0.0, 1.0)
}

fn scaled(value: f32, denominator: f32) -> f32 {
    (value / denominator).clamp(-1.0, 1.0)
}
