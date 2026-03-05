//! Integration test: run real game JSONL through the diff testing pipeline.

use std::fs;
use sts_sim::testing::commod_parser::parse_diff_log;
use sts_sim::testing::replay::{extract_combat_segments, run_diff_test, format_replay_report};

/// Test Layer 1: Validate that all 488 combat steps from the Floor 51 run
/// parse correctly into CombatSnapshots.
#[test]
fn test_real_game_floor51_parsing() {
    let path = "tests/fixtures/real_game_floor51.jsonl";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found", path);
            return;
        }
    };

    let transitions = parse_diff_log(&content);
    assert!(transitions.len() > 700, "Expected 700+ transitions, got {}", transitions.len());

    // Count how many have combat snapshots
    let with_snap: Vec<_> = transitions.iter().filter(|t| t.snapshot.is_some()).collect();
    let without_snap: Vec<_> = transitions.iter().filter(|t| t.snapshot.is_none()).collect();

    eprintln!("=== Floor 51 Parsing Results ===");
    eprintln!("Total transitions: {}", transitions.len());
    eprintln!("With combat snapshot: {}", with_snap.len());
    eprintln!("Without (non-combat): {}", without_snap.len());

    // Should have ~488 combat steps
    assert!(with_snap.len() > 400, "Expected 400+ combat snapshots, got {}", with_snap.len());

    // Validate each combat snapshot
    let mut total_cards_in_hands = 0;
    let mut total_monsters = 0;
    let mut total_powers = 0;
    let mut max_hand_size = 0;
    let mut unique_cards = std::collections::HashSet::new();
    let mut unique_monsters = std::collections::HashSet::new();
    let mut unique_powers = std::collections::HashSet::new();

    for t in &with_snap {
        let snap = t.snapshot.as_ref().unwrap();

        // Sanity checks
        assert!(snap.player_hp >= 0, "Step {}: HP negative: {}", t.step, snap.player_hp);
        assert!(snap.player_max_hp > 0, "Step {}: MaxHP should be > 0", t.step);
        assert!(snap.player_hp <= snap.player_max_hp + 10,  // some relics give temp HP
            "Step {}: HP {} > MaxHP {} + margin", t.step, snap.player_hp, snap.player_max_hp);
        assert!(snap.player_block >= 0, "Step {}: Block negative: {}", t.step, snap.player_block);
        assert!(snap.player_energy >= 0, "Step {}: Energy negative: {}", t.step, snap.player_energy);
        assert!(snap.turn >= 1, "Step {}: Turn should be >= 1, got {}", t.step, snap.turn);

        // Accumulate stats
        total_cards_in_hands += snap.hand.len();
        if snap.hand.len() > max_hand_size {
            max_hand_size = snap.hand.len();
        }
        for card in &snap.hand {
            unique_cards.insert(card.id.clone());
        }
        for enemy in &snap.enemies {
            unique_monsters.insert(enemy.name.clone());
            total_monsters += 1;
            for (pid, _) in &enemy.powers {
                unique_powers.insert(pid.clone());
            }
        }
        for (pid, _) in &snap.player_powers {
            unique_powers.insert(pid.clone());
        }
        total_powers += snap.player_powers.len();
    }

    eprintln!("\n=== Data Richness ===");
    eprintln!("Total cards seen in hands: {}", total_cards_in_hands);
    eprintln!("Max hand size: {}", max_hand_size);
    eprintln!("Unique card IDs: {} ({:?})", unique_cards.len(), unique_cards);
    eprintln!("Unique monsters: {} ({:?})", unique_monsters.len(), unique_monsters);
    eprintln!("Unique powers: {} ({:?})", unique_powers.len(), unique_powers);

    // Minimum data richness checks
    assert!(unique_cards.len() >= 10, "Expected 10+ unique cards, got {}", unique_cards.len());
    assert!(unique_monsters.len() >= 10, "Expected 10+ unique monsters, got {}", unique_monsters.len());
    assert!(unique_powers.len() >= 10, "Expected 10+ unique powers, got {}", unique_powers.len());
}

/// Test Layer 1: Run the full diff test pipeline (combat segment extraction + report).
#[test]
fn test_real_game_floor51_diff_pipeline() {
    let path = "tests/fixtures/real_game_floor51.jsonl";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found", path);
            return;
        }
    };

    let report = run_diff_test(&content);
    eprintln!("\n{}", report);

    // Report should not be empty
    assert!(!report.is_empty(), "Report should not be empty");
    // Should mention combats
    assert!(report.contains("combat"), "Report should mention combats");
}

/// Test Layer 1: Same for Floor 30 run.
#[test]
fn test_real_game_floor30_parsing() {
    let path = "tests/fixtures/real_game_floor30.jsonl";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found", path);
            return;
        }
    };

    let transitions = parse_diff_log(&content);
    let with_snap = transitions.iter().filter(|t| t.snapshot.is_some()).count();

    eprintln!("=== Floor 30 Parsing Results ===");
    eprintln!("Total transitions: {}", transitions.len());
    eprintln!("With combat snapshot: {}", with_snap);

    assert!(transitions.len() > 400, "Expected 400+ transitions, got {}", transitions.len());
    assert!(with_snap > 300, "Expected 300+ combat snapshots, got {}", with_snap);

    let report = run_diff_test(&content);
    eprintln!("\n{}", report);
    assert!(!report.is_empty());
}

/// Test Layer 1: Same for Act 3 run.
#[test]
fn test_real_game_act3_diff_pipeline() {
    let path = "tests/fixtures/act3.jsonl";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found", path);
            return;
        }
    };

    let report = run_diff_test(&content);
    eprintln!("\n{}", report);
    assert!(!report.is_empty());
}
