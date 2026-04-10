use super::super::*;
use super::support::*;

#[test]
fn bloodletting_from_live_style_snapshot_keeps_next_turn_block() {
    let snapshot = serde_json::json!({
        "turn": 7,
        "room_type": "MonsterRoomBoss",
        "player": {
            "current_hp": 25,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": [
                {"id": "Dexterity", "amount": 1}
            ]
        },
        "monsters": [
            {
                "id": "Hexaghost",
                "current_hp": 134,
                "max_hp": 250,
                "block": 12,
                "intent": "ATTACK",
                "move_base_damage": 5,
                "move_adjusted_damage": 7,
                "move_hits": 2,
                "move_id": 2,
                "powers": [
                    {"id": "Strength", "amount": 2}
                ]
            }
        ],
        "hand": [
            {"id": "Defend_R", "uuid": "h1", "upgrades": 0, "cost": 1},
            {"id": "Heavy Blade", "uuid": "h2", "upgrades": 0, "cost": 2},
            {"id": "Bloodletting", "uuid": "h3", "upgrades": 0, "cost": 0},
            {"id": "Defend_R", "uuid": "h4", "upgrades": 0, "cost": 1},
            {"id": "Defend_R", "uuid": "h5", "upgrades": 0, "cost": 1}
        ],
        "draw_pile": [],
        "discard_pile": [],
        "exhaust_pile": [],
        "potions": [],
        "relics": [
            {"id": "Burning Blood", "counter": -1},
            {"id": "Kunai", "counter": 0},
            {"id": "Self Forming Clay", "counter": -1}
        ]
    });

    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);

    let alive = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 2,
            target: None,
        },
    );

    assert!(alive);
    assert_eq!(combat.entities.player.current_hp, 22);
    assert_eq!(combat.turn.energy, 5);
    let player_powers = combat
        .entities
        .power_db
        .get(&0)
        .cloned()
        .unwrap_or_default();
    assert!(player_powers
        .iter()
        .any(|p| { p.power_type == PowerId::NextTurnBlock && p.amount == 3 }));
}

#[test]
fn cloned_live_style_state_keeps_self_forming_clay_bus_for_bloodletting() {
    let snapshot = serde_json::json!({
        "turn": 7,
        "room_type": "MonsterRoomBoss",
        "player": {
            "current_hp": 25,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": [
                {"id": "Dexterity", "amount": 1}
            ]
        },
        "monsters": [
            {
                "id": "Hexaghost",
                "current_hp": 134,
                "max_hp": 250,
                "block": 12,
                "intent": "ATTACK",
                "move_base_damage": 5,
                "move_adjusted_damage": 7,
                "move_hits": 2,
                "move_id": 2,
                "powers": [
                    {"id": "Strength", "amount": 2}
                ]
            }
        ],
        "hand": [
            {"id": "Defend_R", "uuid": "h1", "upgrades": 0, "cost": 1},
            {"id": "Heavy Blade", "uuid": "h2", "upgrades": 0, "cost": 2},
            {"id": "Bloodletting", "uuid": "h3", "upgrades": 0, "cost": 0},
            {"id": "Defend_R", "uuid": "h4", "upgrades": 0, "cost": 1},
            {"id": "Defend_R", "uuid": "h5", "upgrades": 0, "cost": 1}
        ],
        "draw_pile": [],
        "discard_pile": [],
        "exhaust_pile": [],
        "potions": [],
        "relics": [
            {"id": "Burning Blood", "counter": -1},
            {"id": "Kunai", "counter": 0},
            {"id": "Self Forming Clay", "counter": -1}
        ]
    });

    let truth = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);
    let mut combat = truth.clone();
    let mut engine_state = EngineState::CombatPlayerTurn;

    let alive = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 2,
            target: None,
        },
    );

    assert!(alive);
    let player_powers = combat
        .entities
        .power_db
        .get(&0)
        .cloned()
        .unwrap_or_default();
    assert!(player_powers
        .iter()
        .any(|p| { p.power_type == PowerId::NextTurnBlock && p.amount == 3 }));
}

