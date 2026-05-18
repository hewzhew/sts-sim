// action_handlers/powers.rs — Power management domain
//
// Handles: ApplyPower, RemovePower, RemoveAllDebuffs, ApplyStasis,
//          UpdatePowerExtraData, GainEnergy,
//          player-turn energy recharge hooks

use crate::content::powers::store;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, PowerPayload};

pub fn apply_player_turn_energy_recharge_hooks(state: &mut CombatState) {
    // Java PlayerTurnEffect first recharges base energy, then calls
    // relic/power onEnergyRecharge hooks before ordinary start-of-turn hooks.
    for power in store::powers_snapshot_for(state, 0) {
        match power.power_type {
            PowerId::Energized => {
                state.turn.adjust_energy(power.amount);
                state.queue_action_back(Action::RemovePower {
                    target: 0,
                    power_id: PowerId::Energized,
                });
            }
            PowerId::DevaForm => {
                let energy_gain = power.extra_data.max(1);
                state.turn.adjust_energy(energy_gain);
                let _ = store::with_power_mut(state, 0, PowerId::DevaForm, |deva| {
                    deva.extra_data += deva.amount;
                });
            }
            PowerId::CollectPower => {
                state.queue_action_back(Action::MakeTempCardInHand {
                    card_id: crate::content::cards::CardId::Miracle,
                    amount: 1,
                    upgraded: true,
                });
                if power.amount <= 1 {
                    state.queue_action_back(Action::RemovePower {
                        target: 0,
                        power_id: PowerId::CollectPower,
                    });
                } else {
                    state.queue_action_back(Action::ReducePower {
                        target: 0,
                        power_id: PowerId::CollectPower,
                        amount: 1,
                    });
                }
            }
            _ => {}
        }
    }
}

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

pub fn handle_spot_weakness(target: usize, amount: i32, state: &mut CombatState) {
    let Some(target_monster) = state.entities.monsters.iter().find(|m| m.id == target) else {
        return;
    };

    if monster_has_java_attack_intent_base_damage(state, target_monster) {
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount,
        });
    }
}

pub fn handle_apply_weak_if_target_attacking(target: usize, amount: i32, state: &mut CombatState) {
    let Some(target_monster) = state.entities.monsters.iter().find(|m| m.id == target) else {
        return;
    };

    if monster_has_java_attack_intent_base_damage(state, target_monster) {
        state.queue_action_front(Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Weak,
            amount,
        });
    }
}

pub fn handle_doppelganger(
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let effect = x_cost_effect(state, upgraded, energy_on_use);
    if effect > 0 {
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Energized,
            amount: effect,
        });
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::DrawCardNextTurn,
            amount: effect,
        });
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_malaise(
    target: usize,
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let effect = x_cost_effect(state, upgraded, energy_on_use);
    if effect > 0 {
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Strength,
            amount: -effect,
        });
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Weak,
            amount: effect,
        });
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_collect(
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let effect = x_cost_effect(state, upgraded, energy_on_use);
    if effect > 0 {
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::CollectPower,
            amount: effect,
        });
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

fn x_cost_effect(state: &CombatState, upgraded: bool, energy_on_use: i32) -> i32 {
    let base_effect = if energy_on_use != -1 {
        energy_on_use
    } else {
        state.turn.energy as i32
    };
    let mut effect = crate::content::relics::hooks::on_calculate_x_cost(state, base_effect);
    if upgraded {
        effect += 1;
    }
    effect
}

