use super::*;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
use crate::test_support::{blank_test_combat, test_monster};

#[test]
fn disarm_reports_persistent_enemy_strength_down_without_card_id_special_case() {
    let mut combat = blank_test_combat();
    let mut guardian = test_monster(EnemyId::TheGuardian);
    guardian.id = 1;
    guardian.set_planned_move_id(6);
    combat.entities.monsters = vec![guardian];
    let disarm = CombatCard::new(CardId::Disarm, 10);

    let facts = card_play_effect_facts(&combat, &disarm, Some(1));

    assert!(facts.direct.persistent_enemy_strength_down > 0);
    assert_eq!(facts.direct.temporary_enemy_strength_down, 0);
}

#[test]
fn state_mitigation_score_counts_negative_enemy_strength() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::Cultist);
    monster.id = 1;
    combat.entities.monsters = vec![monster];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: -3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    assert_eq!(state_sustained_mitigation_score(&combat), 3);
}

#[test]
fn anger_power_reports_enemy_strength_gain_for_skill_without_monster_special_case() {
    let mut combat = blank_test_combat();
    let mut nob = test_monster(EnemyId::GremlinNob);
    nob.id = 1;
    combat.entities.monsters = vec![nob];
    insert_power(&mut combat, 1, PowerId::Anger, 2);

    let defend = CombatCard::new(CardId::Defend, 10);
    let strike = CombatCard::new(CardId::Strike, 11);

    let defend_facts = card_play_effect_facts(&combat, &defend, None);
    let strike_facts = card_play_effect_facts(&combat, &strike, Some(1));

    assert!(defend_facts.reactive.enemy_strength_gain > 0);
    assert!(defend_facts.enemy_scaling_risk_score() > 0);
    assert_eq!(strike_facts.reactive.enemy_strength_gain, 0);
}

#[test]
fn flex_reports_player_strength_gain_without_enemy_scaling_risk() {
    let combat = blank_test_combat();
    let flex = CombatCard::new(CardId::Flex, 10);

    let facts = card_play_effect_facts(&combat, &flex, None);

    assert_eq!(facts.direct.player_strength_gain, 2);
    assert_eq!(facts.direct.player_temporary_strength_gain, 2);
    assert_eq!(facts.direct.enemy_strength_gain, 0);
    assert_eq!(facts.enemy_scaling_risk_score(), 0);
}

#[test]
fn sharp_hide_reports_reactive_player_hp_loss_for_attack() {
    let mut combat = blank_test_combat();
    let mut guardian = test_monster(EnemyId::TheGuardian);
    guardian.id = 1;
    combat.entities.monsters = vec![guardian];
    insert_power(&mut combat, 1, PowerId::SharpHide, 3);

    let strike = CombatCard::new(CardId::Strike, 10);
    let defend = CombatCard::new(CardId::Defend, 11);

    let strike_facts = card_play_effect_facts(&combat, &strike, Some(1));
    let defend_facts = card_play_effect_facts(&combat, &defend, None);

    assert_eq!(strike_facts.reactive.player_hp_loss, 3);
    assert_eq!(defend_facts.reactive.player_hp_loss, 0);
}

#[test]
fn after_image_reports_reactive_player_block() {
    let mut combat = blank_test_combat();
    insert_power(&mut combat, 0, PowerId::AfterImage, 1);
    let strike = CombatCard::new(CardId::Strike, 10);

    let facts = card_play_effect_facts(&combat, &strike, Some(1));

    assert_eq!(facts.reactive.player_block, 1);
}

#[test]
fn hex_reports_bad_draw_cards_for_non_attack() {
    let mut combat = blank_test_combat();
    let mut chosen = test_monster(EnemyId::Chosen);
    chosen.id = 1;
    combat.entities.monsters = vec![chosen];
    insert_power(&mut combat, 1, PowerId::Hex, 1);

    let defend = CombatCard::new(CardId::Defend, 10);
    let strike = CombatCard::new(CardId::Strike, 11);

    let defend_facts = card_play_effect_facts(&combat, &defend, None);
    let strike_facts = card_play_effect_facts(&combat, &strike, Some(1));

    assert_eq!(defend_facts.reactive.bad_draw_cards, 1);
    assert_eq!(strike_facts.reactive.bad_draw_cards, 0);
}

#[test]
fn time_warp_reports_forced_turn_end() {
    let mut combat = blank_test_combat();
    let mut eater = test_monster(EnemyId::TimeEater);
    eater.id = 1;
    combat.entities.monsters = vec![eater];
    insert_power(&mut combat, 1, PowerId::TimeWarp, 11);
    let strike = CombatCard::new(CardId::Strike, 10);

    let facts = card_play_effect_facts(&combat, &strike, Some(1));

    assert!(facts.reactive.forced_turn_end);
}

fn insert_power(combat: &mut CombatState, owner: usize, power_type: PowerId, amount: i32) {
    combat.entities.power_db.insert(
        owner,
        vec![Power {
            power_type,
            instance_id: None,
            amount,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
}
