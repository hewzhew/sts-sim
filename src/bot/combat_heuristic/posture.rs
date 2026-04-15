use crate::content::cards::{CardId, CardType};

use super::apply::{effective_damage, effective_hits};
use super::sim::{active_hand_cards, SimState};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct HeuristicPostureFeatures {
    pub immediate_survival_pressure: i32,
    pub future_pollution_risk: i32,
    pub expected_fight_length_bucket: i32,
    pub setup_payoff_density: i32,
    pub resource_preservation_pressure: i32,
}

pub(super) fn posture_features(state: &SimState) -> HeuristicPostureFeatures {
    let immediate_survival_pressure = (total_incoming_damage(state) - state.player_block).max(0);
    let future_pollution_risk = status_cards_in_hand(state)
        + state.future_status_cards.max(0)
        + self_pollution_sources(state);
    let expected_fight_length_bucket = expected_fight_length_bucket(state);
    let setup_payoff_density = setup_payoff_density(state, future_pollution_risk);
    let resource_preservation_pressure = future_pollution_risk
        + expected_fight_length_bucket * 2
        + i32::from(immediate_survival_pressure == 0) * 3
        + (state.future_key_delay_weight / 2).max(0)
        + i32::from(state.is_boss_fight || state.is_elite_fight) * 2;

    HeuristicPostureFeatures {
        immediate_survival_pressure,
        future_pollution_risk,
        expected_fight_length_bucket,
        setup_payoff_density,
        resource_preservation_pressure,
    }
}

fn self_pollution_sources(state: &SimState) -> i32 {
    active_hand_cards(state)
        .map(|(_, card)| match card.card_id {
            CardId::PowerThrough | CardId::WildStrike => 2,
            CardId::RecklessCharge | CardId::Immolate => 1,
            _ => 0,
        })
        .sum()
}

fn expected_fight_length_bucket(state: &SimState) -> i32 {
    let total_enemy_hp: i32 = state
        .monsters
        .iter()
        .filter(|monster| !monster.is_gone)
        .map(|monster| monster.hp + monster.block)
        .sum();
    let rough_damage_rate = rough_damage_rate(state).max(1);
    let projected_turns = (total_enemy_hp + rough_damage_rate - 1) / rough_damage_rate;

    if state.is_boss_fight || projected_turns >= 6 || total_enemy_hp >= 90 {
        2
    } else if state.is_elite_fight || projected_turns >= 3 || total_enemy_hp >= 45 {
        1
    } else {
        0
    }
}

fn rough_damage_rate(state: &SimState) -> i32 {
    let attack_count = active_hand_cards(state)
        .filter(|(_, card)| card.card_type == CardType::Attack)
        .count() as i32;
    let attack_damage: i32 = active_hand_cards(state)
        .filter(|(_, card)| card.card_type == CardType::Attack)
        .map(|(_, card)| {
            let hits = if card.hits > 0 {
                card.hits
            } else {
                effective_hits(card, card.cost)
            };
            effective_damage(state, card) * hits.max(1)
        })
        .sum();

    attack_damage.max(attack_count * 8).max(10)
}

fn setup_payoff_density(state: &SimState, future_pollution_risk: i32) -> i32 {
    let setup_cards = active_hand_cards(state)
        .filter(|(_, card)| {
            matches!(
                card.card_id,
                CardId::FireBreathing
                    | CardId::DarkEmbrace
                    | CardId::FeelNoPain
                    | CardId::Evolve
                    | CardId::Corruption
            )
        })
        .count() as i32;

    setup_cards * 2
        + immediate_exhaust_count(state)
        + future_exhaust_source_count(state)
        + (future_pollution_risk / 3)
        + i32::from(state.is_boss_fight || state.is_elite_fight)
}

fn total_incoming_damage(state: &SimState) -> i32 {
    state
        .monsters
        .iter()
        .filter(|monster| !monster.is_gone && monster.is_attacking)
        .map(|monster| (monster.intent_dmg * monster.intent_hits).max(0))
        .sum()
}

fn status_cards_in_hand(state: &SimState) -> i32 {
    active_hand_cards(state)
        .filter(|(_, card)| matches!(card.card_type, CardType::Status | CardType::Curse))
        .count() as i32
}

fn immediate_exhaust_count(state: &SimState) -> i32 {
    active_hand_cards(state)
        .filter(|(_, card)| {
            matches!(
                card.card_id,
                CardId::SecondWind
                    | CardId::BurningPact
                    | CardId::TrueGrit
                    | CardId::FiendFire
                    | CardId::SeverSoul
            )
        })
        .count() as i32
}

fn future_exhaust_source_count(state: &SimState) -> i32 {
    active_hand_cards(state)
        .filter(|(_, card)| {
            matches!(
                card.card_id,
                CardId::SecondWind
                    | CardId::BurningPact
                    | CardId::TrueGrit
                    | CardId::FiendFire
                    | CardId::SeverSoul
                    | CardId::DarkEmbrace
                    | CardId::FeelNoPain
                    | CardId::Corruption
            )
        })
        .count() as i32
}
