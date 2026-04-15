use crate::content::relics::{RelicId, RelicState};
use serde_json::Value;

#[derive(Clone, Copy)]
struct RelicUsedUpPolicy {
    relic_id: RelicId,
}

const RELIC_USED_UP_POLICIES: &[RelicUsedUpPolicy] = &[
    RelicUsedUpPolicy {
        relic_id: RelicId::HoveringKite,
    },
    RelicUsedUpPolicy {
        relic_id: RelicId::LizardTail,
    },
    RelicUsedUpPolicy {
        relic_id: RelicId::Necronomicon,
    },
];

pub fn initialize_relic_runtime_state(relic: &mut RelicState) {
    let _ = relic;
}

pub fn sync_relic_runtime_state_from_snapshot(
    existing_relic: Option<&RelicState>,
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
    }
    if let Some(runtime_amount) = snapshot_runtime_amount {
        next_relic.amount = runtime_amount;
    }
    if next_relic.id == RelicId::ArtOfWar
        && snapshot_runtime_counter.is_none()
        && snapshot_counter == -1
    {
        next_relic.counter = existing_relic
            .map(|relic| relic.counter)
            .filter(|counter| *counter >= 0)
            .unwrap_or(-1);
    }
    if let Some(runtime_used_up) = snapshot_runtime_used_up {
        next_relic.used_up = runtime_used_up;
    } else if relic_uses_runtime_only_activation_flag(next_relic.id) {
        apply_relic_used_up_policies(existing_relic, next_relic);
        if snapshot_used_up == Some(true) {
            next_relic.used_up = true;
        }
    } else if let Some(used_up) = snapshot_used_up {
        next_relic.used_up = used_up;
    } else {
        apply_relic_used_up_policies(existing_relic, next_relic);
    }
}

pub fn snapshot_runtime_used_up_for_relic(
    relic_id: RelicId,
    snapshot_relic: &Value,
) -> Option<bool> {
    let runtime_state = snapshot_relic.get("runtime_state")?;
    match relic_id {
        RelicId::CentennialPuzzle => runtime_state
            .get("used_this_combat")
            .and_then(|value| value.as_bool()),
        _ => None,
    }
}

pub fn snapshot_runtime_counter_for_relic(
    relic_id: RelicId,
    snapshot_relic: &Value,
) -> Option<i32> {
    let runtime_state = snapshot_relic.get("runtime_state")?;
    match relic_id {
        RelicId::ArtOfWar => {
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
    let runtime_state = snapshot_relic.get("runtime_state")?;
    match relic_id {
        RelicId::Pocketwatch => runtime_state
            .get("first_turn")
            .and_then(|value| value.as_bool())
            .map(i32::from),
        _ => None,
    }
}

fn apply_relic_used_up_policies(previous_relic: Option<&RelicState>, next_relic: &mut RelicState) {
    for policy in RELIC_USED_UP_POLICIES {
        if next_relic.id == policy.relic_id {
            next_relic.used_up = previous_relic.map(|relic| relic.used_up).unwrap_or(false);
        }
    }
}

fn relic_uses_runtime_only_activation_flag(relic_id: RelicId) -> bool {
    RELIC_USED_UP_POLICIES
        .iter()
        .any(|policy| policy.relic_id == relic_id)
}
