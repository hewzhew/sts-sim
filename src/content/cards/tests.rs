use super::*;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageInfo, DamageType, NO_SOURCE};
use crate::runtime::combat::{CombatCard, Power};

#[test]
fn ironclad_starter_basic_definitions_match_java_sources() {
    let strike = get_card_definition(CardId::Strike);
    assert_eq!(strike.name, "Strike");
    assert_eq!(strike.card_type, CardType::Attack);
    assert_eq!(strike.rarity, CardRarity::Basic);
    assert_eq!(strike.cost, 1);
    assert_eq!(strike.base_damage, 6);
    assert_eq!(strike.base_block, 0);
    assert_eq!(strike.base_magic, 0);
    assert_eq!(strike.target, CardTarget::Enemy);
    assert_eq!(strike.upgrade_damage, 3);
    assert_eq!(strike.upgrade_block, 0);
    assert_eq!(strike.upgrade_magic, 0);
    assert!(strike.tags.contains(&CardTag::Strike));
    assert!(strike.tags.contains(&CardTag::StarterStrike));

    let defend = get_card_definition(CardId::Defend);
    assert_eq!(defend.name, "Defend");
    assert_eq!(defend.card_type, CardType::Skill);
    assert_eq!(defend.rarity, CardRarity::Basic);
    assert_eq!(defend.cost, 1);
    assert_eq!(defend.base_damage, 0);
    assert_eq!(defend.base_block, 5);
    assert_eq!(defend.base_magic, 0);
    assert_eq!(defend.target, CardTarget::SelfTarget);
    assert_eq!(defend.upgrade_damage, 0);
    assert_eq!(defend.upgrade_block, 3);
    assert_eq!(defend.upgrade_magic, 0);
    assert!(defend.tags.contains(&CardTag::StarterDefend));

    let bash = get_card_definition(CardId::Bash);
    assert_eq!(bash.name, "Bash");
    assert_eq!(bash.card_type, CardType::Attack);
    assert_eq!(bash.rarity, CardRarity::Basic);
    assert_eq!(bash.cost, 2);
    assert_eq!(bash.base_damage, 8);
    assert_eq!(bash.base_block, 0);
    assert_eq!(bash.base_magic, 2);
    assert_eq!(bash.target, CardTarget::Enemy);
    assert_eq!(bash.upgrade_damage, 2);
    assert_eq!(bash.upgrade_block, 0);
    assert_eq!(bash.upgrade_magic, 1);
    assert!(bash.tags.is_empty());
}

#[test]
fn ironclad_starter_basic_runtime_actions_match_java_use_methods() {
    let state = crate::test_support::blank_test_combat();

    let strike_actions = resolve_card_play(
        CardId::Strike,
        &state,
        &CombatCard::new(CardId::Strike, 1),
        Some(7),
    );
    assert_eq!(strike_actions.len(), 1);
    match &strike_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 7);
            assert_eq!(info.base, 6);
            assert_eq!(info.output, 6);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Strike should emit DamageAction, got {other:?}"),
    }

    let defend_actions = resolve_card_play(
        CardId::Defend,
        &state,
        &CombatCard::new(CardId::Defend, 2),
        None,
    );
    assert_eq!(defend_actions.len(), 1);
    match &defend_actions[0].action {
        Action::GainBlock { target, amount } => {
            assert_eq!(*target, 0);
            assert_eq!(*amount, 5);
        }
        other => panic!("Defend should emit GainBlockAction, got {other:?}"),
    }

    let bash_actions = resolve_card_play(
        CardId::Bash,
        &state,
        &CombatCard::new(CardId::Bash, 3),
        Some(7),
    );
    assert_eq!(bash_actions.len(), 2);
    match &bash_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 7);
            assert_eq!(info.base, 8);
            assert_eq!(info.output, 8);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Bash first action should be DamageAction, got {other:?}"),
    }
    match &bash_actions[1].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 7);
            assert_eq!(*power_id, PowerId::Vulnerable);
            assert_eq!(*amount, 2);
        }
        other => panic!("Bash second action should be ApplyPowerAction, got {other:?}"),
    }
}

#[test]
fn ironclad_common_utility_definitions_match_java_sources() {
    let anger = get_card_definition(CardId::Anger);
    assert_eq!(anger.name, "Anger");
    assert_eq!(anger.card_type, CardType::Attack);
    assert_eq!(anger.rarity, CardRarity::Common);
    assert_eq!(anger.cost, 0);
    assert_eq!(anger.base_damage, 6);
    assert_eq!(anger.target, CardTarget::Enemy);
    assert_eq!(anger.upgrade_damage, 2);

    let armaments = get_card_definition(CardId::Armaments);
    assert_eq!(armaments.name, "Armaments");
    assert_eq!(armaments.card_type, CardType::Skill);
    assert_eq!(armaments.rarity, CardRarity::Common);
    assert_eq!(armaments.cost, 1);
    assert_eq!(armaments.base_block, 5);
    assert_eq!(armaments.target, CardTarget::SelfTarget);
    assert_eq!(armaments.upgrade_damage, 0);
    assert_eq!(armaments.upgrade_block, 0);
    assert_eq!(armaments.upgrade_magic, 0);

    let barricade = get_card_definition(CardId::Barricade);
    assert_eq!(barricade.name, "Barricade");
    assert_eq!(barricade.card_type, CardType::Power);
    assert_eq!(barricade.rarity, CardRarity::Rare);
    assert_eq!(barricade.cost, 3);
    assert_eq!(barricade.target, CardTarget::SelfTarget);
    let mut barricade_plus = CombatCard::new(CardId::Barricade, 41);
    barricade_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&barricade_plus), Some(2));

    let battle_trance = get_card_definition(CardId::BattleTrance);
    assert_eq!(battle_trance.name, "Battle Trance");
    assert_eq!(battle_trance.card_type, CardType::Skill);
    assert_eq!(battle_trance.rarity, CardRarity::Uncommon);
    assert_eq!(battle_trance.cost, 0);
    assert_eq!(battle_trance.base_magic, 3);
    assert_eq!(battle_trance.target, CardTarget::None);
    assert_eq!(battle_trance.upgrade_magic, 1);
}

#[test]
fn ironclad_common_utility_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();

    let anger_actions = resolve_card_play(
        CardId::Anger,
        &state,
        &CombatCard::new(CardId::Anger, 10),
        Some(7),
    );
    assert_eq!(anger_actions.len(), 2);
    match &anger_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 7);
            assert_eq!(info.base, 6);
            assert_eq!(info.output, 6);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Anger first action should be DamageAction, got {other:?}"),
    }
    match &anger_actions[1].action {
        Action::MakeCopyInDiscard { original, amount } => {
            assert_eq!(original.id, CardId::Anger);
            assert_eq!(original.upgrades, 0);
            assert_eq!(*amount, 1);
        }
        other => panic!(
            "Anger second action should be MakeTempCardInDiscardAction equivalent, got {other:?}"
        ),
    }

    state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        {
            let mut upgraded = CombatCard::new(CardId::Defend, 12);
            upgraded.upgrades = 1;
            upgraded
        },
        CombatCard::new(CardId::Wound, 13),
    ];
    let mut armaments_plus = CombatCard::new(CardId::Armaments, 14);
    armaments_plus.upgrades = 1;
    let armaments_plus_actions =
        resolve_card_play(CardId::Armaments, &state, &armaments_plus, None);
    assert_eq!(armaments_plus_actions.len(), 2);
    assert!(matches!(
        armaments_plus_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 5
        }
    ));
    match &armaments_plus_actions[1].action {
        Action::UpgradeCard { card_uuid } => assert_eq!(*card_uuid, 11),
        other => panic!("Armaments+ should only upgrade canUpgrade cards, got {other:?}"),
    }

    state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 21),
        CombatCard::new(CardId::Defend, 22),
    ];
    let armaments_actions = resolve_card_play(
        CardId::Armaments,
        &state,
        &CombatCard::new(CardId::Armaments, 23),
        None,
    );
    assert_eq!(armaments_actions.len(), 2);
    match &armaments_actions[1].action {
        Action::SuspendForHandSelect {
            min,
            max,
            can_cancel,
            filter,
            reason,
        } => {
            assert_eq!(*min, 1);
            assert_eq!(*max, 1);
            assert!(!*can_cancel);
            assert_eq!(*filter, crate::state::HandSelectFilter::Upgradeable);
            assert_eq!(*reason, crate::state::HandSelectReason::Upgrade);
        }
        other => panic!("Armaments should open upgrade hand select, got {other:?}"),
    }

    let barricade_actions = resolve_card_play(
        CardId::Barricade,
        &state,
        &CombatCard::new(CardId::Barricade, 30),
        None,
    );
    assert_eq!(barricade_actions.len(), 1);
    match &barricade_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::Barricade);
            assert_eq!(*amount, -1);
        }
        other => panic!("Barricade should apply BarricadePower, got {other:?}"),
    }
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Barricade,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let duplicate_barricade_actions = resolve_card_play(
        CardId::Barricade,
        &state,
        &CombatCard::new(CardId::Barricade, 31),
        None,
    );
    assert!(duplicate_barricade_actions.is_empty());

    let battle_trance_actions = resolve_card_play(
        CardId::BattleTrance,
        &state,
        &CombatCard::new(CardId::BattleTrance, 40),
        None,
    );
    assert_eq!(battle_trance_actions.len(), 2);
    assert!(matches!(
        battle_trance_actions[0].action,
        Action::DrawCards(3)
    ));
    match &battle_trance_actions[1].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::NoDraw);
            assert_eq!(*amount, -1);
        }
        other => panic!("Battle Trance should apply NoDrawPower, got {other:?}"),
    }

    let mut battle_trance_plus = CombatCard::new(CardId::BattleTrance, 41);
    battle_trance_plus.upgrades = 1;
    let battle_trance_plus_actions =
        resolve_card_play(CardId::BattleTrance, &state, &battle_trance_plus, None);
    assert!(matches!(
        battle_trance_plus_actions[0].action,
        Action::DrawCards(4)
    ));
}

#[test]
fn ironclad_cost_and_hp_cards_definitions_match_java_sources() {
    let berserk = get_card_definition(CardId::Berserk);
    assert_eq!(berserk.name, "Berserk");
    assert_eq!(berserk.card_type, CardType::Power);
    assert_eq!(berserk.rarity, CardRarity::Rare);
    assert_eq!(berserk.cost, 0);
    assert_eq!(berserk.base_magic, 2);
    assert_eq!(berserk.target, CardTarget::SelfTarget);
    assert_eq!(berserk.upgrade_magic, -1);

    let blood_for_blood = get_card_definition(CardId::BloodForBlood);
    assert_eq!(blood_for_blood.name, "Blood for Blood");
    assert_eq!(blood_for_blood.card_type, CardType::Attack);
    assert_eq!(blood_for_blood.rarity, CardRarity::Uncommon);
    assert_eq!(blood_for_blood.cost, 4);
    assert_eq!(blood_for_blood.base_damage, 18);
    assert_eq!(blood_for_blood.target, CardTarget::Enemy);
    assert_eq!(blood_for_blood.upgrade_damage, 4);
    let mut blood_for_blood_plus = CombatCard::new(CardId::BloodForBlood, 50);
    blood_for_blood_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&blood_for_blood_plus), Some(3));

    let bloodletting = get_card_definition(CardId::Bloodletting);
    assert_eq!(bloodletting.name, "Bloodletting");
    assert_eq!(bloodletting.card_type, CardType::Skill);
    assert_eq!(bloodletting.rarity, CardRarity::Uncommon);
    assert_eq!(bloodletting.cost, 0);
    assert_eq!(bloodletting.base_magic, 2);
    assert_eq!(bloodletting.target, CardTarget::SelfTarget);
    assert_eq!(bloodletting.upgrade_magic, 1);

    let bludgeon = get_card_definition(CardId::Bludgeon);
    assert_eq!(bludgeon.name, "Bludgeon");
    assert_eq!(bludgeon.card_type, CardType::Attack);
    assert_eq!(bludgeon.rarity, CardRarity::Rare);
    assert_eq!(bludgeon.cost, 3);
    assert_eq!(bludgeon.base_damage, 32);
    assert_eq!(bludgeon.target, CardTarget::Enemy);
    assert_eq!(bludgeon.upgrade_damage, 10);
}

#[test]
fn ironclad_cost_and_hp_cards_runtime_actions_match_java_use_methods() {
    let state = crate::test_support::blank_test_combat();

    let berserk_actions = resolve_card_play(
        CardId::Berserk,
        &state,
        &CombatCard::new(CardId::Berserk, 51),
        None,
    );
    assert_eq!(berserk_actions.len(), 2);
    match &berserk_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::Vulnerable);
            assert_eq!(*amount, 2);
        }
        other => panic!("Berserk first action should apply Vulnerable, got {other:?}"),
    }
    match &berserk_actions[1].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::Berserk);
            assert_eq!(*amount, 1);
        }
        other => panic!("Berserk second action should apply BerserkPower, got {other:?}"),
    }
    let mut berserk_plus = CombatCard::new(CardId::Berserk, 52);
    berserk_plus.upgrades = 1;
    let berserk_plus_actions = resolve_card_play(CardId::Berserk, &state, &berserk_plus, None);
    match &berserk_plus_actions[0].action {
        Action::ApplyPower { amount, .. } => assert_eq!(*amount, 1),
        other => panic!("Berserk+ first action should apply 1 Vulnerable, got {other:?}"),
    }

    let blood_for_blood_actions = resolve_card_play(
        CardId::BloodForBlood,
        &state,
        &CombatCard::new(CardId::BloodForBlood, 53),
        Some(7),
    );
    assert_eq!(blood_for_blood_actions.len(), 1);
    match &blood_for_blood_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 7);
            assert_eq!(info.base, 18);
            assert_eq!(info.output, 18);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Blood for Blood should emit DamageAction, got {other:?}"),
    }

    let bloodletting_actions = resolve_card_play(
        CardId::Bloodletting,
        &state,
        &CombatCard::new(CardId::Bloodletting, 54),
        None,
    );
    assert_eq!(bloodletting_actions.len(), 2);
    match &bloodletting_actions[0].action {
        Action::LoseHp {
            target,
            amount,
            triggers_rupture,
        } => {
            assert_eq!(*target, 0);
            assert_eq!(*amount, 3);
            assert!(*triggers_rupture);
        }
        other => panic!("Bloodletting first action should be LoseHPAction, got {other:?}"),
    }
    assert!(matches!(
        bloodletting_actions[1].action,
        Action::GainEnergy { amount: 2 }
    ));
    let mut bloodletting_plus = CombatCard::new(CardId::Bloodletting, 55);
    bloodletting_plus.upgrades = 1;
    let bloodletting_plus_actions =
        resolve_card_play(CardId::Bloodletting, &state, &bloodletting_plus, None);
    assert!(matches!(
        bloodletting_plus_actions[1].action,
        Action::GainEnergy { amount: 3 }
    ));

    let bludgeon_actions = resolve_card_play(
        CardId::Bludgeon,
        &state,
        &CombatCard::new(CardId::Bludgeon, 56),
        Some(7),
    );
    assert_eq!(bludgeon_actions.len(), 1);
    match &bludgeon_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 7);
            assert_eq!(info.base, 32);
            assert_eq!(info.output, 32);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Bludgeon should emit DamageAction, got {other:?}"),
    }
    let mut bludgeon_plus = CombatCard::new(CardId::Bludgeon, 57);
    bludgeon_plus.upgrades = 1;
    let bludgeon_plus_actions =
        resolve_card_play(CardId::Bludgeon, &state, &bludgeon_plus, Some(7));
    match &bludgeon_plus_actions[0].action {
        Action::Damage(info) => assert_eq!(info.output, 42),
        other => panic!("Bludgeon+ should emit upgraded DamageAction, got {other:?}"),
    }
}

#[test]
fn blood_for_blood_cost_updates_when_player_takes_hp_loss() {
    let mut state = crate::test_support::blank_test_combat();
    state.zones.hand = vec![CombatCard::new(CardId::BloodForBlood, 60)];
    state.zones.discard_pile = vec![CombatCard::new(CardId::BloodForBlood, 61)];
    let mut upgraded = CombatCard::new(CardId::BloodForBlood, 62);
    upgraded.upgrades = 1;
    state.zones.draw_pile = vec![upgraded];

    crate::engine::action_handlers::damage::handle_lose_hp(0, 1, true, &mut state);

    assert_eq!(state.zones.hand[0].cost_modifier, -1);
    assert_eq!(state.zones.hand[0].get_cost(), 3);
    assert_eq!(state.zones.discard_pile[0].cost_modifier, -1);
    assert_eq!(state.zones.discard_pile[0].get_cost(), 3);
    assert_eq!(state.zones.draw_pile[0].cost_modifier, -1);
    assert_eq!(state.zones.draw_pile[0].get_cost(), 2);
    assert_eq!(state.turn.counters.times_damaged_this_combat, 1);
}

#[test]
fn upgraded_base_cost_is_used_when_spending_energy() {
    let mut state = crate::test_support::blank_test_combat();
    let mut barricade_plus = CombatCard::new(CardId::Barricade, 70);
    barricade_plus.upgrades = 1;
    state.zones.hand = vec![barricade_plus];
    state.turn.energy = 3;

    crate::engine::action_handlers::cards::handle_play_card_from_hand(0, None, &mut state)
        .expect("Barricade+ should be playable");

    assert_eq!(state.turn.energy, 1);
}

#[test]
fn ironclad_block_exhaust_and_ethereal_definitions_match_java_sources() {
    let body_slam = get_card_definition(CardId::BodySlam);
    assert_eq!(body_slam.name, "Body Slam");
    assert_eq!(body_slam.card_type, CardType::Attack);
    assert_eq!(body_slam.rarity, CardRarity::Common);
    assert_eq!(body_slam.cost, 1);
    assert_eq!(body_slam.base_damage, 0);
    assert_eq!(body_slam.target, CardTarget::Enemy);
    let mut body_slam_plus = CombatCard::new(CardId::BodySlam, 80);
    body_slam_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&body_slam_plus), Some(0));

    let brutality = get_card_definition(CardId::Brutality);
    assert_eq!(brutality.name, "Brutality");
    assert_eq!(brutality.card_type, CardType::Power);
    assert_eq!(brutality.rarity, CardRarity::Rare);
    assert_eq!(brutality.cost, 0);
    assert_eq!(brutality.target, CardTarget::SelfTarget);
    assert!(!brutality.innate);
    let mut brutality_plus = CombatCard::new(CardId::Brutality, 81);
    brutality_plus.upgrades = 1;
    assert!(is_innate_card(&brutality_plus));

    let burning_pact = get_card_definition(CardId::BurningPact);
    assert_eq!(burning_pact.name, "Burning Pact");
    assert_eq!(burning_pact.card_type, CardType::Skill);
    assert_eq!(burning_pact.rarity, CardRarity::Uncommon);
    assert_eq!(burning_pact.cost, 1);
    assert_eq!(burning_pact.base_magic, 2);
    assert_eq!(burning_pact.target, CardTarget::None);
    assert_eq!(burning_pact.upgrade_magic, 1);

    let carnage = get_card_definition(CardId::Carnage);
    assert_eq!(carnage.name, "Carnage");
    assert_eq!(carnage.card_type, CardType::Attack);
    assert_eq!(carnage.rarity, CardRarity::Uncommon);
    assert_eq!(carnage.cost, 2);
    assert_eq!(carnage.base_damage, 20);
    assert_eq!(carnage.target, CardTarget::Enemy);
    assert!(carnage.ethereal);
    assert_eq!(carnage.upgrade_damage, 8);
}

#[test]
fn ironclad_block_exhaust_and_ethereal_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    state.entities.player.block = 13;

    let body_slam_actions = resolve_card_play(
        CardId::BodySlam,
        &state,
        &CombatCard::new(CardId::BodySlam, 82),
        Some(7),
    );
    assert_eq!(body_slam_actions.len(), 1);
    match &body_slam_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 7);
            assert_eq!(info.base, 13);
            assert_eq!(info.output, 13);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Body Slam should emit block-based DamageAction, got {other:?}"),
    }

    let brutality_actions = resolve_card_play(
        CardId::Brutality,
        &state,
        &CombatCard::new(CardId::Brutality, 83),
        None,
    );
    assert_eq!(brutality_actions.len(), 1);
    match &brutality_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::Brutality);
            assert_eq!(*amount, 1);
        }
        other => panic!("Brutality should apply BrutalityPower, got {other:?}"),
    }

    state.zones.hand = vec![CombatCard::new(CardId::Strike, 84)];
    let burning_pact_actions = resolve_card_play(
        CardId::BurningPact,
        &state,
        &CombatCard::new(CardId::BurningPact, 85),
        None,
    );
    assert_eq!(burning_pact_actions.len(), 2);
    assert_eq!(
        burning_pact_actions[0].action,
        Action::ExhaustFromHand {
            amount: 1,
            random: false,
            any_number: false,
            can_pick_zero: false,
        },
        "Java Burning Pact queues ExhaustAction; it reads hand size when the action executes"
    );
    assert!(matches!(
        burning_pact_actions[1].action,
        Action::DrawCards(2)
    ));

    state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 86),
        CombatCard::new(CardId::Defend, 87),
    ];
    let burning_pact_select_actions = resolve_card_play(
        CardId::BurningPact,
        &state,
        &CombatCard::new(CardId::BurningPact, 88),
        None,
    );
    assert_eq!(
        burning_pact_select_actions[0].action,
        Action::ExhaustFromHand {
            amount: 1,
            random: false,
            any_number: false,
            can_pick_zero: false,
        }
    );
    let mut burning_pact_plus = CombatCard::new(CardId::BurningPact, 89);
    burning_pact_plus.upgrades = 1;
    let burning_pact_plus_actions =
        resolve_card_play(CardId::BurningPact, &state, &burning_pact_plus, None);
    assert!(matches!(
        burning_pact_plus_actions[1].action,
        Action::DrawCards(3)
    ));

    let carnage_actions = resolve_card_play(
        CardId::Carnage,
        &state,
        &CombatCard::new(CardId::Carnage, 90),
        Some(7),
    );
    assert_eq!(carnage_actions.len(), 1);
    match &carnage_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 7);
            assert_eq!(info.base, 20);
            assert_eq!(info.output, 20);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Carnage should emit DamageAction, got {other:?}"),
    }
    let mut carnage_plus = CombatCard::new(CardId::Carnage, 91);
    carnage_plus.upgrades = 1;
    let carnage_plus_actions = resolve_card_play(CardId::Carnage, &state, &carnage_plus, Some(7));
    match &carnage_plus_actions[0].action {
        Action::Damage(info) => assert_eq!(info.output, 28),
        other => panic!("Carnage+ should emit upgraded DamageAction, got {other:?}"),
    }
}

#[test]
fn ironclad_attack_condition_and_dot_power_definitions_match_java_sources() {
    let clash = get_card_definition(CardId::Clash);
    assert_eq!(clash.name, "Clash");
    assert_eq!(clash.card_type, CardType::Attack);
    assert_eq!(clash.rarity, CardRarity::Common);
    assert_eq!(clash.cost, 0);
    assert_eq!(clash.base_damage, 14);
    assert_eq!(clash.target, CardTarget::Enemy);
    assert_eq!(clash.upgrade_damage, 4);

    let cleave = get_card_definition(CardId::Cleave);
    assert_eq!(cleave.name, "Cleave");
    assert_eq!(cleave.card_type, CardType::Attack);
    assert_eq!(cleave.rarity, CardRarity::Common);
    assert_eq!(cleave.cost, 1);
    assert_eq!(cleave.base_damage, 8);
    assert!(cleave.is_multi_damage);
    assert_eq!(cleave.target, CardTarget::AllEnemy);
    assert_eq!(cleave.upgrade_damage, 3);

    let clothesline = get_card_definition(CardId::Clothesline);
    assert_eq!(clothesline.name, "Clothesline");
    assert_eq!(clothesline.card_type, CardType::Attack);
    assert_eq!(clothesline.rarity, CardRarity::Common);
    assert_eq!(clothesline.cost, 2);
    assert_eq!(clothesline.base_damage, 12);
    assert_eq!(clothesline.base_magic, 2);
    assert_eq!(clothesline.target, CardTarget::Enemy);
    assert_eq!(clothesline.upgrade_damage, 2);
    assert_eq!(clothesline.upgrade_magic, 1);

    let combust = get_card_definition(CardId::Combust);
    assert_eq!(combust.name, "Combust");
    assert_eq!(combust.card_type, CardType::Power);
    assert_eq!(combust.rarity, CardRarity::Uncommon);
    assert_eq!(combust.cost, 1);
    assert_eq!(combust.base_magic, 5);
    assert_eq!(combust.target, CardTarget::SelfTarget);
    assert_eq!(combust.upgrade_magic, 2);
}

#[test]
fn ironclad_attack_condition_and_dot_power_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();

    state.zones.hand = vec![
        CombatCard::new(CardId::Clash, 100),
        CombatCard::new(CardId::Strike, 101),
    ];
    assert!(can_play_card(&state.zones.hand[0], &state).is_ok());
    let mut clash_plus = CombatCard::new(CardId::Clash, 102);
    clash_plus.upgrades = 1;
    let clash_plus_actions = resolve_card_play(CardId::Clash, &state, &clash_plus, Some(7));
    assert_eq!(clash_plus_actions.len(), 1);
    match &clash_plus_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 7);
            assert_eq!(info.base, 18);
            assert_eq!(info.output, 18);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Clash+ should emit upgraded DamageAction, got {other:?}"),
    }

    state.zones.hand = vec![
        CombatCard::new(CardId::Clash, 103),
        CombatCard::new(CardId::Defend, 104),
    ];
    assert!(can_play_card(&state.zones.hand[0], &state).is_err());
    let forced_clash = CombatCard::new(CardId::Clash, 105);
    let forced_clash_actions = resolve_card_play(CardId::Clash, &state, &forced_clash, Some(7));
    assert_eq!(
        forced_clash_actions.len(),
        1,
        "Clash.canUse gates manual play only; forced play still runs Clash.use()"
    );
    assert!(matches!(
        forced_clash_actions[0].action,
        Action::Damage(DamageInfo {
            source: 0,
            target: 7,
            base: 14,
            output: 14,
            damage_type: DamageType::Normal,
            ..
        })
    ));

    let mut autoplay_clash_state = crate::test_support::blank_test_combat();
    let mut clash_target = crate::test_support::test_monster(EnemyId::JawWorm);
    clash_target.id = 7;
    autoplay_clash_state.entities.monsters = vec![clash_target.clone()];
    autoplay_clash_state.zones.hand = vec![CombatCard::new(CardId::Defend, 106)];
    autoplay_clash_state.enqueue_card_play(
        crate::runtime::combat::QueuedCardPlay {
            card: CombatCard::new(CardId::Clash, 107),
            target: Some(7),
            energy_on_use: 0,
            ignore_energy_total: true,
            autoplay: true,
            random_target: false,
            is_end_turn_autoplay: false,
            purge_on_use: false,
            source: crate::runtime::combat::QueuedCardSource::Normal,
        },
        false,
    );
    let flush_autoplay_clash = autoplay_clash_state
        .pop_next_action()
        .expect("autoplay Clash should schedule queue flush");
    crate::engine::action_handlers::execute_action(flush_autoplay_clash, &mut autoplay_clash_state);
    let fizzled_clash_cleanup = autoplay_clash_state.pop_next_action();
    assert!(
        matches!(
            fizzled_clash_cleanup,
            Some(Action::UseCardDone {
                should_exhaust: false
            })
        ),
        "Java queued/autoplay cards still call canUse; failed autoplay still resolves UseCardAction cleanup"
    );
    crate::engine::action_handlers::execute_action(
        fizzled_clash_cleanup.unwrap(),
        &mut autoplay_clash_state,
    );
    assert_eq!(autoplay_clash_state.zones.discard_pile.len(), 1);
    assert_eq!(autoplay_clash_state.zones.discard_pile[0].id, CardId::Clash);

    let mut fizzle_then_continue_state = crate::test_support::blank_test_combat();
    fizzle_then_continue_state.entities.monsters = vec![clash_target.clone()];
    fizzle_then_continue_state.zones.hand = vec![CombatCard::new(CardId::Defend, 109)];
    fizzle_then_continue_state.enqueue_card_play(
        crate::runtime::combat::QueuedCardPlay {
            card: CombatCard::new(CardId::Clash, 110),
            target: Some(7),
            energy_on_use: 0,
            ignore_energy_total: true,
            autoplay: true,
            random_target: false,
            is_end_turn_autoplay: false,
            purge_on_use: false,
            source: crate::runtime::combat::QueuedCardSource::Normal,
        },
        false,
    );
    fizzle_then_continue_state.enqueue_card_play(
        crate::runtime::combat::QueuedCardPlay {
            card: CombatCard::new(CardId::Strike, 111),
            target: Some(7),
            energy_on_use: 0,
            ignore_energy_total: true,
            autoplay: true,
            random_target: false,
            is_end_turn_autoplay: false,
            purge_on_use: false,
            source: crate::runtime::combat::QueuedCardSource::Normal,
        },
        false,
    );
    let first_flush = fizzle_then_continue_state
        .pop_next_action()
        .expect("first queued card should schedule flush");
    crate::engine::action_handlers::execute_action(first_flush, &mut fizzle_then_continue_state);
    let cleanup = fizzle_then_continue_state
        .pop_next_action()
        .expect("failed autoplay should clean up before later queued cards");
    assert!(matches!(cleanup, Action::UseCardDone { .. }));
    crate::engine::action_handlers::execute_action(cleanup, &mut fizzle_then_continue_state);
    assert_eq!(
        fizzle_then_continue_state.pop_next_action(),
        Some(Action::FlushNextQueuedCard),
        "a failed autoplay card must not strand later queued cards"
    );

    let mut autoplay_no_energy_state = crate::test_support::blank_test_combat();
    autoplay_no_energy_state.turn.energy = 0;
    autoplay_no_energy_state.entities.monsters = vec![clash_target];
    autoplay_no_energy_state.enqueue_card_play(
        crate::runtime::combat::QueuedCardPlay {
            card: CombatCard::new(CardId::Strike, 108),
            target: Some(7),
            energy_on_use: 0,
            ignore_energy_total: true,
            autoplay: true,
            random_target: false,
            is_end_turn_autoplay: false,
            purge_on_use: false,
            source: crate::runtime::combat::QueuedCardSource::Normal,
        },
        false,
    );
    let flush_autoplay_strike = autoplay_no_energy_state
        .pop_next_action()
        .expect("autoplay Strike should schedule queue flush");
    crate::engine::action_handlers::execute_action(
        flush_autoplay_strike,
        &mut autoplay_no_energy_state,
    );
    assert!(
        matches!(
            autoplay_no_energy_state.pop_next_action(),
            Some(Action::PlayCardDirect { .. })
        ),
        "Java isInAutoplay bypasses only the final energy check"
    );

    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 11;
    first.slot = 0;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 12;
    second.slot = 1;
    state.entities.monsters = vec![first, second];

    let mut cleave_plus = CombatCard::new(CardId::Cleave, 105);
    cleave_plus.upgrades = 1;
    let cleave_plus_actions = resolve_card_play(CardId::Cleave, &state, &cleave_plus, None);
    assert_eq!(cleave_plus_actions.len(), 1);
    match &cleave_plus_actions[0].action {
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            is_modified,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(damages.as_slice(), &[11, 11]);
            assert_eq!(*damage_type, DamageType::Normal);
            assert!(!*is_modified);
        }
        other => panic!("Cleave+ should emit DamageAllEnemiesAction, got {other:?}"),
    }

    let mut clothesline_plus = CombatCard::new(CardId::Clothesline, 106);
    clothesline_plus.upgrades = 1;
    let clothesline_plus_actions =
        resolve_card_play(CardId::Clothesline, &state, &clothesline_plus, Some(11));
    assert_eq!(clothesline_plus_actions.len(), 2);
    match &clothesline_plus_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 11);
            assert_eq!(info.base, 14);
            assert_eq!(info.output, 14);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Clothesline+ first action should be DamageAction, got {other:?}"),
    }
    match &clothesline_plus_actions[1].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 11);
            assert_eq!(*power_id, PowerId::Weak);
            assert_eq!(*amount, 3);
        }
        other => panic!("Clothesline+ second action should apply Weak, got {other:?}"),
    }

    let mut combust_plus = CombatCard::new(CardId::Combust, 107);
    combust_plus.upgrades = 1;
    let combust_plus_actions = resolve_card_play(CardId::Combust, &state, &combust_plus, None);
    assert_eq!(combust_plus_actions.len(), 1);
    match &combust_plus_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::Combust);
            assert_eq!(*amount, 7);
        }
        other => panic!("Combust+ should apply CombustPower with upgraded damage, got {other:?}"),
    }
}

