use sts_simulator::ai::noncombat_strategy_v1::{
    threat_coverage_from_run_state_v1, StrategyThreatProfileV1, StrategyThreatSourceRecordV1,
    StrategyThreatSourceV1, StrategyThreatTagV1,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::run::RunState;

fn one_threat(
    source: StrategyThreatSourceV1,
    subject: &str,
    tag: StrategyThreatTagV1,
) -> StrategyThreatProfileV1 {
    StrategyThreatProfileV1 {
        tags: vec![tag],
        sources: vec![StrategyThreatSourceRecordV1 {
            tag,
            source,
            subject: subject.to_string(),
            evidence: format!("{subject} requires {tag:?}"),
        }],
        ..StrategyThreatProfileV1::default()
    }
}

#[test]
fn exact_deck_aoe_source_closes_three_sentries_aoe_gap() {
    let threats = one_threat(
        StrategyThreatSourceV1::ActEliteEncounter,
        "ThreeSentries",
        StrategyThreatTagV1::AoEValuable,
    );
    let starter = RunState::new(1, 0, false, "Ironclad");
    let starter_coverage = threat_coverage_from_run_state_v1(&starter, &threats);
    assert!(starter_coverage.has_gap(
        StrategyThreatSourceV1::ActEliteEncounter,
        StrategyThreatTagV1::AoEValuable
    ));

    let mut with_whirlwind = starter;
    with_whirlwind
        .master_deck
        .push(CombatCard::new(CardId::Whirlwind, 99_001));
    let patched_coverage = threat_coverage_from_run_state_v1(&with_whirlwind, &threats);
    assert!(!patched_coverage.has_gap(
        StrategyThreatSourceV1::ActEliteEncounter,
        StrategyThreatTagV1::AoEValuable
    ));
}

#[test]
fn exact_deck_strength_down_closes_high_incoming_gap() {
    let threats = one_threat(
        StrategyThreatSourceV1::ActBoss,
        "TheChamp",
        StrategyThreatTagV1::HighIncomingDamage,
    );
    let starter = RunState::new(1, 0, false, "Ironclad");
    let starter_coverage = threat_coverage_from_run_state_v1(&starter, &threats);
    assert!(starter_coverage.has_gap(
        StrategyThreatSourceV1::ActBoss,
        StrategyThreatTagV1::HighIncomingDamage
    ));

    let mut with_shockwave = starter;
    with_shockwave
        .master_deck
        .push(CombatCard::new(CardId::Shockwave, 99_002));
    let patched_coverage = threat_coverage_from_run_state_v1(&with_shockwave, &threats);
    assert!(!patched_coverage.has_gap(
        StrategyThreatSourceV1::ActBoss,
        StrategyThreatTagV1::HighIncomingDamage
    ));
}

#[test]
fn incomplete_exhaust_package_does_not_claim_long_fight_coverage() {
    let threats = one_threat(
        StrategyThreatSourceV1::ActBoss,
        "TheChamp",
        StrategyThreatTagV1::LongFightScaling,
    );
    let mut generator_only = RunState::new(1, 0, false, "Ironclad");
    generator_only.add_card_to_deck(CardId::TrueGrit);
    let incomplete = threat_coverage_from_run_state_v1(&generator_only, &threats);
    assert!(incomplete.has_gap(
        StrategyThreatSourceV1::ActBoss,
        StrategyThreatTagV1::LongFightScaling
    ));

    generator_only.add_card_to_deck(CardId::DarkEmbrace);
    let complete = threat_coverage_from_run_state_v1(&generator_only, &threats);
    assert!(!complete.has_gap(
        StrategyThreatSourceV1::ActBoss,
        StrategyThreatTagV1::LongFightScaling
    ));
}

#[test]
fn temporary_strength_does_not_claim_long_fight_coverage() {
    let threats = one_threat(
        StrategyThreatSourceV1::ActBoss,
        "TheChamp",
        StrategyThreatTagV1::LongFightScaling,
    );
    let mut with_flex = RunState::new(1, 0, false, "Ironclad");
    with_flex.add_card_to_deck(CardId::Flex);
    let coverage = threat_coverage_from_run_state_v1(&with_flex, &threats);
    assert!(coverage.has_gap(
        StrategyThreatSourceV1::ActBoss,
        StrategyThreatTagV1::LongFightScaling
    ));
}

#[test]
fn artifact_coverage_requires_multiple_independent_debuff_applications() {
    let threats = one_threat(
        StrategyThreatSourceV1::ActBoss,
        "Automaton",
        StrategyThreatTagV1::ArtifactBlocksDebuff,
    );
    let starter = RunState::new(1, 0, false, "Ironclad");
    let starter_coverage = threat_coverage_from_run_state_v1(&starter, &threats);
    assert!(starter_coverage.has_gap(
        StrategyThreatSourceV1::ActBoss,
        StrategyThreatTagV1::ArtifactBlocksDebuff
    ));

    let mut with_shockwave = starter;
    with_shockwave.add_card_to_deck(CardId::Shockwave);
    let patched_coverage = threat_coverage_from_run_state_v1(&with_shockwave, &threats);
    assert!(!patched_coverage.has_gap(
        StrategyThreatSourceV1::ActBoss,
        StrategyThreatTagV1::ArtifactBlocksDebuff
    ));
}
