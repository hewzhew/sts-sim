use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::combat::Power;
use serde_json::Value;

use super::power::sync_power_extra_data_from_snapshot;

#[derive(Clone, Copy)]
struct MissingMonsterPowerPolicy {
    monster_type: EnemyId,
    power_type: PowerId,
    prefer_previous_over_seeded_snapshot: bool,
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
    if let Some(existing_powers) = existing_powers {
        apply_missing_monster_power_policies(monster_type, existing_powers, powers);
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
