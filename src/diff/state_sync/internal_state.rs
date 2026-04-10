use crate::combat::{CombatState, Power};
use crate::content::monsters::EnemyId;
use crate::content::powers::store;
use crate::content::powers::PowerId;
use crate::content::relics::{RelicId, RelicState};
use serde_json::Value;

#[derive(Clone, Copy)]
struct MissingMonsterPowerPolicy {
    monster_type: EnemyId,
    power_type: PowerId,
    prefer_previous_over_seeded_snapshot: bool,
}

#[derive(Clone, Copy)]
struct PowerExtraDataPolicy {
    power_type: PowerId,
}

#[derive(Clone, Copy)]
struct RelicCounterPolicy {
    relic_id: RelicId,
}

#[derive(Clone, Copy)]
struct RelicUsedUpPolicy {
    relic_id: RelicId,
}

const MISSING_MONSTER_POWER_POLICIES: &[MissingMonsterPowerPolicy] = &[
    MissingMonsterPowerPolicy {
        monster_type: EnemyId::TheGuardian,
        power_type: PowerId::GuardianThreshold,
        prefer_previous_over_seeded_snapshot: true,
    },
    MissingMonsterPowerPolicy {
        monster_type: EnemyId::GremlinWarrior,
        power_type: PowerId::Angry,
        prefer_previous_over_seeded_snapshot: true,
    },
];

const POWER_EXTRA_DATA_POLICIES: &[PowerExtraDataPolicy] = &[
    PowerExtraDataPolicy {
        power_type: PowerId::Malleable,
    },
    PowerExtraDataPolicy {
        power_type: PowerId::Flight,
    },
    PowerExtraDataPolicy {
        power_type: PowerId::Stasis,
    },
];

const RELIC_COUNTER_POLICIES: &[RelicCounterPolicy] = &[RelicCounterPolicy {
    relic_id: RelicId::ArtOfWar,
}];

const RELIC_USED_UP_POLICIES: &[RelicUsedUpPolicy] = &[RelicUsedUpPolicy {
    relic_id: RelicId::Necronomicon,
}];

pub fn initialize_power_internal_state(power: &mut Power) {
    if POWER_EXTRA_DATA_POLICIES
        .iter()
        .any(|policy| policy.power_type == power.power_type)
    {
        power.extra_data = power.amount;
    }
}

pub fn initialize_power_internal_state_from_snapshot(power: &mut Power, snapshot_power: &Value) {
    initialize_power_internal_state(power);
    sync_power_extra_data_from_snapshot_power(power, snapshot_power);
}

pub fn sync_power_extra_data_from_snapshot_power(power: &mut Power, snapshot_power: &Value) {
    if POWER_EXTRA_DATA_POLICIES
        .iter()
        .any(|policy| policy.power_type == power.power_type)
    {
        if let Some(misc) = snapshot_power.get("misc").and_then(|v| v.as_i64()) {
            power.extra_data = misc as i32;
        }
    }
}

pub fn sync_power_extra_data_from_snapshot(
    existing_powers: Option<&[Power]>,
    next_powers: &mut [Power],
) {
    let Some(existing_powers) = existing_powers else {
        return;
    };
    apply_power_extra_data_policies(existing_powers, next_powers);
}

pub fn seed_monster_internal_state_from_snapshot(
    monster_type: usize,
    snapshot_monster: &Value,
    powers: &mut Vec<Power>,
) {
    for policy in MISSING_MONSTER_POWER_POLICIES {
        if monster_type != policy.monster_type as usize {
            continue;
        }

        if powers
            .iter()
            .any(|power| power.power_type == policy.power_type)
        {
            continue;
        }

        let Some(source_amount) =
            monster_internal_seed_amount(policy.power_type, snapshot_monster, powers)
        else {
            continue;
        };

        powers.push(Power {
            power_type: policy.power_type,
            amount: source_amount,
            extra_data: 0,
            just_applied: false,
        });
    }
}

