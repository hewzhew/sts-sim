use super::*;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::{Potion, PotionId};
use crate::content::powers::store;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::combat::{CombatRng, Power};
use crate::runtime::rng::RngPool;

#[test]
fn private_rng_and_card_uuid_do_not_change_learning_observation() {
    let mut left = crate::test_support::blank_test_combat();
    left.zones.hand = vec![CombatCard::new(CardId::Bash, 1)];
    let mut right = left.clone();
    right.zones.hand[0].uuid = 99;
    right.rng = CombatRng::new(RngPool::new(123_456));

    assert_eq!(
        combat_learning_observation_v1(&left),
        combat_learning_observation_v1(&right)
    );
}

#[test]
fn unordered_public_pile_order_does_not_change_learning_observation() {
    let mut left = crate::test_support::blank_test_combat();
    left.zones.discard_pile = vec![
        CombatCard::new(CardId::Bash, 1),
        CombatCard::new(CardId::Defend, 2),
    ]
    .into();
    left.zones.exhaust_pile = vec![
        CombatCard::new(CardId::Strike, 3),
        CombatCard::new(CardId::AscendersBane, 4),
    ]
    .into();
    let mut right = left.clone();
    let mut discard = std::mem::take(&mut right.zones.discard_pile).into_vec();
    discard.reverse();
    right.zones.discard_pile = discard.into();
    right.zones.exhaust_pile.reverse();

    assert_eq!(
        combat_learning_observation_v1(&left),
        combat_learning_observation_v1(&right)
    );
}

#[test]
fn draw_order_is_hidden_without_frozen_eye_and_visible_with_it() {
    let mut left = crate::test_support::blank_test_combat();
    left.zones.draw_pile = vec![
        CombatCard::new(CardId::Bash, 1),
        CombatCard::new(CardId::Defend, 2),
    ]
    .into();
    let mut right = left.clone();
    right.zones.draw_pile.swap(0, 1);

    assert_eq!(
        combat_learning_observation_v1(&left),
        combat_learning_observation_v1(&right)
    );

    left.entities
        .player
        .add_relic(RelicState::new(RelicId::FrozenEye));
    right
        .entities
        .player
        .add_relic(RelicState::new(RelicId::FrozenEye));
    assert_ne!(
        combat_learning_observation_v1(&left),
        combat_learning_observation_v1(&right)
    );
}

#[test]
fn runic_dome_hides_typed_intent_facts() {
    let mut combat = crate::test_support::blank_test_combat();
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = 7;
    combat.entities.monsters.push(monster);
    combat.set_monster_protocol_visible_intent(
        7,
        Intent::Attack {
            damage: 11,
            hits: 1,
        },
    );
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::RunicDome));

    let observation = combat_learning_observation_v1(&combat);

    assert_eq!(observation.monsters[0].intent.intent, None);
    assert_eq!(
        observation.monsters[0].intent.hidden_reason,
        Some(HiddenInformationReasonV1::RunicDome)
    );
}

#[test]
fn learning_observation_keeps_domain_card_potion_and_enemy_identities() {
    let mut combat = crate::test_support::blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Bash, 1)];
    combat.entities.potions = vec![Some(Potion::new(PotionId::FruitJuice, 41))];
    combat
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::JawWorm));

    let observation = combat_learning_observation_v1(&combat);

    assert_eq!(observation.cards.hand.cards[0].card_id, CardId::Bash);
    assert_eq!(
        observation.potions[0]
            .as_ref()
            .expect("owned potion")
            .potion_id,
        PotionId::FruitJuice
    );
    assert_eq!(
        observation.monsters[0].enemy,
        CombatLearningEnemyIdentityV1::Known {
            enemy_id: EnemyId::JawWorm,
        }
    );
    assert_eq!(
        observation.monsters[0].entity_id,
        combat.entities.monsters[0].id
    );
}

