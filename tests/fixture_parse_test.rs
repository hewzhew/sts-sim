//! Integration test: parse all 139 bottled_ai battle fixtures through commod_parser.

use std::fs;
use sts_sim::testing::commod_parser::{parse_diff_log, parse_combat_snapshot};

#[test]
fn test_parse_all_battle_fixtures() {
    let fixture_path = "tests/fixtures/battle_fixtures.jsonl";
    let content = match fs::read_to_string(fixture_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping fixture test: {} not found. Run collect_fixtures.py first.", fixture_path);
            return;
        }
    };

    let transitions = parse_diff_log(&content);
    assert!(transitions.len() > 100, "Expected 100+ fixtures, got {}", transitions.len());

    let mut parsed_count = 0;
    let mut failed_count = 0;
    let mut monster_set = std::collections::HashSet::new();
    let mut card_set = std::collections::HashSet::new();
    let mut power_set = std::collections::HashSet::new();

    for t in &transitions {
        if let Some(snap) = &t.snapshot {
            parsed_count += 1;

            // Basic sanity checks
            assert!(snap.player_hp > 0, "Step {}: Player HP should be > 0, got {}", t.step, snap.player_hp);
            assert!(snap.player_max_hp > 0, "Step {}: Max HP should be > 0, got {}", t.step, snap.player_max_hp);
            assert!(snap.player_hp <= snap.player_max_hp,
                "Step {}: HP {} > MaxHP {}", t.step, snap.player_hp, snap.player_max_hp);
            assert!(snap.turn >= 1, "Step {}: Turn should be >= 1, got {}", t.step, snap.turn);

            // Collect data for summary
            for e in &snap.enemies {
                monster_set.insert(e.name.clone());
            }
            for c in &snap.hand {
                card_set.insert(c.id.clone());
            }
            for (p, _) in &snap.player_powers {
                power_set.insert(p.clone());
            }
        } else {
            failed_count += 1;
        }
    }

    eprintln!("=== Fixture Parsing Summary ===");
    eprintln!("Total fixtures: {}", transitions.len());
    eprintln!("Successfully parsed: {}", parsed_count);
    eprintln!("Skipped (no combat_state): {}", failed_count);
    eprintln!("Unique monsters seen: {} ({:?})", monster_set.len(), monster_set);
    eprintln!("Unique cards in hands: {}", card_set.len());
    eprintln!("Unique player powers: {} ({:?})", power_set.len(), power_set);

    assert!(parsed_count >= 100, "Expected 100+ parsed snapshots, got {}", parsed_count);
    assert!(monster_set.len() >= 3, "Expected 3+ unique monsters, got {}", monster_set.len());
    assert!(card_set.len() >= 5, "Expected 5+ unique cards, got {}", card_set.len());
}

#[test]
fn test_parse_single_complex_fixture() {
    // Parse the most complex fixture (basic.json which we know the structure of)
    let fixture_path = "tests/fixtures/battle_fixtures.jsonl";
    let content = match fs::read_to_string(fixture_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: fixture file not found");
            return;
        }
    };

    let transitions = parse_diff_log(&content);
    
    // Find a fixture with monsters that have powers
    let with_powers = transitions.iter().find(|t| {
        t.snapshot.as_ref().map(|s| {
            s.enemies.iter().any(|e| !e.powers.is_empty())
        }).unwrap_or(false)
    });

    if let Some(t) = with_powers {
        let snap = t.snapshot.as_ref().unwrap();
        eprintln!("Found fixture with enemy powers in: {}", t.command);
        for e in &snap.enemies {
            if !e.powers.is_empty() {
                eprintln!("  {} has powers: {:?}", e.name, e.powers);
            }
        }
    }

    // Find a fixture with player powers
    let with_player_powers = transitions.iter().find(|t| {
        t.snapshot.as_ref().map(|s| !s.player_powers.is_empty()).unwrap_or(false)
    });

    if let Some(t) = with_player_powers {
        let snap = t.snapshot.as_ref().unwrap();
        eprintln!("Found fixture with player powers in: {}", t.command);
        eprintln!("  Player powers: {:?}", snap.player_powers);
    }
}
