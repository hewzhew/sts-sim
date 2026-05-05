use sts_simulator::content::monsters::beyond::awakened_one::AwakenedOne;
use sts_simulator::content::monsters::beyond::deca::Deca;
use sts_simulator::content::monsters::beyond::donu::Donu;
use sts_simulator::content::monsters::beyond::exploder::Exploder;
use sts_simulator::content::monsters::beyond::giant_head::GiantHead;
use sts_simulator::content::monsters::beyond::maw::Maw;
use sts_simulator::content::monsters::beyond::nemesis::Nemesis;
use sts_simulator::content::monsters::beyond::orb_walker::OrbWalker;
use sts_simulator::content::monsters::beyond::reptomancer::Reptomancer;
use sts_simulator::content::monsters::beyond::repulsor::Repulsor;
use sts_simulator::content::monsters::beyond::snake_dagger::SnakeDagger;
use sts_simulator::content::monsters::beyond::spiker::Spiker;
use sts_simulator::content::monsters::beyond::spire_growth::SpireGrowth;
use sts_simulator::content::monsters::beyond::time_eater::TimeEater;
use sts_simulator::content::monsters::beyond::transient::Transient;
use sts_simulator::content::monsters::beyond::writhing_mass::WrithingMass;
use sts_simulator::content::monsters::city::bandit_bear::BanditBear;
use sts_simulator::content::monsters::city::bandit_leader::BanditLeader;
use sts_simulator::content::monsters::city::bandit_pointy::BanditPointy;
use sts_simulator::content::monsters::city::book_of_stabbing::BookOfStabbing;
use sts_simulator::content::monsters::city::bronze_automaton::BronzeAutomaton;
use sts_simulator::content::monsters::city::bronze_orb::BronzeOrb;
use sts_simulator::content::monsters::city::byrd::Byrd;
use sts_simulator::content::monsters::city::centurion::Centurion;
use sts_simulator::content::monsters::city::champ::Champ;
use sts_simulator::content::monsters::city::chosen::Chosen;
use sts_simulator::content::monsters::city::gremlin_leader::GremlinLeader;
use sts_simulator::content::monsters::city::healer::Healer;
use sts_simulator::content::monsters::city::mugger::Mugger;
use sts_simulator::content::monsters::city::shelled_parasite::ShelledParasite;
use sts_simulator::content::monsters::city::snake_plant::SnakePlant;
use sts_simulator::content::monsters::city::snecko::Snecko;
use sts_simulator::content::monsters::city::spheric_guardian::SphericGuardian;
use sts_simulator::content::monsters::city::taskmaster::Taskmaster;
use sts_simulator::content::monsters::city::the_collector::TheCollector;
use sts_simulator::content::monsters::city::torch_head::TorchHead;
use sts_simulator::content::monsters::ending::corrupt_heart::CorruptHeart;
use sts_simulator::content::monsters::ending::spire_shield::SpireShield;
use sts_simulator::content::monsters::ending::spire_spear::SpireSpear;
use sts_simulator::content::monsters::exordium::cultist::Cultist;
use sts_simulator::content::monsters::factory::{build_encounter, EncounterId};
use sts_simulator::content::monsters::{
    resolve_monster_turn, resolve_monster_turn_plan, resolve_on_death, resolve_pre_battle_actions,
    resolve_roll_move_actions, roll_monster_turn_outcome, roll_monster_turn_plan, EnemyId,
    MonsterBehavior, MonsterRollContext, PreBattleLegacyRng,
};
use sts_simulator::content::powers::PowerId;
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::runtime::action::Action;
use sts_simulator::test_support::{
    blank_test_combat, combat_with_monsters, monster_with_history, planned_monster, test_monster,
};

#[test]
fn cultist_turn_plan_reconstructs_ritual_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Cultist, 3);

    let plan = Cultist::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Buff(BuffSpec { power_id: Ritual, amount: 3 })"
    );
}

#[test]
fn cultist_resolver_uses_semantic_turn_plan() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Cultist, 3);

    let plan = resolve_monster_turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Buff(BuffSpec { power_id: Ritual, amount: 3 })"
    );
}

#[test]
fn spheric_guardian_turn_plan_reconstructs_harden_from_move_id() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::SphericGuardian, 2);
    entity.block = 40;

    let plan = SphericGuardian::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Defend(DefendSpec { block: 25 })"
    );
}

#[test]
fn spheric_guardian_resolver_uses_semantic_turn_plan() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::SphericGuardian, 2);
    entity.block = 40;

    let plan = resolve_monster_turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Defend(DefendSpec { block: 25 })"
    );
}

#[test]
fn byrd_turn_plan_reconstructs_stunned_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Byrd, 4);

    let plan = Byrd::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 4);
    assert_eq!(format!("{:?}", plan.summary_spec()), "Stun");
}

#[test]
fn byrd_grounded_roll_plan_becomes_headbutt() {
    let state = blank_test_combat();
    let mut entity = test_monster(EnemyId::Byrd);
    entity.byrd.first_move = false;
    entity.byrd.is_flying = false;
    let mut rng = state.rng.ai_rng.clone();

    let plan = Byrd::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 5);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 3, hits: 1, damage_kind: Normal })"
    );
}

#[test]
fn byrd_headbutt_sets_go_airborne_without_extra_roll() {
    let mut entity = planned_monster(EnemyId::Byrd, 5);
    entity.byrd.first_move = false;
    entity.byrd.is_flying = false;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Byrd::turn_plan(&state, &entity);

    let actions = Byrd::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 3);
    assert!(matches!(
        actions[0],
        Action::MonsterAttack { base_damage: 3, .. }
    ));
    execute_action(actions[1].clone(), &mut state);
    assert!(!state.entities.monsters[0].byrd.first_move);
    assert!(!state.entities.monsters[0].byrd.is_flying);
    assert!(matches!(
        actions[2],
        Action::SetMonsterMove {
            next_move_byte: 2,
            ..
        }
    ));
}

#[test]
fn byrd_go_airborne_restores_flying_runtime_state_before_roll() {
    let mut entity = planned_monster(EnemyId::Byrd, 2);
    entity.byrd.first_move = false;
    entity.byrd.is_flying = false;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Byrd::turn_plan(&state, &entity);

    let actions = Byrd::take_turn_plan(&mut state, &entity, &plan);

    execute_action(actions[1].clone(), &mut state);
    assert!(state.entities.monsters[0].byrd.protocol_seeded);
    assert!(state.entities.monsters[0].byrd.is_flying);
    assert!(!state.entities.monsters[0].byrd.first_move);
}

#[test]
fn byrd_resolve_pre_battle_action_applies_flight() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::Byrd);
    let mut state = state;
    let actions = resolve_pre_battle_actions(
        &mut state,
        EnemyId::Byrd,
        &entity,
        PreBattleLegacyRng::MonsterHp,
    );

    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        Action::ApplyPower {
            power_id: PowerId::Flight,
            amount: 3,
            ..
        }
    ));
}

#[test]
fn byrd_resolve_on_death_uses_semantic_noop_dispatch() {
    let mut state = blank_test_combat();
    let entity = test_monster(EnemyId::Byrd);

    let actions = resolve_on_death(EnemyId::Byrd, &mut state, &entity);

    assert!(actions.is_empty());
}

#[test]
fn act2_hallway_semantic_pre_battle_dispatches_do_not_panic() {
    let mut state = blank_test_combat();
    let enemy_ids = [
        EnemyId::SphericGuardian,
        EnemyId::Byrd,
        EnemyId::Chosen,
        EnemyId::Snecko,
        EnemyId::Centurion,
        EnemyId::Healer,
        EnemyId::SnakePlant,
        EnemyId::ShelledParasite,
        EnemyId::Mugger,
    ];

    for enemy_id in enemy_ids {
        let entity = test_monster(enemy_id);
        let _ = resolve_pre_battle_actions(
            &mut state,
            enemy_id,
            &entity,
            PreBattleLegacyRng::MonsterHp,
        );
    }
}

#[test]
fn act2_hallway_semantic_on_death_dispatches_do_not_panic() {
    let enemy_ids = [
        EnemyId::SphericGuardian,
        EnemyId::Byrd,
        EnemyId::Chosen,
        EnemyId::Snecko,
        EnemyId::Centurion,
        EnemyId::Healer,
        EnemyId::SnakePlant,
        EnemyId::ShelledParasite,
        EnemyId::Mugger,
    ];

    for enemy_id in enemy_ids {
        let mut state = blank_test_combat();
        let entity = test_monster(enemy_id);
        let _ = resolve_on_death(enemy_id, &mut state, &entity);
    }
}

#[test]
fn act2_encounter_roster_semantic_dispatches_do_not_panic() {
    let encounters = [
        (EncounterId::TwoThieves, false, false),
        (EncounterId::ThreeByrds, false, false),
        (EncounterId::ChosenAlone, false, false),
        (EncounterId::ShellParasite, false, false),
        (EncounterId::SphericGuardian, false, false),
        (EncounterId::ChosenAndByrds, false, false),
        (EncounterId::SentryAndSphere, false, false),
        (EncounterId::SnakePlant, false, false),
        (EncounterId::Snecko, false, false),
        (EncounterId::CenturionAndHealer, false, false),
        (EncounterId::CultistAndChosen, false, false),
        (EncounterId::ThreeCultists, false, false),
        (EncounterId::ShelledParasiteAndFungi, false, false),
        (EncounterId::GremlinLeader, true, false),
        (EncounterId::Slavers, true, false),
        (EncounterId::BookOfStabbing, true, false),
        (EncounterId::Automaton, false, true),
        (EncounterId::TheChamp, false, true),
        (EncounterId::Collector, false, true),
        (EncounterId::MaskedBandits, false, false),
        (EncounterId::ColosseumSlavers, false, false),
        (EncounterId::ColosseumNobs, false, false),
    ];

    for (encounter, is_elite, is_boss) in encounters {
        let mut state = blank_test_combat();
        state.meta.is_elite_fight = is_elite;
        state.meta.is_boss_fight = is_boss;
        let mut misc_rng = state.rng.misc_rng.clone();
        let mut monster_hp_rng = state.rng.monster_hp_rng.clone();
        let monsters = build_encounter(
            encounter,
            &mut misc_rng,
            &mut monster_hp_rng,
            state.meta.ascension_level,
        );
        state.entities.monsters = monsters;

        let pre_battle_monsters = state.entities.monsters.clone();
        for monster in pre_battle_monsters {
            let enemy_id =
                EnemyId::from_id(monster.monster_type).expect("encounter monster should map");
            let actions = resolve_pre_battle_actions(
                &mut state,
                enemy_id,
                &monster,
                PreBattleLegacyRng::MonsterHp,
            );
            for action in actions {
                execute_action(action, &mut state);
            }
        }

        let rolled_monsters = state.entities.monsters.clone();
        for monster in rolled_monsters {
            let enemy_id =
                EnemyId::from_id(monster.monster_type).expect("encounter monster should map");
            let mut ai_rng = state.rng.ai_rng.clone();
            let num = 50;
            let outcome = roll_monster_turn_outcome(
                &mut ai_rng,
                &monster,
                state.meta.ascension_level,
                num,
                &state.entities.monsters,
                &sts_simulator::content::powers::store::powers_snapshot_for(&state, 0),
            );
            for action in outcome.setup_actions {
                execute_action(action, &mut state);
            }
            let plan = outcome.plan;

            let mut planned = state
                .entities
                .monsters
                .iter()
                .find(|candidate| candidate.id == monster.id)
                .cloned()
                .expect("rolled monster should still exist");
            planned.set_planned_move_id(plan.move_id);

            let _ = resolve_monster_turn_plan(&state, &planned);
            let _ = resolve_monster_turn(&mut state, &planned);
            let _ = resolve_on_death(enemy_id, &mut state, &planned);
        }
    }
}

