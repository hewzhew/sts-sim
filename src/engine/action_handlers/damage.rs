// action_handlers/damage.rs — Combat damage domain
//
// Handles: Damage, DamageAllEnemies, AttackDamageRandomEnemy, DropkickDamageAndEffect,
//          FiendFire, Feed, VampireDamage, VampireDamageAllEnemies,
//          LoseHp, GainBlock, GainBlockRandomMonster, LoseBlock, GainEnergy,
//          Heal, GainMaxHp, LoseMaxHp,
//          LimitBreak, BlockPerNonAttack, ExhaustAllNonAttack, ExhaustRandomCard

use crate::content::powers::store;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageType, NO_SOURCE};
use crate::runtime::combat::CombatState;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MonsterDamageOutcome {
    hp_lost: i32,
    died: bool,
}

fn queue_player_hp_loss_hooks(
    state: &mut CombatState,
    amount: i32,
    source: Option<crate::core::EntityId>,
    damage_type: DamageType,
    triggers_rupture: bool,
) {
    if amount <= 0 {
        return;
    }

    for power in &store::powers_snapshot_for(state, 0) {
        let hook_actions = crate::content::powers::resolve_power_on_hp_lost(
            power.power_type,
            state,
            0,
            amount,
            source,
            damage_type,
            triggers_rupture,
        );
        for action in hook_actions.into_iter().rev() {
            state.queue_action_front(action);
        }
    }

    let relic_actions = crate::content::relics::hooks::on_lose_hp(state, amount);
    state.queue_actions(relic_actions);
}

fn queue_red_skull_threshold_actions(state: &mut CombatState, previous_hp: i32, current_hp: i32) {
    if !state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::RedSkull)
    {
        return;
    }

    let actions = crate::content::relics::red_skull::on_player_hp_changed(
        previous_hp,
        current_hp,
        state.entities.player.max_hp,
    );
    state.queue_actions(actions);
}

