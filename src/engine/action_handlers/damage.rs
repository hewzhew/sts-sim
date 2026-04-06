// action_handlers/damage.rs — Combat damage domain
//
// Handles: Damage, DamageAllEnemies, AttackDamageRandomEnemy, DropkickDamageAndEffect,
//          FiendFire, Feed, VampireDamage, VampireDamageAllEnemies,
//          LoseHp, GainBlock, GainBlockRandomMonster, LoseBlock, GainEnergy,
//          Heal, GainMaxHp, LoseMaxHp,
//          LimitBreak, BlockPerNonAttack, ExhaustAllNonAttack, ExhaustRandomCard

use crate::action::{Action, ActionInfo, AddTo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::powers::PowerId;

/// Shared block-deduction logic. Returns unblocked damage.
pub fn deduct_block(block: &mut i32, damage: i32) -> i32 {
    if *block > 0 {
        if damage >= *block {
            let unblocked = damage - *block;
            *block = 0;
            unblocked
        } else {
            *block -= damage;
            0
        }
    } else {
        damage
    }
}

/// Complete damage-to-monster pipeline: block → Boot → onAttackedToChangeDamage → HP loss → death.
/// Returns the final HP lost (after block).
/// This does NOT include on_hp_lost / on_attacked power hooks — those are only fired from
/// the full Action::Damage path (which has access to damage_type for hook guards).
pub fn apply_raw_damage_to_monster(state: &mut CombatState, target_id: usize, raw_damage: i32) -> i32 {
    let mut hp_lost = 0;
    if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target_id) {
        let mut final_damage = raw_damage.max(0);
        final_damage = deduct_block(&mut m.block, final_damage);
        if final_damage > 0 {
            m.current_hp = (m.current_hp - final_damage).max(0);
            hp_lost = final_damage;
        }
    }
    super::check_and_trigger_monster_death(state, target_id);
    hp_lost
}

