// action_handlers/powers.rs — Power management domain
//
// Handles: ApplyPower, RemovePower, RemoveAllDebuffs, ApplyStasis,
//          UpdatePowerExtraData, AwakenedRebirthClear, GainEnergy

use crate::action::Action;
use crate::combat::CombatState;
use crate::content::powers::store;
use crate::content::powers::PowerId;

pub fn handle_apply_power(
    source: usize,
    target: usize,
    power_id: PowerId,
    amount: i32,
    state: &mut CombatState,
) {
    handle_apply_power_detailed(source, target, power_id, amount, None, None, state);
}

pub fn handle_apply_power_detailed(
    source: usize,
    target: usize,
    power_id: PowerId,
    mut amount: i32,
    instance_id: Option<u32>,
    extra_data: Option<i32>,
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

    // U1: Dead/Escaped target guard
    if target == 0 {
        // Player target — always valid
    } else if let Some(m) = state.entities.monsters.iter().find(|m| m.id == target) {
        if m.is_dying || m.current_hp <= 0 {
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
                let hook_actions_4: smallvec::SmallVec<[crate::action::ActionInfo; 4]> =
                    hook_actions.into_iter().collect();
                crate::engine::core::queue_actions(&mut state.engine.action_queue, hook_actions_4);
            }
        }
    }

    // U4: Champion Belt
    let champion_belt_actions =
        crate::content::relics::hooks::on_apply_power(state, power_id, target);
    crate::engine::core::queue_actions(&mut state.engine.action_queue, champion_belt_actions);

    // U5: Monster re-check after hooks
    if target != 0 {
        if let Some(m) = state.entities.monsters.iter().find(|m| m.id == target) {
            if m.is_dying || m.current_hp <= 0 {
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

    // Java ApplyPowerAction: No Draw does not stack and reapplying it is a no-op.
    if crate::content::powers::uses_sentinel_amount(power_id)
        && store::powers_for(state, target)
            .is_some_and(|powers| powers.iter().any(|p| p.power_type == power_id))
    {
        return;
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
                if !crate::content::powers::should_keep_power_instance(power_id, existing.amount) {
                    should_remove_existing = true;
                }
            } else if crate::content::powers::should_keep_power_instance(power_id, amount) {
                powers.push(crate::combat::Power {
                    power_type: power_id,
                    instance_id: Some(instance_id),
                    amount,
                    extra_data: extra_data.unwrap_or(0),
                    just_applied: true,
                });
            }
        } else if let Some(existing) = powers.iter_mut().find(|p| p.power_type == power_id) {
            existing.amount += amount;
            if let Some(extra_data) = extra_data {
                existing.extra_data = extra_data;
            }
            if !crate::content::powers::should_keep_power_instance(power_id, existing.amount) {
                should_remove_existing = true;
            }
        } else if crate::content::powers::should_keep_power_instance(power_id, amount) {
            powers.push(crate::combat::Power {
                power_type: power_id,
                instance_id: None,
                amount,
                extra_data: extra_data.unwrap_or(0),
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
                _ if extra_data.is_some() => (amount, extra_data.unwrap_or(0)),
                _ => (amount, 0),
            };
            powers.push(crate::combat::Power {
                power_type: power_id,
                instance_id: None,
                amount: stored_amount,
                extra_data,
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
    }

    // C2: Corruption on-apply hook
    if power_id == PowerId::Corruption {
        crate::content::cards::ironclad::corruption::corruption_on_apply(state);
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

    // Java TimeWarpPower.callEndTurnEarlySequence:
    // clear queued autoplay cards, then end the turn once the current card fully resolves.
    crate::engine::action_handlers::cards::handle_clear_card_queue(state);
    crate::engine::action_handlers::cards::handle_queue_early_end_turn(state);

    let alive_monster_ids: Vec<usize> = state
        .entities
        .monsters
        .iter()
        .filter(|m| m.current_hp > 0 && !m.is_dying && !m.is_escaped)
        .map(|m| m.id)
        .collect();
    for monster_id in alive_monster_ids {
        state.engine.action_queue.push_back(Action::ApplyPower {
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
        .filter(|m| m.current_hp > 0 && !m.is_dying && !m.is_escaped)
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

    if num_times > 1 {
        let next_target = random_alive_monster(state);
        state.engine.action_queue.push_front(Action::BouncingFlask {
            target: next_target,
            amount,
            num_times: num_times - 1,
        });
    }

    if state
        .entities
        .monsters
        .iter()
        .any(|m| m.id == target_id && m.current_hp > 0 && !m.is_dying && !m.is_escaped)
    {
        state.engine.action_queue.push_front(Action::ApplyPower {
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
        state.engine.action_queue.push_back(action);
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
        state.engine.action_queue.push_back(action);
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
        state
            .engine
            .action_queue
            .push_front(Action::RemovePower { target, power_id });
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

    state
        .engine
        .action_queue
        .push_front(Action::UpdatePowerExtraData {
            target: target_id,
            power_id: PowerId::Stasis,
            value: uuid,
        });
    state.engine.action_queue.push_front(Action::ApplyPower {
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

pub fn handle_awakened_rebirth_clear(target: usize, state: &mut CombatState) {
    store::retain_entity_powers(state, target, |p| {
        p.power_type != PowerId::Curiosity
            && p.power_type != PowerId::Unawakened
            && p.power_type != PowerId::Shackled
            && !crate::content::powers::is_debuff(p.power_type, p.amount)
    });
}

pub fn handle_gain_energy(amount: i32, state: &mut CombatState) {
    state.turn.energy = (state.turn.energy as i32 + amount).max(0) as u8;
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
    use crate::action::Action;
    use crate::combat::Intent;
    use crate::content::monsters::EnemyId;
    use crate::engine::test_support::{basic_combat, CombatTestExt};

    #[test]
    fn reduce_power_subtracts_without_removing_when_amount_remains() {
        let mut combat = basic_combat()
            .with_monster_type(1, EnemyId::Lagavulin)
            .with_monster_max_hp(1, 109)
            .with_monster_hp(1, 109)
            .with_monster_intent(1, Intent::Sleep);
        handle_apply_power(1, 1, PowerId::Metallicize, 12, &mut combat);

        handle_reduce_power(1, PowerId::Metallicize, 8, &mut combat);

        assert_eq!(store::power_amount(&combat, 1, PowerId::Metallicize), 4);
    }

    #[test]
    fn panache_stacking_increases_damage_without_touching_countdown() {
        let mut combat = basic_combat()
            .with_monster_type(1, EnemyId::Lagavulin)
            .with_monster_max_hp(1, 109)
            .with_monster_hp(1, 109)
            .with_monster_intent(1, Intent::Sleep);

        handle_apply_power(0, 0, PowerId::PanachePower, 10, &mut combat);
        handle_apply_power(0, 0, PowerId::PanachePower, 14, &mut combat);

        let panache = store::powers_for(&combat, 0)
            .and_then(|powers| {
                powers
                    .iter()
                    .find(|p| p.power_type == PowerId::PanachePower)
            })
            .expect("panache should exist");
        assert_eq!(panache.amount, 5);
        assert_eq!(panache.extra_data, 24);
    }

    #[test]
    fn panache_turn_start_resets_countdown_without_changing_damage() {
        let combat = basic_combat()
            .with_monster_type(1, EnemyId::Lagavulin)
            .with_monster_max_hp(1, 109)
            .with_monster_hp(1, 109)
            .with_monster_intent(1, Intent::Sleep);

        let acts = crate::content::powers::resolve_power_at_turn_start(
            PowerId::PanachePower,
            &combat,
            0,
            2,
        );

        assert_eq!(acts.len(), 1);
        assert!(matches!(
            acts[0],
            Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::PanachePower,
                amount: 3,
            }
        ));
    }

    #[test]
    fn panache_positive_reset_amount_restores_countdown_not_damage() {
        let mut combat = basic_combat()
            .with_monster_type(1, EnemyId::Lagavulin)
            .with_monster_max_hp(1, 109)
            .with_monster_hp(1, 109)
            .with_monster_intent(1, Intent::Sleep);

        handle_apply_power(0, 0, PowerId::PanachePower, 10, &mut combat);
        let _ = store::with_power_mut(&mut combat, 0, PowerId::PanachePower, |panache| {
            panache.amount = 1;
        });

        handle_apply_power(0, 0, PowerId::PanachePower, 4, &mut combat);

        let panache = store::powers_for(&combat, 0)
            .and_then(|powers| {
                powers
                    .iter()
                    .find(|p| p.power_type == PowerId::PanachePower)
            })
            .expect("panache should exist");
        assert_eq!(panache.amount, 5);
        assert_eq!(panache.extra_data, 10);
    }

    #[test]
    fn the_bomb_supports_multiple_runtime_instances() {
        let mut combat = basic_combat();

        handle_apply_power_detailed(
            0,
            0,
            PowerId::TheBombPower,
            3,
            Some(101),
            Some(40),
            &mut combat,
        );
        handle_apply_power_detailed(
            0,
            0,
            PowerId::TheBombPower,
            3,
            Some(202),
            Some(50),
            &mut combat,
        );

        let bombs = store::powers_for(&combat, 0)
            .expect("player powers should exist")
            .iter()
            .filter(|power| power.power_type == PowerId::TheBombPower)
            .collect::<Vec<_>>();
        assert_eq!(bombs.len(), 2);
        assert!(bombs
            .iter()
            .any(|power| power.instance_id == Some(101) && power.extra_data == 40));
        assert!(bombs
            .iter()
            .any(|power| power.instance_id == Some(202) && power.extra_data == 50));
    }

    #[test]
    fn the_bomb_end_of_turn_ticks_specific_instance_and_uses_its_damage() {
        let mut combat = basic_combat();
        combat.entities.monsters[0].current_hp = 60;

        handle_apply_power_detailed(
            0,
            0,
            PowerId::TheBombPower,
            1,
            Some(101),
            Some(40),
            &mut combat,
        );
        handle_apply_power_detailed(
            0,
            0,
            PowerId::TheBombPower,
            3,
            Some(202),
            Some(50),
            &mut combat,
        );

        let bomb = store::powers_for(&combat, 0)
            .expect("player powers should exist")
            .iter()
            .find(|power| {
                power.power_type == PowerId::TheBombPower && power.instance_id == Some(101)
            })
            .cloned()
            .expect("bomb instance 101 should exist");

        let actions = crate::content::powers::resolve_power_at_end_of_turn(&bomb, &combat, 0);
        for action in actions {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }
        while let Some(action) = combat.engine.action_queue.pop_front() {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        assert_eq!(combat.entities.monsters[0].current_hp, 20);
        let bombs = store::powers_for(&combat, 0)
            .expect("player powers should remain")
            .iter()
            .filter(|power| power.power_type == PowerId::TheBombPower)
            .collect::<Vec<_>>();
        assert_eq!(bombs.len(), 1);
        assert_eq!(bombs[0].instance_id, Some(202));
        assert_eq!(bombs[0].amount, 3);
        assert_eq!(bombs[0].extra_data, 50);
    }

    #[test]
    fn confusion_is_stored_with_sentinel_amount() {
        let mut combat = basic_combat();

        handle_apply_power(1, 0, PowerId::Confusion, 1, &mut combat);

        assert_eq!(store::power_amount(&combat, 0, PowerId::Confusion), -1);
    }

    #[test]
    fn artifact_blocks_confusion_even_with_sentinel_amount() {
        let mut combat = basic_combat().with_player_power(PowerId::Artifact, 1);

        handle_apply_power(1, 0, PowerId::Confusion, 1, &mut combat);

        assert_eq!(store::power_amount(&combat, 0, PowerId::Confusion), 0);
        assert_eq!(store::power_amount(&combat, 0, PowerId::Artifact), 0);
    }

    #[test]
    fn player_ritual_grants_strength_at_end_of_turn() {
        let mut combat = basic_combat();
        handle_apply_power_detailed(
            0,
            0,
            PowerId::Ritual,
            2,
            None,
            Some(crate::content::powers::core::ritual::extra_data(
                true, false,
            )),
            &mut combat,
        );

        let ritual = store::powers_for(&combat, 0)
            .and_then(|powers| powers.iter().find(|p| p.power_type == PowerId::Ritual))
            .cloned()
            .expect("ritual should exist");

        let actions = crate::content::powers::resolve_power_at_end_of_turn(&ritual, &combat, 0);
        for action in actions {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        assert_eq!(store::power_amount(&combat, 0, PowerId::Strength), 2);
    }

    #[test]
    fn monster_ritual_skips_first_round_then_grants_strength() {
        let mut combat = basic_combat()
            .with_monster_type(1, EnemyId::Cultist)
            .with_monster_max_hp(1, 50)
            .with_monster_hp(1, 50);
        handle_apply_power_detailed(
            1,
            1,
            PowerId::Ritual,
            3,
            None,
            Some(crate::content::powers::core::ritual::extra_data(
                false, true,
            )),
            &mut combat,
        );
        let _ = store::with_power_mut(&mut combat, 1, PowerId::Ritual, |ritual| {
            ritual.just_applied = false;
        });

        let first_round_actions = crate::content::powers::resolve_power_at_end_of_round(
            PowerId::Ritual,
            &combat,
            1,
            3,
            false,
        );
        for action in first_round_actions {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }
        while let Some(action) = combat.engine.action_queue.pop_front() {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        assert_eq!(store::power_amount(&combat, 1, PowerId::Strength), 0);
        let ritual = store::powers_for(&combat, 1)
            .and_then(|powers| powers.iter().find(|p| p.power_type == PowerId::Ritual))
            .expect("ritual should remain");
        assert_eq!(
            ritual.extra_data,
            crate::content::powers::core::ritual::extra_data(false, false)
        );

        let second_round_actions = crate::content::powers::resolve_power_at_end_of_round(
            PowerId::Ritual,
            &combat,
            1,
            3,
            false,
        );
        for action in second_round_actions {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        assert_eq!(store::power_amount(&combat, 1, PowerId::Strength), 3);
    }

    #[test]
    fn slow_power_is_kept_at_zero_for_giant_head_semantics() {
        let mut combat = basic_combat()
            .with_monster_type(1, EnemyId::GiantHead)
            .with_monster_max_hp(1, 500)
            .with_monster_hp(1, 500);

        handle_apply_power(1, 1, PowerId::Slow, 0, &mut combat);

        assert_eq!(store::power_amount(&combat, 1, PowerId::Slow), 0);
        assert!(store::powers_for(&combat, 1)
            .is_some_and(|powers| powers.iter().any(|power| power.power_type == PowerId::Slow)));
    }

    #[test]
    fn slow_power_gains_one_stack_after_each_card_played() {
        let mut combat = basic_combat()
            .with_monster_type(1, EnemyId::GiantHead)
            .with_monster_max_hp(1, 500)
            .with_monster_hp(1, 500);

        handle_apply_power(1, 1, PowerId::Slow, 0, &mut combat);

        let actions = crate::content::powers::resolve_power_on_card_played(
            PowerId::Slow,
            &combat,
            1,
            &crate::combat::CombatCard::new(crate::content::cards::CardId::Strike, 100),
            0,
        );
        for action in actions {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        assert_eq!(store::power_amount(&combat, 1, PowerId::Slow), 1);
    }

    #[test]
    fn slow_power_resets_to_zero_at_end_of_round_without_removing_power() {
        let mut combat = basic_combat()
            .with_monster_type(1, EnemyId::GiantHead)
            .with_monster_max_hp(1, 500)
            .with_monster_hp(1, 500)
            .with_monster_power(1, PowerId::Slow, 4);

        let actions = crate::content::powers::resolve_power_at_end_of_round(
            PowerId::Slow,
            &combat,
            1,
            4,
            false,
        );
        for action in actions {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        assert_eq!(store::power_amount(&combat, 1, PowerId::Slow), 0);
        assert!(store::powers_for(&combat, 1)
            .is_some_and(|powers| powers.iter().any(|power| power.power_type == PowerId::Slow)));
    }

    #[test]
    fn stasis_on_death_returns_copied_card_to_hand_and_clears_limbo() {
        let mut combat = basic_combat()
            .with_monster_type(1, EnemyId::BronzeOrb)
            .with_monster_max_hp(1, 54)
            .with_monster_hp(1, 54)
            .with_card_uuid_counter(1000);
        let stasis_card =
            crate::combat::CombatCard::new(crate::content::cards::CardId::PommelStrike, 777);
        combat.zones.limbo.push(stasis_card.clone());
        store::ensure_powers_for_mut(&mut combat, 1).push(crate::combat::Power {
            power_type: PowerId::Stasis,
            instance_id: None,
            amount: -1,
            extra_data: stasis_card.uuid as i32,
            just_applied: false,
        });

        combat.entities.monsters[0].current_hp = 0;
        crate::engine::action_handlers::check_and_trigger_monster_death(&mut combat, 1);
        while let Some(action) = combat.engine.action_queue.pop_front() {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        assert!(combat
            .zones
            .hand
            .iter()
            .any(|card| card.id == crate::content::cards::CardId::PommelStrike));
        assert!(combat.zones.limbo.iter().all(|card| card.uuid != 777));
    }
}