fn queue_on_block_gained_hooks(
    state: &mut CombatState,
    owner: crate::core::EntityId,
    gained_block: i32,
) {
    if gained_block <= 0 {
        return;
    }

    for power in &store::powers_snapshot_for(state, owner) {
        let hook_actions = crate::content::powers::resolve_power_on_block_gained(
            power.power_type,
            state,
            owner,
            power.amount,
            gained_block,
        );
        for action in hook_actions {
            state.queue_action_back(action);
        }
    }
}

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
pub fn apply_raw_damage_to_monster(
    state: &mut CombatState,
    target_id: usize,
    raw_damage: i32,
) -> i32 {
    let mut hp_lost = 0;
    if let Some(m) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == target_id)
    {
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

fn apply_damage_to_monster_via_pipeline(
    state: &mut CombatState,
    info: &crate::runtime::action::DamageInfo,
    mut final_damage: i32,
) -> MonsterDamageOutcome {
    let target_id = info.target;
    let source_id = info.source;
    let mut outcome = MonsterDamageOutcome::default();

    if let Some(mut m) = state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == target_id)
        .cloned()
    {
        if m.is_dying {
            return outcome;
        }

        let target_hp_before_damage = m.current_hp;
        let had_block = m.block > 0;
        final_damage = deduct_block(&mut m.block, final_damage);

        if source_id == 0
            && info.damage_type == DamageType::Normal
            && final_damage > 0
            && final_damage < 5
            && state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::Boot)
        {
            final_damage = 5;
        }

        for power in &store::powers_snapshot_for(state, target_id) {
            final_damage = crate::content::powers::resolve_power_on_attacked_to_change_damage(
                power.power_type,
                state,
                info,
                final_damage,
                power.amount,
            );
        }

        if let Some(real_m) = state
            .entities
            .monsters
            .iter_mut()
            .find(|monster| monster.id == target_id)
        {
            real_m.block = m.block;
            if final_damage > 0 {
                real_m.current_hp = (real_m.current_hp - final_damage).max(0);
                outcome.hp_lost = final_damage;
                outcome.died = real_m.current_hp <= 0;
            }
        }

        super::check_and_trigger_monster_death(state, target_id);

        if had_block
            && m.block == 0
            && state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::HandDrill)
        {
            let hand_drill_actions =
                crate::content::relics::hand_drill::on_break_block(state, target_id);
            state.queue_actions(hand_drill_actions);
        }

        if outcome.hp_lost > 0 {
            for power in &store::powers_snapshot_for(state, target_id) {
                let hook_actions = crate::content::powers::resolve_power_on_hp_lost(
                    power.power_type,
                    state,
                    target_id,
                    outcome.hp_lost,
                    Some(source_id),
                    info.damage_type,
                    false,
                );
                for a in hook_actions.into_iter().rev() {
                    state.queue_action_front(a);
                }
            }

            if let Some(m) = state
                .entities
                .monsters
                .iter()
                .find(|m| m.id == target_id)
                .cloned()
            {
                if let Some(eid) = crate::content::monsters::EnemyId::from_id(m.monster_type) {
                    let monster_actions = crate::content::monsters::dispatch_on_damaged(
                        eid,
                        state,
                        &m,
                        outcome.hp_lost,
                    );
                    state.queue_actions(monster_actions);
                }
            }
        }

        for power in &store::powers_snapshot_for(state, target_id) {
            let should_fire_this_power = match info.damage_type {
                DamageType::HpLoss => false,
                DamageType::Thorns => power.power_type == PowerId::Shifting,
                _ => true,
            };
            if !should_fire_this_power {
                continue;
            }

            if power.power_type == PowerId::Malleable {
                if outcome.hp_lost > 0 && outcome.hp_lost < target_hp_before_damage {
                    state.queue_action_back(Action::GainBlock {
                        target: target_id,
                        amount: power.amount,
                    });
                    let _ = store::with_power_mut(state, target_id, PowerId::Malleable, |mal| {
                        mal.amount += 1;
                    });
                }
                continue;
            }

            let hook_actions = crate::content::powers::resolve_power_on_attacked(
                power.power_type,
                state,
                target_id,
                outcome.hp_lost,
                source_id,
                power.amount,
            );
            if matches!(
                power.power_type,
                PowerId::Malleable | PowerId::CurlUp | PowerId::Flight
            ) {
                for a in hook_actions {
                    state.queue_action_back(a);
                }
                let _ = store::with_power_mut(state, target_id, PowerId::CurlUp, |curl| {
                    if curl.amount > 0 && outcome.hp_lost > 0 && source_id != NO_SOURCE {
                        curl.amount = 0;
                    }
                });
            } else {
                for a in hook_actions {
                    state.queue_action_front(a);
                }
            }
        }
    }

    outcome
}

