use crate::runtime::combat::CombatState;

use super::monster_belief::build_combat_belief_state;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StatePressureFeatures {
    pub visible_incoming: i32,
    pub visible_unblocked: i32,
    pub belief_expected_incoming: i32,
    pub belief_expected_unblocked: i32,
    pub belief_max_incoming: i32,
    pub belief_max_unblocked: i32,
    pub value_incoming: i32,
    pub value_unblocked: i32,
    pub survival_guard_incoming: i32,
    pub survival_guard_unblocked: i32,
    pub incoming: i32,
    pub max_incoming: i32,
    pub unblocked: i32,
    pub max_unblocked: i32,
    pub player_hp: i32,
    pub lethal_pressure: bool,
    pub urgent_pressure: bool,
    pub hidden_intent_active: bool,
    pub attack_probability: f32,
    pub lethal_probability: f32,
    pub urgent_probability: f32,
    pub encounter_risk: bool,
}

impl StatePressureFeatures {
    pub(crate) fn from_combat(combat: &CombatState) -> Self {
        let belief = build_combat_belief_state(combat);
        let visible_incoming = visible_total_incoming_damage(combat);
        let visible_unblocked = (visible_incoming - combat.entities.player.block).max(0);
        let belief_expected_incoming = belief.expected_incoming_damage.round() as i32;
        let belief_expected_unblocked =
            (belief_expected_incoming - combat.entities.player.block).max(0);
        let belief_max_incoming = belief.max_incoming_damage;
        let belief_max_unblocked = (belief_max_incoming - combat.entities.player.block).max(0);
        let hidden_intent_active = belief.hidden_intent_active;
        let value_incoming = if hidden_intent_active {
            belief_expected_incoming
        } else {
            visible_incoming
        };
        let value_unblocked = (value_incoming - combat.entities.player.block).max(0);
        let survival_guard_incoming = if hidden_intent_active {
            belief_max_incoming
        } else {
            visible_incoming
        };
        let survival_guard_unblocked =
            (survival_guard_incoming - combat.entities.player.block).max(0);
        let player_hp = combat.entities.player.current_hp.max(1);
        let lethal_probability = if hidden_intent_active {
            belief.lethal_probability
        } else if visible_unblocked >= player_hp {
            1.0
        } else {
            0.0
        };
        let urgent_probability = if hidden_intent_active {
            belief.urgent_probability
        } else if visible_unblocked >= 8 || visible_unblocked >= player_hp {
            1.0
        } else {
            0.0
        };
        let lethal_pressure = survival_guard_unblocked >= player_hp
            || (hidden_intent_active && belief.lethal_probability >= 0.20);
        let urgent_pressure = lethal_pressure
            || value_unblocked >= 8
            || (hidden_intent_active && belief.urgent_probability >= 0.35);
        Self {
            visible_incoming,
            visible_unblocked,
            belief_expected_incoming,
            belief_expected_unblocked,
            belief_max_incoming,
            belief_max_unblocked,
            value_incoming,
            value_unblocked,
            survival_guard_incoming,
            survival_guard_unblocked,
            incoming: value_incoming,
            max_incoming: survival_guard_incoming,
            unblocked: value_unblocked,
            max_unblocked: survival_guard_unblocked,
            player_hp,
            lethal_pressure,
            urgent_pressure,
            hidden_intent_active,
            attack_probability: if hidden_intent_active {
                belief.attack_probability
            } else {
                visible_attack_probability(combat)
            },
            lethal_probability,
            urgent_probability,
            encounter_risk: combat.meta.is_elite_fight || combat.meta.is_boss_fight,
        }
    }
}

fn visible_total_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .map(|monster| {
            crate::projection::combat::monster_preview_total_damage_in_combat(combat, monster)
        })
        .sum()
}

fn visible_attack_probability(combat: &CombatState) -> f32 {
    let any_attack = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .any(|monster| {
            crate::projection::combat::monster_has_visible_attack_in_combat(combat, monster)
        });
    if any_attack {
        1.0
    } else {
        0.0
    }
}
