use sts_simulator::ai::card_reward_policy_v1::{
    build_card_reward_decision_context_v1, plan_card_reward_decision_v1, CardRewardPolicyActionV1,
    CardRewardPolicyConfigV1,
};
use sts_simulator::ai::noncombat_decision_v1::{
    attach_noncombat_outcome_v1, compare_noncombat_decision_records_v1, DataRoleV1,
    DecisionSiteKindV1, InformationClassV1, NonCombatOutcomeSnapshotV1, NonCombatOutcomeWindowV1,
    NonCombatReplayCandidateSetStatusV1, PolicySelectionStatusV1,
    NONCOMBAT_DECISION_RECORD_SCHEMA_NAME,
};
use sts_simulator::ai::route_planner_v1::{plan_route_decision_v1, RoutePlannerConfigV1};
use sts_simulator::content::cards::CardId;
use sts_simulator::state::core::EngineState;
use sts_simulator::state::rewards::RewardCard;
use sts_simulator::state::RunState;

#[test]
fn route_trace_exports_hidden_free_noncombat_record() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );

    let record = trace.to_noncombat_decision_record_v1();

    assert_eq!(record.schema_name, NONCOMBAT_DECISION_RECORD_SCHEMA_NAME);
    assert_eq!(record.site, DecisionSiteKindV1::Map);
    assert_eq!(record.data_role, DataRoleV1::BehaviorPolicyNotTeacher);
    assert!(!record.information_boundary.hidden_simulator_state_used);
    assert!(record
        .information_boundary
        .allowed_inputs
        .contains(&InformationClassV1::PublicObservation));
    assert!(record
        .information_boundary
        .allowed_inputs
        .contains(&InformationClassV1::KnownDistribution));
    assert!(record
        .information_boundary
        .allowed_inputs
        .contains(&InformationClassV1::Belief));
    assert!(record
        .information_boundary
        .forbidden_inputs
        .contains(&InformationClassV1::HiddenSimulatorState));
    assert_eq!(record.candidates.len(), trace.candidates.len());
    assert_eq!(record.values.len(), trace.candidates.len());
    assert_eq!(record.selection.status, PolicySelectionStatusV1::Selected);
    assert!(record.selection.selected_candidate_id.is_some());
}

