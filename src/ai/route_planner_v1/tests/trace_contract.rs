use crate::ai::route_planner_v1::{
    plan_route_decision_v1, render_route_decision_trace_v1, RoutePlannerConfigV1,
    ROUTE_DECISION_TRACE_SCHEMA_NAME,
};
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
    assert!(value["candidates"][0]["score_terms"]["elite_prep"].is_number());
    assert!(value["candidates"][0]["needs"].is_object());
    assert!(value["candidates"][0]["path_summary"]["first_elite"].is_object());
}

#[test]
fn route_planner_render_shows_first_elite_segment_evidence() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );

    let rendered = render_route_decision_trace_v1(&trace);

    assert!(rendered.contains("elite_prep="));
    assert!(rendered.contains("first_elite="));
}
