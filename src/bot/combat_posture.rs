use crate::bot::monster_belief::build_combat_belief_state;
use crate::runtime::combat::CombatState;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::monsters::EnemyId;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CombatPostureFeatures {
    pub immediate_survival_pressure: i32,
    pub future_pollution_risk: i32,
    pub expected_fight_length_bucket: i32,
    pub setup_payoff_density: i32,
    pub resource_preservation_pressure: i32,
}

pub(crate) fn posture_features(combat: &CombatState) -> CombatPostureFeatures {
    let immediate_survival_pressure = immediate_survival_pressure(combat);
    let visible_statuses = visible_status_count(combat);
    let self_pollution = self_pollution_sources(combat);
    let enemy_pollution = enemy_pollution_risk(combat);
    let future_pollution_risk = visible_statuses + self_pollution + enemy_pollution;
    let expected_fight_length_bucket = expected_fight_length_bucket(combat);
    let setup_payoff_density =
        setup_payoff_density(combat, future_pollution_risk, expected_fight_length_bucket);
    let resource_preservation_pressure = resource_preservation_pressure(
        combat,
        immediate_survival_pressure,
        future_pollution_risk,
        expected_fight_length_bucket,
    );

    CombatPostureFeatures {
        immediate_survival_pressure,
        future_pollution_risk,
        expected_fight_length_bucket,
        setup_payoff_density,
        resource_preservation_pressure,
    }
}

fn immediate_survival_pressure(combat: &CombatState) -> i32 {
    (total_incoming_damage(combat) - combat.entities.player.block).max(0)
}

fn total_incoming_damage(combat: &CombatState) -> i32 {
    build_combat_belief_state(combat)
        .expected_incoming_damage
        .round() as i32
}

fn visible_status_count(combat: &CombatState) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .filter(|card| {
            matches!(
                get_card_definition(card.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .count() as i32
}

fn self_pollution_sources(combat: &CombatState) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .map(|card| match card.id {
            CardId::PowerThrough | CardId::WildStrike => 2,
            CardId::RecklessCharge | CardId::Immolate => 1,
            _ => 0,
        })
        .sum()
}

fn enemy_pollution_risk(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| match EnemyId::from_id(monster.monster_type) {
            Some(EnemyId::SlimeBoss) => 6,
            Some(EnemyId::Sentry) => 4,
            Some(EnemyId::AcidSlimeL) | Some(EnemyId::AcidSlimeM) => 3,
            Some(EnemyId::SpikeSlimeL) | Some(EnemyId::SpikeSlimeM) => 2,
            Some(EnemyId::Chosen) => 2,
            _ => 0,
        })
        .sum()
}

fn expected_fight_length_bucket(combat: &CombatState) -> i32 {
    let total_enemy_hp: i32 = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| monster.current_hp + monster.block)
        .sum();
    let rough_damage_rate = rough_damage_rate(combat).max(1);
    let projected_turns = (total_enemy_hp + rough_damage_rate - 1) / rough_damage_rate;

    if combat.meta.is_boss_fight || projected_turns >= 6 || total_enemy_hp >= 90 {
        2
    } else if combat.meta.is_elite_fight || projected_turns >= 3 || total_enemy_hp >= 45 {
        1
    } else {
        0
    }
}

fn rough_damage_rate(combat: &CombatState) -> i32 {
    let attack_count = combat
        .zones
        .hand
        .iter()
        .filter(|card| get_card_definition(card.id).card_type == CardType::Attack)
        .count() as i32;
    let attack_base_damage: i32 = combat
        .zones
        .hand
        .iter()
        .filter(|card| get_card_definition(card.id).card_type == CardType::Attack)
        .map(|card| {
            card.base_damage_mut
                .max(get_card_definition(card.id).base_damage)
        })
        .sum();

    attack_base_damage.max(attack_count * 8).max(10)
}

fn setup_payoff_density(
    combat: &CombatState,
    future_pollution_risk: i32,
    expected_fight_length_bucket: i32,
) -> i32 {
    let setup_cards = combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .filter(|card| {
            matches!(
                card.id,
                CardId::FireBreathing
                    | CardId::DarkEmbrace
                    | CardId::FeelNoPain
                    | CardId::Evolve
                    | CardId::Corruption
            )
        })
        .count() as i32;
    let exhaust_outlets = combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .filter(|card| {
            matches!(
                card.id,
                CardId::SecondWind
                    | CardId::BurningPact
                    | CardId::TrueGrit
                    | CardId::FiendFire
                    | CardId::SeverSoul
            )
        })
        .count() as i32;

    setup_cards * 2
        + exhaust_outlets
        + expected_fight_length_bucket
        + (future_pollution_risk / 3)
        + i32::from(combat.meta.is_boss_fight || combat.meta.is_elite_fight)
}

fn resource_preservation_pressure(
    combat: &CombatState,
    immediate_survival_pressure: i32,
    future_pollution_risk: i32,
    expected_fight_length_bucket: i32,
) -> i32 {
    future_pollution_risk
        + expected_fight_length_bucket * 2
        + i32::from(immediate_survival_pressure == 0) * 3
        + i32::from(combat.meta.is_boss_fight || combat.meta.is_elite_fight) * 2
}
