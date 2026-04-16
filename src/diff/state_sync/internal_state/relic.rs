//! Relic runtime-state import is strict for current runtime_state-backed
//! counters, activation flags, and auxiliary amounts.
//!
//! Missing protocol truth should now fail fast instead of falling back to
//! previous Rust state or top-level compatibility fields.

use crate::content::relics::{RelicId, RelicState};
use serde_json::Value;

pub fn initialize_relic_runtime_state(relic: &mut RelicState) {
    let _ = relic;
}

pub fn sync_relic_runtime_state_from_snapshot(
    next_relic: &mut RelicState,
    snapshot_runtime_counter: i32,
    snapshot_runtime_used_up: bool,
    snapshot_runtime_amount: Option<i32>,
) {
    next_relic.counter = snapshot_runtime_counter;
    if let Some(runtime_amount) = snapshot_runtime_amount {
        next_relic.amount = runtime_amount;
    } else if relic_requires_runtime_amount(next_relic.id) {
        panic!(
            "strict state_sync: relic.runtime_state.amount missing for {:?}",
            next_relic.id
        );
    }
    next_relic.used_up = snapshot_runtime_used_up;
}

pub fn snapshot_runtime_used_up_for_relic(relic_id: RelicId, snapshot_relic: &Value) -> bool {
    let runtime_state = snapshot_relic.get("runtime_state");
    match relic_id {
        RelicId::CentennialPuzzle => runtime_state
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: relic.runtime_state missing for {:?}",
                    relic_id
                )
            })
            .get("used_this_combat")
            .and_then(|value| value.as_bool())
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: relic.runtime_state.used_this_combat missing for {:?}",
                    relic_id
                )
            }),
        _ => runtime_state
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: relic.runtime_state missing for {:?}",
                    relic_id
                )
            })
            .get("used_up")
            .and_then(|value| value.as_bool())
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: relic.runtime_state.used_up missing for {:?}",
                    relic_id
                )
            }),
    }
}

pub fn snapshot_runtime_counter_for_relic(relic_id: RelicId, snapshot_relic: &Value) -> i32 {
    let runtime_state = snapshot_relic.get("runtime_state");
    match relic_id {
        RelicId::ArtOfWar => {
            let runtime_state = runtime_state.unwrap_or_else(|| {
                panic!(
                    "strict state_sync: relic.runtime_state missing for {:?}",
                    relic_id
                )
            });
            let gain_energy_next = runtime_state
                .get("gain_energy_next")
                .and_then(|value| value.as_bool())
                .unwrap_or_else(|| {
                    panic!(
                        "strict state_sync: relic.runtime_state.gain_energy_next missing for {:?}",
                        relic_id
                    )
                });
            let first_turn = runtime_state
                .get("first_turn")
                .and_then(|value| value.as_bool())
                .unwrap_or_else(|| {
                    panic!(
                        "strict state_sync: relic.runtime_state.first_turn missing for {:?}",
                        relic_id
                    )
                });
            if first_turn {
                -1
            } else if gain_energy_next {
                1
            } else {
                0
            }
        }
        _ => runtime_state
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: relic.runtime_state missing for {:?}",
                    relic_id
                )
            })
            .get("counter")
            .and_then(|value| value.as_i64())
            .map(|value| value as i32)
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: relic.runtime_state.counter missing for {:?}",
                    relic_id
                )
            }),
    }
}

pub fn snapshot_runtime_amount_for_relic(relic_id: RelicId, snapshot_relic: &Value) -> Option<i32> {
    let runtime_state = snapshot_relic.get("runtime_state");
    match relic_id {
        RelicId::Pocketwatch => runtime_state
            .unwrap_or_else(|| {
                panic!(
                    "strict state_sync: relic.runtime_state missing for {:?}",
                    relic_id
                )
            })
            .get("first_turn")
            .and_then(|value| value.as_bool())
            .map(i32::from),
        _ => None,
    }
}

fn relic_requires_runtime_amount(relic_id: RelicId) -> bool {
    matches!(relic_id, RelicId::Pocketwatch)
}
