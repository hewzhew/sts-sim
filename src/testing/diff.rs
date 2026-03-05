//! Snapshot diff engine — compare two CombatSnapshots and report divergences.
//!
//! Produces a structured list of `Divergence` values, each describing
//! exactly one field mismatch between expected and actual combat state.

use std::collections::BTreeMap;
use super::snapshot::*;

// ============================================================================
// Divergence types
// ============================================================================

/// A single field-level mismatch between two snapshots.
#[derive(Debug, Clone, PartialEq)]
pub enum Divergence {
    // Turn tracking
    Turn { expected: u32, actual: u32 },
    CardsPlayed { expected: u32, actual: u32 },

    // Player scalars
    PlayerHp { expected: i32, actual: i32 },
    PlayerMaxHp { expected: i32, actual: i32 },
    PlayerBlock { expected: i32, actual: i32 },
    PlayerEnergy { expected: i32, actual: i32 },
    PlayerStance { expected: String, actual: String },

    // Player powers
    PowerMissing { power: String, expected_stacks: i32 },
    PowerExtra { power: String, actual_stacks: i32 },
    PowerMismatch { power: String, expected: i32, actual: i32 },

    // Card piles
    HandCount { expected: usize, actual: usize },
    HandCardDiff { index: usize, expected: CardSnap, actual: CardSnap },
    DrawPileCount { expected: usize, actual: usize },
    DiscardPileCount { expected: usize, actual: usize },
    ExhaustPileCount { expected: usize, actual: usize },

    // Enemies
    EnemyCount { expected: usize, actual: usize },
    EnemyHp { enemy: String, expected: i32, actual: i32 },
    EnemyBlock { enemy: String, expected: i32, actual: i32 },
    EnemyAlive { enemy: String, expected: bool, actual: bool },
    EnemyPowerMissing { enemy: String, power: String, expected_stacks: i32 },
    EnemyPowerExtra { enemy: String, power: String, actual_stacks: i32 },
    EnemyPowerMismatch { enemy: String, power: String, expected: i32, actual: i32 },
    EnemyMoveMismatch { enemy: String, expected: String, actual: String },

    // Relics
    RelicCounterMismatch { relic: String, expected: i32, actual: i32 },
    RelicActiveMismatch { relic: String, expected: bool, actual: bool },

    // Orbs
    OrbCount { expected: usize, actual: usize },
    OrbMismatch { index: usize, expected: OrbSnap, actual: OrbSnap },
}

impl std::fmt::Display for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Divergence::PlayerHp { expected, actual } =>
                write!(f, "Player HP: expected {}, got {}", expected, actual),
            Divergence::PlayerBlock { expected, actual } =>
                write!(f, "Player Block: expected {}, got {}", expected, actual),
            Divergence::PlayerEnergy { expected, actual } =>
                write!(f, "Player Energy: expected {}, got {}", expected, actual),
            Divergence::PowerMismatch { power, expected, actual } =>
                write!(f, "Power '{}': expected {}, got {}", power, expected, actual),
            Divergence::PowerMissing { power, expected_stacks } =>
                write!(f, "Power '{}': expected {} stacks, not found", power, expected_stacks),
            Divergence::PowerExtra { power, actual_stacks } =>
                write!(f, "Power '{}': unexpected with {} stacks", power, actual_stacks),
            Divergence::EnemyHp { enemy, expected, actual } =>
                write!(f, "Enemy '{}' HP: expected {}, got {}", enemy, expected, actual),
            Divergence::EnemyBlock { enemy, expected, actual } =>
                write!(f, "Enemy '{}' Block: expected {}, got {}", enemy, expected, actual),
            _ => write!(f, "{:?}", self),
        }
    }
}

// ============================================================================
// Diff function
// ============================================================================

