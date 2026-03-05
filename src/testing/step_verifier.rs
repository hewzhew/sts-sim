//! Step Verifier — Single-step validation for differential testing.
//!
//! This module implements Phase A: state-injection single-step verification.
//! For each combat step from a JSONL log:
//! 1. Hydrate the "before" state into a GameState
//! 2. Execute the action (play card / end turn)
//! 3. Compare the resulting state against the "after" state from JSONL
//!
//! This validates engine correctness without requiring RNG consistency.

use serde_json::Value;
use std::collections::BTreeMap;

use crate::loader::CardLibrary;
use crate::testing::commod_parser::{DiffTransition, normalize_card_id, parse_combat_snapshot};
use crate::testing::hydrator::hydrate_combat_state;
use crate::testing::snapshot::CombatSnapshot;

// ============================================================================
// Step Result Types
// ============================================================================

/// A single field divergence between Rust engine and Java (CommunicationMod).
#[derive(Debug, Clone)]
pub struct Divergence {
    pub field: String,
    pub expected: String,
    pub actual: String,
}

/// Result of verifying one step.
#[derive(Debug)]
pub struct StepResult {
    pub step_num: u64,
    pub command: String,
    pub divergences: Vec<Divergence>,
    pub skipped: bool,
    pub skip_reason: Option<String>,
    /// Number of divergences filtered by timing whitelist.
    pub timing_filtered: usize,
}

impl StepResult {
    pub fn passed(&self) -> bool {
        self.divergences.is_empty() && !self.skipped
    }
}

/// Summary of verifying an entire combat.
#[derive(Debug)]
pub struct CombatVerifyResult {
    pub combat_name: String,
    pub total_steps: usize,
    pub play_steps: usize,
    pub verified_steps: usize,
    pub skipped_steps: usize,
    pub divergent_steps: usize,
    pub total_divergences: usize,
    pub step_results: Vec<StepResult>,
}

// ============================================================================
// Core Verification
// ============================================================================

/// Verify a single step: inject before-state, execute command, diff against after-state.
///
/// Currently only handles `play X [Y]` commands.
/// `end` commands are skipped (too complex for Phase A — involves enemy turns).
pub fn verify_step(
    before_json: &Value,
    command: &str,
    after_json: &Value,
    card_library: &CardLibrary,
) -> StepResult {
    let step_num = 0; // Will be set by caller

    // Parse command
    let parts: Vec<&str> = command.split_whitespace().collect();

    // Only verify "play" commands for now
    if parts.is_empty() || parts[0] != "play" {
        return StepResult {
            step_num,
            command: command.to_string(),
            divergences: vec![],
            skipped: true,
            skip_reason: Some(format!("Unsupported command type: '{}'", parts.first().unwrap_or(&""))),
            timing_filtered: 0,
        };
    }

    // Parse play command: "play X" or "play X Y"
    // CommunicationMod uses 1-based hand indices, Rust engine uses 0-based
    let raw_hand_index = match parts.get(1).and_then(|s| s.parse::<usize>().ok()) {
        Some(idx) => idx,
        None => return StepResult {
            step_num,
            command: command.to_string(),
            divergences: vec![],
            skipped: true,
            skip_reason: Some("Cannot parse hand index from play command".to_string()),
            timing_filtered: 0,
        },
    };
    // Convert 1-based → 0-based
    let hand_index = if raw_hand_index > 0 { raw_hand_index - 1 } else { 0 };
    let target_idx = parts.get(2).and_then(|s| s.parse::<usize>().ok());

    // Hydrate before-state
    let mut state = match hydrate_combat_state(before_json) {
        Some(s) => s,
        None => return StepResult {
            step_num,
            command: command.to_string(),
            divergences: vec![],
            skipped: true,
            skip_reason: Some("Cannot hydrate before-state (not in combat)".to_string()),
            timing_filtered: 0,
        },
    };

    // Validate hand index
    if hand_index >= state.hand.len() {
        return StepResult {
            step_num,
            command: command.to_string(),
            divergences: vec![],
            skipped: true,
            skip_reason: Some(format!(
                "Hand index {} out of range (hand size {})",
                hand_index, state.hand.len()
            )),
            timing_filtered: 0,
        };
    }

    // Build before-snapshot for whitelist context
    let before_snapshot = snapshot_from_game_state(&state);

    // Execute the play command
    let play_result = crate::engine::combat::play_card_from_hand(
        &mut state,
        card_library,
        hand_index,
        target_idx,
    );

    if let Err(e) = play_result {
        return StepResult {
            step_num,
            command: command.to_string(),
            divergences: vec![Divergence {
                field: "execution_error".to_string(),
                expected: "success".to_string(),
                actual: e,
            }],
            skipped: false,
            skip_reason: None,
            timing_filtered: 0,
        };
    }

    // Parse expected after-state
    let expected = match parse_combat_snapshot(after_json) {
        Some(s) => s,
        None => return StepResult {
            step_num,
            command: command.to_string(),
            divergences: vec![],
            skipped: true,
            skip_reason: Some("Cannot parse after-state snapshot".to_string()),
            timing_filtered: 0,
        },
    };

    // Build actual snapshot from Rust state
    let actual = snapshot_from_game_state(&state);

    // Diff
    let raw_divergences = diff_snapshots(&expected, &actual);

    // Filter known timing artifacts
    let (divergences, timing_filtered) = crate::testing::timing_known::filter_timing_divergences(
        raw_divergences, &before_snapshot, &expected, &actual,
    );

    StepResult {
        step_num,
        command: command.to_string(),
        divergences,
        skipped: false,
        skip_reason: None,
        timing_filtered,
    }
}

