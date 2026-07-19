use sts_simulator::ai::card_reward_policy_v1::{
    build_card_reward_decision_context_v1, plan_card_reward_decision_v1, CardRewardPolicyConfigV1,
    CardRewardValueSourceV1,
};
use sts_simulator::ai::noncombat_strategy_v1::{
    threat_coverage_from_run_state_v1, StrategyThreatProfileV1, StrategyThreatSourceRecordV1,
    StrategyThreatSourceV1, StrategyThreatTagV1,
};
use sts_simulator::ai::route_planner_v1::{
    plan_route_decision_v1, PathThreatExposureV1, RoutePlannerConfigV1,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::factory::EncounterId;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::EngineState;
use sts_simulator::state::map::node::{MapEdge, MapRoomNode, RoomType};
use sts_simulator::state::map::state::MapState;
use sts_simulator::state::rewards::RewardCard;
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

#[test]
fn card_reward_threat_response_stops_reinforcing_a_closed_aoe_gap() {
    let mut starter = RunState::new(1, 0, false, "Ironclad");
    starter.boss_key = Some(EncounterId::SlimeBoss);
    let starter_context = build_card_reward_decision_context_v1(
        &starter,
        vec![RewardCard::new(CardId::Cleave, 0)],
        None,
    );
    let starter_decision =
        plan_card_reward_decision_v1(&starter_context, &CardRewardPolicyConfigV1::default());
    assert!(public_heuristic_has_component(
        &starter_decision,
        "boss_threat_aoe_response"
    ));

    let mut with_whirlwind = starter;
    with_whirlwind.add_card_to_deck(CardId::Whirlwind);
    let patched_context = build_card_reward_decision_context_v1(
        &with_whirlwind,
        vec![RewardCard::new(CardId::Cleave, 0)],
        None,
    );
    let patched_decision =
        plan_card_reward_decision_v1(&patched_context, &CardRewardPolicyConfigV1::default());
    assert!(!public_heuristic_has_component(
        &patched_decision,
        "boss_threat_aoe_response"
    ));
}

#[test]
fn route_survival_envelope_reads_shared_deck_coverage() {
    let starter = one_hallway_then_rest_run();
    let starter_trace = plan_route_decision_v1(
        &starter,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let starter_envelope = &starter_trace.candidates[0].survival_envelope;
    assert_ne!(
        starter_envelope.threat_exposure,
        PathThreatExposureV1::Covered
    );
    assert!(!starter_envelope.uncovered_threats.is_empty());

    let mut prepared = starter;
    prepared.add_card_to_deck(CardId::Whirlwind);
    prepared.add_card_to_deck(CardId::Shockwave);
    let prepared_trace = plan_route_decision_v1(
        &prepared,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let prepared_envelope = &prepared_trace.candidates[0].survival_envelope;
    assert_eq!(
        prepared_envelope.threat_exposure,
        PathThreatExposureV1::Covered
    );
    assert!(prepared_envelope.uncovered_threats.is_empty());
}

fn public_heuristic_has_component(
    decision: &sts_simulator::ai::card_reward_policy_v1::CardRewardDecisionV1,
    component_name: &str,
) -> bool {
    decision.value_estimates.iter().any(|estimate| {
        estimate.source == CardRewardValueSourceV1::PublicCombatHeuristic
            && estimate
                .components
                .iter()
                .any(|component| component.name == component_name)
    })
}

fn one_hallway_then_rest_run() -> RunState {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.act_num = 2;
    run.floor_num = 18;
    run.event_state = None;
    let mut hallway = MapRoomNode::new(0, 0);
    hallway.class = Some(RoomType::MonsterRoom);
    hallway.edges.insert(MapEdge::new(0, 0, 0, 1));
    let mut rest = MapRoomNode::new(0, 1);
    rest.class = Some(RoomType::RestRoom);
    run.map = MapState::new(vec![vec![hallway], vec![rest]]);
    run
}
