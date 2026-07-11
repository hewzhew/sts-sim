use crate::ai::campfire_policy_v1::{
    build_campfire_decision_context_v1, plan_campfire_decision_v1, CampfirePlanRoleV1,
    CampfirePolicyActionV1, CampfirePolicyClassV1, CampfirePolicyConfigV1,
};
use crate::ai::deck_repair_profile_v1::DeckRepairUpgradePriorityV1;
use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;
use crate::content::relics::{RelicId, RelicState};
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

#[test]
fn campfire_deck_mutation_targets_are_sourced_from_deck_mutation_compiler() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let context = build_campfire_decision_context_v1(
        &run_state,
        vec![CampfireChoice::Smith(0), CampfireChoice::Toke(0)],
    );

    let smith = context
        .candidates
        .iter()
        .find(|candidate| matches!(candidate.choice, CampfireChoice::Smith(_)))
        .expect("expected smith candidate");
    let toke = context
        .candidates
        .iter()
        .find(|candidate| matches!(candidate.choice, CampfireChoice::Toke(_)))
        .expect("expected toke candidate");

    assert!(
        smith
            .evidence
            .iter()
            .any(|item| item.contains("DeckMutationCompilerV1")),
        "campfire smith targets must come from the deck mutation compiler boundary"
    );
    assert!(
        toke.evidence
            .iter()
            .any(|item| item.contains("DeckMutationCompilerV1")),
        "campfire toke targets must come from the deck mutation compiler boundary"
    );
}

#[test]
fn campfire_smith_candidate_exposes_boss_strategy_tag() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.act_num = 2;
    run_state.boss_key = Some(EncounterId::TheChamp);
    run_state.add_card_to_deck(CardId::TrueGrit);
    let true_grit_index = run_state
        .master_deck
        .iter()
        .position(|card| card.id == CardId::TrueGrit)
        .expect("test deck should contain True Grit");

    let context = build_campfire_decision_context_v1(
        &run_state,
        vec![CampfireChoice::Smith(true_grit_index)],
    );
    let true_grit = context
        .candidates
        .iter()
        .find(|candidate| matches!(candidate.choice, CampfireChoice::Smith(idx) if idx == true_grit_index))
        .expect("expected True Grit smith candidate");

    assert!(
        true_grit
            .evidence
            .iter()
            .any(|item| item == "smith strategy tag is upgrade_debt:execute_block"),
        "smith candidate should expose boss strategy tag, got {:?}",
        true_grit.evidence
    );
}

#[test]
fn campfire_decision_selects_from_candidate_plan_pool() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 70;
    run_state.max_hp = 80;
    install_current_room_route(
        &mut run_state,
        RoomType::RestRoom,
        &[RoomType::MonsterRoom, RoomType::MonsterRoomElite],
    );
    let context = build_campfire_decision_context_v1(
        &run_state,
        vec![CampfireChoice::Rest, CampfireChoice::Smith(0)],
    );

    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert_eq!(decision.action, decision.selected_plan.action);
    assert_eq!(
        decision.selected_plan.role,
        CampfirePlanRoleV1::PolicyPreferred
    );
    assert!(decision
        .candidate_plans
        .iter()
        .any(|candidate| candidate.plan_id == decision.selected_plan.plan_id));
}

#[test]
fn campfire_policy_consumes_rest_vs_smith_rest_favored_verdict() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 35;
    run_state.max_hp = 80;
    let bash_index = run_state
        .master_deck
        .iter()
        .position(|card| card.id == CardId::Bash)
        .expect("Ironclad starter deck should include Bash");

    let context = build_campfire_decision_context_v1(
        &run_state,
        vec![CampfireChoice::Rest, CampfireChoice::Smith(bash_index)],
    );
    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CampfirePolicyActionV1::Rest { .. }
    ));
    assert!(decision
        .selected_plan
        .reasons
        .iter()
        .any(|reason| reason.contains("Rest favored by rest-vs-smith plan")));
}

