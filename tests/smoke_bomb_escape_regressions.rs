use serde_json::{json, Value};
use sts_simulator::content::powers::{store, PowerId};
use sts_simulator::diff::state_sync::build_combat_state_from_snapshots;
use sts_simulator::engine::core::tick_until_stable_turn;
use sts_simulator::state::core::{ClientInput, EngineState};

fn strike(uuid: &str) -> Value {
    json!({
        "id": "Strike_R",
        "name": "Strike",
        "uuid": uuid,
        "type": "ATTACK",
        "cost": 1,
        "upgrades": 0,
        "base_damage": 6,
        "rarity": "BASIC",
        "has_target": true,
        "ethereal": false,
        "exhausts": false,
        "is_playable": true
    })
}

fn defend(uuid: &str) -> Value {
    json!({
        "id": "Defend_R",
        "name": "Defend",
        "uuid": uuid,
        "type": "SKILL",
        "cost": 1,
        "upgrades": 0,
        "rarity": "BASIC",
        "has_target": false,
        "ethereal": false,
        "exhausts": false,
        "is_playable": true
    })
}

fn potion_slots() -> Value {
    json!([
        { "id": "Potion Slot", "name": "Potion Slot" },
        { "id": "Potion Slot", "name": "Potion Slot" },
        { "id": "Potion Slot", "name": "Potion Slot" }
    ])
}

fn smoke_escape_snapshots() -> (Value, Value) {
    let truth = json!({
        "turn": 2,
        "room_type": "MonsterRoom",
        "room_smoked": true,
        "player": {
            "current_hp": 30,
            "max_hp": 85,
            "block": 0,
            "energy": 0,
            "is_escaping": true,
            "escape_timer": 2.5,
            "powers": [
                { "id": "Demon Form", "name": "Demon Form", "amount": 3 }
            ]
        },
        "relics": [
            {
                "id": "Burning Blood",
                "name": "Burning Blood",
                "runtime_state": {
                    "used_up": false,
                    "counter": -1
                }
            }
        ],
        "monsters": [
            {
                "id": "Cultist",
                "name": "Cultist",
                "current_hp": 48,
                "max_hp": 48,
                "block": 0,
                "move_id": 1,
                "move_base_damage": 6,
                "move_hits": 1,
                "powers": [],
                "is_gone": false,
                "half_dead": false
            }
        ],
        "hand": [
            strike("strike-1"),
            defend("defend-1")
        ],
        "draw_pile": [
            strike("strike-2")
        ],
        "discard_pile_ids": [],
        "exhaust_pile_ids": [],
        "limbo": [],
        "card_queue": [],
        "potions": potion_slots()
    });

    let observation = json!({
        "turn": 2,
        "room_type": "MonsterRoom",
        "room_smoked": true,
        "using_card": false,
        "player": {
            "current_hp": 30,
            "max_hp": 85,
            "block": 0,
            "energy": 0,
            "is_escaping": true,
            "escape_timer": 2.5,
            "powers": [
                { "id": "Demon Form", "name": "Demon Form", "amount": 3 }
            ]
        },
        "monsters": [
            {
                "id": "Cultist",
                "name": "Cultist",
                "current_hp": 48,
                "max_hp": 48,
                "block": 0,
                "intent": "ATTACK",
                "move_adjusted_damage": 6,
                "move_hits": 1,
                "monster_instance_id": 1,
                "spawn_order": 1,
                "monster_index": 0,
                "draw_x": 920,
                "powers": [],
                "is_gone": false,
                "half_dead": false,
                "is_dying": false,
                "is_escaping": false
            }
        ],
        "hand": truth["hand"].clone(),
        "discard_pile": [],
        "exhaust_pile": [],
        "draw_pile_count": 1,
        "cards_discarded_this_turn": 0,
        "times_damaged": 0,
        "potions": truth["potions"].clone(),
        "relics": truth["relics"].clone(),
        "limbo": []
    });

    (truth, observation)
}

#[test]
fn state_sync_imports_smoke_escape_truth() {
    let (truth, observation) = smoke_escape_snapshots();
    let combat = build_combat_state_from_snapshots(&truth, &observation, &truth["relics"]);

    assert!(combat.runtime.combat_smoked);
    assert!(combat.turn.counters.player_escaping);
    assert_eq!(store::power_amount(&combat, 0, PowerId::DemonForm), 3);
}

#[test]
fn smoke_bomb_end_turn_stops_before_next_turn_refresh() {
    let (truth, observation) = smoke_escape_snapshots();
    let mut combat = build_combat_state_from_snapshots(&truth, &observation, &truth["relics"]);
    let mut engine_state = EngineState::CombatPlayerTurn;

    assert!(tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::EndTurn,
    ));

    assert!(matches!(engine_state, EngineState::CombatProcessing));
    assert!(combat.runtime.combat_smoked);
    assert_eq!(combat.entities.player.current_hp, 36);
    assert_eq!(combat.turn.energy, 0);
    assert!(combat.turn.counters.player_escaping);
    assert!(combat.turn.counters.victory_triggered);
    assert!(combat.turn.counters.escape_pending_reward);
    assert!(combat.zones.hand.is_empty());
    assert_eq!(combat.zones.draw_pile.len(), 1);
    assert_eq!(store::power_amount(&combat, 0, PowerId::DemonForm), 3);
    assert!(!store::has_power(&combat, 0, PowerId::Strength));
}
