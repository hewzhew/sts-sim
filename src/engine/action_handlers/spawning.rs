// action_handlers/spawning.rs — Monster lifecycle domain
//
// Handles: SpawnMonster, SpawnMonsterSmart, SpawnEncounter,
//          Escape, Suicide, AbortDeath, FleeCombat,
//          RollMonsterMove, SetMonsterMove, ExecuteMonsterTurn,
//          UpdateRelicCounter, UpdateRelicAmount, UpdateRelicUsedUp

use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::CombatState;
use crate::semantics::combat::{SpawnHpSpec, SpawnHpValue};

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
        darkling: Default::default(),
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

    if enemy_id == crate::content::monsters::EnemyId::Darkling {
        crate::content::monsters::beyond::darkling::initialize_runtime_state(
            &mut new_monster,
            &mut state.rng.monster_hp_rng,
            state.meta.ascension_level,
        );
    }
    if enemy_id == crate::content::monsters::EnemyId::Byrd {
        new_monster.byrd.first_move = true;
        new_monster.byrd.is_flying = true;
        new_monster.byrd.protocol_seeded = true;
    }
    if enemy_id == crate::content::monsters::EnemyId::Chosen {
        new_monster.chosen.first_turn = true;
        new_monster.chosen.used_hex = false;
        new_monster.chosen.protocol_seeded = true;
    }
    if enemy_id == crate::content::monsters::EnemyId::Snecko {
        new_monster.snecko.first_turn = true;
        new_monster.snecko.protocol_seeded = true;
    }
    if enemy_id == crate::content::monsters::EnemyId::ShelledParasite {
        new_monster.shelled_parasite.first_move = true;
        new_monster.shelled_parasite.protocol_seeded = true;
    }
    if enemy_id == crate::content::monsters::EnemyId::BronzeAutomaton {
        new_monster.bronze_automaton.protocol_seeded = true;
        new_monster.bronze_automaton.first_turn = true;
        new_monster.bronze_automaton.num_turns = 0;
    }
    if enemy_id == crate::content::monsters::EnemyId::BronzeOrb {
        new_monster.bronze_orb.protocol_seeded = true;
        new_monster.bronze_orb.used_stasis = false;
    }
    if enemy_id == crate::content::monsters::EnemyId::BookOfStabbing {
        new_monster.book_of_stabbing.protocol_seeded = true;
        new_monster.book_of_stabbing.stab_count = 1;
    }
    if enemy_id == crate::content::monsters::EnemyId::TheCollector {
        new_monster.collector.protocol_seeded = true;
        new_monster.collector.initial_spawn = true;
        new_monster.collector.ult_used = false;
        new_monster.collector.turns_taken = 0;
    }
    if enemy_id == crate::content::monsters::EnemyId::Champ {
        new_monster.champ.protocol_seeded = true;
        new_monster.champ.first_turn = true;
        new_monster.champ.num_turns = 0;
        new_monster.champ.forge_times = 0;
        new_monster.champ.threshold_reached = false;
    }
    if enemy_id == crate::content::monsters::EnemyId::AwakenedOne {
        new_monster.awakened_one.protocol_seeded = true;
        new_monster.awakened_one.form1 = true;
        new_monster.awakened_one.first_turn = true;
    }
    if enemy_id == crate::content::monsters::EnemyId::CorruptHeart {
        new_monster.corrupt_heart.protocol_seeded = true;
        new_monster.corrupt_heart.first_move = true;
        new_monster.corrupt_heart.move_count = 0;
        new_monster.corrupt_heart.buff_count = 0;
    }
    if matches!(
        enemy_id,
        crate::content::monsters::EnemyId::Looter | crate::content::monsters::EnemyId::Mugger
    ) {
        new_monster.thief.protocol_seeded = true;
        new_monster.thief.slash_count = 0;
        new_monster.thief.stolen_gold = 0;
    }
    if enemy_id == crate::content::monsters::EnemyId::TheGuardian {
        crate::content::monsters::exordium::the_guardian::initialize_runtime_state(
            &mut new_monster,
            state.meta.ascension_level,
        );
    }

    state.entities.monsters.insert(slot as usize, new_monster);
    normalize_monster_slots(state);

    for action in crate::content::relics::hooks::on_spawn_monster(state, slot as usize) {
        state.queue_action_back(action);
    }

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
            amount: 1,
        });
    }

    // Java SpawnMonsterAction calls m.init() during the spawn update, so a freshly
    // spawned monster rolls its first move immediately instead of waiting behind
    // the rest of the current action queue.
    handle_roll_monster_move(new_entity_id, state);
}

