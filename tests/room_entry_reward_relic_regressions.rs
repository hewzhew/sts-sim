use sts_simulator::content::relics::{get_relic_tier, RelicId, RelicState, RelicTier};
use sts_simulator::engine::run_loop::tick_run;
use sts_simulator::map::node::{MapEdge, MapRoomNode, RoomType};
use sts_simulator::map::state::MapState;
use sts_simulator::state::core::{ClientInput, EngineState};
use sts_simulator::state::run::RunState;
use sts_simulator::state::RewardItem;

fn single_entry_map(room_type: RoomType) -> MapState {
    let mut room = MapRoomNode::new(0, 0);
    room.class = Some(room_type);
    room.edges.insert(MapEdge::new(0, 0, 0, 1));

    let exit = MapRoomNode::new(0, 1);
    MapState::new(vec![vec![room], vec![exit]])
}

fn enter_first_room(run_state: &mut RunState) -> EngineState {
    let mut engine_state = EngineState::MapNavigation;
    let mut combat_state = None;
    let keep_running = tick_run(
        &mut engine_state,
        run_state,
        &mut combat_state,
        Some(ClientInput::SelectMapNode(0)),
    );
    assert!(keep_running);
    engine_state
}

#[test]
fn meal_ticket_heal_is_blocked_by_mark_of_the_bloom_on_shop_entry() {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    run_state.map = single_entry_map(RoomType::ShopRoom);
    run_state.current_hp = 40;
    run_state.max_hp = 80;
    run_state.relics.clear();
    run_state.relics.push(RelicState::new(RelicId::MealTicket));
    run_state
        .relics
        .push(RelicState::new(RelicId::MarkOfTheBloom));

    let engine_state = enter_first_room(&mut run_state);

    assert!(matches!(engine_state, EngineState::Shop(_)));
    assert_eq!(run_state.current_hp, 40);
}

#[test]
fn matryoshka_starts_active_and_adds_extra_relic_reward() {
    let mut run_state = RunState::new(7, 0, false, "Ironclad");
    run_state.map = single_entry_map(RoomType::TreasureRoom);
    run_state.relics.clear();
    run_state.relics.push(RelicState::new(RelicId::Matryoshka));
    run_state.common_relic_pool = vec![RelicId::Anchor, RelicId::BagOfMarbles];
    run_state.uncommon_relic_pool = vec![RelicId::Kunai];
    run_state.rare_relic_pool = vec![RelicId::BirdFacedUrn];

    let engine_state = enter_first_room(&mut run_state);

    let reward = match engine_state {
        EngineState::RewardScreen(reward) => reward,
        other => panic!("expected RewardScreen, got {other:?}"),
    };
    let relic_rewards = reward
        .items
        .iter()
        .filter(|item| matches!(item, RewardItem::Relic { .. }))
        .count();
    assert_eq!(relic_rewards, 2);

    let mat = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::Matryoshka)
        .expect("Matryoshka should still be present after first chest");
    assert_eq!(mat.counter, 1);
    assert!(!mat.used_up);
}

#[test]
fn matryoshka_extra_relic_uses_only_common_or_uncommon_tiers() {
    for seed in 1..=64 {
        let mut run_state = RunState::new(seed, 0, false, "Ironclad");
        run_state.map = single_entry_map(RoomType::TreasureRoom);
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::Matryoshka));
        run_state.common_relic_pool = vec![RelicId::Anchor, RelicId::BagOfPreparation];
        run_state.uncommon_relic_pool = vec![RelicId::Kunai, RelicId::QuestionCard];
        run_state.rare_relic_pool = vec![RelicId::BirdFacedUrn, RelicId::Calipers];

        let engine_state = enter_first_room(&mut run_state);
        let reward = match engine_state {
            EngineState::RewardScreen(reward) => reward,
            other => panic!("expected RewardScreen, got {other:?}"),
        };
        let relic_ids = reward
            .items
            .iter()
            .filter_map(|item| match item {
                RewardItem::Relic { relic_id } => Some(*relic_id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            relic_ids.len() >= 2,
            "Matryoshka should add a second relic reward for seed {seed}"
        );

        let extra_tier = get_relic_tier(relic_ids[1]);
        assert!(
            matches!(extra_tier, RelicTier::Common | RelicTier::Uncommon),
            "Matryoshka extra relic should be Common/Uncommon only for seed {seed}, got {extra_tier:?}"
        );
    }
}

#[test]
fn nloths_mask_starts_active_and_removes_one_relic_reward() {
    let mut run_state = RunState::new(11, 0, false, "Ironclad");
    run_state.map = single_entry_map(RoomType::TreasureRoom);
    run_state.relics.clear();
    run_state.relics.push(RelicState::new(RelicId::NlothsMask));
    run_state.common_relic_pool = vec![RelicId::Anchor, RelicId::BagOfMarbles];

    let engine_state = enter_first_room(&mut run_state);

    let reward = match engine_state {
        EngineState::RewardScreen(reward) => reward,
        other => panic!("expected RewardScreen, got {other:?}"),
    };
    let relic_rewards = reward
        .items
        .iter()
        .filter(|item| matches!(item, RewardItem::Relic { .. }))
        .count();
    assert_eq!(relic_rewards, 0);

    let mask = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::NlothsMask)
        .expect("Nloth's Mask should still be present");
    assert_eq!(mask.counter, -2);
    assert!(mask.used_up);
}
