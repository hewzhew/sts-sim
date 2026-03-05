//! Full-turn integration test: Verify end-of-turn cycle against JSONL data.
//!
//! For each `end` command in the JSONL log:
//! 1. Hydrate from the state of the last play command (= before end-of-turn)
//! 2. Execute: on_turn_end → execute_enemy_turn → start_turn + on_turn_start
//! 3. Compare against the `end` command's state (= start of next turn)
//!
//! This validates the full turn cycle: power decay, enemy actions, new turn setup.

use std::fs;
use sts_sim::loader::{CardLibrary, MonsterLibrary};
use sts_sim::testing::commod_parser::{parse_diff_log, parse_combat_snapshot, DiffTransition};
use sts_sim::testing::hydrator::hydrate_combat_state;
use sts_sim::testing::step_verifier::{snapshot_from_game_state, diff_snapshots, Divergence};
use sts_sim::testing::replay::extract_combat_segments;

/// Load the card library from the project's data directory.
fn load_card_library() -> CardLibrary {
    let paths = ["data/cards_patched.json", "data/cards.json"];
    for path in &paths {
        if let Ok(lib) = CardLibrary::load(path) {
            return lib;
        }
    }
    panic!("Cannot find card library");
}

/// Load the monster library.
fn load_monster_library() -> MonsterLibrary {
    MonsterLibrary::load("data/monsters_with_behavior.json")
        .expect("Cannot load monster library")
}

/// Verify a single end-turn step.
///
/// Hydrates the before state, runs on_turn_end → execute_enemy_turn → start_turn,
/// and compares the result against the expected after-state.
fn verify_end_turn(
    before_json: &serde_json::Value,
    after_json: &serde_json::Value,
    card_library: &CardLibrary,
    monster_library: &MonsterLibrary,
) -> (Vec<Divergence>, Vec<String>) {
    let mut notes = Vec::new();
    
    // Hydrate before-state
    let mut state = match hydrate_combat_state(before_json) {
        Some(s) => s,
        None => {
            notes.push("Cannot hydrate before-state".to_string());
            return (vec![], notes);
        }
    };
    
    // Run the full end-of-turn cycle
    // 1. on_turn_end: power hooks (Metallicize, debuff decay, etc.), Flex LoseBuff, hand discard
    sts_sim::engine::combat::on_turn_end(&mut state, card_library, None);
    
    // 2. execute_enemy_turn: enemy intents + enemy debuff decay + plan_enemy_moves
    sts_sim::engine::combat::execute_enemy_turn(&mut state, monster_library);
    
    // 3. start_turn: energy reset, block decay, draw cards, on_turn_start hooks
    state.start_turn();
    sts_sim::engine::combat::on_turn_start(&mut state, card_library, None);
    sts_sim::engine::combat::on_turn_start_post_draw(&mut state, card_library);
    
    // Parse expected after-state
    let expected = match parse_combat_snapshot(after_json) {
        Some(s) => s,
        None => {
            notes.push("Cannot parse after-state".to_string());
            return (vec![], notes);
        }
    };
    
    // Build actual snapshot
    let actual = snapshot_from_game_state(&state);
    
    // Diff — but filter out known RNG-dependent fields
    let all_divs = diff_snapshots(&expected, &actual);
    
    // Filter: only keep deterministic divergences
    // Skip: hand (RNG), draw_pile_count (RNG), discard_pile_count (RNG)
    let filtered: Vec<Divergence> = all_divs.into_iter()
        .filter(|d| {
            // Skip hand-related diffs (draw pile shuffled by RNG)
            !d.field.starts_with("hand") &&
            !d.field.contains("pile_count") &&
            // Skip enemy intent diffs (RNG-dependent)
            !d.field.contains("current_move") &&
            !d.field.contains("intent")
        })
        .collect();
    
    (filtered, notes)
}

/// Test: Verify all end-turn transitions in Floor 51.
#[test]
fn test_full_turn_verify_floor51() {
    let path = "tests/fixtures/real_game_floor51.jsonl";
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Skipping: {} not found", path);
            return;
        }
    };

    let card_library = load_card_library();
    let monster_library = load_monster_library();
    let transitions = parse_diff_log(&content);
    let segments = extract_combat_segments(&transitions);

    let mut total_end_steps = 0;
    let mut total_verified = 0;
    let mut total_divergent = 0;
    let mut total_divs = 0;

    for segment in &segments {
        let trans = &segment.transitions;
        let combat_name: String = trans.first()
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

        // Find end-turn boundaries: where command is "end" and previous has combat state
        for i in 1..trans.len() {
            if !trans[i].command.starts_with("end") {
                continue;
            }
            if trans[i].snapshot.is_none() {
                continue;
            }
            
            // Find the last play/command before this "end" that has combat state
            let before_idx = i - 1;
            if trans[before_idx].snapshot.is_none() {
                continue;
            }

            total_end_steps += 1;

            let (divs, notes) = verify_end_turn(
                &trans[before_idx].raw_state,
                &trans[i].raw_state,
                &card_library,
                &monster_library,
            );

            if !notes.is_empty() {
                eprintln!("⚠️  Step {} ({}): {:?}", trans[i].step, combat_name, notes);
                continue;
            }

            if divs.is_empty() {
                total_verified += 1;
                eprintln!("✅ Step {} ({}): end-turn verified", trans[i].step, combat_name);
            } else {
                total_divergent += 1;
                total_divs += divs.len();
                eprintln!("### Step {} — `end` ({})", trans[i].step, combat_name);
                for d in &divs {
                    eprintln!("- **{}**: expected `{}`, got `{}`", d.field, d.expected, d.actual);
                }
            }
        }
    }

    eprintln!("\n=== FULL-TURN TOTALS ===");
    eprintln!("End steps: {} | Verified: {} | Divergent: {} ({} diffs)",
        total_end_steps, total_verified, total_divergent, total_divs);
}
