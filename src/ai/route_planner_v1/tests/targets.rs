use crate::ai::route_planner_v1::{
    plan_route_decision_v1, route_targets, RouteMoveKindV1, RoutePlannerConfigV1,
};
use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::map::node::RoomType;
use crate::state::RunState;

use super::fixtures::{run_with_current_node_and_next_row, selected_candidate};

#[test]
fn route_planner_expands_wing_boots_next_floor_jumps() {
    let mut run = RunState::new(521, 0, false, "Ironclad");
    run.event_state = None;
    run.map.current_x = 0;
    run.map.current_y = 0;
    run.relics.push(RelicState::new(RelicId::WingBoots));

    let targets = route_targets(&run);

    assert!(
        targets
            .iter()
            .any(|target| target.move_kind == RouteMoveKindV1::WingBootsJump),
        "Wing Boots should add non-edge choices on the next floor when charges are available"
    );
}

#[test]
fn route_planner_preserves_wing_boots_charge_when_normal_route_is_comparable() {
    let mut run = run_with_current_node_and_next_row(
        RoomType::MonsterRoom,
        &[RoomType::MonsterRoom, RoomType::MonsterRoom],
        Some(RoomType::RestRoom),
    );
    run.relics.push(RelicState::new(RelicId::WingBoots));

    let trace = plan_route_decision_v1(
        &run,
        &EngineState::MapNavigation,
        RoutePlannerConfigV1::default(),
    );
    let selected = selected_candidate(&trace);

    assert_eq!(selected.target.move_kind, RouteMoveKindV1::NormalEdge);
    assert!(trace
        .candidates
        .iter()
        .any(|candidate| candidate.target.move_kind == RouteMoveKindV1::WingBootsJump));
}
