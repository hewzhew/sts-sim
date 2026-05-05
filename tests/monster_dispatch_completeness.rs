use sts_simulator::content::monsters::{
    resolve_monster_turn, resolve_monster_turn_plan, resolve_on_death, resolve_pre_battle_actions,
    resolve_roll_move_actions, roll_monster_turn_outcome, EnemyId, PreBattleLegacyRng,
};
use sts_simulator::content::powers::store::powers_snapshot_for;
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::test_support::{blank_test_combat, test_monster};

const ROLL_NUM: i32 = 50;

fn all_enemy_ids() -> Vec<EnemyId> {
    (0..=EnemyId::CorruptHeart as usize)
        .map(|id| EnemyId::from_id(id).expect("EnemyId::from_id should cover the full dense range"))
        .collect()
}

fn seed_dispatch_monster(
    state: &mut sts_simulator::runtime::combat::CombatState,
    enemy_id: EnemyId,
) -> sts_simulator::runtime::combat::MonsterEntity {
    let mut monster = test_monster(enemy_id);
    match enemy_id {
        EnemyId::LouseNormal | EnemyId::LouseDefensive => {
            monster.louse.bite_damage = Some(5);
        }
        EnemyId::Darkling => {
            sts_simulator::content::monsters::beyond::darkling::initialize_runtime_state(
                &mut monster,
                &mut state.rng.monster_hp_rng,
                state.meta.ascension_level,
            );
        }
        EnemyId::TheGuardian => {
            sts_simulator::content::monsters::exordium::the_guardian::initialize_runtime_state(
                &mut monster,
                state.meta.ascension_level,
            );
        }
        _ => {}
    }
    monster
}

fn assert_semantic_dispatches(enemy_id: EnemyId) {
    let mut state = blank_test_combat();
    let monster = seed_dispatch_monster(&mut state, enemy_id);
    state.entities.monsters.push(monster.clone());

    let pre_battle_actions = resolve_pre_battle_actions(
        &mut state,
        enemy_id,
        &monster,
        PreBattleLegacyRng::MonsterHp,
    );
    for action in pre_battle_actions {
        execute_action(action, &mut state);
    }

    let live_monster = state.entities.monsters[0].clone();
    let player_powers = powers_snapshot_for(&state, state.entities.player.id);
    let mut ai_rng = state.rng.ai_rng.clone();
    let outcome = roll_monster_turn_outcome(
        &mut ai_rng,
        &live_monster,
        state.meta.ascension_level,
        ROLL_NUM,
        &state.entities.monsters,
        &player_powers,
    );

    let replayed_setup_actions =
        resolve_roll_move_actions(&state, &live_monster, ROLL_NUM, &outcome.plan);
    assert_eq!(
        format!("{:?}", replayed_setup_actions),
        format!("{:?}", outcome.setup_actions),
        "on_roll_move dispatch drifted for {:?}",
        enemy_id
    );

    for action in outcome.setup_actions {
        execute_action(action, &mut state);
    }

    state.entities.monsters[0].set_planned_move_id(outcome.plan.move_id);
    let planned = state.entities.monsters[0].clone();

    let reconstructed = resolve_monster_turn_plan(&state, &planned);
    assert_eq!(
        reconstructed.move_id, outcome.plan.move_id,
        "turn_plan reconstruction drifted for {:?}",
        enemy_id
    );

    let _ = resolve_monster_turn(&mut state, &planned);
    let _ = resolve_on_death(enemy_id, &mut state, &planned);
}

#[test]
fn enemy_id_table_is_dense_and_names_are_nonempty() {
    let all_ids = all_enemy_ids();
    assert_eq!(all_ids.len(), EnemyId::CorruptHeart as usize + 1);
    for (raw_id, enemy_id) in all_ids.into_iter().enumerate() {
        assert_eq!(
            EnemyId::from_id(raw_id),
            Some(enemy_id),
            "EnemyId mapping drifted at raw id {}",
            raw_id
        );
        assert!(
            !enemy_id.get_name().is_empty(),
            "EnemyId {:?} should have a display name",
            enemy_id
        );
    }
}

#[test]
fn every_enemy_dispatches_semantic_pre_battle_roll_turn_and_on_death() {
    for enemy_id in all_enemy_ids() {
        assert_semantic_dispatches(enemy_id);
    }
}
