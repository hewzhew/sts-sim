use super::types::{
    CardRewardDecisionContextV1, CardRewardValueComponentV1, CardRewardValueEligibilityReasonV1,
    CardRewardValueEligibilityV1, CardRewardValueEstimateV1, CardRewardValueSourceV1,
    CardRewardValueStatusV1,
};

pub(crate) fn estimate_card_reward_values(
    context: &CardRewardDecisionContextV1,
) -> Vec<CardRewardValueEstimateV1> {
    let mut estimates = context
        .candidates
        .iter()
        .map(|candidate| CardRewardValueEstimateV1 {
            index: candidate.index,
            card: candidate.card,
            source: CardRewardValueSourceV1::UncalibratedImpactPrior,
            status: CardRewardValueStatusV1::UncalibratedPrior,
            survival_delta: 0.0,
            progress_delta: 0.0,
            deck_consistency_delta: 0.0,
            uncertainty: 1.0,
            eligibility: CardRewardValueEligibilityV1 {
                usable_for_value_estimate: true,
                usable_for_autopilot_gate: false,
                reasons: vec![
                    CardRewardValueEligibilityReasonV1::UncalibratedPriorNeverGateEligible,
                ],
                bucket_key: None,
                horizon: None,
                outcome_sample_count: None,
            },
            components: impact_prior_components(context, candidate),
        })
        .collect::<Vec<_>>();
    estimates.extend(super::route_risk::estimate_route_risk_values(context));
    estimates
}

fn impact_prior_components(
    context: &CardRewardDecisionContextV1,
    candidate: &super::types::CardRewardCandidateEvidenceV1,
) -> Vec<CardRewardValueComponentV1> {
    let facts = &candidate.facts;
    vec![
        CardRewardValueComponentV1 {
            name: "raw_damage".to_string(),
            value: facts.damage.total_damage.max(0) as f32,
        },
        CardRewardValueComponentV1 {
            name: "raw_block".to_string(),
            value: facts.block.max(0) as f32,
        },
        CardRewardValueComponentV1 {
            name: "raw_draw".to_string(),
            value: facts.draw_cards.max(0) as f32,
        },
        CardRewardValueComponentV1 {
            name: "raw_energy".to_string(),
            value: facts.energy_gain.max(0) as f32,
        },
        CardRewardValueComponentV1 {
            name: "raw_vulnerable".to_string(),
            value: facts.vulnerable.max(0) as f32,
        },
        CardRewardValueComponentV1 {
            name: "raw_weak".to_string(),
            value: facts.weak.max(0) as f32,
        },
        CardRewardValueComponentV1 {
            name: "raw_enemy_strength_down".to_string(),
            value: facts.enemy_strength_down.max(0) as f32,
        },
        CardRewardValueComponentV1 {
            name: "deck_size_after_pick".to_string(),
            value: context.deck.deck_size.saturating_add(1) as f32,
        },
        CardRewardValueComponentV1 {
            name: "uncertainty".to_string(),
            value: 1.0,
        },
    ]
}