#[test]
fn hexaghost_snapshot_with_explicit_orb_state_rolls_inflame_after_sear_two() {
    let snapshot = json!({
        "turn": 8,
        "room_type": "MonsterRoomBoss",
        "player": {
            "current_hp": 36,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": []
        },
        "monsters": [{
            "id": "Hexaghost",
            "current_hp": 160,
            "max_hp": 250,
            "block": 0,
            "intent": "ATTACK_DEBUFF",
            "move_base_damage": 6,
            "move_adjusted_damage": 6,
            "move_hits": 1,
            "move_id": 4,
            "hexaghost_activated": true,
            "hexaghost_orb_active_count": 2,
            "hexaghost_burn_upgraded": false,
            "powers": []
        }],
        "hand": [
            {"id": "Defend_R", "uuid": "h1", "upgrades": 0, "cost": 1}
        ],
        "draw_pile": [],
        "discard_pile": [],
        "exhaust_pile": [],
        "potions": [],
        "relics": []
    });

    let mut engine_state = EngineState::CombatPlayerTurn;
    let mut combat = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);

    let alive = tick_until_stable_turn(&mut engine_state, &mut combat, ClientInput::EndTurn);

    assert!(alive);
    let monster = &combat.entities.monsters[0];
    assert_eq!(monster.next_move_byte, 3);
    assert_eq!(monster.current_intent, Intent::DefendBuff);
    assert_eq!(monster.hexaghost.orb_active_count, 3);
}

#[test]
fn shrug_it_off_live_snapshot_shuffle_window_draws_non_status_top_card() {
    let snapshot = json!({
        "turn": 10,
        "room_type": "MonsterRoomElite",
        "player": {
            "current_hp": 23,
            "max_hp": 80,
            "block": 0,
            "energy": 2,
            "powers": [
                {"id": "Feel No Pain", "amount": 4},
                {"id": "Evolve", "amount": 1}
            ]
        },
        "monsters": [{
            "id": "Book of Stabbing",
            "current_hp": 114,
            "max_hp": 161,
            "block": 0,
            "intent": "ATTACK",
            "move_base_damage": 6,
            "move_adjusted_damage": 6,
            "move_hits": 4,
            "move_id": 2,
            "powers": [
                {"id": "Painful Stabs", "amount": -1},
                {"id": "Metallicize", "amount": 6}
            ]
        }],
        "relics": [],
        "hand": [
            {"id": "Defend_R", "uuid": "b1a89923-c4e2-457a-8e1f-5187c1d4afdc", "upgrades": 0, "cost": 1},
            {"id": "Strike_R", "uuid": "47b8a4ec-ab8e-4c76-8c2c-8faf113a3e5c", "upgrades": 1, "cost": 1},
            {"id": "Second Wind", "uuid": "f1ba99d3-60d1-4961-abbb-dad19eff5c77", "upgrades": 1, "cost": 1},
            {"id": "Whirlwind", "uuid": "e449c7ac-1ce5-4362-9d98-d47e1dfda718", "upgrades": 1, "cost": -1},
            {"id": "Shrug It Off", "uuid": "3ba9fc2f-6e2b-4b94-b9b8-0d0509469272", "upgrades": 1, "cost": 1}
        ],
        "draw_pile": [],
        "discard_pile": [
            {"id": "Defend_R", "uuid": "56ef86fa-e325-4c6e-9d50-8d8f53c0fe7d", "upgrades": 0, "cost": 1},
            {"id": "Defend_R", "uuid": "162d1ecd-2bf0-4950-b6fb-fa881b5e81d8", "upgrades": 0, "cost": 1},
            {"id": "Battle Trance", "uuid": "fec00ce4-0d00-40bf-970d-ab538baf8056", "upgrades": 1, "cost": 0},
            {"id": "Armaments", "uuid": "75a80f3b-6561-4f14-afb6-a07622b76311", "upgrades": 1, "cost": 1},
            {"id": "Wound", "uuid": "7a192976-3c8a-497c-8058-019e0eecac51", "upgrades": 0, "cost": -2},
            {"id": "Bash", "uuid": "deb86a95-cd47-4327-a050-4e4e58434d24", "upgrades": 1, "cost": 2},
            {"id": "Strike_R", "uuid": "8ba2cd2f-ddf6-4a46-8573-4efe250b7df3", "upgrades": 1, "cost": 1},
            {"id": "Strike_R", "uuid": "466ec79e-ce89-4d0f-b6dd-9e1107143701", "upgrades": 1, "cost": 1},
            {"id": "Wound", "uuid": "3d8e3482-2588-4dd8-babd-cc8ce9e6d4ed", "upgrades": 0, "cost": -2},
            {"id": "Pommel Strike", "uuid": "995a5725-2acb-44e5-a1fe-3cc0da205dd1", "upgrades": 0, "cost": 1},
            {"id": "Shrug It Off", "uuid": "912b77aa-3914-443a-8482-4515c13df655", "upgrades": 0, "cost": 1}
        ],
        "exhaust_pile": [],
        "rng_state": {
            "shuffle_rng": {
                "seed0": -6247337117156978221i64,
                "seed1": -8989753778678835992i64,
                "counter": 2
            }
        }
    });

    let mut combat = build_combat_state(&snapshot, &snapshot["relics"]);
    let ok = tick_until_stable_turn(
        &mut EngineState::CombatPlayerTurn,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 4,
            target: None,
        },
    );

    assert!(ok);
    assert_eq!(combat.zones.hand.len(), 5);
    assert!(combat
        .zones
        .hand
        .iter()
        .any(|card| card.id == CardId::PommelStrike));
    assert!(!combat
        .zones
        .hand
        .iter()
        .any(|card| card.id == CardId::Wound));
}

