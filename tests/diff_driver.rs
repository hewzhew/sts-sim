//! Differential Verification Driver
//!
//! Replays Java CommunicationMod combat logs through the Rust engine,
//! comparing Rust output against Java snapshots after every action.
//!
//! Usage: cargo test diff_driver -- --nocapture

use serde_json::Value;
use sts_simulator::diff::protocol::parser::parse_replay;
use sts_simulator::diff::replay::comparator::compare_states;
use sts_simulator::diff::replay::replay_support::{
    continue_deferred_pending_choice, tick_until_stable,
};
use sts_simulator::diff::state_sync::{build_combat_state, sync_state};
use sts_simulator::state::core::{ClientInput, EngineState};

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_diff_all_combats() {
    let jsonl_path = concat!(env!("CARGO_MANIFEST_DIR"), "/tools/replay_short.jsonl");
    let replay = parse_replay(jsonl_path);
    let combats = &replay.combats;

    println!("\n=== Differential Verification ===");
    println!("Loaded {} combats from {}", combats.len(), jsonl_path);

    let mut total_actions = 0;
    let mut total_pass = 0;
    let mut total_fail = 0;
    let mut total_skip = 0;

    for combat in combats {
        println!(
            "\n--- Combat #{} Floor {} vs [{}] ({} actions) ---",
            combat.combat_idx,
            combat.floor,
            combat.monster_names.join(", "),
            combat.actions.len()
        );

        // Build initial state from combat_start snapshot
        let mut cs = build_combat_state(&combat.start_snapshot, &combat.relics_val);
        let mut es;
        let mut carried_pending = None;

        let mut prev_snapshot = combat.start_snapshot.clone();

        for (action_idx, action) in combat.actions.iter().enumerate() {
            total_actions += 1;

            // Sync state from Java's PREVIOUS snapshot (what state was before this action)
            sync_state(&mut cs, &prev_snapshot);
            es = EngineState::CombatPlayerTurn;

            if action.action_type == "potion" {
                if let Some(pending) = carried_pending.take() {
                    let _ = continue_deferred_pending_choice(&pending, &mut cs, &action.result);
                }
            }

            // Sync events (e.g. Gambling Chip choose/confirm): just update snapshot, no execution
            if action.action_type == "sync" {
                prev_snapshot = action.result.clone();
                continue;
            }

            // Track whether this is an end_turn (for relaxed comparison: skip pile sizes)
            let is_end_turn = action.action_type == "end_turn";

            // Build ClientInput
            let input = match action.action_type.as_str() {
                "play" => {
                    let card_idx = action.card_index.unwrap_or(0);
                    // Target: Java monster index → Rust entity ID (1-indexed)
                    let target = action.target.map(|t| t + 1);
                    ClientInput::PlayCard {
                        card_index: card_idx,
                        target,
                    }
                }
                "end_turn" => ClientInput::EndTurn,
                "potion" => {
                    // Parse command: "potion use <slot>" or "potion use <slot> <target>"
                    let cmd = action.command.as_deref().unwrap_or("");
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    if parts.len() >= 3 && parts[0] == "potion" && parts[1] == "use" {
                        let slot = parts[2].parse::<usize>().unwrap_or(0);
                        // Skip if potion slot is empty (CommunicationMod doesn't output potion data yet)
                        if slot >= cs.entities.potions.len() || cs.entities.potions[slot].is_none()
                        {
                            total_skip += 1;
                            prev_snapshot = action.result.clone();
                            continue;
                        }
                        let target = if parts.len() >= 4 {
                            parts[3].parse::<usize>().ok().map(|t| t + 1) // Java 0-indexed → Rust 1-indexed
                        } else {
                            None
                        };
                        ClientInput::UsePotion {
                            potion_index: slot,
                            target,
                        }
                    } else {
                        total_skip += 1;
                        prev_snapshot = action.result.clone();
                        continue;
                    }
                }
                _ => {
                    total_skip += 1;
                    prev_snapshot = action.result.clone();
                    continue;
                }
            };

            // Identify card name for logging
            let card_name = if action.action_type == "play" {
                let idx = action.card_index.unwrap_or(0);
                if idx < cs.zones.hand.len() {
                    let def =
                        sts_simulator::content::cards::get_card_definition(cs.zones.hand[idx].id);
                    def.name.to_string()
                } else {
                    "???".to_string()
                }
            } else if action.action_type == "potion" {
                format!("POTION({})", action.command.as_deref().unwrap_or("?"))
            } else {
                "END_TURN".to_string()
            };

            // Execute action
            let alive = tick_until_stable(&mut es, &mut cs, input);
            carried_pending = match &es {
                EngineState::PendingChoice(choice) => Some(choice.clone()),
                _ => None,
            };

            // Compare with Java result (skip pile sizes for end_turn since RNG not synced)
            let mut context = sts_simulator::diff::replay::comparator::ActionContext {
                last_command: card_name.clone(),
                was_end_turn: is_end_turn,
                has_rng_state: action.result.get("rng_state").is_some(),
                ..Default::default()
            };
            if let Some(monsters) = action.result.get("monsters").and_then(|m| m.as_array()) {
                context.monster_intents = monsters
                    .iter()
                    .map(|m| m["intent"].as_str().unwrap_or("?").to_string())
                    .collect();
                context.monster_names = monsters
                    .iter()
                    .map(|m| m["id"].as_str().unwrap_or("?").to_string())
                    .collect();
            }
            let diffs = compare_states(&cs, &action.result, is_end_turn, &context);

            if diffs.is_empty() {
                total_pass += 1;
            } else {
                total_fail += 1;
                let target_str = action
                    .target
                    .map(|t| format!(" ->target[{}]", t))
                    .unwrap_or_default();
                println!(
                    "  [{}] {} #{}{}: FAIL {} divergence(s)",
                    action.action_type.to_uppercase(),
                    card_name,
                    action_idx + 1,
                    target_str,
                    diffs.len()
                );
                for d in &diffs {
                    println!("    {} : Rust={}, Java={}", d.field, d.rust_val, d.java_val);
                }
                // RNG comparison on divergence
                if let Some(rng_after) = action.result.get("rng_state") {
                    if let Some(ai) = rng_after.get("ai_rng") {
                        println!("    RNG ai_rng: Rust(seed0={}, seed1={}, counter={}) vs Java(seed0={}, seed1={}, counter={})",
                            cs.rng.ai_rng.seed0 as i64, cs.rng.ai_rng.seed1 as i64, cs.rng.ai_rng.counter,
                            ai["seed0"].as_i64().unwrap_or(0), ai["seed1"].as_i64().unwrap_or(0), ai["counter"].as_u64().unwrap_or(0));
                    }
                }
                // EARLY STOP: break on first divergence for focused debugging
                println!("  !! EARLY STOP: first divergence found, halting.");
                break;
            }

            if !alive {
                println!("  [DEATH] Player died or combat ended");
            }

            prev_snapshot = action.result.clone();
        }
    }

    println!("\n{}", "=".repeat(60));
    println!(
        "SUMMARY: {} actions, {} pass, {} fail, {} skip",
        total_actions, total_pass, total_fail, total_skip
    );

    if total_fail > 0 {
        println!("!!  {} divergences found!", total_fail);
    } else {
        println!("OK  All actions verified successfully!");
    }
}