#[test]
fn combust_power_stacks_damage_and_hp_loss_like_java_source() {
    let mut state = crate::test_support::blank_test_combat();

    crate::engine::action_handlers::powers::handle_apply_power(
        0,
        0,
        PowerId::Combust,
        5,
        &mut state,
    );
    crate::engine::action_handlers::powers::handle_apply_power(
        0,
        0,
        PowerId::Combust,
        7,
        &mut state,
    );

    let combust_power = crate::content::powers::store::powers_for(&state, 0)
        .and_then(|powers| powers.iter().find(|p| p.power_type == PowerId::Combust))
        .cloned()
        .expect("Combust power should be stored on the player");
    assert_eq!(combust_power.amount, 12);
    assert_eq!(combust_power.extra_data, 2);

    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 21;
    first.slot = 0;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 22;
    second.slot = 1;
    state.entities.monsters = vec![first, second];

    let actions = crate::content::powers::resolve_power_at_end_of_turn(&combust_power, &state, 0);
    assert_eq!(actions.len(), 2);
    match &actions[0] {
        Action::LoseHp {
            target,
            amount,
            triggers_rupture,
        } => {
            assert_eq!(*target, 0);
            assert_eq!(*amount, 2);
            assert!(*triggers_rupture);
        }
        other => panic!("Combust should lose stored hpLoss first, got {other:?}"),
    }
    match &actions[1] {
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            is_modified,
        } => {
            assert_eq!(*source, NO_SOURCE);
            assert_eq!(damages.as_slice(), &[12, 12]);
            assert_eq!(*damage_type, DamageType::Thorns);
            assert!(!*is_modified);
        }
        other => panic!("Combust should damage all enemies second, got {other:?}"),
    }

    for monster in &mut state.entities.monsters {
        monster.current_hp = 0;
        monster.is_dying = false;
    }
    let zero_hp_not_dying_actions =
        crate::content::powers::resolve_power_at_end_of_turn(&combust_power, &state, 0);
    assert_eq!(
        zero_hp_not_dying_actions.len(),
        2,
        "Java areMonstersBasicallyDead ignores currentHealth; Combust still fires until monsters are isDying/isEscaping"
    );

    for monster in &mut state.entities.monsters {
        monster.current_hp = 0;
        monster.is_dying = true;
    }
    let no_monster_actions =
        crate::content::powers::resolve_power_at_end_of_turn(&combust_power, &state, 0);
    assert!(
        no_monster_actions.is_empty(),
        "Java skips Combust atEndOfTurn when monsters are basically dead"
    );
}

#[test]
fn ironclad_power_and_debuff_definitions_match_java_sources() {
    let corruption = get_card_definition(CardId::Corruption);
    assert_eq!(corruption.name, "Corruption");
    assert_eq!(corruption.card_type, CardType::Power);
    assert_eq!(corruption.rarity, CardRarity::Rare);
    assert_eq!(corruption.cost, 3);
    assert_eq!(corruption.base_magic, 3);
    assert_eq!(corruption.target, CardTarget::SelfTarget);
    let mut corruption_plus = CombatCard::new(CardId::Corruption, 110);
    corruption_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&corruption_plus), Some(2));

    let dark_embrace = get_card_definition(CardId::DarkEmbrace);
    assert_eq!(dark_embrace.name, "Dark Embrace");
    assert_eq!(dark_embrace.card_type, CardType::Power);
    assert_eq!(dark_embrace.rarity, CardRarity::Uncommon);
    assert_eq!(dark_embrace.cost, 2);
    assert_eq!(dark_embrace.target, CardTarget::SelfTarget);
    let mut dark_embrace_plus = CombatCard::new(CardId::DarkEmbrace, 111);
    dark_embrace_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&dark_embrace_plus), Some(1));

    let demon_form = get_card_definition(CardId::DemonForm);
    assert_eq!(demon_form.name, "Demon Form");
    assert_eq!(demon_form.card_type, CardType::Power);
    assert_eq!(demon_form.rarity, CardRarity::Rare);
    assert_eq!(demon_form.cost, 3);
    assert_eq!(demon_form.base_magic, 2);
    assert_eq!(demon_form.target, CardTarget::None);
    assert_eq!(demon_form.upgrade_magic, 1);

    let disarm = get_card_definition(CardId::Disarm);
    assert_eq!(disarm.name, "Disarm");
    assert_eq!(disarm.card_type, CardType::Skill);
    assert_eq!(disarm.rarity, CardRarity::Uncommon);
    assert_eq!(disarm.cost, 1);
    assert_eq!(disarm.base_magic, 2);
    assert_eq!(disarm.target, CardTarget::Enemy);
    assert!(disarm.exhaust);
    assert_eq!(disarm.upgrade_magic, 1);
}

#[test]
fn ironclad_power_and_debuff_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();

    let corruption_actions = resolve_card_play(
        CardId::Corruption,
        &state,
        &CombatCard::new(CardId::Corruption, 112),
        None,
    );
    assert_eq!(corruption_actions.len(), 1);
    match &corruption_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::Corruption);
            assert_eq!(*amount, -1);
        }
        other => panic!("Corruption should apply CorruptionPower once, got {other:?}"),
    }

    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Corruption,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let duplicate_corruption_actions = resolve_card_play(
        CardId::Corruption,
        &state,
        &CombatCard::new(CardId::Corruption, 113),
        None,
    );
    assert!(duplicate_corruption_actions.is_empty());

    let mut corruption_apply_state = crate::test_support::blank_test_combat();
    let mut hand_skill = CombatCard::new(CardId::Defend, 910);
    hand_skill.cost_modifier = 2;
    corruption_apply_state.zones.hand = vec![hand_skill];
    corruption_apply_state.zones.draw_pile = vec![CombatCard::new(CardId::ShrugItOff, 911)];
    corruption_apply_state.zones.discard_pile = vec![CombatCard::new(CardId::BurningPact, 912)];
    corruption_apply_state.zones.exhaust_pile = vec![CombatCard::new(CardId::PowerThrough, 913)];
    corruption_apply_state.zones.limbo = vec![CombatCard::new(CardId::TrueGrit, 914)];

    crate::content::cards::ironclad::corruption::corruption_on_apply(&mut corruption_apply_state);

    assert_eq!(corruption_apply_state.zones.hand[0].cost_for_turn, Some(0));
    assert_eq!(
        corruption_apply_state.zones.hand[0].cost_modifier, -1,
        "Java ApplyPowerAction applies Corruption with modifyCostForCombat(-9), mutating combat cost too"
    );
    assert_eq!(corruption_apply_state.zones.draw_pile[0].cost_modifier, -1);
    assert_eq!(
        corruption_apply_state.zones.discard_pile[0].cost_modifier,
        -1
    );
    assert_eq!(
        corruption_apply_state.zones.exhaust_pile[0].cost_modifier,
        -1
    );
    assert_eq!(
        corruption_apply_state.zones.draw_pile[0].cost_for_turn,
        Some(0)
    );
    assert_eq!(
        corruption_apply_state.zones.discard_pile[0].cost_for_turn,
        Some(0)
    );
    assert_eq!(
        corruption_apply_state.zones.exhaust_pile[0].cost_for_turn,
        Some(0)
    );
    assert_eq!(
        corruption_apply_state.zones.limbo[0].cost_for_turn, None,
        "Java ApplyPowerAction's Corruption constructor scans hand/draw/discard/exhaust, not limbo"
    );

    let dark_embrace_actions = resolve_card_play(
        CardId::DarkEmbrace,
        &state,
        &CombatCard::new(CardId::DarkEmbrace, 114),
        None,
    );
    assert_eq!(dark_embrace_actions.len(), 1);
    match &dark_embrace_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::DarkEmbrace);
            assert_eq!(*amount, 1);
        }
        other => panic!("Dark Embrace should apply DarkEmbracePower(1), got {other:?}"),
    }

    let mut demon_form_plus = CombatCard::new(CardId::DemonForm, 115);
    demon_form_plus.upgrades = 1;
    let demon_form_plus_actions =
        resolve_card_play(CardId::DemonForm, &state, &demon_form_plus, None);
    assert_eq!(demon_form_plus_actions.len(), 1);
    match &demon_form_plus_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::DemonForm);
            assert_eq!(*amount, 3);
        }
        other => panic!("Demon Form+ should apply upgraded DemonFormPower, got {other:?}"),
    }

    let mut disarm_plus = CombatCard::new(CardId::Disarm, 116);
    disarm_plus.upgrades = 1;
    let disarm_plus_actions = resolve_card_play(CardId::Disarm, &state, &disarm_plus, Some(23));
    assert_eq!(disarm_plus_actions.len(), 1);
    match &disarm_plus_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 23);
            assert_eq!(*power_id, PowerId::Strength);
            assert_eq!(*amount, -3);
        }
        other => panic!("Disarm+ should apply negative Strength to target, got {other:?}"),
    }
}

#[test]
fn dark_embrace_and_demon_form_power_hooks_match_java_sources() {
    let mut state = crate::test_support::blank_test_combat();
    let no_monster_dark_embrace = crate::content::powers::resolve_power_on_exhaust(
        PowerId::DarkEmbrace,
        &state,
        0,
        1,
        117,
        CardId::Strike,
    );
    assert!(
        no_monster_dark_embrace.is_empty(),
        "Java skips Dark Embrace draw when monsters are basically dead"
    );

    let mut zero_hp_not_dying = crate::test_support::test_monster(EnemyId::JawWorm);
    zero_hp_not_dying.current_hp = 0;
    zero_hp_not_dying.is_dying = false;
    state.entities.monsters.push(zero_hp_not_dying);
    let zero_hp_dark_embrace = crate::content::powers::resolve_power_on_exhaust(
        PowerId::DarkEmbrace,
        &state,
        0,
        1,
        118,
        CardId::Strike,
    );
    assert_eq!(
        zero_hp_dark_embrace.len(),
        1,
        "Java areMonstersBasicallyDead ignores currentHealth; Dark Embrace still draws until monsters are isDying/isEscaping"
    );

    state.entities.monsters[0].current_hp = 20;
    state.entities.monsters[0].is_dying = false;
    let dark_embrace_actions = crate::content::powers::resolve_power_on_exhaust(
        PowerId::DarkEmbrace,
        &state,
        0,
        1,
        119,
        CardId::Strike,
    );
    assert_eq!(dark_embrace_actions.len(), 1);
    assert!(matches!(dark_embrace_actions[0], Action::DrawCards(1)));

    let demon_form_actions =
        crate::content::powers::resolve_power_on_post_draw(PowerId::DemonForm, &state, 0, 3);
    assert_eq!(demon_form_actions.len(), 1);
    match &demon_form_actions[0] {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::Strength);
            assert_eq!(*amount, 3);
        }
        other => panic!("Demon Form post-draw hook should apply Strength, got {other:?}"),
    }
}

#[test]
fn corruption_power_on_apply_modifies_skill_costs_in_java_piles() {
    let mut state = crate::test_support::blank_test_combat();
    state.zones.hand = vec![
        CombatCard::new(CardId::Defend, 120),
        CombatCard::new(CardId::Strike, 121),
    ];
    state.zones.draw_pile = vec![CombatCard::new(CardId::Armaments, 122)];
    state.zones.discard_pile = vec![CombatCard::new(CardId::Disarm, 123)];
    state.zones.exhaust_pile = vec![CombatCard::new(CardId::BurningPact, 124)];

    crate::engine::action_handlers::powers::handle_apply_power(
        0,
        0,
        PowerId::Corruption,
        -1,
        &mut state,
    );

    assert_eq!(state.zones.hand[0].get_cost(), 0);
    assert_eq!(state.zones.hand[1].get_cost(), 1);
    assert_eq!(state.zones.draw_pile[0].get_cost(), 0);
    assert_eq!(state.zones.discard_pile[0].get_cost(), 0);
    assert_eq!(state.zones.exhaust_pile[0].get_cost(), 0);
}

#[test]
fn ironclad_copy_and_block_definitions_match_java_sources() {
    let double_tap = get_card_definition(CardId::DoubleTap);
    assert_eq!(double_tap.name, "Double Tap");
    assert_eq!(double_tap.card_type, CardType::Skill);
    assert_eq!(double_tap.rarity, CardRarity::Rare);
    assert_eq!(double_tap.cost, 1);
    assert_eq!(double_tap.base_magic, 1);
    assert_eq!(double_tap.target, CardTarget::SelfTarget);
    assert_eq!(double_tap.upgrade_magic, 1);

    let dropkick = get_card_definition(CardId::Dropkick);
    assert_eq!(dropkick.name, "Dropkick");
    assert_eq!(dropkick.card_type, CardType::Attack);
    assert_eq!(dropkick.rarity, CardRarity::Uncommon);
    assert_eq!(dropkick.cost, 1);
    assert_eq!(dropkick.base_damage, 5);
    assert_eq!(dropkick.target, CardTarget::Enemy);
    assert_eq!(dropkick.upgrade_damage, 3);

    let dual_wield = get_card_definition(CardId::DualWield);
    assert_eq!(dual_wield.name, "Dual Wield");
    assert_eq!(dual_wield.card_type, CardType::Skill);
    assert_eq!(dual_wield.rarity, CardRarity::Uncommon);
    assert_eq!(dual_wield.cost, 1);
    assert_eq!(dual_wield.base_magic, 1);
    assert_eq!(dual_wield.target, CardTarget::None);
    assert_eq!(dual_wield.upgrade_magic, 1);

    let entrench = get_card_definition(CardId::Entrench);
    assert_eq!(entrench.name, "Entrench");
    assert_eq!(entrench.card_type, CardType::Skill);
    assert_eq!(entrench.rarity, CardRarity::Uncommon);
    assert_eq!(entrench.cost, 2);
    assert_eq!(entrench.target, CardTarget::SelfTarget);
    let mut entrench_plus = CombatCard::new(CardId::Entrench, 130);
    entrench_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&entrench_plus), Some(1));
}

#[test]
fn ironclad_copy_and_block_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();

    let mut double_tap_plus = CombatCard::new(CardId::DoubleTap, 131);
    double_tap_plus.upgrades = 1;
    let double_tap_plus_actions =
        resolve_card_play(CardId::DoubleTap, &state, &double_tap_plus, None);
    assert_eq!(double_tap_plus_actions.len(), 1);
    match &double_tap_plus_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::DoubleTap);
            assert_eq!(*amount, 2);
        }
        other => panic!("Double Tap+ should apply DoubleTapPower(2), got {other:?}"),
    }

    let mut dropkick_plus = CombatCard::new(CardId::Dropkick, 132);
    dropkick_plus.upgrades = 1;
    let dropkick_plus_actions =
        resolve_card_play(CardId::Dropkick, &state, &dropkick_plus, Some(7));
    assert_eq!(dropkick_plus_actions.len(), 1);
    match &dropkick_plus_actions[0].action {
        Action::DropkickDamageAndEffect {
            target,
            damage_info,
        } => {
            assert_eq!(*target, 7);
            assert_eq!(damage_info.source, 0);
            assert_eq!(damage_info.target, 7);
            assert_eq!(damage_info.base, 8);
            assert_eq!(damage_info.output, 8);
            assert_eq!(damage_info.damage_type, DamageType::Normal);
        }
        other => panic!("Dropkick+ should emit DropkickAction equivalent, got {other:?}"),
    }

    state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 133),
        CombatCard::new(CardId::Defend, 134),
    ];
    let mut dual_wield_plus = CombatCard::new(CardId::DualWield, 135);
    dual_wield_plus.upgrades = 1;
    let dual_wield_auto_actions =
        resolve_card_play(CardId::DualWield, &state, &dual_wield_plus, None);
    assert_eq!(dual_wield_auto_actions.len(), 1);
    match &dual_wield_auto_actions[0].action {
        Action::MakeCopyInHand { original, amount } => {
            assert_eq!(original.id, CardId::Strike);
            assert_eq!(*amount, 2);
        }
        other => panic!("Dual Wield+ should auto-copy sole valid card, got {other:?}"),
    }

    state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 136),
        CombatCard::new(CardId::Inflame, 137),
        CombatCard::new(CardId::Defend, 138),
    ];
    let dual_wield_select_actions =
        resolve_card_play(CardId::DualWield, &state, &dual_wield_plus, None);
    assert_eq!(dual_wield_select_actions.len(), 1);
    match &dual_wield_select_actions[0].action {
        Action::SuspendForHandSelect {
            min,
            max,
            can_cancel,
            filter,
            reason,
        } => {
            assert_eq!(*min, 1);
            assert_eq!(*max, 1);
            assert!(!*can_cancel);
            assert_eq!(*filter, crate::state::HandSelectFilter::AttackOrPower);
            assert_eq!(*reason, crate::state::HandSelectReason::Copy { amount: 3 });
        }
        other => panic!("Dual Wield should open Attack/Power hand select, got {other:?}"),
    }

    state.entities.player.block = 14;
    let entrench_actions = resolve_card_play(
        CardId::Entrench,
        &state,
        &CombatCard::new(CardId::Entrench, 139),
        None,
    );
    assert_eq!(entrench_actions.len(), 1);
    assert!(matches!(
        entrench_actions[0].action,
        Action::DoubleBlock { target: 0 }
    ));

    state.entities.player.block = 9;
    crate::engine::action_handlers::damage::handle_double_block(0, &mut state);
    assert_eq!(
        state.entities.player.block, 18,
        "Java DoubleYourBlockAction reads currentBlock when the action updates"
    );
}

#[test]
fn dropkick_and_double_tap_action_hooks_match_java_sources() {
    let mut state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut state,
        7,
        vec![Power {
            power_type: PowerId::Vulnerable,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::damage::handle_dropkick(
        7,
        crate::runtime::action::DamageInfo {
            source: 0,
            target: 7,
            base: 5,
            output: 5,
            damage_type: DamageType::Normal,
            is_modified: false,
        },
        &mut state,
    );
    assert!(matches!(
        state.pop_next_action(),
        Some(Action::Damage(crate::runtime::action::DamageInfo {
            target: 7,
            ..
        }))
    ));
    assert!(matches!(
        state.pop_next_action(),
        Some(Action::GainEnergy { amount: 1 })
    ));
    assert!(matches!(
        state.pop_next_action(),
        Some(Action::DrawCards(1))
    ));

    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::DoubleTap,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let mut strike = CombatCard::new(CardId::Strike, 140);
    strike.base_damage_mut = 99;
    crate::content::powers::ironclad::double_tap::on_use_card(&mut state, &strike, false, Some(7));
    assert_eq!(
        crate::content::powers::store::power_amount(&state, 0, PowerId::DoubleTap),
        0
    );
    match state.pop_next_action() {
        Some(Action::EnqueueCardPlay { item, in_front }) => {
            assert!(in_front);
            assert_eq!(item.card.id, CardId::Strike);
            assert_eq!(item.target, Some(7));
            assert!(item.ignore_energy_total);
            assert!(item.autoplay);
            assert!(item.purge_on_use);
            assert_eq!(
                item.card.uuid, 140,
                "Java DoubleTapPower uses makeSameInstanceOf(), preserving UUID"
            );
            assert_eq!(
                item.card.base_damage_mut, 0,
                "makeSameInstanceOf is a stat-equivalent copy, not a raw clone of transient evaluated damage"
            );
            assert_eq!(
                item.source,
                crate::runtime::combat::QueuedCardSource::DoubleTap
            );
        }
        other => panic!("Double Tap should enqueue a purge-on-use copy, got {other:?}"),
    }
}

#[test]
fn same_instance_replay_powers_preserve_card_uuid_like_java() {
    let mut burst_state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut burst_state,
        0,
        vec![Power {
            power_type: PowerId::Burst,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let mut shrug = CombatCard::new(CardId::ShrugItOff, 920);
    shrug.base_block_mut = 99;
    crate::content::powers::silent::burst::on_use_card(&mut burst_state, &shrug, false, None);
    match burst_state.pop_next_action() {
        Some(Action::EnqueueCardPlay { item, .. }) => {
            assert_eq!(item.card.uuid, 920);
            assert_eq!(item.card.base_block_mut, 0);
            assert_eq!(item.source, crate::runtime::combat::QueuedCardSource::Burst);
        }
        other => panic!("Burst should enqueue same-instance skill copy, got {other:?}"),
    }

    let mut duplication_state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut duplication_state,
        0,
        vec![Power {
            power_type: PowerId::DuplicationPower,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let mut strike = CombatCard::new(CardId::Strike, 921);
    strike.base_damage_mut = 99;
    crate::content::powers::core::duplication_power::on_use_card(
        &mut duplication_state,
        &strike,
        false,
        Some(7),
    );
    match duplication_state.pop_next_action() {
        Some(Action::EnqueueCardPlay { item, .. }) => {
            assert_eq!(item.card.uuid, 921);
            assert_eq!(item.card.base_damage_mut, 0);
            assert_eq!(
                item.source,
                crate::runtime::combat::QueuedCardSource::Duplication
            );
        }
        other => panic!("DuplicationPower should enqueue same-instance copy, got {other:?}"),
    }
}

#[test]
fn ironclad_exhaust_and_growth_definitions_match_java_sources() {
    let evolve = get_card_definition(CardId::Evolve);
    assert_eq!(evolve.card_type, CardType::Power);
    assert_eq!(evolve.rarity, CardRarity::Uncommon);
    assert_eq!(evolve.cost, 1);
    assert_eq!(evolve.base_magic, 1);
    assert_eq!(evolve.target, CardTarget::SelfTarget);
    assert_eq!(evolve.upgrade_magic, 1);

    let exhume = get_card_definition(CardId::Exhume);
    assert_eq!(exhume.card_type, CardType::Skill);
    assert_eq!(exhume.rarity, CardRarity::Rare);
    assert_eq!(exhume.cost, 1);
    assert_eq!(exhume.target, CardTarget::None);
    assert!(exhume.exhaust);
    let mut exhume_plus = CombatCard::new(CardId::Exhume, 150);
    exhume_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&exhume_plus), Some(0));

    let feed = get_card_definition(CardId::Feed);
    assert_eq!(feed.card_type, CardType::Attack);
    assert_eq!(feed.rarity, CardRarity::Rare);
    assert_eq!(feed.cost, 1);
    assert_eq!(feed.base_damage, 10);
    assert_eq!(feed.base_magic, 3);
    assert_eq!(feed.target, CardTarget::Enemy);
    assert!(feed.exhaust);
    assert!(feed.tags.contains(&CardTag::Healing));
    assert_eq!(feed.upgrade_damage, 2);
    assert_eq!(feed.upgrade_magic, 1);

    let feel_no_pain = get_card_definition(CardId::FeelNoPain);
    assert_eq!(feel_no_pain.card_type, CardType::Power);
    assert_eq!(feel_no_pain.rarity, CardRarity::Uncommon);
    assert_eq!(feel_no_pain.cost, 1);
    assert_eq!(feel_no_pain.base_magic, 3);
    assert_eq!(feel_no_pain.target, CardTarget::SelfTarget);
    assert_eq!(feel_no_pain.upgrade_magic, 1);
}

#[test]
fn ironclad_exhaust_and_growth_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();

    let mut evolve_plus = CombatCard::new(CardId::Evolve, 151);
    evolve_plus.upgrades = 1;
    let evolve_actions = resolve_card_play(CardId::Evolve, &state, &evolve_plus, None);
    assert_eq!(evolve_actions.len(), 1);
    match &evolve_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::Evolve);
            assert_eq!(*amount, 2);
        }
        other => panic!("Evolve+ should apply upgraded EvolvePower, got {other:?}"),
    }

    let mut feel_no_pain_plus = CombatCard::new(CardId::FeelNoPain, 152);
    feel_no_pain_plus.upgrades = 1;
    let feel_no_pain_actions =
        resolve_card_play(CardId::FeelNoPain, &state, &feel_no_pain_plus, None);
    assert_eq!(feel_no_pain_actions.len(), 1);
    match &feel_no_pain_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::FeelNoPain);
            assert_eq!(*amount, 4);
        }
        other => panic!("Feel No Pain+ should apply upgraded power, got {other:?}"),
    }

    let mut feed_plus = CombatCard::new(CardId::Feed, 153);
    feed_plus.upgrades = 1;
    let feed_actions = resolve_card_play(CardId::Feed, &state, &feed_plus, Some(7));
    assert_eq!(feed_actions.len(), 1);
    match &feed_actions[0].action {
        Action::Feed {
            target,
            damage_info,
            max_hp_amount,
        } => {
            assert_eq!(*target, 7);
            assert_eq!(damage_info.source, 0);
            assert_eq!(damage_info.target, 7);
            assert_eq!(damage_info.base, 12);
            assert_eq!(damage_info.output, 12);
            assert_eq!(damage_info.damage_type, DamageType::Normal);
            assert_eq!(*max_hp_amount, 4);
        }
        other => panic!("Feed+ should emit upgraded FeedAction, got {other:?}"),
    }

    state.zones.exhaust_pile = vec![CombatCard::new(CardId::Strike, 154)];
    let exhume_actions = resolve_card_play(
        CardId::Exhume,
        &state,
        &CombatCard::new(CardId::Exhume, 155),
        None,
    );
    assert_eq!(exhume_actions.len(), 1);
    match &exhume_actions[0].action {
        Action::ExhumeCard { card_uuid, upgrade } => {
            assert_eq!(*card_uuid, 154);
            assert!(!*upgrade);
        }
        other => panic!("Exhume should auto-return sole non-Exhume exhaust card, got {other:?}"),
    }

    state.zones.exhaust_pile = vec![
        CombatCard::new(CardId::Exhume, 156),
        CombatCard::new(CardId::Strike, 157),
    ];
    let mut exhume_plus = CombatCard::new(CardId::Exhume, 158);
    exhume_plus.upgrades = 1;
    let select_actions = resolve_card_play(CardId::Exhume, &state, &exhume_plus, None);
    assert_eq!(select_actions.len(), 1);
    match &select_actions[0].action {
        Action::SuspendForGridSelect {
            source_pile,
            min,
            max,
            can_cancel,
            filter,
            reason,
        } => {
            assert_eq!(*source_pile, crate::state::PileType::Exhaust);
            assert_eq!(*min, 1);
            assert_eq!(*max, 1);
            assert!(!*can_cancel);
            assert_eq!(*filter, crate::state::GridSelectFilter::NonExhume);
            assert_eq!(
                *reason,
                crate::state::GridSelectReason::Exhume { upgrade: false }
            );
        }
        other => panic!("Exhume should open non-cancellable exhaust grid select, got {other:?}"),
    }

    state.zones.hand = (0..10)
        .map(|offset| CombatCard::new(CardId::Defend, 170 + offset))
        .collect();
    state.zones.exhaust_pile = vec![CombatCard::new(CardId::Strike, 181)];
    let full_hand_actions = resolve_card_play(
        CardId::Exhume,
        &state,
        &CombatCard::new(CardId::Exhume, 182),
        None,
    );
    assert!(full_hand_actions.is_empty());
}

