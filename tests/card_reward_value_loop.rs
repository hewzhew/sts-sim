use sts_simulator::ai::card_reward_policy_v1::{
    build_card_reward_decision_context_v1, plan_card_reward_decision_v1, CardRewardPolicyConfigV1,
    CardRewardValueSourceV1, CardRewardValueStatusV1, PublicRewardDecisionPacketV1,
};
use sts_simulator::ai::noncombat_decision_v1::{
    attach_noncombat_outcome_v1, DecisionSiteKindV1, NonCombatDecisionRecordV1,
    NonCombatOutcomeSnapshotV1, NonCombatOutcomeWindowV1, PolicySelectionStatusV1,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::eval::card_reward_value_loop::{
    calibrate_card_reward_outcomes_v1, estimate_card_reward_value_from_calibration_v1,
    estimate_card_reward_values_from_calibration_v1, extract_card_reward_value_loop_examples_v1,
    replay_card_reward_records_with_calibration_v1, summarize_card_reward_value_loop_examples_v1,
    CardRewardValueLoopOutcomeStatusV1, CardRewardValueLoopReplayStatusV1,
    CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME,
};
use sts_simulator::eval::run_control::{
    RunActionApplyStatusV1, RunActionResultV1, RunControlConfig, RunControlSession,
    RunControlTraceAnnotationV1, SessionTraceBoundaryFingerprintV1,
    SessionTraceSelectionResolution, SessionTraceStepSourceV1, SessionTraceStepV1, SessionTraceV1,
};
use sts_simulator::state::rewards::RewardCard;
use sts_simulator::state::RunState;

#[test]
fn extracts_card_reward_value_loop_example_with_attached_outcome() {
    let record = selected_card_reward_record(CardId::TwinStrike);
    let outcome = attach_noncombat_outcome_v1(
        &record,
        NonCombatOutcomeWindowV1::AfterOneFloor,
        outcome_snapshot(1, 80, 0),
        outcome_snapshot(2, 74, 1),
    )
    .expect("selected card reward record should accept outcome");
    let mut trace = trace_with_card_reward_record(record);
    trace.noncombat_outcome_attachments.push(outcome);

    let examples =
        extract_card_reward_value_loop_examples_v1(&trace).expect("trace should extract");

    assert_eq!(examples.len(), 1);
    let example = &examples[0];
    assert_eq!(
        example.schema_name,
        CARD_REWARD_VALUE_LOOP_EXAMPLE_SCHEMA_NAME
    );
    assert_eq!(example.trace_step_index, Some(0));
    assert_eq!(example.decision_site, DecisionSiteKindV1::CardReward);
    assert_eq!(
        example.replay_status,
        CardRewardValueLoopReplayStatusV1::RecordOnlyNoPublicPacket
    );
    assert_eq!(
        example.outcome_status,
        CardRewardValueLoopOutcomeStatusV1::Attached
    );
    assert_eq!(
        example.selected_candidate_id.as_deref(),
        Some("card_reward:0:TwinStrike")
    );
    assert_eq!(
        example
            .outcome
            .as_ref()
            .and_then(|outcome| outcome.card_reward.as_ref())
            .and_then(|card_reward| card_reward.next_combat_hp_loss),
        Some(6)
    );
    assert_eq!(example.source_record.candidates.len(), 1);
    assert_eq!(example.source_record.values.len(), 1);
    assert_eq!(example.label_role, "diagnostic_not_teacher_label");
    assert!(!example.trainable_as_action_label);
    assert!(!example.policy_quality_claim);
}

#[test]
fn extracts_card_reward_value_loop_example_with_missing_outcome_marker() {
    let trace = trace_with_card_reward_record(selected_card_reward_record(CardId::TwinStrike));

    let examples =
        extract_card_reward_value_loop_examples_v1(&trace).expect("trace should extract");

    assert_eq!(examples.len(), 1);
    assert_eq!(
        examples[0].outcome_status,
        CardRewardValueLoopOutcomeStatusV1::Missing
    );
    assert!(examples[0].outcome.is_none());
}

#[test]
fn extracts_card_reward_value_loop_example_with_public_packet_for_full_replay() {
    let (record, packet) = selected_card_reward_record_with_packet(CardId::TwinStrike);
    let trace = trace_with_card_reward_record_and_packet(record, Some(packet));

    let examples =
        extract_card_reward_value_loop_examples_v1(&trace).expect("trace should extract");

    assert_eq!(examples.len(), 1);
    assert_eq!(
        examples[0].replay_status,
        CardRewardValueLoopReplayStatusV1::FullPublicPacket
    );
    assert!(examples[0].public_packet.is_some());
}

#[test]
fn public_packet_trace_annotation_round_trips_through_json() {
    let (record, packet) = selected_card_reward_record_with_packet(CardId::TwinStrike);
    let trace = trace_with_card_reward_record_and_packet(record, Some(packet));

    let payload = serde_json::to_string_pretty(&trace).expect("trace should serialize");
    let restored: SessionTraceV1 =
        serde_json::from_str(&payload).expect("trace should deserialize");
    let examples =
        extract_card_reward_value_loop_examples_v1(&restored).expect("trace should extract");

    assert_eq!(examples.len(), 1);
    assert_eq!(
        examples[0].replay_status,
        CardRewardValueLoopReplayStatusV1::FullPublicPacket
    );
    assert_eq!(
        examples[0]
            .public_packet
            .as_ref()
            .unwrap()
            .context
            .candidates
            .len(),
        1
    );
}

#[test]
fn extraction_prefers_policy_packet_over_same_boundary_human_record() {
    let (policy_record, packet) = selected_card_reward_record_with_packet(CardId::TwinStrike);
    let human_record = selected_card_reward_record(CardId::Cleave);
    let mut trace = trace_with_card_reward_record_and_packet(policy_record, Some(packet));
    trace.steps[0]
        .annotations
        .push(RunControlTraceAnnotationV1::NonCombatHumanBoundary {
            record: human_record,
        });

    let examples =
        extract_card_reward_value_loop_examples_v1(&trace).expect("trace should extract");

    assert_eq!(examples.len(), 1);
    assert_eq!(
        examples[0].replay_status,
        CardRewardValueLoopReplayStatusV1::FullPublicPacket
    );
}

#[test]
fn summarizes_card_reward_value_loop_examples_without_strategy_claims() {
    let selected_record = selected_card_reward_record(CardId::TwinStrike);
    let selected_outcome = attach_noncombat_outcome_v1(
        &selected_record,
        NonCombatOutcomeWindowV1::AfterOneFloor,
        outcome_snapshot(1, 80, 0),
        outcome_snapshot(2, 74, 1),
    )
    .expect("selected card reward record should accept outcome");
    let mut selected_trace = trace_with_card_reward_record(selected_record);
    selected_trace
        .noncombat_outcome_attachments
        .push(selected_outcome);
    let missing_trace = trace_with_card_reward_record(selected_card_reward_record(CardId::Cleave));
    let mut examples =
        extract_card_reward_value_loop_examples_v1(&selected_trace).expect("trace should extract");
    examples.extend(
        extract_card_reward_value_loop_examples_v1(&missing_trace).expect("trace should extract"),
    );

    let summary = summarize_card_reward_value_loop_examples_v1(&examples);

    assert_eq!(summary.total_examples, 2);
    assert_eq!(
        histogram_count(&summary.outcome_status_counts, "attached"),
        1
    );
    assert_eq!(
        histogram_count(&summary.outcome_status_counts, "missing"),
        1
    );
    assert_eq!(
        histogram_count(&summary.selection_status_counts, "selected"),
        2
    );
    assert_eq!(
        histogram_count(
            &summary.replay_status_counts,
            "record_only_no_public_packet"
        ),
        2
    );
    assert_eq!(
        histogram_count(
            &summary.value_status_counts,
            "value_status_uncalibrated_prior"
        ),
        0
    );
    assert_eq!(
        histogram_count(&summary.evidence_gap_counts, "UncalibratedValueEstimate"),
        0
    );
    assert_eq!(summary.attached_outcome_count, 1);
    assert_eq!(summary.missing_outcome_count, 1);
    assert_eq!(summary.label_role, "diagnostic_not_teacher_label");
    assert!(!summary.trainable_as_action_label);
    assert!(!summary.policy_quality_claim);
}

#[test]
fn calibrates_selected_outcomes_into_consumable_card_id_prior() {
    let examples = vec![
        example_for_card_with_next_combat_hp_loss(CardId::TwinStrike, 4),
        example_for_card_with_next_combat_hp_loss(CardId::TwinStrike, 6),
        example_for_card_with_next_combat_hp_loss(CardId::Cleave, 12),
    ];

    let calibration = calibrate_card_reward_outcomes_v1(&examples);

    assert_eq!(calibration.total_examples, 3);
    assert_eq!(
        calibration.estimator_kind,
        "selected_outcome_card_id_prior_v1"
    );
    assert_eq!(calibration.card_id_buckets.len(), 2);
    let twin_strike = calibration
        .card_id_buckets
        .iter()
        .find(|bucket| bucket.card_id == "TwinStrike")
        .expect("TwinStrike bucket should be present");
    assert_eq!(twin_strike.selected_count, 2);
    assert_eq!(twin_strike.outcome_attached_count, 2);
    assert_eq!(twin_strike.mean_next_combat_hp_loss, Some(5.0));
    assert!(twin_strike.confidence > 0.0);
    assert!(!twin_strike.usable_for_autopilot_gate);
    assert!(twin_strike.usable_for_value_estimate);

    let run = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run,
        vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
        ],
        None,
    );
    let twin_candidate = context
        .candidates
        .iter()
        .find(|candidate| candidate.card == CardId::TwinStrike)
        .expect("TwinStrike candidate should be present");

    let estimate = estimate_card_reward_value_from_calibration_v1(twin_candidate, &calibration)
        .expect("TwinStrike should get an outcome-calibrated estimate");

    assert_eq!(estimate.index, twin_candidate.index);
    assert_eq!(estimate.card, CardId::TwinStrike);
    assert_eq!(estimate.source, CardRewardValueSourceV1::OutcomeCalibration);
    assert_eq!(estimate.status, CardRewardValueStatusV1::OutcomeCalibrated);
    assert!(estimate.survival_delta > 0.0);
    assert!(estimate.uncertainty > 0.0);
    assert!(estimate
        .components
        .iter()
        .any(|component| component.name == "outcome_sample_count" && component.value == 2.0));
}

