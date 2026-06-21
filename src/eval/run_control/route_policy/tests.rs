use super::*;
use crate::eval::run_control::commands::RunControlCommand;
use crate::eval::run_control::session::{RunControlConfig, RunControlSession};
use crate::eval::run_control::trace_annotation::RunControlTraceAnnotationV1;
use crate::state::core::EngineState;

#[test]
fn route_suggestion_is_read_only_before_map_navigation() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .apply_command(crate::eval::run_control::commands::RunControlCommand::DefaultCandidate)
        .expect("Neow intro should advance");

    let rendered = render_route_suggestion(&session);

    assert!(rendered.contains("read-only"));
    assert!(rendered.contains("route selection is locked"));
    assert!(!rendered.contains("Suggested command: go"));
}

#[test]
fn route_suggestion_recommends_without_mutating_map_position() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = None;
    session.engine_state = EngineState::MapNavigation;
    let before = (
        session.run_state.map.current_x,
        session.run_state.map.current_y,
    );

    let rendered = render_route_suggestion(&session);

    assert!(rendered.contains("Route suggestion"));
    assert!(rendered.contains("Suggested command: go"));
    assert_eq!(
        before,
        (
            session.run_state.map.current_x,
            session.run_state.map.current_y
        )
    );
}

#[test]
fn route_suggest_command_is_read_only() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = None;
    session.engine_state = EngineState::MapNavigation;
    let before = (
        session.run_state.map.current_x,
        session.run_state.map.current_y,
        session.decision_step,
    );

    let outcome = session
        .apply_command(crate::eval::run_control::commands::RunControlCommand::RouteSuggest)
        .expect("route-suggest should render");

    assert!(outcome.message.contains("Route suggestion"));
    assert!(outcome.action_result.is_none());
    assert_eq!(
        before,
        (
            session.run_state.map.current_x,
            session.run_state.map.current_y,
            session.decision_step
        )
    );
}

#[test]
fn route_go_rejects_locked_route_selection() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .apply_command(RunControlCommand::DefaultCandidate)
        .expect("Neow intro should advance");

    let err = apply_route_go(&mut session).expect_err("route-go should reject Neow bonus");

    assert!(err.contains("route-go is only valid on Map"));
    assert!(err.contains("route selection is locked"));
    assert_eq!(session.decision_step, 1);
}

#[test]
fn route_go_executes_selected_map_target() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = None;
    session.engine_state = EngineState::MapNavigation;
    let before_y = session.run_state.map.current_y;

    let outcome = apply_route_go(&mut session).expect("route-go should choose a map node");

    assert!(outcome.message.contains("Route planner selected:"));
    assert!(outcome
        .message
        .contains("label_role: behavior_policy_not_teacher"));
    assert!(outcome.action_result.is_some());
    assert!(session.run_state.map.current_y > before_y);
    assert_eq!(session.decision_step, 1);
}

#[test]
fn route_go_attaches_compact_trace_boundary() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = None;
    session.engine_state = EngineState::MapNavigation;

    let outcome = apply_route_go(&mut session).expect("route-go should choose a map node");

    assert_eq!(outcome.trace_annotations.len(), 1);
    let RunControlTraceAnnotationV1::RoutePlannerSelection {
        summary,
        selected_index,
        candidate_count,
        command,
        top_candidates,
        candidate_pool,
        label_role,
        noncombat_record,
        ..
    } = &outcome.trace_annotations[0]
    else {
        panic!("expected route planner selection annotation");
    };
    assert!(summary.contains("route planner:"));
    assert_eq!(*selected_index, Some(0));
    assert!(*candidate_count >= 1);
    assert!(command.starts_with("go ") || command.starts_with("fly "));
    assert!(!top_candidates.is_empty());
    assert!(top_candidates.len() <= 3);
    assert_eq!(candidate_pool.len(), *candidate_count);
    assert!(candidate_pool.iter().all(|candidate| {
        candidate.command.starts_with("go ") || candidate.command.starts_with("fly ")
    }));
    assert_eq!(label_role, "behavior_policy_not_teacher");
    let record = noncombat_record
        .as_ref()
        .expect("route planner annotation should carry unified noncombat record");
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
        .expect("route planner noncombat record should validate");
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Map
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::BehaviorPolicyNotTeacher
    );
}
