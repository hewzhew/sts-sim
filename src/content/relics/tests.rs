use super::*;
use crate::content::cards::{evaluate_card_for_play, CardId};
use crate::content::monsters::EnemyId;
use crate::content::powers::{store, PowerId};
use crate::runtime::action::{Action, AddTo, DamageType, NO_SOURCE};
use crate::runtime::combat::{CombatCard, Power};

#[test]
fn ironclad_blood_skull_and_frog_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::BurningBlood), RelicTier::Starter);
    assert_eq!(get_relic_tier(RelicId::BlackBlood), RelicTier::Boss);
    assert_eq!(get_relic_tier(RelicId::RedSkull), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::PaperFrog), RelicTier::Uncommon);

    assert!(get_relic_subscriptions(RelicId::BurningBlood).on_victory);
    assert!(get_relic_subscriptions(RelicId::BlackBlood).on_victory);
    assert!(get_relic_subscriptions(RelicId::RedSkull).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::PaperFrog).on_calculate_vulnerable_multiplier);
}

#[test]
fn burning_and_black_blood_victory_heal_matches_java_current_hp_guard() {
    let mut burning_state = crate::test_support::blank_test_combat();
    burning_state.entities.player.current_hp = 10;
    burning_state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::BurningBlood));
    let burning_actions = hooks::on_victory(&mut burning_state);
    assert_eq!(burning_actions.len(), 1);
    assert_eq!(burning_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        burning_actions[0].action,
        Action::Heal {
            target: 0,
            amount: 6
        }
    ));

    burning_state.entities.player.current_hp = 0;
    assert!(hooks::on_victory(&mut burning_state).is_empty());

    let mut black_state = crate::test_support::blank_test_combat();
    black_state.entities.player.current_hp = 10;
    black_state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::BlackBlood));
    let black_actions = hooks::on_victory(&mut black_state);
    assert_eq!(black_actions.len(), 1);
    assert_eq!(black_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        black_actions[0].action,
        Action::Heal {
            target: 0,
            amount: 12
        }
    ));

    black_state.entities.player.current_hp = 0;
    assert!(hooks::on_victory(&mut black_state).is_empty());
}

#[test]
fn red_skull_threshold_hooks_match_java_bloodied_edges() {
    let mut state = crate::test_support::blank_test_combat();
    state.entities.player.current_hp = 30;
    state.entities.player.max_hp = 60;
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::RedSkull));

    let start_actions = hooks::at_battle_start(&mut state);
    assert_eq!(start_actions.len(), 1);
    assert_eq!(start_actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        start_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 3
        }
    ));

    assert!(red_skull::at_battle_start(31, 60).is_empty());
    assert!(matches!(
        red_skull::on_player_hp_changed(31, 30, 60)[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 3
        }
    ));
    assert!(matches!(
        red_skull::on_player_hp_changed(30, 31, 60)[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: -3
        }
    ));
    assert!(red_skull::on_player_hp_changed(30, 20, 60).is_empty());
}

#[test]
fn paper_frog_vulnerable_multiplier_applies_only_to_enemy_targets() {
    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::PaperFrog));
    assert_eq!(
        hooks::on_calculate_vulnerable_multiplier(&state, false),
        1.75
    );
    assert_eq!(hooks::on_calculate_vulnerable_multiplier(&state, true), 1.5);

    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = 801;
    state.entities.monsters = vec![monster];
    store::set_powers_for(
        &mut state,
        801,
        vec![Power {
            power_type: PowerId::Vulnerable,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );

    let strike = CombatCard::new(CardId::Strike, 1);
    let evaluated = evaluate_card_for_play(&strike, &state, Some(801));
    assert_eq!(evaluated.base_damage_mut, 10);
}

#[test]
fn ironclad_brimstone_belt_ashes_flower_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::Brimstone), RelicTier::Shop);
    assert_eq!(get_relic_tier(RelicId::ChampionBelt), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::CharonsAshes), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::MagicFlower), RelicTier::Rare);

    assert!(get_relic_subscriptions(RelicId::Brimstone).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::ChampionBelt).on_apply_power);
    assert!(get_relic_subscriptions(RelicId::CharonsAshes).on_exhaust);
    assert!(get_relic_subscriptions(RelicId::MagicFlower).on_calculate_heal);
}

#[test]
fn brimstone_turn_start_matches_java_strength_sources_and_top_order() {
    let mut state = crate::test_support::blank_test_combat();
    let mut first = crate::test_support::test_monster(EnemyId::JawWorm);
    first.id = 901;
    let mut second = crate::test_support::test_monster(EnemyId::Cultist);
    second.id = 902;
    state.entities.monsters = vec![first, second];

    let actions = brimstone::Brimstone::at_turn_start(&state);
    assert_eq!(actions.len(), 3);

    assert_eq!(actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        actions[0].action,
        Action::ApplyPower {
            source: 902,
            target: 902,
            power_id: PowerId::Strength,
            amount: 1
        }
    ));

    assert_eq!(actions[1].insertion_mode, AddTo::Top);
    assert!(matches!(
        actions[1].action,
        Action::ApplyPower {
            source: 901,
            target: 901,
            power_id: PowerId::Strength,
            amount: 1
        }
    ));

    assert_eq!(actions[2].insertion_mode, AddTo::Top);
    assert!(matches!(
        actions[2].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 2
        }
    ));
}

