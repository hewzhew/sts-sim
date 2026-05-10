use super::*;
use crate::content::cards::{evaluate_card_for_play, CardId};
use crate::content::monsters::EnemyId;
use crate::content::powers::{store, PowerId};
use crate::runtime::action::{Action, AddTo, DamageInfo, DamageType, NO_SOURCE};
use crate::runtime::combat::{CombatCard, Power};
use crate::state::events::EventId;
use crate::state::selection::{DomainEvent, DomainEventSource};

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

#[test]
fn shared_common_battle_start_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::Akabeko), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::Anchor), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::BagOfMarbles), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::BagOfPreparation), RelicTier::Common);

    assert!(get_relic_subscriptions(RelicId::Akabeko).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::Anchor).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::BagOfMarbles).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::BagOfPreparation).at_battle_start);
}

#[test]
fn akabeko_anchor_and_bag_of_preparation_battle_start_actions_match_java_sources() {
    let akabeko_actions = akabeko::Akabeko::at_battle_start();
    assert_eq!(akabeko_actions.len(), 1);
    assert_eq!(akabeko_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        akabeko_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Vigor,
            amount: 8
        }
    ));

    let anchor_actions = anchor::Anchor::at_battle_start();
    assert_eq!(anchor_actions.len(), 1);
    assert_eq!(anchor_actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        anchor_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 10
        }
    ));

    let bag_actions = bag_of_preparation::BagOfPreparation::at_battle_start();
    assert_eq!(bag_actions.len(), 1);
    assert_eq!(bag_actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(bag_actions[0].action, Action::DrawCards(2)));
}

#[test]
fn bag_of_marbles_queues_vulnerable_for_every_current_monster() {
    let mut state = crate::test_support::blank_test_combat();
    let mut alive = crate::test_support::test_monster(EnemyId::JawWorm);
    alive.id = 901;
    let mut dying = crate::test_support::test_monster(EnemyId::Cultist);
    dying.id = 902;
    dying.is_dying = true;
    let mut escaped = crate::test_support::test_monster(EnemyId::AcidSlimeM);
    escaped.id = 903;
    escaped.is_escaped = true;
    state.entities.monsters = vec![alive, dying, escaped];

    let actions = bag_of_marbles::BagOfMarbles::at_battle_start(&state);
    assert_eq!(actions.len(), 3);
    for (action, target) in actions.iter().zip([901, 902, 903]) {
        assert_eq!(action.insertion_mode, AddTo::Bottom);
        assert!(matches!(
            action.action,
            Action::ApplyPower {
                source: 0,
                target: actual_target,
                power_id: PowerId::Vulnerable,
                amount: 1
            } if actual_target == target
        ));
    }
}

#[test]
fn shared_common_hp_and_thorns_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::BloodVial), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::BronzeScales), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::CentennialPuzzle), RelicTier::Common);

    assert!(get_relic_subscriptions(RelicId::BloodVial).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::BronzeScales).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::CentennialPuzzle).at_pre_battle);
    assert!(get_relic_subscriptions(RelicId::CentennialPuzzle).on_lose_hp);
}

#[test]
fn blood_vial_and_bronze_scales_battle_start_actions_match_java_sources() {
    let blood_actions = blood_vial::BloodVial::at_battle_start();
    assert_eq!(blood_actions.len(), 1);
    assert_eq!(blood_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        blood_actions[0].action,
        Action::Heal {
            target: 0,
            amount: 2
        }
    ));

    let scale_actions = bronze_scales::BronzeScales::at_battle_start(0);
    assert_eq!(scale_actions.len(), 1);
    assert_eq!(scale_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        scale_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Thorns,
            amount: 3
        }
    ));
}

#[test]
fn centennial_puzzle_marks_used_immediately_and_resets_pre_battle() {
    let mut relic_state = RelicState::new(RelicId::CentennialPuzzle);
    assert!(centennial_puzzle::CentennialPuzzle::on_lose_hp(&mut relic_state, 0).is_empty());
    assert!(!relic_state.used_up);

    let actions = centennial_puzzle::CentennialPuzzle::on_lose_hp(&mut relic_state, 4);
    assert!(relic_state.used_up);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(actions[0].action, Action::DrawCards(3)));
    assert!(centennial_puzzle::CentennialPuzzle::on_lose_hp(&mut relic_state, 4).is_empty());

    let reset_actions = centennial_puzzle::CentennialPuzzle::at_pre_battle(&mut relic_state);
    assert!(reset_actions.is_empty());
    assert!(!relic_state.used_up);
}

#[test]
fn centennial_puzzle_hook_updates_relic_state_before_draw_action_executes() {
    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::CentennialPuzzle));

    let actions = hooks::on_lose_hp(&mut state, 3);
    assert!(state.entities.player.relics[0].used_up);
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0].action, Action::DrawCards(3)));

    assert!(hooks::on_lose_hp(&mut state, 3).is_empty());
}

#[test]
fn shared_common_counter_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::HappyFlower), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::Lantern), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::Nunchaku), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::PenNib), RelicTier::Common);

    assert!(get_relic_subscriptions(RelicId::HappyFlower).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::Lantern).at_pre_battle);
    assert!(get_relic_subscriptions(RelicId::Lantern).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::Nunchaku).on_use_card);
    assert!(get_relic_subscriptions(RelicId::PenNib).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::PenNib).on_use_card);
}

#[test]
fn happy_flower_counter_updates_immediately_like_java() {
    let mut relic = RelicState::new(RelicId::HappyFlower);

    assert!(happy_flower::HappyFlower::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, 1);
    assert!(happy_flower::HappyFlower::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, 2);

    let actions = happy_flower::HappyFlower::at_turn_start(&mut relic);
    assert_eq!(relic.counter, 0);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::GainEnergy { amount: 1 }
    ));

    relic.counter = -1;
    assert!(happy_flower::HappyFlower::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, 1);
}

#[test]
fn lantern_first_turn_state_updates_immediately_like_java() {
    let mut relic = RelicState::new(RelicId::Lantern);
    relic.used_up = true;

    let pre_battle_actions = lantern::at_pre_battle(&mut relic);
    assert!(pre_battle_actions.is_empty());
    assert!(!relic.used_up);

    let first_turn_actions = lantern::at_turn_start(&mut relic);
    assert!(relic.used_up);
    assert_eq!(first_turn_actions.len(), 1);
    assert_eq!(first_turn_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        first_turn_actions[0].action,
        Action::GainEnergy { amount: 1 }
    ));

    assert!(lantern::at_turn_start(&mut relic).is_empty());
}

#[test]
fn nunchaku_counter_updates_immediately_like_java() {
    let mut relic = RelicState::new(RelicId::Nunchaku);
    relic.counter = 8;

    assert!(nunchaku::on_use_card(&mut relic).is_empty());
    assert_eq!(relic.counter, 9);

    let actions = nunchaku::on_use_card(&mut relic);
    assert_eq!(relic.counter, 0);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::GainEnergy { amount: 1 }
    ));
}

#[test]
fn pen_nib_counter_and_power_timing_match_java() {
    let mut relic = RelicState::new(RelicId::PenNib);
    relic.counter = 8;

    let ninth_actions = pen_nib::on_use_card(&mut relic);
    assert_eq!(relic.counter, 9);
    assert_eq!(ninth_actions.len(), 1);
    assert_eq!(ninth_actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        ninth_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::PenNibPower,
            amount: 1
        }
    ));

    let tenth_actions = pen_nib::on_use_card(&mut relic);
    assert_eq!(relic.counter, 0);
    assert!(tenth_actions.is_empty());

    let battle_start_actions = pen_nib::at_battle_start(9);
    assert_eq!(battle_start_actions.len(), 1);
    assert!(matches!(
        battle_start_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::PenNibPower,
            amount: 1
        }
    ));
    assert!(pen_nib::at_battle_start(8).is_empty());
}

#[test]
fn shared_common_turn_state_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::AncientTeaSet), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::ArtOfWar), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::Orichalcum), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::OddlySmoothStone), RelicTier::Common);

    assert!(get_relic_subscriptions(RelicId::AncientTeaSet).at_pre_battle);
    assert!(get_relic_subscriptions(RelicId::AncientTeaSet).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::AncientTeaSet).on_enter_rest_room);
    assert!(get_relic_subscriptions(RelicId::ArtOfWar).at_pre_battle);
    assert!(get_relic_subscriptions(RelicId::ArtOfWar).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::ArtOfWar).on_use_card);
    assert!(get_relic_subscriptions(RelicId::Orichalcum).at_end_of_turn);
    assert!(get_relic_subscriptions(RelicId::OddlySmoothStone).at_battle_start);
}

