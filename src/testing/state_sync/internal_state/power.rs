//! Power hidden-state import is strict for current runtime_state-backed slices.
//!
//! Do not reintroduce `damage`/`misc` importer fallback for migrated slices
//! after protocol truth has landed upstream.

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
}
