//! Monster hidden-state import is now strict by design.
//!
//! Any remaining policy table here is protocol debt being retired slice by slice,
//! not a long-term source of combat truth.

use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::combat::Power;
use serde_json::Value;

use super::power::sync_power_extra_data_from_snapshot;

#[derive(Clone, Copy)]
struct MissingMonsterPowerPolicy {
    monster_type: EnemyId,
    power_type: PowerId,
}

const MISSING_MONSTER_POWER_POLICIES: &[MissingMonsterPowerPolicy] = &[
    MissingMonsterPowerPolicy {
        monster_type: EnemyId::TheGuardian,
        power_type: PowerId::GuardianThreshold,
    },
    MissingMonsterPowerPolicy {
        monster_type: EnemyId::GremlinWarrior,
        power_type: PowerId::Angry,
    },
];

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
            monster_internal_seed_amount(policy.monster_type, policy.power_type, snapshot_monster)
        else {
            continue;
        };

        powers.push(Power {
            power_type: policy.power_type,
            instance_id: None,
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
}

fn runtime_state<'a>(snapshot_monster: &'a Value, monster_type: EnemyId) -> &'a Value {
    snapshot_monster.get("runtime_state").unwrap_or_else(|| {
        panic!("strict state_sync: monster.runtime_state missing for {monster_type:?}")
    })
}

fn runtime_state_i32(snapshot_monster: &Value, monster_type: EnemyId, key: &str) -> i32 {
    runtime_state(snapshot_monster, monster_type)
        .get(key)
        .and_then(|value| value.as_i64())
        .map(|value| value as i32)
        .unwrap_or_else(|| {
            panic!("strict state_sync: monster.runtime_state.{key} missing for {monster_type:?}")
        })
}

fn monster_internal_seed_amount(
    monster_type: EnemyId,
    power_type: PowerId,
    snapshot_monster: &Value,
) -> Option<i32> {
    match power_type {
        PowerId::GuardianThreshold => Some(runtime_state_i32(
            snapshot_monster,
            monster_type,
            "guardian_threshold",
        )),
        PowerId::Angry => Some(runtime_state_i32(
            snapshot_monster,
            monster_type,
            "angry_amount",
        )),
        _ => None,
    }
}
