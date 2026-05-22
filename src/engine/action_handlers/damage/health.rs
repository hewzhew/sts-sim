use super::core::{
    apply_damage_to_monster_via_pipeline, clear_post_combat_actions_if_ready,
    queue_on_block_gained_hooks, queue_player_hp_loss_hooks, queue_red_skull_threshold_actions,
    update_player_cards_on_damage,
};
use crate::content::powers::{store, PowerId};
use crate::runtime::action::{DamageInfo, DamageType, NO_SOURCE};
use crate::runtime::combat::CombatState;
pub fn handle_lose_hp(target: usize, amount: i32, triggers_rupture: bool, state: &mut CombatState) {
    if target == 0 {
        let final_amount = crate::content::relics::hooks::on_lose_hp_last(state, amount.max(0));
        let previous_hp = state.entities.player.current_hp;
        state.entities.player.current_hp = (state.entities.player.current_hp - final_amount).max(0);
        if final_amount > 0 {
            update_player_cards_on_damage(state);
            state.turn.increment_times_damaged_this_combat();
            queue_red_skull_threshold_actions(state, previous_hp, state.entities.player.current_hp);
            queue_player_hp_loss_hooks(
                state,
                final_amount,
                None,
                DamageType::HpLoss,
                triggers_rupture,
            );
        }
        if state.entities.player.current_hp <= 0 {
            super::super::try_revive(state);
        }
        clear_post_combat_actions_if_ready(state);
    } else {
        let mut final_amount = amount.max(0);
        if crate::content::powers::store::power_amount(
            state,
            target,
            crate::content::powers::PowerId::IntangiblePlayer,
        ) > 0
            && final_amount > 1
        {
            final_amount = 1;
        }
        let info = DamageInfo {
            source: NO_SOURCE,
            target,
            base: final_amount,
            output: final_amount,
            damage_type: DamageType::HpLoss,
            is_modified: false,
        };
        let _ = apply_damage_to_monster_via_pipeline(state, &info, final_amount);
    }
}

pub fn handle_poison_lose_hp(target: usize, amount: i32, state: &mut CombatState) {
    if target == 0 {
        if state.entities.player.current_hp > 0 {
            let final_amount = crate::content::relics::hooks::on_lose_hp_last(state, amount.max(0));
            let previous_hp = state.entities.player.current_hp;
            state.entities.player.current_hp =
                (state.entities.player.current_hp - final_amount).max(0);
            if final_amount > 0 {
                update_player_cards_on_damage(state);
                state.turn.increment_times_damaged_this_combat();
                queue_red_skull_threshold_actions(
                    state,
                    previous_hp,
                    state.entities.player.current_hp,
                );
                queue_player_hp_loss_hooks(state, final_amount, None, DamageType::HpLoss, false);
            }
            if state.entities.player.current_hp <= 0 {
                super::super::try_revive(state);
            }
        }
    } else if state
        .entities
        .monsters
        .iter()
        .any(|m| m.id == target && m.current_hp > 0)
    {
        let info = DamageInfo {
            source: NO_SOURCE,
            target,
            base: amount.max(0),
            output: amount.max(0),
            damage_type: DamageType::HpLoss,
            is_modified: false,
        };
        let _ = apply_damage_to_monster_via_pipeline(state, &info, amount.max(0));
    }

    let should_remove_poison = store::with_power_mut(state, target, PowerId::Poison, |power| {
        power.amount -= 1;
        power.amount == 0
    })
    .unwrap_or(false);
    if should_remove_poison {
        store::remove_power_type(state, target, PowerId::Poison);
    }

    clear_post_combat_actions_if_ready(state);
}

pub fn handle_set_current_hp(target: usize, hp: i32, state: &mut CombatState) {
    let clamped_hp = hp.max(0);
    if target == 0 {
        state.entities.player.current_hp = clamped_hp.min(state.entities.player.max_hp);
        if state.entities.player.current_hp <= 0 {
            super::super::try_revive(state);
        }
        return;
    }

    if let Some(monster) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        monster.current_hp = clamped_hp.min(monster.max_hp);
    }
    super::super::check_and_trigger_monster_death(state, target);
    clear_post_combat_actions_if_ready(state);
}

pub fn handle_gain_block(target: usize, amount: i32, state: &mut CombatState) {
    if target == 0 {
        if state.entities.player.current_hp > 0 {
            state.entities.player.block += amount;
            queue_on_block_gained_hooks(state, 0, amount);
        }
    } else if let Some(m) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        if m.current_hp > 0 {
            m.block += amount;
        }
    }
}

pub fn handle_double_block(target: usize, state: &mut CombatState) {
    let current_block = if target == 0 {
        state.entities.player.block
    } else {
        state
            .entities
            .monsters
            .iter()
            .find(|m| m.id == target)
            .map_or(0, |m| m.block)
    };
    if current_block > 0 {
        handle_gain_block(target, current_block, state);
    }
}

pub fn handle_gain_block_random_monster(source: usize, amount: i32, state: &mut CombatState) {
    let alive: Vec<usize> = state
        .entities
        .monsters
        .iter()
        .filter(|m| {
            m.id != source
                && !matches!(
                    crate::content::monsters::resolve_monster_turn_plan(state, m).summary_spec(),
                    crate::runtime::monster_move::MonsterMoveSpec::Escape
                )
                && !m.is_dying
        })
        .map(|m| m.id)
        .collect();
    let target_id = if !alive.is_empty() {
        let idx = state.rng.ai_rng.random(alive.len() as i32 - 1) as usize;
        alive[idx]
    } else {
        source
    };
    if let Some(m) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == target_id)
    {
        m.block += amount;
    }
}

pub fn handle_lose_block(target: usize, amount: i32, state: &mut CombatState) {
    if target == 0 {
        state.entities.player.block = (state.entities.player.block - amount).max(0);
    } else if let Some(m) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        m.block = (m.block - amount).max(0);
    }
}

pub fn handle_remove_all_block(target: usize, state: &mut CombatState) {
    if target == 0 {
        state.entities.player.block = 0;
    } else if let Some(m) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        m.block = 0;
    }
}

pub fn handle_heal(target: usize, mut amount: i32, state: &mut CombatState) {
    if amount < 0 {
        let pct = (-amount) as f32 / 100.0;
        if target == 0 {
            amount = std::cmp::max(1, (state.entities.player.max_hp as f32 * pct) as i32);
        } else if let Some(m) = state.entities.monsters.iter().find(|m| m.id == target) {
            amount = std::cmp::max(1, (m.max_hp as f32 * pct) as i32);
        }
    }
    if target == 0 {
        amount = crate::content::relics::hooks::on_calculate_heal(state, amount);
        let previous_hp = state.entities.player.current_hp;
        state.entities.player.current_hp =
            (state.entities.player.current_hp + amount).min(state.entities.player.max_hp);
        queue_red_skull_threshold_actions(state, previous_hp, state.entities.player.current_hp);
    } else if let Some(m) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        if m.is_dying {
            return;
        }
        m.current_hp = (m.current_hp + amount).min(m.max_hp);
    }
}

pub fn increase_player_max_hp_like_java(amount: i32, state: &mut CombatState) {
    state.entities.player.max_hp += amount;
    handle_heal(0, amount, state);
}