#[test]
fn champion_belt_respects_java_player_source_and_artifact_guard() {
    let mut state = crate::test_support::blank_test_combat();
    let mut target = crate::test_support::test_monster(EnemyId::JawWorm);
    target.id = 901;
    state.entities.monsters = vec![target];
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::ChampionBelt));

    let actions = hooks::on_apply_power(&mut state, 0, PowerId::Vulnerable, 901);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 901,
            power_id: PowerId::Weak,
            amount: 1
        }
    ));

    assert!(hooks::on_apply_power(&mut state, 901, PowerId::Vulnerable, 0).is_empty());
    assert!(hooks::on_apply_power(&mut state, 901, PowerId::Vulnerable, 902).is_empty());
    assert!(hooks::on_apply_power(&mut state, 0, PowerId::Weak, 901).is_empty());

    store::set_powers_for(
        &mut state,
        901,
        vec![Power {
            power_type: PowerId::Artifact,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    assert!(hooks::on_apply_power(&mut state, 0, PowerId::Vulnerable, 901).is_empty());
}

#[test]
fn charons_ashes_exhaust_damage_matches_java_thorns_null_source() {
    let mut state = crate::test_support::blank_test_combat();
    state.entities.monsters = vec![
        crate::test_support::test_monster(EnemyId::JawWorm),
        crate::test_support::test_monster(EnemyId::Cultist),
    ];

    let actions = charons_ashes::CharonsAshes::on_exhaust(&state);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Top);
    match &actions[0].action {
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            is_modified,
        } => {
            assert_eq!(*source, NO_SOURCE);
            assert_eq!(damages.as_slice(), &[3, 3]);
            assert_eq!(*damage_type, DamageType::Thorns);
            assert!(!*is_modified);
        }
        other => panic!("Charon's Ashes should emit DamageAllEnemiesAction, got {other:?}"),
    }
}

#[test]
fn magic_flower_combat_heal_rounding_matches_java_mathutils_round() {
    assert_eq!(magic_flower::modify_heal(1), 2);
    assert_eq!(magic_flower::modify_heal(2), 3);
    assert_eq!(magic_flower::modify_heal(5), 8);

    let mut state = crate::test_support::blank_test_combat();
    assert_eq!(hooks::on_calculate_heal(&state, 5), 5);

    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::MagicFlower));
    assert_eq!(hooks::on_calculate_heal(&state, 5), 8);
}

#[test]
fn ironclad_pain_cube_clay_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::MarkOfPain), RelicTier::Boss);
    assert_eq!(get_relic_tier(RelicId::RunicCube), RelicTier::Boss);
    assert_eq!(
        get_relic_tier(RelicId::SelfFormingClay),
        RelicTier::Uncommon
    );

    assert!(get_relic_subscriptions(RelicId::MarkOfPain).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::RunicCube).on_lose_hp);
    assert!(get_relic_subscriptions(RelicId::SelfFormingClay).on_lose_hp);
    assert_eq!(energy_master_delta(RelicId::MarkOfPain), 1);
    assert_eq!(energy_master_delta(RelicId::RunicCube), 0);
    assert_eq!(energy_master_delta(RelicId::SelfFormingClay), 0);
}

#[test]
fn mark_of_pain_battle_start_matches_java_wound_generation_and_energy() {
    let actions = mark_of_pain::at_battle_start();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::MakeTempCardInDrawPile {
            card_id: CardId::Wound,
            amount: 2,
            random_spot: true,
            to_bottom: false,
            upgraded: false,
        }
    ));

    let mut state = crate::test_support::blank_test_combat();
    assert_eq!(state.entities.player.energy_master, 3);
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::MarkOfPain));
    assert_eq!(state.entities.player.energy_master, 4);
}

#[test]
fn runic_cube_hp_loss_hook_matches_java_positive_damage_guard() {
    assert!(runic_cube::was_hp_lost(0).is_empty());
    assert!(runic_cube::was_hp_lost(-1).is_empty());

    let actions = runic_cube::was_hp_lost(3);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(actions[0].action, Action::DrawCards(1)));
}

#[test]
fn self_forming_clay_hp_loss_hook_matches_java_positive_damage_guard() {
    assert!(self_forming_clay::on_lose_hp(0).is_empty());
    assert!(self_forming_clay::on_lose_hp(-1).is_empty());

    let actions = self_forming_clay::on_lose_hp(3);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::NextTurnBlock,
            amount: 3
        }
    ));
}