#[test]
fn converts_outcome_calibration_into_current_context_external_estimates() {
    let examples = vec![
        example_for_card_with_next_combat_hp_loss(CardId::TwinStrike, 4),
        example_for_card_with_next_combat_hp_loss(CardId::TwinStrike, 6),
        example_for_card_with_next_combat_hp_loss(CardId::Cleave, 12),
    ];
    let calibration = calibrate_card_reward_outcomes_v1(&examples);
    let run = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run,
        vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
            RewardCard::new(CardId::Shockwave, 0),
        ],
        None,
    );

    let estimates = estimate_card_reward_values_from_calibration_v1(&context, &calibration);

    assert_eq!(estimates.len(), 2);
    assert!(estimates
        .iter()
        .any(|estimate| estimate.card == CardId::TwinStrike
            && estimate.source == CardRewardValueSourceV1::OutcomeCalibration));
    assert!(!estimates
        .iter()
        .any(|estimate| estimate.card == CardId::Shockwave));
}

#[test]
fn replays_record_level_value_changes_with_outcome_calibration() {
    let examples = vec![
        example_for_card_with_next_combat_hp_loss(CardId::TwinStrike, 4),
        example_for_card_with_next_combat_hp_loss(CardId::TwinStrike, 6),
        example_for_card_with_next_combat_hp_loss(CardId::Cleave, 12),
    ];
    let calibration = calibrate_card_reward_outcomes_v1(&examples);

    let replay = replay_card_reward_records_with_calibration_v1(&examples, &calibration);

    assert_eq!(replay.total_examples, 3);
    assert_eq!(replay.replayed_examples, 3);
    assert_eq!(replay.policy_replay_status, "record_only_no_public_packet");
    assert_eq!(replay.examples[0].candidate_replays.len(), 1);
    let candidate = &replay.examples[0].candidate_replays[0];
    assert_eq!(candidate.card_id.as_deref(), Some("TwinStrike"));
    let estimate = candidate
        .calibration_estimate
        .as_ref()
        .expect("TwinStrike should have calibration estimate");
    assert_eq!(estimate.source, "OutcomeCalibration");
    assert_eq!(estimate.status, "OutcomeCalibrated");
    assert!(estimate.survival_delta > 0.0);
    assert!(!estimate.usable_for_autopilot_gate);
    assert_eq!(replay.label_role, "diagnostic_not_teacher_label");
    assert!(!replay.policy_quality_claim);
}

