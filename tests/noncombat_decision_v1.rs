use sts_simulator::ai::card_reward_policy_v1::{
    build_card_reward_decision_context_v1, plan_card_reward_decision_v1, CardRewardPolicyActionV1,
    CardRewardPolicyConfigV1,
};
use sts_simulator::ai::noncombat_decision_v1::{
    DataRoleV1, DecisionSiteKindV1, InformationClassV1, PolicySelectionStatusV1,
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
