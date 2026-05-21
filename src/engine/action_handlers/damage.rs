// action_handlers/damage.rs — Combat damage domain
//
// Handles: Damage, DamageAllEnemies, random-target damage, DropkickDamageAndEffect,
//          FiendFire, Feed, VampireDamage, VampireDamageAllEnemies,
//          LoseHp, GainBlock, GainBlockRandomMonster, LoseBlock, GainEnergy,
//          Heal, GainMaxHp, LoseMaxHp,
//          LimitBreak, BlockPerNonAttack, ExhaustAllNonAttack, ExhaustRandomCard

use crate::content::powers::store;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageInfo, DamageType, NO_SOURCE};
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
        queue_hp_lost_power_actions(state, power.power_type, hook_actions);
    }

    let relic_actions = crate::content::relics::hooks::on_lose_hp(state, amount);
    state.queue_actions(relic_actions);
}

fn queue_hp_lost_power_actions(
    state: &mut CombatState,
    power_id: PowerId,
    actions: smallvec::SmallVec<[Action; 2]>,
) {
    match power_id {
        PowerId::PlatedArmor => {
            for action in actions {
                state.queue_action_back(action);
            }
        }
        _ => {
            for action in actions.into_iter().rev() {
                state.queue_action_front(action);
            }
        }
    }
}

fn queue_red_skull_threshold_actions(state: &mut CombatState, previous_hp: i32, current_hp: i32) {
    let max_hp = state.entities.player.max_hp;
    let actions = crate::content::relics::red_skull::on_player_hp_changed(
        state,
        previous_hp,
        current_hp,
        max_hp,
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
        } else if card.id == crate::content::cards::CardId::MasterfulStab {
            card.update_cost_java(1);
        }
    }
}

fn target_receives_java_unique_kill_reward(state: &CombatState, target: usize) -> bool {
    state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == target)
        .is_some_and(|m| {
            (m.is_dying || m.current_hp <= 0)
                && !m.half_dead
                && !store::has_power(state, target, PowerId::Minion)
        })
}