pub fn handle_damage(info: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    let target_id = info.target;
    let source_id = info.source;

    // Damage contract:
    // - Player-origin Normal damage arrives pre-evaluated in `output`.
    // - Monster-origin Normal damage is re-resolved here from `base`.
    // - Non-Normal damage kinds (`HpLoss`, `Thorns`, etc.) use the supplied numeric value.
    let (calculated_output, damage_already_includes_final_receive) = if !info.is_modified
        && source_id != 0
        && source_id != NO_SOURCE
        && info.damage_type == DamageType::Normal
    {
        (
            crate::content::powers::calculate_monster_damage(
                info.base, source_id, target_id, state,
            ),
            true,
        )
    } else if (source_id == 0 || source_id == NO_SOURCE) && info.damage_type == DamageType::Normal {
        (info.output.max(0), true)
    } else {
        (info.output.max(0), false)
    };

    let mut final_damage = calculated_output;
    let target_is_player = target_id == 0;

    // 1. Final Receive / Intangible Pre-Check
    if !damage_already_includes_final_receive {
        for power in &store::powers_snapshot_for(state, target_id) {
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
        let _had_block = state.entities.player.block > 0;
        final_damage = deduct_block(&mut state.entities.player.block, final_damage);

        // 3. onAttackedToChangeDamage (Relics then Powers)
        final_damage =
            crate::content::relics::hooks::on_attacked_to_change_damage(state, final_damage, &info);
        for power in &store::powers_snapshot_for(state, 0) {
            final_damage = crate::content::powers::resolve_power_on_attacked_to_change_damage(
                power.power_type,
                state,
                &info,
                final_damage,
                power.amount,
            );
        }
        // 4. on_attacked (Target Powers + Relics)
        if source_id != 0 || info.damage_type == DamageType::Normal {
            for power in &store::powers_snapshot_for(state, 0) {
                let hook_actions = crate::content::powers::resolve_power_on_attacked(
                    power.power_type,
                    state,
                    0,
                    final_damage,
                    source_id,
                    power.amount,
                );
                for a in hook_actions.into_iter().rev() {
                    state.queue_action_front(a);
                }
            }
        }

        // 5. onLoseHpLast (Tungsten Rod)
        final_damage = crate::content::relics::hooks::on_lose_hp_last(state, final_damage);

        if final_damage > 0 {
            let previous_hp = state.entities.player.current_hp;
            state.entities.player.current_hp =
                (state.entities.player.current_hp - final_damage).max(0);
            state.turn.increment_times_damaged_this_combat();
            queue_red_skull_threshold_actions(state, previous_hp, state.entities.player.current_hp);
            queue_player_hp_loss_hooks(
                state,
                final_damage,
                Some(source_id),
                info.damage_type,
                false,
            );

            // 7. Death Check
            if state.entities.player.current_hp <= 0 {
                super::try_revive(state);
            }
        }
    } else {
        let _ = apply_damage_to_monster_via_pipeline(state, &info, final_damage);
    }
}

fn damage_type(kind: crate::semantics::combat::DamageKind) -> crate::runtime::action::DamageType {
    match kind {
        crate::semantics::combat::DamageKind::Normal => crate::runtime::action::DamageType::Normal,
        crate::semantics::combat::DamageKind::Thorns => crate::runtime::action::DamageType::Thorns,
        crate::semantics::combat::DamageKind::HpLoss => crate::runtime::action::DamageType::HpLoss,
        crate::semantics::combat::DamageKind::Unknown => panic!("monster attack kind unknown"),
    }
}

/// Executes the canonical monster attack contract.
///
/// `base_damage` is the truth input from monster planning. For `Normal` attacks the
/// engine recalculates final damage from this base using the monster damage pipeline;
/// monster content code must not precompute or guess a modified output value.
pub fn handle_monster_attack(
    source: usize,
    target: usize,
    base_damage: i32,
    damage_kind: crate::semantics::combat::DamageKind,
    state: &mut CombatState,
) {
    handle_damage(
        crate::runtime::action::DamageInfo {
            source,
            target,
            base: base_damage,
            output: base_damage,
            damage_type: damage_type(damage_kind),
            is_modified: !matches!(damage_kind, crate::semantics::combat::DamageKind::Normal),
        },
        state,
    );
}

pub fn handle_damage_all_enemies(
    source: usize,
    damages: smallvec::SmallVec<[i32; 5]>,
    damage_type: DamageType,
    is_modified: bool,
    state: &mut CombatState,
) {
    let mut individual_damages: smallvec::SmallVec<[Action; 5]> = smallvec::SmallVec::new();
    for (i, &dmg) in damages.iter().enumerate() {
        if i >= state.entities.monsters.len() {
            break;
        }
        let m = &state.entities.monsters[i];
        if m.current_hp <= 0 || m.is_dying || m.is_escaped {
            continue;
        }
        individual_damages.push(Action::Damage(crate::runtime::action::DamageInfo {
            source,
            target: m.id,
            base: dmg,
            output: dmg,
            damage_type,
            is_modified,
        }));
    }
    for action in individual_damages.into_iter().rev() {
        state.queue_action_front(action);
    }
}

pub fn handle_attack_damage_random_enemy(
    base_damage: i32,
    damage_type: DamageType,
    applies_target_modifiers: bool,
    state: &mut CombatState,
) {
    let alive: Vec<usize> = state
        .entities
        .monsters
        .iter()
        .filter(|m| m.current_hp > 0 && !m.is_dying && !m.is_escaped)
        .map(|m| m.id)
        .collect();
    if !alive.is_empty() {
        let idx = state.rng.card_random_rng.random(alive.len() as i32 - 1) as usize;
        let target_id = alive[idx];
        let final_damage = if applies_target_modifiers && matches!(damage_type, DamageType::Normal)
        {
            let mut damage = base_damage as f32;
            let pseudo_card = crate::runtime::combat::CombatCard::new(
                crate::content::cards::CardId::SwordBoomerang,
                0,
            );
            for power in &store::powers_snapshot_for(state, target_id) {
                damage = crate::content::powers::resolve_power_on_calculate_damage_from_player(
                    power.power_type,
                    state,
                    &pseudo_card,
                    target_id,
                    damage,
                    power.amount,
                );
            }
            let mut damage_i = damage.max(0.0).floor() as i32;
            for power in &store::powers_snapshot_for(state, target_id) {
                damage_i = crate::content::powers::resolve_power_at_damage_final_receive(
                    power.power_type,
                    damage_i,
                    power.amount,
                    damage_type,
                );
            }
            damage_i.max(0)
        } else {
            base_damage
        };
        handle_damage(
            crate::runtime::action::DamageInfo {
                source: 0,
                target: target_id,
                base: base_damage,
                output: final_damage,
                damage_type,
                is_modified: applies_target_modifiers,
            },
            state,
        );
    }
}

pub fn handle_dropkick(
    target: usize,
    damage_info: crate::runtime::action::DamageInfo,
    state: &mut CombatState,
) {
    let has_vulnerable = store::power_amount(state, target, PowerId::Vulnerable) > 0;
    if has_vulnerable {
        state.queue_action_front(Action::DrawCards(1));
        state.queue_action_front(Action::GainEnergy { amount: 1 });
    }
    state.queue_action_front(Action::Damage(damage_info));
}

pub fn handle_fiend_fire(
    target: usize,
    damage_info: crate::runtime::action::DamageInfo,
    state: &mut CombatState,
) {
    let hand_cards: Vec<crate::runtime::combat::CombatCard> = state.zones.hand.drain(..).collect();
    let count = hand_cards.len();
    for card in hand_cards {
        super::cards::move_card_to_exhaust_pile(card, state);
    }
    for _ in 0..count {
        let mut info = damage_info.clone();
        info.target = target;
        let _ = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
    }
}

pub fn handle_feed(
    target: usize,
    damage_info: crate::runtime::action::DamageInfo,
    max_hp_amount: i32,
    state: &mut CombatState,
) {
    let mut info = damage_info;
    info.target = target;
    let outcome = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
    if outcome.died {
        state.entities.player.max_hp += max_hp_amount;
        state.entities.player.current_hp += max_hp_amount;
    }
}

pub fn handle_hand_of_greed(
    target: usize,
    damage_info: crate::runtime::action::DamageInfo,
    gold_amount: i32,
    state: &mut CombatState,
) {
    let mut info = damage_info;
    info.target = target;
    let outcome = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
    if outcome.died {
        state.queue_action_front(Action::GainGold {
            amount: gold_amount,
        });
    }
}

pub fn handle_ritual_dagger(
    target: usize,
    damage_info: crate::runtime::action::DamageInfo,
    misc_amount: i32,
    card_uuid: u32,
    state: &mut CombatState,
) {
    let mut info = damage_info;
    info.target = target;
    let outcome = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
    if outcome.died {
        state.queue_action_front(Action::ModifyCardMisc {
            card_uuid,
            amount: misc_amount,
        });
    }
}

pub fn handle_gain_gold(amount: i32, state: &mut CombatState) {
    if amount <= 0 {
        return;
    }

    state.entities.player.gold += amount;
    state.entities.player.gold_delta_this_combat += amount;

    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::BloodyIdol)
    {
        let actions = crate::content::relics::bloody_idol::BloodyIdol::on_gain_gold();
        state.queue_actions(actions);
    }
}