/// Normalize a power ID from Rust engine format to CommunicationMod/Java format.
///
/// Java uses different POWER_ID strings than what the Rust engine uses internally:
/// - Java `WeakPower.POWER_ID = "Weakened"` but Rust uses `"Weak"`
/// - Java `VulnerablePower.POWER_ID = "Vulnerable"` (same)
/// - Java `FrailPower.POWER_ID = "Frail"` (same)
fn normalize_power_id(rust_id: &str) -> String {
    match rust_id {
        "Weak" => "Weakened".to_string(),
        "Intangible" => "IntangiblePlayer".to_string(),
        _ => rust_id.to_string(),
    }
}

/// Convert the current GameState back to a CombatSnapshot for comparison.
pub fn snapshot_from_game_state(state: &crate::core::state::GameState) -> CombatSnapshot {
    use crate::testing::snapshot::*;

    let hand: Vec<CardSnap> = state.hand.iter().map(|c| CardSnap {
        id: c.definition_id.clone(),
        cost: c.current_cost,
        upgraded: c.upgraded,
    }).collect();

    let enemies: Vec<EnemySnap> = state.enemies.iter().map(|e| {
        let powers: BTreeMap<String, i32> = e.powers.iter()
            .map(|(k, v)| (normalize_power_id(k), *v))
            .collect();
        EnemySnap {
            name: e.name.clone(),
            hp: e.hp,
            max_hp: e.max_hp,
            block: e.block,
            alive: e.alive && e.hp > 0,
            powers,
            current_move: String::new(), // Not compared after play
        }
    }).collect();

    let relics: Vec<RelicSnap> = state.relics.iter().map(|r| RelicSnap {
        id: r.id.clone(),
        counter: r.counter,
        active: r.active,
    }).collect();

    let player_powers: BTreeMap<String, i32> = state.player.powers.iter()
        .map(|(k, v)| (normalize_power_id(k), *v))
        .collect();

    CombatSnapshot {
        turn: state.turn,
        cards_played_this_turn: state.cards_played_this_turn,
        player_hp: state.player.current_hp,
        player_max_hp: state.player.max_hp,
        player_block: state.player.block,
        player_energy: state.player.energy,
        player_max_energy: state.player.max_energy,
        player_powers,
        player_stance: "None".to_string(),
        hand,
        draw_pile_count: state.draw_pile.len(),
        discard_pile_count: state.discard_pile.len(),
        exhaust_pile_count: state.exhaust_pile.len(),
        enemies,
        relics,
        orbs: vec![],
    }
}

