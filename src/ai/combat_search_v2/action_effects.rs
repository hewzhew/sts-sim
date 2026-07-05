use super::*;
use crate::runtime::combat::CombatCard;

mod card_play_effects;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CardPlayEffectFacts {
    pub(super) direct: DirectCardPlayEffectFacts,
    pub(super) reactive: ReactiveCardPlayEffectFacts,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct DirectCardPlayEffectFacts {
    pub(super) persistent_enemy_strength_down: i32,
    pub(super) temporary_enemy_strength_down: i32,
    pub(super) visible_attack_mitigation_hint: i32,
    pub(super) enemy_strength_gain: i32,
    pub(super) visible_attack_pressure_hint: i32,
    pub(super) player_strength_gain: i32,
    pub(super) player_temporary_strength_gain: i32,
    pub(super) declared_draw_cards: i32,
    pub(super) conditional_draw_cards: i32,
    pub(super) enemy_weak: i32,
    pub(super) enemy_vulnerable: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ReactiveCardPlayEffectFacts {
    pub(super) player_hp_loss: i32,
    pub(super) player_block: i32,
    pub(super) enemy_damage: i32,
    pub(super) bad_draw_cards: i32,
    pub(super) forced_turn_end: bool,
    pub(super) enemy_strength_gain: i32,
    pub(super) visible_attack_pressure_hint: i32,
    pub(super) enemy_weak: i32,
    pub(super) enemy_vulnerable: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CardPlayDerivedEffectScores {
    pub(super) mitigation_score: i32,
    pub(super) enemy_scaling_risk_score: i32,
    pub(super) reactive_risk_score: i32,
    pub(super) net_mitigation_score: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CardPlayEffectDiagnostics {
    pub(super) derived: CardPlayDerivedEffectScores,
    pub(super) direct: CardPlayDirectEffectDiagnostics,
    pub(super) reactive: CardPlayReactiveEffectDiagnostics,
    pub(super) access: CardPlayAccessEffectDiagnostics,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CardPlayDirectEffectDiagnostics {
    pub(super) persistent_enemy_strength_down: i32,
    pub(super) temporary_enemy_strength_down: i32,
    pub(super) visible_attack_mitigation_hint: i32,
    pub(super) enemy_weak: i32,
    pub(super) enemy_vulnerable: i32,
    pub(super) enemy_strength_gain: i32,
    pub(super) visible_attack_pressure_hint: i32,
    pub(super) player_strength_gain: i32,
    pub(super) player_temporary_strength_gain: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CardPlayReactiveEffectDiagnostics {
    pub(super) player_hp_loss: i32,
    pub(super) player_block: i32,
    pub(super) enemy_damage: i32,
    pub(super) bad_draw_cards: i32,
    pub(super) forced_turn_end: bool,
    pub(super) enemy_strength_gain: i32,
    pub(super) visible_attack_pressure_hint: i32,
    pub(super) enemy_weak: i32,
    pub(super) enemy_vulnerable: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CardPlayAccessEffectDiagnostics {
    pub(super) declared_draw_cards: i32,
    pub(super) conditional_draw_cards: i32,
    pub(super) total_draw_cards: i32,
}

impl CardPlayEffectFacts {
    pub(super) fn mitigation_ordering_score(self) -> i32 {
        self.direct
            .persistent_enemy_strength_down
            .saturating_add(self.direct.temporary_enemy_strength_down)
            .saturating_add(self.direct.visible_attack_mitigation_hint)
    }

    pub(super) fn enemy_scaling_risk_score(self) -> i32 {
        self.direct
            .enemy_strength_gain
            .saturating_add(self.reactive.enemy_strength_gain)
            .saturating_add(self.direct.visible_attack_pressure_hint)
            .saturating_add(self.reactive.visible_attack_pressure_hint)
    }

    pub(super) fn reactive_risk_score(self) -> i32 {
        self.enemy_scaling_risk_score()
            .saturating_add(self.reactive.player_hp_loss)
            .saturating_add(self.reactive.bad_draw_cards)
            .saturating_add(i32::from(self.reactive.forced_turn_end))
    }

    pub(super) fn net_mitigation_ordering_score(self) -> i32 {
        self.mitigation_ordering_score()
            .saturating_sub(self.reactive_risk_score())
    }

    pub(super) fn derived_scores(self) -> CardPlayDerivedEffectScores {
        CardPlayDerivedEffectScores {
            mitigation_score: self.mitigation_ordering_score(),
            enemy_scaling_risk_score: self.enemy_scaling_risk_score(),
            reactive_risk_score: self.reactive_risk_score(),
            net_mitigation_score: self.net_mitigation_ordering_score(),
        }
    }

    pub(super) fn total_draw_cards(self) -> i32 {
        self.direct
            .declared_draw_cards
            .saturating_add(self.direct.conditional_draw_cards)
    }

    pub(super) fn has_future_debuff(self) -> bool {
        self.direct.enemy_weak > 0
            || self.direct.enemy_vulnerable > 0
            || self.direct.persistent_enemy_strength_down > 0
            || self.direct.temporary_enemy_strength_down > 0
            || self.reactive.enemy_weak > 0
            || self.reactive.enemy_vulnerable > 0
    }

    pub(super) fn diagnostics(self) -> CardPlayEffectDiagnostics {
        CardPlayEffectDiagnostics {
            derived: self.derived_scores(),
            direct: CardPlayDirectEffectDiagnostics {
                persistent_enemy_strength_down: self.direct.persistent_enemy_strength_down,
                temporary_enemy_strength_down: self.direct.temporary_enemy_strength_down,
                visible_attack_mitigation_hint: self.direct.visible_attack_mitigation_hint,
                enemy_weak: self.direct.enemy_weak,
                enemy_vulnerable: self.direct.enemy_vulnerable,
                enemy_strength_gain: self.direct.enemy_strength_gain,
                visible_attack_pressure_hint: self.direct.visible_attack_pressure_hint,
                player_strength_gain: self.direct.player_strength_gain,
                player_temporary_strength_gain: self.direct.player_temporary_strength_gain,
            },
            reactive: CardPlayReactiveEffectDiagnostics {
                player_hp_loss: self.reactive.player_hp_loss,
                player_block: self.reactive.player_block,
                enemy_damage: self.reactive.enemy_damage,
                bad_draw_cards: self.reactive.bad_draw_cards,
                forced_turn_end: self.reactive.forced_turn_end,
                enemy_strength_gain: self.reactive.enemy_strength_gain,
                visible_attack_pressure_hint: self.reactive.visible_attack_pressure_hint,
                enemy_weak: self.reactive.enemy_weak,
                enemy_vulnerable: self.reactive.enemy_vulnerable,
            },
            access: CardPlayAccessEffectDiagnostics {
                declared_draw_cards: self.direct.declared_draw_cards,
                conditional_draw_cards: self.direct.conditional_draw_cards,
                total_draw_cards: self.total_draw_cards(),
            },
        }
    }
}

impl CardPlayEffectDiagnostics {
    pub(super) fn has_reactive_signal(self) -> bool {
        self.derived.reactive_risk_score > 0
            || self.reactive.player_block > 0
            || self.reactive.enemy_damage > 0
            || self.derived.mitigation_score > 0
    }
}

pub(super) fn card_play_effect_facts(
    combat: &CombatState,
    card: &CombatCard,
    target: Option<usize>,
) -> CardPlayEffectFacts {
    card_play_effects::card_play_effect_facts(combat, card, target)
}

pub(super) fn state_sustained_mitigation_score(combat: &CombatState) -> i32 {
    card_play_effects::state_sustained_mitigation_score(combat)
}

#[cfg(test)]
mod tests;