#[test]
fn ancient_tea_set_first_turn_state_matches_java() {
    let mut relic = RelicState::new(RelicId::AncientTeaSet);
    ancient_tea_set::AncientTeaSet::on_enter_rest_room(&mut relic);
    assert_eq!(relic.counter, -2);

    assert!(ancient_tea_set::AncientTeaSet::at_pre_battle(&mut relic).is_empty());
    assert!(!relic.used_up);

    let actions = ancient_tea_set::AncientTeaSet::at_turn_start(&mut relic);
    assert!(relic.used_up);
    assert_eq!(relic.counter, -1);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        actions[0].action,
        Action::GainEnergy { amount: 2 }
    ));

    assert!(ancient_tea_set::AncientTeaSet::at_turn_start(&mut relic).is_empty());
}

#[test]
fn art_of_war_turn_and_attack_state_matches_java() {
    let state = crate::test_support::blank_test_combat();
    let mut relic = RelicState::new(RelicId::ArtOfWar);

    assert!(art_of_war::at_pre_battle(&mut relic).is_empty());
    assert_eq!(relic.counter, -1);

    assert!(art_of_war::at_turn_start(&state, &mut relic).is_empty());
    assert_eq!(relic.counter, 1);

    assert!(art_of_war::on_use_card(&state, &mut relic, CardId::Defend).is_empty());
    assert_eq!(relic.counter, 1);
    assert!(art_of_war::on_use_card(&state, &mut relic, CardId::Strike).is_empty());
    assert_eq!(relic.counter, 0);

    assert!(art_of_war::at_turn_start(&state, &mut relic).is_empty());
    assert_eq!(relic.counter, 1);

    let actions = art_of_war::at_turn_start(&state, &mut relic);
    assert_eq!(relic.counter, 1);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::GainEnergy { amount: 1 }
    ));
}

#[test]
fn orichalcum_and_smooth_stone_actions_match_java_sources() {
    let mut state = crate::test_support::blank_test_combat();
    state.entities.player.block = 0;
    let orichalcum_actions = orichalcum::at_end_of_turn(&state);
    assert_eq!(orichalcum_actions.len(), 1);
    assert_eq!(orichalcum_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        orichalcum_actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 6
        }
    ));

    state.entities.player.block = 1;
    assert!(orichalcum::at_end_of_turn(&state).is_empty());

    let stone_actions = oddly_smooth_stone::at_battle_start();
    assert_eq!(stone_actions.len(), 1);
    assert_eq!(stone_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        stone_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Dexterity,
            amount: 1
        }
    ));
}

#[test]
fn shared_common_damage_hp_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::Boot), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::PreservedInsect), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::Vajra), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::Strawberry), RelicTier::Common);

    assert!(!get_relic_subscriptions(RelicId::Boot).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::PreservedInsect).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::Vajra).at_battle_start);
    assert_eq!(energy_master_delta(RelicId::Strawberry), 0);
}

#[test]
fn boot_damage_floor_applies_only_to_positive_normal_player_damage() {
    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::Boot));
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = 901;
    monster.current_hp = 20;
    monster.max_hp = 20;
    state.entities.monsters = vec![monster];

    crate::engine::action_handlers::damage::handle_damage(
        DamageInfo {
            source: 0,
            target: 901,
            base: 3,
            output: 3,
            damage_type: DamageType::Normal,
            is_modified: false,
        },
        &mut state,
    );
    assert_eq!(state.entities.monsters[0].current_hp, 15);

    state.entities.monsters[0].current_hp = 20;
    crate::engine::action_handlers::damage::handle_damage(
        DamageInfo {
            source: NO_SOURCE,
            target: 901,
            base: 3,
            output: 3,
            damage_type: DamageType::Thorns,
            is_modified: false,
        },
        &mut state,
    );
    assert_eq!(state.entities.monsters[0].current_hp, 17);
}

#[test]
fn preserved_insect_uses_elite_room_flag_and_reduces_current_hp_only() {
    let mut state = crate::test_support::blank_test_combat();
    state.meta.is_elite_fight = true;
    let mut first = crate::test_support::test_monster(EnemyId::Cultist);
    first.id = 901;
    first.current_hp = 100;
    first.max_hp = 100;
    let mut second = crate::test_support::test_monster(EnemyId::JawWorm);
    second.id = 902;
    second.current_hp = 20;
    second.max_hp = 100;
    state.entities.monsters = vec![first, second];

    let actions = preserved_insect::at_battle_start(&mut state);
    assert!(actions.is_empty());
    assert_eq!(state.entities.monsters[0].current_hp, 75);
    assert_eq!(state.entities.monsters[0].max_hp, 100);
    assert_eq!(
        state.entities.monsters[1].current_hp, 20,
        "Java does not heal monsters that are already below the 75% threshold"
    );
    assert_eq!(state.entities.monsters[1].max_hp, 100);

    state.meta.is_elite_fight = false;
    state.entities.monsters[0].current_hp = 100;
    assert!(preserved_insect::at_battle_start(&mut state).is_empty());
    assert_eq!(state.entities.monsters[0].current_hp, 100);
}

#[test]
fn vajra_and_strawberry_match_java_sources() {
    let vajra_actions = vajra::at_battle_start();
    assert_eq!(vajra_actions.len(), 1);
    assert_eq!(vajra_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        vajra_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 1
        }
    ));

    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 50;
    run_state.max_hp = 80;
    assert!(strawberry::on_equip(&mut run_state).is_none());
    assert_eq!(run_state.max_hp, 87);
    assert_eq!(run_state.current_hp, 57);
}

#[test]
fn shared_common_run_gold_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::CeramicFish), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::DreamCatcher), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::JuzuBracelet), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::MawBank), RelicTier::Common);

    assert!(!get_relic_subscriptions(RelicId::CeramicFish).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::DreamCatcher).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::JuzuBracelet).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::MawBank).at_battle_start);
}

#[test]
fn ceramic_fish_obtain_card_gold_uses_java_gain_gold_semantics() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 10;
    run_state.relics.clear();
    run_state.relics.push(RelicState::new(RelicId::CeramicFish));

    let deck_len = run_state.master_deck.len();
    assert!(run_state.add_card_to_deck(CardId::Strike));
    assert_eq!(run_state.master_deck.len(), deck_len + 1);
    assert_eq!(run_state.gold, 19);

    let mut blocked = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    blocked.gold = 10;
    blocked.relics.clear();
    blocked.relics.push(RelicState::new(RelicId::CeramicFish));
    blocked.relics.push(RelicState::new(RelicId::Omamori));

    let blocked_len = blocked.master_deck.len();
    assert!(!blocked.add_card_to_deck(CardId::Regret));
    assert_eq!(blocked.master_deck.len(), blocked_len);
    assert_eq!(
        blocked.gold, 10,
        "Omamori prevents the curse obtain, so Ceramic Fish onObtainCard does not fire"
    );

    let mut ectoplasm = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    ectoplasm.gold = 10;
    ectoplasm.relics.clear();
    ectoplasm.relics.push(RelicState::new(RelicId::CeramicFish));
    ectoplasm.relics.push(RelicState::new(RelicId::Ectoplasm));

    assert!(ectoplasm.add_card_to_deck(CardId::Strike));
    assert_eq!(
        ectoplasm.gold, 10,
        "Java routes Ceramic Fish through AbstractPlayer.gainGold, which Ectoplasm blocks"
    );
}

#[test]
fn ectoplasm_blocks_run_combat_and_on_equip_gold_gain_paths() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 10;
    run_state.relics.clear();
    run_state.relics.push(RelicState::new(RelicId::Ectoplasm));
    assert_eq!(
        run_state.change_gold_with_source(50, DomainEventSource::Event(EventId::GoldenShrine)),
        0
    );
    assert_eq!(run_state.gold, 10);

    old_coin::on_equip(&mut run_state);
    assert_eq!(
        run_state.gold, 10,
        "Old Coin also routes through Java gainGold semantics when Ectoplasm is present"
    );

    let mut combat_state = crate::test_support::blank_test_combat();
    combat_state.entities.player.gold = 10;
    combat_state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::Ectoplasm));
    crate::engine::action_handlers::damage::handle_gain_gold(50, &mut combat_state);
    assert_eq!(combat_state.entities.player.gold, 10);
    assert_eq!(combat_state.entities.player.gold_delta_this_combat, 0);
}

#[test]
fn maw_bank_only_spending_in_shop_uses_it_up_like_java_lose_gold() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.gold = 100;
    run_state.relics.clear();
    run_state.relics.push(RelicState::new(RelicId::MawBank));

    run_state.change_gold_with_source(-25, DomainEventSource::Event(EventId::TheJoust));
    let maw_bank = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::MawBank)
        .expect("MawBank should be present");
    assert!(!maw_bank.used_up);
    assert_eq!(maw_bank.counter, -1);

    run_state.change_gold_with_source(-25, DomainEventSource::Shop);
    let maw_bank = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::MawBank)
        .expect("MawBank should be present");
    assert!(maw_bank.used_up);
    assert_eq!(maw_bank.counter, -2);
}