/// Compare two snapshots and return all divergences.
///
/// Returns an empty vec if they are identical.
pub fn diff_snapshots(expected: &CombatSnapshot, actual: &CombatSnapshot) -> Vec<Divergence> {
    let mut diffs = Vec::new();

    // Turn tracking
    if expected.turn != actual.turn {
        diffs.push(Divergence::Turn { expected: expected.turn, actual: actual.turn });
    }
    if expected.cards_played_this_turn != actual.cards_played_this_turn {
        diffs.push(Divergence::CardsPlayed {
            expected: expected.cards_played_this_turn,
            actual: actual.cards_played_this_turn,
        });
    }

    // Player scalars
    if expected.player_hp != actual.player_hp {
        diffs.push(Divergence::PlayerHp { expected: expected.player_hp, actual: actual.player_hp });
    }
    if expected.player_max_hp != actual.player_max_hp {
        diffs.push(Divergence::PlayerMaxHp { expected: expected.player_max_hp, actual: actual.player_max_hp });
    }
    if expected.player_block != actual.player_block {
        diffs.push(Divergence::PlayerBlock { expected: expected.player_block, actual: actual.player_block });
    }
    if expected.player_energy != actual.player_energy {
        diffs.push(Divergence::PlayerEnergy { expected: expected.player_energy, actual: actual.player_energy });
    }
    if expected.player_stance != actual.player_stance {
        diffs.push(Divergence::PlayerStance {
            expected: expected.player_stance.clone(),
            actual: actual.player_stance.clone(),
        });
    }

    // Player powers
    diff_powers(&expected.player_powers, &actual.player_powers, &mut diffs);

    // Card piles
    if expected.hand.len() != actual.hand.len() {
        diffs.push(Divergence::HandCount { expected: expected.hand.len(), actual: actual.hand.len() });
    } else {
        for (i, (e, a)) in expected.hand.iter().zip(actual.hand.iter()).enumerate() {
            if e != a {
                diffs.push(Divergence::HandCardDiff { index: i, expected: e.clone(), actual: a.clone() });
            }
        }
    }
    if expected.draw_pile_count != actual.draw_pile_count {
        diffs.push(Divergence::DrawPileCount { expected: expected.draw_pile_count, actual: actual.draw_pile_count });
    }
    if expected.discard_pile_count != actual.discard_pile_count {
        diffs.push(Divergence::DiscardPileCount { expected: expected.discard_pile_count, actual: actual.discard_pile_count });
    }
    if expected.exhaust_pile_count != actual.exhaust_pile_count {
        diffs.push(Divergence::ExhaustPileCount { expected: expected.exhaust_pile_count, actual: actual.exhaust_pile_count });
    }

    // Enemies
    if expected.enemies.len() != actual.enemies.len() {
        diffs.push(Divergence::EnemyCount { expected: expected.enemies.len(), actual: actual.enemies.len() });
    } else {
        for (e, a) in expected.enemies.iter().zip(actual.enemies.iter()) {
            let name = &e.name;
            if e.hp != a.hp {
                diffs.push(Divergence::EnemyHp { enemy: name.clone(), expected: e.hp, actual: a.hp });
            }
            if e.block != a.block {
                diffs.push(Divergence::EnemyBlock { enemy: name.clone(), expected: e.block, actual: a.block });
            }
            if e.alive != a.alive {
                diffs.push(Divergence::EnemyAlive { enemy: name.clone(), expected: e.alive, actual: a.alive });
            }
            if e.current_move != a.current_move {
                diffs.push(Divergence::EnemyMoveMismatch {
                    enemy: name.clone(),
                    expected: e.current_move.clone(),
                    actual: a.current_move.clone(),
                });
            }
            // Enemy powers
            diff_enemy_powers(name, &e.powers, &a.powers, &mut diffs);
        }
    }

    // Relics (compare by position)
    let relic_count = expected.relics.len().min(actual.relics.len());
    for i in 0..relic_count {
        let e = &expected.relics[i];
        let a = &actual.relics[i];
        if e.id == a.id {
            if e.counter != a.counter {
                diffs.push(Divergence::RelicCounterMismatch {
                    relic: e.id.clone(), expected: e.counter, actual: a.counter,
                });
            }
            if e.active != a.active {
                diffs.push(Divergence::RelicActiveMismatch {
                    relic: e.id.clone(), expected: e.active, actual: a.active,
                });
            }
        }
    }

    // Orbs
    if expected.orbs.len() != actual.orbs.len() {
        diffs.push(Divergence::OrbCount { expected: expected.orbs.len(), actual: actual.orbs.len() });
    } else {
        for (i, (e, a)) in expected.orbs.iter().zip(actual.orbs.iter()).enumerate() {
            if e != a {
                diffs.push(Divergence::OrbMismatch { index: i, expected: e.clone(), actual: a.clone() });
            }
        }
    }

    diffs
}

/// Diff player powers.
///
/// Java uses -1 stacks for permanent non-stackable powers (Corruption, Barricade,
/// etc.) while Rust uses positive values. We normalize this: if expected=-1 and
/// actual>0 (or vice versa), both mean "power is present" → no divergence.
fn diff_powers(
    expected: &BTreeMap<String, i32>,
    actual: &BTreeMap<String, i32>,
    diffs: &mut Vec<Divergence>,
) {
    for (key, &exp_val) in expected {
        match actual.get(key) {
            Some(&act_val) if act_val != exp_val => {
                // Normalize: Java -1 ↔ Rust positive both mean "power present"
                let both_present = (exp_val == -1 && act_val > 0) || (act_val == -1 && exp_val > 0);
                if !both_present {
                    diffs.push(Divergence::PowerMismatch {
                        power: key.clone(), expected: exp_val, actual: act_val,
                    });
                }
            }
            None => {
                diffs.push(Divergence::PowerMissing {
                    power: key.clone(), expected_stacks: exp_val,
                });
            }
            _ => {} // match
        }
    }
    for (key, &act_val) in actual {
        if !expected.contains_key(key) {
            diffs.push(Divergence::PowerExtra {
                power: key.clone(), actual_stacks: act_val,
            });
        }
    }
}