#[test]
fn bandit_bear_turn_plan_reconstructs_bear_hug_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::BanditBear, 2);

    let plan = BanditBear::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Dexterity, amount: -2, strength: Strong })"
    );
}

#[test]
fn bandit_bear_take_turn_bear_hug_sets_lunge_without_roll() {
    let entity = planned_monster(EnemyId::BanditBear, 2);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = BanditBear::turn_plan(&state, &entity);

    let actions = BanditBear::take_turn_plan(&mut state, &entity, &plan);

    assert!(actions
        .iter()
        .all(|action| !matches!(action, Action::RollMonsterMove { .. })));
    assert!(matches!(
        actions.last(),
        Some(Action::SetMonsterMove {
            next_move_byte: 3,
            ..
        })
    ));
}

#[test]
fn bandit_leader_turn_plan_reconstructs_agonizing_slash_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::BanditLeader, 3);

    let plan = BanditLeader::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackDebuff(AttackSpec { base_damage: 10, hits: 1, damage_kind: Normal }, DebuffSpec { power_id: Weak, amount: 2, strength: Normal })"
    );
}

#[test]
fn bandit_leader_a17_cross_slash_can_repeat_once() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::BanditLeader, 1);
    entity.move_history_mut().clear();
    entity.move_history_mut().push_back(1);
    let mut rng = state.rng.ai_rng.clone();

    let plan = BanditLeader::roll_move_plan(&mut rng, &entity, 17, 0);

    assert_eq!(plan.move_id, 1);
}

#[test]
fn bandit_leader_take_turn_mock_sets_agonizing_slash_without_roll() {
    let entity = planned_monster(EnemyId::BanditLeader, 2);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = BanditLeader::turn_plan(&state, &entity);

    let actions = BanditLeader::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        Action::SetMonsterMove {
            next_move_byte: 3,
            ..
        }
    ));
}

#[test]
fn bandit_pointy_turn_plan_reconstructs_double_attack_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::BanditPointy, 1);

    let plan = BanditPointy::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 5, hits: 2, damage_kind: Normal })"
    );
}

#[test]
fn bandit_pointy_take_turn_keeps_same_move_without_roll() {
    let entity = planned_monster(EnemyId::BanditPointy, 1);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = BanditPointy::turn_plan(&state, &entity);

    let actions = BanditPointy::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 3);
    assert!(actions
        .iter()
        .all(|action| !matches!(action, Action::RollMonsterMove { .. })));
    assert!(matches!(
        actions.last(),
        Some(Action::SetMonsterMove {
            next_move_byte: 1,
            ..
        })
    ));
}

#[test]
fn bandit_event_semantic_pre_battle_dispatches_do_not_panic() {
    let mut state = blank_test_combat();
    let enemy_ids = [
        EnemyId::BanditBear,
        EnemyId::BanditLeader,
        EnemyId::BanditPointy,
    ];

    for enemy_id in enemy_ids {
        let entity = test_monster(enemy_id);
        let _ = resolve_pre_battle_actions(
            &mut state,
            enemy_id,
            &entity,
            PreBattleLegacyRng::MonsterHp,
        );
    }
}

#[test]
fn bandit_event_semantic_on_death_dispatches_do_not_panic() {
    let enemy_ids = [
        EnemyId::BanditBear,
        EnemyId::BanditLeader,
        EnemyId::BanditPointy,
    ];

    for enemy_id in enemy_ids {
        let mut state = blank_test_combat();
        let entity = test_monster(enemy_id);
        let _ = resolve_on_death(enemy_id, &mut state, &entity);
    }
}

#[test]
fn chosen_turn_plan_reconstructs_hex_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Chosen, 4);

    let plan = Chosen::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 4);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Hex, amount: 1, strength: Strong })"
    );
}

#[test]
fn chosen_roll_move_plan_first_turn_is_poke_when_seeded_non_a17() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::Chosen);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Chosen::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 5);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 5, hits: 2, damage_kind: Normal })"
    );
}

#[test]
fn chosen_roll_move_plan_uses_seeded_hex_state() {
    let state = blank_test_combat();
    let mut entity = test_monster(EnemyId::Chosen);
    entity.chosen.first_turn = false;
    entity.chosen.used_hex = false;
    let mut rng = state.rng.ai_rng.clone();

    let plan = Chosen::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 4);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Hex, amount: 1, strength: Strong })"
    );
}

#[test]
fn chosen_take_turn_poke_updates_runtime_state_before_roll() {
    let mut entity = planned_monster(EnemyId::Chosen, 5);
    entity.chosen.first_turn = true;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Chosen::turn_plan(&state, &entity);

    let actions = Chosen::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 4);
    assert!(matches!(
        actions[0],
        Action::MonsterAttack { base_damage: 5, .. }
    ));
    assert!(matches!(
        actions[1],
        Action::MonsterAttack { base_damage: 5, .. }
    ));
    execute_action(actions[2].clone(), &mut state);
    assert!(!state.entities.monsters[0].chosen.first_turn);
    assert!(state.entities.monsters[0].chosen.protocol_seeded);
    assert!(matches!(actions[3], Action::RollMonsterMove { .. }));
}

#[test]
fn chosen_take_turn_hex_marks_used_hex_before_roll() {
    let mut entity = planned_monster(EnemyId::Chosen, 4);
    entity.chosen.first_turn = false;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Chosen::turn_plan(&state, &entity);

    let actions = Chosen::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 3);
    execute_action(actions[1].clone(), &mut state);
    assert!(state.entities.monsters[0].chosen.used_hex);
    assert!(state.entities.monsters[0].chosen.protocol_seeded);
    assert!(matches!(actions[2], Action::RollMonsterMove { .. }));
}

#[test]
fn snecko_turn_plan_reconstructs_glare_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Snecko, 1);

    let plan = Snecko::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Confusion, amount: 1, strength: Strong })"
    );
}

#[test]
fn snecko_roll_move_plan_first_turn_is_glare() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::Snecko);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Snecko::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Confusion, amount: 1, strength: Strong })"
    );
}

#[test]
fn snecko_roll_move_plan_avoids_third_bite() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::Snecko, 2);
    entity.snecko.first_turn = false;
    entity.move_history_mut().clear();
    entity.move_history_mut().extend([2, 2]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Snecko::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackDebuff(AttackSpec { base_damage: 8, hits: 1, damage_kind: Normal }, DebuffSpec { power_id: Vulnerable, amount: 2, strength: Normal })"
    );
}

#[test]
fn snecko_take_turn_glare_updates_runtime_state_before_roll() {
    let entity = planned_monster(EnemyId::Snecko, 1);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Snecko::turn_plan(&state, &entity);

    let actions = Snecko::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 3);
    execute_action(actions[1].clone(), &mut state);
    assert!(!state.entities.monsters[0].snecko.first_turn);
    assert!(state.entities.monsters[0].snecko.protocol_seeded);
    assert!(matches!(actions[2], Action::RollMonsterMove { .. }));
}

#[test]
fn mugger_turn_plan_reconstructs_smoke_bomb_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Mugger, 2);

    let plan = Mugger::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Defend(DefendSpec { block: 11 })"
    );
}

#[test]
fn mugger_second_mug_sets_non_mug_followup_without_extra_roll() {
    let mut entity = planned_monster(EnemyId::Mugger, 1);
    entity.thief.slash_count = 1;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Mugger::turn_plan(&state, &entity);

    let actions = Mugger::take_turn_plan(&mut state, &entity, &plan);

    assert!(actions
        .iter()
        .all(|action| !matches!(action, Action::RollMonsterMove { .. })));
    match actions.last() {
        Some(Action::SetMonsterMove {
            next_move_byte: 2 | 4,
            ..
        }) => {}
        other => panic!("expected smoke bomb or big swipe followup, got {other:?}"),
    }
}

#[test]
fn mugger_big_swipe_sets_smoke_bomb_without_extra_roll() {
    let entity = planned_monster(EnemyId::Mugger, 4);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Mugger::turn_plan(&state, &entity);

    let actions = Mugger::take_turn_plan(&mut state, &entity, &plan);

    assert!(actions
        .iter()
        .all(|action| !matches!(action, Action::RollMonsterMove { .. })));
    assert!(matches!(
        actions.last(),
        Some(Action::SetMonsterMove {
            next_move_byte: 2,
            ..
        })
    ));
}

#[test]
fn mugger_on_death_returns_stolen_gold_reward() {
    let mut entity = test_monster(EnemyId::Mugger);
    entity.thief.stolen_gold = 30;
    let mut state = blank_test_combat();

    let actions = Mugger::on_death(&mut state, &entity);

    assert!(matches!(
        actions.as_slice(),
        [Action::AddCombatReward { .. }]
    ));
    assert!(format!("{:?}", actions[0]).contains("StolenGold { amount: 30 }"));
}

