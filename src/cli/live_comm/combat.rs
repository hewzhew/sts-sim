use super::frame::LiveFrame;
use super::io::LiveCommIo;
use crate::bot::comm_mod;
use crate::bot::coverage::CoverageDb;
use crate::combat::CombatState;
use crate::diff::comparator::{ActionContext, DiffCategory, DiffResult};
use crate::diff::mapper::{card_id_from_java, power_id_from_java};
use crate::state::core::{ClientInput, EngineState};
use serde_json::Value;
use std::io::Write;

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
    seen_content_gaps: std::collections::HashSet<String>,
}

#[derive(Default)]
pub(super) struct CombatRuntime {
    pub(super) expected_combat_state: Option<CombatState>,
    pub(super) last_combat_truth: Option<CombatState>,
    pub(super) last_input: Option<ClientInput>,
    pub(super) action_context: ActionContext,
    combat_stats: Option<CombatStats>,
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
        let engine_bugs: Vec<_> = self
            .diffs
            .iter()
            .filter(|d| d.category == DiffCategory::EngineBug)
            .collect();
        let content_gaps: Vec<_> = self
            .diffs
            .iter()
            .filter(|d| d.category == DiffCategory::ContentGap)
            .collect();
        let timing: Vec<_> = self
            .diffs
            .iter()
            .filter(|d| d.category == DiffCategory::Timing)
            .collect();

        writeln!(
            log,
            "\n╔══════════════════════════════════════════════════════╗"
        )
        .unwrap();
        writeln!(
            log,
            "║  COMBAT SUMMARY (F{} ~ F{})                          ",
            self.start_frame, end_frame
        )
        .unwrap();
        writeln!(
            log,
            "╠══════════════════════════════════════════════════════╣"
        )
        .unwrap();
        writeln!(
            log,
            "║  Frames: {}  |  Actions: {}",
            end_frame - self.start_frame + 1,
            self.action_count
        )
        .unwrap();
        writeln!(log, "║  ENGINE BUGS:  {}", engine_bugs.len()).unwrap();
        writeln!(log, "║  CONTENT GAPS: {}", content_gaps.len()).unwrap();
        writeln!(log, "║  TIMING:       {}", timing.len()).unwrap();

        if !engine_bugs.is_empty() {
            writeln!(log, "║").unwrap();
            writeln!(log, "║  ⛔ Engine Bugs:").unwrap();
            let mut seen = std::collections::HashMap::<String, (usize, String, String)>::new();
            for d in &engine_bugs {
                let entry = seen.entry(d.field.clone()).or_insert((
                    0,
                    d.rust_val.clone(),
                    d.java_val.clone(),
                ));
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
        writeln!(
            log,
            "╚══════════════════════════════════════════════════════╝"
        )
        .unwrap();
    }
}

impl CombatRuntime {
    pub(super) fn on_java_error(&mut self) {
        self.expected_combat_state = None;
    }

    pub(super) fn clear_after_combat_if_needed(
        &mut self,
        log: &mut std::fs::File,
        frame_count: u64,
    ) {
        self.last_combat_truth = None;
        self.last_input = None;
        if let Some(stats) = self.combat_stats.take() {
            stats.write_summary(log, frame_count.saturating_sub(1));
        }
    }

    pub(super) fn flush_summary_on_game_over(&mut self, log: &mut std::fs::File, frame_count: u64) {
        if let Some(stats) = self.combat_stats.take() {
            stats.write_summary(log, frame_count);
        }
    }

    pub(super) fn ensure_combat_stats(&mut self, frame_count: u64) {
        if self.combat_stats.is_none() {
            self.combat_stats = Some(CombatStats::new(frame_count));
        }
    }

    pub(super) fn increment_action_count(&mut self) {
        if let Some(stats) = self.combat_stats.as_mut() {
            stats.action_count += 1;
        }
    }

