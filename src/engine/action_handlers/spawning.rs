// action_handlers/spawning.rs — Monster lifecycle domain
//
// Handles: SpawnMonster, SpawnMonsterSmart, SpawnEncounter,
//          Escape, Suicide, AbortDeath, FleeCombat,
//          RollMonsterMove, SetMonsterMove, ExecuteMonsterTurn,
//          UpdateRelicCounter, UpdateRelicAmount, UpdateRelicUsedUp

use crate::action::Action;
use crate::combat::CombatState;

fn normalize_monster_slots(state: &mut CombatState) {
    for (idx, monster) in state.entities.monsters.iter_mut().enumerate() {
        monster.slot = idx as u8;
        monster.protocol_identity.group_index = Some(idx);
    }
}

fn next_protocol_sequence(state: &CombatState) -> u64 {
    state
        .entities
        .monsters
        .iter()
        .filter_map(|monster| monster.protocol_identity.spawn_order)
        .max()
        .unwrap_or(0)
        + 1
}

fn spawn_hp_for_monster(
    monster_id: crate::content::monsters::EnemyId,
    hp_rng: &mut crate::rng::StsRng,
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
) {
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

    let (actual_hp, actual_max_hp) = if current_hp == 0 {
        let rolled = spawn_hp_for_monster(
            enemy_id,
            &mut state.rng.monster_hp_rng,
            state.meta.ascension_level,
        );
        (rolled, rolled)
    } else {
        (current_hp, max_hp)
    };

    let mut new_monster = crate::combat::MonsterEntity {
        id: new_entity_id,
        monster_type: enemy_id as usize,
        current_hp: actual_hp,
        max_hp: actual_max_hp,
        block: 0,
        slot,
        is_dying: false,
        is_escaped: false,
        half_dead: false,
        next_move_byte: 0,
        current_intent: crate::combat::Intent::Unknown,
        move_history: std::collections::VecDeque::new(),
        intent_dmg: 0,
        logical_position,
        protocol_identity: crate::combat::MonsterProtocolIdentity {
            instance_id: Some(next_protocol_id),
            spawn_order: Some(next_protocol_id),
            draw_x: protocol_draw_x,
            group_index: Some(slot as usize),
        },
        hexaghost: Default::default(),
        chosen: Default::default(),
        darkling: Default::default(),
        lagavulin: Default::default(),
    };

    if enemy_id == crate::content::monsters::EnemyId::Darkling {
        crate::content::monsters::beyond::darkling::initialize_runtime_state(
            &mut new_monster,
            &mut state.rng.monster_hp_rng,
            state.meta.ascension_level,
        );
    }

    state.entities.monsters.insert(slot as usize, new_monster);
    normalize_monster_slots(state);

    for action in crate::content::relics::hooks::on_spawn_monster(state, slot as usize) {
        state.engine.action_queue.push_back(action);
    }

    let pre_battle_actions = crate::content::monsters::resolve_pre_battle_action(
        enemy_id,
        &state.entities.monsters[slot as usize],
        &mut state.rng.monster_hp_rng,
        state.meta.ascension_level,
    );
    for a in pre_battle_actions {
        state.engine.action_queue.push_back(a);
    }

    if is_minion {
        state.engine.action_queue.push_front(Action::ApplyPower {
            source: new_entity_id,
            target: new_entity_id,
            power_id: crate::content::powers::PowerId::Minion,
            amount: 1,
        });
    }

    state
        .engine
        .action_queue
        .push_back(Action::RollMonsterMove {
            monster_id: new_entity_id,
        });
}

pub fn handle_spawn_monster_smart(
    monster_id: crate::content::monsters::EnemyId,
    logical_position: i32,
    current_hp: i32,
    max_hp: i32,
    protocol_draw_x: Option<i32>,
    is_minion: bool,
    state: &mut CombatState,
) {
    let spawn_sort_key = protocol_draw_x.unwrap_or(logical_position);
    let mut target_slot = 0;
    for m in &state.entities.monsters {
        let existing_sort_key = m.protocol_identity.draw_x.unwrap_or(m.logical_position);
        if spawn_sort_key > existing_sort_key {
            target_slot += 1;
        }
    }
    state.engine.action_queue.push_front(Action::SpawnMonster {
        monster_id,
        slot: target_slot,
        current_hp,
        max_hp,
        logical_position,
        protocol_draw_x,
        is_minion,
    });
}