#[test]
fn shelled_parasite_turn_plan_reconstructs_life_suck_from_move_id() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::ShelledParasite, 3);
    entity.shelled_parasite.first_move = false;

    let plan = ShelledParasite::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackSustain(AttackSpec { base_damage: 10, hits: 1, damage_kind: Normal })"
    );
}

#[test]
fn shelled_parasite_roll_move_plan_first_move_is_fell_at_a17() {
    let mut state = blank_test_combat();
    state.meta.ascension_level = 17;
    let entity = test_monster(EnemyId::ShelledParasite);
    let mut rng = state.rng.ai_rng.clone();

    let plan = ShelledParasite::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackDebuff(AttackSpec { base_damage: 21, hits: 1, damage_kind: Normal }, DebuffSpec { power_id: Frail, amount: 2, strength: Normal })"
    );
}

#[test]
fn shelled_parasite_fell_updates_runtime_before_roll() {
    let mut entity = planned_monster(EnemyId::ShelledParasite, 1);
    entity.shelled_parasite.first_move = true;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = ShelledParasite::turn_plan(&state, &entity);

    let actions = ShelledParasite::take_turn_plan(&mut state, &entity, &plan);

    execute_action(actions[2].clone(), &mut state);
    assert!(!state.entities.monsters[0].shelled_parasite.first_move);
    assert!(state.entities.monsters[0].shelled_parasite.protocol_seeded);
    assert!(matches!(actions[3], Action::RollMonsterMove { .. }));
}

#[test]
fn shelled_parasite_stunned_sets_fell_without_extra_roll() {
    let entity = planned_monster(EnemyId::ShelledParasite, 4);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = ShelledParasite::turn_plan(&state, &entity);

    let actions = ShelledParasite::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        Action::SetMonsterMove {
            next_move_byte: 1,
            ..
        }
    ));
}

#[test]
fn centurion_turn_plan_reconstructs_protect_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Centurion, 2);

    let plan = Centurion::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Defend(DefendSpec { block: 15 })"
    );
}

#[test]
fn centurion_roll_move_high_roll_with_ally_becomes_protect() {
    let state = combat_with_monsters(vec![
        test_monster(EnemyId::Centurion),
        test_monster(EnemyId::Cultist),
    ]);
    let entity = state.entities.monsters[0].clone();
    let mut rng = state.rng.ai_rng.clone();

    let plan = Centurion::roll_move_plan_with_context(
        &mut rng,
        &entity,
        state.meta.ascension_level,
        80,
        MonsterRollContext {
            monsters: &state.entities.monsters,
            player_powers: &[],
        },
    );

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Defend(DefendSpec { block: 15 })"
    );
}

#[test]
fn centurion_roll_move_high_roll_solo_becomes_fury() {
    let state = combat_with_monsters(vec![test_monster(EnemyId::Centurion)]);
    let entity = state.entities.monsters[0].clone();
    let mut rng = state.rng.ai_rng.clone();

    let plan = Centurion::roll_move_plan_with_context(
        &mut rng,
        &entity,
        state.meta.ascension_level,
        80,
        MonsterRollContext {
            monsters: &state.entities.monsters,
            player_powers: &[],
        },
    );

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 6, hits: 3, damage_kind: Normal })"
    );
}

#[test]
fn centurion_take_turn_protect_keeps_roll_move() {
    let entity = planned_monster(EnemyId::Centurion, 2);
    let mut state = combat_with_monsters(vec![entity.clone(), test_monster(EnemyId::Cultist)]);
    let plan = Centurion::turn_plan(&state, &entity);

    let actions = Centurion::take_turn_plan(&mut state, &entity, &plan);

    assert!(matches!(
        actions.as_slice(),
        [
            Action::GainBlockRandomMonster { amount: 15, .. },
            Action::RollMonsterMove { .. }
        ]
    ));
}

#[test]
fn healer_turn_plan_reconstructs_heal_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Healer, 2);

    let plan = Healer::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Heal(HealSpec { target: AllMonsters, amount: 16 })"
    );
}

#[test]
fn healer_roll_move_prefers_heal_when_party_missing_hp() {
    let mut healer = test_monster(EnemyId::Healer);
    healer.id = 1;
    let mut ally = test_monster(EnemyId::Cultist);
    ally.id = 2;
    ally.current_hp = 1;
    ally.max_hp = 20;
    let state = combat_with_monsters(vec![healer.clone(), ally]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Healer::roll_move_plan_with_context(
        &mut rng,
        &healer,
        state.meta.ascension_level,
        99,
        MonsterRollContext {
            monsters: &state.entities.monsters,
            player_powers: &[],
        },
    );

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Heal(HealSpec { target: AllMonsters, amount: 16 })"
    );
}

#[test]
fn healer_a17_attack_checks_only_last_move() {
    let mut state = blank_test_combat();
    state.meta.ascension_level = 17;
    let entity = monster_with_history(EnemyId::Healer, 3, &[1, 3, 1]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Healer::roll_move_plan_with_context(
        &mut rng,
        &entity,
        state.meta.ascension_level,
        80,
        MonsterRollContext {
            monsters: &[entity.clone()],
            player_powers: &[],
        },
    );

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Buff(BuffSpec { power_id: Strength, amount: 4 })"
    );
}

#[test]
fn healer_non_a17_attack_checks_last_two_moves() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::Healer, 3, &[1, 3, 1]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Healer::roll_move_plan_with_context(
        &mut rng,
        &entity,
        state.meta.ascension_level,
        80,
        MonsterRollContext {
            monsters: &[entity.clone()],
            player_powers: &[],
        },
    );

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackDebuff(AttackSpec { base_damage: 8, hits: 1, damage_kind: Normal }, DebuffSpec { power_id: Frail, amount: 2, strength: Normal })"
    );
}

#[test]
fn healer_take_turn_heals_all_living_monsters() {
    let mut healer = planned_monster(EnemyId::Healer, 2);
    healer.id = 1;
    healer.current_hp = 10;
    healer.max_hp = 20;
    let mut ally = test_monster(EnemyId::Cultist);
    ally.id = 2;
    ally.current_hp = 5;
    ally.max_hp = 20;
    let mut dead = test_monster(EnemyId::Cultist);
    dead.id = 3;
    dead.current_hp = 0;
    dead.max_hp = 20;
    dead.is_dying = true;
    let mut state = combat_with_monsters(vec![healer.clone(), ally, dead]);
    let plan = Healer::turn_plan(&state, &healer);

    let actions = Healer::take_turn_plan(&mut state, &healer, &plan);

    assert!(matches!(
        actions.last(),
        Some(Action::RollMonsterMove { .. })
    ));
    for action in actions.iter().take(actions.len() - 1).cloned() {
        execute_action(action, &mut state);
    }
    assert_eq!(state.entities.monsters[0].current_hp, 20);
    assert_eq!(state.entities.monsters[1].current_hp, 20);
    assert_eq!(state.entities.monsters[2].current_hp, 0);
}

#[test]
fn healer_take_turn_buff_applies_strength_to_all_living_monsters() {
    let mut healer = planned_monster(EnemyId::Healer, 3);
    healer.id = 1;
    let mut ally = test_monster(EnemyId::Cultist);
    ally.id = 2;
    let mut dead = test_monster(EnemyId::Cultist);
    dead.id = 3;
    dead.current_hp = 0;
    dead.is_dying = true;
    let mut state = combat_with_monsters(vec![healer.clone(), ally, dead]);
    let plan = Healer::turn_plan(&state, &healer);

    let actions = Healer::take_turn_plan(&mut state, &healer, &plan);

    assert!(matches!(
        actions.last(),
        Some(Action::RollMonsterMove { .. })
    ));
    for action in actions.iter().take(actions.len() - 1).cloned() {
        execute_action(action, &mut state);
    }

    let healer_strength = state
        .entities
        .power_db
        .get(&1)
        .and_then(|powers| {
            powers
                .iter()
                .find(|power| power.power_type == sts_simulator::content::powers::PowerId::Strength)
        })
        .map(|power| power.amount)
        .unwrap_or(0);
    let ally_strength = state
        .entities
        .power_db
        .get(&2)
        .and_then(|powers| {
            powers
                .iter()
                .find(|power| power.power_type == sts_simulator::content::powers::PowerId::Strength)
        })
        .map(|power| power.amount)
        .unwrap_or(0);
    let dead_strength = state
        .entities
        .power_db
        .get(&3)
        .and_then(|powers| {
            powers
                .iter()
                .find(|power| power.power_type == sts_simulator::content::powers::PowerId::Strength)
        })
        .map(|power| power.amount)
        .unwrap_or(0);

    assert_eq!(healer_strength, 2);
    assert_eq!(ally_strength, 2);
    assert_eq!(dead_strength, 0);
}

#[test]
fn snake_plant_turn_plan_reconstructs_spores_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::SnakePlant, 2);

    let plan = SnakePlant::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Frail, amount: 2, strength: Strong })"
    );
}

#[test]
fn snake_plant_a17_uses_last_move_before_spores_guard() {
    let mut state = blank_test_combat();
    state.meta.ascension_level = 17;
    let entity = monster_with_history(EnemyId::SnakePlant, 1, &[2, 1]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = SnakePlant::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 80);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 8, hits: 3, damage_kind: Normal })"
    );
}

#[test]
fn snake_plant_non_a17_does_not_use_last_move_before_spores_guard() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::SnakePlant, 1, &[2, 1]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = SnakePlant::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 80);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Frail, amount: 2, strength: Strong })"
    );
}

#[test]
fn snake_plant_take_turn_spores_applies_frail_and_weak_then_rolls() {
    let entity = planned_monster(EnemyId::SnakePlant, 2);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = SnakePlant::turn_plan(&state, &entity);

    let actions = SnakePlant::take_turn_plan(&mut state, &entity, &plan);

    assert!(matches!(
        actions.last(),
        Some(Action::RollMonsterMove { .. })
    ));
    for action in actions.iter().take(actions.len() - 1).cloned() {
        execute_action(action, &mut state);
    }

    let player_powers = state.entities.power_db.get(&0).cloned().unwrap_or_default();
    let frail = player_powers
        .iter()
        .find(|power| power.power_type == sts_simulator::content::powers::PowerId::Frail)
        .map(|power| power.amount)
        .unwrap_or(0);
    let weak = player_powers
        .iter()
        .find(|power| power.power_type == sts_simulator::content::powers::PowerId::Weak)
        .map(|power| power.amount)
        .unwrap_or(0);

    assert_eq!(frail, 2);
    assert_eq!(weak, 2);
}