#[test]
fn juzu_bracelet_converts_monster_event_roll_without_preserving_monster_chance() {
    let mut generator = crate::events::generator::EventGenerator::new(1);
    generator.monster_chance = 1.0;
    generator.shop_chance = 0.0;
    generator.treasure_chance = 0.0;
    let mut rng = crate::runtime::rng::RngPool::new(1);
    let ctx = crate::events::context::EventContext {
        act_num: 1,
        ascension_level: 0,
        floor_num: 10,
        gold: 99,
        current_hp: 80,
        max_hp: 80,
        has_curses: false,
        tiny_chest_counter: 0,
        has_golden_idol: false,
        has_juzu_bracelet: true,
        relic_count: 1,
    };

    assert_eq!(
        generator.roll_room_type(&mut rng, &ctx),
        crate::events::generator::RoomRoll::Event
    );
    assert_eq!(
        generator.monster_chance, 0.10,
        "Java resets MONSTER_CHANCE after a Juzu-converted monster roll"
    );
}

#[test]
fn shared_common_shop_rest_event_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::MealTicket), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::RegalPillow), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::SmilingMask), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::TinyChest), RelicTier::Common);

    assert!(!get_relic_subscriptions(RelicId::MealTicket).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::RegalPillow).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::SmilingMask).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::TinyChest).at_battle_start);
}

#[test]
fn tiny_chest_counter_forces_treasure_roll_every_fourth_unknown_room() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.relics.clear();
    let mut tiny_chest = RelicState::new(RelicId::TinyChest);
    tiny_chest.counter = 3;
    run_state.relics.push(tiny_chest);

    let _ = run_state.generate_event();
    let tiny_chest = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::TinyChest)
        .expect("Tiny Chest should be present");
    assert_eq!(tiny_chest.counter, 0);

    let mut generator = crate::events::generator::EventGenerator::new(1);
    generator.monster_chance = 1.0;
    generator.shop_chance = 0.0;
    generator.treasure_chance = 0.0;
    let mut rng = crate::runtime::rng::RngPool::new(1);
    let ctx = crate::events::context::EventContext {
        act_num: 1,
        ascension_level: 0,
        floor_num: 10,
        gold: 99,
        current_hp: 80,
        max_hp: 80,
        has_curses: false,
        tiny_chest_counter: 3,
        has_golden_idol: false,
        has_juzu_bracelet: false,
        relic_count: 1,
    };

    assert_eq!(
        generator.roll_room_type(&mut rng, &ctx),
        crate::events::generator::RoomRoll::Treasure
    );
}

#[test]
fn shared_common_obtain_potion_upgrade_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::Omamori), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::PotionBelt), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::ToyOrnithopter), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::WarPaint), RelicTier::Common);
    assert_eq!(get_relic_tier(RelicId::Whetstone), RelicTier::Common);

    assert!(!get_relic_subscriptions(RelicId::Omamori).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::PotionBelt).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::ToyOrnithopter).on_use_potion);
    assert!(!get_relic_subscriptions(RelicId::WarPaint).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::Whetstone).at_battle_start);
}

#[test]
fn omamori_blocks_exactly_two_curse_obtains_then_marks_used_up() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.relics.clear();
    run_state.relics.push(RelicState::new(RelicId::Omamori));
    let deck_len = run_state.master_deck.len();

    assert!(!run_state.add_card_to_deck(CardId::Regret));
    let omamori = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::Omamori)
        .expect("Omamori should be present");
    assert_eq!(omamori.counter, 1);
    assert!(!omamori.used_up);
    assert_eq!(run_state.master_deck.len(), deck_len);

    assert!(!run_state.add_card_to_deck(CardId::Injury));
    let omamori = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::Omamori)
        .expect("Omamori should be present");
    assert_eq!(omamori.counter, 0);
    assert!(omamori.used_up);
    assert_eq!(run_state.master_deck.len(), deck_len);

    assert!(run_state.add_card_to_deck(CardId::Doubt));
    assert_eq!(run_state.master_deck.len(), deck_len + 1);
}

#[test]
fn potion_belt_appends_two_empty_slots_without_reordering_existing_potions() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.potions = vec![
        Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::StrengthPotion,
            1,
        )),
        None,
    ];

    assert!(potion_belt::on_equip(&mut run_state).is_none());
    assert_eq!(run_state.potions.len(), 4);
    assert_eq!(
        run_state.potions[0].as_ref().map(|potion| potion.id),
        Some(crate::content::potions::PotionId::StrengthPotion)
    );
    assert!(run_state.potions[1].is_none());
    assert!(run_state.potions[2].is_none());
    assert!(run_state.potions[3].is_none());
}

#[test]
fn toy_ornithopter_queues_bottom_heal_when_potion_is_used() {
    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::ToyOrnithopter));

    let actions = hooks::on_use_potion(&state, 0);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::Heal {
            target: 0,
            amount: 5
        }
    ));
}

#[test]
fn war_paint_and_whetstone_upgrade_only_matching_card_types_with_relic_source() {
    let mut whetstone_run = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    whetstone_run.master_deck = vec![
        CombatCard::new(CardId::Strike, 1),
        CombatCard::new(CardId::Defend, 2),
    ];
    whetstone_run.emitted_events.clear();

    assert!(whetstone::on_equip(&mut whetstone_run).is_none());
    assert_eq!(whetstone_run.master_deck[0].upgrades, 1);
    assert_eq!(whetstone_run.master_deck[1].upgrades, 0);
    assert!(whetstone_run.emitted_events.iter().any(|event| matches!(
        event,
        DomainEvent::CardUpgraded {
            source: DomainEventSource::Relic(RelicId::Whetstone),
            ..
        }
    )));

    let mut war_paint_run = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    war_paint_run.master_deck = vec![
        CombatCard::new(CardId::Strike, 1),
        CombatCard::new(CardId::Defend, 2),
    ];
    war_paint_run.emitted_events.clear();

    assert!(war_paint::on_equip(&mut war_paint_run).is_none());
    assert_eq!(war_paint_run.master_deck[0].upgrades, 0);
    assert_eq!(war_paint_run.master_deck[1].upgrades, 1);
    assert!(war_paint_run.emitted_events.iter().any(|event| matches!(
        event,
        DomainEvent::CardUpgraded {
            source: DomainEventSource::Relic(RelicId::WarPaint),
            ..
        }
    )));
}

#[test]
fn shared_uncommon_card_reward_relic_metadata_matches_java_sources() {
    assert_eq!(
        get_relic_tier(RelicId::DarkstonePeriapt),
        RelicTier::Uncommon
    );
    assert_eq!(get_relic_tier(RelicId::MoltenEgg), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::ToxicEgg), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::FrozenEgg), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::QuestionCard), RelicTier::Uncommon);

    assert!(!get_relic_subscriptions(RelicId::DarkstonePeriapt).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::MoltenEgg).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::ToxicEgg).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::FrozenEgg).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::QuestionCard).at_battle_start);
}

#[test]
fn darkstone_periapt_triggers_only_after_curse_is_obtained() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.relics.clear();
    run_state
        .relics
        .push(RelicState::new(RelicId::DarkstonePeriapt));
    run_state.current_hp = 50;
    run_state.max_hp = 80;
    run_state.emitted_events.clear();

    let deck_len = run_state.master_deck.len();
    assert!(run_state.add_card_to_deck_with_upgrades_from(
        CardId::Regret,
        0,
        DomainEventSource::RewardScreen
    ));
    assert_eq!(run_state.master_deck.len(), deck_len + 1);
    assert_eq!(run_state.max_hp, 86);
    assert_eq!(run_state.current_hp, 56);
    assert!(run_state.emitted_events.iter().any(|event| matches!(
        event,
        DomainEvent::MaxHpChanged {
            delta: 6,
            source: DomainEventSource::RewardScreen,
            ..
        }
    )));

    let mut blocked = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    blocked.relics.clear();
    blocked
        .relics
        .push(RelicState::new(RelicId::DarkstonePeriapt));
    blocked.relics.push(RelicState::new(RelicId::Omamori));
    blocked.current_hp = 50;
    blocked.max_hp = 80;

    let blocked_len = blocked.master_deck.len();
    assert!(!blocked.add_card_to_deck(CardId::Regret));
    assert_eq!(blocked.master_deck.len(), blocked_len);
    assert_eq!(
        blocked.max_hp, 80,
        "Omamori prevents the curse obtain, so Darkstone Periapt onObtainCard does not fire"
    );
    assert_eq!(blocked.current_hp, 50);
}

