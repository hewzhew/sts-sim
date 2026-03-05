//! State Hydrator — Inject CommunicationMod JSON into executable Rust GameState.
//!
//! This module converts a CommunicationMod game state JSON snapshot into a fully
//! initialized `GameState` that the Rust engine can execute actions on.
//!
//! ## Design
//!
//! The hydrator bypasses RNG entirely. Instead of replaying from seed, it reads the
//! exact game state (hand, draw pile, discard pile, HP, powers, etc.) from JSON and
//! constructs the corresponding Rust structs. This allows validating engine logic
//! (damage calc, block, powers) without matching Java's RNG.

use serde_json::Value;
use smallvec::SmallVec;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256StarStar;

use crate::core::state::{GameState, GamePhase, InsertPosition, Player};
use crate::core::schema::{CardInstance, CardType};
use crate::core::stances::Stance;
use crate::monsters::enemy::{MonsterState, Intent};
use crate::items::relics::RelicInstance;
use crate::powers::PowerSet;

use super::commod_parser::normalize_card_id;

/// Convert a CommunicationMod power ID to the engine's internal power ID.
/// Delegates to the centralized `id_map` module.
fn commod_to_engine_power_id(commod_id: &str) -> String {
    super::id_map::commod_to_engine_power_id(commod_id)
}


// ============================================================================
// Card Instance Hydration
// ============================================================================

/// Translate a CommunicationMod card ID to the format used in CardLibrary.
///
/// CommunicationMod: `Strike_R`, `Defend_G`, `Bash`, `Flame Barrier`, `FlurryOfBlows`
/// CardLibrary:       `Strike_Ironclad`, `Defend_Silent`, `Bash`, `Flame_Barrier`, `Flurry_of_Blows`
pub fn commod_to_library_id(commod_id: &str) -> String {
    // Step 0: Handle CommunicationMod internal names that differ from card library IDs.
    // Java uses internal names (e.g., Apparition → "Ghostly") which CommunicationMod exposes.
    let commod_id = match commod_id {
        "Ghostly" => "Apparition",
        "GhostlyArmor" => "Ghostly_Armor",  // Ghostly Armor is already correct but be safe
        _ => commod_id,
    };

    // First: handle class suffixes (_R/_G/_B/_P → _Ironclad/_Silent/...)
    let id = if commod_id.len() > 2 {
        let suffix = &commod_id[commod_id.len()-2..];
        let base = &commod_id[..commod_id.len()-2];
        match suffix {
            "_R" => format!("{}_Ironclad", base),
            "_G" => format!("{}_Silent", base),
            "_B" => format!("{}_Defect", base),
            "_P" => format!("{}_Watcher", base),
            _ => commod_id.to_string(),
        }
    } else {
        commod_id.to_string()
    };
    // Second: spaces → underscores (CommunicationMod uses spaces, library uses underscores)
    let id = id.replace(' ', "_");
    
    // Third: CamelCase → underscore_separated for Watcher cards
    // CommunicationMod sends "FlurryOfBlows" but library uses "Flurry_of_Blows"
    // Only apply if the ID contains no underscores (already-separated IDs like "Strike_Ironclad" skip this)
    if !id.contains('_') {
        let chars: Vec<char> = id.chars().collect();
        let mut result = String::with_capacity(id.len() + 8);
        for (i, &ch) in chars.iter().enumerate() {
            if i > 0 && ch.is_uppercase() && chars[i-1].is_lowercase() {
                result.push('_');
            }
            result.push(ch);
        }
        // Convert small words to lowercase: "Of" → "of", "The" → "the", etc.
        let parts: Vec<&str> = result.split('_').collect();
        if parts.len() > 1 {
            let fixed: Vec<String> = parts.iter().enumerate().map(|(i, &p)| {
                if i > 0 {
                    match p.to_lowercase().as_str() {
                        "of" | "the" | "to" | "no" => p.to_lowercase(),
                        _ => p.to_string(),
                    }
                } else {
                    p.to_string()
                }
            }).collect();
            return fixed.join("_");
        }
        return result;
    }
    
    id
}

