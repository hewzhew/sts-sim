use serde_json::json;
use sts_simulator::diff::state_sync::build_combat_state_from_snapshots;

fn base_snapshot() -> serde_json::Value {
    json!({
        "turn": 1,
        "player": {
            "current_hp": 80,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": []
        },
        "monsters": [],
        "hand": []
    })
}

#[test]
#[should_panic(expected = "monster.runtime_state.guardian_threshold missing")]
fn guardian_threshold_requires_runtime_state() {
    let mut snapshot = base_snapshot();
    snapshot["monsters"] = json!([{
        "id": "TheGuardian",
        "current_hp": 250,
        "max_hp": 250,
        "block": 0,
        "intent": "UNKNOWN",
        "move_id": 0,
        "move_base_damage": -1,
        "move_adjusted_damage": -1,
        "move_hits": 1,
        "powers": [],
        "runtime_state": {},
        "is_gone": false,
        "half_dead": false
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "monster.runtime_state.damage_taken missing for TheGuardian")]
fn guardian_runtime_requires_damage_taken() {
    let mut snapshot = base_snapshot();
    snapshot["monsters"] = json!([{
        "id": "TheGuardian",
        "current_hp": 250,
        "max_hp": 250,
        "block": 0,
        "intent": "UNKNOWN",
        "move_id": 0,
        "move_base_damage": -1,
        "move_adjusted_damage": -1,
        "move_hits": 1,
        "powers": [],
        "runtime_state": {
            "guardian_threshold": 30
        },
        "is_gone": false,
        "half_dead": false
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "monster.runtime_state.divider_damage missing for Hexaghost")]
fn hexaghost_runtime_requires_divider_damage() {
    let mut snapshot = base_snapshot();
    snapshot["monsters"] = json!([{
        "id": "Hexaghost",
        "current_hp": 250,
        "max_hp": 250,
        "block": 0,
        "intent": "ATTACK",
        "move_id": 1,
        "move_base_damage": 2,
        "move_adjusted_damage": 2,
        "move_hits": 6,
        "powers": [],
        "runtime_state": {
            "activated": true,
            "orb_active_count": 6,
            "burn_upgraded": false
        },
        "is_gone": false,
        "half_dead": false
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "monster.runtime_state.is_open missing for TheGuardian")]
fn guardian_runtime_requires_is_open() {
    let mut snapshot = base_snapshot();
    snapshot["monsters"] = json!([{
        "id": "TheGuardian",
        "current_hp": 250,
        "max_hp": 250,
        "block": 0,
        "intent": "UNKNOWN",
        "move_id": 0,
        "move_base_damage": -1,
        "move_adjusted_damage": -1,
        "move_hits": 1,
        "powers": [],
        "runtime_state": {
            "guardian_threshold": 30,
            "damage_taken": 0
        },
        "is_gone": false,
        "half_dead": false
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "monster.runtime_state.angry_amount missing")]
fn angry_requires_runtime_state() {
    let mut snapshot = base_snapshot();
    snapshot["monsters"] = json!([{
        "id": "GremlinWarrior",
        "current_hp": 14,
        "max_hp": 14,
        "block": 0,
        "intent": "ATTACK",
        "move_id": 3,
        "move_base_damage": 5,
        "move_adjusted_damage": 5,
        "move_hits": 1,
        "powers": [],
        "runtime_state": {},
        "is_gone": false,
        "half_dead": false
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "power.runtime_state.hp_loss missing for Combust")]
fn combust_requires_runtime_state_hp_loss() {
    let mut snapshot = base_snapshot();
    snapshot["player"]["powers"] = json!([{
        "id": "Combust",
        "name": "Combust",
        "amount": 1
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "power.runtime_state.base_power missing for Malleable")]
fn malleable_requires_runtime_state_base_power() {
    let mut snapshot = base_snapshot();
    snapshot["player"]["powers"] = json!([{
        "id": "Malleable",
        "name": "Malleable",
        "amount": 3
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "power.runtime_state.stored_amount missing for Flight")]
fn flight_requires_runtime_state_stored_amount() {
    let mut snapshot = base_snapshot();
    snapshot["player"]["powers"] = json!([{
        "id": "Flight",
        "name": "Flight",
        "amount": 3
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "power.runtime_state.card_uuid missing for Stasis")]
fn stasis_requires_runtime_state_card_uuid() {
    let mut snapshot = base_snapshot();
    snapshot["player"]["powers"] = json!([{
        "id": "Stasis",
        "name": "Stasis",
        "amount": 1
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "power.runtime_state.damage missing for Panache")]
fn panache_requires_runtime_state_damage() {
    let mut snapshot = base_snapshot();
    snapshot["player"]["powers"] = json!([{
        "id": "Panache",
        "name": "Panache",
        "amount": 4
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "power.runtime_state.damage missing for The Bomb")]
fn the_bomb_requires_runtime_state_damage() {
    let mut snapshot = base_snapshot();
    snapshot["player"]["powers"] = json!([{
        "id": "The Bomb",
        "name": "The Bomb",
        "amount": 3
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "relic.runtime_state missing for CentennialPuzzle")]
fn relic_runtime_flags_require_runtime_state() {
    let snapshot = base_snapshot();
    let relics = json!([{
        "id": "Centennial Puzzle",
        "name": "Centennial Puzzle"
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &relics);
}

#[test]
#[should_panic(expected = "relic.runtime_state missing for BurningBlood")]
fn generic_relic_requires_runtime_state() {
    let snapshot = base_snapshot();
    let relics = json!([{
        "id": "Burning Blood",
        "name": "Burning Blood"
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &relics);
}

#[test]
#[should_panic(expected = "relic.runtime_state.counter missing for BurningBlood")]
fn generic_relic_requires_runtime_state_counter() {
    let snapshot = base_snapshot();
    let relics = json!([{
        "id": "Burning Blood",
        "name": "Burning Blood",
        "runtime_state": {
            "used_up": false
        }
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &relics);
}

#[test]
#[should_panic(expected = "relic.runtime_state.used_up missing for BurningBlood")]
fn generic_relic_requires_runtime_state_used_up() {
    let snapshot = base_snapshot();
    let relics = json!([{
        "id": "Burning Blood",
        "name": "Burning Blood",
        "runtime_state": {
            "counter": -1
        }
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &relics);
}

#[test]
#[should_panic(expected = "relic.runtime_state.gain_energy_next missing for ArtOfWar")]
fn art_of_war_requires_runtime_state_counter() {
    let snapshot = base_snapshot();
    let relics = json!([{
        "id": "Art of War",
        "name": "Art of War",
        "runtime_state": {
            "counter": -1,
            "used_up": false
        }
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &relics);
}

#[test]
#[should_panic(expected = "relic.runtime_state.amount missing for Pocketwatch")]
fn pocketwatch_requires_runtime_state_amount() {
    let snapshot = base_snapshot();
    let relics = json!([{
        "id": "Pocketwatch",
        "name": "Pocketwatch",
        "runtime_state": {
            "counter": -1,
            "used_up": false
        }
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &relics);
}

#[test]
#[should_panic(expected = "relic.runtime_state.used_up missing for Necronomicon")]
fn necronomicon_requires_runtime_state_used_up() {
    let snapshot = base_snapshot();
    let relics = json!([{
        "id": "Necronomicon",
        "name": "Necronomicon",
        "runtime_state": {
            "counter": -1
        }
    }]);

    let _ = build_combat_state_from_snapshots(&snapshot, &snapshot, &relics);
}