#[test]
fn egg_relics_preview_obtain_upgrades_without_double_upgrading_existing_plus_cards() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.relics.clear();
    run_state.relics.push(RelicState::new(RelicId::MoltenEgg));
    run_state.relics.push(RelicState::new(RelicId::ToxicEgg));
    run_state.relics.push(RelicState::new(RelicId::FrozenEgg));
    run_state.master_deck.clear();

    assert!(run_state.add_card_to_deck(CardId::Strike));
    assert!(run_state.add_card_to_deck(CardId::Defend));
    assert!(run_state.add_card_to_deck(CardId::Inflame));
    assert!(run_state.add_card_to_deck_with_upgrades(CardId::SearingBlow, 1));

    assert_eq!(run_state.master_deck[0].upgrades, 1);
    assert_eq!(run_state.master_deck[1].upgrades, 1);
    assert_eq!(run_state.master_deck[2].upgrades, 1);
    assert_eq!(
        run_state.master_deck[3].upgrades, 1,
        "Java Egg relics call upgrade only when !card.upgraded; pre-upgraded Searing Blow is not incremented again"
    );
}

#[test]
fn shared_uncommon_combat_trigger_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::GremlinHorn), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::LetterOpener), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::Kunai), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::Shuriken), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::OrnamentalFan), RelicTier::Uncommon);
    assert_eq!(
        get_relic_tier(RelicId::MercuryHourglass),
        RelicTier::Uncommon
    );

    assert!(get_relic_subscriptions(RelicId::GremlinHorn).on_monster_death);
    assert!(get_relic_subscriptions(RelicId::LetterOpener).on_use_card);
    assert!(get_relic_subscriptions(RelicId::LetterOpener).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::LetterOpener).on_victory);
    assert!(get_relic_subscriptions(RelicId::Kunai).on_use_card);
    assert!(get_relic_subscriptions(RelicId::Kunai).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::Kunai).on_victory);
    assert!(get_relic_subscriptions(RelicId::Shuriken).on_use_card);
    assert!(get_relic_subscriptions(RelicId::Shuriken).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::Shuriken).on_victory);
    assert!(get_relic_subscriptions(RelicId::OrnamentalFan).on_use_card);
    assert!(get_relic_subscriptions(RelicId::OrnamentalFan).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::OrnamentalFan).on_victory);
    assert!(get_relic_subscriptions(RelicId::MercuryHourglass).at_turn_start);
}

#[test]
fn gremlin_horn_triggers_only_when_another_monster_remains_alive() {
    let mut dying = crate::test_support::test_monster(EnemyId::JawWorm);
    dying.id = 1;
    dying.current_hp = 0;
    dying.is_dying = true;
    let mut alive = crate::test_support::test_monster(EnemyId::Cultist);
    alive.id = 2;

    let state = crate::test_support::combat_with_monsters(vec![dying.clone(), alive]);
    let actions = gremlin_horn::GremlinHorn::on_monster_death(&state, 1);
    assert_eq!(actions.len(), 2);
    assert!(matches!(
        actions[0].action,
        Action::GainEnergy { amount: 1 }
    ));
    assert!(matches!(actions[1].action, Action::DrawCards(1)));

    let last_monster_state = crate::test_support::combat_with_monsters(vec![dying]);
    assert!(
        gremlin_horn::GremlinHorn::on_monster_death(&last_monster_state, 1).is_empty(),
        "Java Gremlin Horn checks !areMonstersBasicallyDead(), so the final kill does not trigger"
    );
}

#[test]
fn letter_opener_resets_each_turn_and_fires_on_third_skill() {
    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::JawWorm));
    state
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::Cultist));

    let start_actions = letter_opener::at_turn_start();
    assert!(matches!(
        start_actions[0].action,
        Action::UpdateRelicCounter {
            relic_id: RelicId::LetterOpener,
            counter: 0
        }
    ));

    let first_skill = letter_opener::on_use_card(&state, CardId::Defend, -1);
    assert_eq!(first_skill.len(), 1);
    assert!(matches!(
        first_skill[0].action,
        Action::UpdateRelicCounter {
            relic_id: RelicId::LetterOpener,
            counter: 1
        }
    ));

    let third_skill = letter_opener::on_use_card(&state, CardId::Defend, 2);
    assert_eq!(third_skill.len(), 2);
    assert!(matches!(
        third_skill[0].action,
        Action::UpdateRelicCounter {
            relic_id: RelicId::LetterOpener,
            counter: 0
        }
    ));
    match &third_skill[1].action {
        Action::DamageAllEnemies {
            damages,
            damage_type,
            ..
        } => {
            assert_eq!(damages.as_slice(), &[5, 5]);
            assert_eq!(*damage_type, crate::runtime::action::DamageType::Thorns);
        }
        other => panic!("expected Letter Opener all-enemy damage, got {other:?}"),
    }
}

#[test]
fn attack_counter_relics_fire_on_third_attack_and_reset_on_victory() {
    let kunai_actions = kunai::on_use_card(CardId::Strike, 2);
    assert_eq!(kunai_actions.len(), 2);
    assert!(matches!(
        kunai_actions[0].action,
        Action::UpdateRelicCounter {
            relic_id: RelicId::Kunai,
            counter: 0
        }
    ));
    assert!(matches!(
        kunai_actions[1].action,
        Action::ApplyPower {
            power_id: PowerId::Dexterity,
            amount: 1,
            ..
        }
    ));
    assert!(kunai::on_use_card(CardId::Defend, 2).is_empty());

    let shuriken_actions = shuriken::on_use_card(CardId::Strike, 2);
    assert_eq!(shuriken_actions.len(), 2);
    assert!(matches!(
        shuriken_actions[1].action,
        Action::ApplyPower {
            power_id: PowerId::Strength,
            amount: 1,
            ..
        }
    ));

    let fan_actions = ornamental_fan::on_use_card(2);
    assert_eq!(fan_actions.len(), 2);
    assert!(matches!(
        fan_actions[0].action,
        Action::UpdateRelicCounter {
            relic_id: RelicId::OrnamentalFan,
            counter: 0
        }
    ));
    assert!(matches!(
        fan_actions[1].action,
        Action::GainBlock {
            target: 0,
            amount: 4
        }
    ));

    let mut relic = RelicState::new(RelicId::Kunai);
    relic.counter = 2;
    kunai::on_victory(&mut relic);
    assert_eq!(relic.counter, -1);
}

#[test]
fn mercury_hourglass_queues_thorns_damage_to_all_monster_slots() {
    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::JawWorm));
    state
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::Cultist));

    let actions = mercury_hourglass::at_turn_start(&state);
    assert_eq!(actions.len(), 1);
    match &actions[0].action {
        Action::DamageAllEnemies {
            damages,
            damage_type,
            ..
        } => {
            assert_eq!(damages.as_slice(), &[3, 3]);
            assert_eq!(*damage_type, crate::runtime::action::DamageType::Thorns);
        }
        other => panic!("expected Mercury Hourglass all-enemy damage, got {other:?}"),
    }
}

#[test]
fn shared_uncommon_start_victory_reward_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::HornCleat), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::Pantograph), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::MeatOnTheBone), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::Pear), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::SingingBowl), RelicTier::Uncommon);
    assert_eq!(
        get_relic_tier(RelicId::WhiteBeastStatue),
        RelicTier::Uncommon
    );

    assert!(get_relic_subscriptions(RelicId::HornCleat).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::HornCleat).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::HornCleat).on_victory);
    assert!(get_relic_subscriptions(RelicId::Pantograph).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::MeatOnTheBone).on_victory);
    assert!(!get_relic_subscriptions(RelicId::Pear).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::SingingBowl).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::WhiteBeastStatue).at_battle_start);
}

#[test]
fn horn_cleat_triggers_only_on_second_turn_then_disables_until_next_combat() {
    let mut relic = RelicState::new(RelicId::HornCleat);
    horn_cleat::HornCleat::at_battle_start(&mut relic);
    assert_eq!(relic.counter, 0);

    assert!(horn_cleat::HornCleat::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, 1);

    let second_turn = horn_cleat::HornCleat::at_turn_start(&mut relic);
    assert_eq!(relic.counter, -1);
    assert_eq!(second_turn.len(), 1);
    assert_eq!(second_turn[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        second_turn[0].action,
        Action::GainBlock {
            target: 0,
            amount: 14
        }
    ));

    assert!(horn_cleat::HornCleat::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, -1);

    let mut state = crate::test_support::blank_test_combat();
    let mut state_relic = RelicState::new(RelicId::HornCleat);
    state.entities.player.add_relic(state_relic.clone());
    assert!(hooks::at_battle_start(&mut state).is_empty());
    assert_eq!(state.entities.player.relics[0].counter, 0);
    assert!(hooks::at_turn_start(&mut state).is_empty());
    assert_eq!(state.entities.player.relics[0].counter, 1);

    state.entities.player.relics.clear();
    state_relic.counter = 1;
    state_relic.used_up = true;
    state.entities.player.add_relic(state_relic);
    assert!(hooks::on_victory(&mut state).is_empty());
    assert_eq!(state.entities.player.relics[0].counter, -1);
    assert!(!state.entities.player.relics[0].used_up);
}