#[test]
fn gremlin_leader_turn_plan_reconstructs_stab_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::GremlinLeader, 4);

    let plan = GremlinLeader::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 4);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 6, hits: 3, damage_kind: Normal })"
    );
}

#[test]
fn gremlin_leader_roll_move_with_no_gremlins_prefers_rally() {
    let state = combat_with_monsters(vec![test_monster(EnemyId::GremlinLeader)]);
    let entity = state.entities.monsters[0].clone();
    let mut rng = state.rng.ai_rng.clone();

    let plan = GremlinLeader::roll_move_plan_with_context(
        &mut rng,
        &entity,
        state.meta.ascension_level,
        10,
        MonsterRollContext {
            monsters: &state.entities.monsters,
            player_powers: &[],
        },
    );

    assert_eq!(plan.move_id, 2);
    assert_eq!(format!("{:?}", plan.summary_spec()), "Unknown");
}

#[test]
fn gremlin_leader_roll_move_with_two_gremlins_high_roll_is_stab() {
    let mut leader = test_monster(EnemyId::GremlinLeader);
    leader.id = 10;
    let mut ally_a = test_monster(EnemyId::GremlinFat);
    ally_a.id = 11;
    let mut ally_b = test_monster(EnemyId::GremlinWizard);
    ally_b.id = 12;
    let state = combat_with_monsters(vec![leader, ally_a, ally_b]);
    let entity = state.entities.monsters[0].clone();
    let mut rng = state.rng.ai_rng.clone();

    let plan = GremlinLeader::roll_move_plan_with_context(
        &mut rng,
        &entity,
        state.meta.ascension_level,
        90,
        MonsterRollContext {
            monsters: &state.entities.monsters,
            player_powers: &[],
        },
    );

    assert_eq!(plan.move_id, 4);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 6, hits: 3, damage_kind: Normal })"
    );
}

#[test]
fn gremlin_leader_take_turn_rally_only_fills_open_slots() {
    let mut leader = planned_monster(EnemyId::GremlinLeader, 2);
    leader.id = 10;
    leader.logical_position = GremlinLeader::LEADER_LOGICAL_POSITION;

    let mut first = test_monster(EnemyId::GremlinFat);
    first.id = 11;
    first.logical_position = GremlinLeader::GREMLIN_SLOT_LOGICAL_POSITIONS[0];

    let mut state = combat_with_monsters(vec![first, leader.clone()]);
    let plan = GremlinLeader::turn_plan(&state, &leader);

    let actions = GremlinLeader::take_turn_plan(&mut state, &leader, &plan);

    assert_eq!(actions.len(), 3);
    match &actions[0] {
        Action::SpawnMonsterSmart {
            logical_position,
            protocol_draw_x,
            is_minion,
            ..
        } => {
            assert_eq!(
                *logical_position,
                GremlinLeader::GREMLIN_SLOT_DRAW_X[1],
                "first open slot should be filled first"
            );
            assert_eq!(
                *protocol_draw_x,
                Some(GremlinLeader::GREMLIN_SLOT_DRAW_X[1])
            );
            assert!(*is_minion);
        }
        other => panic!("expected first summon action, got {other:?}"),
    }
    match &actions[1] {
        Action::SpawnMonsterSmart {
            logical_position,
            protocol_draw_x,
            is_minion,
            ..
        } => {
            assert_eq!(
                *logical_position,
                GremlinLeader::GREMLIN_SLOT_DRAW_X[2],
                "second open slot should be filled second"
            );
            assert_eq!(
                *protocol_draw_x,
                Some(GremlinLeader::GREMLIN_SLOT_DRAW_X[2])
            );
            assert!(*is_minion);
        }
        other => panic!("expected second summon action, got {other:?}"),
    }
    assert!(matches!(actions[2], Action::RollMonsterMove { .. }));
}

#[test]
fn gremlin_leader_resolve_on_death_escapes_surviving_allies() {
    let mut leader = test_monster(EnemyId::GremlinLeader);
    leader.id = 10;
    let mut ally_a = test_monster(EnemyId::GremlinFat);
    ally_a.id = 11;
    let mut ally_b = test_monster(EnemyId::GremlinWizard);
    ally_b.id = 12;
    let mut dead_ally = test_monster(EnemyId::GremlinThief);
    dead_ally.id = 13;
    dead_ally.is_dying = true;
    let mut state = combat_with_monsters(vec![leader.clone(), ally_a, ally_b, dead_ally]);

    let actions = resolve_on_death(EnemyId::GremlinLeader, &mut state, &leader);

    assert_eq!(
        actions,
        vec![Action::Escape { target: 11 }, Action::Escape { target: 12 }]
    );
}

#[test]
fn gremlin_leader_stateful_pre_battle_applies_minion_to_initial_allies() {
    let mut leader = test_monster(EnemyId::GremlinLeader);
    leader.id = 10;
    leader.logical_position = GremlinLeader::LEADER_LOGICAL_POSITION;
    let mut ally_a = test_monster(EnemyId::GremlinFat);
    ally_a.id = 11;
    ally_a.logical_position = GremlinLeader::GREMLIN_SLOT_LOGICAL_POSITIONS[0];
    let mut ally_b = test_monster(EnemyId::GremlinWizard);
    ally_b.id = 12;
    ally_b.logical_position = GremlinLeader::GREMLIN_SLOT_LOGICAL_POSITIONS[1];
    let mut state = combat_with_monsters(vec![ally_a, leader.clone(), ally_b]);

    let actions = resolve_pre_battle_actions(
        &mut state,
        EnemyId::GremlinLeader,
        &leader,
        PreBattleLegacyRng::Misc,
    );

    assert_eq!(actions.len(), 2);
    for action in actions {
        execute_action(action, &mut state);
    }

    let leader_minion = state
        .entities
        .power_db
        .get(&10)
        .and_then(|powers| {
            powers
                .iter()
                .find(|power| power.power_type == PowerId::Minion)
        })
        .map(|power| power.amount)
        .unwrap_or(0);
    let ally_a_has_minion = state.entities.power_db.get(&11).is_some_and(|powers| {
        powers
            .iter()
            .any(|power| power.power_type == PowerId::Minion)
    });
    let ally_b_has_minion = state.entities.power_db.get(&12).is_some_and(|powers| {
        powers
            .iter()
            .any(|power| power.power_type == PowerId::Minion)
    });

    assert_eq!(leader_minion, 0);
    assert!(ally_a_has_minion);
    assert!(ally_b_has_minion);
}

#[test]
fn collector_turn_plan_reconstructs_fireball_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::TheCollector, 2);

    let plan = TheCollector::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 18, hits: 1, damage_kind: Normal })"
    );
}

#[test]
fn collector_roll_move_plan_initial_spawn_from_runtime() {
    let state = combat_with_monsters(vec![test_monster(EnemyId::TheCollector)]);
    let entity = state.entities.monsters[0].clone();
    let mut rng = state.rng.ai_rng.clone();

    let plan = TheCollector::roll_move_plan_with_context(
        &mut rng,
        &entity,
        state.meta.ascension_level,
        99,
        MonsterRollContext {
            monsters: &state.entities.monsters,
            player_powers: &[],
        },
    );

    assert_eq!(plan.move_id, 1);
    assert_eq!(format!("{:?}", plan.summary_spec()), "Unknown");
}

#[test]
fn collector_take_turn_spawn_updates_runtime_before_roll() {
    let mut entity = planned_monster(EnemyId::TheCollector, 1);
    entity.id = 10;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = TheCollector::turn_plan(&state, &entity);

    let actions = TheCollector::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 4);
    assert!(matches!(
        actions[0],
        Action::SpawnMonsterSmart {
            monster_id: EnemyId::TorchHead,
            logical_position: 770,
            protocol_draw_x: Some(770),
            is_minion: true,
            ..
        }
    ));
    assert!(matches!(
        actions[1],
        Action::SpawnMonsterSmart {
            monster_id: EnemyId::TorchHead,
            logical_position: 647,
            protocol_draw_x: Some(647),
            is_minion: true,
            ..
        }
    ));
    execute_action(actions[2].clone(), &mut state);
    assert!(state.entities.monsters[0].collector.protocol_seeded);
    assert!(!state.entities.monsters[0].collector.initial_spawn);
    assert_eq!(state.entities.monsters[0].collector.turns_taken, 1);
    assert!(matches!(
        actions[3],
        Action::RollMonsterMove { monster_id: 10 }
    ));
}

#[test]
fn collector_revive_respawns_dying_torches_without_granting_block() {
    let mut collector = planned_monster(EnemyId::TheCollector, 5);
    collector.id = 10;
    collector.collector.initial_spawn = false;

    let mut torch_a = test_monster(EnemyId::TorchHead);
    torch_a.id = 11;
    torch_a.current_hp = 0;
    torch_a.is_dying = true;

    let mut torch_b = test_monster(EnemyId::TorchHead);
    torch_b.id = 12;
    torch_b.current_hp = 0;
    torch_b.is_dying = true;

    let mut state = combat_with_monsters(vec![collector.clone(), torch_a, torch_b]);
    state.monster_protocol_identity_mut(11).draw_x = Some(647);
    state.monster_protocol_identity_mut(12).draw_x = Some(770);
    let plan = TheCollector::turn_plan(&state, &collector);

    let actions = TheCollector::take_turn_plan(&mut state, &collector, &plan);

    assert!(actions
        .iter()
        .all(|action| !matches!(action, Action::GainBlock { .. })));
    assert!(matches!(
        actions[0],
        Action::SpawnMonsterSmart {
            monster_id: EnemyId::TorchHead,
            logical_position: 770,
            protocol_draw_x: Some(770),
            is_minion: true,
            ..
        }
    ));
    assert!(matches!(
        actions[1],
        Action::SpawnMonsterSmart {
            monster_id: EnemyId::TorchHead,
            logical_position: 647,
            protocol_draw_x: Some(647),
            is_minion: true,
            ..
        }
    ));
}

