//! Integration test: Run real game data through the step verifier.
//!
//! This tests Phase A: state-injection single-step validation.
//! For each "play" command in the JSONL log, we:
//! 1. Hydrate the before-state into a GameState
//! 2. Execute the play command via the real Rust engine
//! 3. Compare the result against the after-state from JSONL

use std::fs;
use sts_sim::loader::CardLibrary;
use sts_sim::testing::commod_parser::parse_diff_log;
use sts_sim::testing::step_verifier::{verify_combat_transitions, format_verify_report};
use sts_sim::testing::replay::extract_combat_segments;

/// Load the card library from the project's data directory.
fn load_card_library() -> CardLibrary {
    let paths = [
        "data/cards_patched.json",
        "data/cards.json",
    ];
    for path in &paths {
        if let Ok(lib) = CardLibrary::load(path) {
            return lib;
        }
    }
    panic!("Cannot find card library in data/cards_patched.json or data/cards.json");
}

/// Test: Run Floor 51 data through step verifier (first combat only).
#[test]
fn test_step_verify_floor51_first_combat() {
    let path = "tests/fixtures/real_game_floor51.jsonl";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found", path);
            return;
        }
    };

    let library = load_card_library();
    let transitions = parse_diff_log(&content);
    let segments = extract_combat_segments(&transitions);

    assert!(!segments.is_empty(), "Should find at least one combat segment");

    // Verify first combat (Jaw Worm, ~12 steps)
    let first_combat = &segments[0];
    let result = verify_combat_transitions(&first_combat.transitions, &library);

    eprintln!("\n{}", format_verify_report(&result));
    eprintln!("Combat: {} | play steps: {} | verified: {} | skipped: {} | divergent: {} ({} diffs)",
        result.combat_name, result.play_steps, result.verified_steps,
        result.skipped_steps, result.divergent_steps, result.total_divergences);

    // We expect SOME divergences in Phase A (bugs to find!)
    // But we should be able to verify at least some steps without panics
    assert!(result.play_steps > 0, "Should find play commands in first combat");
    // No panics = success for Phase A foundation test
}

/// Test: Run Floor 51 data through step verifier (all combats).
#[test]
fn test_step_verify_floor51_all_combats() {
    let path = "tests/fixtures/real_game_floor51.jsonl";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found", path);
            return;
        }
    };

    let library = load_card_library();
    let transitions = parse_diff_log(&content);
    let segments = extract_combat_segments(&transitions);

    let mut total_play = 0;
    let mut total_verified = 0;
    let mut total_skipped = 0;
    let mut total_divergent = 0;
    let mut total_divs = 0;

    for segment in &segments {
        let result = verify_combat_transitions(&segment.transitions, &library);
        total_play += result.play_steps;
        total_verified += result.verified_steps;
        total_skipped += result.skipped_steps;
        total_divergent += result.divergent_steps;
        total_divs += result.total_divergences;

        // Print divergences for each combat
        if result.divergent_steps > 0 {
            eprintln!("\n{}", format_verify_report(&result));
        } else {
            eprintln!("✅ {} — {} play steps, all clean", result.combat_name, result.play_steps);
        }
    }

    eprintln!("\n=== TOTALS ===");
    eprintln!("Combats: {} | Play steps: {} | Verified: {} | Skipped: {} | Divergent: {} ({} diffs)",
        segments.len(), total_play, total_verified, total_skipped, total_divergent, total_divs);

    // Output results to file for analysis
    let mut report = String::new();
    report.push_str("# Step Verifier Report — Floor 51\n\n");
    report.push_str(&format!(
        "**Totals**: {} combats, {} play steps, {} verified, {} skipped, {} divergent ({} total diffs)\n\n",
        segments.len(), total_play, total_verified, total_skipped, total_divergent, total_divs
    ));
    for segment in &segments {
        let result = verify_combat_transitions(&segment.transitions, &library);
        report.push_str(&format_verify_report(&result));
    }
    let _ = fs::write("tests/step_verify_report.md", &report);
}
