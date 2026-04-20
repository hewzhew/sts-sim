//! Test-only builders shared by behavior and semantic integration tests.

use crate::content::monsters::EnemyId;
use crate::runtime::combat::{
    CardZones, CombatMeta, CombatPhase, CombatRng, CombatRuntimeHints, CombatState, EngineRuntime,
    EntityState, EphemeralCounters, MonsterEntity, MonsterMoveState, PlayerEntity, RelicBuses,
    StanceId, TurnRuntime,
};
use crate::runtime::rng::RngPool;
use crate::state::core::PendingChoice;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};

pub fn continue_deferred_pending_choice_legacy(
    pending: &PendingChoice,
    combat: &mut CombatState,
    snapshot_hint: &Value,
) -> Result<bool, String> {
    crate::diff::replay::continue_deferred_pending_choice_legacy(pending, combat, snapshot_hint)
}

pub fn blank_test_combat() -> CombatState {
    CombatState {
        meta: CombatMeta {
            ascension_level: 0,
            player_class: "Ironclad",
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime {
            turn_count: 1,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            turn_start_draw_modifier: 0,
            counters: EphemeralCounters::default(),
        },
        zones: CardZones {
            draw_pile: vec![],
            hand: vec![],
            discard_pile: vec![],
            exhaust_pile: vec![],
            limbo: vec![],
            queued_cards: VecDeque::new(),
            card_uuid_counter: 0,
        },
        entities: EntityState {
            player: PlayerEntity {
                id: 0,
                current_hp: 80,
                max_hp: 80,
                block: 0,
                gold_delta_this_combat: 0,
                gold: 99,
                max_orbs: 0,
                orbs: vec![],
                stance: StanceId::Neutral,
                relics: vec![],
                relic_buses: RelicBuses::default(),
                energy_master: 3,
            },
            monsters: vec![],
            potions: vec![],
            power_db: HashMap::new(),
        },
        engine: EngineRuntime::new(),
        rng: CombatRng::new(RngPool::new(123)),
        runtime: CombatRuntimeHints::default(),
    }
}

pub fn combat_with_monsters(monsters: Vec<MonsterEntity>) -> CombatState {
    let mut state = blank_test_combat();
    state.entities.monsters = monsters;
    state
}

pub fn test_monster(enemy_id: EnemyId) -> MonsterEntity {
    let mut monster = MonsterEntity {
        id: 1,
        monster_type: enemy_id as usize,
        current_hp: 20,
        max_hp: 20,
        block: 0,
        slot: 0,
        is_dying: false,
        is_escaped: false,
        half_dead: false,
        move_state: MonsterMoveState::default(),
        logical_position: 0,
        hexaghost: Default::default(),
        louse: Default::default(),
        jaw_worm: Default::default(),
        thief: Default::default(),
        byrd: Default::default(),
        chosen: Default::default(),
        snecko: Default::default(),
        shelled_parasite: Default::default(),
        bronze_automaton: Default::default(),
        bronze_orb: Default::default(),
        book_of_stabbing: Default::default(),
        collector: Default::default(),
        champ: Default::default(),
        awakened_one: Default::default(),
        corrupt_heart: Default::default(),
        darkling: Default::default(),
        lagavulin: Default::default(),
        guardian: Default::default(),
    };

    if enemy_id == EnemyId::Byrd {
        monster.byrd.first_move = true;
        monster.byrd.is_flying = true;
        monster.byrd.protocol_seeded = true;
    }
    if enemy_id == EnemyId::Chosen {
        monster.chosen.first_turn = true;
        monster.chosen.used_hex = false;
        monster.chosen.protocol_seeded = true;
    }
    if enemy_id == EnemyId::Snecko {
        monster.snecko.first_turn = true;
        monster.snecko.protocol_seeded = true;
    }
    if enemy_id == EnemyId::ShelledParasite {
        monster.shelled_parasite.first_move = true;
        monster.shelled_parasite.protocol_seeded = true;
    }
    if enemy_id == EnemyId::BronzeAutomaton {
        monster.bronze_automaton.protocol_seeded = true;
        monster.bronze_automaton.first_turn = true;
        monster.bronze_automaton.num_turns = 0;
    }
    if enemy_id == EnemyId::BronzeOrb {
        monster.bronze_orb.protocol_seeded = true;
        monster.bronze_orb.used_stasis = false;
    }
    if enemy_id == EnemyId::BookOfStabbing {
        monster.book_of_stabbing.protocol_seeded = true;
        monster.book_of_stabbing.stab_count = 1;
    }
    if enemy_id == EnemyId::TheCollector {
        monster.collector.protocol_seeded = true;
        monster.collector.initial_spawn = true;
        monster.collector.ult_used = false;
        monster.collector.turns_taken = 0;
    }
    if enemy_id == EnemyId::Champ {
        monster.champ.protocol_seeded = true;
        monster.champ.first_turn = true;
        monster.champ.num_turns = 0;
        monster.champ.forge_times = 0;
        monster.champ.threshold_reached = false;
    }
    if enemy_id == EnemyId::AwakenedOne {
        monster.awakened_one.protocol_seeded = true;
        monster.awakened_one.form1 = true;
        monster.awakened_one.first_turn = true;
    }
    if enemy_id == EnemyId::CorruptHeart {
        monster.corrupt_heart.protocol_seeded = true;
        monster.corrupt_heart.first_move = true;
        monster.corrupt_heart.move_count = 0;
        monster.corrupt_heart.buff_count = 0;
    }
    if matches!(enemy_id, EnemyId::Looter | EnemyId::Mugger) {
        monster.thief.protocol_seeded = true;
        monster.thief.slash_count = 0;
        monster.thief.stolen_gold = 0;
    }

    monster
}

pub fn planned_monster(enemy_id: EnemyId, move_id: u8) -> MonsterEntity {
    let mut monster = test_monster(enemy_id);
    monster.set_planned_move_id(move_id);
    monster
}

pub fn monster_with_history(enemy_id: EnemyId, move_id: u8, history: &[u8]) -> MonsterEntity {
    let mut monster = planned_monster(enemy_id, move_id);
    monster.move_history_mut().extend(history.iter().copied());
    monster
}