pub fn handle_suicide(target: usize, state: &mut CombatState) {
    if let Some(m) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        m.current_hp = 0;
        m.is_dying = true;
    }
}

pub fn handle_escape(target: usize, state: &mut CombatState) {
    if let Some(m) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        m.is_escaped = true;
    }
}

pub fn handle_roll_monster_move(monster_id: usize, state: &mut CombatState) {
    if let Some(m) = state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == monster_id && !m.is_dying)
    {
        let entity_snapshot = m.clone();
        let num = state.rng.ai_rng.random(99);
        let (move_byte, intent) = crate::content::monsters::roll_monster_move(
            &mut state.rng.ai_rng,
            &entity_snapshot,
            state.meta.ascension_level,
            num,
            &state.entities.monsters,
        );
        if let Some(m) = state
            .entities
            .monsters
            .iter_mut()
            .find(|m| m.id == monster_id)
        {
            m.next_move_byte = move_byte;
            m.current_intent = intent;
            m.move_history.push_back(move_byte);
            if crate::content::monsters::EnemyId::from_id(m.monster_type)
                == Some(crate::content::monsters::EnemyId::Darkling)
            {
                m.darkling.first_move = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torch_head_spawn_consumes_two_hp_rolls_and_uses_second_value() {
        let mut combat = crate::engine::test_support::basic_combat();
        combat.meta.ascension_level = 0;

        let mut expected_rng = combat.rng.monster_hp_rng.clone();
        let _first = expected_rng.random_range(38, 40);
        let expected_hp = expected_rng.random_range(38, 40);

        handle_spawn_monster(
            crate::content::monsters::EnemyId::TorchHead,
            0,
            0,
            0,
            647,
            Some(647),
            true,
            &mut combat,
        );

        let spawned = &combat.entities.monsters[0];
        assert_eq!(
            crate::content::monsters::EnemyId::from_id(spawned.monster_type),
            Some(crate::content::monsters::EnemyId::TorchHead)
        );
        assert_eq!(spawned.current_hp, expected_hp);
        assert_eq!(combat.rng.monster_hp_rng.counter, expected_rng.counter);
    }
}

pub fn handle_set_monster_move(
    monster_id: usize,
    next_move_byte: u8,
    intent: crate::combat::Intent,
    state: &mut CombatState,
) {
    if let Some(m) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        m.next_move_byte = next_move_byte;
        m.current_intent = intent;
        m.move_history.push_back(next_move_byte);
        if crate::content::monsters::EnemyId::from_id(m.monster_type)
            == Some(crate::content::monsters::EnemyId::Darkling)
            && next_move_byte != 0
        {
            m.darkling.first_move = false;
        }
    }
}

pub fn handle_update_hexaghost_state(
    monster_id: usize,
    activated: Option<bool>,
    orb_active_count: Option<u8>,
    burn_upgraded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if activated.is_some() || orb_active_count.is_some() || burn_upgraded.is_some() {
            if let Some(value) = activated {
                monster.hexaghost.activated = value;
            }
            if let Some(value) = orb_active_count {
                monster.hexaghost.orb_active_count = value;
            }
            if let Some(value) = burn_upgraded {
                monster.hexaghost.burn_upgraded = value;
            }
        }
    }
}

pub fn handle_update_relic_counter(
    relic_id: crate::content::relics::RelicId,
    counter: i32,
    state: &mut CombatState,
) {
    if let Some(relic) = state
        .entities
        .player
        .relics
        .iter_mut()
        .find(|r| r.id == relic_id)
    {
        relic.counter = counter;
    }
}

pub fn handle_update_relic_amount(
    relic_id: crate::content::relics::RelicId,
    amount: i32,
    state: &mut CombatState,
) {
    if let Some(relic) = state
        .entities
        .player
        .relics
        .iter_mut()
        .find(|r| r.id == relic_id)
    {
        relic.counter += amount;
    }
}

pub fn handle_update_relic_used_up(
    relic_id: crate::content::relics::RelicId,
    used_up: bool,
    state: &mut CombatState,
) {
    if let Some(relic) = state
        .entities
        .player
        .relics
        .iter_mut()
        .find(|r| r.id == relic_id)
    {
        relic.used_up = used_up;
    }
}
