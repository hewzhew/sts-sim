//! Power hidden-state import is strict for migrated runtime_state-backed fields.
//!
//! Do not reintroduce `misc`/carry fallback for migrated slices after protocol
//! truth has landed upstream.

use crate::content::powers::PowerId;
use crate::runtime::combat::Power;
use serde_json::Value;

use super::super::build::snapshot_uuid;

#[derive(Clone, Copy)]
struct PowerExtraDataPolicy {
    power_type: PowerId,
}

const POWER_EXTRA_DATA_POLICIES: &[PowerExtraDataPolicy] = &[
    PowerExtraDataPolicy {
        power_type: PowerId::Combust,
    },
    PowerExtraDataPolicy {
        power_type: PowerId::Malleable,
    },
    PowerExtraDataPolicy {
        power_type: PowerId::Flight,
    },
    PowerExtraDataPolicy {
        power_type: PowerId::Stasis,
    },
    PowerExtraDataPolicy {
        power_type: PowerId::PanachePower,
    },
    PowerExtraDataPolicy {
        power_type: PowerId::TheBombPower,
    },
];

fn initialize_power_internal_state(power: &mut Power) {
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
    let power_id = snapshot_power
        .get("id")
        .and_then(|value| value.as_str())
        .unwrap_or("<unknown>");

    let runtime_state_i32 = |key: &str| {
        snapshot_power
            .get("runtime_state")
            .and_then(|runtime| runtime.get(key))
            .and_then(|value| value.as_i64())
            .map(|value| value as i32)
            .unwrap_or_else(|| {
                panic!("strict state_sync: power.runtime_state.{key} missing for {power_id}")
            })
    };

    if power.power_type == PowerId::Combust {
        power.extra_data = runtime_state_i32("hp_loss");
        return;
    }

    if power.power_type == PowerId::Stasis {
        let card_uuid = snapshot_power
            .get("runtime_state")
            .and_then(|runtime| runtime.get("card_uuid"))
            .unwrap_or_else(|| {
                panic!("strict state_sync: power.runtime_state.card_uuid missing for {power_id}")
            });
        power.extra_data = snapshot_uuid(card_uuid, 0) as i32;
        return;
    }

    if power.power_type == PowerId::Malleable {
        power.extra_data = runtime_state_i32("base_power");
        return;
    }

    if power.power_type == PowerId::Flight {
        power.extra_data = runtime_state_i32("stored_amount");
        return;
    }

    if power.power_type == PowerId::PanachePower || power.power_type == PowerId::TheBombPower {
        power.extra_data = runtime_state_i32("damage");
        return;
    }

    if POWER_EXTRA_DATA_POLICIES
        .iter()
        .any(|policy| policy.power_type == power.power_type)
    {
        if let Some(damage) = snapshot_power.get("damage").and_then(|v| v.as_i64()) {
            power.extra_data = damage as i32;
            return;
        }
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

fn apply_power_extra_data_policies(prev_powers: &[Power], next_powers: &mut [Power]) {
    for policy in POWER_EXTRA_DATA_POLICIES {
        for next_power in next_powers
            .iter_mut()
            .filter(|power| power.power_type == policy.power_type)
        {
            let previous_power = prev_powers
                .iter()
                .find(|power| {
                    power.power_type == policy.power_type
                        && power.instance_id == next_power.instance_id
                })
                .or_else(|| {
                    next_power
                        .instance_id
                        .is_none()
                        .then(|| {
                            prev_powers
                                .iter()
                                .find(|power| power.power_type == policy.power_type)
                        })
                        .flatten()
                });
            if let Some(previous_power) = previous_power {
                next_power.extra_data = previous_power.extra_data;
            }
        }
    }
}
