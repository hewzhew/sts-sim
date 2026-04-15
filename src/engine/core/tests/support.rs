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
pub(super) use crate::engine::test_support::{build_combat_state, basic_combat, CombatTestExt};
pub(super) use serde_json::json;
pub(super) use std::collections::{HashMap, VecDeque};

pub(super) fn test_combat() -> CombatState {
    basic_combat()
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