#[test]
fn evolve_exhume_feed_and_feel_no_pain_hooks_match_java_sources() {
    let mut state = crate::test_support::blank_test_combat();
    state.zones.hand = vec![CombatCard::new(CardId::Wound, 190)];
    let evolve_draw =
        crate::content::powers::resolve_power_on_card_drawn(PowerId::Evolve, &state, 0, 2, 190);
    assert_eq!(evolve_draw.as_slice(), &[Action::DrawCards(2)]);

    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::NoDraw,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let blocked_evolve_draw =
        crate::content::powers::resolve_power_on_card_drawn(PowerId::Evolve, &state, 0, 2, 190);
    assert!(blocked_evolve_draw.is_empty());

    let feel_no_pain_block = crate::content::powers::resolve_power_on_exhaust(
        PowerId::FeelNoPain,
        &state,
        0,
        4,
        191,
        CardId::Strike,
    );
    assert_eq!(
        feel_no_pain_block.as_slice(),
        &[Action::GainBlock {
            target: 0,
            amount: 4
        }]
    );

    let mut exhume_state = crate::test_support::blank_test_combat();
    exhume_state.zones.exhaust_pile = vec![CombatCard::new(CardId::Defend, 192)];
    crate::content::powers::store::set_powers_for(
        &mut exhume_state,
        0,
        vec![Power {
            power_type: PowerId::Corruption,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::cards::handle_exhume_card(192, false, &mut exhume_state);
    assert!(exhume_state.zones.exhaust_pile.is_empty());
    assert_eq!(exhume_state.zones.hand.len(), 1);
    assert_eq!(exhume_state.zones.hand[0].id, CardId::Defend);
    assert_eq!(exhume_state.zones.hand[0].cost_for_turn, Some(0));

    let mut normal_feed_state = crate::test_support::blank_test_combat();
    let mut jaw_worm = crate::test_support::test_monster(EnemyId::JawWorm);
    jaw_worm.id = 31;
    jaw_worm.current_hp = 5;
    normal_feed_state.entities.monsters = vec![jaw_worm];
    crate::engine::action_handlers::damage::handle_feed(
        31,
        crate::runtime::action::DamageInfo {
            source: 0,
            target: 31,
            base: 10,
            output: 10,
            damage_type: DamageType::Normal,
            is_modified: false,
        },
        3,
        &mut normal_feed_state,
    );
    assert_eq!(normal_feed_state.entities.player.max_hp, 83);
    assert_eq!(normal_feed_state.entities.player.current_hp, 83);

    let mut minion_feed_state = crate::test_support::blank_test_combat();
    let mut minion = crate::test_support::test_monster(EnemyId::JawWorm);
    minion.id = 32;
    minion.current_hp = 5;
    minion_feed_state.entities.monsters = vec![minion];
    crate::content::powers::store::set_powers_for(
        &mut minion_feed_state,
        32,
        vec![Power {
            power_type: PowerId::Minion,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::damage::handle_feed(
        32,
        crate::runtime::action::DamageInfo {
            source: 0,
            target: 32,
            base: 10,
            output: 10,
            damage_type: DamageType::Normal,
            is_modified: false,
        },
        3,
        &mut minion_feed_state,
    );
    assert_eq!(minion_feed_state.entities.player.max_hp, 80);
    assert_eq!(minion_feed_state.entities.player.current_hp, 80);
}

#[test]
fn on_kill_card_rewards_ignore_minions_and_half_dead_targets_like_java_actions() {
    fn test_damage(target: usize) -> DamageInfo {
        DamageInfo {
            source: 0,
            target,
            base: 10,
            output: 10,
            damage_type: DamageType::Normal,
            is_modified: false,
        }
    }

    let mut greed_normal = crate::test_support::blank_test_combat();
    let starting_gold = greed_normal.entities.player.gold;
    let mut normal = crate::test_support::test_monster(EnemyId::JawWorm);
    normal.id = 41;
    normal.current_hp = 5;
    greed_normal.entities.monsters = vec![normal];
    crate::engine::action_handlers::damage::handle_hand_of_greed(
        41,
        test_damage(41),
        20,
        &mut greed_normal,
    );
    assert_eq!(greed_normal.entities.player.gold, starting_gold + 20);
    assert_eq!(greed_normal.entities.player.gold_delta_this_combat, 20);
    assert_eq!(
        greed_normal.pop_next_action(),
        None,
        "Java GreedAction calls player.gainGold inside the damage action, before clearPostCombatActions"
    );

    let mut greed_minion = crate::test_support::blank_test_combat();
    let minion_starting_gold = greed_minion.entities.player.gold;
    let mut minion = crate::test_support::test_monster(EnemyId::JawWorm);
    minion.id = 42;
    minion.current_hp = 5;
    greed_minion.entities.monsters = vec![minion];
    crate::content::powers::store::set_powers_for(
        &mut greed_minion,
        42,
        vec![Power {
            power_type: PowerId::Minion,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::damage::handle_hand_of_greed(
        42,
        test_damage(42),
        20,
        &mut greed_minion,
    );
    assert_eq!(greed_minion.entities.player.gold, minion_starting_gold);
    assert_eq!(greed_minion.pop_next_action(), None);

    let mut dagger_normal = crate::test_support::blank_test_combat();
    let mut dagger_target = crate::test_support::test_monster(EnemyId::JawWorm);
    dagger_target.id = 44;
    dagger_target.current_hp = 5;
    dagger_normal.entities.monsters = vec![dagger_target];
    dagger_normal.zones.hand = vec![CombatCard::new(CardId::RitualDagger, 900)];
    dagger_normal.zones.draw_pile = vec![CombatCard::new(CardId::RitualDagger, 900)];
    dagger_normal.zones.discard_pile = vec![CombatCard::new(CardId::RitualDagger, 900)];
    dagger_normal.zones.exhaust_pile = vec![CombatCard::new(CardId::RitualDagger, 900)];
    dagger_normal.zones.limbo = vec![CombatCard::new(CardId::RitualDagger, 900)];
    crate::engine::action_handlers::damage::handle_ritual_dagger(
        44,
        test_damage(44),
        3,
        900,
        &mut dagger_normal,
    );
    assert_eq!(dagger_normal.zones.hand[0].misc_value, 3);
    assert_eq!(dagger_normal.zones.draw_pile[0].misc_value, 3);
    assert_eq!(dagger_normal.zones.discard_pile[0].misc_value, 3);
    assert_eq!(dagger_normal.zones.exhaust_pile[0].misc_value, 3);
    assert_eq!(dagger_normal.zones.limbo[0].misc_value, 3);
    assert_eq!(
        dagger_normal.meta.meta_changes,
        vec![crate::runtime::combat::MetaChange::ModifyCardMisc {
            card_uuid: 900,
            amount: 3,
        }],
        "Java RitualDaggerAction also mutates the matching masterDeck card"
    );
    assert_eq!(
        dagger_normal.pop_next_action(),
        None,
        "Java RitualDaggerAction mutates every matching GetAllInBattleInstances card inside the damage action"
    );

    let mut dagger_half_dead = crate::test_support::blank_test_combat();
    let mut half_dead = crate::test_support::test_monster(EnemyId::AwakenedOne);
    half_dead.id = 43;
    half_dead.current_hp = 5;
    half_dead.half_dead = true;
    dagger_half_dead.entities.monsters = vec![half_dead];
    crate::engine::action_handlers::damage::handle_ritual_dagger(
        43,
        test_damage(43),
        3,
        900,
        &mut dagger_half_dead,
    );
    assert_eq!(dagger_half_dead.pop_next_action(), None);
}

#[test]
fn ironclad_fire_and_strength_definitions_match_java_sources() {
    let fiend_fire = get_card_definition(CardId::FiendFire);
    assert_eq!(fiend_fire.card_type, CardType::Attack);
    assert_eq!(fiend_fire.rarity, CardRarity::Rare);
    assert_eq!(fiend_fire.cost, 2);
    assert_eq!(fiend_fire.base_damage, 7);
    assert_eq!(fiend_fire.target, CardTarget::Enemy);
    assert!(fiend_fire.exhaust);
    assert_eq!(fiend_fire.upgrade_damage, 3);

    let fire_breathing = get_card_definition(CardId::FireBreathing);
    assert_eq!(fire_breathing.card_type, CardType::Power);
    assert_eq!(fire_breathing.rarity, CardRarity::Uncommon);
    assert_eq!(fire_breathing.cost, 1);
    assert_eq!(fire_breathing.base_magic, 6);
    assert_eq!(fire_breathing.target, CardTarget::SelfTarget);
    assert_eq!(fire_breathing.upgrade_magic, 4);

    let flame_barrier = get_card_definition(CardId::FlameBarrier);
    assert_eq!(flame_barrier.card_type, CardType::Skill);
    assert_eq!(flame_barrier.rarity, CardRarity::Uncommon);
    assert_eq!(flame_barrier.cost, 2);
    assert_eq!(flame_barrier.base_block, 12);
    assert_eq!(flame_barrier.base_magic, 4);
    assert_eq!(flame_barrier.target, CardTarget::SelfTarget);
    assert_eq!(flame_barrier.upgrade_block, 4);
    assert_eq!(flame_barrier.upgrade_magic, 2);

    let flex = get_card_definition(CardId::Flex);
    assert_eq!(flex.card_type, CardType::Skill);
    assert_eq!(flex.rarity, CardRarity::Common);
    assert_eq!(flex.cost, 0);
    assert_eq!(flex.base_magic, 2);
    assert_eq!(flex.target, CardTarget::SelfTarget);
    assert_eq!(flex.upgrade_magic, 2);
}

#[test]
fn ironclad_fire_and_strength_runtime_actions_match_java_use_methods() {
    let state = crate::test_support::blank_test_combat();

    let mut fiend_fire_plus = CombatCard::new(CardId::FiendFire, 210);
    fiend_fire_plus.upgrades = 1;
    let fiend_fire_actions =
        resolve_card_play(CardId::FiendFire, &state, &fiend_fire_plus, Some(9));
    assert_eq!(fiend_fire_actions.len(), 1);
    match &fiend_fire_actions[0].action {
        Action::FiendFire {
            target,
            damage_info,
        } => {
            assert_eq!(*target, 9);
            assert_eq!(damage_info.source, 0);
            assert_eq!(damage_info.target, 9);
            assert_eq!(damage_info.base, 10);
            assert_eq!(damage_info.output, 10);
            assert_eq!(damage_info.damage_type, DamageType::Normal);
        }
        other => panic!("Fiend Fire+ should emit upgraded FiendFireAction, got {other:?}"),
    }

    let mut fire_breathing_plus = CombatCard::new(CardId::FireBreathing, 211);
    fire_breathing_plus.upgrades = 1;
    let fire_breathing_actions =
        resolve_card_play(CardId::FireBreathing, &state, &fire_breathing_plus, None);
    assert_eq!(fire_breathing_actions.len(), 1);
    match &fire_breathing_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::FireBreathing);
            assert_eq!(*amount, 10);
        }
        other => panic!("Fire Breathing+ should apply upgraded power, got {other:?}"),
    }

    let mut flame_barrier_plus = CombatCard::new(CardId::FlameBarrier, 212);
    flame_barrier_plus.upgrades = 1;
    let flame_barrier_actions =
        resolve_card_play(CardId::FlameBarrier, &state, &flame_barrier_plus, None);
    assert_eq!(flame_barrier_actions.len(), 2);
    assert!(matches!(
        flame_barrier_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 16
        }
    ));
    match &flame_barrier_actions[1].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::FlameBarrier);
            assert_eq!(*amount, 6);
        }
        other => panic!("Flame Barrier+ should apply upgraded power, got {other:?}"),
    }

    let mut flex_plus = CombatCard::new(CardId::Flex, 213);
    flex_plus.upgrades = 1;
    let flex_actions = resolve_card_play(CardId::Flex, &state, &flex_plus, None);
    assert_eq!(flex_actions.len(), 2);
    match &flex_actions[0].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::Strength);
            assert_eq!(*amount, 4);
        }
        other => panic!("Flex+ first action should apply Strength, got {other:?}"),
    }
    match &flex_actions[1].action {
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*target, 0);
            assert_eq!(*power_id, PowerId::LoseStrength);
            assert_eq!(*amount, 4);
        }
        other => panic!("Flex+ second action should apply LoseStrength, got {other:?}"),
    }
}

#[test]
fn fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources() {
    let mut state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 41;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 42;
    second.slot = 1;
    state.entities.monsters = vec![first, second];
    state.zones.hand = vec![CombatCard::new(CardId::Injury, 220)];

    let fire_breathing_damage = crate::content::powers::resolve_power_on_card_drawn(
        PowerId::FireBreathing,
        &state,
        0,
        10,
        220,
    );
    assert_eq!(fire_breathing_damage.len(), 1);
    match &fire_breathing_damage[0] {
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            is_modified,
        } => {
            assert_eq!(*source, NO_SOURCE);
            assert_eq!(damages.as_slice(), &[10, 10]);
            assert_eq!(*damage_type, DamageType::Thorns);
            assert!(!*is_modified);
        }
        other => panic!("Fire Breathing should damage all enemies with THORNS, got {other:?}"),
    }

    state.zones.hand = vec![CombatCard::new(CardId::Strike, 221)];
    let non_status_draw = crate::content::powers::resolve_power_on_card_drawn(
        PowerId::FireBreathing,
        &state,
        0,
        10,
        221,
    );
    assert!(non_status_draw.is_empty());

    let flame_barrier_damage = crate::content::powers::resolve_power_on_attacked(
        PowerId::FlameBarrier,
        &state,
        0,
        7,
        41,
        DamageType::Normal,
        6,
    );
    assert_eq!(flame_barrier_damage.len(), 1);
    match &flame_barrier_damage[0] {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 41);
            assert_eq!(info.base, 6);
            assert_eq!(info.output, 6);
            assert_eq!(info.damage_type, DamageType::Thorns);
        }
        other => panic!("Flame Barrier should retaliate with THORNS damage, got {other:?}"),
    }
    assert!(crate::content::powers::resolve_power_on_attacked(
        PowerId::FlameBarrier,
        &state,
        0,
        7,
        41,
        DamageType::Thorns,
        6,
    )
    .is_empty());
    assert!(crate::content::powers::resolve_power_on_attacked(
        PowerId::FlameBarrier,
        &state,
        0,
        7,
        NO_SOURCE,
        DamageType::Normal,
        6,
    )
    .is_empty());

    let lose_strength_turn_end = crate::content::powers::resolve_power_at_end_of_turn(
        &Power {
            power_type: PowerId::LoseStrength,
            instance_id: None,
            amount: 4,
            extra_data: 0,
            just_applied: false,
        },
        &state,
        0,
    );
    assert_eq!(
        lose_strength_turn_end.as_slice(),
        &[
            Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::Strength,
                amount: -4
            },
            Action::RemovePower {
                target: 0,
                power_id: PowerId::LoseStrength
            }
        ]
    );

    let mut fiend_fire_state = state.clone();
    fiend_fire_state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 230),
        CombatCard::new(CardId::Defend, 231),
        CombatCard::new(CardId::Bash, 232),
    ];
    fiend_fire_state.entities.monsters[0].current_hp = 40;
    crate::engine::action_handlers::damage::handle_fiend_fire(
        41,
        crate::runtime::action::DamageInfo {
            source: 0,
            target: 41,
            base: 7,
            output: 7,
            damage_type: DamageType::Normal,
            is_modified: false,
        },
        &mut fiend_fire_state,
    );
    assert_eq!(
        fiend_fire_state.zones.hand.len(),
        3,
        "FiendFireAction queues random ExhaustAction instances; it does not drain hand immediately"
    );
    for _ in 0..3 {
        assert_eq!(
            fiend_fire_state.pop_next_action(),
            Some(Action::ExhaustRandomCard { amount: 1 })
        );
    }
    for _ in 0..3 {
        assert!(matches!(
            fiend_fire_state.pop_next_action(),
            Some(Action::Damage(crate::runtime::action::DamageInfo {
                target: 41,
                output: 7,
                ..
            }))
        ));
    }
}

#[test]
fn ironclad_topdeck_and_strength_scaling_definitions_match_java_sources() {
    let ghostly_armor = get_card_definition(CardId::GhostlyArmor);
    assert_eq!(ghostly_armor.card_type, CardType::Skill);
    assert_eq!(ghostly_armor.rarity, CardRarity::Uncommon);
    assert_eq!(ghostly_armor.cost, 1);
    assert_eq!(ghostly_armor.base_block, 10);
    assert_eq!(ghostly_armor.target, CardTarget::SelfTarget);
    assert!(ghostly_armor.ethereal);
    assert_eq!(ghostly_armor.upgrade_block, 3);

    let havoc = get_card_definition(CardId::Havoc);
    assert_eq!(havoc.card_type, CardType::Skill);
    assert_eq!(havoc.rarity, CardRarity::Common);
    assert_eq!(havoc.cost, 1);
    assert_eq!(havoc.target, CardTarget::None);
    let mut havoc_plus = CombatCard::new(CardId::Havoc, 240);
    havoc_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&havoc_plus), Some(0));

    let headbutt = get_card_definition(CardId::Headbutt);
    assert_eq!(headbutt.card_type, CardType::Attack);
    assert_eq!(headbutt.rarity, CardRarity::Common);
    assert_eq!(headbutt.cost, 1);
    assert_eq!(headbutt.base_damage, 9);
    assert_eq!(headbutt.target, CardTarget::Enemy);
    assert_eq!(headbutt.upgrade_damage, 3);

    let heavy_blade = get_card_definition(CardId::HeavyBlade);
    assert_eq!(heavy_blade.card_type, CardType::Attack);
    assert_eq!(heavy_blade.rarity, CardRarity::Common);
    assert_eq!(heavy_blade.cost, 2);
    assert_eq!(heavy_blade.base_damage, 14);
    assert_eq!(heavy_blade.base_magic, 3);
    assert_eq!(heavy_blade.target, CardTarget::Enemy);
    assert_eq!(heavy_blade.upgrade_magic, 2);
}

#[test]
fn ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            just_applied: false,
        }],
    );

    let mut ghostly_armor_plus = CombatCard::new(CardId::GhostlyArmor, 241);
    ghostly_armor_plus.upgrades = 1;
    let ghostly_armor_actions =
        resolve_card_play(CardId::GhostlyArmor, &state, &ghostly_armor_plus, None);
    assert_eq!(ghostly_armor_actions.len(), 1);
    assert!(matches!(
        ghostly_armor_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 13
        }
    ));

    let havoc_actions = resolve_card_play(
        CardId::Havoc,
        &state,
        &CombatCard::new(CardId::Havoc, 242),
        None,
    );
    assert_eq!(havoc_actions.len(), 1);
    assert!(matches!(
        havoc_actions[0].action,
        Action::PlayTopCard {
            target: None,
            exhaust: true
        }
    ));

    let mut headbutt_plus = CombatCard::new(CardId::Headbutt, 243);
    headbutt_plus.upgrades = 1;
    let headbutt_actions = resolve_card_play(CardId::Headbutt, &state, &headbutt_plus, Some(51));
    assert_eq!(headbutt_actions.len(), 2);
    match &headbutt_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 51);
            assert_eq!(info.base, 14);
            assert_eq!(info.output, 14);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Headbutt+ first action should be damage, got {other:?}"),
    }
    assert!(matches!(
        headbutt_actions[1].action,
        Action::DiscardPileToTopOfDeck
    ));

    let mut heavy_blade_plus = CombatCard::new(CardId::HeavyBlade, 244);
    heavy_blade_plus.upgrades = 1;
    let heavy_blade_actions =
        resolve_card_play(CardId::HeavyBlade, &state, &heavy_blade_plus, Some(51));
    assert_eq!(heavy_blade_actions.len(), 1);
    match &heavy_blade_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 51);
            assert_eq!(info.base, 24);
            assert_eq!(info.output, 24);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Heavy Blade+ should multiply Strength by 5, got {other:?}"),
    }
}

#[test]
fn headbutt_and_havoc_execution_helpers_match_java_sources() {
    let mut headbutt_state = crate::test_support::blank_test_combat();
    let mut jaw_worm = crate::test_support::test_monster(EnemyId::JawWorm);
    jaw_worm.id = 61;
    jaw_worm.current_hp = 0;
    jaw_worm.is_dying = true;
    headbutt_state.entities.monsters = vec![jaw_worm];
    headbutt_state.zones.discard_pile = vec![CombatCard::new(CardId::Strike, 250)];
    crate::engine::action_handlers::cards::handle_discard_pile_to_top_of_deck(&mut headbutt_state);
    assert_eq!(headbutt_state.zones.discard_pile.len(), 1);
    assert!(headbutt_state.zones.draw_pile.is_empty());

    headbutt_state.entities.monsters[0].current_hp = 20;
    headbutt_state.entities.monsters[0].is_dying = false;
    crate::engine::action_handlers::cards::handle_discard_pile_to_top_of_deck(&mut headbutt_state);
    assert!(headbutt_state.zones.discard_pile.is_empty());
    assert_eq!(headbutt_state.zones.draw_pile.len(), 1);
    assert_eq!(headbutt_state.zones.draw_pile[0].uuid, 250);

    headbutt_state.zones.discard_pile = vec![
        CombatCard::new(CardId::Strike, 251),
        CombatCard::new(CardId::Defend, 252),
    ];
    crate::engine::action_handlers::cards::handle_discard_pile_to_top_of_deck(&mut headbutt_state);
    match headbutt_state.pop_next_action() {
        Some(Action::SuspendForGridSelect {
            source_pile,
            min,
            max,
            can_cancel,
            filter,
            reason,
        }) => {
            assert_eq!(source_pile, crate::state::PileType::Discard);
            assert_eq!(min, 1);
            assert_eq!(max, 1);
            assert!(!can_cancel);
            assert_eq!(filter, crate::state::GridSelectFilter::Any);
            assert_eq!(reason, crate::state::GridSelectReason::MoveToDrawPile);
        }
        other => {
            panic!("Headbutt should defer multi-card discard choice to grid select, got {other:?}")
        }
    }

    let mut havoc_state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 71;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 72;
    second.slot = 1;
    havoc_state.entities.monsters = vec![first, second];
    havoc_state.zones.discard_pile = vec![CombatCard::new(CardId::Strike, 260)];
    crate::engine::action_handlers::cards::handle_play_top_card(None, true, &mut havoc_state);
    assert!(matches!(
        havoc_state.pop_next_action(),
        Some(Action::EmptyDeckShuffle)
    ));
    match havoc_state.pop_next_action() {
        Some(Action::PlayTopCard { target, exhaust }) => {
            assert!(matches!(target, Some(71 | 72)));
            assert!(exhaust);
        }
        other => panic!("Havoc should lock random target before empty-deck shuffle, got {other:?}"),
    }

    let mut played_havoc_state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 73;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 74;
    second.slot = 1;
    played_havoc_state.entities.monsters = vec![first, second];
    played_havoc_state.zones.hand = vec![CombatCard::new(CardId::Havoc, 270)];
    played_havoc_state.zones.draw_pile = vec![CombatCard::new(CardId::Clash, 271)];
    assert_eq!(played_havoc_state.rng.card_random_rng.counter, 0);

    crate::engine::action_handlers::cards::handle_play_card_from_hand(
        0,
        None,
        &mut played_havoc_state,
    )
    .expect("Havoc should be playable");

    assert_eq!(
        played_havoc_state.rng.card_random_rng.counter, 1,
        "Havoc.use chooses its random monster target immediately"
    );
    match played_havoc_state.pop_next_action() {
        Some(Action::PlayTopCard { target, exhaust }) => {
            assert!(matches!(target, Some(73 | 74)));
            assert!(exhaust);
        }
        other => {
            panic!("played Havoc should enqueue PlayTopCard with locked target, got {other:?}")
        }
    }
}

#[test]
fn ironclad_hp_loss_and_generated_attack_definitions_match_java_sources() {
    let hemokinesis = get_card_definition(CardId::Hemokinesis);
    assert_eq!(hemokinesis.card_type, CardType::Attack);
    assert_eq!(hemokinesis.rarity, CardRarity::Uncommon);
    assert_eq!(hemokinesis.cost, 1);
    assert_eq!(hemokinesis.base_damage, 15);
    assert_eq!(hemokinesis.base_magic, 2);
    assert_eq!(hemokinesis.target, CardTarget::Enemy);
    assert_eq!(hemokinesis.upgrade_damage, 5);

    let immolate = get_card_definition(CardId::Immolate);
    assert_eq!(immolate.card_type, CardType::Attack);
    assert_eq!(immolate.rarity, CardRarity::Rare);
    assert_eq!(immolate.cost, 2);
    assert_eq!(immolate.base_damage, 21);
    assert_eq!(immolate.target, CardTarget::AllEnemy);
    assert!(immolate.is_multi_damage);
    assert_eq!(immolate.upgrade_damage, 7);

    let impervious = get_card_definition(CardId::Impervious);
    assert_eq!(impervious.card_type, CardType::Skill);
    assert_eq!(impervious.rarity, CardRarity::Rare);
    assert_eq!(impervious.cost, 2);
    assert_eq!(impervious.base_block, 30);
    assert_eq!(impervious.target, CardTarget::SelfTarget);
    assert!(impervious.exhaust);
    assert_eq!(impervious.upgrade_block, 10);

    let infernal_blade = get_card_definition(CardId::InfernalBlade);
    assert_eq!(infernal_blade.card_type, CardType::Skill);
    assert_eq!(infernal_blade.rarity, CardRarity::Uncommon);
    assert_eq!(infernal_blade.cost, 1);
    assert_eq!(infernal_blade.target, CardTarget::None);
    assert!(infernal_blade.exhaust);
    let mut infernal_blade_plus = CombatCard::new(CardId::InfernalBlade, 270);
    infernal_blade_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&infernal_blade_plus), Some(0));
}

#[test]
fn ironclad_hp_loss_and_generated_attack_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 81;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 82;
    second.slot = 1;
    state.entities.monsters = vec![first, second];
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            just_applied: false,
        }],
    );

    let mut hemokinesis_plus = CombatCard::new(CardId::Hemokinesis, 271);
    hemokinesis_plus.upgrades = 1;
    let hemokinesis_actions =
        resolve_card_play(CardId::Hemokinesis, &state, &hemokinesis_plus, Some(81));
    assert_eq!(hemokinesis_actions.len(), 2);
    assert!(matches!(
        hemokinesis_actions[0].action,
        Action::LoseHp {
            target: 0,
            amount: 2,
            triggers_rupture: true
        }
    ));
    match &hemokinesis_actions[1].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 81);
            assert_eq!(info.base, 22);
            assert_eq!(info.output, 22);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Hemokinesis+ should damage after HP loss, got {other:?}"),
    }

    let mut immolate_plus = CombatCard::new(CardId::Immolate, 272);
    immolate_plus.upgrades = 1;
    let immolate_actions = resolve_card_play(CardId::Immolate, &state, &immolate_plus, None);
    assert_eq!(immolate_actions.len(), 2);
    match &immolate_actions[0].action {
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            is_modified,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(damages.as_slice(), &[30, 30]);
            assert_eq!(*damage_type, DamageType::Normal);
            assert!(!*is_modified);
        }
        other => panic!("Immolate+ should damage all enemies, got {other:?}"),
    }
    assert!(matches!(
        immolate_actions[1].action,
        Action::MakeTempCardInDiscard {
            card_id: CardId::Burn,
            amount: 1,
            upgraded: false
        }
    ));

    let mut impervious_plus = CombatCard::new(CardId::Impervious, 273);
    impervious_plus.upgrades = 1;
    let impervious_actions = resolve_card_play(CardId::Impervious, &state, &impervious_plus, None);
    assert_eq!(impervious_actions.len(), 1);
    assert!(matches!(
        impervious_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 40
        }
    ));

    let infernal_blade_actions = resolve_card_play(
        CardId::InfernalBlade,
        &state,
        &CombatCard::new(CardId::InfernalBlade, 274),
        None,
    );
    assert_eq!(infernal_blade_actions.len(), 1);
    assert!(matches!(
        infernal_blade_actions[0].action,
        Action::MakeRandomCardInHand {
            card_type: Some(CardType::Attack),
            cost_for_turn: Some(0)
        }
    ));
}

#[test]
fn ironclad_power_and_hybrid_attack_definitions_match_java_sources() {
    let inflame = get_card_definition(CardId::Inflame);
    assert_eq!(inflame.card_type, CardType::Power);
    assert_eq!(inflame.rarity, CardRarity::Uncommon);
    assert_eq!(inflame.cost, 1);
    assert_eq!(inflame.base_magic, 2);
    assert_eq!(inflame.target, CardTarget::SelfTarget);
    assert_eq!(inflame.upgrade_magic, 1);

    let intimidate = get_card_definition(CardId::Intimidate);
    assert_eq!(intimidate.card_type, CardType::Skill);
    assert_eq!(intimidate.rarity, CardRarity::Uncommon);
    assert_eq!(intimidate.cost, 0);
    assert_eq!(intimidate.base_magic, 1);
    assert_eq!(intimidate.target, CardTarget::AllEnemy);
    assert!(intimidate.exhaust);
    assert_eq!(intimidate.upgrade_magic, 1);

    let iron_wave = get_card_definition(CardId::IronWave);
    assert_eq!(iron_wave.card_type, CardType::Attack);
    assert_eq!(iron_wave.rarity, CardRarity::Common);
    assert_eq!(iron_wave.cost, 1);
    assert_eq!(iron_wave.base_damage, 5);
    assert_eq!(iron_wave.base_block, 5);
    assert_eq!(iron_wave.target, CardTarget::Enemy);
    assert_eq!(iron_wave.upgrade_damage, 2);
    assert_eq!(iron_wave.upgrade_block, 2);

    let juggernaut = get_card_definition(CardId::Juggernaut);
    assert_eq!(juggernaut.card_type, CardType::Power);
    assert_eq!(juggernaut.rarity, CardRarity::Rare);
    assert_eq!(juggernaut.cost, 2);
    assert_eq!(juggernaut.base_magic, 5);
    assert_eq!(juggernaut.target, CardTarget::SelfTarget);
    assert_eq!(juggernaut.upgrade_magic, 2);
}

#[test]
fn ironclad_power_and_hybrid_attack_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 91;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 92;
    second.slot = 1;
    state.entities.monsters = vec![first, second];
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            just_applied: false,
        }],
    );

    let mut inflame_plus = CombatCard::new(CardId::Inflame, 280);
    inflame_plus.upgrades = 1;
    let inflame_actions = resolve_card_play(CardId::Inflame, &state, &inflame_plus, None);
    assert_eq!(inflame_actions.len(), 1);
    assert!(matches!(
        inflame_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 3
        }
    ));

    let mut intimidate_plus = CombatCard::new(CardId::Intimidate, 281);
    intimidate_plus.upgrades = 1;
    let intimidate_actions = resolve_card_play(CardId::Intimidate, &state, &intimidate_plus, None);
    assert_eq!(intimidate_actions.len(), 2);
    for (action, expected_target) in intimidate_actions.iter().zip([91, 92]) {
        assert!(matches!(
            action.action,
            Action::ApplyPower {
                source: 0,
                target,
                power_id: PowerId::Weak,
                amount: 2
            } if target == expected_target
        ));
    }

    let mut iron_wave_plus = CombatCard::new(CardId::IronWave, 282);
    iron_wave_plus.upgrades = 1;
    let iron_wave_actions = resolve_card_play(CardId::IronWave, &state, &iron_wave_plus, Some(91));
    assert_eq!(iron_wave_actions.len(), 2);
    assert!(matches!(
        iron_wave_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 7
        }
    ));
    match &iron_wave_actions[1].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 91);
            assert_eq!(info.base, 9);
            assert_eq!(info.output, 9);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Iron Wave+ should gain block then deal damage, got {other:?}"),
    }

    let mut juggernaut_plus = CombatCard::new(CardId::Juggernaut, 283);
    juggernaut_plus.upgrades = 1;
    let juggernaut_actions = resolve_card_play(CardId::Juggernaut, &state, &juggernaut_plus, None);
    assert_eq!(juggernaut_actions.len(), 1);
    assert!(matches!(
        juggernaut_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Juggernaut,
            amount: 7
        }
    ));
}

#[test]
fn intimidate_and_shockwave_enqueue_apply_power_for_every_monster_like_java() {
    let mut state = crate::test_support::blank_test_combat();
    let mut dying = crate::test_support::test_monster(EnemyId::JawWorm);
    dying.id = 910;
    dying.current_hp = 0;
    dying.is_dying = true;
    let mut escaped = crate::test_support::test_monster(EnemyId::Cultist);
    escaped.id = 911;
    escaped.is_escaped = true;
    state.entities.monsters = vec![dying, escaped];

    let mut intimidate_plus = CombatCard::new(CardId::Intimidate, 912);
    intimidate_plus.upgrades = 1;
    let intimidate_actions = resolve_card_play(CardId::Intimidate, &state, &intimidate_plus, None);
    assert_eq!(
        intimidate_actions.len(),
        2,
        "Java Intimidate loops over monsters.monsters and leaves dead/escaped filtering to ApplyPowerAction"
    );
    assert!(matches!(
        intimidate_actions[0].action,
        Action::ApplyPower {
            target: 910,
            power_id: PowerId::Weak,
            amount: 2,
            ..
        }
    ));
    assert!(matches!(
        intimidate_actions[1].action,
        Action::ApplyPower {
            target: 911,
            power_id: PowerId::Weak,
            amount: 2,
            ..
        }
    ));

    let mut shockwave_plus = CombatCard::new(CardId::Shockwave, 913);
    shockwave_plus.upgrades = 1;
    let shockwave_actions = resolve_card_play(CardId::Shockwave, &state, &shockwave_plus, None);
    assert_eq!(
        shockwave_actions.len(),
        4,
        "Java Shockwave loops over monsters.monsters and leaves dead/escaped filtering to ApplyPowerAction"
    );
    assert!(matches!(
        shockwave_actions[0].action,
        Action::ApplyPower {
            target: 910,
            power_id: PowerId::Weak,
            amount: 5,
            ..
        }
    ));
    assert!(matches!(
        shockwave_actions[1].action,
        Action::ApplyPower {
            target: 910,
            power_id: PowerId::Vulnerable,
            amount: 5,
            ..
        }
    ));
    assert!(matches!(
        shockwave_actions[2].action,
        Action::ApplyPower {
            target: 911,
            power_id: PowerId::Weak,
            amount: 5,
            ..
        }
    ));
    assert!(matches!(
        shockwave_actions[3].action,
        Action::ApplyPower {
            target: 911,
            power_id: PowerId::Vulnerable,
            amount: 5,
            ..
        }
    ));
}

