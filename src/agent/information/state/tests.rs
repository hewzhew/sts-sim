use super::*;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::{Potion, PotionId};
use crate::content::powers::store;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::combat::{CombatRng, Power};
use crate::runtime::rng::RngPool;

fn test_power(power_type: PowerId, instance_id: u32, amount: i32) -> Power {
    Power {
        power_type,
        instance_id: Some(instance_id),
        amount,
        extra_data: 0,
        payload: PowerPayload::None,
        just_applied: false,
    }
}

#[test]
fn private_rng_and_execution_handles_do_not_change_public_state() {
    let mut left = crate::test_support::blank_test_combat();
    left.zones.hand = vec![CombatCard::new(CardId::Bash, 1)];
    left.entities.potions = vec![Some(Potion::new(PotionId::FruitJuice, 41))];
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = 7;
    left.entities.monsters.push(monster);
    let mut right = left.clone();
    right.zones.hand[0].uuid = 99;
    right.entities.potions[0].as_mut().unwrap().uuid = 199;
    right.entities.monsters[0].id = 299;
    right.rng = CombatRng::new(RngPool::new(123_456));

    assert_eq!(
        public_combat_state_v1(&left),
        public_combat_state_v1(&right)
    );
}

#[test]
fn unordered_public_pile_order_does_not_change_public_state() {
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
        public_combat_state_v1(&left),
        public_combat_state_v1(&right)
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
        public_combat_state_v1(&left),
        public_combat_state_v1(&right)
    );

    left.entities
        .player
        .add_relic(RelicState::new(RelicId::FrozenEye));
    right
        .entities
        .player
        .add_relic(RelicState::new(RelicId::FrozenEye));
    assert_ne!(
        public_combat_state_v1(&left),
        public_combat_state_v1(&right)
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

    let observation = public_combat_state_v1(&combat);

    assert_eq!(observation.monsters[0].intent.intent, None);
    assert_eq!(
        observation.monsters[0].intent.hidden_reason,
        Some(HiddenInformationReasonV1::RunicDome)
    );
}

#[test]
fn public_state_keeps_domain_card_potion_and_enemy_identities() {
    let mut combat = crate::test_support::blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Bash, 1)];
    combat.entities.potions = vec![Some(Potion::new(PotionId::FruitJuice, 41))];
    combat
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::JawWorm));

    let observation = public_combat_state_v1(&combat);

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
}

#[test]
fn hand_attack_projects_current_damage_for_each_monster_without_mutating_the_card() {
    let mut combat = crate::test_support::blank_test_combat();
    let mut strike = CombatCard::new(CardId::Strike, 41);
    strike.base_damage_mut = 99;
    strike.base_block_mut = 88;
    strike.base_magic_num_mut = 77;
    strike.multi_damage = smallvec::smallvec![66];
    combat.zones.hand = vec![strike.clone()];

    let mut first = crate::test_support::test_monster(EnemyId::Cultist);
    first.id = 1;
    first.slot = 0;
    let mut second = crate::test_support::test_monster(EnemyId::JawWorm);
    second.id = 2;
    second.slot = 1;
    combat.entities.monsters = vec![first, second];
    store::set_powers_for(
        &mut combat,
        0,
        vec![
            test_power(PowerId::Strength, 1, 3),
            test_power(PowerId::Weak, 2, 1),
        ],
    );
    store::set_powers_for(&mut combat, 2, vec![test_power(PowerId::Vulnerable, 3, 1)]);

    let observation = public_combat_state_v1(&combat);
    let projected = &observation.cards.hand.cards[0];

    assert_eq!(projected.current_damage, 6);
    assert_eq!(projected.damage_by_monster_order, vec![6, 10]);
    assert_eq!(combat.zones.hand[0], strike);
}

