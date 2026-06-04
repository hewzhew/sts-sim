use crate::ai::boss_relic_policy_v1::{
    build_boss_relic_decision_context_v1, plan_boss_relic_decision_v1, BossRelicPolicyActionV1,
    BossRelicPolicyClassV1, BossRelicPolicyConfigV1,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;

#[test]
fn boss_relic_policy_picks_starter_upgrade_without_v1_access() {
    let run = RunState::new(7, 0, false, "Ironclad");
    let context = build_boss_relic_decision_context_v1(
        &run,
        vec![
            RelicId::Ectoplasm,
            RelicId::BlackBlood,
            RelicId::CoffeeDripper,
        ],
    );

    let decision = plan_boss_relic_decision_v1(&context, &BossRelicPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        BossRelicPolicyActionV1::Pick {
            index: 1,
            relic: RelicId::BlackBlood,
            ..
        }
    ));
    assert_eq!(
        decision.context.candidates[1].class,
        BossRelicPolicyClassV1::StarterRelicUpgrade
    );
    assert_eq!(
        decision.to_noncombat_decision_record_v1().selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Selected
    );
}

#[test]
fn boss_relic_policy_uses_v2_cleanup_package_for_empty_cage() {
    let mut run = RunState::new(7, 0, false, "Ironclad");
    run.master_deck.push(CombatCard::new(CardId::Doubt, 9001));
    let context = build_boss_relic_decision_context_v1(
        &run,
        vec![RelicId::FusionHammer, RelicId::EmptyCage, RelicId::Sozu],
    );

    let decision = plan_boss_relic_decision_v1(&context, &BossRelicPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        BossRelicPolicyActionV1::Pick {
            index: 1,
            relic: RelicId::EmptyCage,
            ..
        }
    ));
}

#[test]
fn boss_relic_policy_stops_for_strategic_power_choices() {
    let run = RunState::new(7, 0, false, "Ironclad");
    let context = build_boss_relic_decision_context_v1(
        &run,
        vec![
            RelicId::TinyHouse,
            RelicId::RunicPyramid,
            RelicId::SneckoEye,
        ],
    );

    let decision = plan_boss_relic_decision_v1(&context, &BossRelicPolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        BossRelicPolicyActionV1::Stop { .. }
    ));
}