#[test]
fn juggernaut_block_hook_matches_java_source() {
    let state = crate::test_support::blank_test_combat();
    let actions =
        crate::content::powers::resolve_power_on_block_gained(PowerId::Juggernaut, &state, 0, 7, 5);
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Action::DamageRandomEnemy {
            source,
            base_damage,
            damage_type,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(*base_damage, 7);
            assert_eq!(*damage_type, DamageType::Thorns);
        }
        other => {
            panic!("Juggernaut should queue random THORNS damage on block gain, got {other:?}")
        }
    }
}

#[test]
fn ironclad_limit_and_strike_scaling_definitions_match_java_sources() {
    let limit_break = get_card_definition(CardId::LimitBreak);
    assert_eq!(limit_break.card_type, CardType::Skill);
    assert_eq!(limit_break.rarity, CardRarity::Rare);
    assert_eq!(limit_break.cost, 1);
    assert_eq!(limit_break.target, CardTarget::SelfTarget);
    assert!(limit_break.exhaust);
    let mut limit_break_plus = CombatCard::new(CardId::LimitBreak, 290);
    limit_break_plus.upgrades = 1;
    assert!(!exhausts_when_played(&limit_break_plus));

    let metallicize = get_card_definition(CardId::Metallicize);
    assert_eq!(metallicize.card_type, CardType::Power);
    assert_eq!(metallicize.rarity, CardRarity::Uncommon);
    assert_eq!(metallicize.cost, 1);
    assert_eq!(metallicize.base_magic, 3);
    assert_eq!(metallicize.target, CardTarget::SelfTarget);
    assert_eq!(metallicize.upgrade_magic, 1);

    let offering = get_card_definition(CardId::Offering);
    assert_eq!(offering.card_type, CardType::Skill);
    assert_eq!(offering.rarity, CardRarity::Rare);
    assert_eq!(offering.cost, 0);
    assert_eq!(offering.base_magic, 3);
    assert_eq!(offering.target, CardTarget::SelfTarget);
    assert!(offering.exhaust);
    assert_eq!(offering.upgrade_magic, 2);

    let perfected_strike = get_card_definition(CardId::PerfectedStrike);
    assert_eq!(perfected_strike.card_type, CardType::Attack);
    assert_eq!(perfected_strike.rarity, CardRarity::Common);
    assert_eq!(perfected_strike.cost, 2);
    assert_eq!(perfected_strike.base_damage, 6);
    assert_eq!(perfected_strike.base_magic, 2);
    assert_eq!(perfected_strike.target, CardTarget::Enemy);
    assert!(perfected_strike.tags.contains(&CardTag::Strike));
    assert_eq!(perfected_strike.upgrade_magic, 1);
}

#[test]
fn ironclad_limit_and_strike_scaling_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    let mut perfected_strike_plus = CombatCard::new(CardId::PerfectedStrike, 291);
    perfected_strike_plus.upgrades = 1;
    state.zones.hand = vec![
        perfected_strike_plus.clone(),
        CombatCard::new(CardId::Strike, 292),
    ];
    state.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 293)];
    state.zones.discard_pile = vec![CombatCard::new(CardId::Strike, 294)];
    state.zones.limbo = vec![CombatCard::new(CardId::Strike, 295)];

    let perfected_actions = resolve_card_play(
        CardId::PerfectedStrike,
        &state,
        &perfected_strike_plus,
        Some(101),
    );
    assert_eq!(perfected_actions.len(), 1);
    match &perfected_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 101);
            assert_eq!(info.base, 18);
            assert_eq!(info.output, 18);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!(
            "Perfected Strike+ should count hand/draw/discard Strike cards only, got {other:?}"
        ),
    }

    let limit_break_actions = resolve_card_play(
        CardId::LimitBreak,
        &state,
        &CombatCard::new(CardId::LimitBreak, 296),
        None,
    );
    assert_eq!(limit_break_actions.len(), 1);
    assert!(matches!(limit_break_actions[0].action, Action::LimitBreak));

    let mut metallicize_plus = CombatCard::new(CardId::Metallicize, 297);
    metallicize_plus.upgrades = 1;
    let metallicize_actions =
        resolve_card_play(CardId::Metallicize, &state, &metallicize_plus, None);
    assert_eq!(metallicize_actions.len(), 1);
    assert!(matches!(
        metallicize_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Metallicize,
            amount: 4
        }
    ));

    let mut offering_plus = CombatCard::new(CardId::Offering, 298);
    offering_plus.upgrades = 1;
    let offering_actions = resolve_card_play(CardId::Offering, &state, &offering_plus, None);
    assert_eq!(offering_actions.len(), 3);
    assert!(matches!(
        offering_actions[0].action,
        Action::LoseHp {
            target: 0,
            amount: 6,
            triggers_rupture: true
        }
    ));
    assert!(matches!(
        offering_actions[1].action,
        Action::GainEnergy { amount: 2 }
    ));
    assert!(matches!(offering_actions[2].action, Action::DrawCards(5)));
}

#[test]
fn limit_break_and_metallicize_hooks_match_java_sources() {
    let mut state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::damage::handle_limit_break(&mut state);
    assert_eq!(
        state.pop_next_action(),
        Some(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 3,
        }),
        "Java LimitBreakAction queues ApplyPowerAction to top instead of applying Strength inline"
    );
    crate::engine::action_handlers::powers::handle_apply_power(
        0,
        0,
        PowerId::Strength,
        3,
        &mut state,
    );
    assert_eq!(
        crate::content::powers::store::power_amount(&state, 0, PowerId::Strength),
        6
    );

    let metallicize_block = crate::content::powers::resolve_power_at_end_of_turn(
        &Power {
            power_type: PowerId::Metallicize,
            instance_id: None,
            amount: 4,
            extra_data: 0,
            just_applied: false,
        },
        &state,
        0,
    );
    assert_eq!(
        metallicize_block.as_slice(),
        &[Action::GainBlock {
            target: 0,
            amount: 4
        }]
    );
}

#[test]
fn ironclad_multi_hit_and_rage_definitions_match_java_sources() {
    let pommel = get_card_definition(CardId::PommelStrike);
    assert_eq!(pommel.card_type, CardType::Attack);
    assert_eq!(pommel.rarity, CardRarity::Common);
    assert_eq!(pommel.cost, 1);
    assert_eq!(pommel.base_damage, 9);
    assert_eq!(pommel.base_magic, 1);
    assert_eq!(pommel.target, CardTarget::Enemy);
    assert!(pommel.tags.contains(&CardTag::Strike));
    assert_eq!(pommel.upgrade_damage, 1);
    assert_eq!(pommel.upgrade_magic, 1);

    let power_through = get_card_definition(CardId::PowerThrough);
    assert_eq!(power_through.card_type, CardType::Skill);
    assert_eq!(power_through.rarity, CardRarity::Uncommon);
    assert_eq!(power_through.cost, 1);
    assert_eq!(power_through.base_block, 15);
    assert_eq!(power_through.target, CardTarget::SelfTarget);
    assert_eq!(power_through.upgrade_block, 5);

    let pummel = get_card_definition(CardId::Pummel);
    assert_eq!(pummel.card_type, CardType::Attack);
    assert_eq!(pummel.rarity, CardRarity::Uncommon);
    assert_eq!(pummel.cost, 1);
    assert_eq!(pummel.base_damage, 2);
    assert_eq!(pummel.base_magic, 4);
    assert_eq!(pummel.target, CardTarget::Enemy);
    assert!(pummel.exhaust);
    assert_eq!(pummel.upgrade_magic, 1);

    let rage = get_card_definition(CardId::Rage);
    assert_eq!(rage.card_type, CardType::Skill);
    assert_eq!(rage.rarity, CardRarity::Uncommon);
    assert_eq!(rage.cost, 0);
    assert_eq!(rage.base_magic, 3);
    assert_eq!(rage.target, CardTarget::SelfTarget);
    assert_eq!(rage.upgrade_magic, 2);
}

#[test]
fn ironclad_multi_hit_and_rage_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );

    let mut pommel_plus = CombatCard::new(CardId::PommelStrike, 300);
    pommel_plus.upgrades = 1;
    let pommel_actions = resolve_card_play(CardId::PommelStrike, &state, &pommel_plus, Some(111));
    assert_eq!(pommel_actions.len(), 2);
    match &pommel_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 111);
            assert_eq!(info.base, 11);
            assert_eq!(info.output, 11);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Pommel Strike+ should damage first, got {other:?}"),
    }
    assert!(matches!(pommel_actions[1].action, Action::DrawCards(2)));

    let mut power_through_plus = CombatCard::new(CardId::PowerThrough, 301);
    power_through_plus.upgrades = 1;
    let power_through_actions =
        resolve_card_play(CardId::PowerThrough, &state, &power_through_plus, None);
    assert_eq!(power_through_actions.len(), 2);
    assert!(matches!(
        power_through_actions[0].action,
        Action::MakeTempCardInHand {
            card_id: CardId::Wound,
            amount: 2,
            upgraded: false
        }
    ));
    assert!(matches!(
        power_through_actions[1].action,
        Action::GainBlock {
            target: 0,
            amount: 20
        }
    ));

    let mut pummel_plus = CombatCard::new(CardId::Pummel, 302);
    pummel_plus.upgrades = 1;
    let pummel_actions = resolve_card_play(CardId::Pummel, &state, &pummel_plus, Some(111));
    assert_eq!(pummel_actions.len(), 5);
    for (index, action) in pummel_actions.into_iter().enumerate() {
        match action.action {
            Action::PummelDamage(info) if index < 4 => {
                assert_eq!(info.source, 0);
                assert_eq!(info.target, 111);
                assert_eq!(info.base, 3);
                assert_eq!(info.output, 3);
                assert_eq!(info.damage_type, DamageType::Normal);
            }
            Action::Damage(info) if index == 4 => {
                assert_eq!(info.source, 0);
                assert_eq!(info.target, 111);
                assert_eq!(info.base, 3);
                assert_eq!(info.output, 3);
                assert_eq!(info.damage_type, DamageType::Normal);
            }
            other => panic!(
                "Java Pummel+ should emit four PummelDamageAction hits and one final DamageAction, got {other:?} at index {index}"
            ),
        }
    }

    let mut rage_plus = CombatCard::new(CardId::Rage, 303);
    rage_plus.upgrades = 1;
    let rage_actions = resolve_card_play(CardId::Rage, &state, &rage_plus, None);
    assert_eq!(rage_actions.len(), 1);
    assert!(matches!(
        rage_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Rage,
            amount: 5
        }
    ));
}

#[test]
fn rage_power_hooks_match_java_source() {
    let state = crate::test_support::blank_test_combat();
    let strike = CombatCard::new(CardId::Strike, 310);
    let defend = CombatCard::new(CardId::Defend, 311);
    let rage_block =
        crate::content::powers::resolve_power_on_card_played(PowerId::Rage, &state, 0, &strike, 5);
    assert_eq!(
        rage_block.as_slice(),
        &[Action::GainBlock {
            target: 0,
            amount: 5
        }]
    );
    assert!(crate::content::powers::resolve_power_on_card_played(
        PowerId::Rage,
        &state,
        0,
        &defend,
        5,
    )
    .is_empty());
    assert!(
        crate::content::powers::resolve_power_at_turn_start(PowerId::Rage, &state, 0, 5).is_empty()
    );
    assert_eq!(
        crate::content::powers::resolve_power_at_end_of_turn(
            &Power {
                power_type: PowerId::Rage,
                instance_id: None,
                amount: 5,
                extra_data: 0,
                just_applied: false,
            },
            &state,
            0,
        )
        .as_slice(),
        &[Action::RemovePower {
            target: 0,
            power_id: PowerId::Rage
        }]
    );
}

#[test]
fn ironclad_rampage_and_rupture_definitions_match_java_sources() {
    let rampage = get_card_definition(CardId::Rampage);
    assert_eq!(rampage.card_type, CardType::Attack);
    assert_eq!(rampage.rarity, CardRarity::Uncommon);
    assert_eq!(rampage.cost, 1);
    assert_eq!(rampage.base_damage, 8);
    assert_eq!(rampage.base_magic, 5);
    assert_eq!(rampage.target, CardTarget::Enemy);
    assert_eq!(rampage.upgrade_magic, 3);

    let reaper = get_card_definition(CardId::Reaper);
    assert_eq!(reaper.card_type, CardType::Attack);
    assert_eq!(reaper.rarity, CardRarity::Rare);
    assert_eq!(reaper.cost, 2);
    assert_eq!(reaper.base_damage, 4);
    assert_eq!(reaper.target, CardTarget::AllEnemy);
    assert!(reaper.is_multi_damage);
    assert!(reaper.exhaust);
    assert!(reaper.tags.contains(&CardTag::Healing));
    assert_eq!(reaper.upgrade_damage, 1);

    let reckless_charge = get_card_definition(CardId::RecklessCharge);
    assert_eq!(reckless_charge.card_type, CardType::Attack);
    assert_eq!(reckless_charge.rarity, CardRarity::Uncommon);
    assert_eq!(reckless_charge.cost, 0);
    assert_eq!(reckless_charge.base_damage, 7);
    assert_eq!(reckless_charge.target, CardTarget::Enemy);
    assert_eq!(reckless_charge.upgrade_damage, 3);

    let rupture = get_card_definition(CardId::Rupture);
    assert_eq!(rupture.card_type, CardType::Power);
    assert_eq!(rupture.rarity, CardRarity::Uncommon);
    assert_eq!(rupture.cost, 1);
    assert_eq!(rupture.base_magic, 1);
    assert_eq!(rupture.target, CardTarget::SelfTarget);
    assert_eq!(rupture.upgrade_magic, 1);
}

#[test]
fn ironclad_rampage_and_rupture_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 31;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 42;
    state.entities.monsters = vec![first, second];

    let mut rampage_plus = CombatCard::new(CardId::Rampage, 320);
    rampage_plus.upgrades = 1;
    let rampage_actions = resolve_card_play(CardId::Rampage, &state, &rampage_plus, Some(31));
    assert_eq!(rampage_actions.len(), 2);
    match &rampage_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 31);
            assert_eq!(info.base, 10);
            assert_eq!(info.output, 10);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Rampage+ should damage before modifying itself, got {other:?}"),
    }
    assert!(matches!(
        rampage_actions[1].action,
        Action::ModifyCardDamage {
            card_uuid: 320,
            amount: 8
        }
    ));

    let mut reaper_plus = CombatCard::new(CardId::Reaper, 321);
    reaper_plus.upgrades = 1;
    let reaper_actions = resolve_card_play(CardId::Reaper, &state, &reaper_plus, None);
    assert_eq!(reaper_actions.len(), 1);
    match &reaper_actions[0].action {
        Action::VampireDamageAllEnemies {
            source,
            damages,
            damage_type,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(damages.as_slice(), &[7, 7]);
            assert_eq!(*damage_type, DamageType::Normal);
        }
        other => panic!("Reaper+ should emit VampireDamageAllEnemiesAction, got {other:?}"),
    }

    let mut reckless_plus = CombatCard::new(CardId::RecklessCharge, 322);
    reckless_plus.upgrades = 1;
    let reckless_actions =
        resolve_card_play(CardId::RecklessCharge, &state, &reckless_plus, Some(42));
    assert_eq!(reckless_actions.len(), 2);
    match &reckless_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 42);
            assert_eq!(info.base, 12);
            assert_eq!(info.output, 12);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Reckless Charge+ should damage first, got {other:?}"),
    }
    assert!(matches!(
        reckless_actions[1].action,
        Action::MakeTempCardInDrawPile {
            card_id: CardId::Dazed,
            amount: 1,
            random_spot: true,
            to_bottom: false,
            upgraded: false
        }
    ));

    let mut rupture_plus = CombatCard::new(CardId::Rupture, 323);
    rupture_plus.upgrades = 1;
    let rupture_actions = resolve_card_play(CardId::Rupture, &state, &rupture_plus, None);
    assert_eq!(rupture_actions.len(), 1);
    assert!(matches!(
        rupture_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Rupture,
            amount: 2
        }
    ));
}

#[test]
fn rupture_and_reaper_execution_hooks_match_java_sources() {
    let state = crate::test_support::blank_test_combat();
    let rupture_actions = crate::content::powers::resolve_power_on_hp_lost(
        PowerId::Rupture,
        &state,
        0,
        3,
        2,
        None,
        DamageType::HpLoss,
        true,
    );
    assert_eq!(
        rupture_actions.as_slice(),
        &[Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 2
        }]
    );
    assert!(crate::content::powers::resolve_power_on_hp_lost(
        PowerId::Rupture,
        &state,
        0,
        3,
        2,
        None,
        DamageType::HpLoss,
        false,
    )
    .is_empty());

    let mut rupture_state = crate::test_support::blank_test_combat();
    rupture_state.entities.player.current_hp = 70;
    rupture_state
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::JawWorm));
    rupture_state.entities.power_db.insert(
        0,
        vec![Power {
            power_type: PowerId::Rupture,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::damage::handle_lose_hp(0, 5, true, &mut rupture_state);
    assert_eq!(
        rupture_state.pop_next_action(),
        Some(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 2
        }),
        "Java RupturePower grants its power amount, not the HP lost amount"
    );

    let mut reaper_state = crate::test_support::blank_test_combat();
    reaper_state.entities.player.current_hp = 50;
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 31;
    first.current_hp = 20;
    first.max_hp = 20;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 42;
    second.current_hp = 20;
    second.max_hp = 20;
    reaper_state.entities.monsters = vec![first, second];

    crate::engine::action_handlers::damage::handle_vampire_damage_all_enemies(
        0,
        smallvec::smallvec![3, 7],
        DamageType::Normal,
        &mut reaper_state,
    );

    assert_eq!(reaper_state.entities.monsters[0].current_hp, 17);
    assert_eq!(reaper_state.entities.monsters[1].current_hp, 13);
    assert_eq!(
        reaper_state.entities.player.current_hp, 50,
        "Java VampireDamageAllEnemiesAction queues HealAction instead of healing inline"
    );
    assert_eq!(
        reaper_state.pop_next_action(),
        Some(Action::Heal {
            target: 0,
            amount: 10
        })
    );
    crate::engine::action_handlers::damage::handle_heal(0, 10, &mut reaper_state);
    assert_eq!(reaper_state.entities.player.current_hp, 60);

    let mut flower_state = reaper_state.clone();
    flower_state.entities.player.current_hp = 50;
    flower_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::MagicFlower,
        ));
    crate::engine::action_handlers::damage::handle_vampire_damage_all_enemies(
        0,
        smallvec::smallvec![2, 2],
        DamageType::Normal,
        &mut flower_state,
    );
    assert_eq!(
        flower_state.pop_next_action(),
        Some(Action::Heal {
            target: 0,
            amount: 4
        })
    );
    crate::engine::action_handlers::damage::handle_heal(0, 4, &mut flower_state);
    assert_eq!(
        flower_state.entities.player.current_hp, 56,
        "Java Reaper queues HealAction, so Magic Flower modifies the vampire heal"
    );

    let mut bloom_state = reaper_state.clone();
    bloom_state.entities.player.current_hp = 50;
    bloom_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::MarkOfTheBloom,
        ));
    crate::engine::action_handlers::damage::handle_vampire_damage_all_enemies(
        0,
        smallvec::smallvec![2, 2],
        DamageType::Normal,
        &mut bloom_state,
    );
    assert_eq!(
        bloom_state.pop_next_action(),
        Some(Action::Heal {
            target: 0,
            amount: 4
        })
    );
    crate::engine::action_handlers::damage::handle_heal(0, 4, &mut bloom_state);
    assert_eq!(
        bloom_state.entities.player.current_hp, 50,
        "Java Mark of the Bloom returns zero from onPlayerHeal, including Reaper heals"
    );

    let mut single_vampire_state = crate::test_support::blank_test_combat();
    single_vampire_state.entities.player.current_hp = 70;
    let mut parasite = crate::test_support::test_monster(EnemyId::ShelledParasite);
    parasite.id = 77;
    parasite.current_hp = 10;
    parasite.max_hp = 30;
    single_vampire_state.entities.monsters = vec![parasite];
    crate::engine::action_handlers::damage::handle_vampire_damage(
        DamageInfo {
            source: 77,
            target: 0,
            base: 6,
            output: 6,
            damage_type: DamageType::Normal,
            is_modified: true,
        },
        &mut single_vampire_state,
    );
    assert_eq!(single_vampire_state.entities.player.current_hp, 64);
    assert_eq!(single_vampire_state.entities.monsters[0].current_hp, 10);
    assert_eq!(
        single_vampire_state.pop_next_action(),
        Some(Action::Heal {
            target: 77,
            amount: 6
        }),
        "Java VampireDamageAction queues HealAction to top instead of healing inline"
    );
    crate::engine::action_handlers::damage::handle_heal(77, 6, &mut single_vampire_state);
    assert_eq!(single_vampire_state.entities.monsters[0].current_hp, 16);
}

#[test]
fn ironclad_upgrade_and_exhaust_utility_definitions_match_java_sources() {
    let searing_blow = get_card_definition(CardId::SearingBlow);
    assert_eq!(searing_blow.card_type, CardType::Attack);
    assert_eq!(searing_blow.rarity, CardRarity::Uncommon);
    assert_eq!(searing_blow.cost, 2);
    assert_eq!(searing_blow.base_damage, 12);
    assert_eq!(searing_blow.target, CardTarget::Enemy);

    let second_wind = get_card_definition(CardId::SecondWind);
    assert_eq!(second_wind.card_type, CardType::Skill);
    assert_eq!(second_wind.rarity, CardRarity::Uncommon);
    assert_eq!(second_wind.cost, 1);
    assert_eq!(second_wind.base_block, 5);
    assert_eq!(second_wind.target, CardTarget::SelfTarget);
    assert_eq!(second_wind.upgrade_block, 2);

    let seeing_red = get_card_definition(CardId::SeeingRed);
    assert_eq!(seeing_red.card_type, CardType::Skill);
    assert_eq!(seeing_red.rarity, CardRarity::Uncommon);
    assert_eq!(seeing_red.cost, 1);
    assert_eq!(seeing_red.target, CardTarget::None);
    assert!(seeing_red.exhaust);
    let mut seeing_red_plus = CombatCard::new(CardId::SeeingRed, 330);
    seeing_red_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&seeing_red_plus), Some(0));

    let sentinel = get_card_definition(CardId::Sentinel);
    assert_eq!(sentinel.card_type, CardType::Skill);
    assert_eq!(sentinel.rarity, CardRarity::Uncommon);
    assert_eq!(sentinel.cost, 1);
    assert_eq!(sentinel.base_block, 5);
    assert_eq!(sentinel.base_magic, 0);
    assert_eq!(sentinel.target, CardTarget::SelfTarget);
    assert_eq!(sentinel.upgrade_block, 3);
}

#[test]
fn ironclad_upgrade_and_exhaust_utility_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![
            Power {
                power_type: PowerId::Strength,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            },
            Power {
                power_type: PowerId::Dexterity,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            },
        ],
    );

    let mut searing_blow_plus_2 = CombatCard::new(CardId::SearingBlow, 331);
    searing_blow_plus_2.upgrades = 2;
    let searing_actions =
        resolve_card_play(CardId::SearingBlow, &state, &searing_blow_plus_2, Some(77));
    assert_eq!(searing_actions.len(), 1);
    match &searing_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 77);
            assert_eq!(info.base, 23);
            assert_eq!(info.output, 23);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Searing Blow+2 should emit evaluated DamageAction, got {other:?}"),
    }

    let mut second_wind_plus = CombatCard::new(CardId::SecondWind, 332);
    second_wind_plus.upgrades = 1;
    let second_wind_actions =
        resolve_card_play(CardId::SecondWind, &state, &second_wind_plus, None);
    assert_eq!(second_wind_actions.len(), 1);
    assert!(matches!(
        second_wind_actions[0].action,
        Action::BlockPerNonAttack { block_per_card: 9 }
    ));

    let seeing_red_actions = resolve_card_play(
        CardId::SeeingRed,
        &state,
        &CombatCard::new(CardId::SeeingRed, 333),
        None,
    );
    assert_eq!(seeing_red_actions.len(), 1);
    assert!(matches!(
        seeing_red_actions[0].action,
        Action::GainEnergy { amount: 2 }
    ));

    let mut sentinel_plus = CombatCard::new(CardId::Sentinel, 334);
    sentinel_plus.upgrades = 1;
    let sentinel_actions = resolve_card_play(CardId::Sentinel, &state, &sentinel_plus, None);
    assert_eq!(sentinel_actions.len(), 1);
    assert!(matches!(
        sentinel_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 10
        }
    ));
}

#[test]
fn sentinel_exhaust_trigger_matches_java_add_to_top_energy() {
    let state = crate::test_support::blank_test_combat();
    let sentinel = CombatCard::new(CardId::Sentinel, 340);
    let sentinel_hooks = resolve_card_on_exhaust(&sentinel, &state);
    assert_eq!(sentinel_hooks.len(), 1);
    assert!(matches!(
        sentinel_hooks[0].action,
        Action::GainEnergy { amount: 2 }
    ));
    assert_eq!(
        sentinel_hooks[0].insertion_mode,
        crate::runtime::action::AddTo::Top
    );

    let mut sentinel_plus = CombatCard::new(CardId::Sentinel, 341);
    sentinel_plus.upgrades = 1;
    let sentinel_plus_hooks = resolve_card_on_exhaust(&sentinel_plus, &state);
    assert_eq!(sentinel_plus_hooks.len(), 1);
    assert!(matches!(
        sentinel_plus_hooks[0].action,
        Action::GainEnergy { amount: 3 }
    ));
    assert_eq!(
        sentinel_plus_hooks[0].insertion_mode,
        crate::runtime::action::AddTo::Top
    );

    let mut exhaust_state = crate::test_support::blank_test_combat();
    exhaust_state.zones.hand = vec![sentinel_plus];
    crate::content::powers::store::set_powers_for(
        &mut exhaust_state,
        0,
        vec![Power {
            power_type: PowerId::FeelNoPain,
            instance_id: None,
            amount: 4,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::cards::handle_exhaust_card(
        341,
        crate::state::PileType::Hand,
        &mut exhaust_state,
    );
    assert_eq!(
        exhaust_state.pop_next_action(),
        Some(Action::GainEnergy { amount: 3 })
    );
    assert_eq!(
        exhaust_state.pop_next_action(),
        Some(Action::GainBlock {
            target: 0,
            amount: 4
        })
    );

    let mut charon_state = crate::test_support::blank_test_combat();
    charon_state.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
    charon_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::CharonsAshes,
        ));
    charon_state.zones.hand = vec![CombatCard::new(CardId::Sentinel, 342)];
    crate::engine::action_handlers::cards::handle_exhaust_card(
        342,
        crate::state::PileType::Hand,
        &mut charon_state,
    );
    assert_eq!(
        charon_state.pop_next_action(),
        Some(Action::GainEnergy { amount: 2 }),
        "Sentinel triggerOnExhaust is called after relic onExhaust and uses Java addToTop, so it resolves first"
    );
    assert!(
        matches!(
            charon_state.pop_next_action(),
            Some(Action::DamageAllEnemies { .. })
        ),
        "Charon's Ashes addToTop action remains next after the later Sentinel addToTop"
    );
}

#[test]
fn burning_pact_exhausted_sentinel_energy_precedes_followup_draw() {
    let mut state = crate::test_support::blank_test_combat();
    state.zones.hand = vec![CombatCard::new(CardId::Sentinel, 350)];

    let burning_pact = CombatCard::new(CardId::BurningPact, 351);
    let actions = resolve_card_play(CardId::BurningPact, &state, &burning_pact, None);
    state.queue_actions(actions);

    let exhaust = state
        .pop_next_action()
        .expect("Burning Pact should queue its exhaust action before drawing");
    crate::engine::action_handlers::execute_action(exhaust, &mut state);

    assert_eq!(
        state.pop_next_action(),
        Some(Action::GainEnergy { amount: 2 }),
        "Sentinel addToTop should interrupt Burning Pact's already-queued DrawCardAction"
    );
    assert_eq!(state.pop_next_action(), Some(Action::DrawCards(2)));
}

#[test]
fn ironclad_exhaust_debuff_and_intent_definitions_match_java_sources() {
    let sever_soul = get_card_definition(CardId::SeverSoul);
    assert_eq!(sever_soul.card_type, CardType::Attack);
    assert_eq!(sever_soul.rarity, CardRarity::Uncommon);
    assert_eq!(sever_soul.cost, 2);
    assert_eq!(sever_soul.base_damage, 16);
    assert_eq!(sever_soul.target, CardTarget::Enemy);
    assert_eq!(sever_soul.upgrade_damage, 6);

    let shockwave = get_card_definition(CardId::Shockwave);
    assert_eq!(shockwave.card_type, CardType::Skill);
    assert_eq!(shockwave.rarity, CardRarity::Uncommon);
    assert_eq!(shockwave.cost, 2);
    assert_eq!(shockwave.base_magic, 3);
    assert_eq!(shockwave.target, CardTarget::AllEnemy);
    assert!(shockwave.exhaust);
    assert_eq!(shockwave.upgrade_magic, 2);

    let shrug = get_card_definition(CardId::ShrugItOff);
    assert_eq!(shrug.card_type, CardType::Skill);
    assert_eq!(shrug.rarity, CardRarity::Common);
    assert_eq!(shrug.cost, 1);
    assert_eq!(shrug.base_block, 8);
    assert_eq!(shrug.base_magic, 0);
    assert_eq!(shrug.target, CardTarget::SelfTarget);
    assert_eq!(shrug.upgrade_block, 3);

    let spot_weakness = get_card_definition(CardId::SpotWeakness);
    assert_eq!(spot_weakness.card_type, CardType::Skill);
    assert_eq!(spot_weakness.rarity, CardRarity::Uncommon);
    assert_eq!(spot_weakness.cost, 1);
    assert_eq!(spot_weakness.base_magic, 3);
    assert_eq!(spot_weakness.target, CardTarget::SelfAndEnemy);
    assert_eq!(spot_weakness.upgrade_magic, 1);
}

#[test]
fn ironclad_exhaust_debuff_and_intent_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![
            Power {
                power_type: PowerId::Strength,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            },
            Power {
                power_type: PowerId::Dexterity,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            },
        ],
    );
    let mut attack_monster = crate::test_support::test_monster(EnemyId::JawWorm);
    attack_monster.id = 501;
    attack_monster.set_planned_move_id(1);
    let attack_spec =
        crate::semantics::combat::MonsterMoveSpec::Attack(crate::semantics::combat::AttackSpec {
            base_damage: 11,
            hits: 1,
            damage_kind: crate::semantics::combat::DamageKind::Normal,
        });
    attack_monster.move_state.planned_steps = Some(attack_spec.to_steps());
    attack_monster.move_state.planned_visible_spec = Some(attack_spec);

    let mut defend_monster = crate::test_support::test_monster(EnemyId::Cultist);
    defend_monster.id = 502;
    defend_monster.set_planned_move_id(3);
    let defend_spec =
        crate::semantics::combat::MonsterMoveSpec::Defend(crate::semantics::combat::DefendSpec {
            block: 6,
        });
    defend_monster.move_state.planned_steps = Some(defend_spec.to_steps());
    defend_monster.move_state.planned_visible_spec = Some(defend_spec);
    state.entities.monsters = vec![attack_monster, defend_monster];

    let mut sever_plus = CombatCard::new(CardId::SeverSoul, 350);
    sever_plus.upgrades = 1;
    let sever_actions = resolve_card_play(CardId::SeverSoul, &state, &sever_plus, Some(501));
    assert_eq!(sever_actions.len(), 2);
    assert!(matches!(
        sever_actions[0].action,
        Action::ExhaustAllNonAttack
    ));
    match &sever_actions[1].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 501);
            assert_eq!(info.base, 24);
            assert_eq!(info.output, 24);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Sever Soul+ should damage after ExhaustAllNonAttack, got {other:?}"),
    }

    let mut shockwave_plus = CombatCard::new(CardId::Shockwave, 351);
    shockwave_plus.upgrades = 1;
    let shockwave_actions = resolve_card_play(CardId::Shockwave, &state, &shockwave_plus, None);
    assert_eq!(shockwave_actions.len(), 4);
    assert!(matches!(
        shockwave_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 501,
            power_id: PowerId::Weak,
            amount: 5
        }
    ));
    assert!(matches!(
        shockwave_actions[1].action,
        Action::ApplyPower {
            source: 0,
            target: 501,
            power_id: PowerId::Vulnerable,
            amount: 5
        }
    ));
    assert!(matches!(
        shockwave_actions[2].action,
        Action::ApplyPower {
            source: 0,
            target: 502,
            power_id: PowerId::Weak,
            amount: 5
        }
    ));
    assert!(matches!(
        shockwave_actions[3].action,
        Action::ApplyPower {
            source: 0,
            target: 502,
            power_id: PowerId::Vulnerable,
            amount: 5
        }
    ));

    let mut shrug_plus = CombatCard::new(CardId::ShrugItOff, 352);
    shrug_plus.upgrades = 1;
    let shrug_actions = resolve_card_play(CardId::ShrugItOff, &state, &shrug_plus, None);
    assert_eq!(shrug_actions.len(), 2);
    assert!(matches!(
        shrug_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 13
        }
    ));
    assert!(matches!(shrug_actions[1].action, Action::DrawCards(1)));

    let mut spot_plus = CombatCard::new(CardId::SpotWeakness, 353);
    spot_plus.upgrades = 1;
    let spot_actions = resolve_card_play(CardId::SpotWeakness, &state, &spot_plus, Some(501));
    assert_eq!(spot_actions.len(), 1);
    assert!(matches!(
        spot_actions[0].action,
        Action::SpotWeakness {
            target: 501,
            amount: 4
        }
    ));

    let mut spot_state = state.clone();
    crate::engine::action_handlers::execute_action(spot_actions[0].action.clone(), &mut spot_state);
    let queued_apply = spot_state.pop_next_action();
    assert_eq!(
        queued_apply,
        Some(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 4,
        })
    );
    crate::engine::action_handlers::execute_action(queued_apply.unwrap(), &mut spot_state);
    assert_eq!(
        crate::content::powers::store::powers_for(&spot_state, 0)
            .unwrap()
            .iter()
            .find(|power| power.power_type == PowerId::Strength)
            .map(|power| power.amount),
        Some(6)
    );

    let mut changed_state = state.clone();
    changed_state.entities.monsters[0].set_planned_move_id(2);
    crate::engine::action_handlers::execute_action(
        spot_actions[0].action.clone(),
        &mut changed_state,
    );
    assert_eq!(changed_state.pop_next_action(), None);
    assert_eq!(
        crate::content::powers::store::powers_for(&changed_state, 0)
            .unwrap()
            .iter()
            .find(|power| power.power_type == PowerId::Strength)
            .map(|power| power.amount),
        Some(2)
    );
}

