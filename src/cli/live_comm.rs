use std::io::{self, BufRead, Write};
use crate::state::core::EngineState;
use crate::combat::CombatState;
use crate::bot::comm_mod;
use crate::diff::mapper::{card_id_from_java, power_id_from_java};
use crate::diff::comparator::{DiffCategory, ActionContext};

const LOG_PATH: &str = r"d:\rust\sts_simulator\live_comm_debug.txt";
const RAW_PATH: &str = r"d:\rust\sts_simulator\live_comm_raw.jsonl";

// ─── Combat Diff Accumulator (for per-combat summary) ────────

struct CombatDiffRecord {
    _frame: u64,
    field: String,
    category: DiffCategory,
    rust_val: String,
    java_val: String,
}

struct CombatStats {
    start_frame: u64,
    action_count: u32,
    diffs: Vec<CombatDiffRecord>,
    /// Tracks which ContentGap fields we've already logged in full
    seen_content_gaps: std::collections::HashSet<String>,
}

impl CombatStats {
    fn new(frame: u64) -> Self {
        Self {
            start_frame: frame,
            action_count: 0,
            diffs: Vec::new(),
            seen_content_gaps: std::collections::HashSet::new(),
        }
    }
    
    fn write_summary(&self, log: &mut std::fs::File, end_frame: u64) {
        let engine_bugs: Vec<_> = self.diffs.iter().filter(|d| d.category == DiffCategory::EngineBug).collect();
        let content_gaps: Vec<_> = self.diffs.iter().filter(|d| d.category == DiffCategory::ContentGap).collect();
        let timing: Vec<_> = self.diffs.iter().filter(|d| d.category == DiffCategory::Timing).collect();
        
        writeln!(log, "\n╔══════════════════════════════════════════════════════╗").unwrap();
        writeln!(log, "║  COMBAT SUMMARY (F{} ~ F{})                          ", self.start_frame, end_frame).unwrap();
        writeln!(log, "╠══════════════════════════════════════════════════════╣").unwrap();
        writeln!(log, "║  Frames: {}  |  Actions: {}", end_frame - self.start_frame + 1, self.action_count).unwrap();
        writeln!(log, "║  ENGINE BUGS:  {}", engine_bugs.len()).unwrap();
        writeln!(log, "║  CONTENT GAPS: {}", content_gaps.len()).unwrap();
        writeln!(log, "║  TIMING:       {}", timing.len()).unwrap();
        
        if !engine_bugs.is_empty() {
            writeln!(log, "║").unwrap();
            writeln!(log, "║  ⛔ Engine Bugs:").unwrap();
            // Deduplicate by field
            let mut seen = std::collections::HashMap::<String, (usize, String, String)>::new();
            for d in &engine_bugs {
                let entry = seen.entry(d.field.clone()).or_insert((0, d.rust_val.clone(), d.java_val.clone()));
                entry.0 += 1;
            }
            for (field, (count, rv, jv)) in &seen {
                writeln!(log, "║    - {} (×{}) Rust={} Java={}", field, count, rv, jv).unwrap();
            }
        }
        
        if !content_gaps.is_empty() {
            writeln!(log, "║").unwrap();
            writeln!(log, "║  ⚠ Content Gaps:").unwrap();
            let mut seen = std::collections::HashMap::<String, usize>::new();
            for d in &content_gaps {
                *seen.entry(d.field.clone()).or_insert(0) += 1;
            }
            for (field, count) in &seen {
                writeln!(log, "║    - {} (×{})", field, count).unwrap();
            }
        }
        
        let verdict = if engine_bugs.is_empty() {
            "✅ Engine OK"
        } else {
            "❌ Engine Bugs Found"
        };
        let extra = if !content_gaps.is_empty() && engine_bugs.is_empty() {
            " (content gaps only)"
        } else {
            ""
        };
        writeln!(log, "║").unwrap();
        writeln!(log, "║  VERDICT: {}{}", verdict, extra).unwrap();
        writeln!(log, "╚══════════════════════════════════════════════════════╝").unwrap();
    }
}

// ─── Main Loop ───────────────────────────────────────────────