#[test]
fn pantograph_heals_only_in_boss_combat_with_java_top_insertion() {
    let mut boss_state = crate::test_support::blank_test_combat();
    boss_state
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::SlimeBoss));
    let actions = pantograph::at_battle_start(&boss_state);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        actions[0].action,
        Action::Heal {
            target: 0,
            amount: 25
        }
    ));

    let mut hallway_state = crate::test_support::blank_test_combat();
    hallway_state
        .entities
        .monsters
        .push(crate::test_support::test_monster(EnemyId::JawWorm));
    assert!(pantograph::at_battle_start(&hallway_state).is_empty());
}

#[test]
fn meat_on_the_bone_heals_at_or_below_half_hp_without_used_up_gate() {
    let mut state = crate::test_support::blank_test_combat();
    state.entities.player.current_hp = 40;
    state.entities.player.max_hp = 80;

    let actions = meat_on_the_bone::on_victory(&state);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        actions[0].action,
        Action::Heal {
            target: 0,
            amount: 12
        }
    ));

    state.entities.player.current_hp = 41;
    assert!(meat_on_the_bone::on_victory(&state).is_empty());

    state.entities.player.current_hp = 0;
    assert!(meat_on_the_bone::on_victory(&state).is_empty());

    state.entities.player.current_hp = 40;
    let mut used_relic = RelicState::new(RelicId::MeatOnTheBone);
    used_relic.used_up = true;
    state.entities.player.add_relic(used_relic);
    assert_eq!(
        hooks::on_victory(&mut state).len(),
        1,
        "Java Meat on the Bone is not a one-time used_up relic"
    );
}

#[test]
fn pear_on_equip_grants_ten_max_hp_and_heals_same_amount() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 50;
    run_state.max_hp = 80;

    assert!(pear::on_equip(&mut run_state).is_none());
    assert_eq!(run_state.max_hp, 90);
    assert_eq!(run_state.current_hp, 60);
}

#[test]
fn singing_bowl_card_reward_option_grants_two_max_hp_with_reward_source() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.current_hp = 50;
    run_state.max_hp = 80;
    run_state.relics.clear();
    run_state.relics.push(RelicState::new(RelicId::SingingBowl));
    run_state.emitted_events.clear();

    let mut reward_state = crate::rewards::state::RewardState {
        items: Vec::new(),
        skippable: true,
        screen_context: crate::rewards::state::RewardScreenContext::Standard,
        pending_card_choice: Some(vec![crate::rewards::state::RewardCard::new(
            CardId::Strike,
            0,
        )]),
    };

    assert_eq!(
        crate::rewards::handler::handle(
            &mut run_state,
            &mut reward_state,
            Some(crate::state::core::ClientInput::SelectCard(1)),
        ),
        Some(crate::state::core::EngineState::MapNavigation)
    );
    assert_eq!(run_state.max_hp, 82);
    assert_eq!(run_state.current_hp, 52);
    assert!(run_state.emitted_events.iter().any(|event| matches!(
        event,
        DomainEvent::MaxHpChanged {
            delta: 2,
            source: DomainEventSource::RewardScreen,
            ..
        }
    )));
}

#[test]
fn white_beast_statue_forces_potion_reward_unless_sozu_blocks_potions() {
    let mut run_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run_state.relics.clear();
    run_state
        .relics
        .push(RelicState::new(RelicId::WhiteBeastStatue));

    let rewards = crate::rewards::generator::generate_combat_rewards(&mut run_state, false, false);
    assert!(
        rewards
            .items
            .iter()
            .any(|item| matches!(item, crate::rewards::state::RewardItem::Potion { .. })),
        "White Beast Statue sets the post-combat potion drop chance to 100"
    );

    let mut sozu_state = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    sozu_state.relics.clear();
    sozu_state
        .relics
        .push(RelicState::new(RelicId::WhiteBeastStatue));
    sozu_state.relics.push(RelicState::new(RelicId::Sozu));

    let sozu_rewards =
        crate::rewards::generator::generate_combat_rewards(&mut sozu_state, false, false);
    assert!(
        !sozu_rewards
            .items
            .iter()
            .any(|item| matches!(item, crate::rewards::state::RewardItem::Potion { .. })),
        "Java Sozu prevents potion rewards before White Beast can matter"
    );
}

#[test]
fn shared_uncommon_action_counter_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::BlueCandle), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::InkBottle), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::MummifiedHand), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::Sundial), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::StrikeDummy), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::Matryoshka), RelicTier::Uncommon);

    assert!(get_relic_subscriptions(RelicId::BlueCandle).on_use_card);
    assert!(get_relic_subscriptions(RelicId::InkBottle).on_use_card);
    assert!(get_relic_subscriptions(RelicId::MummifiedHand).on_use_card);
    assert!(get_relic_subscriptions(RelicId::Sundial).on_shuffle);
    assert!(!get_relic_subscriptions(RelicId::StrikeDummy).on_use_card);
    assert!(!get_relic_subscriptions(RelicId::Matryoshka).on_use_card);
}

#[test]
fn blue_candle_only_loses_hp_for_curse_cards_and_marks_rupture_path() {
    assert!(blue_candle::BlueCandle::on_use_card(CardId::Strike).is_empty());

    let actions = blue_candle::BlueCandle::on_use_card(CardId::Regret);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::LoseHp {
            target: 0,
            amount: 1,
            triggers_rupture: true
        }
    ));
}

#[test]
fn ink_bottle_counter_mutates_immediately_and_draws_on_tenth_card() {
    let mut relic = RelicState::new(RelicId::InkBottle);
    relic.counter = 8;
    assert!(ink_bottle::on_use_card(&mut relic).is_empty());
    assert_eq!(relic.counter, 9);

    let actions = ink_bottle::on_use_card(&mut relic);
    assert_eq!(relic.counter, 0);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(actions[0].action, Action::DrawCards(1)));

    relic.counter = -1;
    assert!(ink_bottle::on_use_card(&mut relic).is_empty());
    assert_eq!(
        relic.counter, 0,
        "Java ++counter from -1 reaches 0 without triggering the tenth-card draw"
    );

    let mut state = crate::test_support::blank_test_combat();
    let mut state_relic = RelicState::new(RelicId::InkBottle);
    state_relic.counter = 9;
    state.entities.player.add_relic(state_relic);
    let strike = CombatCard::new(CardId::Strike, 11);
    assert_eq!(hooks::on_use_card(&mut state, &strike, None).len(), 1);
    assert_eq!(state.entities.player.relics[0].counter, 0);
}

#[test]
fn sundial_counter_mutates_immediately_and_grants_energy_on_third_shuffle() {
    let mut relic = RelicState::new(RelicId::Sundial);
    relic.counter = 1;
    assert!(sundial::on_shuffle(&mut relic).is_empty());
    assert_eq!(relic.counter, 2);

    let actions = sundial::on_shuffle(&mut relic);
    assert_eq!(relic.counter, 0);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::GainEnergy { amount: 2 }
    ));

    relic.counter = -1;
    assert!(sundial::on_shuffle(&mut relic).is_empty());
    assert_eq!(
        relic.counter, 0,
        "Java ++counter from -1 reaches 0 without granting energy"
    );

    let mut state = crate::test_support::blank_test_combat();
    let mut state_relic = RelicState::new(RelicId::Sundial);
    state_relic.counter = 2;
    state.entities.player.add_relic(state_relic);
    assert_eq!(hooks::on_shuffle(&mut state).len(), 1);
    assert_eq!(state.entities.player.relics[0].counter, 0);
}

#[test]
fn mummified_hand_sets_one_eligible_hand_card_cost_to_zero_immediately() {
    let mut state = crate::test_support::blank_test_combat();
    state.zones.hand = vec![
        CombatCard::new(CardId::Defend, 1),
        {
            let mut zero_cost = CombatCard::new(CardId::Strike, 2);
            zero_cost.cost_for_turn = Some(0);
            zero_cost
        },
        {
            let mut free_once = CombatCard::new(CardId::Bash, 3);
            free_once.free_to_play_once = true;
            free_once
        },
    ];

    let power = CombatCard::new(CardId::Inflame, 99);
    let actions = mummified_hand::on_use_card(&power, &mut state);
    assert!(actions.is_empty());
    assert_eq!(state.zones.hand[0].cost_for_turn, Some(0));
    assert_eq!(state.zones.hand[1].cost_for_turn, Some(0));
    assert_eq!(state.zones.hand[2].cost_for_turn, None);

    state.zones.hand[0].cost_for_turn = None;
    let attack = CombatCard::new(CardId::Strike, 100);
    assert!(mummified_hand::on_use_card(&attack, &mut state).is_empty());
    assert_eq!(state.zones.hand[0].cost_for_turn, None);
}

