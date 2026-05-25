use super::*;
use crate::content::powers::store::powers_snapshot_for;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::CombatCard;
use crate::sim::combat_projection::project_monster_move_preview_in_combat;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PlayCardEffectSummary {
    pub(super) persistent_enemy_strength_down: i32,
    pub(super) temporary_enemy_strength_down: i32,
    pub(super) visible_attack_mitigation_hint: i32,
    pub(super) enemy_strength_gain: i32,
    pub(super) visible_attack_pressure_hint: i32,
    pub(super) reactive_player_hp_loss: i32,
    pub(super) reactive_player_block: i32,
    pub(super) reactive_enemy_damage: i32,
    pub(super) reactive_bad_draw_cards: i32,
    pub(super) reactive_forced_turn_end: bool,
    pub(super) enemy_weak: i32,
    pub(super) enemy_vulnerable: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PlayCardEffectDiagnostics {
    pub(super) mitigation_score: i32,
    pub(super) reactive_risk_score: i32,
    pub(super) enemy_strength_gain: i32,
    pub(super) visible_attack_pressure_hint: i32,
    pub(super) reactive_player_hp_loss: i32,
    pub(super) reactive_player_block: i32,
    pub(super) reactive_enemy_damage: i32,
    pub(super) reactive_bad_draw_cards: i32,
    pub(super) reactive_forced_turn_end: bool,
}

impl PlayCardEffectSummary {
    pub(super) fn mitigation_ordering_score(self) -> i32 {
        self.persistent_enemy_strength_down
            .saturating_add(self.temporary_enemy_strength_down)
            .saturating_add(self.visible_attack_mitigation_hint)
    }

    pub(super) fn enemy_scaling_risk_score(self) -> i32 {
        self.enemy_strength_gain
            .saturating_add(self.visible_attack_pressure_hint)
    }

    pub(super) fn reactive_risk_score(self) -> i32 {
        self.enemy_scaling_risk_score()
            .saturating_add(self.reactive_player_hp_loss)
            .saturating_add(self.reactive_bad_draw_cards)
            .saturating_add(i32::from(self.reactive_forced_turn_end))
    }

    pub(super) fn net_mitigation_ordering_score(self) -> i32 {
        self.mitigation_ordering_score()
            .saturating_sub(self.reactive_risk_score())
    }

    pub(super) fn diagnostics(self) -> PlayCardEffectDiagnostics {
        PlayCardEffectDiagnostics {
            mitigation_score: self.mitigation_ordering_score(),
            reactive_risk_score: self.reactive_risk_score(),
            enemy_strength_gain: self.enemy_strength_gain,
            visible_attack_pressure_hint: self.visible_attack_pressure_hint,
            reactive_player_hp_loss: self.reactive_player_hp_loss,
            reactive_player_block: self.reactive_player_block,
            reactive_enemy_damage: self.reactive_enemy_damage,
            reactive_bad_draw_cards: self.reactive_bad_draw_cards,
            reactive_forced_turn_end: self.reactive_forced_turn_end,
        }
    }
}

impl PlayCardEffectDiagnostics {
    pub(super) fn has_reactive_signal(self) -> bool {
        self.reactive_risk_score > 0
            || self.reactive_player_block > 0
            || self.reactive_enemy_damage > 0
            || self.mitigation_score > 0
    }
}

pub(super) fn summarize_play_card_effects(
    combat: &CombatState,
    card: &CombatCard,
    target: Option<usize>,
) -> PlayCardEffectSummary {
    let actions = crate::content::cards::resolve_card_play_with_context(
        card.id,
        combat,
        card,
        target,
        crate::content::cards::CardUseContext {
            played_from_hand: true,
        },
    );
    let mut raw = RawPowerEffects::default();

    for info in actions {
        observe_power_action(&mut raw, info.action);
    }
    observe_card_play_reactive_power_actions(combat, card, &mut raw);

    summarize_power_effects(combat, raw)
}

pub(super) fn state_sustained_mitigation_score(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| {
            let strength = combat.get_power(monster.id, PowerId::Strength);
            if strength >= 0 {
                return 0;
            }
            (-strength).saturating_mul(monster_attack_relevance(combat, monster.id))
        })
        .sum()
}

#[derive(Default)]
struct RawPowerEffects {
    enemy_strength_down_by_target: BTreeMap<usize, i32>,
    enemy_strength_gain_by_target: BTreeMap<usize, i32>,
    shackled_targets: BTreeSet<usize>,
    reactive_player_hp_loss: i32,
    reactive_player_block: i32,
    reactive_enemy_damage: i32,
    reactive_bad_draw_cards: i32,
    reactive_forced_turn_end: bool,
    enemy_weak: i32,
    enemy_vulnerable: i32,
}

