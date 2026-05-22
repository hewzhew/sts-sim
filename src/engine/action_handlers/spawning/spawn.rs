use super::{initial_runtime, move_roll::handle_roll_monster_move};
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;
use crate::runtime::monster_move::{SpawnHpSpec, SpawnHpValue};
fn normalize_monster_slots(state: &mut CombatState) {
    let monster_ids: Vec<_> = state
        .entities
        .monsters
        .iter()
        .map(|monster| monster.id)
        .collect();
    for (idx, monster) in state.entities.monsters.iter_mut().enumerate() {
        monster.slot = idx as u8;
    }
    for (idx, monster_id) in monster_ids.into_iter().enumerate() {
        state.monster_protocol_identity_mut(monster_id).group_index = Some(idx);
    }
}

fn next_protocol_sequence(state: &CombatState) -> u64 {
    state
        .entities
        .monsters
        .iter()
        .filter_map(|monster| {
            state
                .monster_protocol_identity(monster.id)
                .and_then(|identity| identity.spawn_order)
        })
        .max()
        .unwrap_or(0)
        + 1
}

fn spawn_hp_for_monster(
    monster_id: crate::content::monsters::EnemyId,
    hp_rng: &mut crate::runtime::rng::StsRng,
    ascension_level: u8,
) -> i32 {
    match monster_id {
        crate::content::monsters::EnemyId::TorchHead => {
            // Java TorchHead constructor consumes monsterHpRng twice:
            // once in super(... random(38,40) ...), then again in setHp(...).
            let _unused_ctor_roll = hp_rng.random_range(38, 40);
            if ascension_level >= 9 {
                hp_rng.random_range(40, 45)
            } else {
                hp_rng.random_range(38, 40)
            }
        }
        crate::content::monsters::EnemyId::BronzeOrb => {
            // Java BronzeOrb constructor consumes monsterHpRng twice:
            // once in super(... random(52,58) ...), then again in setHp(...).
            let _unused_ctor_roll = hp_rng.random_range(52, 58);
            if ascension_level >= 9 {
                hp_rng.random_range(54, 60)
            } else {
                hp_rng.random_range(52, 58)
            }
        }
        _ => {
            let (hp_min, hp_max) =
                crate::content::monsters::get_hp_range(monster_id, ascension_level);
            hp_rng.random_range(hp_min, hp_max)
        }
    }
}

