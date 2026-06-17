use crate::ai::card_reward_policy_v1::{
    build_card_reward_decision_context_v1, plan_card_reward_decision_v1,
    replay_card_reward_decision_v1, CardRewardPolicyConfigV1, PublicRewardDecisionPacketV1,
};
use crate::ai::strategic::{
    add_startup_profile_pressure_to_ledger, compile_decision, ledger_from_snapshot,
    CandidateAction, CandidateDelta, LedgerDelta, PressureHorizon, PressureKind, PressureLedger,
    StrategicDebt, StrategicDecisionSite, StrategicDeckFacts, StrategicJob, StrategicSnapshot,
};
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

#[test]
fn startup_profile_pressure_records_snecko_low_cost_volatility() {
    let mut ledger = PressureLedger::default();
    let startup = crate::ai::deck_startup_profile_v1::DeckStartupProfileV1 {
        has_snecko_eye: true,
        has_snecko_low_cost_volatility: true,
        low_cost_card_count: 14,
        high_cost_card_count: 3,
        snecko_random_cost_debt: 2,
        ..Default::default()
    };

    add_startup_profile_pressure_to_ledger(&mut ledger, &startup);

    assert!(ledger.items.iter().any(|item| {
        item.id == "deck_debt:snecko_low_cost_volatility"
            && item.kind == PressureKind::DeckDebt(StrategicDebt::SetupDebt)
            && item
                .evidence
                .iter()
                .any(|line| line.contains("low_cost_cards=14"))
    }));
}

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
fn card_reward_shadow_trace_records_startup_and_shape_debt() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.add_card_to_deck(CardId::WildStrike);
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::WildStrike, 0)],
        None,
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let wild_strike_delta = decision
        .strategic_trace
        .candidate_deltas
        .iter()
        .find(|delta| delta.action.candidate_id().contains("WildStrike"))
        .expect("Wild Strike candidate should have a strategic delta");

    assert!(wild_strike_delta.negative.iter().any(|delta| {
        delta.reason == "startup_rejects_status_generator_duplicate_without_digest"
            && delta.kind == PressureKind::DeckDebt(StrategicDebt::CombatShapeRisk)
    }));
    assert!(wild_strike_delta
        .evidence
        .contains(&"deck_shape_status_generator_duplicate_without_digest".to_string()));
}

#[test]
fn card_component_strength_down_maps_to_enemy_strength_pressure() {
    let run_state = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::Disarm, 0)],
        None,
    );

    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let disarm_delta = decision
        .strategic_trace
        .candidate_deltas
        .iter()
        .find(|delta| delta.action.candidate_id().contains("Disarm"))
        .expect("Disarm candidate should have a strategic delta");
    let direct_strength_down = disarm_delta
        .positive
        .iter()
        .find(|delta| delta.reason == "direct_strength_down_answer")
        .expect("Disarm component report should include direct strength-down answer");

    assert_eq!(
        direct_strength_down.kind,
        PressureKind::MissingJob(StrategicJob::EnemyStrengthDown),
        "component reason mapping should not classify strength-down as generic scaling"
    );
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