#[test]
fn collector_on_death_suicides_surviving_allies() {
    let mut collector = test_monster(EnemyId::TheCollector);
    collector.id = 10;
    let mut torch = test_monster(EnemyId::TorchHead);
    torch.id = 11;
    let mut dying_torch = test_monster(EnemyId::TorchHead);
    dying_torch.id = 12;
    dying_torch.current_hp = 0;
    dying_torch.is_dying = true;
    let mut state = combat_with_monsters(vec![collector.clone(), torch, dying_torch]);

    let actions = TheCollector::on_death(&mut state, &collector);

    assert_eq!(actions, vec![Action::Suicide { target: 11 }]);
}

#[test]
fn torch_head_take_turn_sets_next_move_without_roll() {
    let entity = planned_monster(EnemyId::TorchHead, 1);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = TorchHead::turn_plan(&state, &entity);

    let actions = TorchHead::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 2);
    assert!(matches!(
        actions[0],
        Action::MonsterAttack { base_damage: 7, .. }
    ));
    assert!(matches!(
        actions[1],
        Action::SetMonsterMove {
            monster_id: 1,
            next_move_byte: 1,
            ..
        }
    ));
    assert!(actions
        .iter()
        .all(|action| !matches!(action, Action::RollMonsterMove { .. })));
}

#[test]
fn taskmaster_turn_plan_reconstructs_whip_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Taskmaster, 2);

    let plan = Taskmaster::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackAddCard(AttackSpec { base_damage: 7, hits: 1, damage_kind: Normal }, AddCardStep { card_id: Wound, amount: 1, upgraded: false, destination: Discard, visible_strength: Normal })"
    );
}

#[test]
fn taskmaster_take_turn_a18_adds_strength_before_roll() {
    let mut state = blank_test_combat();
    state.meta.ascension_level = 18;
    let entity = planned_monster(EnemyId::Taskmaster, 2);
    let plan = Taskmaster::turn_plan(&state, &entity);

    let actions = Taskmaster::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 4);
    assert!(matches!(
        actions[0],
        Action::MonsterAttack { base_damage: 7, .. }
    ));
    assert!(matches!(
        actions[1],
        Action::MakeTempCardInDiscard {
            card_id: sts_simulator::content::cards::CardId::Wound,
            amount: 3,
            upgraded: false,
        }
    ));
    execute_action(actions[2].clone(), &mut state);
    let strength = state
        .entities
        .power_db
        .get(&1)
        .and_then(|powers| {
            powers
                .iter()
                .find(|power| power.power_type == sts_simulator::content::powers::PowerId::Strength)
        })
        .map(|power| power.amount)
        .unwrap_or(0);
    assert_eq!(strength, 1);
    assert!(matches!(
        actions[3],
        Action::RollMonsterMove { monster_id: 1 }
    ));
}

#[test]
fn taskmaster_resolve_on_death_uses_semantic_noop_dispatch() {
    let mut state = blank_test_combat();
    let entity = test_monster(EnemyId::Taskmaster);

    let actions = resolve_on_death(EnemyId::Taskmaster, &mut state, &entity);

    assert!(actions.is_empty());
}

#[test]
fn book_of_stabbing_turn_plan_uses_seeded_stab_count() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::BookOfStabbing, 1);
    entity.book_of_stabbing.protocol_seeded = true;
    entity.book_of_stabbing.stab_count = 3;

    let plan = BookOfStabbing::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 6, hits: 3, damage_kind: Normal })"
    );
}

#[test]
fn book_of_stabbing_roll_move_after_big_stab_increments_hits() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::BookOfStabbing, 2);
    entity.book_of_stabbing.protocol_seeded = true;
    entity.book_of_stabbing.stab_count = 3;
    entity.move_history_mut().clear();
    entity.move_history_mut().push_back(2);
    let mut rng = state.rng.ai_rng.clone();

    let plan = BookOfStabbing::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 5);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 6, hits: 4, damage_kind: Normal })"
    );
}

#[test]
fn book_of_stabbing_on_roll_move_updates_stab_count_for_a18_big_stab() {
    let mut entity = planned_monster(EnemyId::BookOfStabbing, 2);
    entity.book_of_stabbing.protocol_seeded = true;
    entity.book_of_stabbing.stab_count = 2;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    state.meta.ascension_level = 18;
    let plan = BookOfStabbing::turn_plan(&state, &entity);
    let actions = BookOfStabbing::on_roll_move(state.meta.ascension_level, &entity, 50, &plan);

    assert_eq!(actions.len(), 1);
    execute_action(actions[0].clone(), &mut state);
    assert_eq!(state.entities.monsters[0].book_of_stabbing.stab_count, 3);
    assert!(state.entities.monsters[0].book_of_stabbing.protocol_seeded);
}

#[test]
fn book_of_stabbing_resolve_on_death_uses_semantic_noop_dispatch() {
    let mut state = blank_test_combat();
    let entity = test_monster(EnemyId::BookOfStabbing);

    let actions = resolve_on_death(EnemyId::BookOfStabbing, &mut state, &entity);

    assert!(actions.is_empty());
}

#[test]
fn time_eater_turn_plan_reconstructs_head_slam_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::TimeEater, 4);

    let plan = TimeEater::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 4);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackDebuff(AttackSpec { base_damage: 26, hits: 1, damage_kind: Normal }, DebuffSpec { power_id: DrawReduction, amount: 1, strength: Normal })"
    );
}

#[test]
fn time_eater_roll_move_plan_below_half_prefers_haste() {
    let state = blank_test_combat();
    let mut entity = test_monster(EnemyId::TimeEater);
    entity.max_hp = 456;
    entity.current_hp = 200;
    let mut rng = state.rng.ai_rng.clone();

    let plan = TimeEater::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 5);

    assert_eq!(plan.move_id, 5);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Heal(HealSpec { target: SelfTarget, amount: 28 })"
    );
    let steps = format!("{:?}", plan.steps);
    assert!(steps.contains("Utility(RemoveAllDebuffs { target: SelfTarget })"));
    assert!(
        steps.contains("RemovePower(RemovePowerStep { target: SelfTarget, power_id: Shackled })")
    );
}

#[test]
fn time_eater_take_turn_haste_cleanses_heals_and_blocks() {
    let mut state = blank_test_combat();
    state.meta.ascension_level = 19;
    let mut entity = planned_monster(EnemyId::TimeEater, 5);
    entity.max_hp = 456;
    entity.current_hp = 200;
    entity.id = 10;
    let plan = TimeEater::turn_plan(&state, &entity);

    let actions = TimeEater::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(
        actions,
        vec![
            Action::RemoveAllDebuffs { target: 10 },
            Action::RemovePower {
                target: 10,
                power_id: PowerId::Shackled,
            },
            Action::Heal {
                target: 10,
                amount: 28,
            },
            Action::GainBlock {
                target: 10,
                amount: 32,
            },
            Action::RollMonsterMove { monster_id: 10 },
        ]
    );
}

#[test]
fn awakened_one_resolve_pre_battle_applies_core_powers() {
    let mut state = blank_test_combat();
    state.meta.ascension_level = 19;
    let mut entity = test_monster(EnemyId::AwakenedOne);
    entity.id = 10;

    let actions = resolve_pre_battle_actions(
        &mut state,
        EnemyId::AwakenedOne,
        &entity,
        PreBattleLegacyRng::MonsterHp,
    );
    for action in actions {
        execute_action(action, &mut state);
    }

    let powers = state.entities.power_db.get(&10).unwrap();
    assert!(powers
        .iter()
        .any(|power| power.power_type == PowerId::Unawakened));
    assert_eq!(
        powers
            .iter()
            .find(|power| power.power_type == PowerId::Regen)
            .map(|power| power.amount),
        Some(15)
    );
    assert_eq!(
        powers
            .iter()
            .find(|power| power.power_type == PowerId::Curiosity)
            .map(|power| power.amount),
        Some(2)
    );
    assert_eq!(
        powers
            .iter()
            .find(|power| power.power_type == PowerId::Strength)
            .map(|power| power.amount),
        Some(2)
    );
}

#[test]
fn awakened_one_roll_move_plan_first_turn_is_slash_and_clears_runtime_flag() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::AwakenedOne);
    let mut rng = state.rng.ai_rng.clone();

    let plan = AwakenedOne::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 20, hits: 1, damage_kind: Normal })"
    );

    let mut execute_state = combat_with_monsters(vec![entity.clone()]);
    for action in resolve_roll_move_actions(&execute_state, &entity, 99, &plan) {
        execute_action(action, &mut execute_state);
    }
    assert!(!execute_state.entities.monsters[0].awakened_one.first_turn);
}

#[test]
fn awakened_one_turn_plan_and_take_turn_rebirth_revive_and_heal() {
    let mut state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::AwakenedOne, 3);
    entity.id = 10;
    entity.current_hp = 0;
    entity.half_dead = true;
    entity.awakened_one.protocol_seeded = true;
    entity.awakened_one.form1 = false;
    entity.awakened_one.first_turn = true;

    let plan = AwakenedOne::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 3);
    assert_eq!(format!("{:?}", plan.summary_spec()), "Unknown");

    let actions = AwakenedOne::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(
        actions,
        vec![
            Action::ReviveMonster { target: 10 },
            Action::Heal {
                target: 10,
                amount: 20,
            },
            Action::RollMonsterMove { monster_id: 10 },
        ]
    );
}

#[test]
fn awakened_one_on_death_makes_cultists_escape() {
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 10;
    let mut cultist_a = test_monster(EnemyId::Cultist);
    cultist_a.id = 11;
    let mut cultist_b = test_monster(EnemyId::Cultist);
    cultist_b.id = 12;
    let mut state = combat_with_monsters(vec![awakened.clone(), cultist_a, cultist_b]);

    let actions = resolve_on_death(EnemyId::AwakenedOne, &mut state, &awakened);

    assert_eq!(
        actions,
        vec![Action::Escape { target: 11 }, Action::Escape { target: 12 },]
    );
}

#[test]
fn spire_shield_resolve_pre_battle_applies_surrounded_and_artifact() {
    let mut state = blank_test_combat();
    let mut entity = test_monster(EnemyId::SpireShield);
    entity.id = 10;

    let actions = resolve_pre_battle_actions(
        &mut state,
        EnemyId::SpireShield,
        &entity,
        PreBattleLegacyRng::MonsterHp,
    );

    assert_eq!(actions.len(), 2);
    for action in actions {
        execute_action(action, &mut state);
    }

    let player_has_surrounded = state.entities.power_db.get(&0).is_some_and(|powers| {
        powers
            .iter()
            .any(|power| power.power_type == PowerId::Surrounded)
    });
    let artifact_amount = state
        .entities
        .power_db
        .get(&10)
        .and_then(|powers| {
            powers
                .iter()
                .find(|power| power.power_type == PowerId::Artifact)
        })
        .map(|power| power.amount)
        .unwrap_or(0);

    assert!(player_has_surrounded);
    assert_eq!(artifact_amount, 1);
}

