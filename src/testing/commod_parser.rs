//! Parser for CommunicationMod JSON game state into CombatSnapshot.
//!
//! CommunicationMod sends game state as JSON via stdin/stdout.
//! This module converts that JSON into our `CombatSnapshot` for comparison.

use std::collections::BTreeMap;
use serde_json::Value;
use super::snapshot::*;

/// A single transition in a differential testing log.
#[derive(Debug, Clone)]
pub struct DiffTransition {
    pub step: u64,
    pub command: String,
    pub snapshot: Option<CombatSnapshot>,
    pub raw_state: Value,
}

/// Parse a full diff log JSONL file into a sequence of transitions.
pub fn parse_diff_log(content: &str) -> Vec<DiffTransition> {
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let entry: Value = serde_json::from_str(line).ok()?;
            let step = entry["step"].as_u64().unwrap_or(0);
            let command = entry["command"].as_str().unwrap_or("").to_string();
            let state = &entry["state"];
            let snapshot = parse_combat_snapshot(state);
            Some(DiffTransition {
                step,
                command,
                snapshot,
                raw_state: state.clone(),
            })
        })
        .collect()
}

/// Parse CommunicationMod JSON into a CombatSnapshot.
/// Returns None if not in combat (no combat_state).
pub fn parse_combat_snapshot(json: &Value) -> Option<CombatSnapshot> {
    let gs = json.get("game_state")?;
    let cs = gs.get("combat_state")?;
    if cs.is_null() {
        return None;
    }

    // Turn info
    let turn = cs["turn"].as_u64().unwrap_or(0) as u32;

    // Player state
    let player = &cs["player"];
    let player_hp = player["current_hp"].as_i64().unwrap_or(0) as i32;
    let player_max_hp = player["max_hp"].as_i64().unwrap_or(0) as i32;
    let player_block = player["block"].as_i64().unwrap_or(0) as i32;
    let player_energy = player["energy"].as_i64().unwrap_or(0) as i32;

    // Player powers
    let player_powers = parse_powers(&player["powers"]);

    // Player stance (CommunicationMod doesn't always expose this)
    // The stance field is in player.stance or not present
    let player_stance = player.get("stance")
        .and_then(|s| s.as_str())
        .unwrap_or("None")
        .to_string();

    // Hand cards
    let hand = parse_card_pile(&cs["hand"]);

    // Pile counts
    let draw_pile_count = cs["draw_pile"].as_array().map(|a| a.len()).unwrap_or(0);
    let discard_pile_count = cs["discard_pile"].as_array().map(|a| a.len()).unwrap_or(0);
    let exhaust_pile_count = cs["exhaust_pile"].as_array().map(|a| a.len()).unwrap_or(0);

    // Enemies
    let enemies = parse_monsters(&cs["monsters"]);

    // Relics
    let relics = parse_relics(&gs["relics"]);

    // Orbs
    let orbs = parse_orbs(&player["orbs"]);

    Some(CombatSnapshot {
        turn,
        cards_played_this_turn: 0, // CommunicationMod doesn't directly expose this
        player_hp,
        player_max_hp,
        player_block,
        player_energy,
        player_max_energy: 0, // Not in CommunicationMod combat_state
        player_powers,
        player_stance,
        hand,
        draw_pile_count,
        discard_pile_count,
        exhaust_pile_count,
        enemies,
        relics,
        orbs,
    })
}

/// Normalize a CommunicationMod card ID to match our internal ID.
/// CommunicationMod uses class-suffixed IDs like "Strike_R", "Defend_G", etc.
pub fn normalize_card_id(commod_id: &str) -> String {
    // Strip class suffixes: _R (Ironclad), _G (Silent), _B (Defect), _P (Watcher)
    let stripped = if commod_id.len() > 2 {
        let suffix = &commod_id[commod_id.len()-2..];
        if matches!(suffix, "_R" | "_G" | "_B" | "_P") {
            &commod_id[..commod_id.len()-2]
        } else {
            commod_id
        }
    } else {
        commod_id
    };
    stripped.to_string()
}

fn parse_card_pile(arr: &Value) -> Vec<CardSnap> {
    arr.as_array()
        .map(|cards| {
            cards.iter().map(|c| {
                CardSnap {
                    id: normalize_card_id(c["id"].as_str().unwrap_or("")),
                    cost: c["cost"].as_i64().unwrap_or(0) as i32,
                    upgraded: c["upgrades"].as_i64().unwrap_or(0) > 0,
                }
            }).collect()
        })
        .unwrap_or_default()
}

fn parse_powers(arr: &Value) -> BTreeMap<String, i32> {
    arr.as_array()
        .map(|powers| {
            powers.iter().filter_map(|p| {
                let id = p["id"].as_str()?;
                let amount = p["amount"].as_i64().unwrap_or(0) as i32;
                // Normalize Java power IDs to Rust engine format
                // e.g. "Weakened" → "Weak", "Time Warp" → "TimeWarp"
                let engine_id = super::id_map::commod_to_engine_power_id(id);
                Some((engine_id, amount))
            }).collect()
        })
        .unwrap_or_default()
}

