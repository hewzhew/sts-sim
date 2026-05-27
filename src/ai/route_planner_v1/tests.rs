use crate::ai::route_planner_v1::{
    plan_route_decision_v1, route_targets, RoutePlannerConfigV1, RouteSafetyFlagV1,
    ROUTE_DECISION_TRACE_SCHEMA_NAME,
};
use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::RunState;

#[test]
fn route_planner_trace_is_behavior_policy_not_teacher_label() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );

    assert_eq!(trace.schema_name, ROUTE_DECISION_TRACE_SCHEMA_NAME);
    assert_eq!(trace.label_role, "behavior_policy_not_teacher");
    assert!(!trace.candidates.is_empty());
    assert!(trace.selected_index.is_some());
    assert!(trace.candidates.iter().all(|candidate| {
        candidate
            .suggested_command
            .as_deref()
            .is_some_and(|command| command.starts_with("go "))
    }));
}

#[test]
fn route_planner_does_not_emit_executable_command_when_map_locked() {
    let run = RunState::new(521, 0, false, "Ironclad");
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::EventRoom,
        RoutePlannerConfigV1::default(),
    );

    assert!(trace
        .warnings
        .iter()
        .any(|warning| warning.contains("locked")));
    assert!(trace
        .candidates
        .iter()
        .all(|candidate| candidate.suggested_command.is_none()));
}

#[test]
fn route_planner_trace_serializes_structured_evidence() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );

    let value = serde_json::to_value(&trace).expect("trace should serialize");

    assert_eq!(value["schema_name"], ROUTE_DECISION_TRACE_SCHEMA_NAME);
    assert_eq!(value["label_role"], "behavior_policy_not_teacher");
    assert!(value["candidates"][0]["score_terms"].is_object());
    assert!(value["candidates"][0]["needs"].is_object());
}

#[test]
fn route_planner_expands_wing_boots_next_floor_jumps() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.map.current_x = 0;
    run.map.current_y = 0;
    run.relics.push(RelicState::new(RelicId::WingBoots));

    let targets = route_targets(&run);

    assert!(
        targets.iter().any(|target| target.move_kind
            == crate::ai::route_planner_v1::RouteMoveKindV1::WingBootsJump),
        "Wing Boots should add non-edge choices on the next floor when charges are available"
    );
}

#[test]
fn route_planner_can_gate_obvious_forced_elite_risk() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.current_hp = 1;
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );

    assert!(trace.candidates.iter().any(|candidate| {
        matches!(
            candidate.safety,
            RouteSafetyFlagV1::Ok
                | RouteSafetyFlagV1::RiskyButAllowed
                | RouteSafetyFlagV1::RejectUnlessNoAlternative
        )
    }));
}