#[test]
fn campfire_policy_respects_deck_mutation_execute_gate_for_smith_targets() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = run_state.max_hp;
    let bash_index = run_state
        .master_deck
        .iter()
        .position(|card| card.id == CardId::Bash)
        .expect("Ironclad starter deck should include Bash");
    let mut context =
        build_campfire_decision_context_v1(&run_state, vec![CampfireChoice::Smith(bash_index)]);
    let smith = context
        .candidates
        .iter_mut()
        .find(
            |candidate| matches!(candidate.choice, CampfireChoice::Smith(idx) if idx == bash_index),
        )
        .expect("expected Bash smith candidate");
    smith.deck_mutation_execute_allowed = Some(false);

    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert!(
        matches!(decision.action, CampfirePolicyActionV1::Stop { .. }),
        "campfire must not re-enable a smith target blocked by DeckMutationCompiler: {:?}",
        decision.action
    );
}

#[test]
fn campfire_policy_uses_full_hp_rest_as_empty_campfire_exit() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = run_state.max_hp;
    run_state
        .relics
        .push(RelicState::new(RelicId::FusionHammer));

    let context = build_campfire_decision_context_v1(
        &run_state,
        crate::engine::campfire_handler::get_available_options(&run_state),
    );
    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CampfirePolicyActionV1::Rest { .. }
    ));
    assert!(decision
        .selected_plan
        .reasons
        .iter()
        .any(|reason| reason.contains("campfire exit")));
}

#[test]
fn reliability_repair_smith_precedes_generic_growth_when_safe() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = run_state.max_hp;
    run_state.master_deck =
        std::iter::once(crate::runtime::combat::CombatCard::new(CardId::Cleave, 1))
            .chain((0..5).map(|index| {
                crate::runtime::combat::CombatCard::new(CardId::Apparition, 100 + index)
            }))
            .collect();
    let is_apparition_smith = |choice: CampfireChoice| match choice {
        CampfireChoice::Smith(index) => run_state
            .master_deck
            .get(index)
            .is_some_and(|card| card.id == CardId::Apparition),
        _ => false,
    };

    let context = build_campfire_decision_context_v1(&run_state, vec![CampfireChoice::Smith(0)]);
    assert!(context.candidates.iter().any(|candidate| {
        is_apparition_smith(candidate.choice)
            && candidate.repair_priority == Some(DeckRepairUpgradePriorityV1::Reliability)
            && candidate.strategy_tag.as_deref() == Some("deck_repair:reliability")
    }));

    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    let selected_index = match decision.action {
        CampfirePolicyActionV1::Smith { deck_index, .. } => deck_index,
        other => panic!("expected Apparition Smith, got {other:?}"),
    };
    assert!(run_state
        .master_deck
        .get(selected_index)
        .is_some_and(|card| card.id == CardId::Apparition));
    assert_eq!(
        decision.selected_plan.repair_priority,
        Some(DeckRepairUpgradePriorityV1::Reliability)
    );
}

#[test]
fn rest_favored_still_blocks_reliability_repair_smith() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 35;
    run_state.max_hp = 80;
    run_state.master_deck =
        std::iter::once(crate::runtime::combat::CombatCard::new(CardId::Cleave, 1))
            .chain((0..5).map(|index| {
                crate::runtime::combat::CombatCard::new(CardId::Apparition, 100 + index)
            }))
            .collect();

    let context = build_campfire_decision_context_v1(
        &run_state,
        vec![CampfireChoice::Rest, CampfireChoice::Smith(0)],
    );
    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());

    assert!(matches!(
        decision.action,
        CampfirePolicyActionV1::Rest { .. }
    ));
    assert!(decision
        .candidate_plans
        .iter()
        .all(|candidate| { candidate.repair_priority.is_none() || !candidate.execute_autopilot }));
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

fn install_current_room_route(
    run_state: &mut RunState,
    current_room: RoomType,
    future_rooms: &[RoomType],
) {
    let mut graph = Vec::new();
    let mut current = map_node(0, 0, current_room);
    if !future_rooms.is_empty() {
        current.edges.insert(MapEdge::new(0, 0, 0, 1));
    }
    graph.push(vec![current]);
    for (idx, room) in future_rooms.iter().enumerate() {
        let y = idx as i32 + 1;
        let mut node = map_node(0, y, *room);
        if idx + 1 < future_rooms.len() {
            node.edges.insert(MapEdge::new(0, y, 0, y + 1));
        }
        graph.push(vec![node]);
    }
    run_state.map = MapState::new(graph);
    run_state.map.current_x = 0;
    run_state.map.current_y = 0;
}

fn map_node(x: i32, y: i32, room_type: RoomType) -> MapRoomNode {
    let mut node = MapRoomNode::new(x, y);
    node.class = Some(room_type);
    node
}