pub fn handle_steal_player_gold(thief_id: usize, amount: i32, state: &mut CombatState) {
    if amount <= 0 {
        if let Some(thief) = state
            .entities
            .monsters
            .iter_mut()
            .find(|m| m.id == thief_id)
        {
            thief.thief.protocol_seeded = true;
            thief.thief.slash_count = thief.thief.slash_count.saturating_add(1);
        }
        return;
    }

    let actual = amount.min(state.entities.player.gold).max(0);
    state.entities.player.gold = (state.entities.player.gold - actual).max(0);
    state.entities.player.gold_delta_this_combat -= actual;

    if let Some(thief) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == thief_id)
    {
        thief.thief.protocol_seeded = true;
        thief.thief.slash_count = thief.thief.slash_count.saturating_add(1);
        thief.thief.stolen_gold += actual;
    }
}

pub fn handle_vampire_damage(info: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    let source = info.source;
    if info.target == 0 {
        let previous_hp = state.entities.player.current_hp;
        handle_damage(info, state);
        let hp_lost = (previous_hp - state.entities.player.current_hp).max(0);
        if hp_lost > 0 {
            heal_vampire_source(state, source, hp_lost);
        }
    } else {
        let outcome = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
        if outcome.hp_lost > 0 {
            heal_vampire_source(state, source, outcome.hp_lost);
        }
    }
}