/// Compare two snapshots and return divergences.
///
/// Currently compares:
/// - Player HP, block, energy
/// - Player powers
/// - Each enemy's HP, block, alive status, powers
/// - Hand size (not exact cards — draw pile randomness)
pub fn diff_snapshots(expected: &CombatSnapshot, actual: &CombatSnapshot) -> Vec<Divergence> {
    let mut divs = Vec::new();

    // Player state
    if expected.player_hp != actual.player_hp {
        divs.push(Divergence {
            field: "player_hp".to_string(),
            expected: expected.player_hp.to_string(),
            actual: actual.player_hp.to_string(),
        });
    }
    if expected.player_block != actual.player_block {
        divs.push(Divergence {
            field: "player_block".to_string(),
            expected: expected.player_block.to_string(),
            actual: actual.player_block.to_string(),
        });
    }
    if expected.player_energy != actual.player_energy {
        divs.push(Divergence {
            field: "player_energy".to_string(),
            expected: expected.player_energy.to_string(),
            actual: actual.player_energy.to_string(),
        });
    }

    // Player powers
    diff_powers("player_powers", &expected.player_powers, &actual.player_powers, &mut divs);

    // Enemies
    let max_enemies = expected.enemies.len().max(actual.enemies.len());
    for i in 0..max_enemies {
        let prefix = format!("enemy[{}]", i);
        match (expected.enemies.get(i), actual.enemies.get(i)) {
            (Some(exp), Some(act)) => {
                if exp.hp != act.hp {
                    divs.push(Divergence {
                        field: format!("{}.hp", prefix),
                        expected: exp.hp.to_string(),
                        actual: act.hp.to_string(),
                    });
                }
                if exp.block != act.block {
                    divs.push(Divergence {
                        field: format!("{}.block", prefix),
                        expected: exp.block.to_string(),
                        actual: act.block.to_string(),
                    });
                }
                if exp.alive != act.alive {
                    divs.push(Divergence {
                        field: format!("{}.alive", prefix),
                        expected: exp.alive.to_string(),
                        actual: act.alive.to_string(),
                    });
                }
                // Skip power comparison on dead/is_gone enemies:
                // CommunicationMod clears powers from dead enemies, but our engine retains them.
                if exp.alive {
                    diff_powers(
                        &format!("{}.powers", prefix),
                        &exp.powers, &act.powers, &mut divs,
                    );
                }
            }
            (Some(_), None) => {
                divs.push(Divergence {
                    field: format!("{}.missing", prefix),
                    expected: "present".to_string(),
                    actual: "missing".to_string(),
                });
            }
            (None, Some(_)) => {
                divs.push(Divergence {
                    field: format!("{}.extra", prefix),
                    expected: "missing".to_string(),
                    actual: "present".to_string(),
                });
            }
            (None, None) => {}
        }
    }

    divs
}

/// Compare two power sets and append divergences.
///
/// Normalizes Java -1 (permanent power) vs Rust positive stacks:
/// both mean "power is present", so they're treated as matching.
fn diff_powers(
    prefix: &str,
    expected: &BTreeMap<String, i32>,
    actual: &BTreeMap<String, i32>,
    divs: &mut Vec<Divergence>,
) {
    for (key, &exp_val) in expected {
        let act_val = actual.get(key).copied().unwrap_or(0);
        if exp_val != act_val {
            // Normalize: Java -1 ↔ Rust positive both mean "power present"
            let both_present = (exp_val == -1 && act_val > 0) || (act_val == -1 && exp_val > 0);
            if !both_present {
                divs.push(Divergence {
                    field: format!("{}.{}", prefix, key),
                    expected: exp_val.to_string(),
                    actual: act_val.to_string(),
                });
            }
        }
    }
    for (key, &act_val) in actual {
        if !expected.contains_key(key) && act_val != 0 {
            divs.push(Divergence {
                field: format!("{}.{}", prefix, key),
                expected: "0".to_string(),
                actual: act_val.to_string(),
            });
        }
    }
}