fn monsters_are_basically_dead_for_post_combat(state: &CombatState) -> bool {
    state.are_monsters_basically_dead_java()
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
        if m.is_dying || m.is_escaped {
            return outcome;
        }

        let target_hp_before_damage = m.current_hp;
        let hp_loss_damage = info.damage_type == DamageType::HpLoss;
        let had_block = !hp_loss_damage && m.block > 0;
        if !hp_loss_damage {
            final_damage = deduct_block(&mut m.block, final_damage);
        }

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

        if source_id != NO_SOURCE {
            for power in &store::powers_snapshot_for(state, source_id) {
                let hook_actions = crate::content::powers::resolve_power_on_attack(
                    power.power_type,
                    source_id,
                    target_id,
                    final_damage,
                    info.damage_type,
                    power.amount,
                );
                for action in hook_actions.into_iter().rev() {
                    state.queue_action_front(action);
                }
            }
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
                if power.power_type == PowerId::Split {
                    // Java large slimes / Slime Boss call setMove(Split) immediately
                    // inside damage(), then add a SetMoveAction to the bottom. The
                    // immediate mutation prevents duplicate split interrupts during
                    // queued multi-hit attacks while preserving the existing queue.
                    // Large slimes also set their private splitTriggered flag
                    // immediately; that state is not itself a queued Java action.
                    for a in hook_actions {
                        super::execute_action(a.clone(), state);
                        if !matches!(
                            a,
                            Action::UpdateMonsterRuntime {
                                patch: crate::runtime::action::MonsterRuntimePatch::LargeSlime { .. },
                                ..
                            }
                        ) {
                            state.queue_action_back(a);
                        }
                    }
                } else {
                    queue_hp_lost_power_actions(state, power.power_type, hook_actions);
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
                PowerId::Malleable | PowerId::CurlUp | PowerId::Flight | PowerId::Reactive
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

fn damage_action_target_is_dead_or_escaped(state: &CombatState, target_id: usize) -> bool {
    if target_id == 0 {
        state.entities.player.current_hp <= 0
    } else {
        state
            .entities
            .monsters
            .iter()
            .find(|m| m.id == target_id)
            .map_or(true, |m| m.is_dead_or_escaped())
    }
}

fn damage_action_source_is_dying_or_half_dead(state: &CombatState, source_id: usize) -> bool {
    if source_id == NO_SOURCE {
        return false;
    }
    if source_id == 0 {
        return state.entities.player.current_hp <= 0;
    }

    state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == source_id)
        .is_some_and(|m| m.is_dying || m.half_dead)
}

fn should_cancel_java_damage_action(
    state: &CombatState,
    info: &crate::runtime::action::DamageInfo,
) -> bool {
    if info.damage_type == DamageType::Thorns {
        return false;
    }

    damage_action_target_is_dead_or_escaped(state, info.target)
        || damage_action_source_is_dying_or_half_dead(state, info.source)
}

fn calculate_damage_action_output(
    state: &CombatState,
    info: &crate::runtime::action::DamageInfo,
) -> (i32, bool) {
    let target_id = info.target;
    let source_id = info.source;

    // Damage contract:
    // - Player-origin Normal damage arrives pre-evaluated in `output`.
    // - Monster-origin Normal damage is re-resolved here from `base`.
    // - Non-Normal damage kinds (`HpLoss`, `Thorns`, etc.) use the supplied numeric value.
    if !info.is_modified
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
    }
}

pub fn handle_damage(info: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    if should_cancel_java_damage_action(state, &info) {
        return;
    }

    let target_id = info.target;
    let source_id = info.source;

    let (calculated_output, damage_already_includes_final_receive) =
        calculate_damage_action_output(state, &info);

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

fn pummel_source_is_dying(state: &CombatState, source_id: usize) -> bool {
    if source_id == 0 {
        state.entities.player.current_hp <= 0
    } else if source_id == NO_SOURCE {
        false
    } else {
        state
            .entities
            .monsters
            .iter()
            .find(|m| m.id == source_id)
            .is_some_and(|m| m.is_dying)
    }
}

fn target_current_hp_is_positive(state: &CombatState, target_id: usize) -> bool {
    if target_id == 0 {
        state.entities.player.current_hp > 0
    } else {
        state
            .entities
            .monsters
            .iter()
            .find(|m| m.id == target_id)
            .is_some_and(|m| m.current_hp > 0)
    }
}

pub fn handle_pummel_damage(info: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    if !target_current_hp_is_positive(state, info.target) {
        return;
    }
    if info.damage_type != DamageType::Thorns && pummel_source_is_dying(state, info.source) {
        return;
    }

    if info.target == 0 {
        handle_damage(info, state);
    } else {
        let final_damage = info.output.max(0);
        let _ = apply_damage_to_monster_via_pipeline(state, &info, final_damage);
        clear_post_combat_actions_if_ready(state);
    }
}

pub fn handle_bane_damage(info: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    if !store::has_power(state, info.target, PowerId::Poison) {
        return;
    }
    if !target_current_hp_is_positive(state, info.target) {
        return;
    }
    if info.damage_type != DamageType::Thorns && pummel_source_is_dying(state, info.source) {
        return;
    }

    if info.target == 0 {
        handle_damage(info, state);
    } else {
        let final_damage = info.output.max(0);
        let _ = apply_damage_to_monster_via_pipeline(state, &info, final_damage);
        clear_post_combat_actions_if_ready(state);
    }
}

pub fn handle_wallop_damage(info: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    if should_cancel_java_damage_action(state, &info) {
        return;
    }
    if info.target == 0 {
        handle_damage(info, state);
        return;
    }

    let (calculated_output, damage_already_includes_final_receive) =
        calculate_damage_action_output(state, &info);
    let mut final_damage = calculated_output;

    if !damage_already_includes_final_receive {
        for power in &store::powers_snapshot_for(state, info.target) {
            final_damage = crate::content::powers::resolve_power_at_damage_final_receive(
                power.power_type,
                final_damage,
                power.amount,
                info.damage_type,
            );
        }
    }

    let outcome = apply_damage_to_monster_via_pipeline(state, &info, final_damage);
    if outcome.hp_lost > 0 {
        state.queue_action_front(Action::GainBlock {
            target: info.source,
            amount: outcome.hp_lost,
        });
    }
    clear_post_combat_actions_if_ready(state);
}

pub fn handle_damage_per_attack_played(
    info: crate::runtime::action::DamageInfo,
    state: &mut CombatState,
) {
    if !target_current_hp_is_positive(state, info.target) {
        return;
    }

    let attack_count_before_finisher = state
        .turn
        .counters
        .attacks_played_this_turn
        .saturating_sub(1) as usize;
    for _ in 0..attack_count_before_finisher {
        state.queue_action_front(Action::Damage(info.clone()));
    }
}

pub fn handle_heel_hook(info: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    if store::has_power(state, info.target, PowerId::Weak) {
        state.queue_action_front(Action::DrawCards(1));
        state.queue_action_front(Action::GainEnergy { amount: 1 });
    }
    state.queue_action_front(Action::Damage(info));
}

pub fn handle_flechettes(info: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    let skill_count = state
        .zones
        .hand
        .iter()
        .filter(|card| {
            crate::content::cards::get_card_definition(card.id).card_type
                == crate::content::cards::CardType::Skill
        })
        .count();
    for _ in 0..skill_count {
        state.queue_action_front(Action::Damage(info.clone()));
    }
}

fn damage_type(
    kind: crate::runtime::monster_move::DamageKind,
) -> crate::runtime::action::DamageType {
    match kind {
        crate::runtime::monster_move::DamageKind::Normal => {
            crate::runtime::action::DamageType::Normal
        }
        crate::runtime::monster_move::DamageKind::Thorns => {
            crate::runtime::action::DamageType::Thorns
        }
        crate::runtime::monster_move::DamageKind::HpLoss => {
            crate::runtime::action::DamageType::HpLoss
        }
        crate::runtime::monster_move::DamageKind::Unknown => panic!("monster attack kind unknown"),
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
    damage_kind: crate::runtime::monster_move::DamageKind,
    state: &mut CombatState,
) {
    handle_damage(
        crate::runtime::action::DamageInfo {
            source,
            target,
            base: base_damage,
            output: base_damage,
            damage_type: damage_type(damage_kind),
            is_modified: !matches!(
                damage_kind,
                crate::runtime::monster_move::DamageKind::Normal
            ),
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
        if m.is_dead_or_escaped() {
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

fn orb_damage_amount_for_target(state: &CombatState, target: usize, base_damage: i32) -> i32 {
    if store::power_amount(state, target, PowerId::LockOn) > 0 {
        base_damage.saturating_mul(3) / 2
    } else {
        base_damage
    }
}

pub fn handle_orb_damage(source: usize, target: usize, base_damage: i32, state: &mut CombatState) {
    let output = orb_damage_amount_for_target(state, target, base_damage);
    handle_damage(
        crate::runtime::action::DamageInfo {
            source,
            target,
            base: base_damage,
            output,
            damage_type: DamageType::Thorns,
            is_modified: output != base_damage,
        },
        state,
    );
}

pub fn handle_orb_damage_random_enemy(source: usize, base_damage: i32, state: &mut CombatState) {
    let alive: Vec<usize> = state
        .entities
        .monsters
        .iter()
        .filter(|m| m.is_random_target_candidate())
        .map(|m| m.id)
        .collect();
    if alive.is_empty() {
        return;
    }
    let idx = state.rng.card_random_rng.random(alive.len() as i32 - 1) as usize;
    let target = alive[idx];
    let output = orb_damage_amount_for_target(state, target, base_damage);
    state.queue_action_front(Action::Damage(crate::runtime::action::DamageInfo {
        source,
        target,
        base: base_damage,
        output,
        damage_type: DamageType::Thorns,
        is_modified: output != base_damage,
    }));
}

pub fn handle_orb_damage_all_enemies(source: usize, base_damage: i32, state: &mut CombatState) {
    let mut individual_damages: smallvec::SmallVec<[Action; 5]> = smallvec::SmallVec::new();
    for monster in &state.entities.monsters {
        if monster.is_dead_or_escaped() {
            continue;
        }
        let output = orb_damage_amount_for_target(state, monster.id, base_damage);
        individual_damages.push(Action::Damage(crate::runtime::action::DamageInfo {
            source,
            target: monster.id,
            base: base_damage,
            output,
            damage_type: DamageType::Thorns,
            is_modified: output != base_damage,
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

pub fn handle_skewer(
    target: usize,
    damage_info: DamageInfo,
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
            let mut hit = damage_info.clone();
            hit.target = target;
            state.queue_action_back(Action::Damage(hit));
        }
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_sunder(
    target: usize,
    mut damage_info: DamageInfo,
    energy_gain: i32,
    state: &mut CombatState,
) {
    damage_info.target = target;
    let final_damage = damage_info.output.max(0);
    let _ = apply_damage_to_monster_via_pipeline(state, &damage_info, final_damage);
    let killed_or_zero_hp = state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target)
        .is_some_and(|monster| monster.is_dying || monster.current_hp <= 0);
    if killed_or_zero_hp && energy_gain > 0 {
        state.queue_action_back(Action::GainEnergy {
            amount: energy_gain,
        });
    }
    clear_post_combat_actions_if_ready(state);
}

pub fn handle_judgement(target: usize, cutoff: i32, state: &mut CombatState) {
    let should_kill = state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target)
        .is_some_and(|monster| monster.current_hp <= cutoff);
    if should_kill {
        state.queue_action_front(Action::InstantKill { target });
    }
}

pub fn handle_instant_kill(target: usize, state: &mut CombatState) {
    if target == 0 {
        state.entities.player.current_hp = 0;
        super::try_revive(state);
        return;
    }
    if let Some(monster) = state.entities.monsters.iter_mut().find(|m| m.id == target) {
        monster.current_hp = 0;
    }
    super::check_and_trigger_monster_death(state, target);
    clear_post_combat_actions_if_ready(state);
}

pub fn handle_trigger_marks(card_id: crate::content::cards::CardId, state: &mut CombatState) {
    if card_id != crate::content::cards::CardId::PressurePoints {
        return;
    }
    let targets: Vec<(usize, i32)> = state
        .entities
        .monsters
        .iter()
        .filter_map(|monster| {
            let amount = crate::content::powers::store::power_amount(
                state,
                monster.id,
                crate::content::powers::PowerId::MarkPower,
            );
            (amount > 0).then_some((monster.id, amount))
        })
        .collect();
    for (target, amount) in targets {
        state.queue_action_back(Action::LoseHp {
            target,
            amount,
            triggers_rupture: false,
        });
    }
}

pub fn handle_damage_random_enemy(
    source: usize,
    base_damage: i32,
    damage_type: DamageType,
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
        state.queue_action_front(Action::Damage(crate::runtime::action::DamageInfo {
            source,
            target: target_id,
            base: base_damage,
            output: base_damage,
            damage_type,
            is_modified: false,
        }));
    }
}

pub fn handle_attack_damage_random_enemy_card(
    card: crate::runtime::combat::CombatCard,
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
        let evaluated =
            crate::content::cards::evaluate_card_for_play(&card, state, Some(target_id));
        state.queue_action_front(Action::Damage(crate::runtime::action::DamageInfo {
            source: 0,
            target: target_id,
            base: evaluated.base_damage_mut,
            output: evaluated.base_damage_mut,
            damage_type: DamageType::Normal,
            is_modified: false,
        }));
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

pub fn handle_ftl(
    target: usize,
    damage_info: crate::runtime::action::DamageInfo,
    card_play_count: i32,
    state: &mut CombatState,
) {
    state.queue_action_back(Action::Damage(crate::runtime::action::DamageInfo {
        target,
        ..damage_info
    }));
    if (state.turn.counters.cards_played_this_turn as i32).saturating_sub(1) < card_play_count {
        state.queue_action_front(Action::DrawCards(1));
    }
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
    let _ = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
    if target_receives_java_unique_kill_reward(state, target) {
        increase_player_max_hp_like_java(max_hp_amount, state);
    }
    clear_post_combat_actions_if_ready(state);
}

pub fn handle_lesson_learned(
    target: usize,
    damage_info: crate::runtime::action::DamageInfo,
    state: &mut CombatState,
) {
    let mut info = damage_info;
    info.target = target;
    let _ = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
    if target_receives_java_unique_kill_reward(state, target) {
        let possible = state
            .meta
            .master_deck_snapshot
            .iter()
            .enumerate()
            .filter(|(_, card)| crate::content::cards::can_upgrade_card_once(card))
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();
        if !possible.is_empty() {
            let pick = state
                .rng
                .misc_rng
                .random_range(0, possible.len() as i32 - 1) as usize;
            let deck_idx = possible[pick];
            let card_uuid = state.meta.master_deck_snapshot[deck_idx].uuid;
            state.meta.master_deck_snapshot[deck_idx].upgrades += 1;
            state
                .meta
                .meta_changes
                .push(crate::runtime::combat::MetaChange::UpgradeMasterDeckCard { card_uuid });
        }
    }
    clear_post_combat_actions_if_ready(state);
}

pub fn handle_hand_of_greed(
    target: usize,
    damage_info: crate::runtime::action::DamageInfo,
    gold_amount: i32,
    state: &mut CombatState,
) {
    let mut info = damage_info;
    info.target = target;
    let _ = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
    if target_receives_java_unique_kill_reward(state, target) {
        handle_gain_gold(gold_amount, state);
    }
    clear_post_combat_actions_if_ready(state);
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
    let _ = apply_damage_to_monster_via_pipeline(state, &info, info.output.max(0));
    if target_receives_java_unique_kill_reward(state, target) {
        crate::engine::action_handlers::cards::handle_modify_card_misc(
            card_uuid,
            misc_amount,
            state,
        );
    }
    clear_post_combat_actions_if_ready(state);
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
            // Java VampireDamageAllEnemiesAction skips only isDying,
            // currentHealth <= 0, and isEscaping.  It does not consult
            // isDeadOrEscaped(), so `halfDead` is intentionally not a filter
            // here.
            if m.is_dying || m.current_hp <= 0 || m.is_escaped {
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
                super::try_revive(state);
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

pub fn handle_limit_break(state: &mut CombatState) {
    if let Some(strength) = store::powers_for(state, 0)
        .and_then(|powers| powers.iter().find(|p| p.power_type == PowerId::Strength))
        .map(|power| power.amount)
    {
        state.queue_action_front(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: strength,
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::action::{DamageInfo, DamageType};
    use crate::runtime::combat::{CombatCard, Power, PowerPayload};
    use crate::runtime::monster_move::{AttackSpec, DamageKind, MonsterMoveSpec};
    use crate::test_support::{blank_test_combat, test_monster};

    fn split_power() -> Power {
        Power {
            power_type: PowerId::Split,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }
    }

    fn normal_player_damage(target: usize, amount: i32) -> DamageInfo {
        DamageInfo {
            source: 0,
            target,
            base: amount,
            output: amount,
            damage_type: DamageType::Normal,
            is_modified: false,
        }
    }

    fn visible_attack_intent() -> MonsterMoveSpec {
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: DamageKind::Normal,
        })
    }

    #[test]
    fn gain_block_random_monster_does_not_filter_is_escaping_like_java() {
        let mut source = test_monster(EnemyId::Centurion);
        source.id = 1;
        source.set_planned_visible_spec(Some(visible_attack_intent()));
        let mut escaping_ally = test_monster(EnemyId::Healer);
        escaping_ally.id = 2;
        escaping_ally.is_escaped = true;
        escaping_ally.set_planned_visible_spec(Some(visible_attack_intent()));
        let mut state = crate::test_support::combat_with_monsters(vec![source, escaping_ally]);

        handle_gain_block_random_monster(1, 15, &mut state);

        assert_eq!(state.entities.monsters[0].block, 0);
        assert_eq!(
            state.entities.monsters[1].block, 15,
            "Java GainBlockRandomMonsterAction filters source, intent == ESCAPE, and isDying; it does not filter isEscaping"
        );
    }

    #[test]
    fn gain_block_random_monster_filters_escape_intent_like_java() {
        let mut source = test_monster(EnemyId::Centurion);
        source.id = 1;
        source.set_planned_visible_spec(Some(visible_attack_intent()));
        let mut escaping_intent_ally = test_monster(EnemyId::Looter);
        escaping_intent_ally.id = 2;
        escaping_intent_ally.set_planned_move_id(3);
        let mut state =
            crate::test_support::combat_with_monsters(vec![source, escaping_intent_ally]);

        handle_gain_block_random_monster(1, 15, &mut state);

        assert_eq!(
            state.entities.monsters[0].block, 15,
            "Java falls back to the source when every non-source monster has ESCAPE intent"
        );
        assert_eq!(state.entities.monsters[1].block, 0);
    }

    #[test]
    fn pummel_damage_action_skips_target_that_is_already_at_zero_hp() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 61;
        monster.current_hp = 0;
        monster.block = 4;
        monster.is_dying = false;
        state.entities.monsters = vec![monster];

        handle_pummel_damage(
            DamageInfo {
                source: 0,
                target: 61,
                base: 10,
                output: 10,
                damage_type: DamageType::Normal,
                is_modified: false,
            },
            &mut state,
        );

        let monster = &state.entities.monsters[0];
        assert_eq!(monster.current_hp, 0);
        assert_eq!(monster.block, 4);
        assert!(!monster.is_dying);
        assert_eq!(
            state.pop_next_action(),
            None,
            "Java PummelDamageAction checks target.currentHealth > 0 before damage and does not run death cleanup from this skipped hit"
        );
    }

    #[test]
    fn pummel_damage_action_applies_to_live_target() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 62;
        monster.current_hp = 12;
        monster.block = 2;
        state.entities.monsters = vec![monster];

        handle_pummel_damage(
            DamageInfo {
                source: 0,
                target: 62,
                base: 5,
                output: 5,
                damage_type: DamageType::Normal,
                is_modified: false,
            },
            &mut state,
        );

        let monster = &state.entities.monsters[0];
        assert_eq!(monster.block, 0);
        assert_eq!(monster.current_hp, 9);
    }

    #[test]
    fn pummel_damage_action_clears_post_combat_actions_after_killing_hit() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 63;
        monster.current_hp = 5;
        state.entities.monsters = vec![monster];
        state.queue_action_back(Action::DrawCards(1));
        state.queue_action_back(Action::Damage(DamageInfo {
            source: 0,
            target: 63,
            base: 1,
            output: 1,
            damage_type: DamageType::Normal,
            is_modified: false,
        }));

        handle_pummel_damage(
            DamageInfo {
                source: 0,
                target: 63,
                base: 5,
                output: 5,
                damage_type: DamageType::Normal,
                is_modified: false,
            },
            &mut state,
        );

        assert!(state.entities.monsters[0].is_dying);
        assert!(
            matches!(state.pop_next_action(), Some(Action::Damage(_))),
            "Java PummelDamageAction calls clearPostCombatActions after a killing hit, retaining only Java-retained post-combat actions"
        );
        assert_eq!(state.pop_next_action(), None);
    }

    #[test]
    fn killing_large_slime_does_not_queue_split_like_java_damage_override() {
        let mut state = blank_test_combat();
        let mut slime = test_monster(EnemyId::AcidSlimeL);
        slime.id = 81;
        slime.current_hp = 4;
        slime.max_hp = 70;
        slime.set_planned_move_id(1);
        state.entities.monsters = vec![slime];
        store::set_powers_for(&mut state, 81, vec![split_power()]);

        handle_damage(normal_player_damage(81, 10), &mut state);

        let slime = &state.entities.monsters[0];
        assert!(slime.is_dying);
        assert_eq!(
            slime.planned_move_id(),
            1,
            "Java split interrupt is guarded by !isDying after super.damage(info)"
        );
        assert_eq!(state.pop_next_action(), None);
    }

    #[test]
    fn reactive_power_reroll_is_added_to_bottom_like_java() {
        let mut state = blank_test_combat();
        let mut mass = test_monster(EnemyId::WrithingMass);
        mass.id = 91;
        mass.current_hp = 20;
        state.entities.monsters = vec![mass];
        store::set_powers_for(
            &mut state,
            91,
            vec![Power {
                power_type: PowerId::Reactive,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        state.queue_action_back(Action::DrawCards(1));

        handle_damage(normal_player_damage(91, 3), &mut state);

        assert_eq!(
            state.pop_next_action(),
            Some(Action::DrawCards(1)),
            "Java ReactivePower.addToBot leaves existing queued actions ahead of the reroll"
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::RollMonsterMove { monster_id: 91 })
        );
    }

    #[test]
    fn plated_armor_hp_loss_reduction_is_added_to_bottom_like_java() {
        let mut state = blank_test_combat();
        let mut parasite = test_monster(EnemyId::ShelledParasite);
        parasite.id = 92;
        parasite.current_hp = 20;
        state.entities.monsters = vec![parasite];
        store::set_powers_for(
            &mut state,
            92,
            vec![Power {
                power_type: PowerId::PlatedArmor,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        state.queue_action_back(Action::DrawCards(1));

        handle_damage(normal_player_damage(92, 3), &mut state);

        assert_eq!(state.entities.monsters[0].current_hp, 17);
        assert_eq!(
            state.pop_next_action(),
            Some(Action::DrawCards(1)),
            "Java PlatedArmorPower.wasHPLost uses addToBot, so existing queued actions stay ahead of ReducePowerAction"
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::ReducePower {
                target: 92,
                power_id: PowerId::PlatedArmor,
                amount: 1,
            })
        );
    }

    #[test]
    fn large_slime_split_sets_intent_immediately_but_keeps_existing_multi_hit_queue() {
        let mut state = blank_test_combat();
        let mut slime = test_monster(EnemyId::AcidSlimeL);
        slime.id = 82;
        slime.current_hp = 40;
        slime.max_hp = 70;
        slime.set_planned_move_id(1);
        state.entities.monsters = vec![slime];
        store::set_powers_for(&mut state, 82, vec![split_power()]);

        let hit = normal_player_damage(82, 6);
        state.queue_action_back(Action::PummelDamage(hit.clone()));
        state.queue_action_back(Action::PummelDamage(hit.clone()));

        handle_pummel_damage(hit, &mut state);

        assert_eq!(state.entities.monsters[0].current_hp, 34);
        assert_eq!(
            state.entities.monsters[0].planned_move_id(),
            3,
            "Java damage() calls setMove(Split) immediately when the threshold is crossed"
        );
        assert!(
            state.entities.monsters[0].large_slime.split_triggered,
            "Java large slimes set private splitTriggered immediately after queuing SetMoveAction"
        );

        let Some(first_queued) = state.pop_next_action() else {
            panic!("remaining queued PummelDamage should stay before queued SetMoveAction");
        };
        assert!(matches!(first_queued, Action::PummelDamage(_)));
        crate::engine::action_handlers::execute_action(first_queued, &mut state);

        let Some(second_queued) = state.pop_next_action() else {
            panic!("second queued PummelDamage should still be preserved");
        };
        assert!(matches!(second_queued, Action::PummelDamage(_)));

        let Some(set_move) = state.pop_next_action() else {
            panic!("Java also queues a SetMoveAction to the bottom after the immediate setMove");
        };
        assert!(matches!(
            set_move,
            Action::SetMonsterMove {
                monster_id: 82,
                next_move_byte: 3,
                ..
            }
        ));
        assert_eq!(
            state.pop_next_action(),
            None,
            "planned split move blocks duplicate split interrupts during queued multi-hit damage"
        );
    }

    #[test]
    fn large_slime_split_triggered_blocks_duplicate_interrupt_even_if_move_changes() {
        let mut state = blank_test_combat();
        let mut slime = test_monster(EnemyId::SpikeSlimeL);
        slime.id = 83;
        slime.current_hp = 30;
        slime.max_hp = 70;
        slime.set_planned_move_id(1);
        slime.large_slime.split_triggered = true;
        state.entities.monsters = vec![slime];
        store::set_powers_for(&mut state, 83, vec![split_power()]);

        handle_damage(normal_player_damage(83, 1), &mut state);

        assert_eq!(
            state.entities.monsters[0].planned_move_id(),
            1,
            "Java large slime damage() also checks private splitTriggered, not only nextMove != SPLIT"
        );
        assert_eq!(state.pop_next_action(), None);
    }

    #[test]
    fn wallop_damage_action_clears_post_combat_actions_after_killing_hit() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 64;
        monster.current_hp = 5;
        state.entities.monsters = vec![monster];
        state.queue_action_back(Action::DrawCards(1));
        state.queue_action_back(Action::Damage(DamageInfo {
            source: 0,
            target: 64,
            base: 1,
            output: 1,
            damage_type: DamageType::Normal,
            is_modified: false,
        }));

        handle_wallop_damage(
            DamageInfo {
                source: 0,
                target: 64,
                base: 5,
                output: 5,
                damage_type: DamageType::Normal,
                is_modified: false,
            },
            &mut state,
        );

        assert!(state.entities.monsters[0].is_dying);
        assert_eq!(
            state.pop_next_action(),
            Some(Action::GainBlock {
                target: 0,
                amount: 5,
            }),
            "Java WallopAction queues GainBlock for lastDamageTaken before post-combat cleanup"
        );
        assert!(
            matches!(state.pop_next_action(), Some(Action::Damage(_))),
            "Java WallopAction calls clearPostCombatActions after a killing hit"
        );
        assert_eq!(state.pop_next_action(), None);
    }

    #[test]
    fn vampire_damage_all_enemies_matches_java_half_dead_filter() {
        let mut state = blank_test_combat();
        state.entities.player.current_hp = 50;
        let mut half_dead = test_monster(EnemyId::Darkling);
        half_dead.id = 67;
        half_dead.current_hp = 10;
        half_dead.half_dead = true;
        half_dead.is_dying = false;
        half_dead.is_escaped = false;
        state.entities.monsters = vec![half_dead];

        handle_vampire_damage_all_enemies(
            0,
            smallvec::smallvec![2],
            DamageType::Normal,
            &mut state,
        );

        assert_eq!(
            state.entities.monsters[0].current_hp, 8,
            "Java VampireDamageAllEnemiesAction does not filter halfDead unless currentHealth <= 0"
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::Heal {
                target: 0,
                amount: 2
            })
        );
    }

    #[test]
    fn player_lose_hp_action_clears_post_combat_actions_like_java() {
        let mut state = blank_test_combat();
        state.entities.monsters.clear();
        state.queue_action_back(Action::DrawCards(1));
        state.queue_action_back(Action::Damage(DamageInfo {
            source: 0,
            target: 64,
            base: 1,
            output: 1,
            damage_type: DamageType::Normal,
            is_modified: false,
        }));

        handle_lose_hp(0, 3, true, &mut state);

        assert_eq!(state.entities.player.current_hp, 77);
        assert!(
            matches!(state.pop_next_action(), Some(Action::Damage(_))),
            "Java LoseHPAction checks clearPostCombatActions even when the target is the player"
        );
        assert_eq!(state.pop_next_action(), None);
    }

    #[test]
    fn monster_lose_hp_action_skips_dying_and_escaping_like_java_damage() {
        for (is_dying, is_escaped) in [(true, false), (false, true)] {
            let mut state = blank_test_combat();
            let mut monster = test_monster(EnemyId::JawWorm);
            monster.id = 71;
            monster.current_hp = 20;
            monster.is_dying = is_dying;
            monster.is_escaped = is_escaped;
            state.entities.monsters = vec![monster];

            handle_lose_hp(71, 7, false, &mut state);

            assert_eq!(
                state.entities.monsters[0].current_hp, 20,
                "Java AbstractMonster.damage returns before HP_LOSS when the target is isDying or isEscaping"
            );
            assert_eq!(state.pop_next_action(), None);
        }
    }

    #[test]
    fn monster_lose_hp_action_ignores_block_but_respects_invincible_like_java() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::CorruptHeart);
        monster.id = 72;
        monster.current_hp = 300;
        monster.max_hp = 300;
        monster.block = 30;
        state.entities.monsters = vec![monster];
        store::set_powers_for(
            &mut state,
            72,
            vec![Power {
                power_type: PowerId::Invincible,
                instance_id: None,
                amount: 5,
                extra_data: 5,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_lose_hp(72, 20, false, &mut state);

        let monster = &state.entities.monsters[0];
        assert_eq!(monster.block, 30, "Java HP_LOSS does not decrement block");
        assert_eq!(
            monster.current_hp, 295,
            "Java InvinciblePower.onAttackedToChangeDamage caps HP_LOSS as well as ordinary damage"
        );
        assert_eq!(store::power_amount(&state, 72, PowerId::Invincible), 0);
    }

    #[test]
    fn ordinary_monster_damage_respects_invincible_like_java() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::CorruptHeart);
        monster.id = 73;
        monster.current_hp = 300;
        monster.max_hp = 300;
        state.entities.monsters = vec![monster];
        store::set_powers_for(
            &mut state,
            73,
            vec![Power {
                power_type: PowerId::Invincible,
                instance_id: None,
                amount: 6,
                extra_data: 6,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_damage(
            DamageInfo {
                source: 0,
                target: 73,
                base: 50,
                output: 50,
                damage_type: DamageType::Normal,
                is_modified: false,
            },
            &mut state,
        );

        assert_eq!(state.entities.monsters[0].current_hp, 294);
        assert_eq!(store::power_amount(&state, 73, PowerId::Invincible), 0);
    }

    #[test]
    fn poison_lose_hp_decrements_poison_before_post_combat_cleanup() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 70;
        monster.current_hp = 3;
        state.entities.monsters = vec![monster];
        store::set_powers_for(
            &mut state,
            70,
            vec![Power {
                power_type: PowerId::Poison,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target: 70,
            power_id: PowerId::Weak,
            amount: 1,
        });

        handle_poison_lose_hp(70, 3, &mut state);

        assert!(state.entities.monsters[0].is_dying);
        assert_eq!(
            store::power_amount(&state, 70, PowerId::Poison),
            1,
            "Java PoisonLoseHpAction decrements Poison before clearPostCombatActions"
        );
        assert_eq!(
            state.pop_next_action(),
            None,
            "non-retained queued actions are still cleared after the atomic PoisonLoseHpAction"
        );
    }

    #[test]
    fn post_combat_cleanup_uses_java_basically_dead_not_zero_hp() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 69;
        monster.current_hp = 0;
        monster.is_dying = false;
        monster.is_escaped = false;
        state.entities.monsters = vec![monster];
        state.queue_action_back(Action::DrawCards(1));

        handle_lose_hp(0, 3, true, &mut state);

        assert_eq!(state.entities.player.current_hp, 77);
        assert_eq!(
            state.pop_next_action(),
            Some(Action::DrawCards(1)),
            "Java MonsterGroup.areMonstersBasicallyDead ignores currentHealth; only isDying/isEscaping count"
        );
    }

    #[test]
    fn damage_all_enemies_does_not_skip_zero_hp_target_unless_dead_or_escaped() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 65;
        monster.current_hp = 0;
        monster.is_dying = false;
        monster.is_escaped = false;
        monster.half_dead = false;
        state.entities.monsters = vec![monster];

        handle_damage_all_enemies(
            0,
            smallvec::smallvec![7],
            DamageType::Normal,
            false,
            &mut state,
        );

        assert!(
            matches!(
                state.pop_next_action(),
                Some(Action::Damage(DamageInfo {
                    target: 65,
                    output: 7,
                    ..
                }))
            ),
            "Java DamageAllEnemiesAction damage loop skips isDeadOrEscaped(), not currentHealth <= 0"
        );
    }

    #[test]
    fn normal_damage_action_cancels_against_dead_or_escaped_target() {
        for (is_dying, half_dead, is_escaped) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let mut state = blank_test_combat();
            let mut monster = test_monster(EnemyId::JawWorm);
            monster.id = 63;
            monster.current_hp = 20;
            monster.block = 3;
            monster.is_dying = is_dying;
            monster.half_dead = half_dead;
            monster.is_escaped = is_escaped;
            state.entities.monsters = vec![monster];

            handle_damage(
                DamageInfo {
                    source: 0,
                    target: 63,
                    base: 10,
                    output: 10,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                },
                &mut state,
            );

            let monster = &state.entities.monsters[0];
            assert_eq!(monster.current_hp, 20);
            assert_eq!(monster.block, 3);
            assert_eq!(
                state.pop_next_action(),
                None,
                "Java DamageAction.shouldCancelAction returns before damage when target.isDeadOrEscaped()"
            );
        }
    }

    #[test]
    fn normal_damage_action_cancels_when_source_is_dying_or_half_dead() {
        for (is_dying, half_dead) in [(true, false), (false, true)] {
            let mut state = blank_test_combat();
            state.entities.player.current_hp = 30;
            let mut source = test_monster(EnemyId::JawWorm);
            source.id = 64;
            source.is_dying = is_dying;
            source.half_dead = half_dead;
            state.entities.monsters = vec![source];

            handle_damage(
                DamageInfo {
                    source: 64,
                    target: 0,
                    base: 7,
                    output: 7,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                },
                &mut state,
            );

            assert_eq!(
                state.entities.player.current_hp, 30,
                "Java DamageAction returns before damage when info.owner is dying or halfDead"
            );
        }
    }

    #[test]
    fn thorns_damage_action_does_not_use_dead_or_escaped_cancel_guard() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 65;
        monster.current_hp = 20;
        monster.block = 0;
        monster.half_dead = true;
        state.entities.monsters = vec![monster];

        handle_damage(
            DamageInfo {
                source: 0,
                target: 65,
                base: 6,
                output: 6,
                damage_type: DamageType::Thorns,
                is_modified: false,
            },
            &mut state,
        );

        assert_eq!(
            state.entities.monsters[0].current_hp, 14,
            "Java DamageAction bypasses shouldCancelAction for THORNS damage"
        );
    }

    fn player_damage(target: usize) -> DamageInfo {
        DamageInfo {
            source: 0,
            target,
            base: 10,
            output: 10,
            damage_type: DamageType::Normal,
            is_modified: false,
        }
    }

    #[test]
    fn feed_action_rewards_already_dying_target_like_java_unique_action() {
        let mut state = blank_test_combat();
        let mut target = test_monster(EnemyId::JawWorm);
        target.id = 66;
        target.current_hp = 0;
        target.is_dying = true;
        state.entities.monsters = vec![target];

        handle_feed(66, player_damage(66), 3, &mut state);

        assert_eq!(state.entities.player.max_hp, 83);
        assert_eq!(
            state.entities.player.current_hp, 83,
            "Java FeedAction does not use shouldCancelAction; after target.damage returns for isDying, the reward condition still passes"
        );
    }

    #[test]
    fn feed_max_hp_reward_uses_java_increase_max_hp_heal_hooks() {
        let mut state = blank_test_combat();
        let mut target = test_monster(EnemyId::JawWorm);
        target.id = 68;
        target.current_hp = 0;
        target.is_dying = true;
        state.entities.monsters = vec![target];
        state.entities.player.max_hp = 80;
        state.entities.player.current_hp = 10;
        state
            .entities
            .player
            .add_relic(RelicState::new(RelicId::MagicFlower));

        handle_feed(68, player_damage(68), 5, &mut state);

        assert_eq!(state.entities.player.max_hp, 85);
        assert_eq!(
            state.entities.player.current_hp, 18,
            "Java FeedAction calls player.increaseMaxHp, whose internal heal goes through Magic Flower in combat"
        );
    }

    #[test]
    fn greed_action_still_blocks_already_dying_minion_reward() {
        let mut state = blank_test_combat();
        let starting_gold = state.entities.player.gold;
        let mut target = test_monster(EnemyId::JawWorm);
        target.id = 67;
        target.current_hp = 0;
        target.is_dying = true;
        state.entities.monsters = vec![target];
        crate::content::powers::store::set_powers_for(
            &mut state,
            67,
            vec![Power {
                power_type: PowerId::Minion,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_hand_of_greed(67, player_damage(67), 20, &mut state);

        assert_eq!(
            state.entities.player.gold, starting_gold,
            "Java GreedAction reward condition still excludes targets with Minion power"
        );
    }

    #[test]
    fn ritual_dagger_action_rewards_already_dying_target_like_java_unique_action() {
        let mut state = blank_test_combat();
        let mut target = test_monster(EnemyId::JawWorm);
        target.id = 68;
        target.current_hp = 0;
        target.is_dying = true;
        state.entities.monsters = vec![target];
        state.zones.hand = vec![CombatCard::new(CardId::RitualDagger, 680)];
        state.zones.limbo = vec![CombatCard::new(CardId::RitualDagger, 680)];

        handle_ritual_dagger(68, player_damage(68), 3, 680, &mut state);

        assert_eq!(state.zones.hand[0].misc_value, 18);
        assert_eq!(state.zones.limbo[0].misc_value, 18);
        assert_eq!(
            state.meta.meta_changes,
            vec![crate::runtime::combat::MetaChange::ModifyCardMisc {
                card_uuid: 680,
                amount: 3,
            }],
            "Java RitualDaggerAction applies the reward from its own post-damage target state, even when damage() returned because the target was already isDying"
        );
    }

    #[test]
    fn lesson_learned_action_upgrades_master_deck_snapshot_and_emits_meta_change() {
        let mut state = blank_test_combat();
        let mut target = test_monster(EnemyId::JawWorm);
        target.id = 69;
        target.current_hp = 0;
        target.is_dying = true;
        state.entities.monsters = vec![target];
        let mut upgraded_defend = CombatCard::new(CardId::Defend, 691);
        upgraded_defend.upgrades = 1;
        state.meta.master_deck_snapshot =
            vec![CombatCard::new(CardId::Strike, 690), upgraded_defend];

        handle_lesson_learned(69, player_damage(69), &mut state);

        assert_eq!(state.meta.master_deck_snapshot[0].upgrades, 1);
        assert_eq!(
            state.meta.meta_changes,
            vec![crate::runtime::combat::MetaChange::UpgradeMasterDeckCard { card_uuid: 690 }],
            "Java LessonLearnedAction upgrades a random canUpgrade() card from player.masterDeck"
        );
    }

    #[test]
    fn lesson_learned_action_does_not_upgrade_minion_kill_like_java() {
        let mut state = blank_test_combat();
        let mut target = test_monster(EnemyId::JawWorm);
        target.id = 70;
        target.current_hp = 0;
        target.is_dying = true;
        state.entities.monsters = vec![target];
        crate::content::powers::store::set_powers_for(
            &mut state,
            70,
            vec![Power {
                power_type: PowerId::Minion,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state.meta.master_deck_snapshot = vec![CombatCard::new(CardId::Strike, 700)];

        handle_lesson_learned(70, player_damage(70), &mut state);

        assert_eq!(state.meta.master_deck_snapshot[0].upgrades, 0);
        assert!(state.meta.meta_changes.is_empty());
    }
}
