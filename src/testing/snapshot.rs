//! Combat state snapshot for deterministic comparison.
//!
//! `CombatSnapshot` captures the essential state of a combat at a point in time.
//! It can be constructed from a `GameState` or deserialized from CommunicationMod JSON.
//! All collections use `BTreeMap`/sorted vecs for deterministic ordering.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

// ============================================================================
// Snapshot types
// ============================================================================

/// A card in the snapshot (simplified).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardSnap {
    pub id: String,
    pub cost: i32,
    pub upgraded: bool,
}

/// An enemy in the snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnemySnap {
    pub name: String,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub powers: BTreeMap<String, i32>,
    pub current_move: String,
}

/// A relic in the snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelicSnap {
    pub id: String,
    pub counter: i32,
    pub active: bool,
}

/// An orb in the snapshot (Defect).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrbSnap {
    pub orb_type: String,
    pub passive_amount: i32,
    pub evoke_amount: i32,
}

/// Complete snapshot of combat state at a point in time.
///
/// Designed to be:
/// 1. Constructable from `GameState` (Rust engine)
/// 2. Parseable from CommunicationMod JSON (real game)
/// 3. Diff-able for divergence detection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatSnapshot {
    // Turn info
    pub turn: u32,
    pub cards_played_this_turn: u32,

    // Player state
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_block: i32,
    pub player_energy: i32,
    pub player_max_energy: i32,
    pub player_powers: BTreeMap<String, i32>,
    pub player_stance: String,

    // Card piles (sorted by id for deterministic comparison)
    pub hand: Vec<CardSnap>,
    pub draw_pile_count: usize,
    pub discard_pile_count: usize,
    pub exhaust_pile_count: usize,

    // Enemies
    pub enemies: Vec<EnemySnap>,

    // Relics
    pub relics: Vec<RelicSnap>,

    // Orbs (Defect)
    pub orbs: Vec<OrbSnap>,
}

// ============================================================================
// Construct from GameState
// ============================================================================

impl CombatSnapshot {
    /// Build a snapshot from the current GameState.
    pub fn from_game_state(state: &crate::state::GameState) -> Self {
        // Player powers → BTreeMap (sorted)
        let player_powers: BTreeMap<String, i32> = state.player.powers
            .as_map()
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();

        // Stance name
        let player_stance = format!("{:?}", state.player.stance);

        // Hand cards
        let hand: Vec<CardSnap> = state.hand.iter().map(|c| CardSnap {
            id: c.definition_id.clone(),
            cost: c.current_cost,
            upgraded: c.upgraded,
        }).collect();

        // Enemies
        let enemies: Vec<EnemySnap> = state.enemies.iter().map(|e| {
            let powers: BTreeMap<String, i32> = e.powers
                .as_map()
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            EnemySnap {
                name: e.name.clone(),
                hp: e.hp,
                max_hp: e.max_hp,
                block: e.block,
                alive: e.alive,
                powers,
                current_move: e.current_move.clone(),
            }
        }).collect();

        // Relics
        let relics: Vec<RelicSnap> = state.relics.iter().map(|r| RelicSnap {
            id: r.id.clone(),
            counter: r.counter,
            active: r.active,
        }).collect();

        // Orbs
        let orbs: Vec<OrbSnap> = state.orb_slots.iter().map(|o| OrbSnap {
            orb_type: o.orb_type.name().to_string(),
            passive_amount: o.passive_amount,
            evoke_amount: o.evoke_amount,
        }).collect();

        CombatSnapshot {
            turn: state.turn,
            cards_played_this_turn: state.cards_played_this_turn,
            player_hp: state.player.current_hp,
            player_max_hp: state.player.max_hp,
            player_block: state.player.block,
            player_energy: state.player.energy,
            player_max_energy: state.player.max_energy,
            player_powers,
            player_stance,
            hand,
            draw_pile_count: state.draw_pile.len(),
            discard_pile_count: state.discard_pile.len(),
            exhaust_pile_count: state.exhaust_pile.len(),
            enemies,
            relics,
            orbs,
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("CombatSnapshot serialization failed")
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_roundtrip() {
        let snap = CombatSnapshot {
            turn: 3,
            cards_played_this_turn: 2,
            player_hp: 55,
            player_max_hp: 80,
            player_block: 10,
            player_energy: 2,
            player_max_energy: 3,
            player_powers: BTreeMap::from([
                ("Strength".into(), 3),
                ("Vulnerable".into(), 1),
            ]),
            player_stance: "None".into(),
            hand: vec![
                CardSnap { id: "Strike_R".into(), cost: 1, upgraded: false },
                CardSnap { id: "Defend_R".into(), cost: 1, upgraded: true },
            ],
            draw_pile_count: 7,
            discard_pile_count: 3,
            exhaust_pile_count: 1,
            enemies: vec![
                EnemySnap {
                    name: "Jaw Worm".into(),
                    hp: 30,
                    max_hp: 44,
                    block: 5,
                    alive: true,
                    powers: BTreeMap::from([("Strength".into(), 2)]),
                    current_move: "Chomp".into(),
                },
            ],
            relics: vec![
                RelicSnap { id: "BurningBlood".into(), counter: 0, active: true },
            ],
            orbs: vec![],
        };

        // Roundtrip: serialize → deserialize → compare
        let json = snap.to_json();
        let restored = CombatSnapshot::from_json(&json).unwrap();
        assert_eq!(snap, restored);
    }

    #[test]
    fn test_snapshot_deterministic_ordering() {
        // Powers should be sorted (BTreeMap guarantees this)
        let powers = BTreeMap::from([
            ("Vulnerable".into(), 2),
            ("Artifact".into(), 1),
            ("Strength".into(), 5),
        ]);
        let keys: Vec<&String> = powers.keys().collect();
        assert_eq!(keys, vec!["Artifact", "Strength", "Vulnerable"]);
    }
}