#[test]
fn shrug_it_off_live_snapshot_nonshuffle_draw_takes_java_top_from_tail() {
    let snapshot = json!({
        "turn": 11,
        "room_type": "MonsterRoomElite",
        "player": {
            "current_hp": 18,
            "max_hp": 80,
            "block": 0,
            "energy": 1,
            "powers": [
                {"id": "Feel No Pain", "amount": 4},
                {"id": "Evolve", "amount": 1}
            ]
        },
        "monsters": [{
            "id": "Book of Stabbing",
            "current_hp": 93,
            "max_hp": 161,
            "block": 0,
            "intent": "ATTACK",
            "move_base_damage": 21,
            "move_adjusted_damage": 21,
            "move_hits": 1,
            "move_id": 4,
            "powers": [
                {"id": "Painful Stabs", "amount": -1},
                {"id": "Metallicize", "amount": 6}
            ]
        }],
        "relics": [],
        "hand": [
            {"id": "Battle Trance", "uuid": "bt1", "upgrades": 1, "cost": 0},
            {"id": "Shrug It Off", "uuid": "shr1", "upgrades": 1, "cost": 1}
        ],
        "draw_pile": [
            {"id": "Wound", "uuid": "dw1", "upgrades": 0, "cost": -2},
            {"id": "Bash", "uuid": "db1", "upgrades": 1, "cost": 2},
            {"id": "Wound", "uuid": "dw2", "upgrades": 0, "cost": -2},
            {"id": "Defend_R", "uuid": "dd1", "upgrades": 0, "cost": 1}
        ],
        "discard_pile": [
            {"id": "Shrug It Off", "uuid": "d1", "upgrades": 1, "cost": 1},
            {"id": "Pommel Strike", "uuid": "d2", "upgrades": 0, "cost": 1},
            {"id": "Whirlwind", "uuid": "d3", "upgrades": 1, "cost": -1},
            {"id": "Defend_R", "uuid": "d4", "upgrades": 0, "cost": 1},
            {"id": "Second Wind", "uuid": "d5", "upgrades": 1, "cost": 1},
            {"id": "Strike_R", "uuid": "d6", "upgrades": 1, "cost": 1},
            {"id": "Defend_R", "uuid": "d7", "upgrades": 0, "cost": 1},
            {"id": "Wound", "uuid": "d8", "upgrades": 0, "cost": -2},
            {"id": "Armaments", "uuid": "d9", "upgrades": 1, "cost": 1},
            {"id": "Strike_R", "uuid": "d10", "upgrades": 1, "cost": 1},
            {"id": "Strike_R", "uuid": "d11", "upgrades": 1, "cost": 1}
        ],
        "exhaust_pile": [],
        "rng_state": {
            "shuffle_rng": {
                "seed0": -8989753778678835992i64,
                "seed1": -3134239283555752834i64,
                "counter": 3
            }
        }
    });

    let mut combat = build_combat_state(&snapshot, &snapshot["relics"]);
    let ok = tick_until_stable_turn(
        &mut EngineState::CombatPlayerTurn,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
    );

    assert!(ok);
    assert_eq!(combat.zones.hand.len(), 2);
    assert!(combat
        .zones
        .hand
        .iter()
        .any(|card| card.id == CardId::Defend));
    assert!(!combat
        .zones
        .hand
        .iter()
        .any(|card| card.id == CardId::Wound));
}