pub fn handle_spawn_monster(
    monster_id: crate::content::monsters::EnemyId,
    slot: u8,
    current_hp: i32,
    max_hp: i32,
    logical_position: i32,
    protocol_draw_x: Option<i32>,
    is_minion: bool,
    state: &mut CombatState,
) -> usize {
    let new_entity_id = state
        .entities
        .monsters
        .iter()
        .map(|m| m.id)
        .max()
        .unwrap_or(0)
        + 1;
    let enemy_id = monster_id;
    let next_protocol_id = next_protocol_sequence(state);

    let louse_bite_damage = match enemy_id {
        crate::content::monsters::EnemyId::LouseNormal
        | crate::content::monsters::EnemyId::LouseDefensive => {
            Some(if state.meta.ascension_level >= 2 {
                state.rng.monster_hp_rng.random_range(6, 8)
            } else {
                state.rng.monster_hp_rng.random_range(5, 7)
            })
        }
        _ => None,
    };

    let mut new_monster = crate::runtime::combat::MonsterEntity {
        id: new_entity_id,
        monster_type: enemy_id as usize,
        current_hp,
        max_hp,
        block: 0,
        slot,
        is_dying: false,
        is_escaped: false,
        half_dead: false,
        move_state: crate::runtime::combat::MonsterMoveState::default(),
        logical_position,
        hexaghost: Default::default(),
        louse: crate::runtime::combat::LouseRuntimeState {
            bite_damage: louse_bite_damage,
        },
        jaw_worm: Default::default(),
        thief: Default::default(),
        byrd: Default::default(),
        chosen: Default::default(),
        snecko: Default::default(),
        shelled_parasite: Default::default(),
        bronze_automaton: Default::default(),
        bronze_orb: Default::default(),
        book_of_stabbing: Default::default(),
        collector: Default::default(),
        champ: Default::default(),
        awakened_one: Default::default(),
        corrupt_heart: Default::default(),
        writhing_mass: Default::default(),
        spiker: Default::default(),
        spire_shield: Default::default(),
        spire_spear: Default::default(),
        slaver_red: Default::default(),
        gremlin_leader: Default::default(),
        gremlin_nob: Default::default(),
        gremlin_wizard: Default::default(),
        cultist: Default::default(),
        sentry: Default::default(),
        slime_boss: Default::default(),
        large_slime: Default::default(),
        spheric_guardian: Default::default(),
        reptomancer: Default::default(),
        darkling: Default::default(),
        nemesis: Default::default(),
        giant_head: Default::default(),
        time_eater: Default::default(),
        donu: Default::default(),
        deca: Default::default(),
        transient: Default::default(),
        exploder: Default::default(),
        maw: Default::default(),
        snake_dagger: Default::default(),
        lagavulin: Default::default(),
        guardian: Default::default(),
    };

    state.runtime.monster_protocol.insert(
        new_entity_id,
        crate::runtime::combat::MonsterProtocolState {
            observation: crate::runtime::combat::MonsterProtocolObservationState {
                visible_intent: crate::runtime::combat::Intent::Unknown,
                preview_damage_per_hit: louse_bite_damage.unwrap_or(0),
            },
            identity: crate::runtime::combat::MonsterProtocolIdentity {
                instance_id: Some(next_protocol_id),
                spawn_order: Some(next_protocol_id),
                draw_x: protocol_draw_x,
                group_index: Some(slot as usize),
            },
        },
    );

    initial_runtime::initialize_spawned_monster_runtime(enemy_id, &mut new_monster, state);

    crate::content::relics::hooks::on_spawn_monster(state, new_entity_id);

    state.entities.monsters.insert(slot as usize, new_monster);
    normalize_monster_slots(state);

    let entity_snapshot = state.entities.monsters[slot as usize].clone();
    let pre_battle_actions = crate::content::monsters::resolve_pre_battle_actions(
        state,
        enemy_id,
        &entity_snapshot,
        crate::content::monsters::PreBattleLegacyRng::MonsterHp,
    );
    for a in pre_battle_actions {
        state.queue_action_back(a);
    }

    if is_minion {
        state.queue_action_front(Action::ApplyPower {
            source: new_entity_id,
            target: new_entity_id,
            power_id: crate::content::powers::PowerId::Minion,
            amount: -1,
        });
    }

    // Java SpawnMonsterAction calls m.init() during the spawn update, so a freshly
    // spawned monster rolls its first move immediately instead of waiting behind
    // the rest of the current action queue.
    handle_roll_monster_move(new_entity_id, state);

    new_entity_id
}

fn resolve_spawn_hp(
    monster_id: crate::content::monsters::EnemyId,
    hp: SpawnHpSpec,
    state: &mut CombatState,
) -> (i32, i32) {
    match (hp.current, hp.max) {
        (SpawnHpValue::Rolled, SpawnHpValue::Rolled) => {
            let rolled = spawn_hp_for_monster(
                monster_id,
                &mut state.rng.monster_hp_rng,
                state.meta.ascension_level,
            );
            (rolled, rolled)
        }
        (current, max) => {
            let resolve_hp =
                |value: SpawnHpValue, hp_rng: &mut crate::runtime::rng::StsRng| match value {
                    SpawnHpValue::Rolled => {
                        spawn_hp_for_monster(monster_id, hp_rng, state.meta.ascension_level)
                    }
                    SpawnHpValue::Fixed(hp) => hp.max(0),
                    SpawnHpValue::SourceCurrentHp | SpawnHpValue::SourceMaxHp => {
                        panic!("spawn action leaked source-relative hp rule")
                    }
                };
            (
                resolve_hp(current, &mut state.rng.monster_hp_rng),
                resolve_hp(max, &mut state.rng.monster_hp_rng),
            )
        }
    }
}

fn smart_spawn_slot(
    state: &CombatState,
    logical_position: i32,
    protocol_draw_x: Option<i32>,
) -> u8 {
    let spawn_sort_key = protocol_draw_x.unwrap_or(logical_position);
    let mut target_slot = 0;
    for m in &state.entities.monsters {
        let existing_sort_key = state
            .monster_protocol_identity(m.id)
            .and_then(|identity| identity.draw_x)
            .unwrap_or(m.logical_position);
        if spawn_sort_key > existing_sort_key {
            target_slot += 1;
        }
    }
    target_slot
}

