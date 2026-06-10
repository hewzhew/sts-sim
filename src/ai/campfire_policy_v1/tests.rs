use crate::ai::campfire_policy_v1::{build_campfire_decision_context_v1, CampfirePolicyClassV1};
use crate::state::core::CampfireChoice;
use crate::state::map::{MapEdge, MapRoomNode, MapState, RoomType};
use crate::state::run::RunState;

#[test]
fn campfire_context_exposes_rest_and_smith_candidates() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 20;
    run_state.max_hp = 80;
    install_visible_rest_route(&mut run_state);
    let context = build_campfire_decision_context_v1(
        &run_state,
        vec![CampfireChoice::Rest, CampfireChoice::Smith(0)],
    );

    assert!(context
        .candidates
        .iter()
        .any(|candidate| candidate.class == CampfirePolicyClassV1::RestRecovery));
    assert!(context
        .candidates
        .iter()
        .any(|candidate| candidate.class == CampfirePolicyClassV1::UpgradeAgency));
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