pub fn sync_monster_internal_state_from_snapshot(
    monster_type: usize,
    existing_powers: Option<&[Power]>,
    snapshot_monster: &Value,
    powers: &mut Vec<Power>,
) {
    sync_power_extra_data_from_snapshot(existing_powers, powers);
    seed_monster_internal_state_from_snapshot(monster_type, snapshot_monster, powers);
    if let Some(existing_powers) = existing_powers {
        apply_missing_monster_power_policies(monster_type, existing_powers, powers);
    }
}

pub fn initialize_relic_runtime_state(relic: &mut RelicState) {
    let _ = relic;
}

pub fn sync_relic_runtime_state_from_snapshot(
    existing_relic: Option<&RelicState>,
    next_relic: &mut RelicState,
    snapshot_counter: i32,
) {
    next_relic.counter = snapshot_counter;
    if next_relic.id == RelicId::ArtOfWar && snapshot_counter == -1 {
        next_relic.counter = existing_relic
            .map(|relic| relic.counter)
            .filter(|counter| *counter >= 0)
            .unwrap_or(-1);
    }
    apply_relic_used_up_policies(existing_relic, next_relic);
}

pub fn carry_internal_runtime_state(previous: &CombatState, next: &mut CombatState) {
    carry_monster_logical_positions(previous, next);
    carry_hidden_monster_turn_state(previous, next);
    carry_internal_monster_power_state(previous, next);
    carry_internal_limbo_state(previous, next);
    carry_internal_relic_state(previous, next);
}

fn carry_monster_logical_positions(previous: &CombatState, next: &mut CombatState) {
    if contains_gremlin_leader(previous) || contains_gremlin_leader(next) {
        carry_gremlin_leader_logical_positions(previous, next);
        return;
    }

    for next_monster in &mut next.entities.monsters {
        if let Some(previous_monster) = previous
            .entities
            .monsters
            .iter()
            .find(|monster| monster.id == next_monster.id)
        {
            next_monster.logical_position = previous_monster.logical_position;
        }
    }
}

fn contains_gremlin_leader(state: &CombatState) -> bool {
    state
        .entities
        .monsters
        .iter()
        .any(|monster| EnemyId::from_id(monster.monster_type) == Some(EnemyId::GremlinLeader))
}

fn is_gremlin_leader_minion(monster_type: usize) -> bool {
    matches!(
        EnemyId::from_id(monster_type),
        Some(EnemyId::GremlinFat)
            | Some(EnemyId::GremlinWarrior)
            | Some(EnemyId::GremlinThief)
            | Some(EnemyId::GremlinTsundere)
            | Some(EnemyId::GremlinWizard)
    )
}

fn carry_gremlin_leader_logical_positions(previous: &CombatState, next: &mut CombatState) {
    let mut previous_used = vec![false; previous.entities.monsters.len()];
    let mut next_used = vec![false; next.entities.monsters.len()];

    // Keep the leader anchored first.
    for (next_idx, next_monster) in next.entities.monsters.iter_mut().enumerate() {
        if EnemyId::from_id(next_monster.monster_type) != Some(EnemyId::GremlinLeader) {
            continue;
        }
        if let Some((idx, previous_monster)) =
            previous
                .entities
                .monsters
                .iter()
                .enumerate()
                .find(|(_, monster)| {
                    EnemyId::from_id(monster.monster_type) == Some(EnemyId::GremlinLeader)
                })
        {
            previous_used[idx] = true;
            next_used[next_idx] = true;
            next_monster.logical_position = previous_monster.logical_position;
        } else {
            next_used[next_idx] = true;
            next_monster.logical_position =
                crate::content::monsters::city::gremlin_leader::GremlinLeader::LEADER_LOGICAL_POSITION;
        }
    }

    for strict_status_match in [true, false] {
        for (next_idx, next_monster) in next.entities.monsters.iter_mut().enumerate() {
            if next_used[next_idx]
                || EnemyId::from_id(next_monster.monster_type) == Some(EnemyId::GremlinLeader)
            {
                continue;
            }

            let maybe_match =
                previous
                    .entities
                    .monsters
                    .iter()
                    .enumerate()
                    .find(|(idx, monster)| {
                        !previous_used[*idx]
                            && monster.monster_type == next_monster.monster_type
                            && (!strict_status_match
                                || (monster.is_dying == next_monster.is_dying
                                    && monster.half_dead == next_monster.half_dead))
                    });

            if let Some((idx, previous_monster)) = maybe_match {
                previous_used[idx] = true;
                next_used[next_idx] = true;
                next_monster.logical_position = previous_monster.logical_position;
            }
        }
    }

    let mut occupied_living_slots = [false; 3];
    for monster in &next.entities.monsters {
        if !is_gremlin_leader_minion(monster.monster_type) || monster.is_dying {
            continue;
        }
        for (slot_idx, logical_position) in crate::content::monsters::city::gremlin_leader::GremlinLeader::GREMLIN_SLOT_LOGICAL_POSITIONS
            .iter()
            .enumerate()
        {
            if monster.logical_position == *logical_position {
                occupied_living_slots[slot_idx] = true;
            }
        }
    }

    for (next_idx, next_monster) in next.entities.monsters.iter_mut().enumerate() {
        if next_used[next_idx]
            || !is_gremlin_leader_minion(next_monster.monster_type)
            || next_monster.is_dying
        {
            continue;
        }

        let next_slot = occupied_living_slots
            .iter()
            .position(|occupied| !occupied)
            .unwrap_or(0);
        occupied_living_slots[next_slot] = true;
        next_used[next_idx] = true;
        next_monster.logical_position =
            crate::content::monsters::city::gremlin_leader::GremlinLeader::GREMLIN_SLOT_LOGICAL_POSITIONS[next_slot];
    }
}