#[test]
fn distilled_chaos_live_snapshot_buffers_top_cards_before_play() {
    let snapshot = serde_json::json!({
        "turn": 5,
        "room_type": "MonsterRoomElite",
        "player": {
            "current_hp": 31,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": []
        },
        "monsters": [{
            "id": "Gremlin Nob",
            "current_hp": 84,
            "max_hp": 84,
            "block": 0,
            "intent": "BUFF",
            "move_base_damage": -1,
            "move_adjusted_damage": -1,
            "move_hits": 0,
            "move_id": 3,
            "powers": [
                {"id": "Strength", "amount": 2}
            ]
        }],
        "relics": [],
        "hand": [
            {"id": "Shrug It Off", "uuid": "h1", "upgrades": 1, "cost": 1},
            {"id": "Defend_R", "uuid": "h2", "upgrades": 0, "cost": 1},
            {"id": "Bash", "uuid": "h3", "upgrades": 0, "cost": 2},
            {"id": "Defend_R", "uuid": "h4", "upgrades": 0, "cost": 1},
            {"id": "Defend_R", "uuid": "h5", "upgrades": 0, "cost": 1}
        ],
        "draw_pile": [
            {"id": "Strike_R", "uuid": "d1", "upgrades": 0, "cost": 1},
            {"id": "Strike_R", "uuid": "d2", "upgrades": 0, "cost": 1},
            {"id": "Strike_R", "uuid": "d3", "upgrades": 0, "cost": 1},
            {"id": "Defend_R", "uuid": "d4", "upgrades": 0, "cost": 1},
            {"id": "Pommel Strike", "uuid": "d5", "upgrades": 0, "cost": 1},
            {"id": "Blood for Blood", "uuid": "d6", "upgrades": 0, "cost": 4},
            {"id": "Strike_R", "uuid": "d7", "upgrades": 0, "cost": 1},
            {"id": "Shrug It Off", "uuid": "d8", "upgrades": 0, "cost": 1},
            {"id": "Whirlwind", "uuid": "d9", "upgrades": 1, "cost": -1},
            {"id": "Battle Trance", "uuid": "d10", "upgrades": 1, "cost": 0}
        ],
        "discard_pile": [],
        "exhaust_pile": [],
        "potions": [
            {"id": "Distilled Chaos", "name": "Distilled Chaos", "can_use": true, "can_discard": true, "requires_target": false},
            {"id": "Potion Slot"},
            {"id": "Potion Slot"}
        ]
    });

    let mut combat = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);
    crate::engine::action_handlers::execute_action(
        Action::UsePotion {
            slot: 0,
            target: None,
        },
        &mut combat,
    );
    while let Some(action) = combat.engine.action_queue.pop_front() {
        crate::engine::action_handlers::execute_action(action, &mut combat);
    }

    assert_eq!(combat.entities.monsters[0].current_hp, 60);
    let hand_ids: Vec<_> = combat.zones.hand.iter().map(|card| card.id).collect();
    assert!(hand_ids.contains(&CardId::PommelStrike));
    assert!(hand_ids.contains(&CardId::BloodForBlood));
    assert!(hand_ids.contains(&CardId::Strike));
    assert!(hand_ids.contains(&CardId::Defend));
    let discard_ids: Vec<_> = combat
        .zones
        .discard_pile
        .iter()
        .map(|card| card.id)
        .collect();
    assert!(discard_ids.contains(&CardId::ShrugItOff));
    assert!(discard_ids.contains(&CardId::Whirlwind));
    assert!(discard_ids.contains(&CardId::BattleTrance));
    assert!(crate::content::powers::store::has_power(
        &combat,
        0,
        PowerId::NoDraw
    ));
}

#[test]
fn strike_dummy_adds_three_damage_to_strike_plus_after_building_from_live_snapshot() {
    let snapshot = live_snapshot_with_strike_dummy(
        serde_json::json!([
            {"id": "Strike_R", "uuid": "card-1", "upgrades": 1, "cost": 1}
        ]),
        serde_json::json!([]),
        serde_json::json!([{
            "id": "JawWorm",
            "current_hp": 40,
            "max_hp": 40,
            "block": 0,
            "intent": "ATTACK",
            "move_base_damage": 11,
            "move_adjusted_damage": 11,
            "move_hits": 1,
            "move_id": 1,
            "powers": []
        }]),
    );

    let mut combat = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);
    let mut engine_state = EngineState::CombatPlayerTurn;
    let ok = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    );

    assert!(ok);
    assert_eq!(combat.entities.monsters[0].current_hp, 25);
}

