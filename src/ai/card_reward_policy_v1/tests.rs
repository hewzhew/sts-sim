use crate::ai::card_reward_policy_v1::{
    plan_card_reward_decision_v1, CardRewardPolicyActionV1, CardRewardPolicyConfigV1,
};
use crate::content::cards::CardId;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

#[test]
fn policy_picks_premium_card_when_score_and_margin_are_clear() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let decision = plan_card_reward_decision_v1(
        &run_state,
        &[
            RewardCard::new(CardId::Shockwave, 0),
            RewardCard::new(CardId::Clash, 0),
            RewardCard::new(CardId::SeverSoul, 0),
        ],
        &CardRewardPolicyConfigV1::default(),
    );

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Pick {
            index: 0,
            card: CardId::Shockwave,
            ..
        }
    ));
    assert_eq!(decision.label_role, "behavior_policy_not_teacher");
    assert!(decision.candidates[0].score > decision.candidates[1].score);
}

#[test]
fn policy_stops_when_good_cards_are_too_close() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let decision = plan_card_reward_decision_v1(
        &run_state,
        &[
            RewardCard::new(CardId::PommelStrike, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
            RewardCard::new(CardId::Armaments, 0),
        ],
        &CardRewardPolicyConfigV1::default(),
    );

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
}

#[test]
fn policy_stops_on_non_premium_early_attack_reward() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let decision = plan_card_reward_decision_v1(
        &run_state,
        &[
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::SwordBoomerang, 0),
            RewardCard::new(CardId::Warcry, 0),
        ],
        &CardRewardPolicyConfigV1::default(),
    );

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert_eq!(decision.candidates[0].card, CardId::TwinStrike);
}

#[test]
fn policy_stops_when_early_attack_choice_depends_on_archetype_or_route() {
    let run_state = RunState::new(915824392, 0, false, "Ironclad");
    let decision = plan_card_reward_decision_v1(
        &run_state,
        &[
            RewardCard::new(CardId::SearingBlow, 0),
            RewardCard::new(CardId::HeavyBlade, 0),
            RewardCard::new(CardId::Clothesline, 0),
        ],
        &CardRewardPolicyConfigV1::default(),
    );

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    let searing_blow = decision
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::SearingBlow)
        .expect("Searing Blow candidate should be scored");
    assert!(searing_blow.notes.contains(&"route-upgrade-dependent"));
    let heavy_blade = decision
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::HeavyBlade)
        .expect("Heavy Blade candidate should be scored");
    assert!(heavy_blade.notes.contains(&"strength-dependent"));
}

#[test]
fn policy_stops_on_single_card_that_does_not_clear_score_gate() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let decision = plan_card_reward_decision_v1(
        &run_state,
        &[RewardCard::new(CardId::ShrugItOff, 0)],
        &CardRewardPolicyConfigV1::default(),
    );

    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
}
