use std::collections::{HashMap, VecDeque};

use crate::content::monsters::factory::{self, EncounterId};
use crate::engine::core::{
    is_smoke_escape_stable_boundary, tick_engine, with_suppressed_engine_warnings,
};
use crate::runtime::action::Action;
use crate::runtime::combat::{CardZones, CombatMeta, TurnRuntime};
use crate::runtime::combat::{CombatRng, CombatState, EngineRuntime, EntityState};
use crate::runtime::rng;
use crate::state::core::EngineState;
use crate::state::map::node::RoomType;
use crate::state::run::RunState;
use crate::state::selection::{EngineDiagnostic, EngineDiagnosticClass, EngineDiagnosticSeverity};

pub fn build_natural_combat_start(
    run_state: &mut RunState,
    encounter_id: EncounterId,
    room_type: RoomType,
) -> Result<(EngineState, CombatState), String> {
    let player = run_state.build_combat_player(0);
    let monsters = factory::build_encounter(
        encounter_id,
        &mut run_state.rng_pool.misc_rng,
        &mut run_state.rng_pool.monster_hp_rng,
        run_state.ascension_level,
    );

    let mut combat = CombatState {
        meta: CombatMeta {
            ascension_level: run_state.ascension_level,
            player_class: run_state.player_class.to_string(),
            is_boss_fight: room_type == RoomType::MonsterRoomBoss,
            is_elite_fight: room_type == RoomType::MonsterRoomElite,
            master_deck_snapshot: run_state.master_deck.clone(),
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime::fresh_player_turn(3),
        zones: CardZones {
            draw_pile: run_state.master_deck.clone(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            queued_cards: VecDeque::new(),
            card_uuid_counter: 9999,
        },
        entities: EntityState {
            player,
            monsters,
            potions: run_state.potions.clone(),
            power_db: HashMap::new(),
        },
        engine: EngineRuntime::new(),
        rng: CombatRng::new(run_state.rng_pool.clone()),
        runtime: Default::default(),
    };

    roll_initial_monster_plans(&mut combat);

    combat.reset_turn_energy_from_player();
    rng::shuffle_with_random_long(&mut combat.zones.draw_pile, &mut combat.rng.shuffle_rng);
    combat.apply_java_initialize_deck_order_after_shuffle();
    combat.queue_action_back(Action::PreBattleTrigger);

    let mut engine_state = EngineState::CombatProcessing;
    let alive = with_suppressed_engine_warnings(|| {
        drain_to_player_boundary(&mut engine_state, &mut combat)
    });
    if !alive {
        return Err("combat initialization reached terminal state before player input".to_string());
    }
    if !matches!(
        engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return Err(format!(
            "combat initialization did not reach stable player turn, got {engine_state:?}"
        ));
    }

    Ok((engine_state, combat))
}

pub fn encounter_id_from_input(raw: &str) -> Result<EncounterId, String> {
    let normalized = normalize_identifier(raw);
    match normalized.as_str() {
        "blueslaver" => Ok(EncounterId::BlueSlaver),
        "cultist" => Ok(EncounterId::Cultist),
        "jawworm" => Ok(EncounterId::JawWorm),
        "looter" => Ok(EncounterId::Looter),
        "gremlingang" => Ok(EncounterId::GremlinGang),
        "redslaver" => Ok(EncounterId::RedSlaver),
        "largeslime" => Ok(EncounterId::LargeSlime),
        "exordiumthugs" => Ok(EncounterId::ExordiumThugs),
        "exordiumwildlife" => Ok(EncounterId::ExordiumWildlife),
        "threelouse" | "3louse" => Ok(EncounterId::ThreeLouse),
        "twolouse" | "2louse" => Ok(EncounterId::TwoLouse),
        "twofungibeasts" | "2fungibeasts" => Ok(EncounterId::TwoFungiBeasts),
        "lotsofslimes" => Ok(EncounterId::LotsOfSlimes),
        "smallslimes" => Ok(EncounterId::SmallSlimes),
        "gremlinnob" | "nob" => Ok(EncounterId::GremlinNob),
        "lagavulin" => Ok(EncounterId::Lagavulin),
        "lagavulinevent" => Ok(EncounterId::LagavulinEvent),
        "threesentries" | "3sentries" | "sentries" => Ok(EncounterId::ThreeSentries),
        "themushroomlair" | "mushroomlair" => Ok(EncounterId::TheMushroomLair),
        "theguardian" | "guardian" => Ok(EncounterId::TheGuardian),
        "hexaghost" => Ok(EncounterId::Hexaghost),
        "slimeboss" => Ok(EncounterId::SlimeBoss),
        "twothieves" | "2thieves" => Ok(EncounterId::TwoThieves),
        "threebyrds" | "3byrds" => Ok(EncounterId::ThreeByrds),
        "chosenalone" | "chosen" => Ok(EncounterId::ChosenAlone),
        "shellparasite" => Ok(EncounterId::ShellParasite),
        "sphericguardian" => Ok(EncounterId::SphericGuardian),
        "chosenandbyrds" => Ok(EncounterId::ChosenAndByrds),
        "sentryandsphere" => Ok(EncounterId::SentryAndSphere),
        "snakeplant" => Ok(EncounterId::SnakePlant),
        "snecko" => Ok(EncounterId::Snecko),
        "centurionandhealer" => Ok(EncounterId::CenturionAndHealer),
        "cultistandchosen" => Ok(EncounterId::CultistAndChosen),
        "threecultists" | "3cultists" => Ok(EncounterId::ThreeCultists),
        "shelledparasiteandfungi" => Ok(EncounterId::ShelledParasiteAndFungi),
        "gremlinleader" => Ok(EncounterId::GremlinLeader),
        "slavers" => Ok(EncounterId::Slavers),
        "bookofstabbing" => Ok(EncounterId::BookOfStabbing),
        "automaton" | "bronzeautomaton" => Ok(EncounterId::Automaton),
        "thechamp" | "champ" => Ok(EncounterId::TheChamp),
        "collector" | "thecollector" => Ok(EncounterId::Collector),
        "maskedbandits" => Ok(EncounterId::MaskedBandits),
        "colosseumslavers" => Ok(EncounterId::ColosseumSlavers),
        "colosseumnobs" => Ok(EncounterId::ColosseumNobs),
        "threedarklings" | "3darklings" => Ok(EncounterId::ThreeDarklings),
        "orbwalker" => Ok(EncounterId::OrbWalker),
        "threeshapes" | "3shapes" => Ok(EncounterId::ThreeShapes),
        "spiregrowth" => Ok(EncounterId::SpireGrowth),
        "transient" | "thetransient" => Ok(EncounterId::Transient),
        "fourshapes" | "4shapes" => Ok(EncounterId::FourShapes),
        "themaw" | "maw" => Ok(EncounterId::TheMaw),
        "sphereandtwoshapes" | "sphereand2shapes" => Ok(EncounterId::SphereAndTwoShapes),
        "jawwormhorde" => Ok(EncounterId::JawWormHorde),
        "writhingmass" => Ok(EncounterId::WrithingMass),
        "gianthead" => Ok(EncounterId::GiantHead),
        "thenemesis" | "nemesis" => Ok(EncounterId::TheNemesis),
        "reptomancer" => Ok(EncounterId::Reptomancer),
        "awakenedone" | "theawakenedone" => Ok(EncounterId::AwakenedOne),
        "timeeater" => Ok(EncounterId::TimeEater),
        "donuanddeca" => Ok(EncounterId::DonuAndDeca),
        "mysterioussphere" => Ok(EncounterId::MysteriousSphere),
        "twoorbwalkers" | "2orbwalkers" => Ok(EncounterId::TwoOrbWalkers),
        "sneckoandmystics" => Ok(EncounterId::SneckoAndMystics),
        "shieldandspear" | "spearandshield" => Ok(EncounterId::ShieldAndSpear),
        "theheart" | "corruptheart" | "heart" => Ok(EncounterId::TheHeart),
        _ => Err(format!("unsupported encounter_id '{raw}'")),
    }
}

pub fn room_type_from_input(raw: &str) -> Result<RoomType, String> {
    let normalized = normalize_identifier(raw);
    match normalized.as_str() {
        "monsterroomboss" | "boss" => Ok(RoomType::MonsterRoomBoss),
        "monsterroomelite" | "elite" => Ok(RoomType::MonsterRoomElite),
        "monsterroom" | "monster" => Ok(RoomType::MonsterRoom),
        _ => Err(format!("unsupported room_type '{raw}'")),
    }
}

fn roll_initial_monster_plans(combat: &mut CombatState) {
    let monsters_clone = combat.entities.monsters.clone();
    let player_powers = crate::content::powers::store::powers_snapshot_for(combat, 0);
    let monster_ids = combat
        .entities
        .monsters
        .iter()
        .map(|monster| monster.id)
        .collect::<Vec<_>>();
    for monster_id in monster_ids {
        let entity_snapshot = combat
            .entities
            .monsters
            .iter()
            .find(|monster| monster.id == monster_id)
            .cloned()
            .expect("initial monster should exist while rolling intent");
        let num = combat.rng.ai_rng.random(99);
        let outcome = crate::content::monsters::roll_monster_turn_outcome(
            &mut combat.rng.ai_rng,
            &entity_snapshot,
            combat.meta.ascension_level,
            num,
            &monsters_clone,
            &player_powers,
        );
        for action in outcome.setup_actions {
            crate::engine::action_handlers::execute_action(action, combat);
        }
        let plan = outcome.plan;
        let monster = combat
            .entities
            .monsters
            .iter_mut()
            .find(|monster| monster.id == monster_id)
            .expect("rolled monster should still exist");
        monster.set_planned_move_id(plan.move_id);
        monster.set_planned_steps(plan.steps);
        monster.set_planned_visible_spec(plan.visible_spec);
        monster.move_history_mut().push_back(plan.move_id);
        combat
            .runtime
            .monster_protocol
            .entry(monster_id)
            .or_default()
            .observation = Default::default();
    }
}

fn drain_to_player_boundary(es: &mut EngineState, cs: &mut CombatState) -> bool {
    let mut iterations = 0;
    loop {
        match es {
            EngineState::CombatPlayerTurn => break,
            EngineState::CombatProcessing if is_smoke_escape_stable_boundary(es, cs) => break,
            EngineState::CombatProcessing => {}
            EngineState::PendingChoice(_) => break,
            EngineState::GameOver(_) => return false,
            _ => break,
        }
        let alive = tick_engine(es, cs, None);
        if !alive {
            return false;
        }
        iterations += 1;
        if iterations > 1000 {
            cs.emit_diagnostic(EngineDiagnostic {
                severity: EngineDiagnosticSeverity::Warning,
                class: EngineDiagnosticClass::Suspicious,
                message: "tick loop exceeded 1000 iterations".to_string(),
            });
            break;
        }
    }
    true
}

fn normalize_identifier(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::build_natural_combat_start;
    use crate::content::monsters::factory::EncounterId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::OrbId;
    use crate::state::map::node::RoomType;
    use crate::state::run::RunState;

    #[test]
    fn natural_combat_start_applies_ring_of_the_serpent_opening_hand_size() {
        let mut run = RunState::new(1, 0, false, "Silent");
        run.relics = vec![RelicState::new(RelicId::RingOfTheSerpent)];

        let (_engine_state, combat) =
            build_natural_combat_start(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");

        assert_eq!(
            combat.zones.hand.len(),
            6,
            "Java Ring of the Serpent increments masterHandSize, so initial draw uses 6"
        );
    }

    #[test]
    fn natural_defect_combat_start_has_java_orb_slots_before_cracked_core() {
        let mut run = RunState::new(1, 0, false, "Defect");

        let (_engine_state, combat) =
            build_natural_combat_start(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");

        assert_eq!(combat.entities.player.max_orbs, 3);
        assert_eq!(combat.entities.player.orbs.len(), 3);
        assert_eq!(
            combat.entities.player.orbs[0].id,
            OrbId::Lightning,
            "Java Defect starts combat with 3 master orb slots before Cracked Core channels Lightning"
        );
        assert!(combat
            .entities
            .player
            .orbs
            .iter()
            .skip(1)
            .all(|orb| orb.id == OrbId::Empty));
    }

    #[test]
    fn natural_non_defect_prismatic_shard_combat_start_has_one_empty_orb_slot() {
        let mut run = RunState::new(1, 0, false, "Silent");
        run.relics.push(RelicState::new(RelicId::PrismaticShard));

        let (_engine_state, combat) =
            build_natural_combat_start(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");

        assert_eq!(
            combat.entities.player.max_orbs, 1,
            "Java PrismaticShard.onEquip grants one master orb slot to non-Defect classes"
        );
        assert_eq!(combat.entities.player.orbs.len(), 1);
        assert_eq!(combat.entities.player.orbs[0].id, OrbId::Empty);
    }
}
