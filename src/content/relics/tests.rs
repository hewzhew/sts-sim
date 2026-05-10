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
