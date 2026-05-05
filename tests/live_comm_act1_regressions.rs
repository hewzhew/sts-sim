use serde_json::json;
use sts_simulator::bot::combat::diagnose_root_search_with_depth;
use sts_simulator::diff::state_sync::build_combat_state_from_snapshots;
use sts_simulator::state::EngineState;

fn strike(uuid: &str) -> serde_json::Value {
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

fn defend(uuid: &str) -> serde_json::Value {
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

fn bash(uuid: &str) -> serde_json::Value {
    json!({
        "id": "Bash",
        "name": "Bash",
        "uuid": uuid,
        "type": "ATTACK",
        "cost": 2,
        "upgrades": 0,
        "base_damage": 8,
        "rarity": "BASIC",
        "has_target": true,
        "ethereal": false,
        "exhausts": false,
        "is_playable": true
    })
}

fn heavy_blade_plus(uuid: &str) -> serde_json::Value {
    json!({
        "id": "Heavy Blade",
        "name": "Heavy Blade+",
        "uuid": uuid,
        "type": "ATTACK",
        "cost": 2,
        "upgrades": 1,
        "base_damage": 14,
        "rarity": "COMMON",
        "has_target": true,
        "ethereal": false,
        "exhausts": false,
        "is_playable": true
    })
}

fn evolve(uuid: &str) -> serde_json::Value {
    json!({
        "id": "Evolve",
        "name": "Evolve",
        "uuid": uuid,
        "type": "POWER",
        "cost": 1,
        "upgrades": 0,
        "rarity": "UNCOMMON",
        "has_target": false,
        "ethereal": false,
        "exhausts": false,
        "is_playable": true
    })
}

fn warcry(uuid: &str) -> serde_json::Value {
    json!({
        "id": "Warcry",
        "name": "Warcry",
        "uuid": uuid,
        "type": "SKILL",
        "cost": 0,
        "upgrades": 0,
        "rarity": "COMMON",
        "has_target": false,
        "ethereal": false,
        "exhausts": true,
        "is_playable": true
    })
}

fn sword_boomerang(uuid: &str) -> serde_json::Value {
    json!({
        "id": "Sword Boomerang",
        "name": "Sword Boomerang",
        "uuid": uuid,
        "type": "ATTACK",
        "cost": 1,
        "upgrades": 0,
        "base_damage": 3,
        "rarity": "COMMON",
        "has_target": false,
        "ethereal": false,
        "exhausts": false,
        "is_playable": true
    })
}

#[test]
fn act1_slimes_root_search_does_not_panic() {
    let truth = json!({
        "turn": 1,
        "room_type": "MonsterRoom",
        "player": {
            "current_hp": 80,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": []
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
                "id": "SpikeSlime_S",
                "name": "Spike Slime (S)",
                "current_hp": 12,
                "max_hp": 12,
                "block": 0,
                "move_id": 1,
                "move_base_damage": 5,
                "move_hits": 1,
                "powers": [],
                "is_gone": false,
                "half_dead": false
            },
            {
                "id": "AcidSlime_M",
                "name": "Acid Slime (M)",
                "current_hp": 31,
                "max_hp": 31,
                "block": 0,
                "move_id": 2,
                "move_base_damage": 10,
                "move_hits": 1,
                "powers": [],
                "is_gone": false,
                "half_dead": false
            }
        ],
        "hand": [
            defend("defend-1"),
            defend("defend-2"),
            bash("bash-1"),
            defend("defend-3"),
            strike("strike-1")
        ],
        "draw_pile": [
            strike("strike-2"),
            strike("strike-3"),
            strike("strike-4"),
            defend("defend-4")
        ],
        "discard_pile_ids": [],
        "exhaust_pile_ids": [],
        "limbo": [],
        "card_queue": [],
        "potions": [
            { "id": "Potion Slot", "name": "Potion Slot" },
            { "id": "Potion Slot", "name": "Potion Slot" },
            { "id": "Potion Slot", "name": "Potion Slot" }
        ]
    });

    let observation = json!({
        "turn": 1,
        "room_type": "MonsterRoom",
        "using_card": false,
        "player": {
            "current_hp": 80,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": []
        },
        "monsters": [
            {
                "id": "SpikeSlime_S",
                "name": "Spike Slime (S)",
                "current_hp": 12,
                "max_hp": 12,
                "block": 0,
                "intent": "ATTACK",
                "move_adjusted_damage": 5,
                "move_hits": 1,
                "monster_instance_id": 1,
                "spawn_order": 1,
                "monster_index": 0,
                "draw_x": 807,
                "powers": [],
                "is_gone": false,
                "half_dead": false,
                "is_dying": false,
                "is_escaping": false
            },
            {
                "id": "AcidSlime_M",
                "name": "Acid Slime (M)",
                "current_hp": 31,
                "max_hp": 31,
                "block": 0,
                "intent": "ATTACK",
                "move_adjusted_damage": 10,
                "move_hits": 1,
                "monster_instance_id": 2,
                "spawn_order": 2,
                "monster_index": 1,
                "draw_x": 983,
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
        "draw_pile_count": 4,
        "cards_discarded_this_turn": 0,
        "times_damaged": 0,
        "potions": truth["potions"].clone(),
        "relics": truth["relics"].clone(),
        "limbo": []
    });

    let combat = build_combat_state_from_snapshots(&truth, &observation, &truth["relics"]);
    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    assert!(diagnostics.legal_moves > 0);
}

#[test]
fn act2_spheric_guardian_root_search_does_not_panic() {
    let truth = json!({
        "turn": 1,
        "room_type": "MonsterRoom",
        "player": {
            "current_hp": 80,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": [
                { "id": "Strength", "name": "Strength", "amount": 1 }
            ]
        },
        "relics": [
            { "id": "SlaversCollar", "name": "Slaver's Collar", "runtime_state": { "used_up": false, "counter": -1 } },
            { "id": "Red Skull", "name": "Red Skull", "runtime_state": { "used_up": false, "counter": -1 } },
            { "id": "Omamori", "name": "Omamori", "runtime_state": { "used_up": false, "counter": 2 } },
            { "id": "Vajra", "name": "Vajra", "runtime_state": { "used_up": false, "counter": -1 } },
            { "id": "SacredBark", "name": "Sacred Bark", "runtime_state": { "used_up": false, "counter": -1 } }
        ],
        "monsters": [
            {
                "id": "SphericGuardian",
                "name": "Spheric Guardian",
                "current_hp": 20,
                "max_hp": 20,
                "block": 40,
                "move_id": 2,
                "move_base_damage": -1,
                "move_hits": 1,
                "powers": [
                    { "id": "Barricade", "name": "Barricade", "amount": -1 },
                    { "id": "Artifact", "name": "Artifact", "amount": 3 }
                ],
                "is_gone": false,
                "half_dead": false
            }
        ],
        "hand": [
            defend("defend-1"),
            evolve("evolve-1"),
            heavy_blade_plus("heavy-blade-1"),
            warcry("warcry-1"),
            defend("defend-2")
        ],
        "draw_pile": [
            json!({"id":"Demon Form","name":"Demon Form","uuid":"demon-form-1","type":"POWER","cost":3,"upgrades":0,"rarity":"RARE","has_target":false,"ethereal":false,"exhausts":false,"is_playable":true}),
            json!({"id":"Clothesline","name":"Clothesline","uuid":"clothesline-1","type":"ATTACK","cost":2,"upgrades":0,"base_damage":12,"rarity":"COMMON","has_target":true,"ethereal":false,"exhausts":false,"is_playable":true}),
            json!({"id":"Bash","name":"Bash+","uuid":"bash-plus-1","type":"ATTACK","cost":2,"upgrades":1,"base_damage":10,"rarity":"BASIC","has_target":true,"ethereal":false,"exhausts":false,"is_playable":true})
        ],
        "discard_pile_ids": [],
        "exhaust_pile_ids": [],
        "limbo": [],
        "card_queue": [],
        "potions": [
            { "id": "Potion Slot", "name": "Potion Slot" },
            { "id": "Potion Slot", "name": "Potion Slot" },
            { "id": "Potion Slot", "name": "Potion Slot" }
        ]
    });

    let observation = json!({
        "turn": 1,
        "room_type": "MonsterRoom",
        "using_card": false,
        "player": {
            "current_hp": 80,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": [
                { "id": "Strength", "name": "Strength", "amount": 1 }
            ]
        },
        "monsters": [
            {
                "id": "SphericGuardian",
                "name": "Spheric Guardian",
                "current_hp": 20,
                "max_hp": 20,
                "block": 40,
                "intent": "DEFEND",
                "move_adjusted_damage": -1,
                "move_hits": 1,
                "monster_instance_id": 21,
                "spawn_order": 21,
                "monster_index": 0,
                "draw_x": 960,
                "powers": [
                    { "id": "Barricade", "name": "Barricade", "amount": -1 },
                    { "id": "Artifact", "name": "Artifact", "amount": 3 }
                ],
                "is_gone": false,
                "half_dead": false,
                "is_dying": false,
                "is_escaping": false
            }
        ],
        "hand": truth["hand"].clone(),
        "discard_pile": [],
        "exhaust_pile": [],
        "draw_pile_count": 3,
        "cards_discarded_this_turn": 0,
        "times_damaged": 0,
        "potions": truth["potions"].clone(),
        "relics": truth["relics"].clone(),
        "limbo": []
    });

    let combat = build_combat_state_from_snapshots(&truth, &observation, &truth["relics"]);
    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    assert!(diagnostics.legal_moves > 0);
}

#[test]
fn act1_gremlin_gang_root_search_does_not_panic() {
    let truth = json!({
        "turn": 1,
        "room_type": "MonsterRoom",
        "player": {
            "current_hp": 80,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": []
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
                "id": "GremlinThief",
                "name": "Sneaky Gremlin",
                "current_hp": 12,
                "max_hp": 12,
                "block": 0,
                "move_id": 1,
                "move_base_damage": 9,
                "move_hits": 1,
                "powers": [],
                "is_gone": false,
                "half_dead": false
            },
            {
                "id": "GremlinTsundere",
                "name": "Shield Gremlin",
                "current_hp": 12,
                "max_hp": 12,
                "block": 0,
                "move_id": 1,
                "move_base_damage": -1,
                "move_hits": 1,
                "powers": [],
                "is_gone": false,
                "half_dead": false
            },
            {
                "id": "GremlinFat",
                "name": "Fat Gremlin",
                "current_hp": 17,
                "max_hp": 17,
                "block": 0,
                "move_id": 2,
                "move_base_damage": 4,
                "move_hits": 1,
                "powers": [],
                "is_gone": false,
                "half_dead": false
            },
            {
                "id": "GremlinWarrior",
                "name": "Mad Gremlin",
                "current_hp": 21,
                "max_hp": 21,
                "block": 0,
                "move_id": 1,
                "move_base_damage": 4,
                "move_hits": 1,
                "runtime_state": {
                    "angry_amount": 1
                },
                "powers": [
                    { "id": "Angry", "name": "Angry", "amount": 1 }
                ],
                "is_gone": false,
                "half_dead": false
            }
        ],
        "hand": [
            defend("defend-1"),
            sword_boomerang("sword-boomerang-1"),
            sword_boomerang("sword-boomerang-2"),
            strike("strike-1"),
            defend("defend-2")
        ],
        "draw_pile": [
            defend("defend-3"),
            bash("bash-1"),
            json!({"id":"Clothesline","name":"Clothesline","uuid":"clothesline-1","type":"ATTACK","cost":2,"upgrades":0,"base_damage":12,"rarity":"COMMON","has_target":true,"ethereal":false,"exhausts":false,"is_playable":true}),
            strike("strike-2"),
            strike("strike-3"),
            json!({"id":"Shockwave","name":"Shockwave","uuid":"shockwave-1","type":"SKILL","cost":2,"upgrades":0,"rarity":"UNCOMMON","has_target":false,"ethereal":false,"exhausts":true,"is_playable":true}),
            strike("strike-4"),
            defend("defend-4"),
            strike("strike-5")
        ],
        "discard_pile_ids": [],
        "exhaust_pile_ids": [],
        "limbo": [],
        "card_queue": [],
        "potions": [
            { "id": "Potion Slot", "name": "Potion Slot" },
            { "id": "Potion Slot", "name": "Potion Slot" },
            { "id": "Potion Slot", "name": "Potion Slot" }
        ]
    });

    let observation = json!({
        "turn": 1,
        "room_type": "MonsterRoom",
        "using_card": false,
        "player": {
            "current_hp": 80,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": []
        },
        "monsters": [
            {
                "id": "GremlinThief",
                "name": "Sneaky Gremlin",
                "current_hp": 12,
                "max_hp": 12,
                "block": 0,
                "intent": "ATTACK",
                "move_adjusted_damage": 9,
                "move_hits": 1,
                "monster_instance_id": 6,
                "spawn_order": 6,
                "monster_index": 0,
                "draw_x": 617,
                "powers": [],
                "is_gone": false,
                "half_dead": false,
                "is_dying": false,
                "is_escaping": false
            },
            {
                "id": "GremlinTsundere",
                "name": "Shield Gremlin",
                "current_hp": 12,
                "max_hp": 12,
                "block": 0,
                "intent": "DEFEND",
                "move_adjusted_damage": -1,
                "move_hits": 1,
                "monster_instance_id": 7,
                "spawn_order": 7,
                "monster_index": 1,
                "draw_x": 799,
                "powers": [],
                "is_gone": false,
                "half_dead": false,
                "is_dying": false,
                "is_escaping": false
            },
            {
                "id": "GremlinFat",
                "name": "Fat Gremlin",
                "current_hp": 17,
                "max_hp": 17,
                "block": 0,
                "intent": "ATTACK_DEBUFF",
                "move_adjusted_damage": 4,
                "move_hits": 1,
                "monster_instance_id": 8,
                "spawn_order": 8,
                "monster_index": 2,
                "draw_x": 977,
                "powers": [],
                "is_gone": false,
                "half_dead": false,
                "is_dying": false,
                "is_escaping": false
            },
            {
                "id": "GremlinWarrior",
                "name": "Mad Gremlin",
                "current_hp": 21,
                "max_hp": 21,
                "block": 0,
                "intent": "ATTACK",
                "move_adjusted_damage": 4,
                "move_hits": 1,
                "monster_instance_id": 9,
                "spawn_order": 9,
                "monster_index": 3,
                "draw_x": 1097,
                "powers": [
                    { "id": "Angry", "name": "Angry", "amount": 1 }
                ],
                "is_gone": false,
                "half_dead": false,
                "is_dying": false,
                "is_escaping": false
            }
        ],
        "hand": truth["hand"].clone(),
        "discard_pile": [],
        "exhaust_pile": [],
        "draw_pile_count": 9,
        "cards_discarded_this_turn": 0,
        "times_damaged": 0,
        "potions": truth["potions"].clone(),
        "relics": truth["relics"].clone(),
        "limbo": []
    });

    let combat = build_combat_state_from_snapshots(&truth, &observation, &truth["relics"]);
    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    assert!(diagnostics.legal_moves > 0);
}