pub fn run_live_comm_loop(mut _agent: crate::bot::agent::Agent) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut log = std::fs::File::create(LOG_PATH).unwrap();
    let mut raw = std::fs::File::create(RAW_PATH).unwrap();
    writeln!(log, "=== Rust Live-Comm Started ===").unwrap();

    println!("ready");
    stdout.flush().unwrap();
    writeln!(log, "Sent: ready").unwrap();

    let mut consecutive_errors: u32 = 0;
    let mut last_error_msg = String::new();
    let mut frame_count: u64 = 0;
    let mut last_sent_cmd = String::new();
    let mut cmd_failed_count: u32 = 0;
    
    let mut expected_combat_state: Option<CombatState> = None;
    let mut engine_bug_total: usize = 0;
    
    // Causal context: what happened last frame
    let mut action_context = ActionContext::default();
    
    // Per-combat stats
    let mut combat_stats: Option<CombatStats> = None;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => { writeln!(log, "STDIN ERR: {}", e).unwrap(); break; }
        };
        if line.trim().is_empty() { continue; }
        frame_count += 1;

        // ── Raw JSON dump ──
        writeln!(raw, "{}", line).unwrap();

        let parsed: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => { writeln!(log, "[F{}] JSON ERR: {}", frame_count, e).unwrap(); continue; }
        };

        // ── Error handling with dedup ──
        if let Some(err) = parsed.get("error").and_then(|v| v.as_str()) {
            expected_combat_state = None; // Java rejected the action, so prediction is void
            consecutive_errors += 1;
            cmd_failed_count += 1;
            // Only log first occurrence + summary, avoid flooding
            if err != last_error_msg || consecutive_errors <= 2 {
                writeln!(log, "[F{}] ERROR #{}: {}", frame_count, consecutive_errors, err).unwrap();
                last_error_msg = err.to_string();
            } else if consecutive_errors == 3 {
                writeln!(log, "  (suppressing repeated errors...)").unwrap();
            }
            if consecutive_errors >= 5 {
                writeln!(log, "  ERROR FLOOD: {} repeats, sleeping 1s", consecutive_errors).unwrap();
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            // On error, re-poll state
            println!("STATE");
            stdout.flush().unwrap();
            continue;
        }
        if consecutive_errors > 0 {
            if consecutive_errors > 2 {
                writeln!(log, "  (total {} errors before recovery)", consecutive_errors).unwrap();
            }
            consecutive_errors = 0;
            last_error_msg.clear();
        }

        // ── Parse available commands ──
        let avail: Vec<&str> = parsed.get("available_commands")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        let has = |c: &str| avail.contains(&c);

        let in_game = parsed["in_game"].as_bool().unwrap_or(false);

        if !in_game {
            if has("start") {
                writeln!(log, "[F{}] Not in game → START", frame_count).unwrap();
                println!("START Ironclad 0");
            } else {
                writeln!(log, "[F{}] Not in game → STATE", frame_count).unwrap();
                println!("STATE");
            }
            stdout.flush().unwrap();
            continue;
        }

        // ── In-game ──
        let gs = &parsed["game_state"];
        let screen = gs["screen_type"].as_str().unwrap_or("?");
        let is_combat = gs.get("combat_state").map_or(false, |v| !v.is_null());
        let room_phase = gs["room_phase"].as_str().unwrap_or("");

        // ── GAME OVER ──
        if screen == "GAME_OVER" || screen == "DEATH" {
            // Flush combat summary if we were in combat
            if let Some(stats) = combat_stats.take() {
                stats.write_summary(&mut log, frame_count);
            }
            let score = gs.get("screen_state").and_then(|s| s["score"].as_i64()).unwrap_or(0);
            let victory = gs.get("screen_state").and_then(|s| s["victory"].as_bool()).unwrap_or(false);
            writeln!(log, "\n[F{}] === GAME OVER === victory={} score={}", frame_count, victory, score).unwrap();
            let _ = log.flush();
            let _ = raw.flush();
            std::thread::sleep(std::time::Duration::from_secs(9999));
            break;
        }
        
        // ── Detect combat end → write summary ──
        if !is_combat || screen != "NONE" {
            if let Some(stats) = combat_stats.take() {
                stats.write_summary(&mut log, frame_count.saturating_sub(1));
            }
        }

        // ── COMBAT ──
        if is_combat && screen == "NONE" && (has("play") || has("end")) {
            let cv = &gs["combat_state"];
            let rv = &gs["relics"];
            let truth = crate::diff::state_sync::build_combat_state(cv, rv);
            
            // Initialize combat stats if this is a new combat
            if combat_stats.is_none() {
                combat_stats = Some(CombatStats::new(frame_count));
            }

            // Summary
            writeln!(log, "\n[F{}] COMBAT  HP={}/{}  E={}  Hand={}  Draw={}  Disc={}  Monsters={}",
                frame_count, truth.player.current_hp, truth.player.max_hp,
                truth.energy, truth.hand.len(),
                truth.draw_pile.len(), truth.discard_pile.len(),
                truth.monsters.len()).unwrap();

            for (i, m) in truth.monsters.iter().enumerate() {
                let powers = format_powers(&truth, m.id);
                let dead_str = if m.is_dying || m.is_escaped { " (DEAD)" } else { "" };
                writeln!(log, "  M[{}] id={} hp={}/{} blk={} intent={:?}{}{}",
                    i, m.id, m.current_hp, m.max_hp, m.block, m.current_intent,
                    if powers.is_empty() { String::new() } else { format!(" pw=[{}]", powers) },
                    dead_str
                ).unwrap();
            }

            let hand_str: Vec<String> = truth.hand.iter().enumerate().map(|(i, c)| {
                let def = crate::content::cards::get_card_definition(c.id);
                let u = if c.upgrades > 0 { "+" } else { "" };
                format!("{}:{}{}", i, def.name, u)
            }).collect();
            writeln!(log, "  Hand: [{}]", hand_str.join(", ")).unwrap();

            let pp = format_powers(&truth, 0);
            if !pp.is_empty() { writeln!(log, "  Player pw: [{}]", pp).unwrap(); }

            // Parse-validation diff: compare what state_sync built vs raw Java JSON
            let sync_diffs = validate_parse(&truth, cv);
            if !sync_diffs.is_empty() {
                writeln!(log, "  >>> PARSE DIFF ({}) <<<", sync_diffs.len()).unwrap();
                for d in &sync_diffs {
                    writeln!(log, "    {}", d).unwrap();
                }
            }

            // =========================================================
            // LIVE ACTION PARITY CHECK
            // =========================================================
            if let Some(expected_cs) = expected_combat_state.take() {
                let action_diffs = crate::diff::comparator::compare_states(&expected_cs, cv, action_context.was_end_turn, &action_context);
                
                if !action_diffs.is_empty() {
                    // Classify and count
                    let bugs: Vec<_> = action_diffs.iter().filter(|d| d.category == DiffCategory::EngineBug).collect();
                    let gaps: Vec<_> = action_diffs.iter().filter(|d| d.category == DiffCategory::ContentGap).collect();
                    let timing: Vec<_> = action_diffs.iter().filter(|d| d.category == DiffCategory::Timing).collect();
                    
                    engine_bug_total += bugs.len();
                    
                    writeln!(log, "  >>> PARITY FAIL ({} diffs: {} bugs, {} gaps, {} timing) <<<",
                        action_diffs.len(), bugs.len(), gaps.len(), timing.len()).unwrap();
                    writeln!(log, "  CAUSED BY: {}", action_context.describe()).unwrap();
                    
                    // Log each diff with category
                    let stats = combat_stats.as_mut().unwrap();
                    for d in &action_diffs {
                        // For ContentGap: only log full detail on first occurrence
                        let is_repeat_gap = d.category == DiffCategory::ContentGap 
                            && stats.seen_content_gaps.contains(&d.field);
                        
                        if !is_repeat_gap {
                            writeln!(log, "    {} : Rust={}, Java={}  [{}]", d.field, d.rust_val, d.java_val, d.category).unwrap();
                        }
                        
                        if d.category == DiffCategory::ContentGap {
                            stats.seen_content_gaps.insert(d.field.clone());
                        }
                        
                        // Accumulate for combat summary
                        stats.diffs.push(CombatDiffRecord {
                            _frame: frame_count,
                            field: d.field.clone(),
                            category: d.category,
                            rust_val: d.rust_val.clone(),
                            java_val: d.java_val.clone(),
                        });
                    }
                    
                    // Mention suppressed gaps
                    let _suppressed = action_diffs.iter()
                        .filter(|d| d.category == DiffCategory::ContentGap 
                            && stats.seen_content_gaps.contains(&d.field)
                            && action_diffs.iter().filter(|d2| d2.field == d.field).count() > 0)
                        .count();
                    // (suppression is implicit: they just don't get logged above)
                    
                    // EARLY STOP: only for engine bugs
                    if engine_bug_total >= 5 {
                        writeln!(log, "  !! EARLY STOP: {} engine bugs detected, halting.", engine_bug_total).unwrap();
                        if let Some(stats) = combat_stats.take() {
                            stats.write_summary(&mut log, frame_count);
                        }
                        let _ = log.flush();
                        std::process::exit(1);
                    }
                    
                    // SELF-HEAL: clear prediction so next frame starts fresh from Java truth
                    // (expected_combat_state is already None since we .take()'d it)
                    writeln!(log, "  [HEALED] Prediction chain reset from Java truth").unwrap();
                } else {
                    writeln!(log, "  >>> PARITY OK <<<").unwrap();
                }
            }

            // Decide
            let input = crate::bot::heuristic::decide_heuristic(&truth);
            writeln!(log, "  → {:?}", input).unwrap();
            
            if let Some(stats) = combat_stats.as_mut() {
                stats.action_count += 1;
            }

            let mut engine_state = EngineState::CombatPlayerTurn;
            if let Some(cmd) = comm_mod::input_to_java_command(&input, &engine_state) {
                if cmd == last_sent_cmd && cmd_failed_count > 0 {
                    writeln!(log, "  [!] AVOIDING REPEATED ERROR BY FORCING END TURN").unwrap();
                    println!("END");
                    last_sent_cmd = "END".to_string();
                } else {
                    writeln!(log, "  SEND: {}", cmd).unwrap();
                    println!("{}", cmd);
                    last_sent_cmd = cmd.clone();
                    
                    // Build action context for next frame's causal tracking
                    let is_end_turn = matches!(input, crate::state::core::ClientInput::EndTurn);
                    action_context = ActionContext {
                        last_command: cmd,
                        was_end_turn: is_end_turn,
                        monster_intents: truth.monsters.iter().map(|m| format!("{:?}", m.current_intent)).collect(),
                        monster_names: truth.monsters.iter().map(|m| {
                            format!("type_{}", m.monster_type)
                        }).collect(),
                    };
                    
                    // Predict the outcome of this action locally
                    let mut local_cs = truth.clone();
                    crate::engine::core::tick_until_stable_turn(&mut engine_state, &mut local_cs, input);
                    expected_combat_state = Some(local_cs);
                }
                cmd_failed_count = 0;
            } else {
                writeln!(log, "  SEND: END (fallback)").unwrap();
                println!("END");
                last_sent_cmd = "END".to_string();
                cmd_failed_count = 0;
            }
            stdout.flush().unwrap();
        } else {
            // ══════════════════════════════════════════════════════
            //  NON-COMBAT ROUTING
            //  Priority chain matching bottled_ai's handler order:
            //    1. leave      → RETURN  (exit shop/event overlay)
            //    2. SHOP_ROOM  → PROCEED (enter the shop)
            //    3. choose     → CHOOSE  (pick first option, or skip/discard potion)
            //    4. proceed    → PROCEED
            //    5. confirm    → PROCEED (confirm acts like proceed)
            //    6. skip       → SKIP    (skip card reward)
            //    7. cancel     → RETURN
            //    8. wait       → WAIT 30 (animation not finished, poll again)
            // ══════════════════════════════════════════════════════

            let choice_list: Vec<&str> = gs.get("choice_list")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let potions_full = if let Some(arr) = gs.get("potions").and_then(|v| v.as_array()) {
                arr.iter().all(|p| p.get("id").and_then(|id| id.as_str()).unwrap_or("Potion Slot") != "Potion Slot")
            } else {
                false
            };

            let mut cmd = if has("leave") && screen != "SHOP_ROOM" {
                "RETURN".to_string()
            } else if screen == "SHOP_ROOM" && has("proceed") {
                "PROCEED".to_string()
            } else if screen == "SHOP_ROOM" && has("choose") && !choice_list.is_empty() {
                "CHOOSE 0".to_string()
            } else if has("choose") && !choice_list.is_empty() {
                if choice_list[0] == "potion" && potions_full {
                    "POTION DISCARD 0".to_string()
                } else if choice_list.len() == 1 && choice_list[0] == "potion" && has("skip") {
                    "SKIP".to_string()
                } else {
                    format!("CHOOSE {}", choose_best_index(&choice_list))
                }
            } else if has("proceed") {
                "PROCEED".to_string()
            } else if has("confirm") {
                "CONFIRM".to_string()
            } else if has("skip") {
                "SKIP".to_string()
            } else if has("leave") {
                "RETURN".to_string()
            } else if has("cancel") {
                "RETURN".to_string()
            } else if has("return") {
                "RETURN".to_string()
            } else if has("wait") {
                "WAIT 30".to_string()
            } else {
                writeln!(log, "  [!] UNKNOWN STATE: avail={:?} screen={}", avail, screen).unwrap();
                "STATE".to_string()
            };

            if cmd == last_sent_cmd && cmd_failed_count > 0 {
                writeln!(log, "  [!] ERROR LOOP DETECTED, FALLING BACK").unwrap();
                if has("skip") { cmd = "SKIP".to_string(); }
                else if has("proceed") { cmd = "PROCEED".to_string(); }
                else { cmd = "RETURN".to_string(); }
            }
            
            last_sent_cmd = cmd.clone();
            cmd_failed_count = 0;

            // Since we are falling back to simple string commands (like "CHOOSE 0"), 
            // we did NOT simulate this natively via `tick_until_stable_turn`. 
            // The future frame will be impossible to predict accurately without simulating this choice.
            // Erase the leftover expected combat state to prevent fake diff errors on the next frame!
            expected_combat_state = None;

            writeln!(log, "[F{}] {}  screen={}  → {}", frame_count, room_phase, screen, cmd).unwrap();
            println!("{}", cmd);
            stdout.flush().unwrap();
        }

        let _ = log.flush();
    }

    writeln!(log, "=== Loop exited ===").unwrap();
    let _ = log.flush();
    let _ = raw.flush();
}