/// Test with the new RNG-enabled replay data
#[test]
fn test_diff_rng_replay() {
    let jsonl_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl"
    );
    if !std::path::Path::new(jsonl_path).exists() {
        println!("Skipping: {} not found", jsonl_path);
        return;
    }
    let replay = parse_replay(jsonl_path);
    let combats = &replay.combats;

    println!("\n=== RNG-Synced Differential Verification ===");
    println!(
        "Loaded {} combats from {} (protocol v{})",
        combats.len(),
        jsonl_path,
        replay.format_version
    );

    let mut total_actions = 0;
    let mut total_pass = 0;
    let mut total_fail = 0;
    let mut total_skip = 0;
    let mut early_stopped = false;

    for combat in combats {
        if early_stopped {
            break;
        }
        println!(
            "\n--- Combat #{} Floor {} vs [{}] ({} actions) ---",
            combat.combat_idx,
            combat.floor,
            combat.monster_names.join(", "),
            combat.actions.len()
        );

        let mut cs = build_combat_state(&combat.start_snapshot, &combat.relics_val);
        let mut es;
        let mut prev_snapshot = combat.start_snapshot.clone();
        let mut carried_pending = None;

        for (action_idx, action) in combat.actions.iter().enumerate() {
            total_actions += 1;

            sync_state(&mut cs, &prev_snapshot);
            es = EngineState::CombatPlayerTurn;

            if action.action_type == "potion" {
                if let Some(pending) = carried_pending.take() {
                    if let Err(err) =
                        continue_deferred_pending_choice(&pending, &mut cs, &action.result)
                    {
                        println!("  [REPLAY PENDING] {}", err);
                    }
                }
            }

            // Sync events (e.g. Gambling Chip choose/confirm): just update snapshot, no execution
            if action.action_type == "sync" {
                prev_snapshot = action.result.clone();
                continue;
            }

            let is_end_turn = action.action_type == "end_turn";

            let input = match action.action_type.as_str() {
                "play" => {
                    let card_idx = action.card_index.unwrap_or(0);
                    // Map Java monster index to Rust entity_id by looking up cs.monsters
                    // Can't use t+1 because after SpawnMonster entity IDs don't match list indices
                    let target = action.target.map(|t| cs.entities.monsters[t as usize].id);
                    ClientInput::PlayCard {
                        card_index: card_idx,
                        target,
                    }
                }
                "end_turn" => ClientInput::EndTurn,
                "potion" => {
                    let cmd = action.command.as_deref().unwrap_or("");
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    if parts.len() >= 3 && parts[0] == "potion" && parts[1] == "use" {
                        let slot = parts[2].parse::<usize>().unwrap_or(0);
                        if slot >= cs.entities.potions.len() || cs.entities.potions[slot].is_none()
                        {
                            total_skip += 1;
                            prev_snapshot = action.result.clone();
                            continue;
                        }
                        let target = if parts.len() >= 4 {
                            parts[3].parse::<usize>().ok().map(|t| t + 1)
                        } else {
                            None
                        };
                        ClientInput::UsePotion {
                            potion_index: slot,
                            target,
                        }
                    } else {
                        total_skip += 1;
                        prev_snapshot = action.result.clone();
                        continue;
                    }
                }
                _ => {
                    total_skip += 1;
                    prev_snapshot = action.result.clone();
                    continue;
                }
            };

            let card_name = if action.action_type == "play" {
                let idx = action.card_index.unwrap_or(0);
                if idx < cs.zones.hand.len() {
                    let def =
                        sts_simulator::content::cards::get_card_definition(cs.zones.hand[idx].id);
                    def.name.to_string()
                } else {
                    "???".to_string()
                }
            } else if action.action_type == "potion" {
                format!("POTION({})", action.command.as_deref().unwrap_or("?"))
            } else {
                "END_TURN".to_string()
            };

            let alive = tick_until_stable(&mut es, &mut cs, input);
            carried_pending = match &es {
                EngineState::PendingChoice(choice) => Some(choice.clone()),
                _ => None,
            };
            let mut context = sts_simulator::diff::replay::comparator::ActionContext {
                last_command: card_name.clone(),
                was_end_turn: is_end_turn,
                has_rng_state: action.result.get("rng_state").is_some(),
                ..Default::default()
            };
            if let Some(monsters) = action.result.get("monsters").and_then(|m| m.as_array()) {
                context.monster_intents = monsters
                    .iter()
                    .map(|m| m["intent"].as_str().unwrap_or("?").to_string())
                    .collect();
                context.monster_names = monsters
                    .iter()
                    .map(|m| m["id"].as_str().unwrap_or("?").to_string())
                    .collect();
            }
            let diffs = compare_states(&cs, &action.result, is_end_turn, &context);

            if diffs.is_empty() {
                total_pass += 1;
            } else {
                total_fail += 1;
                let target_str = action
                    .target
                    .map(|t| format!(" ->target[{}]", t))
                    .unwrap_or_default();
                println!(
                    "  [{}] {} #{}{}: FAIL {} divergence(s)",
                    action.action_type.to_uppercase(),
                    card_name,
                    action_idx + 1,
                    target_str,
                    diffs.len()
                );
                for d in &diffs {
                    println!("    {} : Rust={}, Java={}", d.field, d.rust_val, d.java_val);
                }
                if let Some(rng_after) = action.result.get("rng_state") {
                    if let Some(ai) = rng_after.get("ai_rng") {
                        println!("    RNG ai_rng: Rust(seed0={}, seed1={}, counter={}) vs Java(seed0={}, seed1={}, counter={})",
                            cs.rng.ai_rng.seed0 as i64, cs.rng.ai_rng.seed1 as i64, cs.rng.ai_rng.counter,
                            ai["seed0"].as_i64().unwrap_or(0), ai["seed1"].as_i64().unwrap_or(0), ai["counter"].as_u64().unwrap_or(0));
                    }
                }
                println!("  !! EARLY STOP: first divergence found, halting.");
                early_stopped = true;
                break;
            }

            if !alive {
                println!("  [DEATH] Player died or combat ended");
            }

            prev_snapshot = action.result.clone();
        }
    }

    println!("\n{}", "=".repeat(60));
    println!(
        "SUMMARY: {} actions, {} pass, {} fail, {} skip",
        total_actions, total_pass, total_fail, total_skip
    );

    if total_fail > 0 {
        println!("!!  {} divergences found!", total_fail);
    } else {
        println!("OK  All actions verified successfully!");
    }
}