#[test]
fn strike_dummy_adds_three_damage_to_strike_tag_attacks_before_power_modifiers() {
    let strike = CombatCard::new(CardId::Strike, 1);
    let pommel = CombatCard::new(CardId::PommelStrike, 2);
    let bash = CombatCard::new(CardId::Bash, 3);

    assert_eq!(
        strike_dummy::modify_attack_damage_for_card(&strike, 6.0),
        9.0
    );
    assert_eq!(
        strike_dummy::modify_attack_damage_for_card(&pommel, 9.0),
        12.0
    );
    assert_eq!(strike_dummy::modify_attack_damage_for_card(&bash, 8.0), 8.0);
}

#[test]
fn matryoshka_counter_starts_at_two_and_only_positive_counter_grants_extra_relic() {
    let relic = RelicState::new(RelicId::Matryoshka);
    assert_eq!(relic.counter, 2);
    assert!(matryoshka::should_grant_extra_relic(2));
    assert!(matryoshka::should_grant_extra_relic(1));
    assert!(!matryoshka::should_grant_extra_relic(0));
    assert!(!matryoshka::should_grant_extra_relic(-2));
}

#[test]
fn shared_uncommon_bottle_shop_and_rest_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::BottledFlame), RelicTier::Uncommon);
    assert_eq!(
        get_relic_tier(RelicId::BottledLightning),
        RelicTier::Uncommon
    );
    assert_eq!(get_relic_tier(RelicId::BottledTornado), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::Courier), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::EternalFeather), RelicTier::Uncommon);
    assert_eq!(get_relic_tier(RelicId::NlothsGift), RelicTier::Special);

    assert!(!get_relic_subscriptions(RelicId::BottledFlame).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::BottledLightning).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::BottledTornado).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::Courier).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::EternalFeather).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::NlothsGift).at_battle_start);
}

#[test]
fn normal_relic_rewards_can_return_bottled_relics_but_screenless_rewards_skip_them() {
    let mut no_non_basic_attack = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    no_non_basic_attack.uncommon_relic_pool = vec![RelicId::Pear, RelicId::BottledFlame];
    assert_eq!(
        no_non_basic_attack.random_relic_by_tier(RelicTier::Uncommon),
        RelicId::Pear,
        "Java Bottled Flame canSpawn rejects starter-only attack decks"
    );

    let mut normal = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    normal
        .master_deck
        .push(CombatCard::new(CardId::PommelStrike, 1001));
    normal.uncommon_relic_pool = vec![RelicId::Pear, RelicId::BottledFlame];
    assert_eq!(
        normal.random_relic_by_tier(RelicTier::Uncommon),
        RelicId::BottledFlame,
        "Java returnRandomRelic can return bottled relics from normal reward paths"
    );

    let mut screenless = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    screenless
        .master_deck
        .push(CombatCard::new(CardId::PommelStrike, 1001));
    screenless.uncommon_relic_pool = vec![RelicId::Pear, RelicId::BottledFlame];
    assert_eq!(
        screenless.random_screenless_relic(RelicTier::Uncommon),
        RelicId::Pear,
        "Java returnRandomScreenlessRelic skips Bottled Flame/Lightning/Tornado and Whetstone"
    );
}

#[test]
fn shared_rare_start_and_turn_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::CaptainsWheel), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::ClockworkSouvenir), RelicTier::Shop);
    assert_eq!(get_relic_tier(RelicId::FossilizedHelix), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::IncenseBurner), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::Pocketwatch), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::StoneCalendar), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::ThreadAndNeedle), RelicTier::Rare);

    let captains = get_relic_subscriptions(RelicId::CaptainsWheel);
    assert!(captains.at_battle_start);
    assert!(captains.at_turn_start);
    assert!(captains.on_victory);

    assert!(get_relic_subscriptions(RelicId::ClockworkSouvenir).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::FossilizedHelix).at_battle_start);
    assert!(get_relic_subscriptions(RelicId::IncenseBurner).at_turn_start);
    assert!(get_relic_subscriptions(RelicId::ThreadAndNeedle).at_battle_start);

    let pocketwatch = get_relic_subscriptions(RelicId::Pocketwatch);
    assert!(pocketwatch.at_battle_start);
    assert!(pocketwatch.at_turn_start_post_draw);
    assert!(pocketwatch.on_use_card);
    assert!(pocketwatch.on_victory);

    let stone_calendar = get_relic_subscriptions(RelicId::StoneCalendar);
    assert!(stone_calendar.at_battle_start);
    assert!(stone_calendar.at_turn_start);
    assert!(stone_calendar.at_end_of_turn);
    assert!(stone_calendar.on_victory);
}

#[test]
fn captains_wheel_mutates_counter_immediately_and_fires_once_on_third_turn() {
    let mut relic = RelicState::new(RelicId::CaptainsWheel);
    captains_wheel::CaptainsWheel::at_battle_start(&mut relic);
    assert_eq!(relic.counter, 0);

    assert!(captains_wheel::CaptainsWheel::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, 1);
    assert!(captains_wheel::CaptainsWheel::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, 2);

    let actions = captains_wheel::CaptainsWheel::at_turn_start(&mut relic);
    assert_eq!(relic.counter, -1, "Java sets counter to -1 after firing");
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::GainBlock {
            target: 0,
            amount: 18
        }
    ));

    assert!(captains_wheel::CaptainsWheel::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, -1);
    captains_wheel::CaptainsWheel::on_victory(&mut relic);
    assert_eq!(relic.counter, -1);
}

#[test]
fn incense_burner_counter_mutates_immediately_and_grants_intangible_on_six() {
    let mut relic = RelicState::new(RelicId::IncenseBurner);
    relic.counter = 4;
    assert!(incense_burner::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, 5);

    let actions = incense_burner::at_turn_start(&mut relic);
    assert_eq!(relic.counter, 0, "Java resets counter to 0 when it fires");
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::IntangiblePlayer,
            amount: 1
        }
    ));

    relic.counter = -1;
    assert!(incense_burner::at_turn_start(&mut relic).is_empty());
    assert_eq!(relic.counter, 1, "Java -1 counter advances to 1, not 0");
}

#[test]
fn hook_persists_mutated_turn_start_relic_counters_before_actions_execute() {
    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::CaptainsWheel));
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::IncenseBurner));
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::StoneCalendar));

    state.entities.player.relics[0].counter = 2;
    state.entities.player.relics[1].counter = 5;
    state.entities.player.relics[2].counter = 6;

    let actions = hooks::at_turn_start(&mut state);
    assert_eq!(state.entities.player.relics[0].counter, -1);
    assert_eq!(state.entities.player.relics[1].counter, 0);
    assert_eq!(state.entities.player.relics[2].counter, 7);
    assert_eq!(actions.len(), 2);
    assert!(actions.iter().any(|info| matches!(
        info.action,
        Action::GainBlock {
            target: 0,
            amount: 18
        }
    )));
    assert!(actions.iter().any(|info| matches!(
        info.action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::IntangiblePlayer,
            amount: 1
        }
    )));
}

#[test]
fn stone_calendar_counter_and_null_source_damage_match_java() {
    let mut relic = RelicState::new(RelicId::StoneCalendar);
    stone_calendar::at_battle_start(&mut relic);
    assert_eq!(relic.counter, 0);

    for expected in 1..=7 {
        assert!(stone_calendar::at_turn_start(&mut relic).is_empty());
        assert_eq!(relic.counter, expected);
    }

    let mut state = crate::test_support::blank_test_combat();
    state.entities.monsters = vec![
        crate::test_support::test_monster(EnemyId::JawWorm),
        crate::test_support::test_monster(EnemyId::Cultist),
    ];
    let actions = stone_calendar::at_end_of_turn(&state, relic.counter);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
    match &actions[0].action {
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            is_modified,
        } => {
            assert_eq!(*source, NO_SOURCE);
            assert_eq!(damages.as_slice(), &[52, 52]);
            assert_eq!(*damage_type, DamageType::Thorns);
            assert!(!is_modified);
        }
        other => panic!("unexpected Stone Calendar action: {other:?}"),
    }

    stone_calendar::on_victory(&mut relic);
    assert_eq!(relic.counter, -1);
}

#[test]
fn start_relic_action_order_matches_java_non_ui_actions() {
    let thread_actions = thread_and_needle::at_battle_start();
    assert_eq!(thread_actions.len(), 1);
    assert_eq!(
        thread_actions[0].insertion_mode,
        AddTo::Top,
        "Java uses addToTop for Thread and Needle PlatedArmor; UI relic action is ignored"
    );
    assert!(matches!(
        thread_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::PlatedArmor,
            amount: 4
        }
    ));

    let clockwork_actions = clockwork_souvenir::ClockworkSouvenir::at_battle_start();
    assert_eq!(clockwork_actions.len(), 1);
    assert_eq!(clockwork_actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        clockwork_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Artifact,
            amount: 1
        }
    ));

    let helix_actions = fossilized_helix::FossilizedHelix::at_battle_start();
    assert_eq!(helix_actions.len(), 1);
    assert_eq!(helix_actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(
        helix_actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Buffer,
            amount: 1
        }
    ));
}