#[test]
fn serialized_learning_observation_has_one_card_zone_projection() {
    let observation = combat_learning_observation_v1(&crate::test_support::blank_test_combat());
    let value = serde_json::to_value(observation).expect("serialize learning observation");

    assert!(value.get("cards").is_some());
    assert!(value.get("player").is_some());
    assert!(value.get("player_summary").is_none());
    assert!(value.get("public_summary").is_none());
}

#[test]
fn public_powers_relics_and_dynamic_cards_change_learning_observation() {
    let mut baseline = crate::test_support::blank_test_combat();
    baseline.zones.hand = vec![CombatCard::new(CardId::Bash, 1)];
    let baseline_observation = combat_learning_observation_v1(&baseline);

    let mut changed = baseline.clone();
    changed.zones.hand[0].base_damage_mut = 4;
    changed
        .entities
        .player
        .add_relic(RelicState::new(RelicId::PenNib));
    let player_id = changed.entities.player.id;
    store::set_powers_for(
        &mut changed,
        player_id,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: Some(77),
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    assert_ne!(
        baseline_observation,
        combat_learning_observation_v1(&changed)
    );
}

#[test]
fn private_monster_roll_state_does_not_change_learning_observation() {
    let mut left = crate::test_support::blank_test_combat();
    let mut louse = crate::test_support::test_monster(EnemyId::LouseNormal);
    louse.id = 7;
    louse.set_planned_move_id(1);
    louse.move_history_mut().push_back(1);
    louse.louse.bite_damage = Some(6);
    left.entities.monsters.push(louse);
    left.entities
        .player
        .add_relic(RelicState::new(RelicId::RunicDome));

    let mut right = left.clone();
    right.entities.monsters[0].set_planned_move_id(2);
    right.entities.monsters[0].move_history_mut().clear();
    right.entities.monsters[0].move_history_mut().push_back(2);
    right.entities.monsters[0].louse.bite_damage = Some(9);

    assert_eq!(
        combat_learning_observation_v1(&left),
        combat_learning_observation_v1(&right),
        "the private current roll and unrevealed louse damage must not leak"
    );
}

#[test]
fn executed_monster_history_is_public_but_the_current_roll_is_not() {
    let mut combat = crate::test_support::blank_test_combat();
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = 7;
    monster.set_planned_move_id(2);
    monster.move_history_mut().extend([1, 2]);
    combat.entities.monsters.push(monster);
    combat.record_monster_protocol_executed_move(7, 1);
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::RunicDome));

    let observation = combat_learning_observation_v1(&combat);

    assert_eq!(
        observation.monsters[0].executed_moves,
        CombatLearningMonsterMoveHistoryV1 {
            evidence: ObservationEvidenceKindV1::PublicOrderedCollection,
            move_ids: vec![1],
        }
    );
}

#[test]
fn public_encounter_counters_change_learning_observation() {
    let mut thief_combat = crate::test_support::blank_test_combat();
    let mut looter = crate::test_support::test_monster(EnemyId::Looter);
    looter.thief.stolen_gold = 17;
    thief_combat.entities.monsters.push(looter);

    let thief_observation = combat_learning_observation_v1(&thief_combat);
    assert_eq!(
        thief_observation.monsters[0].public_counters,
        vec![CombatLearningMonsterPublicCounterV1::StolenGold { amount: 17 }]
    );

    let mut hexaghost_combat = crate::test_support::blank_test_combat();
    let mut hexaghost = crate::test_support::test_monster(EnemyId::Hexaghost);
    hexaghost.hexaghost.orb_active_count = 4;
    hexaghost_combat.entities.monsters.push(hexaghost);

    let hexaghost_observation = combat_learning_observation_v1(&hexaghost_combat);
    assert_eq!(
        hexaghost_observation.monsters[0].public_counters,
        vec![CombatLearningMonsterPublicCounterV1::HexaghostActiveOrbs { count: 4 }]
    );
}