#[test]
fn spire_shield_roll_move_plan_move_count_two_is_smash() {
    let state = blank_test_combat();
    let mut entity = test_monster(EnemyId::SpireShield);
    entity.move_history_mut().extend([1, 2]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = SpireShield::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 0);

    assert_eq!(plan.move_id, 3);
}

#[test]
fn spire_shield_turn_plan_reconstructs_smash_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::SpireShield, 3);

    let plan = SpireShield::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackDefend(AttackSpec { base_damage: 34, hits: 1, damage_kind: Normal }, DefendSpec { block: 34 })"
    );
}

#[test]
fn spire_shield_fortify_blocks_all_monsters_and_rolls() {
    let mut shield = planned_monster(EnemyId::SpireShield, 2);
    shield.id = 10;
    let mut ally = test_monster(EnemyId::Cultist);
    ally.id = 11;
    let mut state = combat_with_monsters(vec![shield.clone(), ally]);
    let plan = SpireShield::turn_plan(&state, &shield);

    let actions = SpireShield::take_turn_plan(&mut state, &shield, &plan);

    assert_eq!(
        actions,
        vec![
            Action::GainBlock {
                target: 10,
                amount: 30,
            },
            Action::GainBlock {
                target: 11,
                amount: 30,
            },
            Action::RollMonsterMove { monster_id: 10 },
        ]
    );
}

#[test]
fn spire_spear_roll_move_plan_move_count_one_is_skewer() {
    let state = blank_test_combat();
    let mut entity = test_monster(EnemyId::SpireSpear);
    entity.move_history_mut().push_back(1);
    let mut rng = state.rng.ai_rng.clone();

    let plan = SpireSpear::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 0);

    assert_eq!(plan.move_id, 3);
}

#[test]
fn spire_spear_turn_plan_reconstructs_burn_strike_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::SpireSpear, 1);

    let plan = SpireSpear::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackAddCard(AttackSpec { base_damage: 5, hits: 2, damage_kind: Normal }, AddCardStep { card_id: Burn, amount: 2, upgraded: false, destination: Discard, visible_strength: Normal })"
    );
}

#[test]
fn spire_spear_take_turn_piercer_buffs_all_monsters_and_rolls() {
    let mut spear = planned_monster(EnemyId::SpireSpear, 2);
    spear.id = 10;
    let mut ally = test_monster(EnemyId::Cultist);
    ally.id = 11;
    let mut state = combat_with_monsters(vec![spear.clone(), ally]);
    let plan = SpireSpear::turn_plan(&state, &spear);

    let actions = SpireSpear::take_turn_plan(&mut state, &spear, &plan);

    assert_eq!(
        actions,
        vec![
            Action::ApplyPower {
                source: 10,
                target: 10,
                power_id: PowerId::Strength,
                amount: 2,
            },
            Action::ApplyPower {
                source: 10,
                target: 11,
                power_id: PowerId::Strength,
                amount: 2,
            },
            Action::RollMonsterMove { monster_id: 10 },
        ]
    );
}

#[test]
fn shield_and_spear_on_death_clear_surrounded_and_back_attack() {
    let mut shield = test_monster(EnemyId::SpireShield);
    shield.id = 10;
    let mut spear = test_monster(EnemyId::SpireSpear);
    spear.id = 11;
    let mut state = combat_with_monsters(vec![shield.clone(), spear.clone()]);
    execute_action(
        Action::ApplyPower {
            source: 10,
            target: 0,
            power_id: PowerId::Surrounded,
            amount: 1,
        },
        &mut state,
    );
    execute_action(
        Action::ApplyPower {
            source: 10,
            target: 11,
            power_id: PowerId::BackAttack,
            amount: 1,
        },
        &mut state,
    );

    let actions = resolve_on_death(EnemyId::SpireShield, &mut state, &shield);

    assert_eq!(
        actions,
        vec![
            Action::RemovePower {
                target: 0,
                power_id: PowerId::Surrounded,
            },
            Action::RemovePower {
                target: 11,
                power_id: PowerId::BackAttack,
            },
        ]
    );
}

#[test]
fn corrupt_heart_resolve_pre_battle_applies_invincible_and_beat_of_death() {
    let mut state = blank_test_combat();
    state.meta.ascension_level = 19;
    let mut entity = test_monster(EnemyId::CorruptHeart);
    entity.id = 10;

    let actions = resolve_pre_battle_actions(
        &mut state,
        EnemyId::CorruptHeart,
        &entity,
        PreBattleLegacyRng::MonsterHp,
    );
    for action in actions {
        execute_action(action, &mut state);
    }

    let powers = state.entities.power_db.get(&10).unwrap();
    assert_eq!(
        powers
            .iter()
            .find(|power| power.power_type == PowerId::Invincible)
            .map(|power| power.amount),
        Some(200)
    );
    assert_eq!(
        powers
            .iter()
            .find(|power| power.power_type == PowerId::BeatOfDeath)
            .map(|power| power.amount),
        Some(2)
    );
}

#[test]
fn corrupt_heart_roll_move_plan_first_turn_is_debilitate_and_clears_runtime_flag() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::CorruptHeart);
    let mut rng = state.rng.ai_rng.clone();

    let plan = CorruptHeart::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Vulnerable, amount: 2, strength: Strong })"
    );

    let mut execute_state = combat_with_monsters(vec![entity.clone()]);
    for action in resolve_roll_move_actions(&execute_state, &entity, 99, &plan) {
        execute_action(action, &mut execute_state);
    }
    assert!(!execute_state.entities.monsters[0].corrupt_heart.first_move);
    assert!(
        execute_state.entities.monsters[0]
            .corrupt_heart
            .protocol_seeded
    );
}

#[test]
fn corrupt_heart_roll_move_plan_move_count_one_prefers_opposite_attack() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::CorruptHeart, 2);
    entity.corrupt_heart.first_move = false;
    entity.corrupt_heart.move_count = 1;
    entity.move_history_mut().clear();
    entity.move_history_mut().extend([3, 2]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = CorruptHeart::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 10);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 2, hits: 12, damage_kind: Normal })"
    );
}

#[test]
fn corrupt_heart_take_turn_buff_uses_current_buff_count_and_updates_runtime() {
    let mut state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::CorruptHeart, 4);
    entity.id = 10;
    entity.corrupt_heart.protocol_seeded = true;
    entity.corrupt_heart.first_move = false;
    entity.corrupt_heart.move_count = 2;
    entity.corrupt_heart.buff_count = 2;
    state.entities.power_db.insert(
        10,
        vec![sts_simulator::runtime::combat::Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: -3,
            extra_data: 0,
            just_applied: false,
        }],
    );
    let plan = CorruptHeart::turn_plan(&state, &entity);

    let actions = CorruptHeart::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(
        actions,
        vec![
            Action::ApplyPower {
                source: 10,
                target: 10,
                power_id: PowerId::Strength,
                amount: 5,
            },
            Action::ApplyPower {
                source: 10,
                target: 10,
                power_id: PowerId::PainfulStabs,
                amount: 1,
            },
            Action::UpdateMonsterRuntime {
                monster_id: 10,
                patch: sts_simulator::runtime::action::MonsterRuntimePatch::CorruptHeart {
                    first_move: None,
                    move_count: None,
                    buff_count: Some(3),
                    protocol_seeded: Some(true),
                },
            },
            Action::RollMonsterMove { monster_id: 10 },
        ]
    );
}

#[test]
fn corrupt_heart_turn_plan_reconstructs_blood_shots_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::CorruptHeart, 1);

    let plan = CorruptHeart::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 2, hits: 12, damage_kind: Normal })"
    );
}

#[test]
fn donu_roll_move_plan_first_turn_is_circle_of_protection() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::Donu);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Donu::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Buff(BuffSpec { power_id: Strength, amount: 3 })"
    );
}

#[test]
fn donu_turn_plan_reconstructs_beam_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Donu, 0);

    let plan = Donu::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 0);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 10, hits: 2, damage_kind: Normal })"
    );
}

#[test]
fn donu_take_turn_circle_buffs_all_monsters_and_rolls() {
    let mut donu = planned_monster(EnemyId::Donu, 2);
    donu.id = 10;
    let mut deca = test_monster(EnemyId::Deca);
    deca.id = 11;
    let mut state = combat_with_monsters(vec![donu.clone(), deca]);
    let plan = Donu::turn_plan(&state, &donu);

    let actions = Donu::take_turn_plan(&mut state, &donu, &plan);

    assert_eq!(
        actions,
        vec![
            Action::ApplyPower {
                source: 10,
                target: 10,
                power_id: PowerId::Strength,
                amount: 3,
            },
            Action::ApplyPower {
                source: 10,
                target: 11,
                power_id: PowerId::Strength,
                amount: 3,
            },
            Action::RollMonsterMove { monster_id: 10 },
        ]
    );
}

#[test]
fn deca_roll_move_plan_after_beam_becomes_square() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::Deca, 0);
    entity.move_history_mut().clear();
    entity.move_history_mut().push_back(0);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Deca::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Defend(DefendSpec { block: 16 })"
    );
}

#[test]
fn deca_turn_plan_reconstructs_beam_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Deca, 0);

    let plan = Deca::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 0);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackAddCard(AttackSpec { base_damage: 10, hits: 2, damage_kind: Normal }, AddCardStep { card_id: Dazed, amount: 2, upgraded: false, destination: Discard, visible_strength: Normal })"
    );
}