#[test]
fn replays_public_packet_policy_with_outcome_calibration_inputs() {
    let examples = vec![
        example_for_card_with_next_combat_hp_loss(CardId::TwinStrike, 4),
        example_for_card_with_next_combat_hp_loss(CardId::TwinStrike, 6),
        example_for_card_with_next_combat_hp_loss(CardId::Cleave, 12),
    ];
    let calibration = calibrate_card_reward_outcomes_v1(&examples);
    let (record, packet) = selected_card_reward_record_with_packet(CardId::TwinStrike);
    let trace = trace_with_card_reward_record_and_packet(record, Some(packet));
    let public_packet_examples =
        extract_card_reward_value_loop_examples_v1(&trace).expect("trace should extract");

    let replay =
        replay_card_reward_records_with_calibration_v1(&public_packet_examples, &calibration);

    assert_eq!(replay.policy_replay_status, "full_public_packet_replay");
    assert_eq!(replay.replayed_examples, 1);
    assert_eq!(
        replay.examples[0].policy_replay_status,
        "full_public_packet_replay"
    );
    assert!(replay.examples[0]
        .policy_value_sources
        .contains(&"OutcomeCalibration".to_string()));
}

fn selected_card_reward_record(card: CardId) -> NonCombatDecisionRecordV1 {
    selected_card_reward_record_with_packet(card).0
}