pub fn handle_vampire_damage_all_enemies(
    source: usize,
    damages: smallvec::SmallVec<[i32; 5]>,
    damage_type: DamageType,
    state: &mut CombatState,
) {
    let mut total_hp_lost = 0;
    for (i, &dmg) in damages.iter().enumerate() {
        let target_id = i + 1;
        if let Some(m) = state.entities.monsters.iter().find(|m| m.id == target_id) {
            if m.current_hp <= 0 || m.is_dying {
                continue;
            }
        } else {
            continue;
        }
        let outcome = apply_damage_to_monster_via_pipeline(
            state,
            &crate::runtime::action::DamageInfo {
                source,
                target: target_id,
                base: dmg,
                output: dmg,
                damage_type,
                is_modified: true,
            },
            dmg.max(0),
        );
        total_hp_lost += outcome.hp_lost;
    }
    if total_hp_lost > 0 {
        heal_vampire_source(state, source, total_hp_lost);
    }
}

fn heal_vampire_source(state: &mut CombatState, source: usize, amount: i32) {
    if amount <= 0 {
        return;
    }

    if source == 0 {
        let previous_hp = state.entities.player.current_hp;
        state.entities.player.current_hp =
            (state.entities.player.current_hp + amount).min(state.entities.player.max_hp);
        queue_red_skull_threshold_actions(state, previous_hp, state.entities.player.current_hp);
    } else if let Some(monster) = state.entities.monsters.iter_mut().find(|m| m.id == source) {
        monster.current_hp = (monster.current_hp + amount).min(monster.max_hp);
    }
}