#[test]
fn strategic_compiler_prefers_candidate_matching_active_pressure() {
    let snapshot = StrategicSnapshot {
        site: StrategicDecisionSite::CardReward,
        act: 1,
        floor: 3,
        boss: None,
        hp: 80,
        max_hp: 80,
        gold: 99,
        deck: StrategicDeckFacts::default(),
        route: None,
        formation_needs: vec![StrategicJob::Block],
    };
    let mut ledger = PressureLedger::default();
    ledger.push(
        "missing_job:block",
        PressureKind::MissingJob(StrategicJob::Block),
        PressureHorizon::VisibleRoute,
        1.0,
        1.0,
        vec!["test pressure".to_string()],
    );

    let mut frontload = CandidateDelta::empty(CandidateAction::Unknown {
        id: "frontload".to_string(),
        label: "frontload".to_string(),
    });
    frontload.positive.push(LedgerDelta {
        kind: PressureKind::MissingJob(StrategicJob::Frontload),
        amount: 0.60,
        reason: "frontload_delta".to_string(),
    });

    let mut block = CandidateDelta::empty(CandidateAction::Unknown {
        id: "block".to_string(),
        label: "block".to_string(),
    });
    block.positive.push(LedgerDelta {
        kind: PressureKind::MissingJob(StrategicJob::Block),
        amount: 0.60,
        reason: "block_delta".to_string(),
    });

    let trace = compile_decision(snapshot, ledger, 2, vec![frontload, block]);
    let frontload_score = trace
        .compiled
        .iter()
        .find(|decision| decision.action.candidate_id() == "frontload")
        .expect("frontload candidate should compile")
        .score;
    let block_score = trace
        .compiled
        .iter()
        .find(|decision| decision.action.candidate_id() == "block")
        .expect("block candidate should compile")
        .score;

    assert!(
        block_score > frontload_score,
        "equal raw deltas should be ordered by active pressure alignment"
    );
    assert_eq!(
        trace
            .would_choose
            .expect("trace should choose a non-rejected candidate")
            .candidate_id(),
        "block"
    );
}

#[test]
fn strategic_compiler_amplifies_debt_that_matches_active_pressure() {
    let snapshot = StrategicSnapshot {
        site: StrategicDecisionSite::CardReward,
        act: 2,
        floor: 24,
        boss: None,
        hp: 60,
        max_hp: 80,
        gold: 200,
        deck: StrategicDeckFacts::default(),
        route: None,
        formation_needs: vec![],
    };

    let mut clean = CandidateDelta::empty(CandidateAction::Unknown {
        id: "clean_block".to_string(),
        label: "clean_block".to_string(),
    });
    clean.positive.push(LedgerDelta {
        kind: PressureKind::MissingJob(StrategicJob::Block),
        amount: 0.50,
        reason: "block_delta".to_string(),
    });

    let mut bloated = CandidateDelta::empty(CandidateAction::Unknown {
        id: "bloated_block".to_string(),
        label: "bloated_block".to_string(),
    });
    bloated.positive.push(LedgerDelta {
        kind: PressureKind::MissingJob(StrategicJob::Block),
        amount: 0.50,
        reason: "block_delta".to_string(),
    });
    bloated.negative.push(LedgerDelta {
        kind: PressureKind::DeckDebt(StrategicDebt::CycleTime),
        amount: 0.20,
        reason: "adds_cycle_card".to_string(),
    });

    let empty_trace = compile_decision(
        snapshot.clone(),
        PressureLedger::default(),
        2,
        vec![clean.clone(), bloated.clone()],
    );
    let mut pressured_ledger = PressureLedger::default();
    pressured_ledger.push(
        "deck_debt:cycle_time",
        PressureKind::DeckDebt(StrategicDebt::CycleTime),
        PressureHorizon::LongTerm,
        1.0,
        1.0,
        vec!["test cycle pressure".to_string()],
    );
    let pressured_trace = compile_decision(snapshot, pressured_ledger, 2, vec![clean, bloated]);

    let empty_gap =
        compiled_score(&empty_trace, "clean_block") - compiled_score(&empty_trace, "bloated_block");
    let pressured_gap = compiled_score(&pressured_trace, "clean_block")
        - compiled_score(&pressured_trace, "bloated_block");

    assert!(
        pressured_gap > empty_gap,
        "active cycle pressure should amplify candidates that add cycle debt"
    );
    assert!(pressured_trace
        .compiled
        .iter()
        .find(|decision| decision.action.candidate_id() == "bloated_block")
        .expect("bloated candidate should compile")
        .reasons
        .iter()
        .any(|reason| reason.starts_with("-ledger_pressure:")));
}

