use crate::ai::card_reward_policy_v1::{
    build_card_reward_decision_context_v1, plan_card_reward_decision_v1,
    replay_card_reward_decision_v1, CardRewardPolicyConfigV1, PublicRewardDecisionPacketV1,
};
use crate::ai::strategic::CandidateAction;
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

#[test]
fn card_reward_shadow_trace_covers_each_candidate_with_delta() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![
            RewardCard::new(CardId::Disarm, 0),
            RewardCard::new(CardId::FireBreathing, 0),
        ],
        None,
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert_eq!(decision.strategic_trace.audit.candidate_count, 3);
    assert_eq!(decision.strategic_trace.audit.delta_count, 3);
    assert_eq!(
        decision.strategic_trace.audit.candidate_without_delta_count,
        0
    );
    assert_eq!(
        decision.strategic_trace.snapshot.site,
        crate::ai::strategic::StrategicDecisionSite::CardReward
    );
    assert!(decision
        .strategic_trace
        .candidate_deltas
        .iter()
        .any(|delta| matches!(delta.action, CandidateAction::SkipCardReward)));
}

#[test]
fn card_reward_shadow_trace_includes_singing_bowl_as_decline_candidate() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.relics.push(RelicState::new(RelicId::SingingBowl));
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::Disarm, 0)],
        None,
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    let bowl_delta = decision
        .strategic_trace
        .candidate_deltas
        .iter()
        .find(|delta| matches!(delta.action, CandidateAction::TakeSingingBowl { .. }))
        .expect("Singing Bowl should be represented as a non-card reward candidate");

    assert_eq!(decision.strategic_trace.audit.candidate_count, 2);
    assert_eq!(decision.strategic_trace.audit.delta_count, 2);
    assert_eq!(
        decision.strategic_trace.audit.candidate_without_delta_count,
        0
    );
    assert!(bowl_delta
        .evidence
        .contains(&"singing_bowl_max_hp_alternative".to_string()));
}

#[test]
fn card_reward_shadow_trace_records_component_debt() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![
            RewardCard::new(CardId::Rupture, 0),
            RewardCard::new(CardId::PommelStrike, 0),
        ],
        None,
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let rupture_delta = decision
        .strategic_trace
        .candidate_deltas
        .iter()
        .find(|delta| delta.action.candidate_id().contains("Rupture"))
        .expect("Rupture candidate should have a strategic delta");

    assert!(rupture_delta
        .negative
        .iter()
        .any(|delta| delta.reason == "self_damage_payoff_without_enabler"));
}

#[test]
fn card_reward_replay_exposes_strategic_delta_summary() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::Disarm, 0)],
        None,
    );
    let packet = PublicRewardDecisionPacketV1::from_context(&context);
    let replay =
        replay_card_reward_decision_v1(&packet, &CardRewardPolicyConfigV1::default(), None);

    assert!(replay.candidates[0]
        .value_summary
        .iter()
        .any(|line| line.starts_with("strategic_audit=delta_coverage:")));
    assert!(replay.candidates[0]
        .value_summary
        .iter()
        .any(|line| line.starts_with("strategic_role=")));
}
