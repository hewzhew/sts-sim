use crate::ai::card_reward_policy_v1::{
    build_card_reward_decision_context_v1, plan_card_reward_decision_v1, CardRewardEvidenceGapV1,
    CardRewardPolicyActionV1, CardRewardPolicyConfigV1, CardRewardValueSourceV1,
    CardRewardValueStatusV1,
};
use crate::content::cards::CardId;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::types::{CardRewardRouteEvidenceV1, CardRewardSelectedRouteV1};

#[test]
fn card_facts_are_mechanical_and_do_not_contain_pick_value() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::TwinStrike, 0)],
        None,
    );

    let candidate = &context.candidates[0];

    assert_eq!(candidate.facts.card, CardId::TwinStrike);
    assert_eq!(candidate.facts.damage.damage_per_hit, 5);
    assert_eq!(candidate.facts.damage.hit_count, 2);
    assert_eq!(candidate.facts.damage.total_damage, 10);
    assert!(candidate.facts.pick_dependencies.is_empty());
    assert!(candidate.impact.certification_blockers.is_empty());
}

#[test]
fn decision_context_marks_missing_route_evidence_as_policy_gap_not_card_fact() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::Shockwave, 0)],
        None,
    );

    assert!(context.route.is_none());

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision
        .evidence_gaps
        .contains(&CardRewardEvidenceGapV1::MissingRouteEvidence));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn archetype_dependent_card_requires_matching_deck_evidence() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::HeavyBlade, 0)],
        None,
    );

    let candidate = &context.candidates[0];

    assert!(candidate
        .facts
        .pick_dependencies
        .contains(&crate::ai::card_reward_policy_v1::CardRewardPickDependencyV1::StrengthScaling));
    assert!(candidate
        .impact
        .certification_blockers
        .contains(&CardRewardEvidenceGapV1::UnsatisfiedStrengthScalingEvidence));

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn score_threshold_overrides_cannot_force_a_pick_without_certificate() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::SearingBlow, 0)],
        None,
    );
    let decision = plan_card_reward_decision_v1(
        &context,
        &CardRewardPolicyConfigV1 {
            allow_automatic_pick_certificates: true,
        },
    );

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
}

#[test]
fn decision_builds_prior_value_estimates_for_every_candidate() {
    let mut context = context_for_cards(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Shockwave, 0),
    ]);
    context.route = Some(route_with_combat_pressure());

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert_eq!(decision.value_estimates.len(), 2);
    assert!(decision
        .value_estimates
        .iter()
        .all(|estimate| estimate.source == CardRewardValueSourceV1::ImpactPrior));
    assert!(decision
        .value_estimates
        .iter()
        .all(|estimate| estimate.status == CardRewardValueStatusV1::UncalibratedPrior));
    assert!(decision.value_estimates.iter().all(|estimate| {
        estimate.survival_delta == 0.0
            && estimate.progress_delta == 0.0
            && estimate.deck_consistency_delta == 0.0
            && estimate.uncertainty == 1.0
    }));
}

#[test]
fn uncalibrated_prior_values_are_consumed_by_gate_but_cannot_certify_pick() {
    let mut context = context_for_cards(vec![RewardCard::new(CardId::Shockwave, 0)]);
    context.route = Some(route_with_combat_pressure());

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert!(decision.pick_certificate.is_none());
    assert!(decision
        .evidence_gaps
        .contains(&CardRewardEvidenceGapV1::UncalibratedValueEstimate));
}

fn context_for_cards(
    cards: Vec<RewardCard>,
) -> crate::ai::card_reward_policy_v1::CardRewardDecisionContextV1 {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    build_card_reward_decision_context_v1(&run_state, cards, None)
}

fn route_with_combat_pressure() -> CardRewardRouteEvidenceV1 {
    CardRewardRouteEvidenceV1 {
        route_policy: "test_route_evidence",
        selected_route: Some(CardRewardSelectedRouteV1 {
            next_x: 3,
            next_y: 1,
            min_fires: 1,
            max_fires: 2,
            first_fire_floor: Some(6),
            min_elites: 0,
            max_elites: 1,
            min_early_pressure: 1,
            max_early_pressure: 3,
        }),
        candidate_count: 2,
        need_card_rewards: 1.0,
        need_upgrade: 0.5,
        need_heal: 0.1,
        can_take_elite: 0.6,
        avoid_damage: 0.4,
        warnings: Vec::new(),
    }
}
