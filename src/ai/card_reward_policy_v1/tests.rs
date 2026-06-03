use crate::ai::card_reward_policy_v1::{
    build_card_reward_decision_context_v1, plan_card_reward_decision_v1, CardRewardEvidenceGapV1,
    CardRewardPolicyActionV1, CardRewardPolicyConfigV1,
};
use crate::content::cards::CardId;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

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