#[test]
fn hand_multi_attack_projection_stays_aligned_to_monster_order() {
    let mut combat = crate::test_support::blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::ThunderClap, 41)];

    let mut first = crate::test_support::test_monster(EnemyId::Cultist);
    first.id = 1;
    first.slot = 0;
    let mut second = crate::test_support::test_monster(EnemyId::JawWorm);
    second.id = 2;
    second.slot = 1;
    combat.entities.monsters = vec![first, second];
    store::set_powers_for(&mut combat, 2, vec![test_power(PowerId::Vulnerable, 1, 1)]);

    let observation = public_combat_state_v1(&combat);
    let projected = &observation.cards.hand.cards[0];

    assert_eq!(projected.current_damage, 4);
    assert_eq!(projected.damage_by_monster_order, vec![4, 6]);
}

#[test]
fn hand_skill_projects_current_block_without_fake_target_damage() {
    let mut combat = crate::test_support::blank_test_combat();
    let mut defend = CombatCard::new(CardId::Defend, 41);
    defend.base_block_mut = 99;
    defend.multi_damage = smallvec::smallvec![55];
    combat.zones.hand = vec![defend.clone()];
    combat
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::Cultist));
    store::set_powers_for(&mut combat, 0, vec![test_power(PowerId::Dexterity, 1, 2)]);

    let observation = public_combat_state_v1(&combat);
    let projected = &observation.cards.hand.cards[0];

    assert_eq!(projected.current_block, 7);
    assert!(projected.damage_by_monster_order.is_empty());
    assert_eq!(combat.zones.hand[0], defend);
}

#[test]
fn serialized_public_state_uses_current_card_projection() {
    let mut combat = crate::test_support::blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 1)];
    let observation = public_combat_state_v1(&combat);
    let value = serde_json::to_value(observation).expect("serialize public combat state");

    assert_eq!(value["schema_name"], "PublicCombatState");
    assert_eq!(value["schema_version"], 1);
    assert!(value.get("cards").is_some());
    assert!(value.get("player").is_some());
    assert!(value.get("player_summary").is_none());
    assert!(value.get("public_summary").is_none());
    let card = &value["cards"]["hand"]["cards"][0];
    assert_eq!(card["current_damage"], 6);
    assert!(card.get("base_damage_mut").is_none());
}

#[test]
fn public_powers_relics_and_dynamic_cards_change_public_state() {
    let mut baseline = crate::test_support::blank_test_combat();
    baseline.zones.hand = vec![CombatCard::new(CardId::Bash, 1)];
    let baseline_observation = public_combat_state_v1(&baseline);

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

    assert_ne!(baseline_observation, public_combat_state_v1(&changed));
}

#[test]
fn private_monster_roll_state_does_not_change_public_state() {
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
        public_combat_state_v1(&left),
        public_combat_state_v1(&right),
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

    let observation = public_combat_state_v1(&combat);

    assert_eq!(
        observation.monsters[0].executed_moves,
        CombatLearningMonsterMoveHistoryV1 {
            evidence: ObservationEvidenceKindV1::PublicOrderedCollection,
            move_ids: vec![1],
        }
    );
}

#[test]
fn public_encounter_counters_change_public_state() {
    let mut thief_combat = crate::test_support::blank_test_combat();
    let mut looter = crate::test_support::test_monster(EnemyId::Looter);
    looter.thief.stolen_gold = 17;
    thief_combat.entities.monsters.push(looter);

    let thief_observation = public_combat_state_v1(&thief_combat);
    assert_eq!(
        thief_observation.monsters[0].public_counters,
        vec![CombatLearningMonsterPublicCounterV1::StolenGold { amount: 17 }]
    );

    let mut hexaghost_combat = crate::test_support::blank_test_combat();
    let mut hexaghost = crate::test_support::test_monster(EnemyId::Hexaghost);
    hexaghost.hexaghost.orb_active_count = 4;
    hexaghost_combat.entities.monsters.push(hexaghost);

    let hexaghost_observation = public_combat_state_v1(&hexaghost_combat);
    assert_eq!(
        hexaghost_observation.monsters[0].public_counters,
        vec![CombatLearningMonsterPublicCounterV1::HexaghostActiveOrbs { count: 4 }]
    );
}