fn selected_card_reward_record_with_packet(
    card: CardId,
) -> (NonCombatDecisionRecordV1, PublicRewardDecisionPacketV1) {
    let run = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(&run, vec![RewardCard::new(card, 0)], None);
    let packet = PublicRewardDecisionPacketV1::from_context(&context);
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let mut record = decision.to_noncombat_decision_record_v1();
    record.selection.status = PolicySelectionStatusV1::Selected;
    record.selection.selected_candidate_id = Some(format!("card_reward:0:{card:?}"));
    record.selection.reason = "test selected visible card reward".to_string();
    record.selection.confidence = 1.0;
    (record, packet)
}

fn example_for_card_with_next_combat_hp_loss(
    card: CardId,
    hp_loss: i32,
) -> sts_simulator::eval::card_reward_value_loop::CardRewardValueLoopExampleV1 {
    let record = selected_card_reward_record(card);
    let outcome = attach_noncombat_outcome_v1(
        &record,
        NonCombatOutcomeWindowV1::AfterOneFloor,
        outcome_snapshot(1, 80, 0),
        outcome_snapshot(2, 80 - hp_loss, 1),
    )
    .expect("selected card reward record should accept outcome");
    let mut trace = trace_with_card_reward_record(record);
    trace.noncombat_outcome_attachments.push(outcome);
    extract_card_reward_value_loop_examples_v1(&trace)
        .expect("trace should extract")
        .into_iter()
        .next()
        .expect("one card reward example should be extracted")
}

fn trace_with_card_reward_record(record: NonCombatDecisionRecordV1) -> SessionTraceV1 {
    trace_with_card_reward_record_and_packet(record, None)
}

fn trace_with_card_reward_record_and_packet(
    record: NonCombatDecisionRecordV1,
    public_packet: Option<PublicRewardDecisionPacketV1>,
) -> SessionTraceV1 {
    let session = RunControlSession::new(RunControlConfig::default());
    let mut trace = SessionTraceV1::new(&session);
    let boundary = boundary();
    trace.steps.push(SessionTraceStepV1 {
        step_index: 0,
        step_source: SessionTraceStepSourceV1::ManualOrAutomation,
        raw_command_line: "0".to_string(),
        decision_step_before: 1,
        decision_step_after: 2,
        screen_title: "Card Reward".to_string(),
        decision_kind: "CardReward".to_string(),
        before: boundary.clone(),
        after: boundary,
        visible_candidates: Vec::new(),
        selected_candidate: None,
        selection_resolution: SessionTraceSelectionResolution::Unresolved,
        annotations: vec![RunControlTraceAnnotationV1::NonCombatPolicyDecision {
            record,
            card_reward_packet: public_packet,
        }],
        action_result: RunActionResultV1 {
            chosen_label: "Twin Strike".to_string(),
            status: RunActionApplyStatusV1::Running,
            changes: Vec::new(),
        },
    });
    trace
}

fn boundary() -> SessionTraceBoundaryFingerprintV1 {
    SessionTraceBoundaryFingerprintV1 {
        decision_step: 1,
        engine_state: "RewardScreen".to_string(),
        active_combat_engine_state: None,
        screen_title: "Card Reward".to_string(),
        decision_kind: "CardReward".to_string(),
        decision_label: "Choose a card or skip".to_string(),
        act: 1,
        floor: 1,
        current_hp: 80,
        max_hp: 80,
        gold: 99,
        boss: "The Guardian".to_string(),
        candidate_count: 1,
        candidate_set_hash: "candidate-set".to_string(),
        candidate_order_hash: "candidate-order".to_string(),
        combat: None,
    }
}

fn outcome_snapshot(
    floor: i32,
    current_hp: i32,
    combats_completed: u32,
) -> NonCombatOutcomeSnapshotV1 {
    NonCombatOutcomeSnapshotV1 {
        act: 1,
        floor,
        current_hp,
        max_hp: 80,
        gold: 99,
        deck_size: 11,
        relic_count: 1,
        potion_count: 0,
        combats_completed,
        elites_completed: 0,
        bosses_completed: 0,
        run_terminal: None,
    }
}

fn histogram_count(
    entries: &[sts_simulator::eval::card_reward_value_loop::HistogramEntryV1],
    key: &str,
) -> usize {
    entries
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.count)
        .unwrap_or_default()
}