/// Create a CardInstance from a CommunicationMod card JSON object.
///
/// CommunicationMod format:
/// ```json
/// { "id": "Strike_R", "name": "Strike", "type": "ATTACK", "cost": 1,
///   "upgrades": 0, "has_target": true, "exhausts": false, "ethereal": false }
/// ```
pub fn hydrate_card(json: &Value) -> CardInstance {
    let raw_id = json["id"].as_str().unwrap_or("");
    let definition_id = commod_to_library_id(raw_id);
    let upgraded = json["upgrades"].as_i64().unwrap_or(0) > 0;
    let cost = json["cost"].as_i64().unwrap_or(0) as i32;
    let card_type = match json["type"].as_str().unwrap_or("ATTACK") {
        "ATTACK" => CardType::Attack,
        "SKILL" => CardType::Skill,
        "POWER" => CardType::Power,
        "STATUS" => CardType::Status,
        "CURSE" => CardType::Curse,
        _ => CardType::Attack,
    };
    let is_ethereal = json["ethereal"].as_bool().unwrap_or(false);

    CardInstance {
        definition_id,
        upgraded,
        base_cost: cost,
        current_cost: cost,
        is_ethereal,
        is_innate: false, // CommunicationMod doesn't expose this
        self_retain: false,
        card_type,
    }
}

/// Hydrate a card pile (hand, draw, discard, exhaust) from CommunicationMod JSON array.
pub fn hydrate_card_pile<A: smallvec::Array<Item = CardInstance>>(arr: &Value) -> SmallVec<A> {
    arr.as_array()
        .map(|cards| cards.iter().map(hydrate_card).collect())
        .unwrap_or_default()
}

/// Hydrate a Vec<CardInstance> from a JSON array (for master_deck).
pub fn hydrate_card_vec(arr: &Value) -> Vec<CardInstance> {
    arr.as_array()
        .map(|cards| cards.iter().map(hydrate_card).collect())
        .unwrap_or_default()
}

// ============================================================================
// Player Hydration
// ============================================================================

/// Create a Player from CommunicationMod's combat_state.player JSON.
pub fn hydrate_player(json: &Value) -> Player {
    let mut powers = PowerSet::new();
    if let Some(power_arr) = json["powers"].as_array() {
        for p in power_arr {
            if let Some(raw_id) = p["id"].as_str() {
                let id = commod_to_engine_power_id(raw_id);
                let amount = p["amount"].as_i64().unwrap_or(0) as i32;
                powers.set(&id, amount);
            }
        }
    }

    // Parse stance: CommunicationMod exposes player.stance as a string
    // "Neutral", "Wrath", "Calm", "Divinity", or missing (defaults to Neutral)
    let stance = json.get("stance")
        .and_then(|s| s.as_str())
        .map(Stance::from_str)
        .unwrap_or(Stance::Neutral);

    Player {
        max_hp: json["max_hp"].as_i64().unwrap_or(80) as i32,
        current_hp: json["current_hp"].as_i64().unwrap_or(80) as i32,
        block: json["block"].as_i64().unwrap_or(0) as i32,
        energy: json["energy"].as_i64().unwrap_or(3) as i32,
        max_energy: 3, // CommunicationMod doesn't expose this
        powers,
        stance,
        gold: 0,
    }
}

// ============================================================================
// Enemy (MonsterState) Hydration
// ============================================================================

