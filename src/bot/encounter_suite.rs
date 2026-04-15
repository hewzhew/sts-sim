use crate::runtime::combat::{
    CombatPhase, CombatRng, CombatState, EngineRuntime, EntityState, TurnRuntime,
};
use crate::content::monsters::factory::{self, EncounterId};
use crate::runtime::rng::{self, RngPool};
use crate::state::core::EngineState;
use crate::state::run::RunState;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterSuiteId {
    Act1Pathing,
    Act2Pathing,
    Act3Pathing,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EncounterSuiteWeights {
    pub frontload: i32,
    pub block: i32,
    pub scaling: i32,
    pub deck_thinning: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EncounterSuiteEntry {
    pub encounter: EncounterId,
    pub weight: i32,
    pub is_elite: bool,
    pub is_boss: bool,
    pub seed_offset: u64,
}

const ACT1_ENTRIES: [EncounterSuiteEntry; 4] = [
    EncounterSuiteEntry {
        encounter: EncounterId::JawWorm,
        weight: 3,
        is_elite: false,
        is_boss: false,
        seed_offset: 0x11,
    },
    EncounterSuiteEntry {
        encounter: EncounterId::ThreeLouse,
        weight: 2,
        is_elite: false,
        is_boss: false,
        seed_offset: 0x12,
    },
    EncounterSuiteEntry {
        encounter: EncounterId::GremlinNob,
        weight: 3,
        is_elite: true,
        is_boss: false,
        seed_offset: 0x13,
    },
    EncounterSuiteEntry {
        encounter: EncounterId::Hexaghost,
        weight: 4,
        is_elite: false,
        is_boss: true,
        seed_offset: 0x14,
    },
];

const ACT2_ENTRIES: [EncounterSuiteEntry; 4] = [
    EncounterSuiteEntry {
        encounter: EncounterId::CultistAndChosen,
        weight: 2,
        is_elite: false,
        is_boss: false,
        seed_offset: 0x21,
    },
    EncounterSuiteEntry {
        encounter: EncounterId::ShelledParasiteAndFungi,
        weight: 2,
        is_elite: false,
        is_boss: false,
        seed_offset: 0x22,
    },
    EncounterSuiteEntry {
        encounter: EncounterId::BookOfStabbing,
        weight: 3,
        is_elite: true,
        is_boss: false,
        seed_offset: 0x23,
    },
    EncounterSuiteEntry {
        encounter: EncounterId::Automaton,
        weight: 4,
        is_elite: false,
        is_boss: true,
        seed_offset: 0x24,
    },
];

const ACT3_ENTRIES: [EncounterSuiteEntry; 4] = [
    EncounterSuiteEntry {
        encounter: EncounterId::SpireGrowth,
        weight: 2,
        is_elite: false,
        is_boss: false,
        seed_offset: 0x31,
    },
    EncounterSuiteEntry {
        encounter: EncounterId::JawWormHorde,
        weight: 2,
        is_elite: false,
        is_boss: false,
        seed_offset: 0x32,
    },
    EncounterSuiteEntry {
        encounter: EncounterId::Reptomancer,
        weight: 3,
        is_elite: true,
        is_boss: false,
        seed_offset: 0x33,
    },
    EncounterSuiteEntry {
        encounter: EncounterId::TimeEater,
        weight: 4,
        is_elite: false,
        is_boss: true,
        seed_offset: 0x34,
    },
];

pub(crate) fn suite_for_run(rs: &RunState) -> EncounterSuiteId {
    match rs.act_num {
        1 => EncounterSuiteId::Act1Pathing,
        2 => EncounterSuiteId::Act2Pathing,
        _ => EncounterSuiteId::Act3Pathing,
    }
}

pub(crate) fn weights_for_suite(suite: EncounterSuiteId) -> EncounterSuiteWeights {
    match suite {
        EncounterSuiteId::Act1Pathing => EncounterSuiteWeights {
            frontload: 16,
            block: 10,
            scaling: 4,
            deck_thinning: 14,
        },
        EncounterSuiteId::Act2Pathing => EncounterSuiteWeights {
            frontload: 8,
            block: 16,
            scaling: 10,
            deck_thinning: 8,
        },
        EncounterSuiteId::Act3Pathing => EncounterSuiteWeights {
            frontload: 6,
            block: 12,
            scaling: 14,
            deck_thinning: 6,
        },
    }
}

pub(crate) fn rollout_entries_for_suite(suite: EncounterSuiteId) -> &'static [EncounterSuiteEntry] {
    match suite {
        EncounterSuiteId::Act1Pathing => &ACT1_ENTRIES,
        EncounterSuiteId::Act2Pathing => &ACT2_ENTRIES,
        EncounterSuiteId::Act3Pathing => &ACT3_ENTRIES,
    }
}

pub(crate) fn start_suite_encounter(
    rs: &RunState,
    entry: EncounterSuiteEntry,
) -> (EngineState, CombatState) {
    let mut seeded = rs.clone();
    seeded.rng_pool = RngPool::new(rs.seed ^ entry.seed_offset.rotate_left(17));

    let player = seeded.build_combat_player(0);
    let monsters = factory::build_encounter(
        entry.encounter,
        &mut seeded.rng_pool.misc_rng,
        &mut seeded.rng_pool.monster_hp_rng,
        seeded.ascension_level,
    );

    let mut combat = CombatState {
        meta: crate::runtime::combat::CombatMeta {
            ascension_level: seeded.ascension_level,
            player_class: seeded.player_class,
            is_boss_fight: entry.is_boss,
            is_elite_fight: entry.is_elite,
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime {
            turn_count: 0,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            turn_start_draw_modifier: 0,
            counters: Default::default(),
        },
        zones: crate::runtime::combat::CardZones {
            draw_pile: seeded.master_deck.clone(),
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
            potions: seeded.potions.clone(),
            power_db: HashMap::new(),
        },
        engine: EngineRuntime {
            action_queue: VecDeque::new(),
        },
        rng: CombatRng::new(seeded.rng_pool.clone()),
        runtime: Default::default(),
    };

    let monsters_clone = combat.entities.monsters.clone();
    for monster in &mut combat.entities.monsters {
        let num = combat.rng.ai_rng.random(99);
        let (move_byte, intent) = crate::content::monsters::roll_monster_move(
            &mut combat.rng.ai_rng,
            monster,
            combat.meta.ascension_level,
            num,
            &monsters_clone,
        );
        monster.next_move_byte = move_byte;
        monster.current_intent = intent;
        monster.move_history.push_back(move_byte);
    }

    combat.turn.energy = combat.entities.player.energy_master;
    rng::shuffle_with_random_long(&mut combat.zones.draw_pile, &mut combat.rng.shuffle_rng);
    let mut innate = Vec::new();
    let mut normal = Vec::new();
    for card in std::mem::take(&mut combat.zones.draw_pile) {
        if crate::content::cards::is_innate_card(&card) {
            innate.push(card);
        } else {
            normal.push(card);
        }
    }
    innate.extend(normal);
    combat.zones.draw_pile = innate;
    combat
        .engine
        .action_queue
        .push_back(crate::runtime::action::Action::PreBattleTrigger);

    let mut engine = EngineState::CombatProcessing;
    advance_suite_engine(&mut engine, &mut combat);
    (engine, combat)
}

pub(crate) fn advance_suite_engine(engine: &mut EngineState, combat: &mut CombatState) {
    let mut iterations = 0;
    loop {
        match engine {
            EngineState::CombatPlayerTurn
            | EngineState::PendingChoice(_)
            | EngineState::GameOver(_) => break,
            EngineState::CombatProcessing => {}
            _ => break,
        }

        if !crate::engine::core::with_suppressed_engine_warnings(|| {
            crate::engine::core::tick_engine(engine, combat, None)
        }) {
            break;
        }

        if *engine == EngineState::CombatPlayerTurn
            && (!combat.engine.action_queue.is_empty() || !combat.zones.queued_cards.is_empty())
        {
            *engine = EngineState::CombatProcessing;
        }

        iterations += 1;
        if iterations > 2_000 {
            break;
        }
    }
}