#[test]
fn second_wind_block_per_non_attack_matches_java_add_to_top_order() {
    let mut state = crate::test_support::blank_test_combat();
    state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 350),
        CombatCard::new(CardId::Defend, 351),
        CombatCard::new(CardId::ShrugItOff, 352),
    ];

    crate::engine::action_handlers::damage::handle_block_per_non_attack(7, &mut state);

    assert_eq!(
        state.pop_next_action(),
        Some(Action::ExhaustCard {
            card_uuid: 352,
            source_pile: crate::state::PileType::Hand
        })
    );
    assert_eq!(
        state.pop_next_action(),
        Some(Action::ExhaustCard {
            card_uuid: 351,
            source_pile: crate::state::PileType::Hand
        })
    );
    assert!(matches!(
        state.pop_next_action(),
        Some(Action::GainBlock {
            target: 0,
            amount: 7
        })
    ));
    assert!(matches!(
        state.pop_next_action(),
        Some(Action::GainBlock {
            target: 0,
            amount: 7
        })
    ));
}

#[test]
fn sever_soul_exhaust_all_non_attack_queues_exhausts_before_following_damage() {
    let mut state = crate::test_support::blank_test_combat();
    state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 360),
        CombatCard::new(CardId::Defend, 361),
        CombatCard::new(CardId::ShrugItOff, 362),
    ];
    state.queue_action_back(Action::Damage(crate::runtime::action::DamageInfo {
        source: 0,
        target: 501,
        base: 16,
        output: 16,
        damage_type: DamageType::Normal,
        is_modified: false,
    }));

    crate::engine::action_handlers::damage::handle_exhaust_all_non_attack(&mut state);

    assert_eq!(
        state.pop_next_action(),
        Some(Action::ExhaustCard {
            card_uuid: 362,
            source_pile: crate::state::PileType::Hand
        })
    );
    assert_eq!(
        state.pop_next_action(),
        Some(Action::ExhaustCard {
            card_uuid: 361,
            source_pile: crate::state::PileType::Hand
        })
    );
    assert!(matches!(
        state.pop_next_action(),
        Some(Action::Damage(crate::runtime::action::DamageInfo {
            target: 501,
            ..
        }))
    ));
}

#[test]
fn ironclad_random_and_exhaust_attack_definitions_match_java_sources() {
    let sword_boomerang = get_card_definition(CardId::SwordBoomerang);
    assert_eq!(sword_boomerang.card_type, CardType::Attack);
    assert_eq!(sword_boomerang.rarity, CardRarity::Common);
    assert_eq!(sword_boomerang.cost, 1);
    assert_eq!(sword_boomerang.base_damage, 3);
    assert_eq!(sword_boomerang.base_magic, 3);
    assert_eq!(sword_boomerang.target, CardTarget::AllEnemy);
    assert_eq!(sword_boomerang.upgrade_magic, 1);

    let thunderclap = get_card_definition(CardId::ThunderClap);
    assert_eq!(thunderclap.name, "Thunderclap");
    assert_eq!(thunderclap.card_type, CardType::Attack);
    assert_eq!(thunderclap.rarity, CardRarity::Common);
    assert_eq!(thunderclap.cost, 1);
    assert_eq!(thunderclap.base_damage, 4);
    assert_eq!(thunderclap.base_magic, 0);
    assert_eq!(thunderclap.target, CardTarget::AllEnemy);
    assert!(thunderclap.is_multi_damage);
    assert_eq!(thunderclap.upgrade_damage, 3);

    let true_grit = get_card_definition(CardId::TrueGrit);
    assert_eq!(true_grit.card_type, CardType::Skill);
    assert_eq!(true_grit.rarity, CardRarity::Common);
    assert_eq!(true_grit.cost, 1);
    assert_eq!(true_grit.base_block, 7);
    assert_eq!(true_grit.base_magic, 0);
    assert_eq!(true_grit.target, CardTarget::SelfTarget);
    assert_eq!(true_grit.upgrade_block, 2);

    let twin_strike = get_card_definition(CardId::TwinStrike);
    assert_eq!(twin_strike.card_type, CardType::Attack);
    assert_eq!(twin_strike.rarity, CardRarity::Common);
    assert_eq!(twin_strike.cost, 1);
    assert_eq!(twin_strike.base_damage, 5);
    assert_eq!(twin_strike.target, CardTarget::Enemy);
    assert!(twin_strike.tags.contains(&CardTag::Strike));
    assert_eq!(twin_strike.upgrade_damage, 2);
}

#[test]
fn ironclad_random_and_exhaust_attack_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![
            Power {
                power_type: PowerId::Strength,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            },
            Power {
                power_type: PowerId::Dexterity,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            },
        ],
    );
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 601;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 602;
    state.entities.monsters = vec![first, second];

    let mut sword_plus = CombatCard::new(CardId::SwordBoomerang, 370);
    sword_plus.upgrades = 1;
    let sword_actions = resolve_card_play(CardId::SwordBoomerang, &state, &sword_plus, None);
    assert_eq!(sword_actions.len(), 4);
    for action in &sword_actions {
        match &action.action {
            Action::AttackDamageRandomEnemyCard { card } => {
                assert_eq!(card.id, CardId::SwordBoomerang);
                assert_eq!(card.upgrades, 1);
            }
            other => {
                panic!("Sword Boomerang+ should queue AttackDamageRandomEnemyAction, got {other:?}")
            }
        }
    }

    let mut thunder_plus = CombatCard::new(CardId::ThunderClap, 371);
    thunder_plus.upgrades = 1;
    let thunder_actions = resolve_card_play(CardId::ThunderClap, &state, &thunder_plus, None);
    assert_eq!(thunder_actions.len(), 3);
    match &thunder_actions[0].action {
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            is_modified,
        } => {
            assert_eq!(*source, 0);
            assert_eq!(damages.as_slice(), &[9, 9]);
            assert_eq!(*damage_type, DamageType::Normal);
            assert!(!*is_modified);
        }
        other => panic!("Thunderclap+ should damage all enemies first, got {other:?}"),
    }
    assert!(matches!(
        thunder_actions[1].action,
        Action::ApplyPower {
            source: 0,
            target: 601,
            power_id: PowerId::Vulnerable,
            amount: 1
        }
    ));
    assert!(matches!(
        thunder_actions[2].action,
        Action::ApplyPower {
            source: 0,
            target: 602,
            power_id: PowerId::Vulnerable,
            amount: 1
        }
    ));

    let mut true_grit_plus = CombatCard::new(CardId::TrueGrit, 372);
    true_grit_plus.upgrades = 1;
    let mut true_grit_state = state.clone();
    true_grit_state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 373),
        CombatCard::new(CardId::Defend, 374),
    ];
    let true_grit_actions =
        resolve_card_play(CardId::TrueGrit, &true_grit_state, &true_grit_plus, None);
    assert_eq!(true_grit_actions.len(), 2);
    assert!(matches!(
        true_grit_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 11
        }
    ));
    assert!(matches!(
        true_grit_actions[1].action,
        Action::ExhaustFromHand {
            amount: 1,
            random: false,
            any_number: false,
            can_pick_zero: false
        }
    ));

    let mut twin_plus = CombatCard::new(CardId::TwinStrike, 375);
    twin_plus.upgrades = 1;
    let twin_actions = resolve_card_play(CardId::TwinStrike, &state, &twin_plus, Some(601));
    assert_eq!(twin_actions.len(), 2);
    for action in &twin_actions {
        match &action.action {
            Action::Damage(info) => {
                assert_eq!(info.source, 0);
                assert_eq!(info.target, 601);
                assert_eq!(info.base, 9);
                assert_eq!(info.output, 9);
                assert_eq!(info.damage_type, DamageType::Normal);
            }
            other => panic!("Twin Strike+ should emit two DamageActions, got {other:?}"),
        }
    }
}

#[test]
fn random_enemy_attacks_ignore_half_dead_monsters_like_java_random_monster() {
    let mut state = crate::test_support::blank_test_combat();
    let mut half_dead = crate::test_support::test_monster(EnemyId::Darkling);
    half_dead.id = 610;
    half_dead.current_hp = 20;
    half_dead.half_dead = true;
    state.entities.monsters = vec![half_dead];

    crate::engine::action_handlers::damage::handle_damage_random_enemy(
        0,
        7,
        DamageType::Normal,
        &mut state,
    );

    let target = state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == 610)
        .expect("test monster should remain present");
    assert_eq!(
        target.current_hp, 20,
        "Java MonsterGroup.getRandomMonster(aliveOnly=true) excludes halfDead monsters"
    );
}

#[test]
fn random_enemy_attacks_do_not_filter_zero_hp_before_action_like_java_random_monster() {
    let mut state = crate::test_support::blank_test_combat();
    let mut zero_hp_not_dying = crate::test_support::test_monster(EnemyId::JawWorm);
    zero_hp_not_dying.id = 614;
    zero_hp_not_dying.current_hp = 0;
    zero_hp_not_dying.is_dying = false;
    zero_hp_not_dying.is_escaped = false;
    zero_hp_not_dying.half_dead = false;
    state.entities.monsters = vec![zero_hp_not_dying];

    crate::engine::action_handlers::damage::handle_damage_random_enemy(
        0,
        7,
        DamageType::Normal,
        &mut state,
    );

    match state
        .pop_next_action()
        .expect("Java random target selection can still pick a 0 HP non-dying monster")
    {
        Action::Damage(info) => {
            assert_eq!(info.target, 614);
            assert_eq!(info.output, 7);
        }
        other => panic!("DamageRandomEnemyAction should queue DamageAction, got {other:?}"),
    }
}

#[test]
fn attack_damage_random_enemy_card_recalculates_damage_at_execution_like_java() {
    let mut state = crate::test_support::blank_test_combat();
    let mut target = crate::test_support::test_monster(EnemyId::JawWorm);
    target.id = 611;
    target.current_hp = 30;
    state.entities.monsters = vec![target];

    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 4,
            extra_data: 0,
            just_applied: false,
        }],
    );

    crate::engine::action_handlers::execute_action(
        Action::AttackDamageRandomEnemyCard {
            card: Box::new(CombatCard::new(CardId::SwordBoomerang, 612)),
        },
        &mut state,
    );

    assert_eq!(
        state.entities.monsters[0].current_hp, 30,
        "Java AttackDamageRandomEnemyAction queues a DamageAction instead of damaging inline"
    );
    match state
        .pop_next_action()
        .expect("AttackDamageRandomEnemyAction should queue DamageAction")
    {
        Action::Damage(info) => {
            assert_eq!(info.target, 611);
            assert_eq!(
                info.output, 7,
                "Java recalculates the card at random-target action execution time"
            );
        }
        other => panic!("AttackDamageRandomEnemyAction should queue DamageAction, got {other:?}"),
    }
}

#[test]
fn true_grit_exhaust_action_edges_match_java_exhaust_action() {
    let empty_state = crate::test_support::blank_test_combat();
    let true_grit = CombatCard::new(CardId::TrueGrit, 380);
    let empty_actions = resolve_card_play(CardId::TrueGrit, &empty_state, &true_grit, None);
    assert_eq!(empty_actions.len(), 2);
    assert!(matches!(
        empty_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 7
        }
    ));
    assert_eq!(
        empty_actions[1].action,
        Action::ExhaustFromHand {
            amount: 1,
            random: true,
            any_number: false,
            can_pick_zero: false,
        },
        "Java True Grit queues ExhaustAction even when it will fizzle on an empty hand"
    );

    let mut one_card_state = crate::test_support::blank_test_combat();
    one_card_state.zones.hand = vec![CombatCard::new(CardId::Strike, 381)];
    let one_card_actions = resolve_card_play(CardId::TrueGrit, &one_card_state, &true_grit, None);
    assert_eq!(one_card_actions.len(), 2);
    assert!(matches!(
        one_card_actions[1].action,
        Action::ExhaustFromHand {
            amount: 1,
            random: true,
            any_number: false,
            can_pick_zero: false
        }
    ));

    let mut two_card_state = crate::test_support::blank_test_combat();
    two_card_state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 382),
        CombatCard::new(CardId::Defend, 383),
    ];
    let two_card_actions = resolve_card_play(CardId::TrueGrit, &two_card_state, &true_grit, None);
    assert_eq!(two_card_actions.len(), 2);
    assert!(matches!(
        two_card_actions[1].action,
        Action::ExhaustFromHand {
            amount: 1,
            random: true,
            any_number: false,
            can_pick_zero: false
        }
    ));

    let mut true_grit_plus = CombatCard::new(CardId::TrueGrit, 384);
    true_grit_plus.upgrades = 1;
    let one_card_plus_actions =
        resolve_card_play(CardId::TrueGrit, &one_card_state, &true_grit_plus, None);
    assert_eq!(one_card_plus_actions.len(), 2);
    assert!(matches!(
        one_card_plus_actions[1].action,
        Action::ExhaustFromHand {
            amount: 1,
            random: false,
            any_number: false,
            can_pick_zero: false
        }
    ));

    let two_card_plus_actions =
        resolve_card_play(CardId::TrueGrit, &two_card_state, &true_grit_plus, None);
    assert_eq!(two_card_plus_actions.len(), 2);
    assert!(matches!(
        two_card_plus_actions[1].action,
        Action::ExhaustFromHand {
            amount: 1,
            random: false,
            any_number: false,
            can_pick_zero: false
        }
    ));
}

#[test]
fn ironclad_debuff_draw_xcost_and_wound_definitions_match_java_sources() {
    let uppercut = get_card_definition(CardId::Uppercut);
    assert_eq!(uppercut.card_type, CardType::Attack);
    assert_eq!(uppercut.rarity, CardRarity::Uncommon);
    assert_eq!(uppercut.cost, 2);
    assert_eq!(uppercut.base_damage, 13);
    assert_eq!(uppercut.base_magic, 1);
    assert_eq!(uppercut.target, CardTarget::Enemy);
    assert_eq!(uppercut.upgrade_magic, 1);

    let warcry = get_card_definition(CardId::Warcry);
    assert_eq!(warcry.card_type, CardType::Skill);
    assert_eq!(warcry.rarity, CardRarity::Common);
    assert_eq!(warcry.cost, 0);
    assert_eq!(warcry.base_magic, 1);
    assert_eq!(warcry.target, CardTarget::SelfTarget);
    assert!(warcry.exhaust);
    assert_eq!(warcry.upgrade_magic, 1);

    let whirlwind = get_card_definition(CardId::Whirlwind);
    assert_eq!(whirlwind.card_type, CardType::Attack);
    assert_eq!(whirlwind.rarity, CardRarity::Uncommon);
    assert_eq!(whirlwind.cost, -1);
    assert_eq!(whirlwind.base_damage, 5);
    assert_eq!(whirlwind.target, CardTarget::AllEnemy);
    assert!(whirlwind.is_multi_damage);
    assert_eq!(whirlwind.upgrade_damage, 3);

    let wild_strike = get_card_definition(CardId::WildStrike);
    assert_eq!(wild_strike.card_type, CardType::Attack);
    assert_eq!(wild_strike.rarity, CardRarity::Common);
    assert_eq!(wild_strike.cost, 1);
    assert_eq!(wild_strike.base_damage, 12);
    assert_eq!(wild_strike.target, CardTarget::Enemy);
    assert!(wild_strike.tags.contains(&CardTag::Strike));
    assert_eq!(wild_strike.upgrade_damage, 5);
}

#[test]
fn ironclad_debuff_draw_xcost_and_wound_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    crate::content::powers::store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 701;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 702;
    state.entities.monsters = vec![first, second];

    let mut uppercut_plus = CombatCard::new(CardId::Uppercut, 390);
    uppercut_plus.upgrades = 1;
    let uppercut_actions = resolve_card_play(CardId::Uppercut, &state, &uppercut_plus, Some(701));
    assert_eq!(uppercut_actions.len(), 3);
    match &uppercut_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 701);
            assert_eq!(info.base, 15);
            assert_eq!(info.output, 15);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Uppercut+ should damage before debuffs, got {other:?}"),
    }
    assert!(matches!(
        uppercut_actions[1].action,
        Action::ApplyPower {
            source: 0,
            target: 701,
            power_id: PowerId::Weak,
            amount: 2
        }
    ));
    assert!(matches!(
        uppercut_actions[2].action,
        Action::ApplyPower {
            source: 0,
            target: 701,
            power_id: PowerId::Vulnerable,
            amount: 2
        }
    ));

    let mut warcry_plus = CombatCard::new(CardId::Warcry, 391);
    warcry_plus.upgrades = 1;
    let warcry_actions = resolve_card_play(CardId::Warcry, &state, &warcry_plus, None);
    assert_eq!(warcry_actions.len(), 2);
    assert!(matches!(warcry_actions[0].action, Action::DrawCards(2)));
    assert!(matches!(
        warcry_actions[1].action,
        Action::PutOnDeck {
            amount: 1,
            random: false
        }
    ));

    let mut whirlwind_plus = CombatCard::new(CardId::Whirlwind, 392);
    whirlwind_plus.upgrades = 1;
    whirlwind_plus.energy_on_use = 3;
    let whirlwind_actions = resolve_card_play(CardId::Whirlwind, &state, &whirlwind_plus, None);
    assert_eq!(whirlwind_actions.len(), 1);
    match &whirlwind_actions[0].action {
        Action::Whirlwind {
            damages,
            damage_type,
            free_to_play_once,
            energy_on_use,
        } => {
            assert_eq!(damages.as_slice(), &[10, 10]);
            assert_eq!(*damage_type, DamageType::Normal);
            assert!(!*free_to_play_once);
            assert_eq!(*energy_on_use, 3);
        }
        other => panic!("Whirlwind+ should emit WhirlwindAction equivalent, got {other:?}"),
    }

    let mut chemical_x_state = state.clone();
    chemical_x_state.turn.energy = 3;
    chemical_x_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::ChemicalX,
        ));
    crate::engine::action_handlers::execute_action(
        Action::Whirlwind {
            damages: smallvec::smallvec![10, 10],
            damage_type: DamageType::Normal,
            free_to_play_once: false,
            energy_on_use: 3,
        },
        &mut chemical_x_state,
    );
    assert_eq!(chemical_x_state.turn.energy, 0);
    let mut queued_damage_all = 0;
    while let Some(action) = chemical_x_state.pop_next_action() {
        assert!(matches!(action, Action::DamageAllEnemies { .. }));
        queued_damage_all += 1;
    }
    assert_eq!(queued_damage_all, 5);

    let mut free_x_state = state.clone();
    free_x_state.turn.energy = 3;
    crate::engine::action_handlers::execute_action(
        Action::Whirlwind {
            damages: smallvec::smallvec![10, 10],
            damage_type: DamageType::Normal,
            free_to_play_once: true,
            energy_on_use: 3,
        },
        &mut free_x_state,
    );
    assert_eq!(free_x_state.turn.energy, 3);

    let mut hand_x_state = state.clone();
    hand_x_state.turn.energy = 3;
    hand_x_state.zones.hand = vec![whirlwind_plus.clone()];
    crate::engine::action_handlers::cards::handle_play_card_from_hand(0, None, &mut hand_x_state)
        .expect("Whirlwind should be playable with current energy captured as X");
    assert_eq!(
        hand_x_state.turn.energy, 3,
        "Java does not spend X-card energy in the generic useCard path"
    );
    let queued_whirlwind = hand_x_state
        .pop_next_action()
        .expect("WhirlwindAction should be queued before UseCardDone");
    match &queued_whirlwind {
        Action::Whirlwind { energy_on_use, .. } => assert_eq!(*energy_on_use, 3),
        other => panic!("WhirlwindAction should be queued before UseCardDone, got {other:?}"),
    }
    crate::engine::action_handlers::execute_action(queued_whirlwind, &mut hand_x_state);
    assert_eq!(hand_x_state.turn.energy, 0);

    let mut autoplay_x_state = state.clone();
    autoplay_x_state.turn.energy = 3;
    let mut queued_whirlwind_card = CombatCard::new(CardId::Whirlwind, 394);
    queued_whirlwind_card.energy_on_use = 3;
    autoplay_x_state.enqueue_card_play(
        crate::runtime::combat::QueuedCardPlay {
            card: queued_whirlwind_card,
            target: None,
            energy_on_use: 3,
            ignore_energy_total: false,
            autoplay: true,
            random_target: false,
            is_end_turn_autoplay: false,
            purge_on_use: false,
            source: crate::runtime::combat::QueuedCardSource::Normal,
        },
        false,
    );
    let flush_autoplay = autoplay_x_state
        .pop_next_action()
        .expect("enqueueing an autoplay card should schedule a queue flush");
    assert!(matches!(flush_autoplay, Action::FlushNextQueuedCard));
    crate::engine::action_handlers::execute_action(flush_autoplay, &mut autoplay_x_state);
    let queued_direct_play = autoplay_x_state
        .pop_next_action()
        .expect("autoplay queue should emit a direct card play");
    match &queued_direct_play {
        Action::PlayCardDirect { card, .. } => assert!(
            !card.free_to_play_once,
            "Java autoplay sets isInAutoplay/ignoreEnergyOnUse, not freeToPlayOnce"
        ),
        other => panic!("autoplay queue should emit PlayCardDirect, got {other:?}"),
    }
    crate::engine::action_handlers::execute_action(queued_direct_play, &mut autoplay_x_state);
    let autoplay_whirlwind = autoplay_x_state
        .pop_next_action()
        .expect("autoplay Whirlwind should queue WhirlwindAction");
    match &autoplay_whirlwind {
        Action::Whirlwind {
            free_to_play_once,
            energy_on_use,
            ..
        } => {
            assert!(!*free_to_play_once);
            assert_eq!(*energy_on_use, 3);
        }
        other => panic!("autoplay Whirlwind should emit WhirlwindAction, got {other:?}"),
    }
    crate::engine::action_handlers::execute_action(autoplay_whirlwind, &mut autoplay_x_state);
    assert_eq!(
        autoplay_x_state.turn.energy, 0,
        "Java autoplayed X-cost cards still let their card-specific X action spend energy"
    );

    let mut wild_plus = CombatCard::new(CardId::WildStrike, 393);
    wild_plus.upgrades = 1;
    let wild_actions = resolve_card_play(CardId::WildStrike, &state, &wild_plus, Some(702));
    assert_eq!(wild_actions.len(), 2);
    match &wild_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.source, 0);
            assert_eq!(info.target, 702);
            assert_eq!(info.base, 19);
            assert_eq!(info.output, 19);
            assert_eq!(info.damage_type, DamageType::Normal);
        }
        other => panic!("Wild Strike+ should damage before Wound generation, got {other:?}"),
    }
    assert!(matches!(
        wild_actions[1].action,
        Action::MakeTempCardInDrawPile {
            card_id: CardId::Wound,
            amount: 1,
            random_spot: true,
            to_bottom: false,
            upgraded: false
        }
    ));
}

#[test]
fn lethal_damage_filters_post_combat_actions_like_java_action_manager() {
    let mut state = crate::test_support::blank_test_combat();
    let mut target = crate::test_support::test_monster(EnemyId::JawWorm);
    target.id = 720;
    target.current_hp = 5;
    state.entities.monsters = vec![target];

    state.queue_action_back(Action::MakeTempCardInDrawPile {
        card_id: CardId::Wound,
        amount: 1,
        random_spot: true,
        to_bottom: false,
        upgraded: false,
    });
    state.queue_action_back(Action::DrawCards(1));
    state.queue_action_back(Action::GainEnergy { amount: 1 });
    state.queue_action_back(Action::ApplyPower {
        source: 0,
        target: 720,
        power_id: PowerId::Vulnerable,
        amount: 1,
    });
    state.queue_action_back(Action::Whirlwind {
        damages: smallvec::smallvec![3],
        damage_type: DamageType::Normal,
        free_to_play_once: false,
        energy_on_use: 1,
    });
    state.queue_action_back(Action::DamageRandomEnemy {
        source: 0,
        base_damage: 3,
        damage_type: DamageType::Normal,
    });
    state.queue_action_back(Action::AttackDamageRandomEnemyCard {
        card: Box::new(CombatCard::new(CardId::SwordBoomerang, 721)),
    });
    state.queue_action_back(Action::DropkickDamageAndEffect {
        target: 720,
        damage_info: crate::runtime::action::DamageInfo {
            source: 0,
            target: 720,
            base: 5,
            output: 5,
            damage_type: DamageType::Normal,
            is_modified: false,
        },
    });
    state.queue_action_back(Action::GainBlock {
        target: 0,
        amount: 3,
    });
    state.queue_action_back(Action::Heal {
        target: 0,
        amount: 2,
    });
    state.queue_action_back(Action::UseCardDone {
        should_exhaust: false,
    });

    crate::engine::action_handlers::execute_action(
        Action::Damage(crate::runtime::action::DamageInfo {
            source: 0,
            target: 720,
            base: 99,
            output: 99,
            damage_type: DamageType::Normal,
            is_modified: false,
        }),
        &mut state,
    );

    let remaining: Vec<_> = std::iter::from_fn(|| state.pop_next_action()).collect();
    assert_eq!(
        remaining,
        vec![
            Action::DamageRandomEnemy {
                source: 0,
                base_damage: 3,
                damage_type: DamageType::Normal,
            },
            Action::GainBlock {
                target: 0,
                amount: 3
            },
            Action::Heal {
                target: 0,
                amount: 2
            },
            Action::UseCardDone {
                should_exhaust: false
            }
        ],
        "Java GameActionManager.clearPostCombatActions keeps DamageRandomEnemyAction/Heal/GainBlock/UseCardAction and removes generated cards, draw, energy, powers, and AttackDamageRandomEnemyAction"
    );
}