fn observe_power_action(raw: &mut RawPowerEffects, action: Action) {
    match action {
        Action::ApplyPower {
            target,
            power_id,
            amount,
            ..
        }
        | Action::ApplyPowerDetailed {
            target,
            power_id,
            amount,
            ..
        }
        | Action::ApplyPowerWithPayload {
            target,
            power_id,
            amount,
            ..
        } => observe_apply_power(raw, target, power_id, amount),
        _ => {}
    }
}

fn observe_apply_power(raw: &mut RawPowerEffects, target: usize, power_id: PowerId, amount: i32) {
    match power_id {
        PowerId::Strength if amount < 0 => {
            *raw.enemy_strength_down_by_target.entry(target).or_default() += -amount;
        }
        PowerId::Strength if amount > 0 => {
            *raw.enemy_strength_gain_by_target.entry(target).or_default() += amount;
        }
        PowerId::Shackled if amount > 0 => {
            raw.shackled_targets.insert(target);
        }
        PowerId::Weak if amount > 0 => {
            raw.enemy_weak = raw.enemy_weak.saturating_add(amount);
        }
        PowerId::Vulnerable if amount > 0 => {
            raw.enemy_vulnerable = raw.enemy_vulnerable.saturating_add(amount);
        }
        _ => {}
    }
}

fn observe_card_play_reactive_power_actions(
    combat: &CombatState,
    card: &CombatCard,
    raw: &mut RawPowerEffects,
) {
    let trigger_owners = std::iter::once(0usize)
        .chain(combat.entities.monsters.iter().map(|monster| monster.id))
        .collect::<Vec<_>>();
    for owner in trigger_owners {
        for power in powers_snapshot_for(combat, owner) {
            let actions = crate::content::powers::resolve_power_on_card_played(
                power.power_type,
                combat,
                owner,
                card,
                power.amount,
            );
            for action in actions {
                observe_reactive_action(combat, raw, action);
            }
        }
    }
}

fn observe_reactive_action(combat: &CombatState, raw: &mut RawPowerEffects, action: Action) {
    observe_power_action(raw, action.clone());
    match action {
        Action::Damage(info)
        | Action::PummelDamage(info)
        | Action::BaneDamage(info)
        | Action::WallopDamage(info)
        | Action::DamagePerAttackPlayed(info)
        | Action::DropkickDamageAndEffect {
            damage_info: info, ..
        }
        | Action::Ftl {
            damage_info: info, ..
        }
        | Action::Skewer {
            damage_info: info, ..
        }
        | Action::VampireDamage(info)
        | Action::Barrage { damage: info }
        | Action::Sunder {
            damage_info: info, ..
        }
        | Action::FearNoEvil {
            damage_info: info, ..
        }
        | Action::FiendFire {
            damage_info: info, ..
        }
        | Action::Feed {
            damage_info: info, ..
        }
        | Action::LessonLearned {
            damage_info: info, ..
        }
        | Action::HandOfGreed {
            damage_info: info, ..
        }
        | Action::RitualDagger {
            damage_info: info, ..
        } => observe_reactive_damage_info(combat, raw, info.target, info.output.max(info.base)),
        Action::LoseHp { target, amount, .. } | Action::PoisonLoseHp { target, amount } => {
            observe_reactive_hp_loss(combat, raw, target, amount)
        }
        Action::DamageAllEnemies { damages, .. }
        | Action::VampireDamageAllEnemies { damages, .. } => {
            for (slot, damage) in damages.into_iter().enumerate() {
                if let Some(monster) = combat.entities.monsters.get(slot) {
                    observe_reactive_hp_loss(combat, raw, monster.id, damage);
                }
            }
        }
        Action::GainBlock { target, amount } if target == 0 => {
            raw.reactive_player_block = raw.reactive_player_block.saturating_add(amount.max(0));
        }
        Action::MakeTempCardInDrawPile {
            card_id, amount, ..
        } => {
            if generated_card_is_bad_draw(card_id) {
                raw.reactive_bad_draw_cards = raw
                    .reactive_bad_draw_cards
                    .saturating_add(i32::from(amount));
            }
        }
        Action::TriggerTimeWarpEndTurn { .. } => {
            raw.reactive_forced_turn_end = true;
        }
        _ => {}
    }
}