pub fn handle_spawn_monster_smart(
    monster_id: crate::content::monsters::EnemyId,
    logical_position: i32,
    hp: SpawnHpSpec,
    protocol_draw_x: Option<i32>,
    is_minion: bool,
    state: &mut CombatState,
) {
    let (current_hp, max_hp) = match (hp.current, hp.max) {
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
    };
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

pub fn handle_suicide(target: usize, state: &mut CombatState) {
    if let Some(m) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        m.current_hp = 0;
        m.is_dying = true;
    }
}

pub fn handle_escape(target: usize, state: &mut CombatState) {
    if let Some(m) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        m.is_escaped = true;
        if m.thief.stolen_gold > 0 {
            state.runtime.combat_mugged = true;
        }
    }
}

pub fn handle_add_combat_reward(item: crate::rewards::state::RewardItem, state: &mut CombatState) {
    state.runtime.pending_rewards.push(item);
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
        let player_powers = crate::content::powers::store::powers_snapshot_for(state, 0);
        let outcome = crate::content::monsters::roll_monster_turn_outcome(
            &mut state.rng.ai_rng,
            &entity_snapshot,
            state.meta.ascension_level,
            num,
            &state.entities.monsters,
            &player_powers,
        );
        for action in outcome.setup_actions {
            crate::engine::action_handlers::execute_action(action, state);
        }
        let plan = outcome.plan;
        let mut updated_monster_id = None;
        if let Some(m) = state
            .entities
            .monsters
            .iter_mut()
            .find(|m| m.id == monster_id)
        {
            m.set_planned_move_id(plan.move_id);
            m.set_planned_steps(plan.steps);
            m.set_planned_visible_spec(plan.visible_spec);
            m.move_history_mut().push_back(plan.move_id);
            updated_monster_id = Some(m.id);
            if crate::content::monsters::EnemyId::from_id(m.monster_type)
                == Some(crate::content::monsters::EnemyId::Darkling)
            {
                m.darkling.first_move = false;
            }
        }
        if let Some(updated_monster_id) = updated_monster_id {
            state.clear_monster_protocol_observation(updated_monster_id);
        }
    }
}

pub fn handle_set_monster_move(
    monster_id: usize,
    next_move_byte: u8,
    planned_steps: crate::semantics::combat::MonsterTurnSteps,
    planned_visible_spec: Option<crate::semantics::combat::MonsterMoveSpec>,
    state: &mut CombatState,
) {
    let mut updated_monster_id = None;
    if let Some(m) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        m.set_planned_move_id(next_move_byte);
        m.set_planned_steps(planned_steps);
        m.set_planned_visible_spec(planned_visible_spec);
        m.move_history_mut().push_back(next_move_byte);
        updated_monster_id = Some(m.id);
        if crate::content::monsters::EnemyId::from_id(m.monster_type)
            == Some(crate::content::monsters::EnemyId::Darkling)
            && next_move_byte != 0
        {
            m.darkling.first_move = false;
        }
    }
    if let Some(updated_monster_id) = updated_monster_id {
        state.clear_monster_protocol_observation(updated_monster_id);
    }
}

fn handle_update_hexaghost_state(
    monster_id: usize,
    activated: Option<bool>,
    orb_active_count: Option<u8>,
    burn_upgraded: Option<bool>,
    divider_damage: Option<i32>,
    clear_divider_damage: bool,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if activated.is_some()
            || orb_active_count.is_some()
            || burn_upgraded.is_some()
            || divider_damage.is_some()
            || clear_divider_damage
        {
            if let Some(value) = activated {
                monster.hexaghost.activated = value;
            }
            if let Some(value) = orb_active_count {
                monster.hexaghost.orb_active_count = value;
            }
            if let Some(value) = burn_upgraded {
                monster.hexaghost.burn_upgraded = value;
            }
            if clear_divider_damage {
                monster.hexaghost.divider_damage = None;
            }
            if let Some(value) = divider_damage {
                monster.hexaghost.divider_damage = Some(value);
            }
        }
    }
}

