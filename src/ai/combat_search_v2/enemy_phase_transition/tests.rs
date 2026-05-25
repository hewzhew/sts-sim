use super::*;
use crate::content::cards::CardId;
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
use crate::test_support::{blank_test_combat, test_monster};

fn test_power(power_type: PowerId, amount: i32) -> Power {
    Power {
        power_type,
        instance_id: None,
        amount,
        extra_data: 0,
        payload: PowerPayload::None,
        just_applied: false,
    }
}

#[test]
fn detects_large_slime_split_trigger_from_card_damage() {
    let mut combat = blank_test_combat();
    let mut slime = test_monster(EnemyId::AcidSlimeL);
    slime.id = 11;
    slime.current_hp = 40;
    slime.max_hp = 65;
    combat.entities.monsters = vec![slime];
    combat
        .entities
        .power_db
        .insert(11, vec![test_power(PowerId::Split, -1)]);
    combat.zones.hand = vec![CombatCard::new(CardId::Carnage, 20)];

    let hint = enemy_phase_transition_hint_for_input(
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(11),
        },
    );

    assert_eq!(hint.split_trigger_count, 1);
    assert!(hint.split_debt_hp > 0);
    assert!(hint.ordering_risk_score() > 0);
}

#[test]
fn lethal_split_monster_damage_is_not_split_debt() {
    let mut combat = blank_test_combat();
    let mut slime = test_monster(EnemyId::AcidSlimeL);
    slime.id = 11;
    slime.current_hp = 20;
    slime.max_hp = 65;
    combat.entities.monsters = vec![slime];
    combat
        .entities
        .power_db
        .insert(11, vec![test_power(PowerId::Split, -1)]);
    combat.zones.hand = vec![CombatCard::new(CardId::Carnage, 20)];

    let hint = enemy_phase_transition_hint_for_input(
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(11),
        },
    );

    assert_eq!(hint.split_trigger_count, 0);
    assert_eq!(hint.split_debt_hp, 0);
}

#[test]
fn detects_slime_boss_split_trigger_from_card_damage() {
    let mut combat = blank_test_combat();
    let mut slime = test_monster(EnemyId::SlimeBoss);
    slime.id = 14;
    slime.current_hp = 80;
    slime.max_hp = 140;
    combat.entities.monsters = vec![slime];
    combat
        .entities
        .power_db
        .insert(14, vec![test_power(PowerId::Split, -1)]);
    combat.zones.hand = vec![CombatCard::new(CardId::Carnage, 20)];

    let hint = enemy_phase_transition_hint_for_input(
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(14),
        },
    );

    assert_eq!(hint.split_trigger_count, 1);
    assert!(hint.split_debt_hp > 0);
}

#[test]
fn detects_guardian_mode_shift_trigger_from_card_damage() {
    let mut combat = blank_test_combat();
    let mut guardian = test_monster(EnemyId::TheGuardian);
    guardian.id = 12;
    guardian.current_hp = 100;
    guardian.max_hp = 240;
    guardian.guardian.is_open = true;
    combat.entities.monsters = vec![guardian];
    combat
        .entities
        .power_db
        .insert(12, vec![test_power(PowerId::ModeShift, 5)]);
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 21)];

    let hint = enemy_phase_transition_hint_for_input(
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(12),
        },
    );

    assert_eq!(hint.guardian_mode_shift_trigger_count, 1);
    assert_eq!(hint.guardian_min_threshold_remaining_before_hit, Some(5));
}

#[test]
fn detects_lagavulin_wake_risk_from_card_damage() {
    let mut combat = blank_test_combat();
    let mut lagavulin = test_monster(EnemyId::Lagavulin);
    lagavulin.id = 13;
    lagavulin.lagavulin.is_out = false;
    combat.entities.monsters = vec![lagavulin];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 22)];

    let hint = enemy_phase_transition_hint_for_input(
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(13),
        },
    );

    assert_eq!(hint.lagavulin_wake_risk_count, 1);
}