#[test]
fn pocketwatch_first_turn_and_three_card_limit_match_java() {
    let mut relic = RelicState::new(RelicId::Pocketwatch);
    pocketwatch::at_battle_start(&mut relic);
    assert_eq!(relic.counter, 0);
    assert_eq!(relic.amount, 1, "amount stores Java firstTurn=true");

    assert!(pocketwatch::at_turn_start_post_draw(&mut relic).is_empty());
    assert_eq!(relic.counter, 0);
    assert_eq!(relic.amount, 0, "first turn is consumed without drawing");

    for _ in 0..3 {
        pocketwatch::on_use_card(&mut relic);
    }
    let draw_actions = pocketwatch::at_turn_start_post_draw(&mut relic);
    assert_eq!(draw_actions.len(), 1);
    assert_eq!(draw_actions[0].insertion_mode, AddTo::Bottom);
    assert!(matches!(draw_actions[0].action, Action::DrawCards(3)));
    assert_eq!(relic.counter, 0);
    assert_eq!(relic.amount, 0);

    for _ in 0..4 {
        pocketwatch::on_use_card(&mut relic);
    }
    assert!(pocketwatch::at_turn_start_post_draw(&mut relic).is_empty());
    assert_eq!(relic.counter, 0);
    assert_eq!(relic.amount, 0);

    pocketwatch::on_victory(&mut relic);
    assert_eq!(relic.counter, -1);
    assert_eq!(relic.amount, 0);
}

#[test]
fn shared_rare_damage_retention_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::Calipers), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::Torii), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::TungstenRod), RelicTier::Rare);

    let calipers = get_relic_subscriptions(RelicId::Calipers);
    assert!(calipers.on_calculate_block_retained);
    assert!(!calipers.at_battle_start);

    let torii = get_relic_subscriptions(RelicId::Torii);
    assert!(torii.on_attacked_to_change_damage);
    assert!(!torii.on_lose_hp_last);

    let tungsten = get_relic_subscriptions(RelicId::TungstenRod);
    assert!(
        !tungsten.on_lose_hp,
        "Java Tungsten Rod only overrides onLoseHpLast"
    );
    assert!(tungsten.on_lose_hp_last);
}

#[test]
fn calipers_retains_only_block_above_fifteen_without_barricade_logic() {
    assert_eq!(calipers::on_calculate_block_retained(0), 0);
    assert_eq!(calipers::on_calculate_block_retained(14), 0);
    assert_eq!(calipers::on_calculate_block_retained(15), 0);
    assert_eq!(calipers::on_calculate_block_retained(16), 1);
    assert_eq!(calipers::on_calculate_block_retained(40), 25);

    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::Calipers));
    assert_eq!(hooks::on_calculate_block_retained(&state, 40), 25);
}

#[test]
fn torii_requires_real_non_player_owner_and_normal_non_thorns_damage() {
    let owned_normal = DamageInfo {
        source: 901,
        target: 0,
        base: 4,
        output: 4,
        damage_type: DamageType::Normal,
        is_modified: true,
    };
    assert_eq!(torii::on_attacked_to_change_damage(&owned_normal, 4), 1);
    assert_eq!(torii::on_attacked_to_change_damage(&owned_normal, 1), 1);
    assert_eq!(torii::on_attacked_to_change_damage(&owned_normal, 6), 6);

    let player_owned = DamageInfo {
        source: 0,
        ..owned_normal.clone()
    };
    assert_eq!(torii::on_attacked_to_change_damage(&player_owned, 4), 4);

    let no_owner = DamageInfo {
        source: NO_SOURCE,
        ..owned_normal.clone()
    };
    assert_eq!(
        torii::on_attacked_to_change_damage(&no_owner, 4),
        4,
        "Java requires info.owner != null; Rust NO_SOURCE must not trigger Torii"
    );

    let hp_loss = DamageInfo {
        damage_type: DamageType::HpLoss,
        ..owned_normal.clone()
    };
    assert_eq!(torii::on_attacked_to_change_damage(&hp_loss, 4), 4);

    let thorns = DamageInfo {
        damage_type: DamageType::Thorns,
        ..owned_normal
    };
    assert_eq!(torii::on_attacked_to_change_damage(&thorns, 4), 4);
}

#[test]
fn tungsten_rod_is_only_final_hp_loss_modifier() {
    assert_eq!(tungsten_rod::modify_hp_loss(0), 0);
    assert_eq!(tungsten_rod::modify_hp_loss(1), 0);
    assert_eq!(tungsten_rod::modify_hp_loss(5), 4);

    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::TungstenRod));
    assert_eq!(hooks::on_lose_hp_last(&state, 5), 4);
    assert!(
        hooks::on_lose_hp(&mut state, 5).is_empty(),
        "Java Tungsten Rod has no onLoseHp action hook"
    );
}

#[test]
fn shared_rare_passive_resource_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::Ginger), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::Turnip), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::IceCream), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::LizardTail), RelicTier::Rare);

    assert!(get_relic_subscriptions(RelicId::Ginger).on_receive_power_modify);
    assert!(get_relic_subscriptions(RelicId::Turnip).on_receive_power_modify);
    assert!(get_relic_subscriptions(RelicId::IceCream).on_calculate_energy_retained);

    let lizard = get_relic_subscriptions(RelicId::LizardTail);
    assert!(!lizard.on_lose_hp);
    assert!(!lizard.on_lose_hp_last);
    assert!(!lizard.on_calculate_heal);
}

#[test]
fn ginger_and_turnip_block_apply_power_before_artifact_without_blocking_cleanup() {
    let mut state = crate::test_support::blank_test_combat();
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::Ginger));
    store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Artifact,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );

    crate::engine::action_handlers::powers::handle_apply_power(
        901,
        0,
        PowerId::Weak,
        2,
        &mut state,
    );
    let player_powers = store::powers_snapshot_for(&state, 0);
    assert!(
        !player_powers.iter().any(|p| p.power_type == PowerId::Weak),
        "Ginger should block Weakened ApplyPowerAction"
    );
    assert_eq!(
        player_powers
            .iter()
            .find(|p| p.power_type == PowerId::Artifact)
            .map(|p| p.amount),
        Some(1),
        "Java checks Ginger before Artifact, so Artifact is not consumed"
    );

    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::Turnip));
    crate::engine::action_handlers::powers::handle_apply_power(
        901,
        0,
        PowerId::Frail,
        2,
        &mut state,
    );
    assert!(
        !store::powers_snapshot_for(&state, 0)
            .iter()
            .any(|p| p.power_type == PowerId::Frail),
        "Turnip should block Frail ApplyPowerAction"
    );

    let weak_cleanup = crate::content::powers::core::weak::at_end_of_round(0, 2, false);
    assert_eq!(weak_cleanup.len(), 1);
    assert!(matches!(
        weak_cleanup[0],
        Action::ReducePower {
            target: 0,
            power_id: PowerId::Weak,
            amount: 1
        }
    ));

    let frail_cleanup = crate::content::powers::core::frail::at_end_of_round(0, 2, false);
    assert_eq!(frail_cleanup.len(), 1);
    assert!(matches!(
        frail_cleanup[0],
        Action::ReducePower {
            target: 0,
            power_id: PowerId::Frail,
            amount: 1
        }
    ));

    store::set_powers_for(
        &mut state,
        0,
        vec![Power {
            power_type: PowerId::Weak,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            just_applied: false,
        }],
    );
    crate::engine::action_handlers::powers::handle_reduce_power(0, PowerId::Weak, 1, &mut state);
    assert_eq!(
        store::powers_snapshot_for(&state, 0)
            .iter()
            .find(|p| p.power_type == PowerId::Weak)
            .map(|p| p.amount),
        Some(1),
        "Weak/Frail cleanup is ReducePowerAction in Java and must not be blocked by Ginger/Turnip"
    );
}

#[test]
fn ice_cream_recharge_preserves_unspent_energy_before_adding_base_energy() {
    let mut state = crate::test_support::blank_test_combat();
    state.turn.energy = 2;
    state.entities.player.energy_master = 3;
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::IceCream));

    assert!(hooks::on_calculate_energy_retained(&state));
    state.begin_next_player_turn();
    assert_eq!(
        state.turn.energy, 5,
        "Java Ice Cream uses EnergyPanel.addEnergy(base), preserving prior total"
    );
    assert_eq!(state.turn.counters.cards_played_this_turn, 0);
}