fn observe_reactive_damage_info(
    combat: &CombatState,
    raw: &mut RawPowerEffects,
    target: usize,
    amount: i32,
) {
    observe_reactive_hp_loss(combat, raw, target, amount);
}

fn observe_reactive_hp_loss(
    combat: &CombatState,
    raw: &mut RawPowerEffects,
    target: usize,
    amount: i32,
) {
    let amount = amount.max(0);
    if amount == 0 {
        return;
    }
    if target == 0 {
        raw.reactive_player_hp_loss = raw.reactive_player_hp_loss.saturating_add(amount);
    } else if is_living_monster_id(combat, target) {
        raw.reactive_enemy_damage = raw.reactive_enemy_damage.saturating_add(amount);
    }
}

fn generated_card_is_bad_draw(card_id: crate::content::cards::CardId) -> bool {
    let def = crate::content::cards::get_card_definition(card_id);
    matches!(
        def.card_type,
        crate::content::cards::CardType::Status | crate::content::cards::CardType::Curse
    )
}

fn summarize_power_effects(combat: &CombatState, raw: RawPowerEffects) -> PlayCardEffectSummary {
    let mut summary = PlayCardEffectSummary {
        reactive_player_hp_loss: raw.reactive_player_hp_loss,
        reactive_player_block: raw.reactive_player_block,
        reactive_enemy_damage: raw.reactive_enemy_damage,
        reactive_bad_draw_cards: raw.reactive_bad_draw_cards,
        reactive_forced_turn_end: raw.reactive_forced_turn_end,
        enemy_weak: raw.enemy_weak,
        enemy_vulnerable: raw.enemy_vulnerable,
        ..PlayCardEffectSummary::default()
    };

    for (target, amount) in raw.enemy_strength_down_by_target {
        if !is_living_monster_id(combat, target) {
            continue;
        }
        let weighted_amount = amount.saturating_mul(monster_attack_relevance(combat, target));
        if raw.shackled_targets.contains(&target) {
            summary.temporary_enemy_strength_down = summary
                .temporary_enemy_strength_down
                .saturating_add(weighted_amount);
        } else {
            summary.persistent_enemy_strength_down = summary
                .persistent_enemy_strength_down
                .saturating_add(weighted_amount);
        }
        summary.visible_attack_mitigation_hint =
            summary.visible_attack_mitigation_hint.saturating_add(
                visible_strength_down_mitigation_hint(combat, target, amount),
            );
    }
    for (target, amount) in raw.enemy_strength_gain_by_target {
        if !is_living_monster_id(combat, target) {
            continue;
        }
        let weighted_amount = amount.saturating_mul(monster_attack_relevance(combat, target));
        summary.enemy_strength_gain = summary.enemy_strength_gain.saturating_add(weighted_amount);
        summary.visible_attack_pressure_hint = summary
            .visible_attack_pressure_hint
            .saturating_add(visible_strength_gain_pressure_hint(combat, target, amount));
    }

    summary
}

fn visible_strength_down_mitigation_hint(
    combat: &CombatState,
    target: usize,
    strength_down: i32,
) -> i32 {
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target && monster.is_alive_for_action())
    else {
        return 0;
    };
    let preview = project_monster_move_preview_in_combat(combat, monster);
    let Some(damage_per_hit) = preview.damage_per_hit else {
        return 0;
    };
    let per_hit = strength_down.min(damage_per_hit).max(0);
    per_hit.saturating_mul(preview.hits.max(1) as i32)
}

fn visible_strength_gain_pressure_hint(
    combat: &CombatState,
    target: usize,
    strength_gain: i32,
) -> i32 {
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target && monster.is_alive_for_action())
    else {
        return 0;
    };
    let preview = project_monster_move_preview_in_combat(combat, monster);
    if preview.damage_per_hit.is_none() {
        return 0;
    }
    strength_gain
        .max(0)
        .saturating_mul(preview.hits.max(1) as i32)
}

fn monster_attack_relevance(combat: &CombatState, target: usize) -> i32 {
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target && monster.is_alive_for_action())
    else {
        return 0;
    };
    let preview = project_monster_move_preview_in_combat(combat, monster);
    if preview.hits > 0 {
        preview.hits as i32
    } else {
        1
    }
}

fn is_living_monster_id(combat: &CombatState, target: usize) -> bool {
    combat
        .entities
        .monsters
        .iter()
        .any(|monster| monster.id == target && monster.is_alive_for_action())
}

#[cfg(test)]
mod tests;