#[test]
fn strike_dummy_adds_three_damage_to_pommel_strike_after_building_from_live_snapshot() {
    let snapshot = live_snapshot_with_strike_dummy(
        serde_json::json!([
            {"id": "Pommel Strike", "uuid": "card-1", "upgrades": 0, "cost": 1}
        ]),
        serde_json::json!([
            {"id": "Strike_R", "uuid": "draw-1", "upgrades": 0, "cost": 1}
        ]),
        serde_json::json!([{
            "id": "JawWorm",
            "current_hp": 40,
            "max_hp": 40,
            "block": 0,
            "intent": "ATTACK",
            "move_base_damage": 11,
            "move_adjusted_damage": 11,
            "move_hits": 1,
            "move_id": 1,
            "powers": []
        }]),
    );

    let mut combat = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);
    let mut engine_state = EngineState::CombatPlayerTurn;
    let ok = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    );

    assert!(ok);
    assert_eq!(combat.entities.monsters[0].current_hp, 25);
    assert_eq!(combat.zones.hand.len(), 1);
    assert_eq!(combat.zones.hand[0].id, CardId::Strike);
}

#[test]
fn strike_dummy_damage_is_modified_before_weak_rounding() {
    let snapshot = serde_json::json!({
        "turn": 3,
        "room_type": "MonsterRoom",
        "player": {
            "current_hp": 50,
            "max_hp": 80,
            "block": 0,
            "energy": 3,
            "powers": [
                {"id": "Weakened", "amount": 1}
            ]
        },
        "monsters": [{
            "id": "JawWorm",
            "current_hp": 40,
            "max_hp": 40,
            "block": 0,
            "intent": "ATTACK",
            "move_base_damage": 11,
            "move_adjusted_damage": 11,
            "move_hits": 1,
            "move_id": 1,
            "powers": []
        }],
        "relics": [
            {"id": "StrikeDummy", "counter": -1}
        ],
        "hand": [
            {"id": "Strike_R", "uuid": "card-1", "upgrades": 0, "cost": 1}
        ],
        "draw_pile": [],
        "discard_pile": [],
        "exhaust_pile": []
    });

    let mut combat = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);
    let mut engine_state = EngineState::CombatPlayerTurn;
    let ok = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    );

    assert!(ok);
    // Java ordering: Strike Dummy modifies base damage first, then Weak scales it.
    // (6 + 3) * 0.75 = 6.75 -> floor 6 damage
    assert_eq!(combat.entities.monsters[0].current_hp, 34);
}

#[test]
fn strike_dummy_applies_when_targeting_second_monster_from_live_snapshot() {
    let snapshot = live_snapshot_with_strike_dummy(
        serde_json::json!([
            {"id": "Strike_R", "uuid": "card-1", "upgrades": 0, "cost": 1}
        ]),
        serde_json::json!([]),
        serde_json::json!([
            {
                "id": "JawWorm",
                "current_hp": 40,
                "max_hp": 40,
                "block": 0,
                "intent": "ATTACK",
                "move_base_damage": 11,
                "move_adjusted_damage": 11,
                "move_hits": 1,
                "move_id": 1,
                "powers": []
            },
            {
                "id": "Cultist",
                "current_hp": 40,
                "max_hp": 40,
                "block": 0,
                "intent": "ATTACK",
                "move_base_damage": 6,
                "move_adjusted_damage": 6,
                "move_hits": 1,
                "move_id": 1,
                "powers": []
            }
        ]),
    );

    let mut combat = crate::diff::state_sync::build_combat_state(&snapshot, &snapshot["relics"]);
    let mut engine_state = EngineState::CombatPlayerTurn;
    let ok = tick_until_stable_turn(
        &mut engine_state,
        &mut combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
    );

    assert!(ok);
    assert_eq!(combat.entities.monsters[0].current_hp, 40);
    assert_eq!(combat.entities.monsters[1].current_hp, 28);
}