pub fn handle_lose_hp(target: usize, amount: i32, triggers_rupture: bool, state: &mut CombatState) {
    if target == 0 {
        let final_amount = crate::content::relics::hooks::on_lose_hp_last(state, amount.max(0));
        let previous_hp = state.entities.player.current_hp;
        state.entities.player.current_hp = (state.entities.player.current_hp - final_amount).max(0);
        if final_amount > 0 {
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
            super::try_revive(state);
        }
    } else {
        let mut actual_lost = 0;
        if let Some(m) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
            let prev = m.current_hp;
            m.current_hp = (m.current_hp - amount).max(0);
            actual_lost = prev - m.current_hp;
        }
        super::check_and_trigger_monster_death(state, target);

        if actual_lost > 0 {
            // Trait hook for Wakeup by Poison/Thorns (equivalent of calling damage() in Java)
            if let Some(m) = state
                .entities
                .monsters
                .iter()
                .find(|m| m.id == target)
                .cloned()
            {
                if let Some(eid) = crate::content::monsters::EnemyId::from_id(m.monster_type) {
                    let monster_actions =
                        crate::content::monsters::dispatch_on_damaged(eid, state, &m, actual_lost);
                    state.queue_actions(monster_actions);
                }
            }
        }
    }
}

pub fn handle_set_current_hp(target: usize, hp: i32, state: &mut CombatState) {
    let clamped_hp = hp.max(0);
    if target == 0 {
        state.entities.player.current_hp = clamped_hp.min(state.entities.player.max_hp);
        if state.entities.player.current_hp <= 0 {
            super::try_revive(state);
        }
        return;
    }

    if let Some(monster) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        monster.current_hp = clamped_hp.min(monster.max_hp);
    }
    super::check_and_trigger_monster_death(state, target);
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

pub fn handle_gain_block_random_monster(source: usize, amount: i32, state: &mut CombatState) {
    let alive: Vec<usize> = state
        .entities
        .monsters
        .iter()
        .filter(|m| {
            m.id != source
                && m.current_hp > 0
                && !m.is_escaped
                && !matches!(
                    crate::content::monsters::resolve_monster_turn_plan(state, m).summary_spec(),
                    crate::semantics::combat::MonsterMoveSpec::Escape
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

pub fn handle_limit_break(state: &mut CombatState) {
    let _ = store::with_power_mut(state, 0, PowerId::Strength, |str_power| {
        str_power.amount *= 2;
    });
}

pub fn handle_block_per_non_attack(block_per_card: i32, state: &mut CombatState) {
    let non_attacks: Vec<u32> = state
        .zones
        .hand
        .iter()
        .filter(|c| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.card_type != crate::content::cards::CardType::Attack
        })
        .map(|c| c.uuid)
        .collect();

    // Java BlockPerNonAttackAction queues one ExhaustSpecificCardAction per non-attack,
    // then one GainBlockAction per exhausted card. That matters for on-exhaust and
    // on-gained-block hooks such as Feel No Pain and Juggernaut.
    let mut queued_actions = Vec::new();
    for uuid in &non_attacks {
        queued_actions.push(Action::ExhaustCard {
            card_uuid: *uuid,
            source_pile: crate::state::PileType::Hand,
        });
    }
    for _ in &non_attacks {
        queued_actions.push(Action::GainBlock {
            target: 0,
            amount: block_per_card,
        });
    }

    for action in queued_actions.into_iter().rev() {
        state.queue_action_front(action);
    }
}

pub fn handle_exhaust_all_non_attack(state: &mut CombatState) {
    let non_attacks: Vec<u32> = state
        .zones
        .hand
        .iter()
        .filter(|c| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.card_type != crate::content::cards::CardType::Attack
        })
        .map(|c| c.uuid)
        .collect();
    for uuid in non_attacks {
        state.queue_actions(smallvec::smallvec![ActionInfo {
            action: Action::ExhaustCard {
                card_uuid: uuid,
                source_pile: crate::state::PileType::Hand
            },
            insertion_mode: AddTo::Bottom
        }]);
    }
}

pub fn handle_exhaust_random_card(amount: usize, state: &mut CombatState) {
    for _ in 0..amount {
        if state.zones.hand.is_empty() {
            break;
        }
        let idx = state
            .rng
            .card_random_rng
            .random(state.zones.hand.len() as i32 - 1) as usize;
        let card_uuid = state.zones.hand[idx].uuid;
        super::cards::handle_exhaust_card(card_uuid, crate::state::PileType::Hand, state);
    }
}
