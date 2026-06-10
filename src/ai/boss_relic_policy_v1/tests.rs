use crate::ai::boss_relic_policy_v1::{
    build_boss_relic_decision_context_v1, BossRelicPolicyClassV1,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;

#[test]
fn boss_relic_context_classifies_starter_relic_upgrade() {
    let run = RunState::new(7, 0, false, "Ironclad");
    let context = build_boss_relic_decision_context_v1(
        &run,
        vec![
            RelicId::Ectoplasm,
            RelicId::BlackBlood,
            RelicId::CoffeeDripper,
        ],
    );

    assert_eq!(context.candidates[1].relic, RelicId::BlackBlood);
    assert_eq!(
        context.candidates[1].class,
        BossRelicPolicyClassV1::StarterRelicUpgrade
    );
}

#[test]
fn boss_relic_context_uses_v2_cleanup_package_for_empty_cage() {
    let mut run = RunState::new(7, 0, false, "Ironclad");
    run.master_deck.push(CombatCard::new(CardId::Doubt, 9001));
    let context = build_boss_relic_decision_context_v1(
        &run,
        vec![RelicId::FusionHammer, RelicId::EmptyCage, RelicId::Sozu],
    );

    assert_eq!(context.candidates[1].relic, RelicId::EmptyCage);
    assert_eq!(
        context.candidates[1].class,
        BossRelicPolicyClassV1::DeckCleanup
    );
}

#[test]
fn boss_relic_context_marks_strategic_power_choices() {
    let run = RunState::new(7, 0, false, "Ironclad");
    let context = build_boss_relic_decision_context_v1(
        &run,
        vec![
            RelicId::TinyHouse,
            RelicId::RunicPyramid,
            RelicId::SneckoEye,
        ],
    );

    assert!(context
        .candidates
        .iter()
        .any(|candidate| candidate.class == BossRelicPolicyClassV1::StrategicPower));
}