#[test]
fn transmutation_x_cost_action_matches_java_energy_and_chemical_x_timing() {
    let mut state = crate::test_support::blank_test_combat();
    state.turn.energy = 3;
    let mut transmutation = CombatCard::new(CardId::Transmutation, 394);
    transmutation.upgrades = 1;
    transmutation.energy_on_use = 1;

    let actions = resolve_card_play(CardId::Transmutation, &state, &transmutation, None);
    assert_eq!(actions.len(), 1);
    match &actions[0].action {
        Action::Transmutation {
            upgraded,
            free_to_play_once,
            energy_on_use,
        } => {
            assert!(*upgraded);
            assert!(!*free_to_play_once);
            assert_eq!(
                *energy_on_use, 3,
                "Java Transmutation.use raises stale energyOnUse to current EnergyPanel.totalCount"
            );
        }
        other => panic!("Transmutation should emit TransmutationAction equivalent, got {other:?}"),
    }

    state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::ChemicalX,
        ));
    crate::engine::action_handlers::execute_action(actions[0].action.clone(), &mut state);
    assert_eq!(state.turn.energy, 0);
    let mut generated = 0;
    while let Some(action) = state.pop_next_action() {
        assert!(matches!(
            action,
            Action::MakeRandomColorlessCardInHand {
                cost_for_turn: Some(0),
                upgraded: true
            }
        ));
        generated += 1;
    }
    assert_eq!(generated, 5);
}

#[test]
fn forethought_resolves_as_execution_time_action_like_java() {
    let state = crate::test_support::blank_test_combat();
    let forethought = CombatCard::new(CardId::Forethought, 830);
    let actions = resolve_card_play(CardId::Forethought, &state, &forethought, None);
    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0].action,
        Action::Forethought { upgraded: false },
        "Java Forethought.use always queues ForethoughtAction; hand size is read when that action executes"
    );

    let mut one_card_state = crate::test_support::blank_test_combat();
    let mut temporarily_free_defend = CombatCard::new(CardId::Defend, 831);
    temporarily_free_defend.set_cost_for_turn_java(0);
    one_card_state.zones.hand = vec![temporarily_free_defend];

    crate::engine::action_handlers::cards::handle_forethought(false, &mut one_card_state);

    assert!(one_card_state.zones.hand.is_empty());
    assert_eq!(one_card_state.zones.draw_pile.len(), 1);
    assert_eq!(one_card_state.zones.draw_pile[0].uuid, 831);
    assert!(
        one_card_state.zones.draw_pile[0].free_to_play_once,
        "Java ForethoughtAction auto-move path also checks AbstractCard.cost, not costForTurn"
    );
    assert_eq!(
        one_card_state.pop_next_action(),
        None,
        "Java ForethoughtAction auto-moves the only selectable card without opening the hand-select screen"
    );
}

#[test]
fn upgraded_forethought_opens_any_number_selection_at_execution() {
    let mut state = crate::test_support::blank_test_combat();
    state.zones.hand = vec![CombatCard::new(CardId::Strike, 832)];

    crate::engine::action_handlers::cards::handle_forethought(true, &mut state);

    assert!(matches!(
        state.pop_next_action(),
        Some(Action::SuspendForHandSelect {
            min: 0,
            max: 99,
            can_cancel: true,
            filter: crate::state::HandSelectFilter::Any,
            reason: crate::state::HandSelectReason::PutToBottomOfDraw,
        })
    ));
}

#[test]
fn thinking_ahead_uses_java_use_time_hand_visibility() {
    let mut direct_empty_hand = crate::test_support::blank_test_combat();
    direct_empty_hand.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 833)];
    let thinking = CombatCard::new(CardId::ThinkingAhead, 834);

    let direct_actions = resolve_card_play_with_context(
        CardId::ThinkingAhead,
        &direct_empty_hand,
        &thinking,
        None,
        CardUseContext {
            played_from_hand: false,
        },
    );
    assert_eq!(direct_actions.len(), 1);
    assert!(matches!(direct_actions[0].action, Action::DrawCards(2)));

    let hand_play_actions = resolve_card_play_with_context(
        CardId::ThinkingAhead,
        &direct_empty_hand,
        &thinking,
        None,
        CardUseContext {
            played_from_hand: true,
        },
    );
    assert_eq!(hand_play_actions.len(), 2);
    assert!(matches!(
        hand_play_actions[1].action,
        Action::SuspendForHandSelect {
            reason: crate::state::HandSelectReason::PutOnDrawPile,
            ..
        }
    ));

    let mut direct_with_hand = direct_empty_hand.clone();
    direct_with_hand.zones.hand = vec![CombatCard::new(CardId::Defend, 835)];
    let direct_with_hand_actions = resolve_card_play_with_context(
        CardId::ThinkingAhead,
        &direct_with_hand,
        &thinking,
        None,
        CardUseContext {
            played_from_hand: false,
        },
    );
    assert_eq!(
        direct_with_hand_actions.len(),
        2,
        "Java direct/autoplay use paths only queue PutOnDeckAction when another card is already in hand"
    );
}

#[test]
fn discard_cards_queue_java_discard_action_instead_of_prechecking_hand() {
    let empty_state = crate::test_support::blank_test_combat();

    let mut acrobatics = CombatCard::new(CardId::Acrobatics, 840);
    acrobatics.base_magic_num_mut = 3;
    let acrobatics_actions = resolve_card_play(CardId::Acrobatics, &empty_state, &acrobatics, None);
    assert_eq!(acrobatics_actions.len(), 2);
    assert!(matches!(acrobatics_actions[0].action, Action::DrawCards(3)));
    assert_eq!(
        acrobatics_actions[1].action,
        Action::DiscardFromHand {
            amount: 1,
            random: false,
            end_turn: false,
        },
        "Java Acrobatics.use always queues DiscardAction after DrawCardAction; it does not precheck the hand before drawing"
    );

    let survivor = CombatCard::new(CardId::Survivor, 841);
    let survivor_actions = resolve_card_play(CardId::Survivor, &empty_state, &survivor, None);
    assert_eq!(survivor_actions.len(), 2);
    assert_eq!(
        survivor_actions[1].action,
        Action::DiscardFromHand {
            amount: 1,
            random: false,
            end_turn: false,
        }
    );

    let mut prepared_plus = CombatCard::new(CardId::Prepared, 842);
    prepared_plus.upgrades = 1;
    prepared_plus.base_magic_num_mut = 2;
    let prepared_actions = resolve_card_play(CardId::Prepared, &empty_state, &prepared_plus, None);
    assert_eq!(
        prepared_actions[1].action,
        Action::DiscardFromHand {
            amount: 2,
            random: false,
            end_turn: false,
        }
    );

    let dagger_actions = resolve_card_play(
        CardId::DaggerThrow,
        &empty_state,
        &CombatCard::new(CardId::DaggerThrow, 843),
        Some(844),
    );
    assert_eq!(
        dagger_actions[2].action,
        Action::DiscardFromHand {
            amount: 1,
            random: false,
            end_turn: false,
        }
    );
}

#[test]
fn discard_from_hand_auto_discards_all_when_hand_size_is_not_greater_than_amount() {
    let mut state = crate::test_support::blank_test_combat();
    state.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
    state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 850),
        CombatCard::new(CardId::Defend, 851),
    ];

    crate::engine::action_handlers::cards::handle_discard_from_hand(2, false, false, &mut state);

    assert!(state.zones.hand.is_empty());
    assert_eq!(
        state
            .zones
            .discard_pile
            .iter()
            .map(|card| card.uuid)
            .collect::<Vec<_>>(),
        vec![851, 850],
        "Java DiscardAction repeatedly moves hand.getTopCard(); Rust discard_pile preserves Java CardGroup order with top at the end"
    );
    assert_eq!(
        state.pop_next_action(),
        None,
        "Java DiscardAction does not open a choice screen when hand.size() <= amount"
    );
}

#[test]
fn silent_common_batch_definitions_match_java_sources() {
    let cases = [
        (
            CardId::Deflect,
            "Deflect",
            CardType::Skill,
            0,
            0,
            4,
            0,
            CardTarget::SelfTarget,
            0,
            3,
            0,
        ),
        (
            CardId::QuickSlash,
            "Quick Slash",
            CardType::Attack,
            1,
            8,
            0,
            0,
            CardTarget::Enemy,
            4,
            0,
            0,
        ),
        (
            CardId::Slice,
            "Slice",
            CardType::Attack,
            0,
            6,
            0,
            0,
            CardTarget::Enemy,
            3,
            0,
            0,
        ),
        (
            CardId::FlyingKnee,
            "Flying Knee",
            CardType::Attack,
            1,
            8,
            0,
            0,
            CardTarget::Enemy,
            3,
            0,
            0,
        ),
        (
            CardId::DodgeAndRoll,
            "Dodge and Roll",
            CardType::Skill,
            1,
            0,
            4,
            0,
            CardTarget::SelfTarget,
            0,
            2,
            0,
        ),
        (
            CardId::SuckerPunch,
            "Sucker Punch",
            CardType::Attack,
            1,
            7,
            0,
            1,
            CardTarget::Enemy,
            2,
            0,
            1,
        ),
    ];

    let java_map = build_java_id_map();
    for (
        id,
        java_name,
        card_type,
        cost,
        damage,
        block,
        magic,
        target,
        upgrade_damage,
        upgrade_block,
        upgrade_magic,
    ) in cases
    {
        let def = get_card_definition(id);
        assert_eq!(def.name, java_name);
        assert_eq!(def.card_type, card_type);
        assert_eq!(def.rarity, CardRarity::Common);
        assert_eq!(def.cost, cost);
        assert_eq!(def.base_damage, damage);
        assert_eq!(def.base_block, block);
        assert_eq!(def.base_magic, magic);
        assert_eq!(def.target, target);
        assert_eq!(def.upgrade_damage, upgrade_damage);
        assert_eq!(def.upgrade_block, upgrade_block);
        assert_eq!(def.upgrade_magic, upgrade_magic);
        assert_eq!(java_id(id), java_name);
        assert_eq!(java_map.get(java_name), Some(&id));
    }
}

#[test]
fn silent_common_batch_runtime_actions_match_java_use_methods() {
    fn assert_damage(action: &Action, target: usize, amount: i32) {
        match action {
            Action::Damage(info) => {
                assert_eq!(info.source, 0);
                assert_eq!(info.target, target);
                assert_eq!(info.base, amount);
                assert_eq!(info.output, amount);
                assert_eq!(info.damage_type, DamageType::Normal);
            }
            other => panic!("expected DamageAction, got {other:?}"),
        }
    }

    let state = crate::test_support::blank_test_combat();

    let deflect = resolve_card_play(
        CardId::Deflect,
        &state,
        &CombatCard::new(CardId::Deflect, 880),
        None,
    );
    assert_eq!(
        deflect[0].action,
        Action::GainBlock {
            target: 0,
            amount: 4,
        }
    );

    let quick_slash = resolve_card_play(
        CardId::QuickSlash,
        &state,
        &CombatCard::new(CardId::QuickSlash, 881),
        Some(7),
    );
    assert_damage(&quick_slash[0].action, 7, 8);
    assert_eq!(quick_slash[1].action, Action::DrawCards(1));

    let slice = resolve_card_play(
        CardId::Slice,
        &state,
        &CombatCard::new(CardId::Slice, 882),
        Some(7),
    );
    assert_damage(&slice[0].action, 7, 6);

    let flying_knee = resolve_card_play(
        CardId::FlyingKnee,
        &state,
        &CombatCard::new(CardId::FlyingKnee, 883),
        Some(7),
    );
    assert_damage(&flying_knee[0].action, 7, 8);
    assert_eq!(
        flying_knee[1].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Energized,
            amount: 1,
        }
    );

    let dodge = resolve_card_play(
        CardId::DodgeAndRoll,
        &state,
        &CombatCard::new(CardId::DodgeAndRoll, 884),
        None,
    );
    assert_eq!(
        dodge[0].action,
        Action::GainBlock {
            target: 0,
            amount: 4,
        }
    );
    assert_eq!(
        dodge[1].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::NextTurnBlock,
            amount: 4,
        }
    );

    let sucker_punch = resolve_card_play(
        CardId::SuckerPunch,
        &state,
        &CombatCard::new(CardId::SuckerPunch, 885),
        Some(7),
    );
    assert_damage(&sucker_punch[0].action, 7, 7);
    assert_eq!(
        sucker_punch[1].action,
        Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Weak,
            amount: 1,
        }
    );
}

#[test]
fn silent_economy_and_dash_cards_match_java_sources() {
    let outmaneuver = get_card_definition(CardId::Outmaneuver);
    assert_eq!(outmaneuver.name, "Outmaneuver");
    assert_eq!(outmaneuver.card_type, CardType::Skill);
    assert_eq!(outmaneuver.rarity, CardRarity::Common);
    assert_eq!(outmaneuver.cost, 1);
    assert_eq!(outmaneuver.base_magic, 2);
    assert_eq!(outmaneuver.upgrade_magic, 1);
    assert_eq!(java_id(CardId::Outmaneuver), "Outmaneuver");

    let sneaky = get_card_definition(CardId::SneakyStrike);
    assert_eq!(sneaky.name, "Sneaky Strike");
    assert_eq!(sneaky.card_type, CardType::Attack);
    assert_eq!(sneaky.rarity, CardRarity::Common);
    assert_eq!(sneaky.cost, 2);
    assert_eq!(sneaky.base_damage, 12);
    assert_eq!(sneaky.upgrade_damage, 4);
    assert!(sneaky.tags.contains(&CardTag::Strike));
    assert_eq!(java_id(CardId::SneakyStrike), "Underhanded Strike");
    assert_eq!(
        build_java_id_map().get("Underhanded Strike"),
        Some(&CardId::SneakyStrike)
    );

    let dash = get_card_definition(CardId::Dash);
    assert_eq!(dash.name, "Dash");
    assert_eq!(dash.card_type, CardType::Attack);
    assert_eq!(dash.rarity, CardRarity::Uncommon);
    assert_eq!(dash.cost, 2);
    assert_eq!(dash.base_damage, 10);
    assert_eq!(dash.base_block, 10);
    assert_eq!(dash.upgrade_damage, 3);
    assert_eq!(dash.upgrade_block, 3);
}

#[test]
fn silent_economy_and_dash_runtime_actions_match_java_use_methods() {
    let state = crate::test_support::blank_test_combat();

    let outmaneuver = resolve_card_play(
        CardId::Outmaneuver,
        &state,
        &CombatCard::new(CardId::Outmaneuver, 886),
        None,
    );
    assert_eq!(
        outmaneuver[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Energized,
            amount: 2,
        }
    );

    let mut outmaneuver_plus = CombatCard::new(CardId::Outmaneuver, 887);
    outmaneuver_plus.upgrades = 1;
    let outmaneuver_plus_actions =
        resolve_card_play(CardId::Outmaneuver, &state, &outmaneuver_plus, None);
    assert_eq!(
        outmaneuver_plus_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Energized,
            amount: 3,
        }
    );

    let sneaky = resolve_card_play(
        CardId::SneakyStrike,
        &state,
        &CombatCard::new(CardId::SneakyStrike, 888),
        Some(7),
    );
    match &sneaky[0].action {
        Action::Damage(info) => {
            assert_eq!(info.target, 7);
            assert_eq!(info.output, 12);
        }
        other => panic!("Sneaky Strike first action should damage, got {other:?}"),
    }
    assert_eq!(
        sneaky[1].action,
        Action::GainEnergyIfDiscardedThisTurn { amount: 2 }
    );

    let mut no_discard_state = crate::test_support::blank_test_combat();
    no_discard_state.turn.set_energy(0);
    crate::engine::action_handlers::execute_action(
        Action::GainEnergyIfDiscardedThisTurn { amount: 2 },
        &mut no_discard_state,
    );
    assert_eq!(no_discard_state.turn.energy, 0);

    let mut discard_state = crate::test_support::blank_test_combat();
    discard_state.turn.set_energy(0);
    discard_state.turn.increment_cards_discarded();
    crate::engine::action_handlers::execute_action(
        Action::GainEnergyIfDiscardedThisTurn { amount: 2 },
        &mut discard_state,
    );
    assert_eq!(discard_state.turn.energy, 2);

    let dash = resolve_card_play(
        CardId::Dash,
        &state,
        &CombatCard::new(CardId::Dash, 889),
        Some(7),
    );
    assert_eq!(
        dash[0].action,
        Action::GainBlock {
            target: 0,
            amount: 10,
        }
    );
    match &dash[1].action {
        Action::Damage(info) => {
            assert_eq!(info.target, 7);
            assert_eq!(info.output, 10);
        }
        other => panic!("Dash second action should damage, got {other:?}"),
    }
}

#[test]
fn bane_uses_java_delayed_poison_check_for_second_hit() {
    let bane = get_card_definition(CardId::Bane);
    assert_eq!(bane.name, "Bane");
    assert_eq!(bane.card_type, CardType::Attack);
    assert_eq!(bane.rarity, CardRarity::Common);
    assert_eq!(bane.cost, 1);
    assert_eq!(bane.base_damage, 7);
    assert_eq!(bane.upgrade_damage, 3);
    assert_eq!(java_id(CardId::Bane), "Bane");

    let state = crate::test_support::blank_test_combat();
    let actions = resolve_card_play(
        CardId::Bane,
        &state,
        &CombatCard::new(CardId::Bane, 890),
        Some(7),
    );
    assert!(matches!(actions[0].action, Action::Damage(_)));
    let Action::BaneDamage(bane_info) = actions[1].action.clone() else {
        panic!("Bane second action should be Java BaneAction");
    };
    assert_eq!(bane_info.target, 7);
    assert_eq!(bane_info.output, 7);

    let mut no_poison_state = crate::test_support::blank_test_combat();
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = 7;
    monster.current_hp = 20;
    no_poison_state.entities.monsters = vec![monster.clone()];
    crate::engine::action_handlers::execute_action(
        Action::BaneDamage(bane_info.clone()),
        &mut no_poison_state,
    );
    assert_eq!(
        no_poison_state.entities.monsters[0].current_hp, 20,
        "Java BaneAction does nothing if the target lacks Poison at execution time"
    );

    let mut poison_state = crate::test_support::blank_test_combat();
    poison_state.entities.monsters = vec![monster.clone()];
    poison_state.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Poison,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::execute_action(
        Action::BaneDamage(bane_info.clone()),
        &mut poison_state,
    );
    assert_eq!(poison_state.entities.monsters[0].current_hp, 13);

    let mut dead_poison_state = crate::test_support::blank_test_combat();
    let mut dead_monster = monster;
    dead_monster.current_hp = 0;
    dead_poison_state.entities.monsters = vec![dead_monster];
    dead_poison_state.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Poison,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::execute_action(
        Action::BaneDamage(bane_info),
        &mut dead_poison_state,
    );
    assert_eq!(
        dead_poison_state.entities.monsters[0].current_hp, 0,
        "Java BaneAction checks target.currentHealth > 0 before applying the second hit"
    );
}

#[test]
fn silent_discard_action_cards_match_java_sources() {
    let all_out = get_card_definition(CardId::AllOutAttack);
    assert_eq!(all_out.name, "All-Out Attack");
    assert_eq!(all_out.card_type, CardType::Attack);
    assert_eq!(all_out.rarity, CardRarity::Uncommon);
    assert_eq!(all_out.cost, 1);
    assert_eq!(all_out.base_damage, 10);
    assert!(all_out.is_multi_damage);
    assert_eq!(all_out.upgrade_damage, 4);
    assert_eq!(java_id(CardId::AllOutAttack), "All Out Attack");

    let concentrate = get_card_definition(CardId::Concentrate);
    assert_eq!(concentrate.name, "Concentrate");
    assert_eq!(concentrate.card_type, CardType::Skill);
    assert_eq!(concentrate.rarity, CardRarity::Uncommon);
    assert_eq!(concentrate.cost, 0);
    assert_eq!(concentrate.base_magic, 3);
    assert_eq!(concentrate.upgrade_magic, -1);

    let gamble = get_card_definition(CardId::CalculatedGamble);
    assert_eq!(gamble.name, "Calculated Gamble");
    assert_eq!(gamble.card_type, CardType::Skill);
    assert_eq!(gamble.rarity, CardRarity::Uncommon);
    assert_eq!(gamble.cost, 0);
    assert!(gamble.exhaust);
    assert!(exhausts_when_played(&CombatCard::new(
        CardId::CalculatedGamble,
        891
    )));
    let mut gamble_plus = CombatCard::new(CardId::CalculatedGamble, 892);
    gamble_plus.upgrades = 1;
    assert!(
        !exhausts_when_played(&gamble_plus),
        "Calculated Gamble+ changes exhaust only"
    );
}

#[test]
fn silent_discard_action_cards_runtime_actions_match_java_use_methods() {
    let mut state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 7;
    let mut second = crate::test_support::test_monster(EnemyId::JawWorm);
    second.id = 8;
    state.entities.monsters = vec![first, second];

    let all_out = resolve_card_play(
        CardId::AllOutAttack,
        &state,
        &CombatCard::new(CardId::AllOutAttack, 893),
        None,
    );
    assert_eq!(all_out.len(), 2);
    assert_eq!(
        all_out[0].action,
        Action::DamageAllEnemies {
            source: 0,
            damages: smallvec::smallvec![10, 10],
            damage_type: DamageType::Normal,
            is_modified: true,
        }
    );
    assert_eq!(
        all_out[1].action,
        Action::DiscardFromHand {
            amount: 1,
            random: true,
            end_turn: false,
        }
    );

    let concentrate = resolve_card_play(
        CardId::Concentrate,
        &state,
        &CombatCard::new(CardId::Concentrate, 894),
        None,
    );
    assert_eq!(
        concentrate[0].action,
        Action::DiscardFromHand {
            amount: 3,
            random: false,
            end_turn: false,
        }
    );
    assert_eq!(concentrate[1].action, Action::GainEnergy { amount: 2 });

    let mut concentrate_plus = CombatCard::new(CardId::Concentrate, 895);
    concentrate_plus.upgrades = 1;
    let concentrate_plus_actions =
        resolve_card_play(CardId::Concentrate, &state, &concentrate_plus, None);
    assert_eq!(
        concentrate_plus_actions[0].action,
        Action::DiscardFromHand {
            amount: 2,
            random: false,
            end_turn: false,
        }
    );

    let gamble = resolve_card_play(
        CardId::CalculatedGamble,
        &state,
        &CombatCard::new(CardId::CalculatedGamble, 896),
        None,
    );
    assert_eq!(
        gamble[0].action,
        Action::CalculatedGamble { draw_extra: false }
    );

    let mut gamble_plus = CombatCard::new(CardId::CalculatedGamble, 897);
    gamble_plus.upgrades = 1;
    let gamble_plus_actions =
        resolve_card_play(CardId::CalculatedGamble, &state, &gamble_plus, None);
    assert_eq!(
        gamble_plus_actions[0].action,
        Action::CalculatedGamble { draw_extra: false },
        "Java CalculatedGamble.use passes false even when upgraded"
    );

    let mut runtime_state = crate::test_support::blank_test_combat();
    runtime_state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 898),
        CombatCard::new(CardId::Defend, 899),
    ];
    crate::engine::action_handlers::execute_action(
        Action::CalculatedGamble { draw_extra: false },
        &mut runtime_state,
    );
    assert_eq!(
        runtime_state.pop_next_action(),
        Some(Action::DiscardFromHand {
            amount: 2,
            random: true,
            end_turn: false,
        })
    );
    assert_eq!(runtime_state.pop_next_action(), Some(Action::DrawCards(2)));
}

#[test]
fn silent_hand_conversion_cards_match_java_sources() {
    let storm = get_card_definition(CardId::StormOfSteel);
    assert_eq!(storm.name, "Storm of Steel");
    assert_eq!(storm.card_type, CardType::Skill);
    assert_eq!(storm.rarity, CardRarity::Rare);
    assert_eq!(storm.cost, 1);
    assert_eq!(storm.target, CardTarget::None);
    assert_eq!(java_id(CardId::StormOfSteel), "Storm of Steel");

    let unload = get_card_definition(CardId::Unload);
    assert_eq!(unload.name, "Unload");
    assert_eq!(unload.card_type, CardType::Attack);
    assert_eq!(unload.rarity, CardRarity::Rare);
    assert_eq!(unload.cost, 1);
    assert_eq!(unload.base_damage, 14);
    assert_eq!(unload.upgrade_damage, 4);
    assert_eq!(java_id(CardId::Unload), "Unload");
}

#[test]
fn silent_hand_conversion_cards_queue_java_execution_actions() {
    let state = crate::test_support::blank_test_combat();

    let storm = resolve_card_play(
        CardId::StormOfSteel,
        &state,
        &CombatCard::new(CardId::StormOfSteel, 900),
        None,
    );
    assert_eq!(storm[0].action, Action::BladeFury { upgraded: false });

    let mut storm_plus = CombatCard::new(CardId::StormOfSteel, 901);
    storm_plus.upgrades = 1;
    let storm_plus_actions = resolve_card_play(CardId::StormOfSteel, &state, &storm_plus, None);
    assert_eq!(
        storm_plus_actions[0].action,
        Action::BladeFury { upgraded: true }
    );

    let mut blade_state = crate::test_support::blank_test_combat();
    blade_state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 902),
        CombatCard::new(CardId::Defend, 903),
    ];
    crate::engine::action_handlers::execute_action(
        Action::BladeFury { upgraded: true },
        &mut blade_state,
    );
    assert_eq!(
        blade_state.pop_next_action(),
        Some(Action::DiscardFromHand {
            amount: 2,
            random: false,
            end_turn: false,
        }),
        "Java BladeFuryAction addToTop's DiscardAction after MakeTempCardInHandAction, so discard executes first"
    );
    assert_eq!(
        blade_state.pop_next_action(),
        Some(Action::MakeTempCardInHand {
            card_id: CardId::Shiv,
            amount: 2,
            upgraded: true,
        })
    );

    let unload = resolve_card_play(
        CardId::Unload,
        &state,
        &CombatCard::new(CardId::Unload, 904),
        Some(7),
    );
    match &unload[0].action {
        Action::Damage(info) => {
            assert_eq!(info.target, 7);
            assert_eq!(info.output, 14);
        }
        other => panic!("Unload first action should damage, got {other:?}"),
    }
    assert_eq!(unload[1].action, Action::UnloadNonAttack);

    let mut unload_state = crate::test_support::blank_test_combat();
    unload_state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 905),
        CombatCard::new(CardId::Defend, 906),
        CombatCard::new(CardId::Reflex, 907),
    ];
    crate::engine::action_handlers::execute_action(Action::UnloadNonAttack, &mut unload_state);
    assert_eq!(
        unload_state.pop_next_action(),
        Some(Action::DiscardCard { card_uuid: 907 }),
        "Java UnloadAction addToTop's DiscardSpecificCardAction while iterating hand; later non-attacks execute first"
    );
    assert_eq!(
        unload_state.pop_next_action(),
        Some(Action::DiscardCard { card_uuid: 906 })
    );
    assert_eq!(unload_state.pop_next_action(), None);
}

#[test]
fn silent_direct_attack_batch_matches_java_sources() {
    let backstab = get_card_definition(CardId::Backstab);
    assert_eq!(backstab.name, "Backstab");
    assert_eq!(backstab.card_type, CardType::Attack);
    assert_eq!(backstab.rarity, CardRarity::Uncommon);
    assert_eq!(backstab.cost, 0);
    assert_eq!(backstab.base_damage, 11);
    assert_eq!(backstab.upgrade_damage, 4);
    assert!(backstab.exhaust);
    assert!(backstab.innate);
    assert_eq!(java_id(CardId::Backstab), "Backstab");

    let riddle = get_card_definition(CardId::RiddleWithHoles);
    assert_eq!(riddle.name, "Riddle With Holes");
    assert_eq!(riddle.card_type, CardType::Attack);
    assert_eq!(riddle.rarity, CardRarity::Uncommon);
    assert_eq!(riddle.cost, 2);
    assert_eq!(riddle.base_damage, 3);
    assert_eq!(riddle.upgrade_damage, 1);
    assert_eq!(java_id(CardId::RiddleWithHoles), "Riddle With Holes");

    let die = get_card_definition(CardId::DieDieDie);
    assert_eq!(die.name, "Die Die Die");
    assert_eq!(die.card_type, CardType::Attack);
    assert_eq!(die.rarity, CardRarity::Rare);
    assert_eq!(die.cost, 1);
    assert_eq!(die.base_damage, 13);
    assert_eq!(die.upgrade_damage, 4);
    assert!(die.is_multi_damage);
    assert!(die.exhaust);
    assert_eq!(java_id(CardId::DieDieDie), "Die Die Die");

    let finisher = get_card_definition(CardId::Finisher);
    assert_eq!(finisher.name, "Finisher");
    assert_eq!(finisher.card_type, CardType::Attack);
    assert_eq!(finisher.rarity, CardRarity::Uncommon);
    assert_eq!(finisher.cost, 1);
    assert_eq!(finisher.base_damage, 6);
    assert_eq!(finisher.upgrade_damage, 2);
    assert_eq!(java_id(CardId::Finisher), "Finisher");

    let flechettes = get_card_definition(CardId::Flechettes);
    assert_eq!(flechettes.name, "Flechettes");
    assert_eq!(flechettes.card_type, CardType::Attack);
    assert_eq!(flechettes.rarity, CardRarity::Uncommon);
    assert_eq!(flechettes.cost, 1);
    assert_eq!(flechettes.base_damage, 4);
    assert_eq!(flechettes.upgrade_damage, 2);
    assert_eq!(java_id(CardId::Flechettes), "Flechettes");

    let heel = get_card_definition(CardId::HeelHook);
    assert_eq!(heel.name, "Heel Hook");
    assert_eq!(heel.card_type, CardType::Attack);
    assert_eq!(heel.rarity, CardRarity::Uncommon);
    assert_eq!(heel.cost, 1);
    assert_eq!(heel.base_damage, 5);
    assert_eq!(heel.upgrade_damage, 3);
    assert_eq!(java_id(CardId::HeelHook), "Heel Hook");

    let expertise = get_card_definition(CardId::Expertise);
    assert_eq!(expertise.name, "Expertise");
    assert_eq!(expertise.card_type, CardType::Skill);
    assert_eq!(expertise.rarity, CardRarity::Uncommon);
    assert_eq!(expertise.cost, 1);
    assert_eq!(expertise.base_magic, 6);
    assert_eq!(expertise.upgrade_magic, 1);
    assert_eq!(java_id(CardId::Expertise), "Expertise");

    let escape_plan = get_card_definition(CardId::EscapePlan);
    assert_eq!(escape_plan.name, "Escape Plan");
    assert_eq!(escape_plan.card_type, CardType::Skill);
    assert_eq!(escape_plan.rarity, CardRarity::Uncommon);
    assert_eq!(escape_plan.cost, 0);
    assert_eq!(escape_plan.base_block, 3);
    assert_eq!(escape_plan.upgrade_block, 2);
    assert_eq!(java_id(CardId::EscapePlan), "Escape Plan");

    let eviscerate = get_card_definition(CardId::Eviscerate);
    assert_eq!(eviscerate.name, "Eviscerate");
    assert_eq!(eviscerate.card_type, CardType::Attack);
    assert_eq!(eviscerate.rarity, CardRarity::Uncommon);
    assert_eq!(eviscerate.cost, 3);
    assert_eq!(eviscerate.base_damage, 7);
    assert_eq!(eviscerate.upgrade_damage, 2);
    assert_eq!(java_id(CardId::Eviscerate), "Eviscerate");

    let predator = get_card_definition(CardId::Predator);
    assert_eq!(predator.name, "Predator");
    assert_eq!(predator.card_type, CardType::Attack);
    assert_eq!(predator.rarity, CardRarity::Uncommon);
    assert_eq!(predator.cost, 2);
    assert_eq!(predator.base_damage, 15);
    assert_eq!(predator.upgrade_damage, 5);
    assert_eq!(java_id(CardId::Predator), "Predator");

    let accuracy = get_card_definition(CardId::Accuracy);
    assert_eq!(accuracy.name, "Accuracy");
    assert_eq!(accuracy.card_type, CardType::Power);
    assert_eq!(accuracy.rarity, CardRarity::Uncommon);
    assert_eq!(accuracy.cost, 1);
    assert_eq!(accuracy.base_magic, 4);
    assert_eq!(accuracy.upgrade_magic, 2);
    assert_eq!(java_id(CardId::Accuracy), "Accuracy");

    let caltrops = get_card_definition(CardId::Caltrops);
    assert_eq!(caltrops.name, "Caltrops");
    assert_eq!(caltrops.card_type, CardType::Power);
    assert_eq!(caltrops.rarity, CardRarity::Uncommon);
    assert_eq!(caltrops.cost, 1);
    assert_eq!(caltrops.base_magic, 3);
    assert_eq!(caltrops.upgrade_magic, 2);
    assert_eq!(java_id(CardId::Caltrops), "Caltrops");

    let infinite = get_card_definition(CardId::InfiniteBlades);
    assert_eq!(infinite.name, "Infinite Blades");
    assert_eq!(infinite.card_type, CardType::Power);
    assert_eq!(infinite.rarity, CardRarity::Uncommon);
    assert_eq!(infinite.cost, 1);
    assert!(!infinite.innate);
    let mut infinite_plus = CombatCard::new(CardId::InfiniteBlades, 927);
    infinite_plus.upgrades = 1;
    assert!(is_innate_card(&infinite_plus));
    assert_eq!(java_id(CardId::InfiniteBlades), "Infinite Blades");

    let masterful = get_card_definition(CardId::MasterfulStab);
    assert_eq!(masterful.name, "Masterful Stab");
    assert_eq!(masterful.card_type, CardType::Attack);
    assert_eq!(masterful.rarity, CardRarity::Uncommon);
    assert_eq!(masterful.cost, 0);
    assert_eq!(masterful.base_damage, 12);
    assert_eq!(masterful.upgrade_damage, 4);
    assert_eq!(java_id(CardId::MasterfulStab), "Masterful Stab");

    assert_eq!(build_java_id_map().get("Backstab"), Some(&CardId::Backstab));
    assert_eq!(
        build_java_id_map().get("Riddle With Holes"),
        Some(&CardId::RiddleWithHoles)
    );
    assert_eq!(
        build_java_id_map().get("Die Die Die"),
        Some(&CardId::DieDieDie)
    );
    assert_eq!(build_java_id_map().get("Finisher"), Some(&CardId::Finisher));
    assert_eq!(
        build_java_id_map().get("Flechettes"),
        Some(&CardId::Flechettes)
    );
    assert_eq!(
        build_java_id_map().get("Heel Hook"),
        Some(&CardId::HeelHook)
    );
    assert_eq!(
        build_java_id_map().get("Expertise"),
        Some(&CardId::Expertise)
    );
    assert_eq!(
        build_java_id_map().get("Escape Plan"),
        Some(&CardId::EscapePlan)
    );
    assert_eq!(
        build_java_id_map().get("Eviscerate"),
        Some(&CardId::Eviscerate)
    );
    assert_eq!(build_java_id_map().get("Predator"), Some(&CardId::Predator));
    assert_eq!(build_java_id_map().get("Accuracy"), Some(&CardId::Accuracy));
    assert_eq!(build_java_id_map().get("Caltrops"), Some(&CardId::Caltrops));
    assert_eq!(
        build_java_id_map().get("Infinite Blades"),
        Some(&CardId::InfiniteBlades)
    );
    assert_eq!(
        build_java_id_map().get("Masterful Stab"),
        Some(&CardId::MasterfulStab)
    );
}