#[test]
fn pressure_ledger_exposes_access_and_package_debts_from_deck_facts() {
    let snapshot = StrategicSnapshot {
        site: StrategicDecisionSite::CardReward,
        act: 2,
        floor: 25,
        boss: None,
        hp: 60,
        max_hp: 80,
        gold: 200,
        deck: StrategicDeckFacts {
            deck_size: 26,
            draw_sources: 1,
            status_generators: 2,
            status_payoffs: 0,
            exhaust_generators: 0,
            exhaust_payoffs: 1,
            strength_sources: 0,
            strength_payoffs: 1,
            ..StrategicDeckFacts::default()
        },
        route: None,
        formation_needs: vec![],
    };

    let ledger = ledger_from_snapshot(&snapshot);

    assert!(ledger
        .items
        .iter()
        .any(|item| item.id == "deck_debt:low_access_large_deck"));
    assert!(ledger
        .items
        .iter()
        .any(|item| item.id == "deck_debt:status_without_digest"));
    assert!(ledger
        .items
        .iter()
        .any(|item| item.id == "deck_debt:exhaust_payoff_without_enabler"));
    assert!(ledger
        .items
        .iter()
        .any(|item| item.id == "deck_debt:strength_payoff_without_source"));
}

#[test]
fn card_reward_strategic_snapshot_preserves_convertible_strength_facts() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.master_deck.clear();
    run_state.add_card_to_deck(CardId::Flex);
    run_state.add_card_to_deck(CardId::LimitBreak);
    run_state.add_card_to_deck(CardId::HeavyBlade);

    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::PommelStrike, 0)],
        None,
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert_eq!(decision.strategic_trace.snapshot.deck.strength_sources, 0);
    assert_eq!(
        decision
            .strategic_trace
            .snapshot
            .deck
            .temporary_strength_bursts,
        1
    );
    assert_eq!(
        decision.strategic_trace.snapshot.deck.strength_converters,
        1
    );
    assert_eq!(
        decision
            .strategic_trace
            .snapshot
            .deck
            .convertible_strength_sources,
        1
    );
}

#[test]
fn pressure_ledger_distinguishes_convertible_strength_from_stable_scaling() {
    let snapshot = StrategicSnapshot {
        site: StrategicDecisionSite::CardReward,
        act: 2,
        floor: 25,
        boss: None,
        hp: 60,
        max_hp: 80,
        gold: 200,
        deck: StrategicDeckFacts {
            strength_sources: 0,
            temporary_strength_bursts: 1,
            strength_converters: 1,
            convertible_strength_sources: 1,
            strength_payoffs: 2,
            ..StrategicDeckFacts::default()
        },
        route: None,
        formation_needs: vec![],
    };

    let ledger = ledger_from_snapshot(&snapshot);

    assert!(ledger
        .items
        .iter()
        .any(|item| item.id == "deck_debt:strength_payoff_without_stable_source"));
    assert!(!ledger
        .items
        .iter()
        .any(|item| item.id == "deck_debt:strength_payoff_without_source"));
}

#[test]
fn collector_pressure_and_aoe_candidate_share_boss_tax_kind() {
    let mut run_state = RunState::new(521, 0, false, "Ironclad");
    run_state.boss_key = Some(crate::content::monsters::factory::EncounterId::Collector);
    let context = build_card_reward_decision_context_v1(
        &run_state,
        vec![RewardCard::new(CardId::Immolate, 0)],
        None,
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    assert!(decision
        .strategic_trace
        .ledger
        .items
        .iter()
        .any(|item| item.id == "boss_tax:collector_minion_plan"));

    let immolate_delta = decision
        .strategic_trace
        .candidate_deltas
        .iter()
        .find(|delta| delta.action.candidate_id().contains("Immolate"))
        .expect("Immolate should have a strategic delta");

    assert!(immolate_delta.positive.iter().any(|delta| {
        delta.kind
            == PressureKind::BossTax(crate::ai::strategic::StrategicBossTax::CollectorMinionPlan)
    }));
}

fn compiled_score(trace: &crate::ai::strategic::StrategicDecisionTrace, id: &str) -> f32 {
    trace
        .compiled
        .iter()
        .find(|decision| decision.action.candidate_id() == id)
        .unwrap_or_else(|| panic!("{id} candidate should compile"))
        .score
}
