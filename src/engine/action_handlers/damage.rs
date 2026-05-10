// action_handlers/damage.rs — Combat damage domain
//
// Handles: Damage, DamageAllEnemies, AttackDamageRandomEnemy, DropkickDamageAndEffect,
//          FiendFire, Feed, VampireDamage, VampireDamageAllEnemies,
//          LoseHp, GainBlock, GainBlockRandomMonster, LoseBlock, GainEnergy,
//          Heal, GainMaxHp, LoseMaxHp,
//          LimitBreak, BlockPerNonAttack, ExhaustAllNonAttack, ExhaustRandomCard

use crate::content::powers::store;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageType, NO_SOURCE};
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
            power.amount,
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

fn update_player_cards_on_damage(state: &mut CombatState) {
    for card in state
        .zones
        .hand
        .iter_mut()
        .chain(state.zones.discard_pile.iter_mut())
        .chain(state.zones.draw_pile.iter_mut())
    {
        if card.id == crate::content::cards::CardId::BloodForBlood {
            card.update_cost_java(-1);
        }
    }
}

fn target_qualifies_for_non_minion_kill_reward(state: &CombatState, target: usize) -> bool {
    state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == target)
        .is_some_and(|m| !m.half_dead && !store::has_power(state, target, PowerId::Minion))
}

fn monsters_are_basically_dead_for_post_combat(state: &CombatState) -> bool {
    !state
        .entities
        .monsters
        .iter()
        .any(|m| m.current_hp > 0 && !m.is_dying && !m.is_escaped && !m.half_dead)
}

fn clear_post_combat_actions_if_ready(state: &mut CombatState) {
    if monsters_are_basically_dead_for_post_combat(state) {
        state.clear_post_combat_actions();
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
            let actual_hp_lost = final_damage.min(m.current_hp.max(0));
            m.current_hp = (m.current_hp - final_damage).max(0);
            hp_lost = actual_hp_lost;
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
                let actual_hp_lost = final_damage.min(real_m.current_hp.max(0));
                real_m.current_hp = (real_m.current_hp - final_damage).max(0);
                outcome.hp_lost = actual_hp_lost;
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
                    power.amount,
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
                info.damage_type,
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

    clear_post_combat_actions_if_ready(state);
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
                    info.damage_type,
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
            update_player_cards_on_damage(state);
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
        if !m.is_alive_for_action() {
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

pub fn handle_whirlwind(
    damages: smallvec::SmallVec<[i32; 5]>,
    damage_type: DamageType,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let base_effect = if energy_on_use != -1 {
        energy_on_use
    } else {
        state.turn.energy as i32
    };
    let effect = crate::content::relics::hooks::on_calculate_x_cost(state, base_effect);

    if effect > 0 {
        for _ in 0..effect {
            state.queue_action_back(Action::DamageAllEnemies {
                source: 0,
                damages: damages.clone(),
                damage_type,
                is_modified: false,
            });
        }
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
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
        .filter(|m| m.is_random_target_candidate())
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
    let count = state.zones.hand.len();
    for _ in 0..count {
        let mut info = damage_info.clone();
        info.target = target;
        state.queue_action_front(Action::Damage(info));
    }
    for _ in 0..count {
        state.queue_action_front(Action::ExhaustRandomCard { amount: 1 });
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
    if outcome.died && target_qualifies_for_non_minion_kill_reward(state, target) {
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
    if outcome.died && target_qualifies_for_non_minion_kill_reward(state, target) {
        handle_gain_gold(gold_amount, state);
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
    if outcome.died && target_qualifies_for_non_minion_kill_reward(state, target) {
        crate::engine::action_handlers::cards::handle_modify_card_misc(
            card_uuid,
            misc_amount,
            state,
        );
    }
}

pub fn handle_gain_gold(amount: i32, state: &mut CombatState) {
    if amount <= 0 {
        return;
    }

    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::Ectoplasm)
    {
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
            queue_vampire_heal_source(state, source, hp_lost, AddTo::Top);
        }
    } else {
        let outcome = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
        if outcome.hp_lost > 0 {
            queue_vampire_heal_source(state, source, outcome.hp_lost, AddTo::Top);
        }
        clear_post_combat_actions_if_ready(state);
    }
}

pub fn handle_vampire_damage_all_enemies(
    source: usize,
    damages: smallvec::SmallVec<[i32; 5]>,
    damage_type: DamageType,
    state: &mut CombatState,
) {
    let mut total_hp_lost = 0;
    let target_damage_pairs: Vec<(usize, i32)> = state
        .entities
        .monsters
        .iter()
        .zip(damages.iter())
        .filter_map(|(m, &dmg)| {
            if !m.is_alive_for_action() {
                None
            } else {
                Some((m.id, dmg))
            }
        })
        .collect();

    for (target_id, dmg) in target_damage_pairs {
        if dmg <= 0 {
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
        queue_vampire_heal_source(state, source, total_hp_lost, AddTo::Bottom);
    }
    clear_post_combat_actions_if_ready(state);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AddTo {
    Top,
    Bottom,
}

fn queue_vampire_heal_source(state: &mut CombatState, source: usize, amount: i32, add_to: AddTo) {
    if amount <= 0 {
        return;
    }

    let action = Action::Heal {
        target: source,
        amount,
    };
    match add_to {
        AddTo::Top => state.queue_action_front(action),
        AddTo::Bottom => state.queue_action_back(action),
    }
}

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
        clear_post_combat_actions_if_ready(state);
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

pub fn handle_limit_break(state: &mut CombatState) {
    if let Some(strength) = store::powers_for(state, 0)
        .and_then(|powers| powers.iter().find(|p| p.power_type == PowerId::Strength))
        .map(|power| power.amount)
    {
        super::powers::handle_apply_power(0, 0, PowerId::Strength, strength, state);
    }
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

    // Java BlockPerNonAttackAction uses addToTop in two loops: first
    // GainBlockAction for every card, then ExhaustSpecificCardAction for every
    // card. The resulting queue executes all exhausts before all block gains,
    // and each group is reversed relative to hand iteration order.
    for _ in &non_attacks {
        state.queue_action_front(Action::GainBlock {
            target: 0,
            amount: block_per_card,
        });
    }
    for uuid in &non_attacks {
        state.queue_action_front(Action::ExhaustCard {
            card_uuid: *uuid,
            source_pile: crate::state::PileType::Hand,
        });
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
        state.queue_action_front(Action::ExhaustCard {
            card_uuid: uuid,
            source_pile: crate::state::PileType::Hand,
        });
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