#[test]
fn deca_take_turn_square_a19_grants_block_and_plated_armor_to_all_monsters() {
    let mut state = blank_test_combat();
    state.meta.ascension_level = 19;
    let mut deca = planned_monster(EnemyId::Deca, 2);
    deca.id = 10;
    let mut donu = test_monster(EnemyId::Donu);
    donu.id = 11;
    let mut state = combat_with_monsters(vec![deca.clone(), donu]);
    state.meta.ascension_level = 19;
    let plan = Deca::turn_plan(&state, &deca);

    let actions = Deca::take_turn_plan(&mut state, &deca, &plan);

    assert_eq!(
        actions,
        vec![
            Action::GainBlock {
                target: 10,
                amount: 16,
            },
            Action::ApplyPower {
                source: 10,
                target: 10,
                power_id: PowerId::PlatedArmor,
                amount: 3,
            },
            Action::GainBlock {
                target: 11,
                amount: 16,
            },
            Action::ApplyPower {
                source: 10,
                target: 11,
                power_id: PowerId::PlatedArmor,
                amount: 3,
            },
            Action::RollMonsterMove { monster_id: 10 },
        ]
    );
}

#[test]
fn exploder_resolve_pre_battle_applies_explosive() {
    let mut state = blank_test_combat();
    let entity = test_monster(EnemyId::Exploder);

    let actions = resolve_pre_battle_actions(
        &mut state,
        EnemyId::Exploder,
        &entity,
        PreBattleLegacyRng::MonsterHp,
    );

    assert_eq!(
        actions,
        vec![Action::ApplyPower {
            source: 1,
            target: 1,
            power_id: PowerId::Explosive,
            amount: 3,
        }]
    );
}

#[test]
fn exploder_roll_move_plan_after_two_turns_is_unknown() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::Exploder, 1, &[1, 1]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Exploder::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 2);
    assert_eq!(format!("{:?}", plan.summary_spec()), "Unknown");
}

#[test]
fn repulsor_roll_move_low_roll_not_last_attack_is_attack() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::Repulsor, 1, &[1]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Repulsor::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 19);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 11, hits: 1, damage_kind: Normal })"
    );
}

#[test]
fn repulsor_roll_move_high_roll_is_daze() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::Repulsor);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Repulsor::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 20);

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AddCard(AddCardStep { card_id: Dazed, amount: 2, upgraded: false, destination: DrawPileRandom, visible_strength: Normal })"
    );
}

#[test]
fn spiker_resolve_pre_battle_applies_a17_bonus_thorns() {
    let mut state = blank_test_combat();
    state.meta.ascension_level = 17;
    let entity = test_monster(EnemyId::Spiker);

    let actions = resolve_pre_battle_actions(
        &mut state,
        EnemyId::Spiker,
        &entity,
        PreBattleLegacyRng::MonsterHp,
    );

    assert_eq!(
        actions,
        vec![Action::ApplyPower {
            source: 1,
            target: 1,
            power_id: PowerId::Thorns,
            amount: 7,
        }]
    );
}

#[test]
fn spiker_turn_plan_reconstructs_buff_thorns() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Spiker, 2);

    let plan = Spiker::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Buff(BuffSpec { power_id: Thorns, amount: 2 })"
    );
}

#[test]
fn orb_walker_roll_move_respects_last_two_moves() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::OrbWalker, 2, &[2, 2]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = OrbWalker::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 10);

    assert_eq!(plan.move_id, 1);
}

#[test]
fn orb_walker_turn_plan_reconstructs_laser() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::OrbWalker, 1);

    let plan = OrbWalker::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 1);
    assert!(matches!(plan.steps.as_slice(), [_, _, _]));
}

#[test]
fn snake_dagger_first_turn_wound_then_explode() {
    let state = blank_test_combat();
    let mut rng = state.rng.ai_rng.clone();
    let first = SnakeDagger::roll_move_plan(
        &mut rng,
        &test_monster(EnemyId::SnakeDagger),
        state.meta.ascension_level,
        99,
    );
    let second = SnakeDagger::roll_move_plan(
        &mut rng,
        &monster_with_history(EnemyId::SnakeDagger, 1, &[1]),
        state.meta.ascension_level,
        99,
    );

    assert_eq!(first.move_id, 1);
    assert_eq!(second.move_id, 2);
}

#[test]
fn maw_roll_move_plan_first_turn_is_roar() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::Maw);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Maw::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Weak, amount: 3, strength: Strong })"
    );
}

#[test]
fn maw_turn_plan_reconstructs_nom_hits_from_history() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::Maw, 5, &[2, 3, 5]);

    let plan = Maw::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 5);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 5, hits: 2, damage_kind: Normal })"
    );
}

#[test]
fn spire_growth_roll_move_rechecks_player_constricted_from_stateful_context() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::SpireGrowth, 1, &[1]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = roll_monster_turn_plan(
        &mut rng,
        &entity,
        state.meta.ascension_level,
        99,
        std::slice::from_ref(&entity),
        &[],
    );

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Constricted, amount: 10, strength: Strong })"
    );
}

#[test]
fn spire_growth_turn_plan_reconstructs_smash() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::SpireGrowth, 3);

    let plan = SpireGrowth::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 22, hits: 1, damage_kind: Normal })"
    );
}

#[test]
fn transient_resolve_pre_battle_applies_fading_and_shifting() {
    let mut state = blank_test_combat();
    let entity = test_monster(EnemyId::Transient);

    let actions = resolve_pre_battle_actions(
        &mut state,
        EnemyId::Transient,
        &entity,
        PreBattleLegacyRng::MonsterHp,
    );

    assert_eq!(
        actions,
        vec![
            Action::ApplyPower {
                source: 1,
                target: 1,
                power_id: PowerId::Fading,
                amount: 5,
            },
            Action::ApplyPower {
                source: 1,
                target: 1,
                power_id: PowerId::Shifting,
                amount: 1,
            },
        ]
    );
}

#[test]
fn transient_take_turn_sets_next_move_without_roll() {
    let entity = monster_with_history(EnemyId::Transient, 1, &[1, 1, 1]);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Transient::turn_plan(&state, &entity);

    let actions = Transient::take_turn_plan(&mut state, &entity, &plan);

    assert!(actions
        .iter()
        .all(|action| !matches!(action, Action::RollMonsterMove { .. })));
    assert!(matches!(
        actions.last(),
        Some(Action::SetMonsterMove {
            next_move_byte: 1,
            ..
        })
    ));
}

#[test]
fn giant_head_roll_move_threshold_becomes_it_is_time() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::GiantHead, 3, &[1, 3, 1, 3]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = GiantHead::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 10);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 30, hits: 1, damage_kind: Normal })"
    );
}

#[test]
fn giant_head_turn_plan_reconstructs_it_is_time_damage() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::GiantHead, 2, &[1, 3, 1, 3]);

    let plan = GiantHead::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 30, hits: 1, damage_kind: Normal })"
    );
}

#[test]
fn nemesis_roll_move_first_turn_can_be_tri_attack() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::Nemesis);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Nemesis::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 10);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 6, hits: 3, damage_kind: Normal })"
    );
}

#[test]
fn nemesis_take_turn_tri_burn_adds_burn_and_intangible() {
    let entity = planned_monster(EnemyId::Nemesis, 4);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Nemesis::turn_plan(&state, &entity);

    let actions = Nemesis::take_turn_plan(&mut state, &entity, &plan);

    assert!(matches!(
        actions.as_slice(),
        [
            Action::MakeTempCardInDiscard {
                card_id: sts_simulator::content::cards::CardId::Burn,
                amount: 3,
                upgraded: false
            },
            Action::ApplyPower {
                power_id: PowerId::Intangible,
                amount: 1,
                ..
            },
            Action::RollMonsterMove { .. }
        ]
    ));
}

#[test]
fn reptomancer_resolve_pre_battle_applies_minion_to_daggers() {
    let mut left = test_monster(EnemyId::SnakeDagger);
    left.id = 10;
    let mut repto = test_monster(EnemyId::Reptomancer);
    repto.id = 11;
    let mut right = test_monster(EnemyId::SnakeDagger);
    right.id = 12;
    let mut state = combat_with_monsters(vec![left, repto.clone(), right]);

    let actions = resolve_pre_battle_actions(
        &mut state,
        EnemyId::Reptomancer,
        &repto,
        PreBattleLegacyRng::MonsterHp,
    );

    assert_eq!(
        actions,
        vec![
            Action::ApplyPower {
                source: 11,
                target: 10,
                power_id: PowerId::Minion,
                amount: 1,
            },
            Action::ApplyPower {
                source: 11,
                target: 12,
                power_id: PowerId::Minion,
                amount: 1,
            },
        ]
    );
}

#[test]
fn reptomancer_take_turn_spawn_dagger_uses_open_slots() {
    let mut left = test_monster(EnemyId::SnakeDagger);
    left.logical_position = Reptomancer::DAGGER_DRAW_X[1];
    let mut repto = planned_monster(EnemyId::Reptomancer, 2);
    repto.id = 10;
    repto.logical_position = 0;
    let mut right = test_monster(EnemyId::SnakeDagger);
    right.logical_position = Reptomancer::DAGGER_DRAW_X[0];
    let mut state = combat_with_monsters(vec![left, repto.clone(), right]);
    state.meta.ascension_level = 18;
    let plan = Reptomancer::turn_plan(&state, &repto);

    let actions = Reptomancer::take_turn_plan(&mut state, &repto, &plan);

    assert!(matches!(
        actions.as_slice(),
        [
            Action::SpawnMonsterSmart {
                monster_id: EnemyId::SnakeDagger,
                logical_position: 180,
                protocol_draw_x: Some(180),
                is_minion: true,
                ..
            },
            Action::SpawnMonsterSmart {
                monster_id: EnemyId::SnakeDagger,
                logical_position: -250,
                protocol_draw_x: Some(-250),
                is_minion: true,
                ..
            },
            Action::RollMonsterMove { monster_id: 10 }
        ]
    ));
}

#[test]
fn reptomancer_on_death_suicides_surviving_allies() {
    let mut repto = test_monster(EnemyId::Reptomancer);
    repto.id = 10;
    let mut dagger = test_monster(EnemyId::SnakeDagger);
    dagger.id = 11;
    let mut other = test_monster(EnemyId::OrbWalker);
    other.id = 12;
    let mut state = combat_with_monsters(vec![repto.clone(), dagger, other]);

    let actions = resolve_on_death(EnemyId::Reptomancer, &mut state, &repto);

    assert_eq!(
        actions,
        vec![
            Action::Suicide { target: 11 },
            Action::Suicide { target: 12 }
        ]
    );
}