pub fn handle_damage(info: crate::action::DamageInfo, state: &mut CombatState) {
    let target_id = info.target;
    let source_id = info.source;

    // Apply power modifiers if not already applied (is_modified == false)
    let calculated_output = if !info.is_modified && source_id != 0
        && info.damage_type == DamageType::Normal {
        crate::content::powers::calculate_monster_damage(info.base, source_id, target_id, state)
    } else {
        info.output.max(0)
    };

    let mut final_damage = calculated_output;
    let target_is_player = target_id == 0;

    // 1. Final Receive / Intangible Pre-Check
    if let Some(target_powers) = state.power_db.get(&target_id).cloned() {
        for power in &target_powers {
            final_damage = crate::content::powers::resolve_power_at_damage_final_receive(
                power.power_type,
                final_damage,
                power.amount,
                info.damage_type,
            );
        }
    }

    if target_is_player {
        // 2. Block Deduction
        let _had_block = state.player.block > 0;
        final_damage = deduct_block(&mut state.player.block, final_damage);

        // 3. onAttackedToChangeDamage (Relics then Powers)
        final_damage = crate::content::relics::hooks::on_attacked_to_change_damage(state, final_damage, &info);
        if let Some(powers) = state.power_db.get(&0).cloned() {
            for power in &powers {
                final_damage = crate::content::powers::resolve_power_on_attacked_to_change_damage(
                    power.power_type, state, &info, final_damage, power.amount
                );
            }
        }

        // 4. on_attacked (Target Powers + Relics)
        if source_id != 0 || info.damage_type == DamageType::Normal {
            if let Some(powers) = state.power_db.get(&0).cloned() {
                for power in &powers {
                    let hook_actions = crate::content::powers::resolve_power_on_attacked(
                        power.power_type, state, 0, final_damage, source_id, power.amount
                    );
                    for a in hook_actions.into_iter().rev() {
                        state.action_queue.push_front(a);
                    }
                }
            }
        }

        // 5. onLoseHpLast (Tungsten Rod)
        final_damage = crate::content::relics::hooks::on_lose_hp_last(state, final_damage);

        if final_damage > 0 {
            // 6. Power onLoseHp + Relic onLoseHp
            let lose_hp_actions = crate::content::relics::hooks::on_lose_hp(state, final_damage);
            crate::engine::core::queue_actions(&mut state.action_queue, lose_hp_actions);

            state.player.current_hp = (state.player.current_hp - final_damage).max(0);
            state.counters.times_damaged_this_combat += 1;

            // 7. Death Check
            if state.player.current_hp <= 0 {
                super::try_revive(state);
            }
        }
    } else if let Some(mut m) = state.monsters.iter().find(|m| m.id == target_id).cloned() {
        // Skip damage to dying/escaping monsters
        if m.is_dying {
            return;
        }

        // Damage to monster
        let had_block = m.block > 0;
        final_damage = deduct_block(&mut m.block, final_damage);

        // Boot relic
        if source_id == 0
            && info.damage_type == DamageType::Normal
            && final_damage > 0 && final_damage < 5
            && state.player.has_relic(crate::content::relics::RelicId::Boot)
        {
            final_damage = 5;
        }

        // Monster powers onAttackedToChangeDamage
        if let Some(powers) = state.power_db.get(&target_id).cloned() {
            for power in &powers {
                final_damage = crate::content::powers::resolve_power_on_attacked_to_change_damage(
                    power.power_type, state, &info, final_damage, power.amount
                );
            }
        }

        // Write back block to real monster and apply HP loss
        if let Some(real_m) = state.monsters.iter_mut().find(|monster| monster.id == target_id) {
            real_m.block = m.block;
            if final_damage > 0 {
                real_m.current_hp = (real_m.current_hp - final_damage).max(0);
            }
        }

        // Centralized death mechanics
        super::check_and_trigger_monster_death(state, target_id);

        // HandDrill: if block broke, apply 2 Vulnerable
        if had_block && m.block == 0 && state.player.has_relic(crate::content::relics::RelicId::HandDrill) {
            let hand_drill_actions = crate::content::relics::hand_drill::on_break_block(state, target_id);
            crate::engine::core::queue_actions(&mut state.action_queue, hand_drill_actions);
        }

        // on_hp_lost power hooks (ModeShift, Split, etc.)
        if final_damage > 0 {
            if let Some(powers) = state.power_db.get(&target_id).cloned() {
                for power in &powers {
                    let hook_actions = crate::content::powers::resolve_power_on_hp_lost(
                        power.power_type, state, target_id, final_damage
                    );
                    for a in hook_actions.into_iter().rev() {
                        state.action_queue.push_front(a);
                    }
                }
            }

            // Monster trait hooks (takes care of Java's damage() overrides for behavior)
            if let Some(m) = state.monsters.iter().find(|m| m.id == target_id).cloned() {
                if let Some(eid) = crate::content::monsters::EnemyId::from_id(m.monster_type) {
                    let monster_actions = crate::content::monsters::dispatch_on_damaged(eid, state, &m, final_damage);
                    crate::engine::core::queue_actions(&mut state.action_queue, monster_actions);
                }
            }
        }

        // Monster onAttacked (Thorns, CurlUp, Angry, etc.)
        let should_fire_monster_on_attacked = info.damage_type != DamageType::Thorns
            && info.damage_type != DamageType::HpLoss;
        if should_fire_monster_on_attacked {
            if let Some(powers) = state.power_db.get(&target_id).cloned() {
                for power in &powers {
                    let hook_actions = crate::content::powers::resolve_power_on_attacked(
                        power.power_type, state, target_id, final_damage, source_id, power.amount
                    );
                    if power.power_type == PowerId::Malleable {
                        for a in hook_actions {
                            state.action_queue.push_back(a);
                        }
                        if let Some(powers_mut) = state.power_db.get_mut(&target_id) {
                            if let Some(mal) = powers_mut.iter_mut().find(|p| p.power_type == PowerId::Malleable) {
                                if final_damage > 0 {
                                    mal.amount += 1;
                                }
                            }
                        }
                    } else {
                        for a in hook_actions {
                            state.action_queue.push_front(a);
                        }
                    }
                }
            }
        }
        // CurlUp: zero amount after dispatch to prevent re-trigger on multi-hit
        if let Some(powers) = state.power_db.get_mut(&target_id) {
            if let Some(curl) = powers.iter_mut().find(|p| p.power_type == PowerId::CurlUp) {
                if curl.amount > 0 && final_damage > 0 {
                    curl.amount = 0;
                }
            }
        }
    }
}

