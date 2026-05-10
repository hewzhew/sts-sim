use super::*;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageType, NO_SOURCE};
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
    match &burning_pact_actions[0].action {
        Action::ExhaustCard {
            card_uuid,
            source_pile,
        } => {
            assert_eq!(*card_uuid, 84);
            assert_eq!(*source_pile, crate::state::PileType::Hand);
        }
        other => panic!("Burning Pact should exhaust one hand card first, got {other:?}"),
    }
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
    match &burning_pact_select_actions[0].action {
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
            assert_eq!(*filter, crate::state::HandSelectFilter::Any);
            assert_eq!(*reason, crate::state::HandSelectReason::Exhaust);
        }
        other => panic!("Burning Pact should open ExhaustAction hand select, got {other:?}"),
    }
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
    assert!(resolve_card_play(CardId::Clash, &state, &state.zones.hand[0], Some(7)).is_empty());

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

    state
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::JawWorm));
    let dark_embrace_actions = crate::content::powers::resolve_power_on_exhaust(
        PowerId::DarkEmbrace,
        &state,
        0,
        1,
        118,
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