#[test]
fn darkling_roll_move_even_position_can_choose_chomp() {
    let state = blank_test_combat();
    let mut entity = monster_with_history(EnemyId::Darkling, 2, &[2]);
    entity.darkling.first_move = false;
    let mut rng = state.rng.ai_rng.clone();

    let plan = roll_monster_turn_plan(
        &mut rng,
        &entity,
        state.meta.ascension_level,
        10,
        std::slice::from_ref(&entity),
        &[],
    );

    assert_eq!(plan.move_id, 1);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 8, hits: 2, damage_kind: Normal })"
    );
}

#[test]
fn darkling_on_death_becomes_half_dead_when_siblings_remain() {
    let mut first = test_monster(EnemyId::Darkling);
    first.id = 10;
    let mut second = test_monster(EnemyId::Darkling);
    second.id = 11;
    let mut state = combat_with_monsters(vec![first.clone(), second]);

    let actions = resolve_on_death(EnemyId::Darkling, &mut state, &first);

    assert!(state.entities.monsters[0].half_dead);
    assert!(!state.entities.monsters[0].is_dying);
    assert_eq!(state.entities.monsters[0].current_hp, 0);
    assert!(matches!(
        actions.as_slice(),
        [Action::SetMonsterMove {
            next_move_byte: 4,
            ..
        }]
    ));
}

#[test]
fn writhing_mass_roll_move_first_turn_can_be_attack_block() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::WrithingMass);
    let mut rng = state.rng.ai_rng.clone();

    let plan = WrithingMass::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 50);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackDefend(AttackSpec { base_damage: 15, hits: 1, damage_kind: Normal }, DefendSpec { block: 15 })"
    );
}

#[test]
fn writhing_mass_roll_move_mega_debuff_is_locked_out_after_use() {
    let state = blank_test_combat();
    let entity = monster_with_history(EnemyId::WrithingMass, 4, &[4]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = WrithingMass::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 15);

    assert_ne!(plan.move_id, 4);
}

#[test]
fn writhing_mass_turn_plan_reconstructs_attack_debuff() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::WrithingMass, 3);

    let plan = WrithingMass::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "AttackDebuff(AttackSpec { base_damage: 10, hits: 1, damage_kind: Normal }, DebuffSpec { power_id: Weak, amount: 2, strength: Normal })"
    );
}

#[test]
fn champ_turn_plan_reconstructs_execute_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::Champ, 3);

    let plan = Champ::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 10, hits: 2, damage_kind: Normal })"
    );
}

#[test]
fn champ_roll_move_threshold_crossing_becomes_anger() {
    let state = blank_test_combat();
    let mut entity = test_monster(EnemyId::Champ);
    entity.current_hp = 199;
    entity.max_hp = 420;
    let mut rng = state.rng.ai_rng.clone();

    let plan = Champ::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 7);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Buff(BuffSpec { power_id: Strength, amount: 6 })"
    );
}

#[test]
fn champ_roll_move_threshold_state_prefers_execute_when_recent_history_allows() {
    let state = blank_test_combat();
    let mut entity = planned_monster(EnemyId::Champ, 5);
    entity.champ.threshold_reached = true;
    entity.champ.first_turn = false;
    entity.move_history_mut().clear();
    entity.move_history_mut().extend([4, 5]);
    let mut rng = state.rng.ai_rng.clone();

    let plan = Champ::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 10);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 10, hits: 2, damage_kind: Normal })"
    );
}

#[test]
fn champ_on_roll_move_updates_num_turns_and_forge_times() {
    let mut entity = planned_monster(EnemyId::Champ, 2);
    entity.champ.first_turn = false;
    entity.champ.num_turns = 2;
    entity.champ.forge_times = 1;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Champ::turn_plan(&state, &entity);

    let actions = Champ::on_roll_move(state.meta.ascension_level, &entity, 5, &plan);

    assert_eq!(actions.len(), 1);
    execute_action(actions[0].clone(), &mut state);
    assert!(state.entities.monsters[0].champ.protocol_seeded);
    assert_eq!(state.entities.monsters[0].champ.num_turns, 3);
    assert_eq!(state.entities.monsters[0].champ.forge_times, 2);
    assert!(!state.entities.monsters[0].champ.threshold_reached);
}

#[test]
fn champ_take_turn_first_turn_anger_clears_first_turn_and_rolls() {
    let mut entity = planned_monster(EnemyId::Champ, 7);
    entity.id = 10;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = Champ::turn_plan(&state, &entity);

    let actions = Champ::take_turn_plan(&mut state, &entity, &plan);

    assert!(matches!(
        actions.last(),
        Some(Action::RollMonsterMove { monster_id: 10 })
    ));
    execute_action(actions[0].clone(), &mut state);
    assert!(state.entities.monsters[0].champ.protocol_seeded);
    assert!(!state.entities.monsters[0].champ.first_turn);
}

#[test]
fn champ_resolve_on_death_uses_semantic_noop_dispatch() {
    let mut state = blank_test_combat();
    let entity = test_monster(EnemyId::Champ);

    let actions = resolve_on_death(EnemyId::Champ, &mut state, &entity);

    assert!(actions.is_empty());
}

#[test]
fn bronze_automaton_turn_plan_reconstructs_hyper_beam_from_move_id() {
    let state = blank_test_combat();
    let entity = planned_monster(EnemyId::BronzeAutomaton, 2);

    let plan = BronzeAutomaton::turn_plan(&state, &entity);

    assert_eq!(plan.move_id, 2);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "Attack(AttackSpec { base_damage: 45, hits: 1, damage_kind: Normal })"
    );
}

#[test]
fn bronze_automaton_roll_move_plan_first_turn_spawns_orbs() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::BronzeAutomaton);
    let mut rng = state.rng.ai_rng.clone();

    let plan = BronzeAutomaton::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 99);

    assert_eq!(plan.move_id, 4);
    assert_eq!(format!("{:?}", plan.summary_spec()), "Unknown");
}

#[test]
fn bronze_automaton_on_roll_move_updates_num_turns() {
    let mut entity = planned_monster(EnemyId::BronzeAutomaton, 5);
    entity.bronze_automaton.first_turn = false;
    entity.bronze_automaton.num_turns = 2;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = BronzeAutomaton::turn_plan(&state, &entity);

    let actions = BronzeAutomaton::on_roll_move(state.meta.ascension_level, &entity, 50, &plan);

    assert_eq!(actions.len(), 1);
    execute_action(actions[0].clone(), &mut state);
    assert!(state.entities.monsters[0].bronze_automaton.protocol_seeded);
    assert!(!state.entities.monsters[0].bronze_automaton.first_turn);
    assert_eq!(state.entities.monsters[0].bronze_automaton.num_turns, 3);
}

#[test]
fn bronze_automaton_take_turn_spawn_orbs_uses_expected_draw_x() {
    let mut entity = planned_monster(EnemyId::BronzeAutomaton, 4);
    entity.id = 10;
    let mut state = combat_with_monsters(vec![entity.clone()]);
    state.monster_protocol_identity_mut(10).draw_x = Some(927);
    let plan = BronzeAutomaton::turn_plan(&state, &entity);

    let actions = BronzeAutomaton::take_turn_plan(&mut state, &entity, &plan);

    assert_eq!(actions.len(), 3);
    assert!(matches!(
        actions[0],
        Action::SpawnMonsterSmart {
            monster_id: EnemyId::BronzeOrb,
            logical_position: 760,
            protocol_draw_x: Some(760),
            is_minion: true,
            ..
        }
    ));
    assert!(matches!(
        actions[1],
        Action::SpawnMonsterSmart {
            monster_id: EnemyId::BronzeOrb,
            logical_position: 1093,
            protocol_draw_x: Some(1093),
            is_minion: true,
            ..
        }
    ));
    assert!(matches!(
        actions[2],
        Action::RollMonsterMove { monster_id: 10 }
    ));
}

#[test]
fn bronze_automaton_on_death_suicides_surviving_allies() {
    let mut automaton = test_monster(EnemyId::BronzeAutomaton);
    automaton.id = 10;
    let mut orb = test_monster(EnemyId::BronzeOrb);
    orb.id = 11;
    let mut dying_orb = test_monster(EnemyId::BronzeOrb);
    dying_orb.id = 12;
    dying_orb.current_hp = 0;
    dying_orb.is_dying = true;
    let mut state = combat_with_monsters(vec![automaton.clone(), orb, dying_orb]);

    let actions = BronzeAutomaton::on_death(&mut state, &automaton);

    assert_eq!(actions, vec![Action::Suicide { target: 11 }]);
}

#[test]
fn bronze_orb_roll_move_plan_prefers_stasis_when_unused() {
    let state = blank_test_combat();
    let entity = test_monster(EnemyId::BronzeOrb);
    let mut rng = state.rng.ai_rng.clone();

    let plan = BronzeOrb::roll_move_plan(&mut rng, &entity, state.meta.ascension_level, 25);

    assert_eq!(plan.move_id, 3);
    assert_eq!(
        format!("{:?}", plan.summary_spec()),
        "StrongDebuff(DebuffSpec { power_id: Stasis, amount: 1, strength: Strong })"
    );
}

#[test]
fn bronze_orb_on_roll_move_marks_used_stasis() {
    let entity = planned_monster(EnemyId::BronzeOrb, 3);
    let mut state = combat_with_monsters(vec![entity.clone()]);
    let plan = BronzeOrb::turn_plan(&state, &entity);

    let actions = BronzeOrb::on_roll_move(state.meta.ascension_level, &entity, 25, &plan);

    assert_eq!(actions.len(), 1);
    execute_action(actions[0].clone(), &mut state);
    assert!(state.entities.monsters[0].bronze_orb.protocol_seeded);
    assert!(state.entities.monsters[0].bronze_orb.used_stasis);
}

#[test]
fn bronze_orb_take_turn_support_beam_targets_automaton() {
    let mut automaton = test_monster(EnemyId::BronzeAutomaton);
    automaton.id = 10;
    let mut orb = planned_monster(EnemyId::BronzeOrb, 2);
    orb.id = 11;
    let mut state = combat_with_monsters(vec![automaton, orb.clone()]);
    let plan = BronzeOrb::turn_plan(&state, &orb);

    let actions = BronzeOrb::take_turn_plan(&mut state, &orb, &plan);

    assert_eq!(
        actions,
        vec![
            Action::GainBlock {
                target: 10,
                amount: 12,
            },
            Action::RollMonsterMove { monster_id: 11 },
        ]
    );
}