fn parse_monsters(arr: &Value) -> Vec<EnemySnap> {
    arr.as_array()
        .map(|monsters| {
            monsters.iter().map(|m| {
                let powers = parse_powers(&m["powers"]);
                let intent = m["intent"].as_str().unwrap_or("UNKNOWN");
                EnemySnap {
                    name: m["name"].as_str().unwrap_or("").to_string(),
                    hp: m["current_hp"].as_i64().unwrap_or(0) as i32,
                    max_hp: m["max_hp"].as_i64().unwrap_or(0) as i32,
                    block: m["block"].as_i64().unwrap_or(0) as i32,
                    alive: !m["is_gone"].as_bool().unwrap_or(false)
                        && !m["half_dead"].as_bool().unwrap_or(false),
                    powers,
                    current_move: intent.to_string(),
                }
            }).collect()
        })
        .unwrap_or_default()
}

fn parse_relics(arr: &Value) -> Vec<RelicSnap> {
    arr.as_array()
        .map(|relics| {
            relics.iter().map(|r| {
                // Normalize relic ID: remove spaces to match Rust convention
                let id = r["id"].as_str().unwrap_or("");
                let normalized = id.replace(' ', "");
                RelicSnap {
                    id: normalized,
                    counter: r["counter"].as_i64().unwrap_or(-1) as i32,
                    active: true, // CommunicationMod doesn't expose .active
                }
            }).collect()
        })
        .unwrap_or_default()
}

fn parse_orbs(arr: &Value) -> Vec<OrbSnap> {
    arr.as_array()
        .map(|orbs| {
            orbs.iter().filter_map(|o| {
                let id = o["id"].as_str()?;
                if id == "Empty" {
                    return None;
                }
                Some(OrbSnap {
                    orb_type: id.to_string(),
                    passive_amount: o.get("passive_amount")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                    evoke_amount: o.get("evoke_amount")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0) as i32,
                })
            }).collect()
        })
        .unwrap_or_default()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_card_id() {
        assert_eq!(normalize_card_id("Strike_R"), "Strike");
        assert_eq!(normalize_card_id("Defend_G"), "Defend");
        assert_eq!(normalize_card_id("Zap_B"), "Zap");
        assert_eq!(normalize_card_id("Eruption_P"), "Eruption");
        assert_eq!(normalize_card_id("Bash"), "Bash");
        assert_eq!(normalize_card_id("Iron Wave"), "Iron Wave");
    }

    #[test]
    fn test_parse_combat_snapshot_basic() {
        let json_str = r#"{
            "game_state": {
                "combat_state": {
                    "turn": 1,
                    "hand": [
                        {"id": "Strike_R", "cost": 1, "upgrades": 0},
                        {"id": "Defend_R", "cost": 1, "upgrades": 1}
                    ],
                    "draw_pile": [{"id": "Bash", "cost": 2, "upgrades": 0}],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "player": {
                        "current_hp": 80,
                        "max_hp": 80,
                        "block": 0,
                        "energy": 3,
                        "powers": [{"id": "Strength", "amount": 2}],
                        "orbs": []
                    },
                    "monsters": [
                        {
                            "name": "Cultist",
                            "id": "Cultist",
                            "current_hp": 52,
                            "max_hp": 52,
                            "block": 0,
                            "intent": "BUFF",
                            "is_gone": false,
                            "half_dead": false,
                            "powers": []
                        }
                    ]
                },
                "relics": [
                    {"id": "Burning Blood", "name": "Burning Blood", "counter": -1}
                ]
            }
        }"#;
        let json: Value = serde_json::from_str(json_str).unwrap();
        let snap = parse_combat_snapshot(&json).unwrap();

        assert_eq!(snap.turn, 1);
        assert_eq!(snap.player_hp, 80);
        assert_eq!(snap.player_block, 0);
        assert_eq!(snap.player_energy, 3);
        assert_eq!(snap.player_powers.get("Strength"), Some(&2));
        assert_eq!(snap.hand.len(), 2);
        assert_eq!(snap.hand[0].id, "Strike"); // normalized
        assert_eq!(snap.hand[1].upgraded, true);
        assert_eq!(snap.draw_pile_count, 1);
        assert_eq!(snap.enemies.len(), 1);
        assert_eq!(snap.enemies[0].name, "Cultist");
        assert_eq!(snap.enemies[0].hp, 52);
        assert_eq!(snap.enemies[0].alive, true);
        assert_eq!(snap.relics[0].id, "BurningBlood"); // space removed
    }

    #[test]
    fn test_parse_no_combat_state() {
        let json_str = r#"{"game_state": {"screen_type": "MAP"}}"#;
        let json: Value = serde_json::from_str(json_str).unwrap();
        assert!(parse_combat_snapshot(&json).is_none());
    }

    #[test]
    fn test_parse_diff_log_line() {
        let line = r#"{"step":0,"command":"play 3 0","state":{"game_state":{"combat_state":{"turn":1,"hand":[],"draw_pile":[],"discard_pile":[],"exhaust_pile":[],"player":{"current_hp":80,"max_hp":80,"block":0,"energy":2,"powers":[],"orbs":[]},"monsters":[]},"relics":[]}}}"#;
        let transitions = parse_diff_log(line);
        assert_eq!(transitions.len(), 1);
        assert_eq!(transitions[0].step, 0);
        assert_eq!(transitions[0].command, "play 3 0");
        assert!(transitions[0].snapshot.is_some());
    }
}
