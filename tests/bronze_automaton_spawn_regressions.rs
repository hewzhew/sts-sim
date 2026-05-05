use sts_simulator::content::monsters::resolve_monster_turn;
use sts_simulator::content::monsters::roll_monster_turn_plan;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::runtime::action::Action;
use sts_simulator::runtime::combat::CombatState;
use sts_simulator::runtime::rng::StsRng;
use sts_simulator::test_support::{combat_with_monsters, monster_with_history, test_monster};

fn failing_frame_rng_state() -> StsRng {
    StsRng {
        seed0: 14816428027183242661,
        seed1: 13090347038078055980,
        counter: 1,
    }
}

fn drain_action_queue(state: &mut CombatState) {
    while let Some(action) = state.pop_next_action() {
        execute_action(action, state);
    }
}

fn bronze_automaton_spawn_state() -> CombatState {
    let mut automaton = monster_with_history(EnemyId::BronzeAutomaton, 4, &[4]);
    automaton.id = 10;
    automaton.logical_position = 927;

    let mut state = combat_with_monsters(vec![automaton]);
    state.monster_protocol_identity_mut(10).draw_x = Some(927);
    state.rng.ai_rng = failing_frame_rng_state();
    state.rng.monster_hp_rng = failing_frame_rng_state();
    state
}

#[test]
fn bronze_automaton_spawn_executes_spawned_orb_rolls_immediately() {
    let mut state = bronze_automaton_spawn_state();
    let automaton = state.entities.monsters[0].clone();

    for action in resolve_monster_turn(&mut state, &automaton) {
        state.queue_action_back(action);
    }

    let first = state
        .pop_next_action()
        .expect("left spawn should be queued");
    assert!(matches!(first, Action::SpawnMonsterSmart { .. }));
    execute_action(first, &mut state);

    let second = state
        .pop_next_action()
        .expect("smart spawn should immediately lower to concrete spawn");
    assert!(matches!(second, Action::SpawnMonster { .. }));
    execute_action(second, &mut state);

    assert!(
        state
            .engine
            .action_queue
            .iter()
            .all(|action| !matches!(action, Action::RollMonsterMove { monster_id: 11 })),
        "spawned Bronze Orb should roll immediately during spawn, not remain queued behind Automaton"
    );
}

#[test]
fn bronze_automaton_spawn_uses_java_roll_order_for_spawned_orbs() {
    let mut state = bronze_automaton_spawn_state();
    let automaton = state.entities.monsters[0].clone();
    let mut expected_rng = state.rng.ai_rng.clone();

    let left_roll = expected_rng.random(99);
    let expected_left = roll_monster_turn_plan(
        &mut expected_rng,
        &test_monster(EnemyId::BronzeOrb),
        state.meta.ascension_level,
        left_roll,
        &[],
        &[],
    )
    .move_id;

    let right_roll = expected_rng.random(99);
    let expected_right = roll_monster_turn_plan(
        &mut expected_rng,
        &test_monster(EnemyId::BronzeOrb),
        state.meta.ascension_level,
        right_roll,
        &[],
        &[],
    )
    .move_id;

    let automaton_roll = expected_rng.random(99);
    let expected_automaton = roll_monster_turn_plan(
        &mut expected_rng,
        &automaton,
        state.meta.ascension_level,
        automaton_roll,
        &[],
        &[],
    )
    .move_id;

    for action in resolve_monster_turn(&mut state, &automaton) {
        state.queue_action_back(action);
    }
    drain_action_queue(&mut state);

    let left_orb = state
        .entities
        .monsters
        .iter()
        .find(|monster| {
            monster.monster_type == EnemyId::BronzeOrb as usize && monster.logical_position == 760
        })
        .expect("left Bronze Orb should be spawned");
    let right_orb = state
        .entities
        .monsters
        .iter()
        .find(|monster| {
            monster.monster_type == EnemyId::BronzeOrb as usize && monster.logical_position == 1093
        })
        .expect("right Bronze Orb should be spawned");
    let automaton = state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == 10)
        .expect("Bronze Automaton should still exist");

    assert_eq!(
        left_orb.planned_move_id(),
        expected_left,
        "left Bronze Orb should consume the first post-spawn roll"
    );
    assert_eq!(
        right_orb.planned_move_id(),
        expected_right,
        "right Bronze Orb should consume the second post-spawn roll"
    );
    assert_eq!(
        automaton.planned_move_id(),
        expected_automaton,
        "Bronze Automaton should roll after both spawned orbs"
    );
}

#[test]
fn bronze_automaton_spawn_matches_java_bronze_orb_hp_roll_consumption() {
    let mut state = bronze_automaton_spawn_state();
    let automaton = state.entities.monsters[0].clone();

    for action in resolve_monster_turn(&mut state, &automaton) {
        state.queue_action_back(action);
    }
    drain_action_queue(&mut state);

    let left_orb = state
        .entities
        .monsters
        .iter()
        .find(|monster| {
            monster.monster_type == EnemyId::BronzeOrb as usize && monster.logical_position == 760
        })
        .expect("left Bronze Orb should be spawned");
    let right_orb = state
        .entities
        .monsters
        .iter()
        .find(|monster| {
            monster.monster_type == EnemyId::BronzeOrb as usize && monster.logical_position == 1093
        })
        .expect("right Bronze Orb should be spawned");

    assert_eq!(
        left_orb.current_hp, 58,
        "BronzeOrb should use the second constructor/setHp roll, matching Java's left-orb HP"
    );
    assert_eq!(left_orb.max_hp, 58);
    assert_eq!(
        right_orb.current_hp, 56,
        "BronzeOrb should consume two hp rolls per spawn, matching Java's right-orb HP"
    );
    assert_eq!(right_orb.max_hp, 56);
}
