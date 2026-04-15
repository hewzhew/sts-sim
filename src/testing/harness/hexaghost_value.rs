use serde::{Deserialize, Serialize};

use crate::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::{EnemyId, MonsterBehavior};
use crate::content::powers::store;
use crate::state::core::EngineState;

const MAX_PROJECTED_WINDOWS: usize = 9;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HexaghostFutureWindowSummary {
    pub enemy_turn_index: usize,
    pub move_kind: String,
    pub hits: u8,
    pub damage_per_hit: i32,
    pub total_raw_damage: i32,
    pub is_multihit: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HexaghostFutureScriptSummary {
    pub windows: Vec<HexaghostFutureWindowSummary>,
    pub future_raw_damage_total: i32,
    pub future_multihit_raw_damage_total: i32,
    pub future_attack_windows: usize,
    pub future_inferno_windows: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct HexaghostPersistentAttackScriptValue {
    pub strength_down: i32,
    pub future_script_after_action: HexaghostFutureScriptSummary,
    pub future_raw_damage_prevented_total: i32,
    pub future_raw_damage_prevented_by_window: Vec<i32>,
    pub future_multihit_damage_prevented_total: i32,
    pub future_attack_windows_affected: usize,
    pub future_inferno_damage_prevented: i32,
}

pub fn project_hexaghost_future_script(
    engine_state: &EngineState,
    combat: &CombatState,
) -> Option<HexaghostFutureScriptSummary> {
    if !matches!(engine_state, EngineState::CombatPlayerTurn) {
        return None;
    }
    let mut monster = find_hexaghost(combat)?.clone();
    let mut current_strength = combat.get_power(monster.id, PowerId::Strength);
    let (mut current_weak, mut weak_just_applied) = current_weak_status(combat, monster.id);
    let player_current_hp = combat.entities.player.current_hp;
    let mut windows = Vec::new();

    for enemy_turn_index in 1..=MAX_PROJECTED_WINDOWS {
        let window = summarize_window(enemy_turn_index, &monster, current_strength, current_weak)?;
        windows.push(window);
        advance_projection_state(
            &mut monster,
            combat.meta.ascension_level,
            player_current_hp,
            &mut current_strength,
            &mut current_weak,
            &mut weak_just_applied,
        );
    }

    let future_raw_damage_total = windows.iter().map(|window| window.total_raw_damage).sum();
    let future_multihit_raw_damage_total = windows
        .iter()
        .filter(|window| window.is_multihit)
        .map(|window| window.total_raw_damage)
        .sum();
    let future_attack_windows = windows
        .iter()
        .filter(|window| window.total_raw_damage > 0)
        .count();
    let future_inferno_windows = windows
        .iter()
        .filter(|window| window.move_kind == "Inferno")
        .count();

    Some(HexaghostFutureScriptSummary {
        windows,
        future_raw_damage_total,
        future_multihit_raw_damage_total,
        future_attack_windows,
        future_inferno_windows,
    })
}

pub fn analyze_hexaghost_persistent_attack_script(
    before_engine_state: &EngineState,
    before: &CombatState,
    after_engine_state: &EngineState,
    after: &CombatState,
) -> Option<HexaghostPersistentAttackScriptValue> {
    let before_monster = find_hexaghost(before)?;
    let after_monster = find_hexaghost(after)?;
    let strength_down = (before.get_power(before_monster.id, PowerId::Strength)
        - after.get_power(after_monster.id, PowerId::Strength))
    .max(0);
    let before_summary = project_hexaghost_future_script(before_engine_state, before)?;
    let after_summary = project_hexaghost_future_script(after_engine_state, after)?;

    let future_raw_damage_prevented_by_window = before_summary
        .windows
        .iter()
        .zip(after_summary.windows.iter())
        .map(|(before_window, after_window)| {
            (before_window.total_raw_damage - after_window.total_raw_damage).max(0)
        })
        .collect::<Vec<_>>();

    let future_raw_damage_prevented_total =
        (before_summary.future_raw_damage_total - after_summary.future_raw_damage_total).max(0);
    let future_multihit_damage_prevented_total = before_summary
        .windows
        .iter()
        .zip(after_summary.windows.iter())
        .filter(|(before_window, _)| before_window.is_multihit)
        .map(|(before_window, after_window)| {
            (before_window.total_raw_damage - after_window.total_raw_damage).max(0)
        })
        .sum();
    let future_attack_windows_affected = before_summary
        .windows
        .iter()
        .zip(after_summary.windows.iter())
        .filter(|(before_window, after_window)| {
            before_window.total_raw_damage > after_window.total_raw_damage
        })
        .count();
    let future_inferno_damage_prevented = before_summary
        .windows
        .iter()
        .zip(after_summary.windows.iter())
        .filter(|(before_window, _)| before_window.move_kind == "Inferno")
        .map(|(before_window, after_window)| {
            (before_window.total_raw_damage - after_window.total_raw_damage).max(0)
        })
        .sum();

    Some(HexaghostPersistentAttackScriptValue {
        strength_down,
        future_script_after_action: after_summary,
        future_raw_damage_prevented_total,
        future_raw_damage_prevented_by_window,
        future_multihit_damage_prevented_total,
        future_attack_windows_affected,
        future_inferno_damage_prevented,
    })
}

fn find_hexaghost(combat: &CombatState) -> Option<&MonsterEntity> {
    combat.entities.monsters.iter().find(|monster| {
        !monster.is_dying
            && !monster.is_escaped
            && !monster.half_dead
            && EnemyId::from_id(monster.monster_type) == Some(EnemyId::Hexaghost)
    })
}

fn current_weak_status(combat: &CombatState, entity_id: usize) -> (i32, bool) {
    if let Some(power) = store::powers_for(combat, entity_id).and_then(|powers| {
        powers
            .iter()
            .find(|power| power.power_type == PowerId::Weak)
    }) {
        (power.amount.max(0), power.just_applied)
    } else {
        (0, false)
    }
}

fn summarize_window(
    enemy_turn_index: usize,
    monster: &MonsterEntity,
    current_strength: i32,
    current_weak: i32,
) -> Option<HexaghostFutureWindowSummary> {
    let move_kind = move_kind_name(monster.next_move_byte);
    let (hits, base_damage, uses_locked_damage) = intent_shape(&monster.current_intent);
    let mut damage_per_hit = if hits == 0 {
        0
    } else if uses_locked_damage {
        base_damage.max(0)
    } else {
        (base_damage + current_strength).max(0)
    };
    if hits > 0 && current_weak > 0 {
        damage_per_hit = ((damage_per_hit as f32) * 0.75).floor() as i32;
    }
    let total_raw_damage = damage_per_hit * i32::from(hits);
    Some(HexaghostFutureWindowSummary {
        enemy_turn_index,
        move_kind,
        hits,
        damage_per_hit,
        total_raw_damage,
        is_multihit: hits > 1,
    })
}

fn intent_shape(intent: &Intent) -> (u8, i32, bool) {
    match intent {
        Intent::Attack { damage, hits }
        | Intent::AttackBuff { damage, hits }
        | Intent::AttackDebuff { damage, hits }
        | Intent::AttackDefend { damage, hits } => (*hits, *damage, false),
        _ => (0, 0, false),
    }
}

fn move_kind_name(next_move_byte: u8) -> String {
    match next_move_byte {
        1 => "Divider",
        2 => "Tackle",
        3 => "Inflame",
        4 => "Sear",
        5 => "Activate",
        6 => "Inferno",
        _ => "Unknown",
    }
    .to_string()
}

fn advance_projection_state(
    monster: &mut MonsterEntity,
    ascension_level: u8,
    player_current_hp: i32,
    current_strength: &mut i32,
    current_weak: &mut i32,
    weak_just_applied: &mut bool,
) {
    let current_move = monster.next_move_byte;
    let mut forced_next_move = false;
    monster.move_history.push_back(current_move);
    while monster.move_history.len() > 8 {
        monster.move_history.pop_front();
    }

    match current_move {
        5 => {
            monster.hexaghost.activated = true;
            monster.hexaghost.orb_active_count = 6;
            monster.next_move_byte = 1;
            monster.current_intent = Intent::Attack {
                damage: divider_damage(player_current_hp),
                hits: 6,
            };
            forced_next_move = true;
        }
        1 => {
            monster.hexaghost.orb_active_count = 0;
        }
        2 => {
            monster.hexaghost.orb_active_count =
                monster.hexaghost.orb_active_count.saturating_add(1).min(6);
        }
        3 => {
            let str_amount = if ascension_level >= 19 { 3 } else { 2 };
            *current_strength += str_amount;
            monster.hexaghost.orb_active_count =
                monster.hexaghost.orb_active_count.saturating_add(1).min(6);
        }
        4 => {
            monster.hexaghost.orb_active_count =
                monster.hexaghost.orb_active_count.saturating_add(1).min(6);
        }
        6 => {
            monster.hexaghost.orb_active_count = 0;
            monster.hexaghost.burn_upgraded = true;
        }
        _ => {}
    }

    if *current_weak > 0 {
        if *weak_just_applied {
            *weak_just_applied = false;
        } else {
            *current_weak = current_weak.saturating_sub(1);
        }
    } else {
        *weak_just_applied = false;
    }

    if forced_next_move {
        return;
    }

    let (next_move_byte, next_intent) =
        crate::content::monsters::exordium::hexaghost::Hexaghost::roll_move(
            &mut crate::rng::StsRng::new(0),
            monster,
            ascension_level,
            0,
        );
    monster.next_move_byte = next_move_byte;
    monster.current_intent = next_intent;
}

fn divider_damage(player_current_hp: i32) -> i32 {
    (player_current_hp / 12) + 1
}