#[test]
fn silent_reward_pools_preserve_java_registration_order_for_implemented_cards() {
    assert_eq!(
        SILENT_COMMON_POOL,
        &[
            CardId::Acrobatics,
            CardId::Backflip,
            CardId::Bane,
            CardId::BladeDance,
            CardId::CloakAndDagger,
            CardId::DaggerSpray,
            CardId::DaggerThrow,
            CardId::DeadlyPoison,
            CardId::Deflect,
            CardId::DodgeAndRoll,
            CardId::FlyingKnee,
            CardId::Outmaneuver,
            CardId::PiercingWail,
            CardId::PoisonedStab,
            CardId::Prepared,
            CardId::QuickSlash,
            CardId::Slice,
            CardId::SuckerPunch,
            CardId::SneakyStrike,
        ]
    );
    assert_eq!(
        SILENT_UNCOMMON_POOL,
        &[
            CardId::Accuracy,
            CardId::AllOutAttack,
            CardId::Backstab,
            CardId::Blur,
            CardId::BouncingFlask,
            CardId::CalculatedGamble,
            CardId::Caltrops,
            CardId::Catalyst,
            CardId::Choke,
            CardId::Concentrate,
            CardId::CripplingPoison,
            CardId::Dash,
            CardId::EndlessAgony,
            CardId::EscapePlan,
            CardId::Eviscerate,
            CardId::Expertise,
            CardId::Finisher,
            CardId::Flechettes,
            CardId::Footwork,
            CardId::HeelHook,
            CardId::InfiniteBlades,
            CardId::LegSweep,
            CardId::MasterfulStab,
            CardId::NoxiousFumes,
            CardId::Predator,
            CardId::Reflex,
            CardId::RiddleWithHoles,
            CardId::Skewer,
            CardId::Tactician,
            CardId::Terror,
        ]
    );
    assert_eq!(
        SILENT_RARE_POOL,
        &[
            CardId::Adrenaline,
            CardId::AfterImage,
            CardId::Burst,
            CardId::DieDieDie,
            CardId::Doppelganger,
            CardId::GlassKnife,
            CardId::GrandFinale,
            CardId::Malaise,
            CardId::PhantasmalKiller,
            CardId::StormOfSteel,
            CardId::Unload,
            CardId::WraithForm,
        ]
    );
}

#[test]
fn silent_direct_attack_batch_runtime_actions_match_java_use_methods() {
    fn assert_damage(action: &Action, target: usize, amount: i32) {
        match action {
            Action::Damage(info) => {
                assert_eq!(info.target, target);
                assert_eq!(info.output, amount);
                assert_eq!(info.damage_type, DamageType::Normal);
            }
            other => panic!("expected DamageAction, got {other:?}"),
        }
    }

    let mut state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 7;
    let mut second = crate::test_support::test_monster(EnemyId::JawWorm);
    second.id = 8;
    state.entities.monsters = vec![first, second];

    let backstab = resolve_card_play(
        CardId::Backstab,
        &state,
        &CombatCard::new(CardId::Backstab, 910),
        Some(7),
    );
    assert_damage(&backstab[0].action, 7, 11);

    let riddle = resolve_card_play(
        CardId::RiddleWithHoles,
        &state,
        &CombatCard::new(CardId::RiddleWithHoles, 911),
        Some(7),
    );
    assert_eq!(riddle.len(), 5);
    for action in riddle {
        assert_damage(&action.action, 7, 3);
    }

    let die = resolve_card_play(
        CardId::DieDieDie,
        &state,
        &CombatCard::new(CardId::DieDieDie, 912),
        None,
    );
    assert_eq!(
        die[0].action,
        Action::DamageAllEnemies {
            source: 0,
            damages: smallvec::smallvec![13, 13],
            damage_type: DamageType::Normal,
            is_modified: true,
        }
    );
}

#[test]
fn finisher_reads_attack_count_when_queued_action_executes() {
    let state = crate::test_support::blank_test_combat();
    let finisher = resolve_card_play(
        CardId::Finisher,
        &state,
        &CombatCard::new(CardId::Finisher, 913),
        Some(7),
    );
    let Action::DamagePerAttackPlayed(info) = finisher[0].action.clone() else {
        panic!("Finisher should emit Java DamagePerAttackPlayedAction");
    };
    assert_eq!(info.target, 7);
    assert_eq!(info.output, 6);

    let mut runtime_state = crate::test_support::blank_test_combat();
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = 7;
    monster.current_hp = 50;
    runtime_state.entities.monsters = vec![monster];
    for _ in 0..4 {
        runtime_state.turn.increment_attacks_played();
    }
    crate::engine::action_handlers::execute_action(
        Action::DamagePerAttackPlayed(info.clone()),
        &mut runtime_state,
    );
    let queued = [
        runtime_state.pop_next_action(),
        runtime_state.pop_next_action(),
        runtime_state.pop_next_action(),
        runtime_state.pop_next_action(),
    ];
    for action in &queued[..3] {
        match action {
            Some(Action::Damage(damage)) => {
                assert_eq!(damage.target, 7);
                assert_eq!(damage.output, 6);
            }
            other => panic!("Finisher should queue ordinary DamageAction, got {other:?}"),
        }
    }
    assert_eq!(queued[3], None);

    let mut dead_target_state = crate::test_support::blank_test_combat();
    let mut dead_monster = crate::test_support::test_monster(EnemyId::JawWorm);
    dead_monster.id = 7;
    dead_monster.current_hp = 0;
    dead_target_state.entities.monsters = vec![dead_monster];
    dead_target_state.turn.increment_attacks_played();
    dead_target_state.turn.increment_attacks_played();
    crate::engine::action_handlers::execute_action(
        Action::DamagePerAttackPlayed(info),
        &mut dead_target_state,
    );
    assert_eq!(
        dead_target_state.pop_next_action(),
        None,
        "Java DamagePerAttackPlayedAction does nothing unless target.currentHealth > 0"
    );
}

#[test]
fn silent_execution_time_action_cards_match_java_actions() {
    let state = crate::test_support::blank_test_combat();

    let heel = resolve_card_play(
        CardId::HeelHook,
        &state,
        &CombatCard::new(CardId::HeelHook, 914),
        Some(7),
    );
    let Action::HeelHook(heel_info) = heel[0].action.clone() else {
        panic!("Heel Hook should emit Java HeelHookAction");
    };
    assert_eq!(heel_info.target, 7);
    assert_eq!(heel_info.output, 5);

    let mut weak_state = crate::test_support::blank_test_combat();
    let mut weak_monster = crate::test_support::test_monster(EnemyId::JawWorm);
    weak_monster.id = 7;
    weak_state.entities.monsters = vec![weak_monster];
    weak_state.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Weak,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::execute_action(
        Action::HeelHook(heel_info.clone()),
        &mut weak_state,
    );
    assert!(matches!(
        weak_state.pop_next_action(),
        Some(Action::Damage(_))
    ));
    assert_eq!(
        weak_state.pop_next_action(),
        Some(Action::GainEnergy { amount: 1 }),
        "Java HeelHookAction addToTop order makes damage execute before the energy follow-up"
    );
    assert_eq!(weak_state.pop_next_action(), Some(Action::DrawCards(1)));

    let mut no_weak_state = crate::test_support::blank_test_combat();
    no_weak_state.entities.monsters = weak_state.entities.monsters.clone();
    crate::engine::action_handlers::execute_action(Action::HeelHook(heel_info), &mut no_weak_state);
    assert!(matches!(
        no_weak_state.pop_next_action(),
        Some(Action::Damage(_))
    ));
    assert_eq!(no_weak_state.pop_next_action(), None);

    let flechettes = resolve_card_play(
        CardId::Flechettes,
        &state,
        &CombatCard::new(CardId::Flechettes, 915),
        Some(7),
    );
    let Action::Flechettes(flechettes_info) = flechettes[0].action.clone() else {
        panic!("Flechettes should emit Java FlechetteAction");
    };
    let mut flechettes_state = crate::test_support::blank_test_combat();
    flechettes_state.zones.hand = vec![
        CombatCard::new(CardId::DefendG, 916),
        CombatCard::new(CardId::Prepared, 917),
        CombatCard::new(CardId::StrikeG, 918),
    ];
    crate::engine::action_handlers::execute_action(
        Action::Flechettes(flechettes_info),
        &mut flechettes_state,
    );
    assert!(matches!(
        flechettes_state.pop_next_action(),
        Some(Action::Damage(_))
    ));
    assert!(matches!(
        flechettes_state.pop_next_action(),
        Some(Action::Damage(_))
    ));
    assert_eq!(flechettes_state.pop_next_action(), None);

    let expertise = resolve_card_play(
        CardId::Expertise,
        &state,
        &CombatCard::new(CardId::Expertise, 919),
        None,
    );
    assert_eq!(
        expertise[0].action,
        Action::ExpertiseDraw {
            target_hand_size: 6,
        }
    );
    let mut expertise_state = crate::test_support::blank_test_combat();
    expertise_state.zones.hand = vec![
        CombatCard::new(CardId::StrikeG, 920),
        CombatCard::new(CardId::DefendG, 921),
    ];
    crate::engine::action_handlers::execute_action(
        Action::ExpertiseDraw {
            target_hand_size: 6,
        },
        &mut expertise_state,
    );
    assert_eq!(
        expertise_state.pop_next_action(),
        Some(Action::DrawCards(4))
    );

    let escape = resolve_card_play(
        CardId::EscapePlan,
        &state,
        &CombatCard::new(CardId::EscapePlan, 923),
        None,
    );
    assert_eq!(
        escape[0].action,
        Action::DrawCardsWithHistory {
            amount: 1,
            clear_history: true,
        }
    );
    assert_eq!(
        escape[1].action,
        Action::EscapePlanBlockIfSkill { block: 3 }
    );
    let mut escape_state = crate::test_support::blank_test_combat();
    escape_state.zones.draw_pile = vec![CombatCard::new(CardId::Prepared, 924)];
    escape_state.runtime.last_drawn_cards = vec![CardId::StrikeG];
    crate::engine::action_handlers::execute_action(escape[0].action.clone(), &mut escape_state);
    assert_eq!(
        escape_state.runtime.last_drawn_cards,
        vec![CardId::Prepared]
    );
    crate::engine::action_handlers::execute_action(escape[1].action.clone(), &mut escape_state);
    assert_eq!(
        escape_state.pop_next_action(),
        Some(Action::GainBlock {
            target: 0,
            amount: 3,
        })
    );

    let mut split_escape_state = crate::test_support::blank_test_combat();
    split_escape_state.zones.draw_pile.clear();
    split_escape_state.zones.discard_pile = vec![CombatCard::new(CardId::Prepared, 925)];
    split_escape_state.runtime.last_drawn_cards = vec![CardId::StrikeG];
    crate::engine::action_handlers::execute_action(
        escape[0].action.clone(),
        &mut split_escape_state,
    );
    assert_eq!(
        split_escape_state.runtime.last_drawn_cards,
        Vec::<CardId>::new()
    );
    assert_eq!(
        split_escape_state.pop_next_action(),
        Some(Action::EmptyDeckShuffle)
    );
    assert_eq!(
        split_escape_state.pop_next_action(),
        Some(Action::DrawCardsWithHistory {
            amount: 1,
            clear_history: false,
        })
    );

    let predator = resolve_card_play(
        CardId::Predator,
        &state,
        &CombatCard::new(CardId::Predator, 926),
        Some(7),
    );
    match &predator[0].action {
        Action::Damage(info) => {
            assert_eq!(info.target, 7);
            assert_eq!(info.output, 15);
        }
        other => panic!("Predator first action should damage, got {other:?}"),
    }
    assert_eq!(
        predator[1].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::DrawCardNextTurn,
            amount: 2,
        }
    );

    let post_draw_actions =
        crate::content::powers::resolve_power_on_post_draw(PowerId::DrawCardNextTurn, &state, 0, 2);
    assert_eq!(
        post_draw_actions.as_slice(),
        &[
            Action::DrawCards(2),
            Action::RemovePower {
                target: 0,
                power_id: PowerId::DrawCardNextTurn,
            },
        ]
    );
}

#[test]
fn silent_power_cards_match_java_power_hooks() {
    let state = crate::test_support::blank_test_combat();

    let accuracy = resolve_card_play(
        CardId::Accuracy,
        &state,
        &CombatCard::new(CardId::Accuracy, 928),
        None,
    );
    assert_eq!(
        accuracy[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::AccuracyPower,
            amount: 4,
        }
    );

    let mut accuracy_state = crate::test_support::blank_test_combat();
    let mut target = crate::test_support::test_monster(EnemyId::JawWorm);
    target.id = 7;
    accuracy_state.entities.monsters = vec![target];
    accuracy_state.entities.power_db.insert(
        0,
        vec![Power {
            power_type: PowerId::AccuracyPower,
            instance_id: None,
            amount: 4,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let shiv_damage = evaluate_card_for_play(
        &CombatCard::new(CardId::Shiv, 929),
        &accuracy_state,
        Some(7),
    );
    assert_eq!(shiv_damage.base_damage_mut, 8);

    let caltrops = resolve_card_play(
        CardId::Caltrops,
        &state,
        &CombatCard::new(CardId::Caltrops, 930),
        None,
    );
    assert_eq!(
        caltrops[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Thorns,
            amount: 3,
        }
    );

    let infinite = resolve_card_play(
        CardId::InfiniteBlades,
        &state,
        &CombatCard::new(CardId::InfiniteBlades, 931),
        None,
    );
    assert_eq!(
        infinite[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::InfiniteBladesPower,
            amount: 1,
        }
    );
    let mut live_state = crate::test_support::blank_test_combat();
    let mut live_monster = crate::test_support::test_monster(EnemyId::JawWorm);
    live_monster.id = 7;
    live_state.entities.monsters = vec![live_monster];
    let start_actions = crate::content::powers::resolve_power_at_turn_start(
        PowerId::InfiniteBladesPower,
        &live_state,
        0,
        1,
    );
    assert_eq!(
        start_actions.as_slice(),
        &[Action::MakeTempCardInHand {
            card_id: CardId::Shiv,
            amount: 1,
            upgraded: false,
        }]
    );
}

#[test]
fn silent_dynamic_cost_cards_match_java_draw_discard_and_damage_hooks() {
    let mut state = crate::test_support::blank_test_combat();
    let mut target = crate::test_support::test_monster(EnemyId::JawWorm);
    target.id = 7;
    state.entities.monsters = vec![target];

    let eviscerate = resolve_card_play(
        CardId::Eviscerate,
        &state,
        &CombatCard::new(CardId::Eviscerate, 932),
        Some(7),
    );
    assert_eq!(eviscerate.len(), 3);
    for action in eviscerate {
        match action.action {
            Action::Damage(info) => {
                assert_eq!(info.target, 7);
                assert_eq!(info.output, 7);
            }
            other => panic!("Eviscerate should queue three DamageActions, got {other:?}"),
        }
    }

    let masterful = resolve_card_play(
        CardId::MasterfulStab,
        &state,
        &CombatCard::new(CardId::MasterfulStab, 933),
        Some(7),
    );
    match &masterful[0].action {
        Action::Damage(info) => {
            assert_eq!(info.target, 7);
            assert_eq!(info.output, 12);
        }
        other => panic!("Masterful Stab should queue DamageAction, got {other:?}"),
    }

    let mut discard_state = crate::test_support::blank_test_combat();
    discard_state.zones.hand = vec![
        CombatCard::new(CardId::StrikeG, 934),
        CombatCard::new(CardId::Eviscerate, 935),
    ];
    discard_state.zones.draw_pile = vec![CombatCard::new(CardId::Eviscerate, 936)];
    discard_state.zones.discard_pile = vec![CombatCard::new(CardId::Eviscerate, 937)];
    crate::engine::action_handlers::cards::handle_discard_card(934, &mut discard_state);
    assert_eq!(discard_state.turn.counters.cards_discarded_this_turn, 1);
    assert_eq!(discard_state.zones.hand[0].cost_for_turn_java(), 2);
    assert_eq!(discard_state.zones.draw_pile[0].cost_for_turn_java(), 2);
    assert_eq!(discard_state.zones.discard_pile[0].cost_for_turn_java(), 2);

    let mut end_turn_discard_state = crate::test_support::blank_test_combat();
    end_turn_discard_state.zones.hand = vec![CombatCard::new(CardId::StrikeG, 938)];
    end_turn_discard_state.zones.draw_pile = vec![CombatCard::new(CardId::Eviscerate, 939)];
    crate::engine::action_handlers::cards::handle_discard_from_hand(
        1,
        false,
        true,
        &mut end_turn_discard_state,
    );
    assert_eq!(
        end_turn_discard_state.zones.draw_pile[0].cost_for_turn_java(),
        3,
        "Java incrementDiscard(endTurn=true) increments count but does not call updateCardsOnDiscard"
    );

    let mut draw_state = crate::test_support::blank_test_combat();
    draw_state.turn.increment_cards_discarded();
    draw_state.turn.increment_cards_discarded();
    draw_state.zones.draw_pile = vec![CombatCard::new(CardId::Eviscerate, 940)];
    crate::engine::action_handlers::execute_action(Action::DrawCards(1), &mut draw_state);
    assert_eq!(draw_state.zones.hand[0].cost_for_turn_java(), 1);

    crate::content::cards::hooks::at_turn_start_in_hand(&mut draw_state);
    assert_eq!(
        draw_state.zones.hand[0].cost_for_turn_java(),
        3,
        "Java Eviscerate.atTurnStart resetAttributes clears temporary discard reductions"
    );

    let mut damage_state = crate::test_support::blank_test_combat();
    let mut attacker = crate::test_support::test_monster(EnemyId::JawWorm);
    attacker.id = 7;
    damage_state.entities.monsters = vec![attacker];
    damage_state.entities.player.current_hp = 50;
    damage_state.zones.hand = vec![CombatCard::new(CardId::MasterfulStab, 941)];
    damage_state.zones.draw_pile = vec![CombatCard::new(CardId::MasterfulStab, 942)];
    damage_state.zones.discard_pile = vec![CombatCard::new(CardId::MasterfulStab, 943)];
    crate::engine::action_handlers::execute_action(
        Action::Damage(DamageInfo {
            source: 7,
            target: 0,
            base: 1,
            output: 1,
            damage_type: DamageType::Normal,
            is_modified: false,
        }),
        &mut damage_state,
    );
    assert_eq!(
        damage_state.zones.hand[0].combat_cost_without_turn_override_java(),
        1
    );
    assert_eq!(
        damage_state.zones.draw_pile[0].combat_cost_without_turn_override_java(),
        1
    );
    assert_eq!(
        damage_state.zones.discard_pile[0].combat_cost_without_turn_override_java(),
        1
    );
}

#[test]
fn silent_target_control_cards_match_java_sources_and_power_hooks() {
    let blur = get_card_definition(CardId::Blur);
    assert_eq!(blur.name, "Blur");
    assert_eq!(blur.card_type, CardType::Skill);
    assert_eq!(blur.rarity, CardRarity::Uncommon);
    assert_eq!(blur.cost, 1);
    assert_eq!(blur.base_block, 5);
    assert_eq!(blur.upgrade_block, 3);
    assert_eq!(java_id(CardId::Blur), "Blur");

    let choke = get_card_definition(CardId::Choke);
    assert_eq!(choke.name, "Choke");
    assert_eq!(choke.card_type, CardType::Attack);
    assert_eq!(choke.rarity, CardRarity::Uncommon);
    assert_eq!(choke.cost, 2);
    assert_eq!(choke.base_damage, 12);
    assert_eq!(choke.base_magic, 3);
    assert_eq!(choke.upgrade_magic, 2);
    assert_eq!(java_id(CardId::Choke), "Choke");

    let crippling = get_card_definition(CardId::CripplingPoison);
    assert_eq!(crippling.name, "Crippling Poison");
    assert_eq!(crippling.card_type, CardType::Skill);
    assert_eq!(crippling.rarity, CardRarity::Uncommon);
    assert_eq!(crippling.cost, 2);
    assert_eq!(crippling.base_magic, 4);
    assert_eq!(crippling.upgrade_magic, 3);
    assert!(exhausts_when_played(&CombatCard::new(
        CardId::CripplingPoison,
        950
    )));
    assert_eq!(java_id(CardId::CripplingPoison), "Crippling Poison");

    let leg_sweep = get_card_definition(CardId::LegSweep);
    assert_eq!(leg_sweep.name, "Leg Sweep");
    assert_eq!(leg_sweep.card_type, CardType::Skill);
    assert_eq!(leg_sweep.rarity, CardRarity::Uncommon);
    assert_eq!(leg_sweep.cost, 2);
    assert_eq!(leg_sweep.base_block, 11);
    assert_eq!(leg_sweep.base_magic, 2);
    assert_eq!(leg_sweep.upgrade_block, 3);
    assert_eq!(leg_sweep.upgrade_magic, 1);
    assert_eq!(java_id(CardId::LegSweep), "Leg Sweep");

    let terror = get_card_definition(CardId::Terror);
    assert_eq!(terror.name, "Terror");
    assert_eq!(terror.card_type, CardType::Skill);
    assert_eq!(terror.rarity, CardRarity::Uncommon);
    assert_eq!(terror.cost, 1);
    assert!(exhausts_when_played(&CombatCard::new(CardId::Terror, 951)));
    let mut terror_plus = CombatCard::new(CardId::Terror, 952);
    terror_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&terror_plus), Some(0));
    assert_eq!(java_id(CardId::Terror), "Terror");

    assert_eq!(build_java_id_map().get("Blur"), Some(&CardId::Blur));
    assert_eq!(build_java_id_map().get("Choke"), Some(&CardId::Choke));
    assert_eq!(
        build_java_id_map().get("Crippling Poison"),
        Some(&CardId::CripplingPoison)
    );
    assert_eq!(
        build_java_id_map().get("Leg Sweep"),
        Some(&CardId::LegSweep)
    );
    assert_eq!(build_java_id_map().get("Terror"), Some(&CardId::Terror));

    let mut state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 7;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 8;
    state.entities.monsters = vec![first, second];

    let blur_actions = resolve_card_play(
        CardId::Blur,
        &state,
        &CombatCard::new(CardId::Blur, 953),
        None,
    );
    assert_eq!(blur_actions.len(), 2);
    assert!(matches!(
        blur_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 5
        }
    ));
    assert!(matches!(
        blur_actions[1].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Blur,
            amount: 1
        }
    ));

    let choke_actions = resolve_card_play(
        CardId::Choke,
        &state,
        &CombatCard::new(CardId::Choke, 954),
        Some(7),
    );
    assert_eq!(choke_actions.len(), 2);
    match &choke_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.target, 7);
            assert_eq!(info.output, 12);
        }
        other => panic!("Choke should first queue DamageAction, got {other:?}"),
    }
    assert!(matches!(
        choke_actions[1].action,
        Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Choked,
            amount: 3
        }
    ));

    let choked_actions = crate::content::powers::resolve_power_on_card_played(
        PowerId::Choked,
        &state,
        7,
        &CombatCard::new(CardId::DefendG, 955),
        3,
    );
    assert_eq!(
        choked_actions[0],
        Action::LoseHp {
            target: 7,
            amount: 3,
            triggers_rupture: false,
        }
    );
    assert_eq!(
        crate::content::powers::resolve_power_at_turn_start(PowerId::Choked, &state, 7, 3)[0],
        Action::RemovePower {
            target: 7,
            power_id: PowerId::Choked
        }
    );

    let mut leg_plus = CombatCard::new(CardId::LegSweep, 956);
    leg_plus.upgrades = 1;
    let leg_actions = resolve_card_play(CardId::LegSweep, &state, &leg_plus, Some(7));
    assert_eq!(leg_actions.len(), 2);
    assert!(matches!(
        leg_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Weak,
            amount: 3
        }
    ));
    assert!(matches!(
        leg_actions[1].action,
        Action::GainBlock {
            target: 0,
            amount: 14
        }
    ));

    let terror_actions = resolve_card_play(
        CardId::Terror,
        &state,
        &CombatCard::new(CardId::Terror, 957),
        Some(8),
    );
    assert_eq!(
        terror_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 8,
            power_id: PowerId::Vulnerable,
            amount: 99,
        }
    );

    let mut cripple_state = state.clone();
    cripple_state.entities.monsters[1].is_dying = true;
    let cripple_actions = resolve_card_play(
        CardId::CripplingPoison,
        &cripple_state,
        &CombatCard::new(CardId::CripplingPoison, 958),
        None,
    );
    assert_eq!(
        cripple_actions
            .iter()
            .map(|info| &info.action)
            .collect::<Vec<_>>(),
        vec![
            &Action::ApplyPower {
                source: 0,
                target: 7,
                power_id: PowerId::Poison,
                amount: 4,
            },
            &Action::ApplyPower {
                source: 0,
                target: 7,
                power_id: PowerId::Weak,
                amount: 2,
            },
        ]
    );

    assert_eq!(
        crate::content::powers::resolve_power_at_end_of_round(PowerId::Blur, &state, 0, 1, false)
            [0],
        Action::ReducePower {
            target: 0,
            power_id: PowerId::Blur,
            amount: 1,
        }
    );
}

