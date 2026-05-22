// action_handlers/damage.rs — Combat damage domain
//
// Handles: Damage, DamageAllEnemies, random-target damage, DropkickDamageAndEffect,
//          FiendFire, Feed, VampireDamage, VampireDamageAllEnemies,
//          LoseHp, GainBlock, GainBlockRandomMonster, LoseBlock, GainEnergy,
//          Heal, GainMaxHp, LoseMaxHp,
//          LimitBreak, BlockPerNonAttack, ExhaustAllNonAttack, ExhaustRandomCard

mod core;
mod gold;
mod health;
mod vampire;

use crate::content::powers::store;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageInfo, DamageType, NO_SOURCE};
use crate::runtime::combat::CombatState;
use core::{
    apply_damage_to_monster_via_pipeline, calculate_damage_action_output,
    clear_post_combat_actions_if_ready, should_cancel_java_damage_action,
    target_receives_java_unique_kill_reward,
};
pub use core::{apply_raw_damage_to_monster, deduct_block, handle_damage};
pub use gold::{handle_gain_gold, handle_steal_player_gold};
pub use health::{
    handle_double_block, handle_gain_block, handle_gain_block_random_monster, handle_heal,
    handle_lose_block, handle_lose_hp, handle_poison_lose_hp, handle_remove_all_block,
    handle_set_current_hp, increase_player_max_hp_like_java,
};
pub use vampire::{handle_vampire_damage, handle_vampire_damage_all_enemies};
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
