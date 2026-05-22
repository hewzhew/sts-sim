use crate::content::powers::{store, PowerId};
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, PowerPayload};
pub fn handle_apply_power(
    source: usize,
    target: usize,
    power_id: PowerId,
    amount: i32,
    state: &mut CombatState,
) {
    handle_apply_power_detailed(source, target, power_id, amount, None, None, state);
}

pub fn handle_apply_power_with_payload(
    source: usize,
    target: usize,
    power_id: PowerId,
    amount: i32,
    instance_id: Option<u32>,
    extra_data: Option<i32>,
    payload: PowerPayload,
    state: &mut CombatState,
) {
    handle_apply_power_detailed_internal(
        source,
        target,
        power_id,
        amount,
        instance_id,
        extra_data,
        payload,
        state,
    );
}

pub fn handle_apply_power_detailed(
    source: usize,
    target: usize,
    power_id: PowerId,
    amount: i32,
    instance_id: Option<u32>,
    extra_data: Option<i32>,
    state: &mut CombatState,
) {
    handle_apply_power_detailed_internal(
        source,
        target,
        power_id,
        amount,
        instance_id,
        extra_data,
        PowerPayload::None,
        state,
    );
}

fn handle_apply_power_detailed_internal(
    source: usize,
    target: usize,
    power_id: PowerId,
    mut amount: i32,
    instance_id: Option<u32>,
    extra_data: Option<i32>,
    payload: PowerPayload,
    state: &mut CombatState,
) {
    amount = crate::content::powers::canonicalize_applied_amount(power_id, amount);

    // C1: Snake Skull → +1 Poison
    if amount > 0
        && power_id == PowerId::Poison
        && state
            .entities
            .player
            .has_relic(crate::content::relics::RelicId::SneckoSkull)
    {
        if source == 0 && target != 0 {
            amount += 1;
        }
    }

    // U1: Java ApplyPowerAction.update(): target.isDeadOrEscaped() returns true
    // for dying, escaped, and half-dead monsters. It does not check currentHealth.
    if target == 0 {
        // Player target — always valid
    } else if let Some(m) = state.entities.monsters.iter().find(|m| m.id == target) {
        if m.is_dead_or_escaped() {
            return;
        }
    }

    // U3: onApplyPower hooks (Sadistic Nature)
    if source != 0 || target != 0 {
        if let Some(source_powers) = store::powers_for(state, source).map(|powers| powers.to_vec())
        {
            for power in &source_powers {
                let hook_actions = crate::content::powers::resolve_power_on_apply_power(
                    power.power_type,
                    power.amount,
                    power_id,
                    amount,
                    target,
                    source,
                    state,
                );
                let hook_actions_4: smallvec::SmallVec<[crate::runtime::action::ActionInfo; 4]> =
                    hook_actions.into_iter().collect();
                state.queue_actions(hook_actions_4);
            }
        }
    }

    // U4: Champion Belt
    let champion_belt_actions =
        crate::content::relics::hooks::on_apply_power(state, source, power_id, target);
    state.queue_actions(champion_belt_actions);

    // U5: Monster re-check after hooks
    if target != 0 {
        if let Some(m) = state.entities.monsters.iter().find(|m| m.id == target) {
            if m.is_dead_or_escaped() {
                return;
            }
        }
    }

    // U6+U7: Ginger/Turnip
    if target == 0 {
        amount = crate::content::relics::hooks::on_receive_power_modify(state, power_id, amount);
        if amount == 0 && crate::content::powers::is_debuff(power_id, amount) {
            return;
        }
    }

    // U8: Artifact blocks actual debuff applications, not debuff cleanup like Weak -1.
    if crate::content::powers::is_debuff_application(power_id, amount) {
        let has_artifact = store::powers_for(state, target)
            .is_some_and(|powers| powers.iter().any(|p| p.power_type == PowerId::Artifact));
        if has_artifact {
            if let Some(amount) = store::with_power_mut(state, target, PowerId::Artifact, |art| {
                art.amount -= 1;
                art.amount
            }) {
                if amount <= 0 {
                    store::remove_power_type(state, target, PowerId::Artifact);
                }
            }
            return;
        }
    }

    // Java ApplyPowerAction: NoDrawPower is the duplicate-application special
    // case. Other sentinel amount powers still flow through stackPower(-1).
    if crate::content::powers::reapplying_existing_power_is_noop(power_id)
        && store::powers_for(state, target)
            .is_some_and(|powers| powers.iter().any(|p| p.power_type == power_id))
    {
        return;
    }

    let had_existing_power = store::has_power(state, target, power_id);
    if power_id == PowerId::Mantra && target == 0 {
        state.turn.counters.mantra_gained_this_combat += amount;
    }

    // Core power application
    let powers = store::ensure_powers_for_mut(state, target);
    let mut should_remove_existing = false;
    if crate::content::powers::uses_distinct_instances(power_id) {
        if let Some(instance_id) = instance_id {
            if let Some(existing) = powers
                .iter_mut()
                .find(|p| p.power_type == power_id && p.instance_id == Some(instance_id))
            {
                existing.amount += amount;
                if let Some(extra_data) = extra_data {
                    existing.extra_data = extra_data;
                }
                if !matches!(payload, PowerPayload::None) {
                    existing.payload = payload;
                }
                if !crate::content::powers::should_keep_power_instance(power_id, existing.amount) {
                    should_remove_existing = true;
                }
            } else if crate::content::powers::should_keep_power_instance(power_id, amount) {
                powers.push(crate::runtime::combat::Power {
                    power_type: power_id,
                    instance_id: Some(instance_id),
                    amount,
                    extra_data: extra_data.unwrap_or(0),
                    payload,
                    just_applied: true,
                });
            }
        } else if let Some(existing) = powers.iter_mut().find(|p| p.power_type == power_id) {
            existing.amount += amount;
            if let Some(extra_data) = extra_data {
                existing.extra_data = extra_data;
            }
            if !matches!(payload, PowerPayload::None) {
                existing.payload = payload;
            }
            if !crate::content::powers::should_keep_power_instance(power_id, existing.amount) {
                should_remove_existing = true;
            }
        } else if crate::content::powers::should_keep_power_instance(power_id, amount) {
            powers.push(crate::runtime::combat::Power {
                power_type: power_id,
                instance_id: None,
                amount,
                extra_data: extra_data.unwrap_or(0),
                payload,
                just_applied: true,
            });
        }
    } else if let Some(existing) = powers.iter_mut().find(|p| p.power_type == power_id) {
        if power_id == PowerId::Combust {
            existing.amount += amount;
            existing.extra_data += 1;
        } else if power_id == PowerId::PanachePower && amount > 0 {
            // Panache has two distinct positive-amount paths:
            //   1. card application / stacking: amount is damage (10/14), damage stacks
            //   2. countdown reset: internal ApplyPower(+1..+4), amount should reset countdown
            if amount <= 4 && existing.amount < 5 {
                existing.amount = (existing.amount + amount).min(5);
            } else {
                existing.extra_data += amount;
            }
        } else if power_id == PowerId::Malleable && amount > 0 {
            existing.amount += amount;
            existing.extra_data += amount;
        } else if power_id == PowerId::Flight && amount > 0 {
            existing.amount += amount;
            existing.extra_data = existing.amount.max(existing.extra_data);
        } else if power_id == PowerId::LikeWaterPower && amount > 0 {
            existing.amount = (existing.amount + amount).min(999);
        } else if power_id == PowerId::DevaForm && amount > 0 {
            existing.amount += amount;
            existing.extra_data += 1;
        } else if power_id == PowerId::CollectPower && amount > 0 {
            existing.amount = (existing.amount + amount).min(999);
        } else {
            existing.amount += amount;
        }

        // Strength/Dexterity/Focus can go negative, but Java still removes them at exactly 0.
        if !crate::content::powers::should_keep_power_instance(power_id, existing.amount) {
            should_remove_existing = true;
        }
    } else {
        // If they don't have it, we only add it if amount > 0, OR if it's a negative amount of a power that CAN go negative
        if crate::content::powers::should_keep_power_instance(power_id, amount) {
            let (stored_amount, extra_data) = match power_id {
                PowerId::PanachePower => (5, amount),
                PowerId::Combust => (amount, 1),
                PowerId::Malleable => (amount, amount),
                PowerId::Flight => (amount, amount),
                PowerId::DevaForm => (amount, 1),
                PowerId::CollectPower => (amount.min(999), 0),
                PowerId::Invincible => (amount, amount),
                PowerId::Rebound => (amount, extra_data.unwrap_or(1)),
                _ if extra_data.is_some() => (amount, extra_data.unwrap_or(0)),
                _ => (amount, 0),
            };
            powers.push(crate::runtime::combat::Power {
                power_type: power_id,
                instance_id: None,
                amount: stored_amount,
                extra_data,
                payload,
                just_applied: true,
            });
        }
    }
    if should_remove_existing {
        if let Some(instance_id) = instance_id {
            store::remove_power_instance(state, target, power_id, instance_id);
        } else {
            store::remove_power_type(state, target, power_id);
        }
    } else {
        store::sort_powers_for_java(state, target);
    }

    // C2: Corruption on-apply hook
    if power_id == PowerId::Corruption {
        crate::content::cards::ironclad::corruption::corruption_on_apply(state);
    }
    if power_id == PowerId::Surrounded && target == 0 {
        crate::content::powers::core::surrounded::sync_back_attack_markers(state);
    }

    // Java MantraPower.stackPower(): only stacking an existing Mantra power
    // checks for the 10-point threshold, then subtracts 10 and enters Divinity.
    if power_id == PowerId::Mantra && target == 0 && had_existing_power {
        let current = store::power_amount(state, target, PowerId::Mantra);
        if current >= 10 {
            let remainder = current - 10;
            if remainder > 0 {
                let _ = store::with_power_mut(state, target, PowerId::Mantra, |power| {
                    power.amount = remainder;
                });
            } else {
                store::remove_power_type(state, target, PowerId::Mantra);
            }
            state.queue_action_front(Action::EnterStance("Divinity".to_string()));
        }
    }

    if target == 0 {
        state.recompute_turn_start_draw_modifier();
    }
}