fn handle_update_lagavulin_state(
    monster_id: usize,
    idle_count: Option<u8>,
    debuff_turn_count: Option<u8>,
    is_out: Option<bool>,
    is_out_triggered: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = idle_count {
            monster.lagavulin.idle_count = value;
        }
        if let Some(value) = debuff_turn_count {
            monster.lagavulin.debuff_turn_count = value;
        }
        if let Some(value) = is_out {
            monster.lagavulin.is_out = value;
        }
        if let Some(value) = is_out_triggered {
            monster.lagavulin.is_out_triggered = value;
        }
    }
}

fn handle_update_guardian_state(
    monster_id: usize,
    damage_threshold: Option<i32>,
    damage_taken: Option<i32>,
    is_open: Option<bool>,
    close_up_triggered: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = damage_threshold {
            monster.guardian.damage_threshold = value;
        }
        if let Some(value) = damage_taken {
            monster.guardian.damage_taken = value;
        }
        if let Some(value) = is_open {
            monster.guardian.is_open = value;
        }
        if let Some(value) = close_up_triggered {
            monster.guardian.close_up_triggered = value;
        }
    }
}

fn handle_update_byrd_state(
    monster_id: usize,
    first_move: Option<bool>,
    is_flying: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.byrd.first_move = value;
        }
        if let Some(value) = is_flying {
            monster.byrd.is_flying = value;
        }
        if let Some(value) = protocol_seeded {
            monster.byrd.protocol_seeded = value;
        }
    }
}

fn handle_update_chosen_state(
    monster_id: usize,
    first_turn: Option<bool>,
    used_hex: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.chosen.first_turn = value;
        }
        if let Some(value) = used_hex {
            monster.chosen.used_hex = value;
        }
        if let Some(value) = protocol_seeded {
            monster.chosen.protocol_seeded = value;
        }
    }
}

fn handle_update_snecko_state(
    monster_id: usize,
    first_turn: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.snecko.first_turn = value;
        }
        if let Some(value) = protocol_seeded {
            monster.snecko.protocol_seeded = value;
        }
    }
}

fn handle_update_shelled_parasite_state(
    monster_id: usize,
    first_move: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.shelled_parasite.first_move = value;
        }
        if let Some(value) = protocol_seeded {
            monster.shelled_parasite.protocol_seeded = value;
        }
    }
}

fn handle_update_bronze_automaton_state(
    monster_id: usize,
    first_turn: Option<bool>,
    num_turns: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.bronze_automaton.first_turn = value;
        }
        if let Some(value) = num_turns {
            monster.bronze_automaton.num_turns = value;
        }
        if let Some(value) = protocol_seeded {
            monster.bronze_automaton.protocol_seeded = value;
        }
    }
}

fn handle_update_bronze_orb_state(
    monster_id: usize,
    used_stasis: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = used_stasis {
            monster.bronze_orb.used_stasis = value;
        }
        if let Some(value) = protocol_seeded {
            monster.bronze_orb.protocol_seeded = value;
        }
    }
}

fn handle_update_collector_state(
    monster_id: usize,
    initial_spawn: Option<bool>,
    ult_used: Option<bool>,
    turns_taken: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = initial_spawn {
            monster.collector.initial_spawn = value;
        }
        if let Some(value) = ult_used {
            monster.collector.ult_used = value;
        }
        if let Some(value) = turns_taken {
            monster.collector.turns_taken = value;
        }
        if let Some(value) = protocol_seeded {
            monster.collector.protocol_seeded = value;
        }
    }
}

fn handle_update_champ_state(
    monster_id: usize,
    first_turn: Option<bool>,
    num_turns: Option<u8>,
    forge_times: Option<u8>,
    threshold_reached: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_turn {
            monster.champ.first_turn = value;
        }
        if let Some(value) = num_turns {
            monster.champ.num_turns = value;
        }
        if let Some(value) = forge_times {
            monster.champ.forge_times = value;
        }
        if let Some(value) = threshold_reached {
            monster.champ.threshold_reached = value;
        }
        if let Some(value) = protocol_seeded {
            monster.champ.protocol_seeded = value;
        }
    }
}

fn handle_update_book_of_stabbing_state(
    monster_id: usize,
    stab_count: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = stab_count {
            monster.book_of_stabbing.stab_count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.book_of_stabbing.protocol_seeded = value;
        }
    }
}

fn handle_update_awakened_one_state(
    monster_id: usize,
    form1: Option<bool>,
    first_turn: Option<bool>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = form1 {
            monster.awakened_one.form1 = value;
        }
        if let Some(value) = first_turn {
            monster.awakened_one.first_turn = value;
        }
        if let Some(value) = protocol_seeded {
            monster.awakened_one.protocol_seeded = value;
        }
    }
}

