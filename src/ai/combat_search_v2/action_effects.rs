use super::*;
use crate::runtime::combat::CombatCard;

mod power_signals;

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
    power_signals::summarize_play_card_power_effects(combat, card, target)
}

pub(super) fn state_sustained_mitigation_score(combat: &CombatState) -> i32 {
    power_signals::state_sustained_mitigation_score(combat)
}

#[cfg(test)]
mod tests;