#[test]
fn lizard_tail_uses_java_counter_gate_and_fairy_priority() {
    let mut state = crate::test_support::blank_test_combat();
    state.entities.player.current_hp = 0;
    state.entities.player.max_hp = 80;
    state
        .entities
        .player
        .add_relic(RelicState::new(RelicId::LizardTail));

    crate::engine::action_handlers::try_revive(&mut state);
    assert_eq!(state.entities.player.current_hp, 40);
    assert_eq!(state.entities.player.relics[0].counter, -2);
    assert!(state.entities.player.relics[0].used_up);

    let mut counter_used = crate::test_support::blank_test_combat();
    counter_used.entities.player.current_hp = 0;
    let mut used_lizard = RelicState::new(RelicId::LizardTail);
    used_lizard.counter = -2;
    used_lizard.used_up = false;
    counter_used.entities.player.add_relic(used_lizard);
    crate::engine::action_handlers::try_revive(&mut counter_used);
    assert_eq!(
        counter_used.entities.player.current_hp, 0,
        "Java checks LizardTail.counter == -1, not just used_up"
    );

    let mut fairy_first = crate::test_support::blank_test_combat();
    fairy_first.entities.player.current_hp = 0;
    fairy_first.entities.player.max_hp = 80;
    fairy_first
        .entities
        .player
        .add_relic(RelicState::new(RelicId::LizardTail));
    fairy_first.entities.potions = vec![Some(crate::content::potions::Potion::new(
        crate::content::potions::PotionId::FairyPotion,
        7001,
    ))];
    crate::engine::action_handlers::try_revive(&mut fairy_first);
    assert_eq!(fairy_first.entities.player.current_hp, 24);
    assert!(fairy_first.entities.potions[0].is_none());
    assert_eq!(
        fairy_first.entities.player.relics[0].counter, -1,
        "Java consumes Fairy before Lizard Tail"
    );
    assert!(!fairy_first.entities.player.relics[0].used_up);

    let mut bloom_blocks = crate::test_support::blank_test_combat();
    bloom_blocks.entities.player.current_hp = 0;
    bloom_blocks
        .entities
        .player
        .add_relic(RelicState::new(RelicId::MarkOfTheBloom));
    bloom_blocks
        .entities
        .player
        .add_relic(RelicState::new(RelicId::LizardTail));
    bloom_blocks.entities.potions = vec![Some(crate::content::potions::Potion::new(
        crate::content::potions::PotionId::FairyPotion,
        7002,
    ))];
    crate::engine::action_handlers::try_revive(&mut bloom_blocks);
    assert_eq!(bloom_blocks.entities.player.current_hp, 0);
    assert!(
        bloom_blocks.entities.potions[0].is_some(),
        "Mark of the Bloom blocks Fairy and Lizard revive before either is consumed"
    );
    assert_eq!(bloom_blocks.entities.player.relics[1].counter, -1);
}

#[test]
fn shared_rare_run_campfire_relic_metadata_matches_java_sources() {
    assert_eq!(get_relic_tier(RelicId::Mango), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::OldCoin), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::Girya), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::PeacePipe), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::PrayerWheel), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::Shovel), RelicTier::Rare);
    assert_eq!(get_relic_tier(RelicId::WingBoots), RelicTier::Rare);

    assert_eq!(RelicState::new(RelicId::Girya).counter, 0);
    assert_eq!(RelicState::new(RelicId::WingBoots).counter, 3);

    assert!(get_relic_subscriptions(RelicId::Girya).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::Mango).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::OldCoin).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::PeacePipe).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::PrayerWheel).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::Shovel).at_battle_start);
    assert!(!get_relic_subscriptions(RelicId::WingBoots).at_battle_start);
}

#[test]
fn mango_and_old_coin_on_equip_match_java_resource_changes() {
    let mut mango_run = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    mango_run.max_hp = 80;
    mango_run.current_hp = 40;
    assert!(mango::on_equip(&mut mango_run).is_none());
    assert_eq!(mango_run.max_hp, 94);
    assert_eq!(mango_run.current_hp, 54);

    let mut old_coin_run = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    old_coin_run.gold = 25;
    assert!(old_coin::on_equip(&mut old_coin_run).is_none());
    assert_eq!(old_coin_run.gold, 325);

    let mut ectoplasm_run = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    ectoplasm_run.gold = 25;
    ectoplasm_run
        .relics
        .push(RelicState::new(RelicId::Ectoplasm));
    assert!(old_coin::on_equip(&mut ectoplasm_run).is_none());
    assert_eq!(
        ectoplasm_run.gold, 25,
        "Java Old Coin uses player.gainGold, so Ectoplasm blocks it"
    );
}

#[test]
fn rare_run_relic_can_spawn_gates_match_java_sources() {
    fn rare_spawn_result(
        blocked_candidate: RelicId,
        floor_num: i32,
        owned_relics: Vec<RelicState>,
    ) -> RelicId {
        let mut run = crate::state::run::RunState::new(1, 0, false, "Ironclad");
        run.floor_num = floor_num;
        run.relics = owned_relics;
        run.rare_relic_pool = vec![RelicId::Mango, blocked_candidate];
        run.random_relic_by_tier(RelicTier::Rare)
    }

    assert_eq!(
        rare_spawn_result(RelicId::OldCoin, 48, vec![]),
        RelicId::OldCoin
    );
    assert_eq!(
        rare_spawn_result(RelicId::OldCoin, 49, vec![]),
        RelicId::Mango
    );
    assert_eq!(
        rare_spawn_result(RelicId::PrayerWheel, 49, vec![]),
        RelicId::Mango
    );
    assert_eq!(
        rare_spawn_result(RelicId::WingBoots, 40, vec![]),
        RelicId::WingBoots
    );
    assert_eq!(
        rare_spawn_result(RelicId::WingBoots, 41, vec![]),
        RelicId::Mango
    );

    assert_eq!(
        rare_spawn_result(RelicId::Girya, 48, vec![]),
        RelicId::Mango,
        "Java Girya/PeacePipe/Shovel reject floorNum >= 48"
    );
    assert_eq!(
        rare_spawn_result(
            RelicId::Girya,
            47,
            vec![
                RelicState::new(RelicId::PeacePipe),
                RelicState::new(RelicId::Shovel)
            ],
        ),
        RelicId::Mango,
        "Java campfire relics reject when two of Girya/Peace Pipe/Shovel are already owned"
    );
    assert_eq!(
        rare_spawn_result(
            RelicId::Girya,
            47,
            vec![RelicState::new(RelicId::PeacePipe)]
        ),
        RelicId::Girya
    );
    assert_eq!(
        rare_spawn_result(
            RelicId::PeacePipe,
            47,
            vec![
                RelicState::new(RelicId::Girya),
                RelicState::new(RelicId::Shovel)
            ],
        ),
        RelicId::Mango
    );
    assert_eq!(
        rare_spawn_result(
            RelicId::Shovel,
            47,
            vec![
                RelicState::new(RelicId::Girya),
                RelicState::new(RelicId::PeacePipe)
            ],
        ),
        RelicId::Mango
    );
}

#[test]
fn girya_lift_counter_and_battle_start_strength_match_java() {
    let mut engine_state = crate::state::core::EngineState::Campfire;
    let mut run = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    run.relics.clear();
    run.relics.push(RelicState::new(RelicId::Girya));

    assert!(crate::engine::campfire_handler::handle(
        &mut engine_state,
        &mut run,
        Some(crate::state::core::ClientInput::CampfireOption(
            crate::state::core::CampfireChoice::Lift,
        )),
    ));
    assert_eq!(run.relics[0].counter, 1);

    let actions = girya::Girya::at_battle_start(run.relics[0].counter);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].insertion_mode, AddTo::Top);
    assert!(matches!(
        actions[0].action,
        Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount: 1
        }
    ));

    run.relics[0].counter = 3;
    let options = crate::engine::campfire_handler::get_available_options(&run);
    assert!(
        !options.contains(&crate::state::core::CampfireChoice::Lift),
        "Java LiftOption is disabled once Girya counter reaches 3"
    );
}

#[test]
fn prayer_wheel_adds_second_non_boss_card_reward() {
    let mut normal = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    normal.relics.clear();
    normal.relics.push(RelicState::new(RelicId::PrayerWheel));
    let normal_rewards =
        crate::rewards::generator::generate_combat_rewards(&mut normal, false, false);
    assert_eq!(
        normal_rewards
            .items
            .iter()
            .filter(|item| matches!(item, crate::rewards::state::RewardItem::Card { .. }))
            .count(),
        2
    );

    let mut boss = crate::state::run::RunState::new(1, 0, false, "Ironclad");
    boss.relics.clear();
    boss.relics.push(RelicState::new(RelicId::PrayerWheel));
    let boss_rewards = crate::rewards::generator::generate_combat_rewards(&mut boss, false, true);
    assert_eq!(
        boss_rewards
            .items
            .iter()
            .filter(|item| matches!(item, crate::rewards::state::RewardItem::Card { .. }))
            .count(),
        1,
        "Java Prayer Wheel does not add an extra boss card reward"
    );
}
