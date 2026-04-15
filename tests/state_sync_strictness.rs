use serde_json::json;
use sts_simulator::diff::state_sync::build_combat_state;

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

    let _ = build_combat_state(&snapshot, &json!([]));
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

    let _ = build_combat_state(&snapshot, &json!([]));
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

    let _ = build_combat_state(&snapshot, &json!([]));
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

    let _ = build_combat_state(&snapshot, &json!([]));
}

#[test]
#[should_panic(expected = "relic.runtime_state missing for CentennialPuzzle")]
fn relic_runtime_flags_require_runtime_state() {
    let snapshot = base_snapshot();
    let relics = json!([{
        "id": "Centennial Puzzle",
        "name": "Centennial Puzzle",
        "counter": -1,
        "used_up": false
    }]);

    let _ = build_combat_state(&snapshot, &relics);
}
