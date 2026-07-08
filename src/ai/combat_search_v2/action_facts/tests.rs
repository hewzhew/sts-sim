use crate::content::cards::{CardId, CardType};
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
use crate::sim::combat::EngineCombatStepper;
use crate::test_support::{blank_test_combat, planned_monster, test_monster};

use super::*;

#[test]
fn facts_report_card_definition_and_exact_delta_for_strike() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    combat.entities.monsters = vec![planned_monster(EnemyId::JawWorm, 1)];

    let facts = summarize_action_facts(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        &EngineCombatStepper,
        250,
    );

    let card = facts.card.expect("strike card facts");
    assert_eq!(card.name, "Strike");
    assert_eq!(card.card_type, CardType::Attack);
    assert!(facts.immediate.damage_hint > 0);
    assert!(facts.exact_one_step_delta.total_enemy_hp_delta < 0);
}

#[test]
fn facts_report_action_payload_damage_for_multi_hit_card() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::TwinStrike, 10)];
    combat.entities.monsters = vec![planned_monster(EnemyId::JawWorm, 1)];

    let facts = summarize_action_facts(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        &EngineCombatStepper,
        250,
    );

    assert_eq!(
        facts.card.as_ref().map(|card| card.evaluated_damage),
        Some(5)
    );
    assert_eq!(facts.immediate.action_payload_damage_hint, 10);
    assert_eq!(facts.immediate.action_payload_damage_hit_count_hint, 2);
    assert_eq!(facts.immediate.target_progress_hint, 10);
    assert_eq!(facts.exact_one_step_delta.total_enemy_hp_delta, -10);
}

#[test]
fn facts_report_nob_anger_from_reactive_power_without_card_tag() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Defend, 10)];
    let mut nob = test_monster(EnemyId::GremlinNob);
    nob.id = 1;
    combat.entities.monsters = vec![nob];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Anger,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let facts = summarize_action_facts(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
        &EngineCombatStepper,
        250,
    );

    assert_eq!(facts.card.as_ref().map(|card| card.name), Some("Defend"));
    assert!(facts.immediate.block_hint > 0);
    assert!(facts.mechanics.reactive.enemy_strength_gain > 0);
    assert!(facts.mechanics.derived.enemy_strength_gain > 0);
}

#[test]
fn facts_report_player_strength_gain_without_enemy_scaling_risk() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Flex, 10)];

    let facts = summarize_action_facts(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
        &EngineCombatStepper,
        250,
    );

    assert_eq!(facts.card.as_ref().map(|card| card.name), Some("Flex"));
    assert_eq!(facts.mechanics.direct.player_strength_gain, 2);
    assert_eq!(facts.mechanics.direct.player_temporary_strength_gain, 2);
    assert_eq!(facts.mechanics.derived.enemy_strength_gain, 0);
}

#[test]
fn facts_report_effective_ethereal_after_upgrade_sensitive_overrides() {
    let mut combat = blank_test_combat();
    let mut echo_form = CombatCard::new(CardId::EchoForm, 10);
    echo_form.upgrades = 1;
    combat.zones.hand = vec![echo_form];

    let facts = summarize_action_facts(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
        &EngineCombatStepper,
        250,
    );

    let card = facts.card.expect("echo form card facts");
    assert_eq!(card.name, "Echo Form");
    assert!(!card.ethereal, "Echo Form+ should not be reported ethereal");
}

#[test]
fn facts_report_dropkick_contextual_draw_and_energy_delta_from_simulator() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Dropkick, 10)];
    combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 11)];
    let mut monster = planned_monster(EnemyId::JawWorm, 1);
    monster.id = 1;
    combat.entities.monsters = vec![monster];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Vulnerable,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    let facts = summarize_action_facts(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        &EngineCombatStepper,
        250,
    );

    assert_eq!(
        facts.target.as_ref().map(|target| target.vulnerable),
        Some(2)
    );
    assert_eq!(facts.exact_one_step_delta.energy_delta, 0);
    assert_eq!(facts.exact_one_step_delta.draw_delta, -1);
    assert_eq!(facts.exact_one_step_delta.hand_delta, 0);
}

#[test]
fn facts_report_fiend_fire_hand_resource_conversion_timing() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![
        CombatCard::new(CardId::FiendFire, 10),
        CombatCard::new(CardId::Offering, 11),
        CombatCard::new(CardId::SpotWeakness, 12),
        CombatCard::new(CardId::Strike, 13),
    ];
    let mut monster = planned_monster(EnemyId::JawWorm, 1);
    monster.id = 1;
    monster.current_hp = 80;
    monster.max_hp = 80;
    combat.entities.monsters = vec![monster];

    let facts = summarize_action_facts(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        &EngineCombatStepper,
        250,
    );

    assert_eq!(
        facts.card.as_ref().map(|card| card.name),
        Some("Fiend Fire")
    );
    assert_eq!(facts.mechanics.resource_timing.hand_exhaust_target_count, 3);
    assert_eq!(facts.mechanics.resource_timing.hand_exhaust_fuel_count, 1);
    assert!(facts.mechanics.resource_timing.hand_exhaust_value_at_risk > 0);
    assert!(facts.mechanics.resource_timing.premature_conversion_risk > 0);
    assert_eq!(facts.mechanics.resource_timing.conversion_damage_hint, 21);
    assert_eq!(facts.immediate.damage_hint, 21);
    assert_eq!(facts.immediate.action_payload_damage_hit_count_hint, 3);
}