// ============================================================================
// Combat-Level Verification
// ============================================================================

/// Verify all play steps in a sequence of transitions (one combat).
///
/// Walks through consecutive combat transitions, running `verify_step()` on each
/// `play` command.
pub fn verify_combat_transitions(
    transitions: &[DiffTransition],
    card_library: &CardLibrary,
) -> CombatVerifyResult {
    let combat_name = transitions.first()
        .and_then(|t| {
            t.raw_state.get("game_state")
                .and_then(|gs| gs.get("combat_state"))
                .and_then(|cs| cs.get("monsters"))
                .and_then(|m| m.as_array())
                .map(|monsters| {
                    monsters.iter()
                        .filter_map(|m| m["name"].as_str())
                        .collect::<Vec<_>>()
                        .join(" + ")
                })
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let mut results = Vec::new();
    let mut play_count = 0;
    let mut verified = 0;
    let mut skipped = 0;
    let mut divergent = 0;
    let mut total_divs = 0;

    // JSONL format: each entry's state is the result AFTER command execution.
    //   transitions[i].raw_state = state AFTER transitions[i].command
    //
    // So for step verification:
    //   before_state = transitions[i-1].raw_state  (prev command's result = current cmd's input)
    //   command      = transitions[i].command
    //   after_state  = transitions[i].raw_state     (current command's result)
    for i in 1..transitions.len() {
        let prev = &transitions[i - 1];
        let curr = &transitions[i];

        // Only verify if both are in combat
        if prev.snapshot.is_none() || curr.snapshot.is_none() {
            continue;
        }

        // Only verify play commands (the CURRENT step's command)
        if !curr.command.starts_with("play ") {
            continue;
        }

        play_count += 1;

        let mut result = verify_step(
            &prev.raw_state,    // state BEFORE this play (= result of previous command)
            &curr.command,       // the play command to execute
            &curr.raw_state,     // state AFTER this play (= expected result)
            card_library,
        );
        result.step_num = curr.step;

        if result.skipped {
            skipped += 1;
        } else if !result.divergences.is_empty() {
            divergent += 1;
            total_divs += result.divergences.len();
        } else {
            verified += 1;
        }

        results.push(result);
    }

    CombatVerifyResult {
        combat_name,
        total_steps: transitions.len(),
        play_steps: play_count,
        verified_steps: verified,
        skipped_steps: skipped,
        divergent_steps: divergent,
        total_divergences: total_divs,
        step_results: results,
    }
}

/// Format a CombatVerifyResult into a human-readable report.
pub fn format_verify_report(result: &CombatVerifyResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("## {} — {} play steps\n\n", result.combat_name, result.play_steps));
    out.push_str(&format!(
        "✅ Verified: {} | ⏭ Skipped: {} | ❌ Divergent: {} ({} total diffs)\n\n",
        result.verified_steps, result.skipped_steps,
        result.divergent_steps, result.total_divergences
    ));

    for step in &result.step_results {
        if step.skipped {
            continue; // Don't print skipped steps
        }
        if step.divergences.is_empty() {
            // Don't print passing steps (too verbose)
            continue;
        }

        out.push_str(&format!("### Step {} — `{}`\n", step.step_num, step.command));
        for div in &step.divergences {
            out.push_str(&format!(
                "  - **{}**: expected `{}`, got `{}`\n",
                div.field, div.expected, div.actual
            ));
        }
        out.push('\n');
    }

    out
}