fn monster_has_java_attack_intent_base_damage(
    state: &CombatState,
    monster: &crate::runtime::combat::MonsterEntity,
) -> bool {
    let current_plan = crate::content::monsters::resolve_monster_turn_plan(state, monster);
    if current_plan.attack().is_some() {
        return true;
    }

    if !monster.is_dead_or_escaped() {
        return false;
    }

    if monster
        .move_state
        .planned_visible_spec
        .as_ref()
        .and_then(|spec| spec.attack())
        .is_some()
    {
        return true;
    }

    monster
        .move_state
        .planned_steps
        .as_ref()
        .is_some_and(|steps| {
            steps
                .iter()
                .any(|step| matches!(step, crate::semantics::combat::MoveStep::Attack(_)))
        })
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

pub fn handle_trigger_time_warp_end_turn(owner: usize, state: &mut CombatState) {
    let current_amount = store::power_amount(state, owner, PowerId::TimeWarp);
    if current_amount != 0 {
        let _ = store::with_power_mut(state, owner, PowerId::TimeWarp, |power| {
            power.amount = 0;
            power.just_applied = false;
        });
    }

    // Java TimeWarpPower.callEndTurnEarlySequence clears cardQueue but preserves
    // autoplay cards as dontTriggerOnUseCard cleanup actions.
    crate::engine::action_handlers::cards::handle_queue_early_end_turn(state);

    let alive_monster_ids: Vec<usize> = state
        .entities
        .monsters
        .iter()
        .filter(|m| m.is_random_target_candidate())
        .map(|m| m.id)
        .collect();
    for monster_id in alive_monster_ids {
        state.queue_action_back(Action::ApplyPower {
            source: monster_id,
            target: monster_id,
            power_id: PowerId::Strength,
            amount: 2,
        });
    }
}

fn random_alive_monster(state: &mut CombatState) -> Option<usize> {
    let alive: Vec<usize> = state
        .entities
        .monsters
        .iter()
        .filter(|m| m.is_random_target_candidate())
        .map(|m| m.id)
        .collect();
    if alive.is_empty() {
        None
    } else {
        let idx = state.rng.card_random_rng.random(alive.len() as i32 - 1) as usize;
        Some(alive[idx])
    }
}

pub fn handle_bouncing_flask(
    target: Option<usize>,
    amount: i32,
    num_times: u8,
    state: &mut CombatState,
) {
    let Some(target_id) = target.or_else(|| random_alive_monster(state)) else {
        return;
    };

    if state.are_monsters_basically_dead_java() {
        state.clear_post_combat_actions();
        return;
    }

    if num_times > 1 {
        let next_target = random_alive_monster(state);
        state.queue_action_front(Action::BouncingFlask {
            target: next_target,
            amount,
            num_times: num_times - 1,
        });
    }

    if state
        .entities
        .monsters
        .iter()
        .any(|m| m.id == target_id && m.is_alive_for_action())
    {
        state.queue_action_front(Action::ApplyPower {
            source: 0,
            target: target_id,
            power_id: PowerId::Poison,
            amount,
        });
    }
}

pub fn handle_remove_power(target: usize, power_id: PowerId, state: &mut CombatState) {
    let had_power = store::powers_for(state, target)
        .is_some_and(|powers| powers.iter().any(|p| p.power_type == power_id));
    if !had_power {
        return;
    }

    let on_remove_actions =
        crate::content::powers::resolve_power_on_remove(power_id, state, target);
    for action in on_remove_actions {
        state.queue_action_back(action);
    }

    store::remove_power_type(state, target, power_id);
}

pub fn handle_remove_power_instance(
    target: usize,
    power_id: PowerId,
    instance_id: u32,
    state: &mut CombatState,
) {
    let power_snapshot = store::powers_for(state, target).and_then(|powers| {
        powers
            .iter()
            .find(|p| p.power_type == power_id && p.instance_id == Some(instance_id))
            .cloned()
    });
    let Some(power_snapshot) = power_snapshot else {
        return;
    };

    let on_remove_actions =
        crate::content::powers::resolve_power_on_remove(power_snapshot.power_type, state, target);
    for action in on_remove_actions {
        state.queue_action_back(action);
    }

    store::remove_power_instance(state, target, power_id, instance_id);
}

pub fn handle_reduce_power(target: usize, power_id: PowerId, amount: i32, state: &mut CombatState) {
    if amount <= 0 {
        return;
    }

    let Some(remaining) = store::with_power_mut(state, target, power_id, |power| {
        power.amount -= amount;
        power.amount
    }) else {
        return;
    };

    if !crate::content::powers::should_keep_power_instance(power_id, remaining) {
        handle_remove_power(target, power_id, state);
    }

    if target == 0 {
        state.recompute_turn_start_draw_modifier();
    }
}

pub fn handle_reduce_power_instance(
    target: usize,
    power_id: PowerId,
    instance_id: u32,
    amount: i32,
    state: &mut CombatState,
) {
    if amount <= 0 {
        return;
    }

    let Some(remaining) =
        store::with_power_instance_mut(state, target, power_id, instance_id, |power| {
            power.amount -= amount;
            power.amount
        })
    else {
        return;
    };

    if !crate::content::powers::should_keep_power_instance(power_id, remaining) {
        handle_remove_power_instance(target, power_id, instance_id, state);
    }

    if target == 0 {
        state.recompute_turn_start_draw_modifier();
    }
}

pub fn handle_remove_all_debuffs(target: usize, state: &mut CombatState) {
    let debuffs = store::powers_for(state, target)
        .map(|powers| {
            powers
                .iter()
                .filter(|p| crate::content::powers::is_debuff(p.power_type, p.amount))
                .map(|p| p.power_type)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    for power_id in debuffs {
        state.queue_action_front(Action::RemovePower { target, power_id });
    }
}

pub fn handle_apply_stasis(target_id: usize, state: &mut CombatState) {
    if state.zones.draw_pile.is_empty() && state.zones.discard_pile.is_empty() {
        return;
    }

    let source_pile_draw = !state.zones.draw_pile.is_empty();
    let source_pile = if source_pile_draw {
        &state.zones.draw_pile
    } else {
        &state.zones.discard_pile
    };

    let rarities_to_check = [
        crate::content::cards::CardRarity::Rare,
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Common,
    ];

    let mut candidates = Vec::new();
    for expected_rarity in rarities_to_check {
        for (i, card) in source_pile.iter().enumerate() {
            let def = crate::content::cards::get_card_definition(card.id);
            if def.rarity == expected_rarity {
                candidates.push(i);
            }
        }
        if !candidates.is_empty() {
            candidates.sort_by(|left, right| {
                crate::content::cards::java_id(source_pile[*left].id)
                    .cmp(crate::content::cards::java_id(source_pile[*right].id))
            });
            break;
        }
    }

    if candidates.is_empty() {
        for i in 0..source_pile.len() {
            candidates.push(i);
        }
    }

    let pick_idx = if candidates.len() > 1 {
        let r = state
            .rng
            .card_random_rng
            .random(candidates.len() as i32 - 1) as usize;
        candidates[r]
    } else {
        candidates[0]
    };

    let card = if source_pile_draw {
        state.zones.draw_pile.remove(pick_idx)
    } else {
        state.zones.discard_pile.remove(pick_idx)
    };

    let uuid = card.uuid as i32;
    state.zones.limbo.push(card);

    state.queue_action_front(Action::UpdatePowerExtraData {
        target: target_id,
        power_id: PowerId::Stasis,
        value: uuid,
    });
    state.queue_action_front(Action::ApplyPower {
        source: target_id,
        target: target_id,
        power_id: PowerId::Stasis,
        amount: -1,
    });
}

pub fn handle_update_power_extra_data(
    target: usize,
    power_id: PowerId,
    value: i32,
    state: &mut CombatState,
) {
    let _ = store::with_power_mut(state, target, power_id, |power| {
        power.extra_data = value;
    });
}

pub fn handle_update_power_extra_data_instance(
    target: usize,
    power_id: PowerId,
    instance_id: u32,
    value: i32,
    state: &mut CombatState,
) {
    let _ = store::with_power_instance_mut(state, target, power_id, instance_id, |power| {
        power.extra_data = value;
    });
}

pub fn handle_gain_energy(amount: i32, state: &mut CombatState) {
    state.turn.adjust_energy(amount);
}

pub fn handle_double_energy(state: &mut CombatState) {
    let current_energy = state.turn.energy as i32;
    if current_energy > 0 {
        state.turn.adjust_energy(current_energy);
    }
}

pub fn handle_gain_max_hp(amount: i32, state: &mut CombatState) {
    state.entities.player.max_hp += amount;
    state.entities.player.current_hp =
        (state.entities.player.current_hp + amount).min(state.entities.player.max_hp);
}

pub fn handle_lose_max_hp(target: usize, amount: i32, state: &mut CombatState) {
    if target == 0 {
        state.entities.player.max_hp = (state.entities.player.max_hp - amount).max(1);
        state.entities.player.current_hp = state
            .entities
            .player
            .current_hp
            .min(state.entities.player.max_hp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::{CombatCard, Power, QueuedCardPlay, QueuedCardSource};
    use crate::semantics::combat::{AttackSpec, DamageKind, MonsterMoveSpec};
    use crate::test_support::blank_test_combat;

    #[test]
    fn player_turn_energy_recharge_applies_energized_immediately() {
        let mut state = blank_test_combat();
        state.turn.energy = 0;
        state.entities.player.energy_master = 3;
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Energized,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );

        state.begin_next_player_turn();
        apply_player_turn_energy_recharge_hooks(&mut state);

        assert_eq!(state.turn.energy, 5);
        assert_eq!(
            state.pop_next_action(),
            Some(Action::RemovePower {
                target: 0,
                power_id: PowerId::Energized,
            })
        );
        assert_eq!(state.pop_next_action(), None);
    }

    #[test]
    fn time_warp_end_turn_preserves_autoplay_cleanup_like_java_call_end_turn_early_sequence() {
        let mut state = blank_test_combat();
        let mut time_eater = crate::test_support::test_monster(EnemyId::TimeEater);
        time_eater.id = 7;
        state.entities.monsters = vec![time_eater];
        store::set_powers_for(
            &mut state,
            7,
            vec![Power {
                power_type: PowerId::TimeWarp,
                instance_id: None,
                amount: 12,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        state.zones.queued_cards = std::collections::VecDeque::from([
            QueuedCardPlay {
                card: CombatCard::new(crate::content::cards::CardId::Strike, 900),
                target: Some(7),
                energy_on_use: 0,
                ignore_energy_total: true,
                autoplay: false,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: QueuedCardSource::Normal,
            },
            QueuedCardPlay {
                card: CombatCard::new(crate::content::cards::CardId::Defend, 901),
                target: None,
                energy_on_use: 0,
                ignore_energy_total: true,
                autoplay: true,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: QueuedCardSource::Normal,
            },
        ]);

        handle_trigger_time_warp_end_turn(7, &mut state);

        assert_eq!(store::power_amount(&state, 7, PowerId::TimeWarp), 0);
        assert!(state.zones.queued_cards.is_empty());
        assert_eq!(
            state
                .zones
                .limbo
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![901],
            "Java callEndTurnEarlySequence keeps autoplay queued cards for dontTriggerOnUseCard cleanup"
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::UseCardDone {
                should_exhaust: false,
                trigger_after_use_hooks: false,
            })
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::ApplyPower {
                source: 7,
                target: 7,
                power_id: PowerId::Strength,
                amount: 2,
            })
        );
    }

    #[test]
    fn player_turn_energy_recharge_without_hooks_keeps_base_energy() {
        let mut state = blank_test_combat();
        state.turn.energy = 0;
        state.entities.player.energy_master = 3;

        state.begin_next_player_turn();
        apply_player_turn_energy_recharge_hooks(&mut state);

        assert_eq!(state.turn.energy, 3);
        assert_eq!(state.pop_next_action(), None);
    }

    #[test]
    fn energized_is_not_an_ordinary_at_turn_start_power() {
        let mut state = blank_test_combat();

        let actions = crate::content::powers::resolve_power_at_turn_start(
            PowerId::Energized,
            &mut state,
            0,
            2,
        );

        assert!(
            actions.is_empty(),
            "Energized belongs to Java onEnergyRecharge, not applyStartOfTurnPowers"
        );
    }

    #[test]
    fn stacking_existing_mantra_to_ten_queues_divinity_and_consumes_ten_mantra() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Mantra,
                instance_id: None,
                amount: 7,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_apply_power(0, 0, PowerId::Mantra, 3, &mut state);

        assert_eq!(store::power_amount(&state, 0, PowerId::Mantra), 0);
        assert_eq!(
            state.pop_next_action(),
            Some(Action::EnterStance("Divinity".to_string()))
        );
    }

    #[test]
    fn stacking_existing_mantra_over_ten_keeps_remainder_like_java() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Mantra,
                instance_id: None,
                amount: 8,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_apply_power(0, 0, PowerId::Mantra, 5, &mut state);

        assert_eq!(store::power_amount(&state, 0, PowerId::Mantra), 3);
        assert_eq!(
            state.pop_next_action(),
            Some(Action::EnterStance("Divinity".to_string()))
        );
    }

    #[test]
    fn apply_stasis_sorts_rarity_candidates_by_java_card_id_before_rng_pick() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(crate::content::cards::CardId::Catalyst, 101),
            CombatCard::new(crate::content::cards::CardId::Alchemize, 102),
        ];
        let mut expected_rng = state.rng.card_random_rng.clone();
        let mut expected_order = state
            .zones
            .draw_pile
            .iter()
            .map(|card| (crate::content::cards::java_id(card.id), card.uuid))
            .collect::<Vec<_>>();
        expected_order.sort_by(|left, right| left.0.cmp(right.0));
        let expected_uuid =
            expected_order[expected_rng.random(expected_order.len() as i32 - 1) as usize].1;

        handle_apply_stasis(7, &mut state);

        assert_eq!(
            state.zones.limbo.iter().map(|card| card.uuid).collect::<Vec<_>>(),
            vec![expected_uuid],
            "Java CardGroup.getRandomCard(rng, rarity) sorts matching cards by cardID before consuming the random index"
        );
    }

    #[test]
    fn sentinel_power_reapplication_matches_java_apply_power_special_cases() {
        let mut no_draw_state = blank_test_combat();
        handle_apply_power(0, 0, PowerId::NoDraw, 1, &mut no_draw_state);
        handle_apply_power(0, 0, PowerId::NoDraw, 1, &mut no_draw_state);
        assert_eq!(
            store::power_amount(&no_draw_state, 0, PowerId::NoDraw),
            -1,
            "Java ApplyPowerAction explicitly no-ops duplicate NoDrawPower applications"
        );

        let mut master_reality_state = blank_test_combat();
        handle_apply_power(
            0,
            0,
            PowerId::MasterRealityPower,
            -1,
            &mut master_reality_state,
        );
        handle_apply_power(
            0,
            0,
            PowerId::MasterRealityPower,
            -1,
            &mut master_reality_state,
        );
        assert_eq!(
            store::power_amount(&master_reality_state, 0, PowerId::MasterRealityPower),
            -2,
            "Java MasterRealityPower has AbstractPower.amount == -1 and duplicate ApplyPowerAction uses default stackPower(-1)"
        );

        let mut shifting_state = blank_test_combat();
        handle_apply_power(0, 0, PowerId::Shifting, -1, &mut shifting_state);
        handle_apply_power(0, 0, PowerId::Shifting, -1, &mut shifting_state);
        assert_eq!(
            store::power_amount(&shifting_state, 0, PowerId::Shifting),
            -2,
            "Java ShiftingPower inherits AbstractPower.amount == -1 and duplicate ApplyPowerAction uses default stackPower(-1)"
        );
    }

    #[test]
    fn apply_power_ignores_half_dead_monsters_like_java_is_dead_or_escaped() {
        let mut state = blank_test_combat();
        let mut half_dead = crate::test_support::test_monster(EnemyId::Darkling);
        half_dead.id = 44;
        half_dead.current_hp = 12;
        half_dead.half_dead = true;
        state.entities.monsters = vec![half_dead];

        handle_apply_power(0, 44, PowerId::Weak, 2, &mut state);

        assert!(
            store::powers_snapshot_for(&state, 44).is_empty(),
            "Java ApplyPowerAction returns before applying powers when target.isDeadOrEscaped(), and halfDead is part of that predicate"
        );
    }

    #[test]
    fn apply_power_does_not_skip_zero_hp_target_unless_dead_or_escaped() {
        let mut state = blank_test_combat();
        let mut target = crate::test_support::test_monster(EnemyId::JawWorm);
        target.id = 47;
        target.current_hp = 0;
        target.is_dying = false;
        target.half_dead = false;
        target.is_escaped = false;
        state.entities.monsters = vec![target];

        handle_apply_power(0, 47, PowerId::Weak, 2, &mut state);

        assert_eq!(
            store::power_amount(&state, 47, PowerId::Weak),
            2,
            "Java ApplyPowerAction checks isDeadOrEscaped(), not currentHealth <= 0"
        );
    }

    #[test]
    fn apply_invincible_stores_turn_reset_amount_like_java_max_amt() {
        let mut state = blank_test_combat();
        let mut target = crate::test_support::test_monster(EnemyId::CorruptHeart);
        target.id = 48;
        state.entities.monsters = vec![target];

        handle_apply_power(48, 48, PowerId::Invincible, 200, &mut state);

        let power = store::powers_snapshot_for(&state, 48)
            .into_iter()
            .find(|p| p.power_type == PowerId::Invincible)
            .expect("Invincible should be applied");
        assert_eq!(power.amount, 200);
        assert_eq!(
            power.extra_data, 200,
            "Java InvinciblePower keeps maxAmt for start-of-turn reset; Rust stores that in extra_data"
        );
    }

    #[test]
    fn bouncing_flask_random_target_ignores_half_dead_monsters_like_java_random_monster() {
        let mut state = blank_test_combat();
        let mut half_dead = crate::test_support::test_monster(EnemyId::Darkling);
        half_dead.id = 45;
        half_dead.current_hp = 12;
        half_dead.half_dead = true;
        state.entities.monsters = vec![half_dead];

        handle_bouncing_flask(None, 3, 1, &mut state);

        assert_eq!(
            state.pop_next_action(),
            None,
            "Java BouncingFlaskAction chooses targets via getRandomMonster(aliveOnly=true), which excludes halfDead monsters"
        );
    }

    #[test]
    fn bouncing_flask_clears_post_combat_actions_if_monsters_are_basically_dead() {
        let mut state = blank_test_combat();
        let mut dying = crate::test_support::test_monster(EnemyId::JawWorm);
        dying.id = 48;
        dying.current_hp = 0;
        dying.is_dying = true;
        state.entities.monsters = vec![dying];
        state.queue_action_back(Action::DrawCards(1));

        handle_bouncing_flask(Some(48), 3, 2, &mut state);

        assert_eq!(
            state.pop_next_action(),
            None,
            "Java BouncingFlaskAction clears non-retained post-combat actions when all monsters are basically dead"
        );
    }

    #[test]
    fn spot_weakness_reads_raw_intent_base_damage_even_if_target_is_dying() {
        let attack = MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 0,
            hits: 1,
            damage_kind: DamageKind::Normal,
        });
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = 46;
        monster.current_hp = 0;
        monster.is_dying = true;
        monster.move_state.planned_steps = Some(attack.to_steps());
        monster.move_state.planned_visible_spec = Some(attack);

        let mut state = blank_test_combat();
        state.entities.monsters = vec![monster];

        handle_spot_weakness(46, 3, &mut state);

        assert_eq!(
            state.pop_next_action(),
            Some(Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::Strength,
                amount: 3,
            }),
            "Java SpotWeaknessAction only checks targetMonster.getIntentBaseDmg() >= 0; it does not cancel just because the target object is already dying"
        );
    }
}
