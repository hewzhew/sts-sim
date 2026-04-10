#![allow(unused_imports)]

use super::super::*;
pub(super) use crate::action::Action;
pub(super) use crate::combat::{
    CombatCard, CombatMeta, CombatRng, EngineRuntime, Intent, MonsterEntity, PlayerEntity, Power,
    QueuedCardPlay, QueuedCardSource, RelicBuses, StanceId, TurnRuntime,
};
pub(super) use crate::content::cards::CardId;
pub(super) use crate::content::powers::PowerId;
pub(super) use crate::content::relics::{RelicId, RelicState};
pub(super) use crate::diff::state_sync::build_combat_state;
pub(super) use serde_json::json;
pub(super) use std::collections::{HashMap, VecDeque};

pub(super) fn test_combat() -> CombatState {
    CombatState {
        meta: CombatMeta {
            ascension_level: 0,
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime {
            turn_count: 1,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            turn_start_draw_modifier: 0,
            counters: Default::default(),
        },
        zones: crate::combat::CardZones {
            draw_pile: Vec::new(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            queued_cards: VecDeque::new(),
            card_uuid_counter: 10,
        },
        entities: crate::combat::EntityState {
            player: PlayerEntity {
                id: 0,
                current_hp: 80,
                max_hp: 80,
                block: 0,
                gold_delta_this_combat: 0,
                gold: 99,
                max_orbs: 0,
                orbs: Vec::new(),
                stance: StanceId::Neutral,
                relics: Vec::new(),
                relic_buses: RelicBuses::default(),
                energy_master: 3,
            },
            monsters: vec![MonsterEntity {
                id: 1,
                monster_type: crate::content::monsters::EnemyId::JawWorm as usize,
                current_hp: 40,
                max_hp: 40,
                block: 0,
                slot: 0,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Unknown,
                move_history: VecDeque::new(),
                intent_dmg: 0,
                logical_position: 0,
                hexaghost: Default::default(),
                darkling: Default::default(),
            }],
            potions: vec![None, None, None],
            power_db: HashMap::new(),
        },
        engine: EngineRuntime {
            action_queue: VecDeque::new(),
        },
        rng: CombatRng::new(crate::rng::RngPool::new(123)),
    }
}

pub(super) fn live_snapshot_with_strike_dummy(
    hand: serde_json::Value,
    draw_pile: serde_json::Value,
    monsters: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "turn": 3,
        "room_type": "MonsterRoom",
        "player": {
            "current_hp": 50,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": [
                {"id": "Strength", "amount": 3}
            ]
        },
        "monsters": monsters,
        "relics": [
            {"id": "StrikeDummy", "counter": -1}
        ],
        "hand": hand,
        "draw_pile": draw_pile,
        "discard_pile": [],
        "exhaust_pile": []
    })
}

pub(super) fn drain_processing(engine_state: &mut EngineState, combat: &mut CombatState) {
    let mut iterations = 0;
    while (!combat.engine.action_queue.is_empty() || !combat.zones.queued_cards.is_empty())
        && iterations < 64
    {
        *engine_state = EngineState::CombatProcessing;
        assert!(tick_engine(engine_state, combat, None));
        iterations += 1;
    }
    assert!(iterations < 64, "combat queue failed to drain");
}
