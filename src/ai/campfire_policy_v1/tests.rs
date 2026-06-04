use crate::ai::campfire_policy_v1::{
    build_campfire_decision_context_v1, plan_campfire_decision_v1, CampfirePolicyActionV1,
    CampfirePolicyConfigV1,
};
use crate::state::core::CampfireChoice;
use crate::state::map::{MapEdge, MapRoomNode, MapState, RoomType};
use crate::state::run::RunState;

#[test]
fn campfire_policy_rests_under_recovery_pressure() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 20;
    run_state.max_hp = 80;
    install_visible_rest_route(&mut run_state);
    let context = build_campfire_decision_context_v1(
        &run_state,
        vec![CampfireChoice::Rest, CampfireChoice::Smith(0)],
    );

    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CampfirePolicyActionV1::Rest { .. }
    ));
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(
        &decision.to_noncombat_decision_record_v1(),
    )
    .expect("campfire policy record should validate");
}

#[test]
fn campfire_policy_stops_when_hp_is_full() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 80;
    run_state.max_hp = 80;
    let context = build_campfire_decision_context_v1(&run_state, vec![CampfireChoice::Rest]);

    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CampfirePolicyActionV1::Stop { .. }
    ));
}

fn install_visible_rest_route(run_state: &mut RunState) {
    let mut rest = MapRoomNode::new(0, 0);
    rest.class = Some(RoomType::RestRoom);
    rest.edges.insert(MapEdge::new(0, 0, 0, 1));
    let mut next = MapRoomNode::new(0, 1);
    next.class = Some(RoomType::MonsterRoom);
    run_state.map = MapState::new(vec![vec![rest], vec![next]]);
    run_state.map.current_x = 0;
    run_state.map.current_y = 0;
}