fn carry_hidden_monster_turn_state(previous: &CombatState, next: &mut CombatState) {
    for next_monster in &mut next.entities.monsters {
        let Some(previous_monster) = previous
            .entities
            .monsters
            .iter()
            .find(|monster| monster.id == next_monster.id)
        else {
            continue;
        };

        // Some live snapshots omit the current monster move entirely (for example
        // under Runic Dome). Preserve the internal next_move_byte and move
        // history from the previous runtime state so END can still execute the
        // hidden monster turn faithfully.
        if next_monster.next_move_byte == 0 && previous_monster.next_move_byte != 0 {
            next_monster.next_move_byte = previous_monster.next_move_byte;
            next_monster.move_history = previous_monster.move_history.clone();
        }

        if EnemyId::from_id(next_monster.monster_type) == Some(EnemyId::Hexaghost)
            && !next_monster.hexaghost.activated
            && previous_monster.hexaghost.activated
        {
            next_monster.hexaghost = previous_monster.hexaghost.clone();
        }

        if EnemyId::from_id(next_monster.monster_type) == Some(EnemyId::Darkling) {
            if next_monster.darkling.nip_dmg == 0 && previous_monster.darkling.nip_dmg > 0 {
                next_monster.darkling.nip_dmg = previous_monster.darkling.nip_dmg;
            }
            if !next_monster.move_history.is_empty() || next_monster.next_move_byte != 0 {
                next_monster.darkling.first_move = false;
            } else if previous_monster.darkling.first_move {
                next_monster.darkling.first_move = true;
            }
        }
    }
}

fn carry_internal_monster_power_state(previous: &CombatState, next: &mut CombatState) {
    let monsters: Vec<(usize, usize)> = next
        .entities
        .monsters
        .iter()
        .map(|monster| (monster.id, monster.monster_type))
        .collect();

    for (monster_id, monster_type) in monsters {
        let Some(prev_powers) = previous.entities.power_db.get(&monster_id) else {
            continue;
        };
        let next_powers = store::ensure_powers_for_mut(next, monster_id);

        apply_missing_monster_power_policies(monster_type, prev_powers, next_powers);
        apply_power_extra_data_policies(prev_powers, next_powers);
    }
}