/// Parse CommunicationMod intent string + move data into Rust Intent enum.
///
/// CommunicationMod exposes:
/// - `intent`: "ATTACK", "DEFEND", "BUFF", "DEBUFF", "ATTACK_BUFF", etc.
/// - `move_adjusted_damage`: damage after STR applied
/// - `move_hits`: number of hits  
fn hydrate_intent(json: &Value) -> Intent {
    let intent_str = json["intent"].as_str().unwrap_or("UNKNOWN");
    let damage = json["move_adjusted_damage"].as_i64()
        .or_else(|| json["move_base_damage"].as_i64())
        .unwrap_or(0) as i32;
    let hits = json["move_hits"].as_i64().unwrap_or(1) as i32;

    match intent_str {
        "ATTACK" => Intent::Attack { damage, times: hits },
        "ATTACK_BUFF" => Intent::Attack { damage, times: hits }, // Simplified
        "ATTACK_DEBUFF" => Intent::Attack { damage, times: hits },
        "ATTACK_DEFEND" => Intent::AttackDefend { damage, block: 0 },
        "DEFEND" => Intent::Defend { block: 0 },
        "DEFEND_BUFF" => Intent::Defend { block: 0 },
        "BUFF" => Intent::Buff { name: String::new(), amount: 0 },
        "DEBUFF" => Intent::Debuff { name: String::new(), amount: 0 },
        "STRONG_DEBUFF" => Intent::Debuff { name: String::new(), amount: 0 },
        "DEFEND_DEBUFF" => Intent::Debuff { name: String::new(), amount: 0 },
        "STUN" => Intent::Stunned,
        "SLEEP" => Intent::Sleep,
        "ESCAPE" => Intent::Escape,
        "MAGIC" => Intent::Special { name: "MAGIC".to_string() },
        "NONE" => Intent::Unknown,
        "UNKNOWN" => Intent::Unknown,
        _ => Intent::Unknown,
    }
}

/// Create a MonsterState from a CommunicationMod monster JSON object.
pub fn hydrate_enemy(json: &Value) -> MonsterState {
    let name = json["name"].as_str().unwrap_or("Unknown").to_string();
    let hp = json["current_hp"].as_i64().unwrap_or(0) as i32;
    let max_hp = json["max_hp"].as_i64().unwrap_or(hp as i64) as i32;
    let block = json["block"].as_i64().unwrap_or(0) as i32;

    let is_gone = json["is_gone"].as_bool().unwrap_or(false);
    let half_dead = json["half_dead"].as_bool().unwrap_or(false);
    let is_escaping = json["is_escaping"].as_bool().unwrap_or(false);
    let misc_bool = json["miscBool"].as_bool().unwrap_or(false);
    let misc_int = json["miscInt"].as_i64().unwrap_or(0) as i32;

    let mut monster = MonsterState::new_simple(&name, max_hp);
    monster.hp = hp;
    monster.max_hp = max_hp;
    monster.block = block;
    monster.alive = !is_gone && !half_dead && hp > 0;
    monster.current_intent = hydrate_intent(json);
    monster.is_escaping = is_escaping;
    monster.misc_bool = misc_bool;
    monster.misc_int = misc_int;

    // Hydrate powers
    if let Some(power_arr) = json["powers"].as_array() {
        for p in power_arr {
            if let Some(raw_id) = p["id"].as_str() {
                let id = commod_to_engine_power_id(raw_id);
                let amount = p["amount"].as_i64().unwrap_or(0) as i32;
                monster.powers.set(&id, amount);
            }
        }
    }

    // Monster-specific state recovery from powers
    // Guardian: if Mode Shift power exists, set is_open=true and dmg_threshold
    let monster_id = json["id"].as_str().unwrap_or("");
    if monster_id == "TheGuardian" {
        if monster.powers.has("Mode Shift") {
            monster.is_open = true;
            monster.dmg_threshold = monster.powers.get("Mode Shift");
        }
        monster.definition_id = "TheGuardian".to_string();
    }

    // Awakened One: if Unawakened power exists, set activated=true to prevent
    // the synchronous rebirth from firing during step verification.
    // Java defers rebirth via ActionQueue (snapshot captures HP=0), but Rust
    // does it synchronously in check_damage_triggers (HP→max). Setting activated=true
    // makes check_damage_triggers skip the rebirth, matching Java's step-level snapshot.
    if monster_id == "AwakenedOne" || name == "Awakened One" {
        if monster.powers.has("Unawakened") {
            // Phase 1: Unawakened present → set activated=true to PREVENT rebirth
            // (rebirth in Java is a deferred action that appears in the NEXT step)
            monster.activated = true;
        }
        monster.definition_id = "AwakenedOne".to_string();
    }

    monster
}

// ============================================================================
// Relic Hydration
// ============================================================================