/// Diff enemy powers.
fn diff_enemy_powers(
    enemy_name: &str,
    expected: &BTreeMap<String, i32>,
    actual: &BTreeMap<String, i32>,
    diffs: &mut Vec<Divergence>,
) {
    for (key, &exp_val) in expected {
        match actual.get(key) {
            Some(&act_val) if act_val != exp_val => {
                diffs.push(Divergence::EnemyPowerMismatch {
                    enemy: enemy_name.into(), power: key.clone(),
                    expected: exp_val, actual: act_val,
                });
            }
            None => {
                diffs.push(Divergence::EnemyPowerMissing {
                    enemy: enemy_name.into(), power: key.clone(),
                    expected_stacks: exp_val,
                });
            }
            _ => {}
        }
    }
    for (key, &act_val) in actual {
        if !expected.contains_key(key) {
            diffs.push(Divergence::EnemyPowerExtra {
                enemy: enemy_name.into(), power: key.clone(),
                actual_stacks: act_val,
            });
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn base_snapshot() -> CombatSnapshot {
        CombatSnapshot {
            turn: 1,
            cards_played_this_turn: 0,
            player_hp: 70,
            player_max_hp: 80,
            player_block: 0,
            player_energy: 3,
            player_max_energy: 3,
            player_powers: BTreeMap::new(),
            player_stance: "None".into(),
            hand: vec![
                CardSnap { id: "Strike_R".into(), cost: 1, upgraded: false },
            ],
            draw_pile_count: 8,
            discard_pile_count: 0,
            exhaust_pile_count: 0,
            enemies: vec![
                EnemySnap {
                    name: "Jaw Worm".into(),
                    hp: 44,
                    max_hp: 44,
                    block: 0,
                    alive: true,
                    powers: BTreeMap::new(),
                    current_move: "Chomp".into(),
                },
            ],
            relics: vec![
                RelicSnap { id: "BurningBlood".into(), counter: 0, active: true },
            ],
            orbs: vec![],
        }
    }

    #[test]
    fn test_identical_snapshots_no_diff() {
        let a = base_snapshot();
        let b = a.clone();
        let diffs = diff_snapshots(&a, &b);
        assert!(diffs.is_empty(), "Expected no diffs, got: {:?}", diffs);
    }

    #[test]
    fn test_player_hp_diff() {
        let a = base_snapshot();
        let mut b = a.clone();
        b.player_hp = 60;
        let diffs = diff_snapshots(&a, &b);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0], Divergence::PlayerHp { expected: 70, actual: 60 }));
    }

    #[test]
    fn test_power_diff() {
        let mut a = base_snapshot();
        a.player_powers.insert("Strength".into(), 3);
        a.player_powers.insert("Vulnerable".into(), 2);

        let mut b = a.clone();
        b.player_powers.insert("Strength".into(), 5);   // mismatch
        b.player_powers.remove("Vulnerable");             // missing
        b.player_powers.insert("Weak".into(), 1);         // extra

        let diffs = diff_snapshots(&a, &b);
        assert_eq!(diffs.len(), 3);
        assert!(diffs.iter().any(|d| matches!(d,
            Divergence::PowerMismatch { power, expected: 3, actual: 5 } if power == "Strength"
        )));
        assert!(diffs.iter().any(|d| matches!(d,
            Divergence::PowerMissing { power, expected_stacks: 2 } if power == "Vulnerable"
        )));
        assert!(diffs.iter().any(|d| matches!(d,
            Divergence::PowerExtra { power, actual_stacks: 1 } if power == "Weak"
        )));
    }

    #[test]
    fn test_enemy_hp_diff() {
        let a = base_snapshot();
        let mut b = a.clone();
        b.enemies[0].hp = 30;
        let diffs = diff_snapshots(&a, &b);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(&diffs[0], Divergence::EnemyHp { enemy, expected: 44, actual: 30 } if enemy == "Jaw Worm"));
    }

    #[test]
    fn test_multiple_diffs() {
        let a = base_snapshot();
        let mut b = a.clone();
        b.player_hp = 60;
        b.player_block = 5;
        b.enemies[0].hp = 30;
        b.draw_pile_count = 7;
        let diffs = diff_snapshots(&a, &b);
        assert_eq!(diffs.len(), 4);
    }

    #[test]
    fn test_display_formatting() {
        let d = Divergence::PlayerHp { expected: 70, actual: 60 };
        let s = format!("{}", d);
        assert!(s.contains("70"));
        assert!(s.contains("60"));
    }
}