fn handle_update_corrupt_heart_state(
    monster_id: usize,
    first_move: Option<bool>,
    move_count: Option<u8>,
    buff_count: Option<u8>,
    protocol_seeded: Option<bool>,
    state: &mut CombatState,
) {
    if let Some(monster) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == monster_id)
    {
        if let Some(value) = first_move {
            monster.corrupt_heart.first_move = value;
        }
        if let Some(value) = move_count {
            monster.corrupt_heart.move_count = value;
        }
        if let Some(value) = buff_count {
            monster.corrupt_heart.buff_count = value;
        }
        if let Some(value) = protocol_seeded {
            monster.corrupt_heart.protocol_seeded = value;
        }
    }
}

pub fn handle_update_monster_runtime(
    monster_id: usize,
    patch: MonsterRuntimePatch,
    state: &mut CombatState,
) {
    match patch {
        MonsterRuntimePatch::Hexaghost {
            activated,
            orb_active_count,
            burn_upgraded,
            divider_damage,
            clear_divider_damage,
        } => handle_update_hexaghost_state(
            monster_id,
            activated,
            orb_active_count,
            burn_upgraded,
            divider_damage,
            clear_divider_damage,
            state,
        ),
        MonsterRuntimePatch::Lagavulin {
            idle_count,
            debuff_turn_count,
            is_out,
            is_out_triggered,
        } => handle_update_lagavulin_state(
            monster_id,
            idle_count,
            debuff_turn_count,
            is_out,
            is_out_triggered,
            state,
        ),
        MonsterRuntimePatch::Guardian {
            damage_threshold,
            damage_taken,
            is_open,
            close_up_triggered,
        } => handle_update_guardian_state(
            monster_id,
            damage_threshold,
            damage_taken,
            is_open,
            close_up_triggered,
            state,
        ),
        MonsterRuntimePatch::Byrd {
            first_move,
            is_flying,
            protocol_seeded,
        } => handle_update_byrd_state(monster_id, first_move, is_flying, protocol_seeded, state),
        MonsterRuntimePatch::Chosen {
            first_turn,
            used_hex,
            protocol_seeded,
        } => handle_update_chosen_state(monster_id, first_turn, used_hex, protocol_seeded, state),
        MonsterRuntimePatch::Snecko {
            first_turn,
            protocol_seeded,
        } => handle_update_snecko_state(monster_id, first_turn, protocol_seeded, state),
        MonsterRuntimePatch::ShelledParasite {
            first_move,
            protocol_seeded,
        } => handle_update_shelled_parasite_state(monster_id, first_move, protocol_seeded, state),
        MonsterRuntimePatch::BronzeAutomaton {
            first_turn,
            num_turns,
            protocol_seeded,
        } => handle_update_bronze_automaton_state(
            monster_id,
            first_turn,
            num_turns,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::BronzeOrb {
            used_stasis,
            protocol_seeded,
        } => handle_update_bronze_orb_state(monster_id, used_stasis, protocol_seeded, state),
        MonsterRuntimePatch::BookOfStabbing {
            stab_count,
            protocol_seeded,
        } => handle_update_book_of_stabbing_state(monster_id, stab_count, protocol_seeded, state),
        MonsterRuntimePatch::Collector {
            initial_spawn,
            ult_used,
            turns_taken,
            protocol_seeded,
        } => handle_update_collector_state(
            monster_id,
            initial_spawn,
            ult_used,
            turns_taken,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::Champ {
            first_turn,
            num_turns,
            forge_times,
            threshold_reached,
            protocol_seeded,
        } => handle_update_champ_state(
            monster_id,
            first_turn,
            num_turns,
            forge_times,
            threshold_reached,
            protocol_seeded,
            state,
        ),
        MonsterRuntimePatch::AwakenedOne {
            form1,
            first_turn,
            protocol_seeded,
        } => {
            handle_update_awakened_one_state(monster_id, form1, first_turn, protocol_seeded, state)
        }
        MonsterRuntimePatch::CorruptHeart {
            first_move,
            move_count,
            buff_count,
            protocol_seeded,
        } => handle_update_corrupt_heart_state(
            monster_id,
            first_move,
            move_count,
            buff_count,
            protocol_seeded,
            state,
        ),
    }
}

pub fn handle_revive_monster(target: usize, state: &mut CombatState) {
    if let Some(monster) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        monster.is_dying = false;
        monster.half_dead = false;
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