/// Test ALL v2 replay files in the replays directory
#[test]
fn test_diff_all_v2_replays() {
    let replays_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tools/replays");
    let mut total_files = 0;
    let mut total_pass_files = 0;
    let mut grand_total_actions = 0;
    let mut grand_total_pass = 0;
    let mut grand_total_fail = 0;
    let mut grand_total_skip = 0;
    /// Recent action log entry for context printing on failure
    struct ActionLog {
        combat_idx: usize,
        action_idx: usize,
        action_type: String,
        card_name: String,
        target: Option<usize>,
    }

    let mut entries: Vec<_> = std::fs::read_dir(replays_dir)
        .expect("Failed to read replays directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "jsonl"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    // Count v2 files silently
    let v2_count = entries
        .iter()
        .filter(|e| {
            let path = e.path();
            let replay = parse_replay(path.to_str().unwrap());
            replay.format_version >= 2 && replay.capabilities.contains(&"rng_state".to_string())
        })
        .count();
    println!(
        "\n=== Differential Verification: {} v2 replays ===\n",
        v2_count
    );

    for entry in &entries {
        let path = entry.path();
        let fname = path.file_name().unwrap().to_str().unwrap();
        let replay = parse_replay(path.to_str().unwrap());

        // Skip non-v2 replays silently
        if replay.format_version < 2 || !replay.capabilities.contains(&"rng_state".to_string()) {
            continue;
        }

        total_files += 1;
        let combats = &replay.combats;

        let mut file_actions = 0;
        let mut file_pass = 0;
        let mut file_fail = 0;
        let mut file_skip = 0;
        let mut early_stopped = false;
        // Buffer last N actions for context on failure
        const CONTEXT_SIZE: usize = 5;
        let mut recent_actions: Vec<ActionLog> = Vec::new();

        for combat in combats {
            if early_stopped {
                break;
            }

            let relics_val = &Value::Null;
            let mut cs = build_combat_state(&combat.start_snapshot, relics_val);
            let mut es;
            let mut prev_snapshot = combat.start_snapshot.clone();
            let mut carried_pending = None;

            for (action_idx, action) in combat.actions.iter().enumerate() {
                file_actions += 1;

                sync_state(&mut cs, &prev_snapshot);
                es = EngineState::CombatPlayerTurn;

                if action.action_type == "potion" {
                    if let Some(pending) = carried_pending.take() {
                        let _ = continue_deferred_pending_choice(&pending, &mut cs, &action.result);
                    }
                }

                let is_end_turn = action.action_type == "end_turn";

                let input = match action.action_type.as_str() {
                    "play" => {
                        let card_idx = action.card_index.unwrap_or(0);
                        let target = action.target.map(|t| cs.entities.monsters[t as usize].id);
                        ClientInput::PlayCard {
                            card_index: card_idx,
                            target,
                        }
                    }
                    "end_turn" => ClientInput::EndTurn,
                    "potion" => {
                        let cmd = action.command.as_deref().unwrap_or("");
                        let parts: Vec<&str> = cmd.split_whitespace().collect();
                        if parts.len() >= 3 && parts[0] == "potion" && parts[1] == "use" {
                            let slot = parts[2].parse::<usize>().unwrap_or(0);
                            if slot >= cs.entities.potions.len()
                                || cs.entities.potions[slot].is_none()
                            {
                                file_skip += 1;
                                prev_snapshot = action.result.clone();
                                continue;
                            }
                            let target = if parts.len() >= 4 {
                                parts[3].parse::<usize>().ok().map(|t| t + 1)
                            } else {
                                None
                            };
                            ClientInput::UsePotion {
                                potion_index: slot,
                                target,
                            }
                        } else {
                            file_skip += 1;
                            prev_snapshot = action.result.clone();
                            continue;
                        }
                    }
                    _ => {
                        file_skip += 1;
                        prev_snapshot = action.result.clone();
                        continue;
                    }
                };

                let card_name = if action.action_type == "play" {
                    let idx = action.card_index.unwrap_or(0);
                    if idx < cs.zones.hand.len() {
                        let def = sts_simulator::content::cards::get_card_definition(
                            cs.zones.hand[idx].id,
                        );
                        def.name.to_string()
                    } else {
                        "???".to_string()
                    }
                } else if action.action_type == "potion" {
                    format!("POTION({})", action.command.as_deref().unwrap_or("?"))
                } else {
                    "END_TURN".to_string()
                };

                let _alive = tick_until_stable(&mut es, &mut cs, input);
                carried_pending = match &es {
                    EngineState::PendingChoice(choice) => Some(choice.clone()),
                    _ => None,
                };
                let mut context = sts_simulator::diff::replay::comparator::ActionContext {
                    last_command: card_name.clone(),
                    was_end_turn: is_end_turn,
                    has_rng_state: action.result.get("rng_state").is_some(),
                    ..Default::default()
                };
                if let Some(monsters) = action.result.get("monsters").and_then(|m| m.as_array()) {
                    context.monster_intents = monsters
                        .iter()
                        .map(|m| m["intent"].as_str().unwrap_or("?").to_string())
                        .collect();
                    context.monster_names = monsters
                        .iter()
                        .map(|m| m["id"].as_str().unwrap_or("?").to_string())
                        .collect();
                }
                let diffs = compare_states(&cs, &action.result, is_end_turn, &context);

                if diffs.is_empty() {
                    file_pass += 1;
                    // Buffer for context — keep last CONTEXT_SIZE
                    recent_actions.push(ActionLog {
                        combat_idx: combat.combat_idx,
                        action_idx: action_idx + 1,
                        action_type: action.action_type.clone(),
                        card_name: card_name.clone(),
                        target: action.target,
                    });
                    if recent_actions.len() > CONTEXT_SIZE {
                        recent_actions.remove(0);
                    }
                } else {
                    file_fail += 1;
                    // Print file header + context + failure
                    println!("--- {} ({} combats) ---", fname, combats.len());
                    if !recent_actions.is_empty() {
                        println!(
                            "  ... {} preceding actions OK, last {}:",
                            file_pass,
                            recent_actions.len()
                        );
                        for log in &recent_actions {
                            let t = log
                                .target
                                .map(|t| format!(" ->target[{}]", t))
                                .unwrap_or_default();
                            println!(
                                "    [OK] Combat #{} [{}] {} #{}{}",
                                log.combat_idx,
                                log.action_type.to_uppercase(),
                                log.card_name,
                                log.action_idx,
                                t
                            );
                        }
                    }
                    let target_str = action
                        .target
                        .map(|t| format!(" ->target[{}]", t))
                        .unwrap_or_default();
                    println!(
                        "  >> FAIL Combat #{} [{}] {} #{}{}",
                        combat.combat_idx,
                        action.action_type.to_uppercase(),
                        card_name,
                        action_idx + 1,
                        target_str
                    );
                    for d in &diffs {
                        println!(
                            "       {} : Rust={}, Java={}",
                            d.field, d.rust_val, d.java_val
                        );
                    }
                    early_stopped = true;
                    break;
                }

                prev_snapshot = action.result.clone();
            }
        }

        grand_total_actions += file_actions;
        grand_total_pass += file_pass;
        grand_total_fail += file_fail;
        grand_total_skip += file_skip;

        if file_fail == 0 {
            total_pass_files += 1;
            // One-line pass summary
            println!("  OK  {} — {}/{} pass", fname, file_pass, file_actions);
        }
    }

    println!("\n{}", "=".repeat(60));
    println!(
        "GRAND TOTAL: {}/{} files pass, {} actions ({} pass, {} fail, {} skip)",
        total_pass_files,
        total_files,
        grand_total_actions,
        grand_total_pass,
        grand_total_fail,
        grand_total_skip
    );

    if grand_total_fail > 0 {
        println!("!!  {} divergences found!", grand_total_fail);
    } else {
        println!("OK  All replays verified!");
    }
}
