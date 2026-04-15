//! Relic runtime-state import is strict for runtime-only flags and counters.
//!
//! Missing protocol truth should now fail fast instead of falling back to
//! previous Rust state.

use crate::content::relics::{RelicId, RelicState};
use serde_json::Value;

#[derive(Clone, Copy)]
struct RuntimeOnlyUsedUpRelic {
    relic_id: RelicId,
}

const RUNTIME_ONLY_USED_UP_RELICS: &[RuntimeOnlyUsedUpRelic] = &[
    RuntimeOnlyUsedUpRelic {
        relic_id: RelicId::HoveringKite,
    },
    RuntimeOnlyUsedUpRelic {
        relic_id: RelicId::LizardTail,
    },
    RuntimeOnlyUsedUpRelic {
        relic_id: RelicId::Necronomicon,
    },
];

pub fn initialize_relic_runtime_state(relic: &mut RelicState) {
    let _ = relic;
}

pub fn sync_relic_runtime_state_from_snapshot(
    next_relic: &mut RelicState,
    snapshot_counter: i32,
    snapshot_runtime_counter: Option<i32>,
    snapshot_used_up: Option<bool>,
    snapshot_runtime_used_up: Option<bool>,
    snapshot_runtime_amount: Option<i32>,
) {
    next_relic.counter = snapshot_counter;
    if let Some(runtime_counter) = snapshot_runtime_counter {
        next_relic.counter = runtime_counter;
    } else if relic_requires_runtime_counter(next_relic.id) {
        panic!("strict state_sync: relic.runtime_state.counter missing for {:?}", next_relic.id);
    }
    if let Some(runtime_amount) = snapshot_runtime_amount {
        next_relic.amount = runtime_amount;
    } else if relic_requires_runtime_amount(next_relic.id) {
        panic!("strict state_sync: relic.runtime_state.amount missing for {:?}", next_relic.id);
    }
    if let Some(runtime_used_up) = snapshot_runtime_used_up {
        next_relic.used_up = runtime_used_up;
    } else if relic_uses_runtime_only_activation_flag(next_relic.id) {
        panic!("strict state_sync: relic.runtime_state.used_up missing for {:?}", next_relic.id);
    } else if let Some(used_up) = snapshot_used_up {
        next_relic.used_up = used_up;
    }
}

pub fn snapshot_runtime_used_up_for_relic(
    relic_id: RelicId,
    snapshot_relic: &Value,
) -> Option<bool> {
    let runtime_state = snapshot_relic.get("runtime_state");
    match relic_id {
        RelicId::CentennialPuzzle => runtime_state
            .unwrap_or_else(|| {
                panic!("strict state_sync: relic.runtime_state missing for {:?}", relic_id)
            })
            .get("used_this_combat")
            .and_then(|value| value.as_bool()),
        _ if relic_uses_runtime_only_activation_flag(relic_id) => runtime_state
            .unwrap_or_else(|| {
                panic!("strict state_sync: relic.runtime_state missing for {:?}", relic_id)
            })
            .get("used_up")
            .and_then(|value| value.as_bool()),
        _ => None,
    }
}

pub fn snapshot_runtime_counter_for_relic(
    relic_id: RelicId,
    snapshot_relic: &Value,
) -> Option<i32> {
    let runtime_state = snapshot_relic.get("runtime_state");
    match relic_id {
        RelicId::ArtOfWar => {
            let runtime_state = runtime_state.unwrap_or_else(|| {
                panic!("strict state_sync: relic.runtime_state missing for {:?}", relic_id)
            });
            let gain_energy_next = runtime_state
                .get("gain_energy_next")
                .and_then(|value| value.as_bool())?;
            let first_turn = runtime_state
                .get("first_turn")
                .and_then(|value| value.as_bool())?;
            Some(if first_turn {
                -1
            } else if gain_energy_next {
                1
            } else {
                0
            })
        }
        _ => None,
    }
}

pub fn snapshot_runtime_amount_for_relic(
    relic_id: RelicId,
    snapshot_relic: &Value,
) -> Option<i32> {
    let runtime_state = snapshot_relic.get("runtime_state");
    match relic_id {
        RelicId::Pocketwatch => runtime_state
            .unwrap_or_else(|| {
                panic!("strict state_sync: relic.runtime_state missing for {:?}", relic_id)
            })
            .get("first_turn")
            .and_then(|value| value.as_bool())
            .map(i32::from),
        _ => None,
    }
}

fn relic_requires_runtime_counter(relic_id: RelicId) -> bool {
    matches!(relic_id, RelicId::ArtOfWar)
}

fn relic_requires_runtime_amount(relic_id: RelicId) -> bool {
    matches!(relic_id, RelicId::Pocketwatch)
}

fn relic_uses_runtime_only_activation_flag(relic_id: RelicId) -> bool {
    RUNTIME_ONLY_USED_UP_RELICS
        .iter()
        .any(|policy| policy.relic_id == relic_id)
        || relic_id == RelicId::CentennialPuzzle
}