fn apply_missing_monster_power_policies(
    monster_type: usize,
    prev_powers: &[Power],
    next_powers: &mut Vec<Power>,
) {
    for policy in MISSING_MONSTER_POWER_POLICIES {
        if monster_type != policy.monster_type as usize {
            continue;
        }

        let previous_power = prev_powers
            .iter()
            .find(|power| power.power_type == policy.power_type)
            .cloned();

        if policy.prefer_previous_over_seeded_snapshot {
            if let (Some(previous_power), Some(next_power)) = (
                previous_power.clone(),
                next_powers
                    .iter_mut()
                    .find(|power| power.power_type == policy.power_type),
            ) {
                next_power.amount = previous_power.amount.max(next_power.amount);
                next_power.extra_data = previous_power.extra_data;
                next_power.just_applied = previous_power.just_applied;
                continue;
            }
        }

        if next_powers
            .iter()
            .any(|power| power.power_type == policy.power_type)
        {
            continue;
        }

        if let Some(previous_power) = previous_power {
            next_powers.push(previous_power);
        }
    }
}

fn apply_power_extra_data_policies(prev_powers: &[Power], next_powers: &mut [Power]) {
    for policy in POWER_EXTRA_DATA_POLICIES {
        let Some(previous_power) = prev_powers
            .iter()
            .find(|power| power.power_type == policy.power_type)
        else {
            continue;
        };

        let Some(next_power) = next_powers
            .iter_mut()
            .find(|power| power.power_type == policy.power_type)
        else {
            continue;
        };

        next_power.extra_data = previous_power.extra_data;
    }
}

fn carry_internal_relic_state(previous: &CombatState, next: &mut CombatState) {
    for next_relic in &mut next.entities.player.relics {
        let Some(previous_relic) = previous
            .entities
            .player
            .relics
            .iter()
            .find(|relic| relic.id == next_relic.id)
        else {
            continue;
        };

        apply_relic_counter_policies(previous_relic.counter, next_relic);
        apply_relic_used_up_policies(Some(previous_relic), next_relic);
    }
}

fn carry_internal_limbo_state(previous: &CombatState, next: &mut CombatState) {
    let stasis_uuids = next
        .entities
        .power_db
        .values()
        .flat_map(|powers| powers.iter())
        .filter(|power| power.power_type == PowerId::Stasis && power.extra_data > 0)
        .map(|power| power.extra_data as u32)
        .collect::<Vec<_>>();

    for uuid in stasis_uuids {
        if next.zones.limbo.iter().any(|card| card.uuid == uuid) {
            continue;
        }
        if let Some(card) = previous.zones.limbo.iter().find(|card| card.uuid == uuid) {
            next.zones.limbo.push(card.clone());
        }
    }
}

fn apply_relic_counter_policies(
    previous_counter: i32,
    next_relic: &mut crate::content::relics::RelicState,
) {
    for policy in RELIC_COUNTER_POLICIES {
        if next_relic.id == policy.relic_id {
            next_relic.counter = previous_counter;
        }
    }
}

fn apply_relic_used_up_policies(
    previous_relic: Option<&crate::content::relics::RelicState>,
    next_relic: &mut crate::content::relics::RelicState,
) {
    for policy in RELIC_USED_UP_POLICIES {
        if next_relic.id == policy.relic_id {
            next_relic.used_up = previous_relic.map(|relic| relic.used_up).unwrap_or(false);
        }
    }
}

fn monster_internal_seed_amount(
    power_type: PowerId,
    snapshot_monster: &Value,
    powers: &[Power],
) -> Option<i32> {
    match power_type {
        PowerId::GuardianThreshold => snapshot_monster
            .get("guardian_dmg_threshold")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .or_else(|| mode_shift_amount(powers)),
        PowerId::Angry => snapshot_monster
            .get("powers")
            .and_then(|v| v.as_array())
            .and_then(|powers_arr| {
                powers_arr.iter().find_map(|power| {
                    (power.get("id").and_then(|v| v.as_str()) == Some("Angry"))
                        .then(|| {
                            power
                                .get("amount")
                                .and_then(|v| v.as_i64())
                                .map(|v| v as i32)
                        })
                        .flatten()
                })
            })
            .or(Some(1)),
        _ => mode_shift_amount(powers),
    }
}

fn mode_shift_amount(powers: &[Power]) -> Option<i32> {
    powers
        .iter()
        .find(|power| power.power_type == PowerId::ModeShift)
        .map(|power| power.amount)
}
