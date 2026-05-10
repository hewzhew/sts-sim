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
        Action::GainBlock {
            target: 0,
            amount: 14
        }
    ));
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
    let strike = CombatCard::new(CardId::Strike, 140);
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
                item.source,
                crate::runtime::combat::QueuedCardSource::DoubleTap
            );
        }
        other => panic!("Double Tap should enqueue a purge-on-use copy, got {other:?}"),
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
    assert!(fiend_fire_state.zones.hand.is_empty());
    assert_eq!(fiend_fire_state.zones.exhaust_pile.len(), 3);
    assert_eq!(fiend_fire_state.entities.monsters[0].current_hp, 19);
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
fn juggernaut_block_hook_matches_java_source() {
    let state = crate::test_support::blank_test_combat();
    let actions =
        crate::content::powers::resolve_power_on_block_gained(PowerId::Juggernaut, &state, 0, 7, 5);
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        Action::AttackDamageRandomEnemy {
            base_damage,
            damage_type,
            applies_target_modifiers,
        } => {
            assert_eq!(*base_damage, 7);
            assert_eq!(*damage_type, DamageType::Thorns);
            assert!(!*applies_target_modifiers);
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
    for action in pummel_actions {
        match action.action {
            Action::Damage(info) => {
                assert_eq!(info.source, 0);
                assert_eq!(info.target, 111);
                assert_eq!(info.base, 3);
                assert_eq!(info.output, 3);
                assert_eq!(info.damage_type, DamageType::Normal);
            }
            other => panic!("Pummel+ should emit one damage action per hit, got {other:?}"),
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
            amount: 3
        }]
    );
    assert!(crate::content::powers::resolve_power_on_hp_lost(
        PowerId::Rupture,
        &state,
        0,
        3,
        None,
        DamageType::HpLoss,
        false,
    )
    .is_empty());

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
    assert_eq!(reaper_state.entities.player.current_hp, 60);
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
}
