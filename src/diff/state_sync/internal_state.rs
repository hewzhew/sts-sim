use crate::combat::Power;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::content::relics::{RelicId, RelicState};
use serde_json::Value;

fn stable_u32_from_str(s: &str) -> u32 {
    let mut hash = 0x811C9DC5u32;
    for &byte in s.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn snapshot_uuid(raw: &Value, fallback: u32) -> u32 {
    if let Some(uuid) = raw.as_u64() {
        uuid as u32
    } else if let Some(uuid) = raw.as_str() {
        stable_u32_from_str(uuid)
    } else {
        fallback
    }
}

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

pub fn initialize_power_internal_state(power: &mut Power) {
    if power.power_type == PowerId::Combust {
        // Java CombustPower has hidden `hpLoss` state that is not fully exposed in
        // the current live-comm protocol. Seed a conservative default of 1 and rely
        // on carry/sync to preserve the true stacked value across snapshots.
        power.extra_data = 1;
        return;
    }

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
    if power.power_type == PowerId::Combust {
        // Java snapshots currently expose CombustPower.hpLoss as `misc` in combat
        // snapshots. Prefer an explicit `hp_loss` field if protocol gains one later,
        // then fall back to `misc`.
        if let Some(hp_loss) = snapshot_power.get("hp_loss").and_then(|v| v.as_i64()) {
            power.extra_data = hp_loss as i32;
        } else if let Some(hp_loss) = snapshot_power.get("misc").and_then(|v| v.as_i64()) {
            power.extra_data = hp_loss as i32;
        }
        return;
    }

    if power.power_type == PowerId::Stasis {
        if let Some(card_uuid) = snapshot_power.get("card").and_then(|card| card.get("uuid")) {
            power.extra_data = snapshot_uuid(card_uuid, 0) as i32;
        }
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

pub fn snapshot_runtime_amount_for_relic(relic_id: RelicId, snapshot_relic: &Value) -> Option<i32> {
    let runtime_state = snapshot_relic.get("runtime_state")?;
    match relic_id {
        RelicId::Pocketwatch => runtime_state
            .get("first_turn")
            .and_then(|value| value.as_bool())
            .map(i32::from),
        _ => None,
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

fn relic_uses_runtime_only_activation_flag(relic_id: RelicId) -> bool {
    RELIC_USED_UP_POLICIES
        .iter()
        .any(|policy| policy.relic_id == relic_id)
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

#[cfg(test)]
mod tests {
    use super::{
        snapshot_runtime_counter_for_relic, sync_power_extra_data_from_snapshot_power,
        sync_relic_runtime_state_from_snapshot, RELIC_USED_UP_POLICIES,
    };
    use crate::combat::Power;
    use crate::content::powers::PowerId;
    use crate::content::relics::{RelicId, RelicState};
    use serde_json::json;

    #[test]
    fn relic_used_up_policies_cover_runtime_only_activation_flags() {
        let policy_ids: Vec<RelicId> = RELIC_USED_UP_POLICIES
            .iter()
            .map(|policy| policy.relic_id)
            .collect();

        assert!(policy_ids.contains(&RelicId::HoveringKite));
        assert!(policy_ids.contains(&RelicId::LizardTail));
        assert!(policy_ids.contains(&RelicId::Necronomicon));
    }

    #[test]
    fn sync_relic_runtime_state_preserves_hovering_kite_used_up() {
        let mut previous = RelicState::new(RelicId::HoveringKite);
        previous.used_up = true;

        let mut next = RelicState::new(RelicId::HoveringKite);
        next.used_up = false;

        sync_relic_runtime_state_from_snapshot(
            Some(&previous),
            &mut next,
            -1,
            None,
            None,
            None,
            None,
        );

        assert!(next.used_up);
    }

    #[test]
    fn sync_relic_runtime_state_ignores_false_snapshot_for_runtime_only_flags() {
        let mut previous = RelicState::new(RelicId::HoveringKite);
        previous.used_up = true;

        let mut next = RelicState::new(RelicId::HoveringKite);
        next.used_up = false;

        sync_relic_runtime_state_from_snapshot(
            Some(&previous),
            &mut next,
            -1,
            None,
            Some(false),
            None,
            None,
        );

        assert!(next.used_up);
    }

    #[test]
    fn sync_relic_runtime_state_still_accepts_true_snapshot_for_runtime_only_flags() {
        let previous = RelicState::new(RelicId::HoveringKite);

        let mut next = RelicState::new(RelicId::HoveringKite);
        next.used_up = false;

        sync_relic_runtime_state_from_snapshot(
            Some(&previous),
            &mut next,
            -1,
            None,
            Some(true),
            None,
            None,
        );

        assert!(next.used_up);
    }

    #[test]
    fn sync_relic_runtime_state_prefers_runtime_protocol_truth_for_centennial_puzzle() {
        let mut previous = RelicState::new(RelicId::CentennialPuzzle);
        previous.used_up = true;

        let mut next = RelicState::new(RelicId::CentennialPuzzle);
        next.used_up = false;

        sync_relic_runtime_state_from_snapshot(
            Some(&previous),
            &mut next,
            -1,
            None,
            Some(false),
            Some(false),
            None,
        );

        assert!(!next.used_up);
    }

    #[test]
    fn sync_relic_runtime_state_reads_pocketwatch_first_turn_protocol_truth() {
        let previous = RelicState::new(RelicId::Pocketwatch);
        let mut next = RelicState::new(RelicId::Pocketwatch);
        next.amount = 0;

        sync_relic_runtime_state_from_snapshot(
            Some(&previous),
            &mut next,
            2,
            None,
            Some(false),
            None,
            Some(1),
        );

        assert_eq!(next.counter, 2);
        assert_eq!(next.amount, 1);
    }

    #[test]
    fn sync_relic_runtime_state_preserves_art_of_war_counter_when_snapshot_counter_is_sentinel() {
        let mut previous = RelicState::new(RelicId::ArtOfWar);
        previous.counter = 1;

        let mut next = RelicState::new(RelicId::ArtOfWar);

        sync_relic_runtime_state_from_snapshot(
            Some(&previous),
            &mut next,
            -1,
            None,
            None,
            None,
            None,
        );

        assert_eq!(next.counter, 1);
    }

    #[test]
    fn snapshot_runtime_counter_reads_art_of_war_protocol_truth() {
        let snapshot = json!({
            "runtime_state": {
                "gain_energy_next": false,
                "first_turn": false
            }
        });

        assert_eq!(
            snapshot_runtime_counter_for_relic(RelicId::ArtOfWar, &snapshot),
            Some(0)
        );
    }

    #[test]
    fn sync_combust_extra_data_from_snapshot_misc() {
        let mut power = Power {
            power_type: PowerId::Combust,
            instance_id: None,
            amount: 10,
            extra_data: 1,
            just_applied: false,
        };

        sync_power_extra_data_from_snapshot_power(
            &mut power,
            &json!({
                "id": "Combust",
                "amount": 10,
                "misc": 2
            }),
        );

        assert_eq!(power.extra_data, 2);
    }

    #[test]
    fn sync_stasis_extra_data_from_snapshot_card_uuid() {
        let mut power = Power {
            power_type: PowerId::Stasis,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            just_applied: false,
        };

        sync_power_extra_data_from_snapshot_power(
            &mut power,
            &json!({
                "id": "Stasis",
                "amount": -1,
                "card": {"id": "Strike_R", "uuid": "stasis-card"}
            }),
        );

        assert_ne!(power.extra_data, 0);
    }

    #[test]
    fn sync_ritual_extra_data_from_snapshot_is_noop() {
        let mut power = Power {
            power_type: PowerId::Ritual,
            instance_id: None,
            amount: 3,
            extra_data: 7,
            just_applied: true,
        };

        sync_power_extra_data_from_snapshot_power(
            &mut power,
            &json!({
                "id": "Ritual",
                "amount": 3,
                "just_applied": true
            }),
        );

        assert_eq!(power.extra_data, 7);
    }
}