pub fn handle_damage_all_enemies(source: usize, damages: smallvec::SmallVec<[i32; 5]>, damage_type: DamageType, is_modified: bool, state: &mut CombatState) {
    let mut individual_damages: smallvec::SmallVec<[Action; 5]> = smallvec::SmallVec::new();
    for (i, &dmg) in damages.iter().enumerate() {
        if i >= state.monsters.len() { break; }
        let m = &state.monsters[i];
        if m.current_hp <= 0 || m.is_dying || m.is_escaped { continue; }
        individual_damages.push(Action::Damage(crate::action::DamageInfo {
            source,
            target: m.id,
            base: dmg,
            output: dmg,
            damage_type,
            is_modified,
        }));
    }
    for action in individual_damages.into_iter().rev() {
        state.action_queue.push_front(action);
    }
}

pub fn handle_attack_damage_random_enemy(base_damage: i32, state: &mut CombatState) {
    let alive: Vec<usize> = state.monsters.iter()
        .filter(|m| m.current_hp > 0 && !m.is_dying && !m.is_escaped)
        .map(|m| m.id)
        .collect();
    if !alive.is_empty() {
        let idx = state.rng.card_random_rng.random(alive.len() as i32 - 1) as usize;
        let target_id = alive[idx];
        apply_raw_damage_to_monster(state, target_id, base_damage);
    }
}

pub fn handle_dropkick(target: usize, damage_info: crate::action::DamageInfo, state: &mut CombatState) {
    let has_vulnerable = state.power_db.get(&target).map_or(false, |powers| {
        powers.iter().any(|p| p.power_type == PowerId::Vulnerable && p.amount > 0)
    });
    if has_vulnerable {
        state.action_queue.push_front(Action::DrawCards(1));
        state.action_queue.push_front(Action::GainEnergy { amount: 1 });
    }
    state.action_queue.push_front(Action::Damage(damage_info));
}

pub fn handle_fiend_fire(target: usize, damage_info: crate::action::DamageInfo, state: &mut CombatState) {
    let hand_cards: Vec<crate::combat::CombatCard> = state.hand.drain(..).collect();
    let count = hand_cards.len();
    for card in hand_cards {
        state.exhaust_pile.push(card);
        let exhaust_actions = crate::content::relics::hooks::on_exhaust(state);
        crate::engine::core::queue_actions(&mut state.action_queue, exhaust_actions);
    }
    for _ in 0..count {
        apply_raw_damage_to_monster(state, target, damage_info.output);
    }
}

pub fn handle_feed(target: usize, damage_info: crate::action::DamageInfo, max_hp_amount: i32, state: &mut CombatState) {
    let mut killed = false;
    if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
        let mut final_damage = damage_info.output.max(0);
        final_damage = deduct_block(&mut m.block, final_damage);
        if final_damage > 0 {
            m.current_hp = (m.current_hp - final_damage).max(0);
        }
        if m.current_hp <= 0 {
            killed = true;
        }
    }
    super::check_and_trigger_monster_death(state, target);
    if killed {
        state.player.max_hp += max_hp_amount;
        state.player.current_hp += max_hp_amount;
    }
}

pub fn handle_vampire_damage(info: crate::action::DamageInfo, state: &mut CombatState) {
    let hp_lost = apply_raw_damage_to_monster(state, info.target, info.output);
    if hp_lost > 0 {
        state.player.current_hp = (state.player.current_hp + hp_lost).min(state.player.max_hp);
    }
}

pub fn handle_vampire_damage_all_enemies(damages: smallvec::SmallVec<[i32; 5]>, state: &mut CombatState) {
    let mut total_hp_lost = 0;
    for (i, &dmg) in damages.iter().enumerate() {
        let target_id = i + 1;
        if let Some(m) = state.monsters.iter().find(|m| m.id == target_id) {
            if m.current_hp <= 0 || m.is_dying { continue; }
        } else {
            continue;
        }
        total_hp_lost += apply_raw_damage_to_monster(state, target_id, dmg);
    }
    if total_hp_lost > 0 {
        state.player.current_hp = (state.player.current_hp + total_hp_lost).min(state.player.max_hp);
    }
}

pub fn handle_lose_hp(target: usize, amount: i32, state: &mut CombatState) {
    if target == 0 {
        state.player.current_hp -= amount;
        if state.player.current_hp <= 0 {
            super::try_revive(state);
        }
        if amount > 0 {
            state.counters.times_damaged_this_combat += 1;
        }
    } else {
        let mut actual_lost = 0;
        if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
            let prev = m.current_hp;
            m.current_hp = (m.current_hp - amount).max(0);
            actual_lost = prev - m.current_hp;
        }
        super::check_and_trigger_monster_death(state, target);
        
        if actual_lost > 0 {
            // Trait hook for Wakeup by Poison/Thorns (equivalent of calling damage() in Java)
            if let Some(m) = state.monsters.iter().find(|m| m.id == target).cloned() {
                if let Some(eid) = crate::content::monsters::EnemyId::from_id(m.monster_type) {
                    let monster_actions = crate::content::monsters::dispatch_on_damaged(eid, state, &m, actual_lost);
                    crate::engine::core::queue_actions(&mut state.action_queue, monster_actions);
                }
            }
        }
    }
}