#[test]
fn card_reward_stop_exports_hidden_free_uncalibrated_value_inputs() {
    let run = RunState::new(521, 0, false, "Ironclad");
    let route_trace = plan_route_decision_v1(&run, &EngineState::MapNavigation, Default::default());
    let context = build_card_reward_decision_context_v1(
        &run,
        vec![
            RewardCard::new(CardId::Shockwave, 0),
            RewardCard::new(CardId::Clash, 0),
            RewardCard::new(CardId::SeverSoul, 0),
        ],
        Some(&route_trace),
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    let record = decision.to_noncombat_decision_record_v1();

    assert_eq!(record.schema_name, NONCOMBAT_DECISION_RECORD_SCHEMA_NAME);
    assert_eq!(record.site, DecisionSiteKindV1::CardReward);
    assert_eq!(record.data_role, DataRoleV1::BehaviorPolicyNotTeacher);
    assert!(!record.information_boundary.hidden_simulator_state_used);
    assert_eq!(record.candidates.len(), decision.candidates.len());
    assert_eq!(record.values.len(), decision.candidates.len());
    assert!(record.values.iter().all(|value| value.confidence == 0.0));
    assert!(record.values.iter().all(|value| {
        value
            .components
            .iter()
            .any(|component| component.name == "value_status_uncalibrated_prior")
    }));
    assert!(matches!(
        decision.action,
        CardRewardPolicyActionV1::Stop { .. }
    ));
    assert_eq!(record.selection.status, PolicySelectionStatusV1::Stopped);
    assert!(record.selection.selected_candidate_id.is_none());
    assert_eq!(record.selection.selection_mode, "pick_certificate_gate");
}

#[test]
fn card_reward_stop_exports_noncombat_record_without_selected_candidate() {
    let run = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run,
        vec![
            RewardCard::new(CardId::PommelStrike, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
            RewardCard::new(CardId::Armaments, 0),
        ],
        None,
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());

    let record = decision.to_noncombat_decision_record_v1();

    assert_eq!(record.site, DecisionSiteKindV1::CardReward);
    assert_eq!(record.data_role, DataRoleV1::BehaviorPolicyNotTeacher);
    assert_eq!(record.selection.status, PolicySelectionStatusV1::Stopped);
    assert!(record.selection.selected_candidate_id.is_none());
    assert!(!record.candidates.is_empty());
    assert!(!record.selection.reason.is_empty());
}

#[test]
fn decision_replay_comparison_detects_selection_and_value_changes_without_hidden_inputs() {
    let run = RunState::new(521, 0, false, "Ironclad");
    let route_trace = plan_route_decision_v1(&run, &EngineState::MapNavigation, Default::default());
    let context = build_card_reward_decision_context_v1(
        &run,
        vec![
            RewardCard::new(CardId::Shockwave, 0),
            RewardCard::new(CardId::Clash, 0),
            RewardCard::new(CardId::SeverSoul, 0),
        ],
        Some(&route_trace),
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let old_record = decision.to_noncombat_decision_record_v1();
    let mut new_record = old_record.clone();
    new_record.provenance.source_policy = "card_reward_policy_v1_experiment".to_string();
    new_record.selection.reason = "changed stop reason for replay test".to_string();
    new_record.values[0].confidence = 0.5;

    let report = compare_noncombat_decision_records_v1(&old_record, &new_record)
        .expect("hidden-free records should be replay-comparable");

    assert_eq!(report.site, DecisionSiteKindV1::CardReward);
    assert_eq!(
        report.candidate_set.status,
        NonCombatReplayCandidateSetStatusV1::Unchanged
    );
    assert!(report.selection_changed);
    assert_eq!(report.value_deltas.len(), old_record.values.len());
    assert!(report
        .value_deltas
        .iter()
        .any(|delta| delta.confidence_delta > 0.0));
    assert_eq!(report.label_role, "diagnostic_not_teacher_label");
    assert!(!report.policy_quality_claim);
}

#[test]
fn outcome_attachment_records_public_short_horizon_deltas() {
    let run = RunState::new(521, 0, false, "Ironclad");
    let context = build_card_reward_decision_context_v1(
        &run,
        vec![RewardCard::new(CardId::TwinStrike, 0)],
        None,
    );
    let decision = plan_card_reward_decision_v1(&context, &CardRewardPolicyConfigV1::default());
    let record = decision.to_noncombat_decision_record_v1();

    let before = NonCombatOutcomeSnapshotV1 {
        act: 1,
        floor: 1,
        current_hp: 80,
        max_hp: 80,
        gold: 99,
        deck_size: 10,
        relic_count: 1,
        potion_count: 0,
        combats_completed: 0,
        elites_completed: 0,
        bosses_completed: 0,
        run_terminal: None,
    };
    let after = NonCombatOutcomeSnapshotV1 {
        floor: 2,
        current_hp: 74,
        gold: 118,
        deck_size: 11,
        potion_count: 1,
        combats_completed: 1,
        ..before.clone()
    };

    let attachment = attach_noncombat_outcome_v1(
        &record,
        NonCombatOutcomeWindowV1::AfterOneFloor,
        before,
        after,
    )
    .expect("valid hidden-free decision record should accept public outcome attachment");

    assert_eq!(attachment.site, DecisionSiteKindV1::CardReward);
    assert_eq!(attachment.window, NonCombatOutcomeWindowV1::AfterOneFloor);
    assert!(!attachment.decision_record_hash.is_empty());
    assert_eq!(attachment.metrics.floor_delta, 1);
    assert_eq!(attachment.metrics.hp_delta, -6);
    assert_eq!(attachment.metrics.gold_delta, 19);
    assert_eq!(attachment.metrics.deck_size_delta, 1);
    assert_eq!(attachment.metrics.combats_completed_delta, 1);
    assert_eq!(attachment.label_role, "diagnostic_not_teacher_label");
    assert!(!attachment.trainable_as_action_label);
    assert!(!attachment.policy_quality_claim);
}