/// Create a RelicInstance from CommunicationMod relic JSON.
pub fn hydrate_relic(json: &Value) -> RelicInstance {
    let id = json["id"].as_str().unwrap_or("");
    // First: map Java relic IDs to Rust engine names
    let engine_id = commod_to_engine_relic_id(id);
    // Then: normalize spaces (some relics use spaces in CommunicationMod but not in Rust)
    let normalized_id = engine_id.replace(' ', "");

    RelicInstance {
        id: normalized_id,
        counter: json["counter"].as_i64().unwrap_or(-1) as i32,
        active: true,
        pulsed: false,
    }
}

/// Convert a CommunicationMod relic ID to the engine's internal relic ID.
/// Delegates to the centralized `id_map` module.
fn commod_to_engine_relic_id(commod_id: &str) -> String {
    super::id_map::commod_to_engine_relic_id(commod_id)
}



/// Hydrate all relics from a JSON array.
pub fn hydrate_relics(arr: &Value) -> Vec<RelicInstance> {
    arr.as_array()
        .map(|relics| relics.iter().map(hydrate_relic).collect())
        .unwrap_or_default()
}

// ============================================================================
// Full GameState Hydration
// ============================================================================

/// Hydrate a complete GameState from a CommunicationMod JSON state snapshot.
///
/// This is the main entry point. It takes the full `{"game_state": {...}}` JSON
/// and produces an executable GameState ready for `play_card_from_hand()`.
///
/// # Arguments
/// * `json` - The full CommunicationMod state (containing "game_state" key)
///
/// # Returns
/// `Some(GameState)` if in combat, `None` otherwise.
pub fn hydrate_combat_state(json: &Value) -> Option<GameState> {
    let gs = json.get("game_state")?;
    let cs = gs.get("combat_state")?;
    if cs.is_null() {
        return None;
    }

    let player_json = &cs["player"];

    // Build game state
    let mut state = GameState::new(0); // seed doesn't matter for injection

    // Player
    state.player = hydrate_player(player_json);

    // Card piles
    state.hand = hydrate_card_pile(&cs["hand"]);
    state.draw_pile = hydrate_card_pile(&cs["draw_pile"]);
    state.discard_pile = hydrate_card_pile(&cs["discard_pile"]);
    state.exhaust_pile = hydrate_card_pile(&cs["exhaust_pile"]);

    // Enemies
    state.enemies = cs["monsters"].as_array()
        .map(|monsters| {
            monsters.iter().map(hydrate_enemy).collect()
        })
        .unwrap_or_default();

    // Relics
    state.relics = hydrate_relics(&gs["relics"]);

    // Turn
    state.turn = cs["turn"].as_u64().unwrap_or(1) as u32;

    // Combat screen
    state.screen = GamePhase::Combat;

    // Gold
    state.gold = gs["gold"].as_i64().unwrap_or(0) as i32;

    // Ascension
    state.ascension_level = gs.get("ascension_level")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;

    // Floor
    state.floor = gs.get("floor")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;

    // RNG: use a fixed dummy — we're injecting state, not replaying from seed
    state.rng = Xoshiro256StarStar::seed_from_u64(42);
    state.encounter_rng = Xoshiro256StarStar::seed_from_u64(42);

    Some(state)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_hydrate_card_basic() {
        let card_json = json!({
            "id": "Strike_R",
            "name": "Strike",
            "type": "ATTACK",
            "cost": 1,
            "upgrades": 0,
            "has_target": true,
            "exhausts": false,
            "ethereal": false
        });

        let card = hydrate_card(&card_json);
        assert_eq!(card.definition_id, "Strike_Ironclad");
        assert_eq!(card.current_cost, 1);
        assert!(!card.upgraded);
        assert_eq!(card.card_type, CardType::Attack);
        assert!(!card.is_ethereal);
    }

    #[test]
    fn test_hydrate_card_upgraded() {
        let card_json = json!({
            "id": "Bash",
            "type": "ATTACK",
            "cost": 2,
            "upgrades": 1
        });

        let card = hydrate_card(&card_json);
        assert_eq!(card.definition_id, "Bash");
        assert!(card.upgraded);
        assert_eq!(card.current_cost, 2);
    }

    #[test]
    fn test_hydrate_player() {
        let player_json = json!({
            "current_hp": 65,
            "max_hp": 80,
            "block": 12,
            "energy": 2,
            "powers": [
                { "id": "Strength", "amount": 3 },
                { "id": "Vulnerable", "amount": 1 }
            ]
        });

        let player = hydrate_player(&player_json);
        assert_eq!(player.current_hp, 65);
        assert_eq!(player.max_hp, 80);
        assert_eq!(player.block, 12);
        assert_eq!(player.energy, 2);
        assert_eq!(player.powers.get("Strength"), 3);
        assert_eq!(player.powers.get("Vulnerable"), 1);
    }

    #[test]
    fn test_hydrate_enemy() {
        let enemy_json = json!({
            "name": "Jaw Worm",
            "id": "JawWorm",
            "current_hp": 40,
            "max_hp": 44,
            "block": 5,
            "is_gone": false,
            "half_dead": false,
            "intent": "ATTACK",
            "move_adjusted_damage": 11,
            "move_hits": 1,
            "powers": [
                { "id": "Strength", "amount": 5 }
            ]
        });

        let enemy = hydrate_enemy(&enemy_json);
        assert_eq!(enemy.name, "Jaw Worm");
        assert_eq!(enemy.hp, 40);
        assert_eq!(enemy.max_hp, 44);
        assert_eq!(enemy.block, 5);
        assert!(enemy.alive);
        assert_eq!(enemy.powers.get("Strength"), 5);
        assert!(matches!(enemy.current_intent, Intent::Attack { damage: 11, times: 1 }));
    }

    #[test]
    fn test_hydrate_relic() {
        let relic_json = json!({
            "id": "Burning Blood",
            "counter": -1
        });

        let relic = hydrate_relic(&relic_json);
        assert_eq!(relic.id, "BurningBlood");
        assert_eq!(relic.counter, -1);
        assert!(relic.active);
    }

    #[test]
    fn test_hydrate_combat_state_basic() {
        let state_json = json!({
            "game_state": {
                "gold": 99,
                "relics": [
                    { "id": "Burning Blood", "counter": -1 }
                ],
                "combat_state": {
                    "turn": 2,
                    "player": {
                        "current_hp": 65,
                        "max_hp": 80,
                        "block": 0,
                        "energy": 3,
                        "powers": []
                    },
                    "hand": [
                        { "id": "Strike_R", "type": "ATTACK", "cost": 1, "upgrades": 0 },
                        { "id": "Defend_R", "type": "SKILL", "cost": 1, "upgrades": 0 }
                    ],
                    "draw_pile": [
                        { "id": "Bash", "type": "ATTACK", "cost": 2, "upgrades": 0 }
                    ],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "monsters": [
                        {
                            "name": "Cultist",
                            "id": "Cultist",
                            "current_hp": 48,
                            "max_hp": 48,
                            "block": 0,
                            "is_gone": false,
                            "half_dead": false,
                            "intent": "ATTACK",
                            "move_adjusted_damage": 6,
                            "move_hits": 1,
                            "powers": [{ "id": "Ritual", "amount": 3 }]
                        }
                    ]
                }
            }
        });

        let state = hydrate_combat_state(&state_json).expect("Should hydrate");
        assert_eq!(state.turn, 2);
        assert_eq!(state.player.current_hp, 65);
        assert_eq!(state.hand.len(), 2);
        assert_eq!(state.hand[0].definition_id, "Strike_Ironclad");
        assert_eq!(state.hand[1].definition_id, "Defend_Ironclad");
        assert_eq!(state.draw_pile.len(), 1);
        assert_eq!(state.enemies.len(), 1);
        assert_eq!(state.enemies[0].name, "Cultist");
        assert_eq!(state.enemies[0].powers.get("Ritual"), 3);
        assert_eq!(state.relics.len(), 1);
        assert_eq!(state.relics[0].id, "BurningBlood");
        assert_eq!(state.gold, 99);
    }

    #[test]
    fn test_hydrate_non_combat_returns_none() {
        let state_json = json!({
            "game_state": {
                "gold": 99,
                "relics": [],
                "combat_state": null
            }
        });

        assert!(hydrate_combat_state(&state_json).is_none());
    }
}