pub fn handle_gain_block(target: usize, amount: i32, state: &mut CombatState) {
    if target == 0 {
        if state.player.current_hp > 0 {
            state.player.block += amount;
        }
    } else if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
        if m.current_hp > 0 {
            m.block += amount;
        }
    }
}

pub fn handle_gain_block_random_monster(source: usize, amount: i32, state: &mut CombatState) {
    let alive: Vec<usize> = state.monsters.iter()
        .filter(|m| m.id != source
            && m.current_intent != Intent::Escape
            && !m.is_dying)
        .map(|m| m.id)
        .collect();
    let target_id = if !alive.is_empty() {
        let idx = state.rng.ai_rng.random(alive.len() as i32 - 1) as usize;
        alive[idx]
    } else {
        source
    };
    if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target_id) {
        m.block += amount;
    }
}

pub fn handle_lose_block(target: usize, amount: i32, state: &mut CombatState) {
    if target == 0 {
        state.player.block = (state.player.block - amount).max(0);
    } else if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
        m.block = (m.block - amount).max(0);
    }
}

pub fn handle_heal(target: usize, mut amount: i32, state: &mut CombatState) {
    if amount < 0 {
        let pct = (-amount) as f32 / 100.0;
        if target == 0 {
            amount = std::cmp::max(1, (state.player.max_hp as f32 * pct) as i32);
        } else if let Some(m) = state.monsters.iter().find(|m| m.id == target) {
            amount = std::cmp::max(1, (m.max_hp as f32 * pct) as i32);
        }
    }
    if target == 0 {
        state.player.current_hp = (state.player.current_hp + amount).min(state.player.max_hp);
    } else if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target) {
        m.current_hp = (m.current_hp + amount).min(m.max_hp);
    }
}

pub fn handle_limit_break(state: &mut CombatState) {
    if let Some(powers) = state.power_db.get_mut(&0) {
        if let Some(str_power) = powers.iter_mut().find(|p| p.power_type == PowerId::Strength) {
            str_power.amount *= 2;
        }
    }
}

pub fn handle_block_per_non_attack(block_per_card: i32, state: &mut CombatState) {
    let non_attacks: Vec<u32> = state.hand.iter()
        .filter(|c| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.card_type != crate::content::cards::CardType::Attack
        })
        .map(|c| c.uuid)
        .collect();
    let count = non_attacks.len() as i32;
    state.player.block += block_per_card * count;
    for uuid in non_attacks {
        crate::engine::core::queue_actions(&mut state.action_queue, smallvec::smallvec![
            ActionInfo {
                action: Action::ExhaustCard { card_uuid: uuid, source_pile: crate::state::PileType::Hand },
                insertion_mode: AddTo::Bottom
            }
        ]);
    }
}

pub fn handle_exhaust_all_non_attack(state: &mut CombatState) {
    let non_attacks: Vec<u32> = state.hand.iter()
        .filter(|c| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.card_type != crate::content::cards::CardType::Attack
        })
        .map(|c| c.uuid)
        .collect();
    for uuid in non_attacks {
        crate::engine::core::queue_actions(&mut state.action_queue, smallvec::smallvec![
            ActionInfo {
                action: Action::ExhaustCard { card_uuid: uuid, source_pile: crate::state::PileType::Hand },
                insertion_mode: AddTo::Bottom
            }
        ]);
    }
}

pub fn handle_exhaust_random_card(amount: usize, state: &mut CombatState) {
    for _ in 0..amount {
        if state.hand.is_empty() { break; }
        let idx = state.rng.card_random_rng.random(state.hand.len() as i32 - 1) as usize;
        let card = state.hand.remove(idx);
        state.exhaust_pile.push(card);
        let exhaust_actions = crate::content::relics::hooks::on_exhaust(state);
        crate::engine::core::queue_actions(&mut state.action_queue, exhaust_actions);
    }
}