#[test]
fn silent_special_attack_cards_match_java_draw_and_mutation_hooks() {
    let endless = get_card_definition(CardId::EndlessAgony);
    assert_eq!(endless.name, "Endless Agony");
    assert_eq!(endless.card_type, CardType::Attack);
    assert_eq!(endless.rarity, CardRarity::Uncommon);
    assert_eq!(endless.cost, 0);
    assert_eq!(endless.base_damage, 4);
    assert_eq!(endless.upgrade_damage, 2);
    assert!(exhausts_when_played(&CombatCard::new(
        CardId::EndlessAgony,
        959
    )));
    assert_eq!(java_id(CardId::EndlessAgony), "Endless Agony");

    let glass = get_card_definition(CardId::GlassKnife);
    assert_eq!(glass.name, "Glass Knife");
    assert_eq!(glass.card_type, CardType::Attack);
    assert_eq!(glass.rarity, CardRarity::Rare);
    assert_eq!(glass.cost, 1);
    assert_eq!(glass.base_damage, 8);
    assert_eq!(glass.upgrade_damage, 4);
    assert_eq!(java_id(CardId::GlassKnife), "Glass Knife");

    let finale = get_card_definition(CardId::GrandFinale);
    assert_eq!(finale.name, "Grand Finale");
    assert_eq!(finale.card_type, CardType::Attack);
    assert_eq!(finale.rarity, CardRarity::Rare);
    assert_eq!(finale.cost, 0);
    assert_eq!(finale.base_damage, 50);
    assert_eq!(finale.upgrade_damage, 10);
    assert!(finale.is_multi_damage);
    assert_eq!(java_id(CardId::GrandFinale), "Grand Finale");
    let skewer = get_card_definition(CardId::Skewer);
    assert_eq!(skewer.name, "Skewer");
    assert_eq!(skewer.card_type, CardType::Attack);
    assert_eq!(skewer.rarity, CardRarity::Uncommon);
    assert_eq!(skewer.cost, -1);
    assert_eq!(skewer.base_damage, 7);
    assert_eq!(skewer.upgrade_damage, 3);
    assert_eq!(skewer.target, CardTarget::Enemy);
    assert_eq!(java_id(CardId::Skewer), "Skewer");

    assert_eq!(
        build_java_id_map().get("Endless Agony"),
        Some(&CardId::EndlessAgony)
    );
    assert_eq!(
        build_java_id_map().get("Glass Knife"),
        Some(&CardId::GlassKnife)
    );
    assert_eq!(
        build_java_id_map().get("Grand Finale"),
        Some(&CardId::GrandFinale)
    );
    assert_eq!(build_java_id_map().get("Skewer"), Some(&CardId::Skewer));

    let mut state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 7;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 8;
    state.entities.monsters = vec![first, second];

    let endless_actions = resolve_card_play(
        CardId::EndlessAgony,
        &state,
        &CombatCard::new(CardId::EndlessAgony, 960),
        Some(7),
    );
    match &endless_actions[0].action {
        Action::Damage(info) => {
            assert_eq!(info.target, 7);
            assert_eq!(info.output, 4);
        }
        other => panic!("Endless Agony should queue DamageAction, got {other:?}"),
    }

    let glass_actions = resolve_card_play(
        CardId::GlassKnife,
        &state,
        &CombatCard::new(CardId::GlassKnife, 961),
        Some(7),
    );
    assert_eq!(glass_actions.len(), 3);
    for action in glass_actions.iter().take(2) {
        match &action.action {
            Action::Damage(info) => {
                assert_eq!(info.target, 7);
                assert_eq!(info.output, 8);
            }
            other => panic!("Glass Knife should queue two DamageActions, got {other:?}"),
        }
    }
    assert_eq!(
        glass_actions[2].action,
        Action::ModifyCardDamage {
            card_uuid: 961,
            amount: -2,
        }
    );

    let finale_actions = resolve_card_play(
        CardId::GrandFinale,
        &state,
        &CombatCard::new(CardId::GrandFinale, 962),
        None,
    );
    assert_eq!(
        finale_actions[0].action,
        Action::DamageAllEnemies {
            source: 0,
            damages: smallvec::smallvec![50, 50],
            damage_type: DamageType::Normal,
            is_modified: true,
        }
    );

    let mut finale_blocked = state.clone();
    finale_blocked.zones.draw_pile = vec![CombatCard::new(CardId::StrikeG, 963)];
    assert!(
        can_play_card(&CombatCard::new(CardId::GrandFinale, 964), &finale_blocked).is_err(),
        "Java Grand Finale.canUse requires an empty draw pile"
    );
    let mut finale_allowed = state.clone();
    finale_allowed.zones.draw_pile.clear();
    assert!(can_play_card(&CombatCard::new(CardId::GrandFinale, 965), &finale_allowed).is_ok());

    let mut draw_state = crate::test_support::blank_test_combat();
    let mut drawn = CombatCard::new(CardId::EndlessAgony, 966);
    drawn.upgrades = 1;
    draw_state.zones.draw_pile = vec![drawn];
    crate::engine::action_handlers::execute_action(Action::DrawCards(1), &mut draw_state);
    assert_eq!(draw_state.zones.hand.len(), 1);
    assert_eq!(draw_state.zones.hand[0].id, CardId::EndlessAgony);
    let trigger = draw_state
        .pop_next_action()
        .expect("Endless Agony.triggerWhenDrawn should queue MakeTempCardInHandAction");
    match trigger {
        Action::MakeCopyInHand { original, amount } => {
            assert_eq!(original.id, CardId::EndlessAgony);
            assert_eq!(original.upgrades, 1);
            assert_eq!(amount, 1);
            crate::engine::action_handlers::execute_action(
                Action::MakeCopyInHand { original, amount },
                &mut draw_state,
            );
        }
        other => panic!("Endless Agony draw hook should make a hand copy, got {other:?}"),
    }
    assert_eq!(draw_state.zones.hand.len(), 2);
    assert_eq!(draw_state.zones.hand[1].id, CardId::EndlessAgony);
    assert_eq!(draw_state.zones.hand[1].upgrades, 1);

    let mut knife_state = crate::test_support::blank_test_combat();
    knife_state.zones.limbo = vec![CombatCard::new(CardId::GlassKnife, 967)];
    crate::engine::action_handlers::execute_action(
        Action::ModifyCardDamage {
            card_uuid: 967,
            amount: -2,
        },
        &mut knife_state,
    );
    assert_eq!(knife_state.zones.limbo[0].base_damage_override, Some(6));
}

#[test]
fn skewer_matches_java_x_cost_single_target_action() {
    let mut state = crate::test_support::blank_test_combat();
    let mut target = crate::test_support::test_monster(EnemyId::JawWorm);
    target.id = 7;
    state.entities.monsters = vec![target];

    let mut skewer_card = CombatCard::new(CardId::Skewer, 970);
    skewer_card.energy_on_use = 2;
    let actions = resolve_card_play(CardId::Skewer, &state, &skewer_card, Some(7));
    assert_eq!(actions.len(), 1);
    match &actions[0].action {
        Action::Skewer {
            target,
            damage_info,
            free_to_play_once,
            energy_on_use,
        } => {
            assert_eq!(*target, 7);
            assert_eq!(damage_info.target, 7);
            assert_eq!(damage_info.output, 7);
            assert!(!free_to_play_once);
            assert_eq!(*energy_on_use, 2);
        }
        other => panic!("Skewer should emit SkewerAction equivalent, got {other:?}"),
    }

    let mut energy_state = crate::test_support::blank_test_combat();
    energy_state.turn.energy = 5;
    crate::engine::action_handlers::execute_action(
        Action::Skewer {
            target: 7,
            damage_info: DamageInfo {
                source: 0,
                target: 7,
                base: 7,
                output: 7,
                damage_type: DamageType::Normal,
                is_modified: true,
            },
            free_to_play_once: false,
            energy_on_use: 2,
        },
        &mut energy_state,
    );
    assert_eq!(
        energy_state.turn.energy, 0,
        "Java SkewerAction spends EnergyPanel.totalCount, not energyOnUse"
    );
    for _ in 0..2 {
        match energy_state.pop_next_action() {
            Some(Action::Damage(info)) => {
                assert_eq!(info.target, 7);
                assert_eq!(info.output, 7);
            }
            other => panic!("SkewerAction should queue fixed target DamageAction, got {other:?}"),
        }
    }
    assert!(energy_state.pop_next_action().is_none());

    let mut chemical_x_state = crate::test_support::blank_test_combat();
    chemical_x_state.turn.energy = 0;
    chemical_x_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::ChemicalX,
        ));
    crate::engine::action_handlers::execute_action(
        Action::Skewer {
            target: 7,
            damage_info: DamageInfo {
                source: 0,
                target: 7,
                base: 7,
                output: 7,
                damage_type: DamageType::Normal,
                is_modified: true,
            },
            free_to_play_once: false,
            energy_on_use: -1,
        },
        &mut chemical_x_state,
    );
    assert_eq!(
        chemical_x_state.action_queue_len(),
        2,
        "Chemical X adds two Skewer hits even when current energy is zero"
    );
}

#[test]
fn silent_x_cost_power_cards_match_java_actions() {
    let doppelganger = get_card_definition(CardId::Doppelganger);
    assert_eq!(doppelganger.name, "Doppelganger");
    assert_eq!(doppelganger.card_type, CardType::Skill);
    assert_eq!(doppelganger.rarity, CardRarity::Rare);
    assert_eq!(doppelganger.cost, -1);
    assert_eq!(doppelganger.target, CardTarget::SelfTarget);
    assert!(doppelganger.exhaust);
    assert!(exhausts_when_played(&CombatCard::new(
        CardId::Doppelganger,
        980
    )));
    assert_eq!(java_id(CardId::Doppelganger), "Doppelganger");

    let malaise = get_card_definition(CardId::Malaise);
    assert_eq!(malaise.name, "Malaise");
    assert_eq!(malaise.card_type, CardType::Skill);
    assert_eq!(malaise.rarity, CardRarity::Rare);
    assert_eq!(malaise.cost, -1);
    assert_eq!(malaise.target, CardTarget::Enemy);
    assert!(malaise.exhaust);
    assert!(exhausts_when_played(&CombatCard::new(CardId::Malaise, 981)));
    assert_eq!(java_id(CardId::Malaise), "Malaise");

    assert_eq!(
        build_java_id_map().get("Doppelganger"),
        Some(&CardId::Doppelganger)
    );
    assert_eq!(build_java_id_map().get("Malaise"), Some(&CardId::Malaise));

    let state = crate::test_support::blank_test_combat();
    let mut doppelganger_card = CombatCard::new(CardId::Doppelganger, 982);
    doppelganger_card.upgrades = 1;
    doppelganger_card.energy_on_use = 2;
    let doppelganger_actions =
        resolve_card_play(CardId::Doppelganger, &state, &doppelganger_card, None);
    assert_eq!(doppelganger_actions.len(), 1);
    assert_eq!(
        doppelganger_actions[0].action,
        Action::Doppelganger {
            upgraded: true,
            free_to_play_once: false,
            energy_on_use: 2,
        }
    );

    let mut malaise_card = CombatCard::new(CardId::Malaise, 983);
    malaise_card.upgrades = 1;
    malaise_card.free_to_play_once = true;
    malaise_card.energy_on_use = 2;
    let malaise_actions = resolve_card_play(CardId::Malaise, &state, &malaise_card, Some(7));
    assert_eq!(malaise_actions.len(), 1);
    assert_eq!(
        malaise_actions[0].action,
        Action::Malaise {
            target: 7,
            upgraded: true,
            free_to_play_once: true,
            energy_on_use: 2,
        }
    );

    let mut doppelganger_state = crate::test_support::blank_test_combat();
    doppelganger_state.turn.energy = 5;
    crate::engine::action_handlers::execute_action(
        Action::Doppelganger {
            upgraded: true,
            free_to_play_once: false,
            energy_on_use: 2,
        },
        &mut doppelganger_state,
    );
    assert_eq!(
        doppelganger_state.turn.energy, 0,
        "Java DoppelgangerAction spends current EnergyPanel.totalCount"
    );
    assert_eq!(
        doppelganger_state.pop_next_action(),
        Some(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Energized,
            amount: 3,
        })
    );
    assert_eq!(
        doppelganger_state.pop_next_action(),
        Some(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::DrawCardNextTurn,
            amount: 3,
        })
    );
    assert!(doppelganger_state.pop_next_action().is_none());

    let mut malaise_state = crate::test_support::blank_test_combat();
    malaise_state.turn.energy = 5;
    crate::engine::action_handlers::execute_action(
        Action::Malaise {
            target: 7,
            upgraded: false,
            free_to_play_once: false,
            energy_on_use: 2,
        },
        &mut malaise_state,
    );
    assert_eq!(
        malaise_state.turn.energy, 0,
        "Java MalaiseAction spends current EnergyPanel.totalCount"
    );
    assert_eq!(
        malaise_state.pop_next_action(),
        Some(Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Strength,
            amount: -2,
        })
    );
    assert_eq!(
        malaise_state.pop_next_action(),
        Some(Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Weak,
            amount: 2,
        })
    );
    assert!(malaise_state.pop_next_action().is_none());

    let mut chemical_x_state = crate::test_support::blank_test_combat();
    chemical_x_state.turn.energy = 0;
    chemical_x_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::ChemicalX,
        ));
    crate::engine::action_handlers::execute_action(
        Action::Malaise {
            target: 7,
            upgraded: true,
            free_to_play_once: false,
            energy_on_use: -1,
        },
        &mut chemical_x_state,
    );
    assert_eq!(
        chemical_x_state.pop_next_action(),
        Some(Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Strength,
            amount: -3,
        })
    );
    assert_eq!(
        chemical_x_state.pop_next_action(),
        Some(Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Weak,
            amount: 3,
        })
    );
}

#[test]
fn wraith_form_matches_java_intangible_and_dexterity_loss_power() {
    let wraith = get_card_definition(CardId::WraithForm);
    assert_eq!(wraith.name, "Wraith Form");
    assert_eq!(wraith.card_type, CardType::Power);
    assert_eq!(wraith.rarity, CardRarity::Rare);
    assert_eq!(wraith.cost, 3);
    assert_eq!(wraith.base_magic, 2);
    assert_eq!(wraith.upgrade_magic, 1);
    assert_eq!(wraith.target, CardTarget::SelfTarget);
    assert_eq!(java_id(CardId::WraithForm), "Wraith Form v2");
    assert_eq!(
        build_java_id_map().get("Wraith Form v2"),
        Some(&CardId::WraithForm)
    );

    let state = crate::test_support::blank_test_combat();
    let mut wraith_plus = CombatCard::new(CardId::WraithForm, 990);
    wraith_plus.upgrades = 1;
    let actions = resolve_card_play(CardId::WraithForm, &state, &wraith_plus, None);
    assert_eq!(actions.len(), 2);
    assert_eq!(
        actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::IntangiblePlayer,
            amount: 3,
        }
    );
    assert_eq!(
        actions[1].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::WraithForm,
            amount: -1,
        }
    );

    assert!(
        crate::content::powers::allows_negative_amount(PowerId::WraithForm),
        "Java WraithFormPower stacks by adding negative amounts"
    );
    assert!(
        crate::content::powers::is_debuff_application(PowerId::WraithForm, -1),
        "Java WraithFormPower is PowerType.DEBUFF even though it is applied by a card"
    );

    let wraith_power = Power {
        power_type: PowerId::WraithForm,
        instance_id: None,
        amount: -1,
        extra_data: 0,
        just_applied: false,
    };
    let turn_end = crate::content::powers::resolve_power_at_end_of_turn(&wraith_power, &state, 0);
    assert_eq!(turn_end.len(), 1);
    assert_eq!(
        turn_end[0],
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Dexterity,
            amount: -1,
        }
    );
}

#[test]
fn phantasmal_killer_matches_java_delayed_double_damage_power() {
    let phantasmal = get_card_definition(CardId::PhantasmalKiller);
    assert_eq!(phantasmal.name, "Phantasmal Killer");
    assert_eq!(phantasmal.card_type, CardType::Skill);
    assert_eq!(phantasmal.rarity, CardRarity::Rare);
    assert_eq!(phantasmal.cost, 1);
    assert_eq!(phantasmal.target, CardTarget::SelfTarget);
    assert_eq!(java_id(CardId::PhantasmalKiller), "Phantasmal Killer");
    assert_eq!(
        build_java_id_map().get("Phantasmal Killer"),
        Some(&CardId::PhantasmalKiller)
    );
    let mut phantasmal_plus = CombatCard::new(CardId::PhantasmalKiller, 995);
    phantasmal_plus.upgrades = 1;
    assert_eq!(upgraded_base_cost_override(&phantasmal_plus), Some(0));

    let state = crate::test_support::blank_test_combat();
    let actions = resolve_card_play(CardId::PhantasmalKiller, &state, &phantasmal_plus, None);
    assert_eq!(
        actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Phantasmal,
            amount: 1,
        }
    );

    let turn_start =
        crate::content::powers::resolve_power_at_turn_start(PowerId::Phantasmal, &state, 0, 2);
    assert_eq!(
        turn_start[0],
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::DoubleDamage,
            amount: 1,
        }
    );
    assert_eq!(
        turn_start[1],
        Action::ReducePower {
            target: 0,
            power_id: PowerId::Phantasmal,
            amount: 1,
        }
    );

    let double_damage_round_end = crate::content::powers::resolve_power_at_end_of_round(
        PowerId::DoubleDamage,
        &state,
        0,
        1,
        false,
    );
    assert_eq!(
        double_damage_round_end[0],
        Action::ReducePower {
            target: 0,
            power_id: PowerId::DoubleDamage,
            amount: 1,
        }
    );

    let mut damage_state = crate::test_support::blank_test_combat();
    let mut target = crate::test_support::test_monster(EnemyId::JawWorm);
    target.id = 7;
    damage_state.entities.monsters = vec![target];
    damage_state.entities.power_db.insert(
        0,
        vec![Power {
            power_type: PowerId::DoubleDamage,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let strike_actions = resolve_card_play(
        CardId::StrikeG,
        &damage_state,
        &CombatCard::new(CardId::StrikeG, 996),
        Some(7),
    );
    match &strike_actions[0].action {
        Action::Damage(info) => assert_eq!(info.output, 12),
        other => panic!("DoubleDamagePower should affect normal attack damage, got {other:?}"),
    }
}

#[test]
fn piercing_wail_matches_java_artifact_and_shackled_rules() {
    let piercing = get_card_definition(CardId::PiercingWail);
    assert_eq!(piercing.name, "Piercing Wail");
    assert_eq!(piercing.card_type, CardType::Skill);
    assert_eq!(piercing.rarity, CardRarity::Common);
    assert_eq!(piercing.cost, 1);
    assert_eq!(piercing.base_magic, 6);
    assert_eq!(piercing.upgrade_magic, 2);
    assert!(piercing.exhaust);
    assert_eq!(java_id(CardId::PiercingWail), "PiercingWail");
    assert!(
        !crate::content::powers::is_debuff_application(PowerId::Shackled, 6),
        "Java ApplyPowerAction does not let Artifact block GainStrengthPower/Shackled"
    );

    let mut state = crate::test_support::blank_test_combat();
    let mut no_artifact = crate::test_support::test_monster(EnemyId::JawWorm);
    no_artifact.id = 7;
    let mut with_artifact = crate::test_support::test_monster(EnemyId::JawWorm);
    with_artifact.id = 8;
    state.entities.monsters = vec![no_artifact, with_artifact];
    state.entities.power_db.insert(
        8,
        vec![Power {
            power_type: PowerId::Artifact,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );

    let actions = resolve_card_play(
        CardId::PiercingWail,
        &state,
        &CombatCard::new(CardId::PiercingWail, 908),
        None,
    );
    assert_eq!(actions.len(), 3);
    assert_eq!(
        actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Strength,
            amount: -6,
        }
    );
    assert_eq!(
        actions[1].action,
        Action::ApplyPower {
            source: 0,
            target: 8,
            power_id: PowerId::Strength,
            amount: -6,
        }
    );
    assert_eq!(
        actions[2].action,
        Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Shackled,
            amount: 6,
        },
        "Java queues GainStrengthPower only for monsters that lack Artifact at use time"
    );

    let mut upgraded = CombatCard::new(CardId::PiercingWail, 909);
    upgraded.upgrades = 1;
    let upgraded_actions = resolve_card_play(CardId::PiercingWail, &state, &upgraded, None);
    assert_eq!(
        upgraded_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Strength,
            amount: -8,
        }
    );

    let mut artifact_state = crate::test_support::blank_test_combat();
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = 7;
    artifact_state.entities.monsters = vec![monster];
    artifact_state.entities.power_db.insert(
        7,
        vec![Power {
            power_type: PowerId::Artifact,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::execute_action(
        Action::ApplyPower {
            source: 0,
            target: 7,
            power_id: PowerId::Shackled,
            amount: 6,
        },
        &mut artifact_state,
    );
    assert_eq!(
        crate::content::powers::store::power_amount(&artifact_state, 7, PowerId::Artifact),
        1
    );
    assert_eq!(
        crate::content::powers::store::power_amount(&artifact_state, 7, PowerId::Shackled),
        6
    );
}

#[test]
fn reflex_and_tactician_manual_discard_hooks_match_java_order() {
    let reflex = get_card_definition(CardId::Reflex);
    assert_eq!(reflex.cost, -2);
    assert_eq!(reflex.card_type, CardType::Skill);
    assert_eq!(reflex.rarity, CardRarity::Uncommon);
    assert_eq!(reflex.base_magic, 2);
    assert_eq!(reflex.upgrade_magic, 1);

    let tactician = get_card_definition(CardId::Tactician);
    assert_eq!(tactician.cost, -2);
    assert_eq!(tactician.base_magic, 1);
    assert_eq!(tactician.upgrade_magic, 1);
    assert_eq!(java_id(CardId::Reflex), "Reflex");
    assert_eq!(java_id(CardId::Tactician), "Tactician");
    assert_eq!(build_java_id_map().get("Reflex"), Some(&CardId::Reflex));
    assert_eq!(
        build_java_id_map().get("Tactician"),
        Some(&CardId::Tactician)
    );

    let mut reflex_state = crate::test_support::blank_test_combat();
    reflex_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::Tingsha,
        ));
    reflex_state.zones.hand = vec![CombatCard::new(CardId::Reflex, 870)];
    crate::engine::action_handlers::cards::handle_discard_card(870, &mut reflex_state);

    assert_eq!(reflex_state.turn.counters.cards_discarded_this_turn, 1);
    assert_eq!(reflex_state.zones.discard_pile[0].id, CardId::Reflex);
    assert!(matches!(
        reflex_state.pop_next_action(),
        Some(Action::DamageRandomEnemy { .. }),
    ));
    assert_eq!(
        reflex_state.pop_next_action(),
        Some(Action::DrawCards(2)),
        "Java DiscardSpecificCardAction/GamblingChipAction increments discard before Reflex.triggerOnManualDiscard"
    );

    let mut tactician_state = crate::test_support::blank_test_combat();
    tactician_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::ToughBandages,
        ));
    let mut tactician_plus = CombatCard::new(CardId::Tactician, 871);
    tactician_plus.upgrades = 1;
    tactician_state.zones.hand = vec![tactician_plus];
    crate::engine::action_handlers::cards::handle_discard_card(871, &mut tactician_state);

    assert_eq!(tactician_state.turn.counters.cards_discarded_this_turn, 1);
    assert_eq!(
        tactician_state.pop_next_action(),
        Some(Action::GainEnergy { amount: 2 }),
        "Java Tactician.triggerOnManualDiscard uses addToTop, so it executes before relic addToBot actions queued by incrementDiscard"
    );
    assert_eq!(
        tactician_state.pop_next_action(),
        Some(Action::GainBlock {
            target: 0,
            amount: 3,
        })
    );
}

#[test]
fn discard_action_manual_discard_hooks_run_card_before_relic_hooks() {
    let mut reflex_state = crate::test_support::blank_test_combat();
    reflex_state.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
    reflex_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::Tingsha,
        ));
    reflex_state.zones.hand = vec![CombatCard::new(CardId::Reflex, 872)];

    crate::engine::action_handlers::cards::handle_discard_from_hand(
        1,
        false,
        false,
        &mut reflex_state,
    );

    assert_eq!(reflex_state.turn.counters.cards_discarded_this_turn, 1);
    assert_eq!(
        reflex_state.pop_next_action(),
        Some(Action::DrawCards(2)),
        "Java DiscardAction calls triggerOnManualDiscard before incrementDiscard"
    );
    assert!(matches!(
        reflex_state.pop_next_action(),
        Some(Action::DamageRandomEnemy { .. }),
    ));

    let mut chip_state = crate::test_support::blank_test_combat();
    chip_state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::ToughBandages,
        ));
    let mut tactician_plus = CombatCard::new(CardId::Tactician, 873);
    tactician_plus.upgrades = 1;
    chip_state.zones.hand = vec![tactician_plus];
    let mut engine_state = crate::state::EngineState::CombatProcessing;

    crate::engine::pending_choices::handle_hand_select(
        &mut engine_state,
        &mut chip_state,
        &[873],
        1,
        false,
        true,
        crate::state::HandSelectReason::GamblingChip,
        crate::state::ClientInput::SubmitHandSelect(vec![873]),
    )
    .expect("valid Gambling Chip hand selection");

    assert_eq!(chip_state.turn.counters.cards_discarded_this_turn, 1);
    assert_eq!(
        chip_state.pop_next_action(),
        Some(Action::GainEnergy { amount: 2 }),
        "Java GamblingChipAction queues DrawCardAction to top before discards, so later Tactician addToTop executes first"
    );
    assert_eq!(chip_state.pop_next_action(), Some(Action::DrawCards(1)));
    assert_eq!(
        chip_state.pop_next_action(),
        Some(Action::GainBlock {
            target: 0,
            amount: 3,
        })
    );
}

#[test]
fn discard_action_random_end_turn_path_keeps_card_hook_without_relic_hook() {
    let mut state = crate::test_support::blank_test_combat();
    state.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
    state
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::Tingsha,
        ));
    state.zones.hand = vec![
        CombatCard::new(CardId::Reflex, 874),
        CombatCard::new(CardId::Reflex, 875),
    ];

    crate::engine::action_handlers::cards::handle_discard_from_hand(1, true, true, &mut state);

    assert_eq!(state.turn.counters.cards_discarded_this_turn, 1);
    assert_eq!(state.zones.discard_pile.len(), 1);
    assert_eq!(
        state.pop_next_action(),
        Some(Action::DrawCards(2)),
        "Java DiscardAction random branch calls triggerOnManualDiscard even when endTurn=true"
    );
    assert_eq!(
        state.pop_next_action(),
        None,
        "GameActionManager.incrementDiscard(endTurn=true) suppresses relic onManualDiscard"
    );
}

#[test]
fn exhaust_from_hand_matches_java_auto_and_any_number_paths() {
    let mut auto_state = crate::test_support::blank_test_combat();
    auto_state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 860),
        CombatCard::new(CardId::Defend, 861),
    ];

    crate::engine::action_handlers::cards::handle_exhaust_from_hand(
        2,
        false,
        false,
        false,
        &mut auto_state,
    );

    assert!(auto_state.zones.hand.is_empty());
    assert_eq!(
        auto_state
            .zones
            .exhaust_pile
            .iter()
            .map(|card| card.uuid)
            .collect::<Vec<_>>(),
        vec![861, 860],
        "Java ExhaustAction auto path repeatedly moves hand.getTopCard when hand.size() <= amount"
    );
    assert_eq!(auto_state.pop_next_action(), None);

    let mut any_number_state = crate::test_support::blank_test_combat();
    any_number_state.zones.hand = vec![CombatCard::new(CardId::Strike, 862)];
    crate::engine::action_handlers::cards::handle_exhaust_from_hand(
        3,
        false,
        true,
        true,
        &mut any_number_state,
    );

    assert!(matches!(
        any_number_state.pop_next_action(),
        Some(Action::SuspendForHandSelect {
            min: 0,
            max: 3,
            can_cancel: true,
            filter: crate::state::HandSelectFilter::Any,
            reason: crate::state::HandSelectReason::Exhaust,
        })
    ));
}

#[test]
fn upgraded_blind_and_trip_enqueue_apply_power_for_every_monster_like_java() {
    let mut state = crate::test_support::blank_test_combat();
    let mut zero_hp_not_dying = crate::test_support::test_monster(EnemyId::JawWorm);
    zero_hp_not_dying.id = 810;
    zero_hp_not_dying.current_hp = 0;
    zero_hp_not_dying.is_dying = false;
    let mut half_dead = crate::test_support::test_monster(EnemyId::Darkling);
    half_dead.id = 811;
    half_dead.half_dead = true;
    state.entities.monsters = vec![zero_hp_not_dying, half_dead];

    let mut blind_plus = CombatCard::new(CardId::Blind, 812);
    blind_plus.upgrades = 1;
    let blind_actions = resolve_card_play(CardId::Blind, &state, &blind_plus, None);
    assert_eq!(
        blind_actions.len(),
        2,
        "Java Blind+ loops over monsters.monsters and lets ApplyPowerAction handle dead/escaped filtering"
    );
    assert!(matches!(
        blind_actions[0].action,
        Action::ApplyPower {
            target: 810,
            power_id: PowerId::Weak,
            ..
        }
    ));
    assert!(matches!(
        blind_actions[1].action,
        Action::ApplyPower {
            target: 811,
            power_id: PowerId::Weak,
            ..
        }
    ));

    let mut trip_plus = CombatCard::new(CardId::Trip, 813);
    trip_plus.upgrades = 1;
    let trip_actions = resolve_card_play(CardId::Trip, &state, &trip_plus, None);
    assert_eq!(
        trip_actions.len(),
        2,
        "Java Trip+ loops over monsters.monsters and lets ApplyPowerAction handle dead/escaped filtering"
    );
    assert!(matches!(
        trip_actions[0].action,
        Action::ApplyPower {
            target: 810,
            power_id: PowerId::Vulnerable,
            ..
        }
    ));
    assert!(matches!(
        trip_actions[1].action,
        Action::ApplyPower {
            target: 811,
            power_id: PowerId::Vulnerable,
            ..
        }
    ));
}

#[test]
fn put_on_deck_action_matches_java_rng_and_selection_edges() {
    let mut one_card_state = crate::test_support::blank_test_combat();
    one_card_state.zones.hand = vec![CombatCard::new(CardId::Strike, 400)];
    one_card_state.zones.draw_pile = vec![CombatCard::new(CardId::Defend, 401)];
    assert_eq!(one_card_state.rng.card_random_rng.counter, 0);

    crate::engine::action_handlers::cards::handle_put_on_deck(1, false, &mut one_card_state);

    assert!(one_card_state.zones.hand.is_empty());
    assert_eq!(one_card_state.zones.draw_pile[0].uuid, 400);
    assert_eq!(one_card_state.zones.draw_pile[1].uuid, 401);
    assert_eq!(
        one_card_state.rng.card_random_rng.counter, 1,
        "Java PutOnDeckAction uses getRandomCard(cardRandomRng) when hand size <= amount"
    );

    let mut two_card_state = crate::test_support::blank_test_combat();
    two_card_state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 402),
        CombatCard::new(CardId::Defend, 403),
    ];
    crate::engine::action_handlers::cards::handle_put_on_deck(1, false, &mut two_card_state);
    assert_eq!(two_card_state.zones.hand.len(), 2);
    assert_eq!(two_card_state.rng.card_random_rng.counter, 0);
    assert!(matches!(
        two_card_state.pop_next_action(),
        Some(Action::SuspendForHandSelect {
            min: 1,
            max: 1,
            can_cancel: false,
            filter: crate::state::HandSelectFilter::Any,
            reason: crate::state::HandSelectReason::PutOnDrawPile,
        })
    ));

    let mut fallback_state = crate::test_support::blank_test_combat();
    fallback_state.zones.hand = vec![
        CombatCard::new(CardId::Strike, 404),
        CombatCard::new(CardId::Defend, 405),
    ];
    crate::engine::action_handlers::cards::handle_put_on_deck(2, false, &mut fallback_state);
    assert_eq!(
        fallback_state.zones.hand.len(),
        1,
        "Java PutOnDeckAction fallback loop checks the shrinking hand size each iteration"
    );
    assert_eq!(fallback_state.zones.draw_pile.len(), 1);
    assert_eq!(fallback_state.rng.card_random_rng.counter, 1);
}

#[test]
fn apotheosis_uses_combat_wide_upgrade_action_not_armaments_hand_action() {
    let state = crate::test_support::blank_test_combat();
    let actions = resolve_card_play(
        CardId::Apotheosis,
        &state,
        &CombatCard::new(CardId::Apotheosis, 900),
        None,
    );

    assert_eq!(actions.len(), 1);
    assert_eq!(
        actions[0].action,
        Action::UpgradeAllCardsInCombat,
        "Java ApotheosisAction upgrades hand, draw, discard, and exhaust; Blessing of the Forge is the ArmamentsAction(true) hand-only path"
    );
}