// ─── Choose best option index (simple heuristics) ────────────

fn choose_best_index(_choices: &[&str]) -> usize {
    // Prefer non-skip options, pick first by default
    // Special: for card rewards, prefer anything available (index 0)
    // In the future this can become smarter (evaluate events, etc.)
    0
}

// ─── Format powers for logging ───────────────────────────────

fn format_powers(cs: &CombatState, entity_id: usize) -> String {
    cs.power_db.get(&entity_id).map_or(String::new(), |powers| {
        powers.iter().map(|p| {
            let def = crate::content::powers::get_power_definition(p.power_type);
            format!("{}={}", def.name, p.amount)
        }).collect::<Vec<_>>().join(", ")
    })
}

// ─── Java-side context dump (only on diff) ───────────────────

#[allow(dead_code)]
fn write_java_context(log: &mut std::fs::File, cv: &serde_json::Value) {
    let jp = &cv["player"];
    writeln!(log, "    [Java] player: hp={} blk={} energy={}",
        jp["current_hp"].as_i64().unwrap_or(-1),
        jp["block"].as_i64().unwrap_or(-1),
        jp["energy"].as_i64().unwrap_or(-1),
    ).unwrap();

    if let Some(arr) = jp["powers"].as_array() {
        let ps: Vec<String> = arr.iter().map(|p|
            format!("{}={}", p["id"].as_str().unwrap_or("?"), p["amount"].as_i64().unwrap_or(0))
        ).collect();
        if !ps.is_empty() {
            writeln!(log, "    [Java] player pw: [{}]", ps.join(", ")).unwrap();
        }
    }

    if let Some(arr) = cv["hand"].as_array() {
        let hs: Vec<String> = arr.iter().enumerate().map(|(i, c)|
            format!("{}:{}", i, c["name"].as_str().unwrap_or("?"))
        ).collect();
        writeln!(log, "    [Java] hand({}): [{}]", arr.len(), hs.join(", ")).unwrap();
    }

    if let Some(arr) = cv["monsters"].as_array() {
        for (i, jm) in arr.iter().enumerate() {
            let powers: Vec<String> = jm["powers"].as_array().map_or(vec![], |ps|
                ps.iter().map(|p| format!("{}={}", p["id"].as_str().unwrap_or("?"), p["amount"])).collect()
            );
            writeln!(log, "    [Java] M[{}]: hp={} blk={} pw=[{}]",
                i, jm["current_hp"].as_i64().unwrap_or(-1),
                jm["block"].as_i64().unwrap_or(-1), powers.join(", "),
            ).unwrap();
        }
    }

    writeln!(log, "    [Java] draw={} disc={} exhaust={}",
        cv["draw_pile"].as_array().map_or(0, |a| a.len()),
        cv["discard_pile"].as_array().map_or(0, |a| a.len()),
        cv["exhaust_pile"].as_array().map_or(0, |a| a.len()),
    ).unwrap();
}