pub fn handle_spawn_monster_smart(
    monster_id: crate::content::monsters::EnemyId,
    logical_position: i32,
    hp: SpawnHpSpec,
    protocol_draw_x: Option<i32>,
    is_minion: bool,
    state: &mut CombatState,
) {
    let (current_hp, max_hp) = resolve_spawn_hp(monster_id, hp, state);
    let target_slot = smart_spawn_slot(state, logical_position, protocol_draw_x);
    state.queue_action_front(Action::SpawnMonster {
        monster_id,
        slot: target_slot,
        current_hp,
        max_hp,
        logical_position,
        protocol_draw_x,
        is_minion,
    });
}

pub fn handle_spawn_collector_torch(
    collector_id: usize,
    collector_slot: u8,
    logical_position: i32,
    hp: SpawnHpSpec,
    protocol_draw_x: Option<i32>,
    state: &mut CombatState,
) {
    assert!(
        matches!(collector_slot, 1 | 2),
        "collector torch slot must be the Java enemySlots key 1 or 2"
    );
    let monster_id = crate::content::monsters::EnemyId::TorchHead;
    let (current_hp, max_hp) = resolve_spawn_hp(monster_id, hp, state);
    let target_slot = smart_spawn_slot(state, logical_position, protocol_draw_x);
    let new_entity_id = handle_spawn_monster(
        monster_id,
        target_slot,
        current_hp,
        max_hp,
        logical_position,
        protocol_draw_x,
        true,
        state,
    );

    if let Some(collector) = state
        .entities
        .monsters
        .iter_mut()
        .find(|monster| monster.id == collector_id)
    {
        collector.collector.enemy_slots[usize::from(collector_slot - 1)] = Some(new_entity_id);
        collector.collector.protocol_seeded = true;
    }
}

pub fn handle_spawn_gremlin_leader_minion(
    leader_id: usize,
    gremlin_slot: u8,
    monster_id: crate::content::monsters::EnemyId,
    logical_position: i32,
    hp: SpawnHpSpec,
    protocol_draw_x: Option<i32>,
    state: &mut CombatState,
) {
    assert!(
        gremlin_slot < 3,
        "gremlin leader slot must be one of Java gremlins[0..3)"
    );
    let (current_hp, max_hp) = resolve_spawn_hp(monster_id, hp, state);
    let target_slot = smart_spawn_slot(state, logical_position, protocol_draw_x);
    let new_entity_id = handle_spawn_monster(
        monster_id,
        target_slot,
        current_hp,
        max_hp,
        logical_position,
        protocol_draw_x,
        true,
        state,
    );

    if let Some(leader) = state
        .entities
        .monsters
        .iter_mut()
        .find(|monster| monster.id == leader_id)
    {
        leader.gremlin_leader.gremlin_slots[usize::from(gremlin_slot)] = Some(new_entity_id);
        leader.gremlin_leader.protocol_seeded = true;
    }
}

pub fn handle_spawn_reptomancer_dagger(
    reptomancer_id: usize,
    dagger_slot: u8,
    logical_position: i32,
    hp: SpawnHpSpec,
    protocol_draw_x: Option<i32>,
    state: &mut CombatState,
) {
    assert!(
        dagger_slot < 4,
        "reptomancer dagger slot must be one of Java daggers[0..4)"
    );
    let monster_id = crate::content::monsters::EnemyId::SnakeDagger;
    let (current_hp, max_hp) = resolve_spawn_hp(monster_id, hp, state);
    let target_slot = smart_spawn_slot(state, logical_position, protocol_draw_x);
    let new_entity_id = handle_spawn_monster(
        monster_id,
        target_slot,
        current_hp,
        max_hp,
        logical_position,
        protocol_draw_x,
        true,
        state,
    );

    if let Some(reptomancer) = state
        .entities
        .monsters
        .iter_mut()
        .find(|monster| monster.id == reptomancer_id)
    {
        reptomancer.reptomancer.dagger_slots[usize::from(dagger_slot)] = Some(new_entity_id);
        reptomancer.reptomancer.protocol_seeded = true;
    }
}