    pub(super) fn record_action_diffs(
        &mut self,
        action_diffs: &[DiffResult],
        frame_count: u64,
        log: &mut std::fs::File,
        engine_bug_summary_interval: usize,
        engine_bug_total: &mut usize,
        content_gap_total: &mut usize,
    ) {
        let bugs: Vec<_> = action_diffs
            .iter()
            .filter(|d| d.category == DiffCategory::EngineBug)
            .collect();
        let gaps: Vec<_> = action_diffs
            .iter()
            .filter(|d| d.category == DiffCategory::ContentGap)
            .collect();
        let timing: Vec<_> = action_diffs
            .iter()
            .filter(|d| d.category == DiffCategory::Timing)
            .collect();

        *engine_bug_total += bugs.len();
        *content_gap_total += gaps.len();

        writeln!(
            log,
            "  >>> PARITY FAIL ({} diffs: {} bugs, {} gaps, {} timing) <<<",
            action_diffs.len(),
            bugs.len(),
            gaps.len(),
            timing.len()
        )
        .unwrap();
        writeln!(log, "  CAUSED BY: {}", self.action_context.describe()).unwrap();

        let stats = self.combat_stats.as_mut().unwrap();
        for d in action_diffs {
            let is_repeat_gap = d.category == DiffCategory::ContentGap
                && stats.seen_content_gaps.contains(&d.field);

            if !is_repeat_gap {
                writeln!(
                    log,
                    "    {} : Rust={}, Java={}  [{}]",
                    d.field, d.rust_val, d.java_val, d.category
                )
                .unwrap();
            }

            if d.category == DiffCategory::ContentGap {
                stats.seen_content_gaps.insert(d.field.clone());
            }

            stats.diffs.push(CombatDiffRecord {
                _frame: frame_count,
                field: d.field.clone(),
                category: d.category,
                rust_val: d.rust_val.clone(),
                java_val: d.java_val.clone(),
            });
        }

        if *engine_bug_total > 0 && *engine_bug_total % engine_bug_summary_interval == 0 {
            writeln!(
                log,
                "  [SAMPLING] {} engine bugs observed so far; continuing collection.",
                *engine_bug_total
            )
            .unwrap();
        }

        writeln!(log, "  [HEALED] Prediction chain reset from Java truth").unwrap();
    }
}

fn is_hexaghost_monster_type(monster_type: usize) -> bool {
    monster_type == crate::content::monsters::EnemyId::Hexaghost as usize
}

fn format_move_history(history: &std::collections::VecDeque<u8>) -> String {
    history
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn log_hexaghost_end_turn_debug(log: &mut std::fs::File, expected_cs: &CombatState, cv: &Value) {
    let rust_hex = expected_cs
        .entities
        .monsters
        .iter()
        .find(|m| is_hexaghost_monster_type(m.monster_type));
    let java_hex = cv
        .get("monsters")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter().find(|m| {
                m.get("id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| id.eq_ignore_ascii_case("Hexaghost"))
            })
        });

    if rust_hex.is_none() && java_hex.is_none() {
        return;
    }

    writeln!(log, "  [HEXAGHOST END DEBUG]").unwrap();
    if let Some(rust_hex) = rust_hex {
        writeln!(
            log,
            "    rust_post_end: hp={}/{} blk={} next_move_byte={} intent={:?} move_history=[{}] intent_dmg={}",
            rust_hex.current_hp,
            rust_hex.max_hp,
            rust_hex.block,
            rust_hex.next_move_byte,
            rust_hex.current_intent,
            format_move_history(&rust_hex.move_history),
            rust_hex.intent_dmg
        )
        .unwrap();
    }
    if let Some(java_hex) = java_hex {
        writeln!(
            log,
            "    java_post_end: hp={}/{} blk={} move_id={} intent={} base_dmg={} adj_dmg={} hits={}",
            java_hex.get("current_hp").and_then(|v| v.as_i64()).unwrap_or(-1),
            java_hex.get("max_hp").and_then(|v| v.as_i64()).unwrap_or(-1),
            java_hex.get("block").and_then(|v| v.as_i64()).unwrap_or(-1),
            java_hex.get("move_id").and_then(|v| v.as_i64()).unwrap_or(-1),
            java_hex.get("intent").and_then(|v| v.as_str()).unwrap_or("?"),
            java_hex
                .get("move_base_damage")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            java_hex
                .get("move_adjusted_damage")
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            java_hex.get("move_hits").and_then(|v| v.as_i64()).unwrap_or(-1)
        )
        .unwrap();
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn handle_live_combat_frame<W: Write>(
    frame: &LiveFrame,
    gs: &Value,
    frame_count: u64,
    last_sent_cmd: &mut String,
    cmd_failed_count: &mut u32,
    engine_bug_total: &mut usize,
    content_gap_total: &mut usize,
    coverage_db: &mut CoverageDb,
    combat_runtime: &mut CombatRuntime,
    live_io: &mut LiveCommIo,
    stdout: &mut W,
    engine_bug_summary_interval: usize,
    signature_source_file: &str,
) {
    let cv = frame
        .combat_state()
        .expect("combat branch requires combat_state");
    let rv = frame.relics();
    let combat_snapshot = build_live_combat_snapshot(gs);
    let mut truth = crate::diff::state_sync::build_combat_state(&combat_snapshot, rv);
    if let Some(previous_runtime) = combat_runtime
        .expected_combat_state
        .as_ref()
        .or(combat_runtime.last_combat_truth.as_ref())
    {
        crate::diff::state_sync::carry_internal_runtime_state(previous_runtime, &mut truth);
    }

    if let (Some(prev_truth), Some(prev_input)) = (
        &combat_runtime.last_combat_truth,
        &combat_runtime.last_input,
    ) {
        let after_engine = EngineState::CombatPlayerTurn;
        let signature = crate::interaction_coverage::signature_from_transition(
            &EngineState::CombatPlayerTurn,
            prev_truth,
            prev_input,
            &after_engine,
            &truth,
        );
        let signature_key = signature.canonical_key();
        let is_novel = !coverage_db.tested_signatures.contains(&signature_key);
        let novel_archetypes: Vec<String> = signature
            .archetype_tags
            .iter()
            .filter(|tag| !coverage_db.tested_archetypes.contains(*tag))
            .cloned()
            .collect();
        coverage_db.record_signature(&signature);
        coverage_db.save();
        let record = crate::interaction_coverage::ObservedInteractionRecord {
            observed_from: "live_comm".to_string(),
            source_file: signature_source_file.to_string(),
            combat_idx: None,
            action_idx: Some(frame_count as usize),
            command: crate::interaction_coverage::command_string(prev_input),
            signature_key,
            source_combo_key: signature.source_combo_key(),
            signature,
        };
        writeln!(
            live_io.signature_log,
            "{}",
            serde_json::to_string(&record).unwrap_or_else(|_| "{}".to_string())
        )
        .unwrap();
        if is_novel {
            writeln!(live_io.log, "  [NOVEL SIGNATURE] {}", record.signature_key).unwrap();
        }
        if !novel_archetypes.is_empty() {
            writeln!(
                live_io.log,
                "  [NOVEL ARCHETYPE] {} via {}",
                novel_archetypes.join(", "),
                record.signature.source_id
            )
            .unwrap();
        }
        if !record.signature.archetype_tags.is_empty() {
            writeln!(
                live_io.log,
                "  [ARCHETYPES] {}",
                record.signature.archetype_tags.join(", ")
            )
            .unwrap();
        }
    }

    combat_runtime.ensure_combat_stats(frame_count);
    log_combat_overview(&mut live_io.log, frame_count, &truth);

    let sync_diffs = validate_parse(&truth, cv);
    if !sync_diffs.is_empty() {
        writeln!(live_io.log, "  >>> PARSE DIFF ({}) <<<", sync_diffs.len()).unwrap();
        for d in &sync_diffs {
            writeln!(live_io.log, "    {}", d).unwrap();
        }
    }

    if let Some(expected_cs) = combat_runtime.expected_combat_state.take() {
        let action_diffs = crate::diff::comparator::compare_states(
            &expected_cs,
            cv,
            combat_runtime.action_context.was_end_turn,
            &combat_runtime.action_context,
        );

        if !action_diffs.is_empty() {
            if combat_runtime.action_context.was_end_turn
                && (expected_cs
                    .entities
                    .monsters
                    .iter()
                    .any(|m| is_hexaghost_monster_type(m.monster_type))
                    || cv
                        .get("monsters")
                        .and_then(|v| v.as_array())
                        .is_some_and(|arr| {
                            arr.iter().any(|m| {
                                m.get("id")
                                    .and_then(|v| v.as_str())
                                    .is_some_and(|id| id.eq_ignore_ascii_case("Hexaghost"))
                            })
                        }))
            {
                log_hexaghost_end_turn_debug(&mut live_io.log, &expected_cs, cv);
            }
            combat_runtime.record_action_diffs(
                &action_diffs,
                frame_count,
                &mut live_io.log,
                engine_bug_summary_interval,
                engine_bug_total,
                content_gap_total,
            );
        } else {
            writeln!(live_io.log, "  >>> PARITY OK <<<").unwrap();
        }
    }

    let input = crate::bot::combat_heuristic::decide_heuristic(&truth);
    writeln!(live_io.log, "  → {:?}", input).unwrap();
    if matches!(input, crate::state::core::ClientInput::EndTurn) {
        for line in crate::bot::combat_heuristic::describe_end_turn_options(&truth) {
            writeln!(live_io.log, "  [END DIAG] {}", line).unwrap();
        }
    }

    combat_runtime.increment_action_count();

    let mut engine_state = EngineState::CombatPlayerTurn;
    if let Some(cmd) = comm_mod::input_to_java_command(&input, &engine_state) {
        if cmd == *last_sent_cmd && *cmd_failed_count > 0 {
            writeln!(
                live_io.log,
                "  [!] AVOIDING REPEATED ERROR BY FORCING END TURN"
            )
            .unwrap();
            live_io.send_line(stdout, "END");
            *last_sent_cmd = "END".to_string();
        } else {
            writeln!(live_io.log, "  SEND: {}", cmd).unwrap();
            live_io.send_line(stdout, &cmd);
            *last_sent_cmd = cmd.clone();

            let is_end_turn = matches!(input, crate::state::core::ClientInput::EndTurn);
            combat_runtime.action_context = ActionContext {
                last_command: cmd,
                was_end_turn: is_end_turn,
                monster_intents: truth
                    .entities
                    .monsters
                    .iter()
                    .map(|m| format!("{:?}", m.current_intent))
                    .collect(),
                monster_names: truth
                    .entities
                    .monsters
                    .iter()
                    .map(|m| format!("type_{}", m.monster_type))
                    .collect(),
                has_rng_state: cv.get("rng_state").is_some(),
            };

            let mut local_cs = truth.clone();
            crate::engine::core::tick_until_stable_turn(
                &mut engine_state,
                &mut local_cs,
                input.clone(),
            );
            combat_runtime.expected_combat_state = Some(local_cs);
        }
        *cmd_failed_count = 0;
        combat_runtime.last_combat_truth = Some(truth.clone());
        combat_runtime.last_input = Some(input.clone());
    } else {
        writeln!(live_io.log, "  SEND: END (fallback)").unwrap();
        live_io.send_line(stdout, "END");
        *last_sent_cmd = "END".to_string();
        *cmd_failed_count = 0;
    }
}

pub(super) fn build_live_combat_snapshot(gs: &Value) -> Value {
    let mut snapshot = gs
        .get("combat_state")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = snapshot.as_object_mut() {
        if let Some(room_type) = gs.get("room_type").cloned() {
            obj.insert("room_type".to_string(), room_type);
        }
        if let Some(potions) = gs.get("potions").cloned() {
            obj.insert("potions".to_string(), potions);
        }
    }
    snapshot
}

pub(super) fn log_combat_overview(log: &mut std::fs::File, frame_count: u64, truth: &CombatState) {
    writeln!(
        log,
        "\n[F{}] COMBAT  HP={}/{}  E={}  Hand={}  Draw={}  Disc={}  Monsters={}",
        frame_count,
        truth.entities.player.current_hp,
        truth.entities.player.max_hp,
        truth.turn.energy,
        truth.zones.hand.len(),
        truth.zones.draw_pile.len(),
        truth.zones.discard_pile.len(),
        truth.entities.monsters.len()
    )
    .unwrap();

    for (i, m) in truth.entities.monsters.iter().enumerate() {
        let powers = format_powers(truth, m.id);
        let dead_str = if m.is_dying || m.is_escaped {
            " (DEAD)"
        } else {
            ""
        };
        writeln!(
            log,
            "  M[{}] id={} hp={}/{} blk={} intent={:?}{}{}",
            i,
            m.id,
            m.current_hp,
            m.max_hp,
            m.block,
            m.current_intent,
            if powers.is_empty() {
                String::new()
            } else {
                format!(" pw=[{}]", powers)
            },
            dead_str
        )
        .unwrap();
    }

    let hand_str: Vec<String> = truth
        .zones
        .hand
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let def = crate::content::cards::get_card_definition(c.id);
            let u = if c.upgrades > 0 { "+" } else { "" };
            format!("{}:{}{}", i, def.name, u)
        })
        .collect();
    writeln!(log, "  Hand: [{}]", hand_str.join(", ")).unwrap();

    let pp = format_powers(truth, 0);
    if !pp.is_empty() {
        writeln!(log, "  Player pw: [{}]", pp).unwrap();
    }
}

fn format_powers(cs: &CombatState, entity_id: usize) -> String {
    cs.entities
        .power_db
        .get(&entity_id)
        .map_or(String::new(), |powers| {
            powers
                .iter()
                .map(|p| {
                    let def = crate::content::powers::get_power_definition(p.power_type);
                    format!("{}={}", def.name, p.amount)
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
}

#[allow(dead_code)]
pub(super) fn write_java_context(log: &mut std::fs::File, cv: &Value) {
    let jp = &cv["player"];
    writeln!(
        log,
        "    [Java] player: hp={} blk={} energy={}",
        jp["current_hp"].as_i64().unwrap_or(-1),
        jp["block"].as_i64().unwrap_or(-1),
        jp["energy"].as_i64().unwrap_or(-1),
    )
    .unwrap();

    if let Some(arr) = jp["powers"].as_array() {
        let ps: Vec<String> = arr
            .iter()
            .map(|p| {
                format!(
                    "{}={}",
                    p["id"].as_str().unwrap_or("?"),
                    p["amount"].as_i64().unwrap_or(0)
                )
            })
            .collect();
        if !ps.is_empty() {
            writeln!(log, "    [Java] player pw: [{}]", ps.join(", ")).unwrap();
        }
    }

    if let Some(arr) = cv["hand"].as_array() {
        let hs: Vec<String> = arr
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{}:{}", i, c["name"].as_str().unwrap_or("?")))
            .collect();
        writeln!(log, "    [Java] hand({}): [{}]", arr.len(), hs.join(", ")).unwrap();
    }

    if let Some(arr) = cv["monsters"].as_array() {
        for (i, jm) in arr.iter().enumerate() {
            let powers: Vec<String> = jm["powers"].as_array().map_or(vec![], |ps| {
                ps.iter()
                    .map(|p| format!("{}={}", p["id"].as_str().unwrap_or("?"), p["amount"]))
                    .collect()
            });
            writeln!(
                log,
                "    [Java] M[{}]: hp={} blk={} pw=[{}]",
                i,
                jm["current_hp"].as_i64().unwrap_or(-1),
                jm["block"].as_i64().unwrap_or(-1),
                powers.join(", "),
            )
            .unwrap();
        }
    }

    writeln!(
        log,
        "    [Java] draw={} disc={} exhaust={}",
        cv["draw_pile"].as_array().map_or(0, |a| a.len()),
        cv["discard_pile"].as_array().map_or(0, |a| a.len()),
        cv["exhaust_pile"].as_array().map_or(0, |a| a.len()),
    )
    .unwrap();
}

pub(super) fn validate_parse(cs: &CombatState, cv: &Value) -> Vec<String> {
    let mut diffs = Vec::new();
    let jp = &cv["player"];

    let j_energy = jp["energy"].as_u64().unwrap_or(0) as u8;
    if cs.turn.energy != j_energy {
        diffs.push(format!("energy: rust={} java={}", cs.turn.energy, j_energy));
    }

    let j_hp = jp["current_hp"].as_i64().unwrap_or(0) as i32;
    if cs.entities.player.current_hp != j_hp {
        diffs.push(format!(
            "player.hp: rust={} java={}",
            cs.entities.player.current_hp, j_hp
        ));
    }

    let j_block = jp["block"].as_i64().unwrap_or(0) as i32;
    if cs.entities.player.block != j_block {
        diffs.push(format!(
            "player.block: rust={} java={}",
            cs.entities.player.block, j_block
        ));
    }

    if let Some(j_hand) = cv["hand"].as_array() {
        if cs.zones.hand.len() != j_hand.len() {
            diffs.push(format!(
                "hand_size: rust={} java={}",
                cs.zones.hand.len(),
                j_hand.len()
            ));
        }
        for (i, jc) in j_hand.iter().enumerate() {
            let jid = jc["id"].as_str().unwrap_or("?");
            if card_id_from_java(jid).is_none() {
                let name = jc["name"].as_str().unwrap_or("?");
                diffs.push(format!(
                    "hand[{}]: UNMAPPED card java_id='{}' name='{}'",
                    i, jid, name
                ));
            }
        }
    }

    if let Some(j_draw) = cv["draw_pile"].as_array() {
        for jc in j_draw {
            let jid = jc["id"].as_str().unwrap_or("?");
            if card_id_from_java(jid).is_none() {
                let name = jc["name"].as_str().unwrap_or("?");
                diffs.push(format!(
                    "draw_pile: UNMAPPED card java_id='{}' name='{}'",
                    jid, name
                ));
            }
        }
    }

    if let Some(j_monsters) = cv["monsters"].as_array() {
        if cs.entities.monsters.len() != j_monsters.len() {
            diffs.push(format!(
                "monster_count: rust={} java={}",
                cs.entities.monsters.len(),
                j_monsters.len()
            ));
        }
        for (i, jm) in j_monsters.iter().enumerate() {
            if i >= cs.entities.monsters.len() {
                diffs.push(format!("monster[{}]: MISSING in Rust", i));
                continue;
            }
            let rm = &cs.entities.monsters[i];
            let j_mhp = jm["current_hp"].as_i64().unwrap_or(0) as i32;
            if rm.current_hp != j_mhp {
                diffs.push(format!(
                    "monster[{}].hp: rust={} java={}",
                    i, rm.current_hp, j_mhp
                ));
            }
            if let Some(j_powers) = jm["powers"].as_array() {
                for jp in j_powers {
                    let pid = jp["id"].as_str().unwrap_or("?");
                    if power_id_from_java(pid).is_none() {
                        diffs.push(format!(
                            "monster[{}].power: UNMAPPED '{}' amount={}",
                            i,
                            pid,
                            jp["amount"].as_i64().unwrap_or(0)
                        ));
                    }
                }
            }
        }
    }

    if let Some(j_powers) = jp["powers"].as_array() {
        for jp_item in j_powers {
            let pid = jp_item["id"].as_str().unwrap_or("?");
            if power_id_from_java(pid).is_none() {
                diffs.push(format!(
                    "player.power: UNMAPPED '{}' amount={}",
                    pid,
                    jp_item["amount"].as_i64().unwrap_or(0)
                ));
            }
        }
    }

    diffs
}