// ─── Parse-validation: check if state_sync correctly parsed Java JSON ─────

fn validate_parse(cs: &CombatState, cv: &serde_json::Value) -> Vec<String> {
    let mut diffs = Vec::new();
    let jp = &cv["player"];

    // Energy
    let j_energy = jp["energy"].as_u64().unwrap_or(0) as u8;
    if cs.energy != j_energy {
        diffs.push(format!("energy: rust={} java={}", cs.energy, j_energy));
    }

    // Player HP
    let j_hp = jp["current_hp"].as_i64().unwrap_or(0) as i32;
    if cs.player.current_hp != j_hp {
        diffs.push(format!("player.hp: rust={} java={}", cs.player.current_hp, j_hp));
    }

    // Player block
    let j_block = jp["block"].as_i64().unwrap_or(0) as i32;
    if cs.player.block != j_block {
        diffs.push(format!("player.block: rust={} java={}", cs.player.block, j_block));
    }

    // Hand size
    if let Some(j_hand) = cv["hand"].as_array() {
        if cs.hand.len() != j_hand.len() {
            diffs.push(format!("hand_size: rust={} java={}", cs.hand.len(), j_hand.len()));
        }
        // Check for unmapped cards
        for (i, jc) in j_hand.iter().enumerate() {
            let jid = jc["id"].as_str().unwrap_or("?");
            if card_id_from_java(jid).is_none() {
                let name = jc["name"].as_str().unwrap_or("?");
                diffs.push(format!("hand[{}]: UNMAPPED card java_id='{}' name='{}'", i, jid, name));
            }
        }
    }

    // Draw pile unmapped cards
    if let Some(j_draw) = cv["draw_pile"].as_array() {
        for jc in j_draw {
            let jid = jc["id"].as_str().unwrap_or("?");
            if card_id_from_java(jid).is_none() {
                let name = jc["name"].as_str().unwrap_or("?");
                diffs.push(format!("draw_pile: UNMAPPED card java_id='{}' name='{}'", jid, name));
            }
        }
    }

    // Monster count
    if let Some(j_monsters) = cv["monsters"].as_array() {
        if cs.monsters.len() != j_monsters.len() {
            diffs.push(format!("monster_count: rust={} java={}", cs.monsters.len(), j_monsters.len()));
        }
        for (i, jm) in j_monsters.iter().enumerate() {
            if i >= cs.monsters.len() {
                diffs.push(format!("monster[{}]: MISSING in Rust", i));
                continue;
            }
            let rm = &cs.monsters[i];
            let j_mhp = jm["current_hp"].as_i64().unwrap_or(0) as i32;
            if rm.current_hp != j_mhp {
                diffs.push(format!("monster[{}].hp: rust={} java={}", i, rm.current_hp, j_mhp));
            }
            // Check unmapped powers on monster
            if let Some(j_powers) = jm["powers"].as_array() {
                for jp in j_powers {
                    let pid = jp["id"].as_str().unwrap_or("?");
                    if power_id_from_java(pid).is_none() {
                        diffs.push(format!("monster[{}].power: UNMAPPED '{}' amount={}",
                            i, pid, jp["amount"].as_i64().unwrap_or(0)));
                    }
                }
            }
        }
    }

    // Player unmapped powers
    if let Some(j_powers) = jp["powers"].as_array() {
        for jp_item in j_powers {
            let pid = jp_item["id"].as_str().unwrap_or("?");
            if power_id_from_java(pid).is_none() {
                diffs.push(format!("player.power: UNMAPPED '{}' amount={}",
                    pid, jp_item["amount"].as_i64().unwrap_or(0)));
            }
        }
    }

    diffs
}
