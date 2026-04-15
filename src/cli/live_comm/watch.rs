use super::{unix_time_millis, LiveWatchCaptureConfig, LiveWatchMatchMode, RAW_PATH};
use crate::diff::protocol::{card_id_from_java, power_id_from_java, relic_id_from_java};
use crate::testing::fixtures::live_capture::build_fixture_from_record_window;
use crate::testing::fixtures::scenario::{ScenarioAssertion, ScenarioProvenance};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub(super) struct LiveWatchRuntime {
    recent_records: BTreeMap<i64, Value>,
    captured_response_ids: HashSet<i64>,
    recent_capture_signatures: BTreeMap<String, i64>,
    captures_written: usize,
}

#[derive(Debug, Default)]
pub(super) struct LiveWatchMatch {
    pub(super) tags: Vec<String>,
    pub(super) assertions: Vec<ScenarioAssertion>,
    pub(super) notes: Vec<String>,
}

pub(super) fn remember_live_record(
    runtime: &mut LiveWatchRuntime,
    response_id: i64,
    root: &Value,
    max_records: usize,
) {
    runtime.recent_records.insert(response_id, root.clone());
    while runtime.recent_records.len() > max_records.max(1) {
        let Some(oldest) = runtime.recent_records.keys().next().copied() else {
            break;
        };
        runtime.recent_records.remove(&oldest);
    }
}

pub(super) fn collect_live_watch_match(
    config: &LiveWatchCaptureConfig,
    root: &Value,
) -> Option<LiveWatchMatch> {
    let gs = root.get("game_state")?;
    let mut matched = LiveWatchMatch::default();
    let mut seen_fields = HashSet::new();
    let mut seen_tags = HashSet::new();
    let mut matched_requirements = HashSet::new();
    let configured_requirements = configured_watch_requirements(config);

    let screen = gs
        .get("screen_type")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    for wanted in &config.screens {
        if wanted.eq_ignore_ascii_case(screen) {
            matched_requirements.insert(watch_requirement_key("screen", wanted));
            let tag = format!("watch_screen:{screen}");
            if seen_tags.insert(tag.clone()) {
                matched.tags.push(tag);
            }
            matched.notes.push(format!("screen={screen}"));
        }
    }

    let room_phase = gs.get("room_phase").and_then(|v| v.as_str()).unwrap_or("?");
    for wanted in &config.room_phases {
        if wanted.eq_ignore_ascii_case(room_phase) {
            matched_requirements.insert(watch_requirement_key("room_phase", wanted));
            let tag = format!("watch_room_phase:{room_phase}");
            if seen_tags.insert(tag.clone()) {
                matched.tags.push(tag);
            }
            matched.notes.push(format!("room_phase={room_phase}"));
        }
    }

    let command_kind = root
        .get("protocol_meta")
        .and_then(|m| m.get("last_command_kind"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    for wanted in &config.command_kinds {
        if wanted.eq_ignore_ascii_case(command_kind) {
            matched_requirements.insert(watch_requirement_key("command_kind", wanted));
            let tag = format!("watch_command_kind:{command_kind}");
            if seen_tags.insert(tag.clone()) {
                matched.tags.push(tag);
            }
            matched.notes.push(format!("command_kind={command_kind}"));
        }
    }

    if let Some(relics) = gs.get("relics").and_then(|v| v.as_array()) {
        for wanted in &config.relics {
            let count = relics
                .iter()
                .filter(|relic| {
                    relic
                        .get("id")
                        .and_then(|v| v.as_str())
                        .is_some_and(|raw| watch_matches_relic(wanted, raw))
                })
                .count() as i64;
            if count > 0 {
                matched_requirements.insert(watch_requirement_key("relic", wanted));
                let canonical = canonical_relic_watch_id(wanted, relics);
                let field = format!("relics.count[{canonical}]");
                if seen_fields.insert(field.clone()) {
                    matched.assertions.push(ScenarioAssertion {
                        field,
                        expected_kind: "number".to_string(),
                        expected_value: Some(json!(count)),
                        note: Some(format!("watch relic {wanted}")),
                        ..Default::default()
                    });
                }
                let tag = format!("watch_relic:{canonical}");
                if seen_tags.insert(tag.clone()) {
                    matched.tags.push(tag);
                }
            }
        }
    }

    let combat = gs.get("combat_state").filter(|v| !v.is_null());
    if let Some(combat) = combat {
        for (pile, cards) in [
            ("hand", combat.get("hand")),
            ("draw_pile", combat.get("draw_pile")),
            ("discard_pile", combat.get("discard_pile")),
            ("exhaust_pile", combat.get("exhaust_pile")),
            ("limbo", combat.get("limbo")),
        ] {
            let Some(cards) = cards.and_then(|v| v.as_array()) else {
                continue;
            };
            for wanted in &config.cards {
                let mut matched_raw = None;
                let count = cards
                    .iter()
                    .filter(|card| {
                        let raw = card.get("id").and_then(|v| v.as_str());
                        if let Some(raw) = raw {
                            if watch_matches_card(wanted, raw) {
                                matched_raw.get_or_insert(raw.to_string());
                                return true;
                            }
                        }
                        false
                    })
                    .count() as i64;
                if count > 0 {
                    matched_requirements.insert(watch_requirement_key("card", wanted));
                    let canonical = canonical_card_watch_id(wanted, matched_raw.as_deref());
                    let field = format!("{pile}.count[{canonical}]");
                    if seen_fields.insert(field.clone()) {
                        matched.assertions.push(ScenarioAssertion {
                            field,
                            expected_kind: "number".to_string(),
                            expected_value: Some(json!(count)),
                            note: Some(format!("watch card {wanted} in {pile}")),
                            ..Default::default()
                        });
                    }
                    let tag = format!("watch_card:{canonical}");
                    if seen_tags.insert(tag.clone()) {
                        matched.tags.push(tag);
                    }
                }
            }
        }

        if let Some(player_powers) = combat
            .get("player")
            .and_then(|v| v.get("powers"))
            .and_then(|v| v.as_array())
        {
            collect_power_watch_assertions(
                &mut matched,
                &mut seen_fields,
                &mut seen_tags,
                &mut matched_requirements,
                &config.powers,
                player_powers,
                None,
            );
        }
        if let Some(monsters) = combat.get("monsters").and_then(|v| v.as_array()) {
            for wanted in &config.monsters {
                let count = monsters
                    .iter()
                    .filter(|monster| {
                        monster
                            .get("id")
                            .and_then(|v| v.as_str())
                            .is_some_and(|raw| raw.eq_ignore_ascii_case(wanted))
                    })
                    .count() as i64;
                if count > 0 {
                    matched_requirements.insert(watch_requirement_key("monster", wanted));
                    let canonical = monsters
                        .iter()
                        .filter_map(|monster| monster.get("id").and_then(|v| v.as_str()))
                        .find(|raw| raw.eq_ignore_ascii_case(wanted))
                        .unwrap_or(wanted);
                    let field = format!("monsters.count[{canonical}]");
                    if seen_fields.insert(field.clone()) {
                        matched.assertions.push(ScenarioAssertion {
                            field,
                            expected_kind: "number".to_string(),
                            expected_value: Some(json!(count)),
                            note: Some(format!("watch monster {wanted}")),
                            ..Default::default()
                        });
                    }
                    let tag = format!("watch_monster:{canonical}");
                    if seen_tags.insert(tag.clone()) {
                        matched.tags.push(tag);
                    }
                }
            }
            for (monster_idx, monster) in monsters.iter().enumerate() {
                let Some(powers) = monster.get("powers").and_then(|v| v.as_array()) else {
                    continue;
                };
                collect_power_watch_assertions(
                    &mut matched,
                    &mut seen_fields,
                    &mut seen_tags,
                    &mut matched_requirements,
                    &config.powers,
                    powers,
                    Some(monster_idx),
                );
            }
        }
    }

    if let Some(cards) = gs
        .get("screen_state")
        .and_then(|v| v.get("cards"))
        .and_then(|v| v.as_array())
    {
        for wanted in &config.cards {
            for card in cards {
                let Some(raw) = card.get("id").and_then(|v| v.as_str()) else {
                    continue;
                };
                if watch_matches_card(wanted, raw) {
                    matched_requirements.insert(watch_requirement_key("card", wanted));
                    let canonical = canonical_card_watch_id(wanted, Some(raw));
                    let tag = format!("watch_screen_card:{canonical}");
                    if seen_tags.insert(tag.clone()) {
                        matched.tags.push(tag);
                    }
                    matched.notes.push(format!("screen_card={canonical}"));
                }
            }
        }
    }

    let has_any_match = !matched_requirements.is_empty();
    let all_requirements_matched = configured_requirements
        .iter()
        .all(|key| matched_requirements.contains(key));
    let match_ok = match config.match_mode {
        LiveWatchMatchMode::Any => has_any_match,
        LiveWatchMatchMode::All => !configured_requirements.is_empty() && all_requirements_matched,
    };

    if !match_ok
        || (matched.tags.is_empty() && matched.assertions.is_empty() && matched.notes.is_empty())
    {
        None
    } else {
        Some(matched)
    }
}

fn collect_power_watch_assertions(
    matched: &mut LiveWatchMatch,
    seen_fields: &mut HashSet<String>,
    seen_tags: &mut HashSet<String>,
    matched_requirements: &mut HashSet<String>,
    wanted_powers: &[String],
    powers: &[Value],
    monster_idx: Option<usize>,
) {
    for wanted in wanted_powers {
        for power in powers {
            let Some(raw_id) = power.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            if !watch_matches_power(wanted, raw_id) {
                continue;
            }
            matched_requirements.insert(watch_requirement_key("power", wanted));
            let amount = power.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
            let field = match monster_idx {
                Some(idx) => format!("monster[{idx}].power[{raw_id}].amount"),
                None => format!("player.power[{raw_id}].amount"),
            };
            if seen_fields.insert(field.clone()) {
                matched.assertions.push(ScenarioAssertion {
                    field,
                    expected_kind: "number".to_string(),
                    expected_value: Some(json!(amount)),
                    note: Some(format!("watch power {wanted}")),
                    ..Default::default()
                });
            }
            let tag = format!("watch_power:{raw_id}");
            if seen_tags.insert(tag.clone()) {
                matched.tags.push(tag);
            }
        }
    }
}

fn configured_watch_requirements(config: &LiveWatchCaptureConfig) -> HashSet<String> {
    let mut out = HashSet::new();
    for wanted in &config.cards {
        out.insert(watch_requirement_key("card", wanted));
    }
    for wanted in &config.relics {
        out.insert(watch_requirement_key("relic", wanted));
    }
    for wanted in &config.powers {
        out.insert(watch_requirement_key("power", wanted));
    }
    for wanted in &config.monsters {
        out.insert(watch_requirement_key("monster", wanted));
    }
    for wanted in &config.screens {
        out.insert(watch_requirement_key("screen", wanted));
    }
    for wanted in &config.room_phases {
        out.insert(watch_requirement_key("room_phase", wanted));
    }
    for wanted in &config.command_kinds {
        out.insert(watch_requirement_key("command_kind", wanted));
    }
    out
}

fn watch_requirement_key(kind: &str, wanted: &str) -> String {
    format!("{kind}:{}", wanted.trim().to_ascii_lowercase())
}

pub(super) fn watch_capture_signature(root: &Value, matched: &LiveWatchMatch) -> String {
    let gs = root.get("game_state").unwrap_or(&Value::Null);
    let screen = gs
        .get("screen_type")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let room_phase = gs.get("room_phase").and_then(|v| v.as_str()).unwrap_or("?");
    let command_kind = root
        .get("protocol_meta")
        .and_then(|m| m.get("last_command_kind"))
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    let mut tags = matched.tags.clone();
    tags.sort();
    tags.dedup();

    let mut assertion_keys = matched
        .assertions
        .iter()
        .map(|a| {
            format!(
                "{}={}",
                a.field,
                a.expected_value
                    .as_ref()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| a.expected_kind.clone())
            )
        })
        .collect::<Vec<_>>();
    assertion_keys.sort();
    assertion_keys.dedup();

    format!(
        "screen={screen}|room_phase={room_phase}|command_kind={command_kind}|tags={}|assertions={}",
        tags.join(","),
        assertion_keys.join(",")
    )
}

pub(super) fn capture_deduped(
    runtime: &mut LiveWatchRuntime,
    signature: &str,
    response_id: i64,
    dedupe_window_responses: usize,
) -> bool {
    if dedupe_window_responses == 0 {
        runtime
            .recent_capture_signatures
            .insert(signature.to_string(), response_id);
        return false;
    }

    if let Some(previous_response_id) = runtime.recent_capture_signatures.get(signature).copied() {
        if response_id.saturating_sub(previous_response_id) <= dedupe_window_responses as i64 {
            return true;
        }
    }

    runtime
        .recent_capture_signatures
        .insert(signature.to_string(), response_id);
    let min_response_id = response_id.saturating_sub(dedupe_window_responses as i64);
    runtime
        .recent_capture_signatures
        .retain(|_, seen_response_id| *seen_response_id >= min_response_id);
    false
}

pub(super) fn maybe_capture_live_watch(
    config: &LiveWatchCaptureConfig,
    runtime: &mut LiveWatchRuntime,
    root: &Value,
    frame_count: u64,
    log: &mut std::fs::File,
    watch_audit: &mut std::fs::File,
    watch_noncombat_audit: &mut std::fs::File,
) {
    if !config.enabled() || runtime.captures_written >= config.max_captures.max(1) {
        return;
    }
    let Some(response_id) = root
        .get("protocol_meta")
        .and_then(|m| m.get("response_id"))
        .and_then(|v| v.as_i64())
    else {
        return;
    };
    if runtime.captured_response_ids.contains(&response_id) {
        return;
    }

    let Some(matched) = collect_live_watch_match(config, root) else {
        return;
    };
    let capture_signature = watch_capture_signature(root, &matched);
    if capture_deduped(
        runtime,
        &capture_signature,
        response_id,
        config.dedupe_window_responses,
    ) {
        let _ = writeln!(
            log,
            "[F{}] WATCH SKIPPED DUPLICATE response_id={} cooldown={} signature={}",
            frame_count, response_id, config.dedupe_window_responses, capture_signature
        );
        return;
    }
    let is_capture_eligible = root
        .get("game_state")
        .and_then(|gs| gs.get("combat_state"))
        .is_some_and(|v| !v.is_null());
    if !is_capture_eligible {
        write_noncombat_watch_sidecar(
            config,
            runtime,
            root,
            frame_count,
            &matched,
            log,
            watch_noncombat_audit,
        );
        return;
    }

    let start_response_id = contiguous_window_start(
        &runtime.recent_records,
        response_id,
        config.window_responses.max(1),
    );
    let safe_name = sanitize_capture_name(&matched.tags.join("__"));
    let file_name = format!("watch_{}_{}.json", response_id, safe_name);
    let out_path = config.out_dir.join(file_name);
    let fixture_name = format!("live_watch_{response_id}");
    let mut notes = matched.notes.clone();
    notes.push(format!("frame={frame_count}"));
    let mut assertions = matched.assertions.clone();
    assertions.extend(default_watch_context_assertions(root));
    let fixture = match build_fixture_from_record_window(
        &runtime.recent_records,
        start_response_id,
        response_id,
        fixture_name,
        dedupe_assertions(assertions),
        {
            let mut tags = vec!["watch_capture".to_string()];
            tags.extend(matched.tags.clone());
            tags
        },
        Some(ScenarioProvenance {
            source: Some("live_comm_watch".to_string()),
            source_path: Some(RAW_PATH.to_string()),
            response_id_range: Some((start_response_id as u64, response_id as u64)),
            failure_frame: None,
            notes,
            ..Default::default()
        }),
    ) {
        Ok(fixture) => fixture,
        Err(err) => {
            let _ = writeln!(
                log,
                "[F{}] WATCH CAPTURE FAILED response_id={} err={}",
                frame_count, response_id, err
            );
            return;
        }
    };

    if let Err(err) = std::fs::create_dir_all(&config.out_dir) {
        let _ = writeln!(
            log,
            "[F{}] WATCH CAPTURE FAILED creating dir {}: {}",
            frame_count,
            config.out_dir.display(),
            err
        );
        return;
    }
    let fixture_text = match serde_json::to_string_pretty(&fixture) {
        Ok(text) => text,
        Err(err) => {
            let _ = writeln!(
                log,
                "[F{}] WATCH CAPTURE FAILED serializing fixture: {}",
                frame_count, err
            );
            return;
        }
    };
    if let Err(err) = std::fs::write(&out_path, fixture_text) {
        let _ = writeln!(
            log,
            "[F{}] WATCH CAPTURE FAILED writing {}: {}",
            frame_count,
            out_path.display(),
            err
        );
        return;
    }

    runtime.captured_response_ids.insert(response_id);
    runtime.captures_written += 1;
    let assertion_count = fixture.assertions.len();
    let fixture_tags = fixture.tags.clone();
    let suggested_minimize_out_path = derive_watch_minimized_fixture_path(&out_path);
    let suggested_minimize_cmd =
        build_watch_minimize_suggestion(&out_path, &suggested_minimize_out_path);
    let audit = json!({
        "logged_at_unix_ms": unix_time_millis(),
        "frame": frame_count,
        "response_id": response_id,
        "start_response_id": start_response_id,
        "out_path": out_path.to_string_lossy(),
        "suggested_minimize_out_path": suggested_minimize_out_path.to_string_lossy(),
        "suggested_minimize_cmd": suggested_minimize_cmd,
        "tags": fixture_tags,
        "assertion_count": assertion_count,
    });
    let _ = writeln!(watch_audit, "{}", audit);
    let _ = watch_audit.flush();
    let _ = writeln!(
        log,
        "[F{}] WATCH CAPTURED {} tags={} assertions={}",
        frame_count,
        out_path.display(),
        matched.tags.join(", "),
        assertion_count
    );
    let _ = writeln!(
        log,
        "[F{}] WATCH MINIMIZE {}",
        frame_count,
        audit
            .get("suggested_minimize_cmd")
            .and_then(|v| v.as_str())
            .unwrap_or("")
    );
}

pub(super) fn derive_watch_minimized_fixture_path(out_path: &Path) -> PathBuf {
    if let Some(ext) = out_path.extension().and_then(|v| v.to_str()) {
        out_path.with_extension(format!("min.{ext}"))
    } else {
        let file_name = out_path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("watch_fixture");
        out_path.with_file_name(format!("{file_name}.min.json"))
    }
}

pub(super) fn build_watch_minimize_suggestion(
    out_path: &Path,
    minimized_out_path: &Path,
) -> String {
    format!(
        "python tools/analysis/live_regression.py minimize --repo-root \"D:\\rust\\sts_simulator\" --fixture \"{}\" --out \"{}\"",
        out_path.display(),
        minimized_out_path.display()
    )
}

pub(super) fn default_watch_context_assertions(root: &Value) -> Vec<ScenarioAssertion> {
    let Some(combat) = root
        .get("game_state")
        .and_then(|gs| gs.get("combat_state"))
        .filter(|v| !v.is_null())
    else {
        return Vec::new();
    };

    let mut assertions = Vec::new();
    if let Some(player) = combat.get("player") {
        if let Some(hp) = player.get("current_hp").and_then(|v| v.as_i64()) {
            assertions.push(number_assertion("player.hp", hp, "watch context player hp"));
        }
        if let Some(block) = player.get("block").and_then(|v| v.as_i64()) {
            assertions.push(number_assertion(
                "player.block",
                block,
                "watch context player block",
            ));
        }
        if let Some(energy) = player.get("energy").and_then(|v| v.as_i64()) {
            assertions.push(number_assertion(
                "player.energy",
                energy,
                "watch context player energy",
            ));
        }
    }
    if let Some(monsters) = combat.get("monsters").and_then(|v| v.as_array()) {
        assertions.push(number_assertion(
            "monster_count",
            monsters.len() as i64,
            "watch context monster count",
        ));
        for (idx, monster) in monsters.iter().enumerate() {
            if let Some(hp) = monster.get("current_hp").and_then(|v| v.as_i64()) {
                assertions.push(number_assertion(
                    &format!("monster[{idx}].hp"),
                    hp,
                    "watch context monster hp",
                ));
            }
            if let Some(block) = monster.get("block").and_then(|v| v.as_i64()) {
                assertions.push(number_assertion(
                    &format!("monster[{idx}].block"),
                    block,
                    "watch context monster block",
                ));
            }
        }
    }
    for (field, key) in [
        ("hand_size", "hand"),
        ("draw_pile_size", "draw_pile"),
        ("discard_pile_size", "discard_pile"),
        ("exhaust_pile_size", "exhaust_pile"),
        ("limbo_size", "limbo"),
    ] {
        if let Some(count) = combat
            .get(key)
            .and_then(|v| v.as_array())
            .map(|v| v.len() as i64)
        {
            assertions.push(number_assertion(field, count, "watch context pile size"));
        }
    }
    assertions
}

fn number_assertion(field: &str, value: i64, note: &str) -> ScenarioAssertion {
    ScenarioAssertion {
        field: field.to_string(),
        expected_kind: "number".to_string(),
        expected_value: Some(json!(value)),
        note: Some(note.to_string()),
        ..Default::default()
    }
}

pub(super) fn build_noncombat_watch_sidecar(
    response_id: i64,
    frame_count: u64,
    root: &Value,
    matched: &LiveWatchMatch,
    out_path: &Path,
) -> Value {
    json!({
        "kind": "live_watch_noncombat",
        "response_id": response_id,
        "frame": frame_count,
        "tags": matched.tags.clone(),
        "notes": matched.notes.clone(),
        "protocol_meta": root.get("protocol_meta").cloned().unwrap_or(Value::Null),
        "available_commands": root.get("available_commands").cloned().unwrap_or(Value::Null),
        "screen_summary": build_noncombat_screen_summary(root),
        "context_summary": build_noncombat_context_summary(root),
        "out_path": out_path.to_string_lossy(),
    })
}

pub(super) fn build_noncombat_screen_summary(root: &Value) -> Value {
    let Some(gs) = root.get("game_state") else {
        return Value::Null;
    };
    let screen = gs
        .get("screen_type")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let choice_list = gs
        .get("choice_list")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    match screen {
        "EVENT" => json!({
            "screen": screen,
            "event_id": gs.get("screen_state").and_then(|s| s.get("event_id")).and_then(|v| v.as_str()),
            "choice_list": choice_list,
        }),
        "CARD_REWARD" => {
            let cards = gs
                .get("screen_state")
                .and_then(|s| s.get("cards"))
                .and_then(|v| v.as_array())
                .map(|cards| {
                    cards
                        .iter()
                        .map(summarize_card_like_entry)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json!({
                "screen": screen,
                "skip_available": gs.get("screen_state").and_then(|s| s.get("skip_available")).and_then(|v| v.as_bool()),
                "cards": cards,
            })
        }
        "COMBAT_REWARD" => {
            let rewards = gs
                .get("screen_state")
                .and_then(|s| s.get("rewards"))
                .and_then(|v| v.as_array())
                .map(|rewards| {
                    rewards
                        .iter()
                        .map(summarize_reward_entry)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json!({
                "screen": screen,
                "rewards": rewards,
            })
        }
        "SHOP_SCREEN" => {
            let shop = gs.get("screen_state").unwrap_or(&Value::Null);
            let cards = shop
                .get("cards")
                .and_then(|v| v.as_array())
                .map(|cards| {
                    cards
                        .iter()
                        .map(summarize_shop_card_entry)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let relics = shop
                .get("relics")
                .and_then(|v| v.as_array())
                .map(|relics| {
                    relics
                        .iter()
                        .map(summarize_shop_relic_entry)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let potions = shop
                .get("potions")
                .and_then(|v| v.as_array())
                .map(|potions| {
                    potions
                        .iter()
                        .map(summarize_shop_potion_entry)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json!({
                "screen": screen,
                "purge_available": shop.get("purge_available").and_then(|v| v.as_bool()),
                "purge_cost": shop.get("purge_cost").and_then(|v| v.as_i64()),
                "cards": cards,
                "relics": relics,
                "potions": potions,
            })
        }
        "MAP" => json!({
            "screen": screen,
            "current_node": gs.get("screen_state").and_then(|s| s.get("current_node")).cloned().unwrap_or(Value::Null),
            "boss_node_available": gs.get("screen_state").and_then(|s| s.get("boss_node_available")).and_then(|v| v.as_bool()),
            "choice_list": choice_list,
        }),
        "REST" => json!({
            "screen": screen,
            "choice_list": choice_list,
        }),
        "GRID" => {
            let state = gs.get("screen_state").unwrap_or(&Value::Null);
            let cards = state
                .get("cards")
                .and_then(|v| v.as_array())
                .map(|cards| {
                    cards
                        .iter()
                        .map(summarize_card_like_entry)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            json!({
                "screen": screen,
                "for_upgrade": state.get("for_upgrade").and_then(|v| v.as_bool()),
                "for_purge": state.get("for_purge").and_then(|v| v.as_bool()),
                "for_transform": state.get("for_transform").and_then(|v| v.as_bool()),
                "selected_cards_len": state.get("selected_cards").and_then(|v| v.as_array()).map(|v| v.len()),
                "cards": cards,
            })
        }
        _ => json!({
            "screen": screen,
            "choice_list": choice_list,
        }),
    }
}

pub(super) fn build_noncombat_context_summary(root: &Value) -> Value {
    let Some(gs) = root.get("game_state") else {
        return Value::Null;
    };

    let deck = gs
        .get("deck")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let relics = gs
        .get("relics")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let potions = gs
        .get("potions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let deck_size = deck.len();
    let upgraded_cards = deck
        .iter()
        .filter(|card| card.get("upgrades").and_then(|v| v.as_i64()).unwrap_or(0) > 0)
        .count();
    let curse_count = deck
        .iter()
        .filter(|card| {
            card.get("type")
                .and_then(|v| v.as_str())
                .is_some_and(|ty| ty.eq_ignore_ascii_case("CURSE"))
        })
        .count();
    let power_count = deck
        .iter()
        .filter(|card| {
            card.get("type")
                .and_then(|v| v.as_str())
                .is_some_and(|ty| ty.eq_ignore_ascii_case("POWER"))
        })
        .count();
    let relic_ids = relics
        .iter()
        .filter_map(|relic| relic.get("id").and_then(|v| v.as_str()).map(str::to_owned))
        .collect::<Vec<_>>();
    let potion_ids = potions
        .iter()
        .filter_map(|potion| potion.get("id").and_then(|v| v.as_str()))
        .filter(|id| *id != "Potion Slot")
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let current_node = gs
        .get("screen_state")
        .and_then(|s| s.get("current_node"))
        .cloned()
        .unwrap_or(Value::Null);
    let rest_options = gs
        .get("screen_state")
        .and_then(|s| s.get("rest_options"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    json!({
        "class": gs.get("class").and_then(|v| v.as_str()),
        "act": gs.get("act").and_then(|v| v.as_i64()),
        "floor": gs.get("floor").and_then(|v| v.as_i64()),
        "act_boss": gs.get("act_boss").and_then(|v| v.as_str()),
        "room_type": gs.get("room_type").and_then(|v| v.as_str()),
        "room_phase": gs.get("room_phase").and_then(|v| v.as_str()),
        "screen_type": gs.get("screen_type").and_then(|v| v.as_str()),
        "current_hp": gs.get("current_hp").and_then(|v| v.as_i64()),
        "max_hp": gs.get("max_hp").and_then(|v| v.as_i64()),
        "gold": gs.get("gold").and_then(|v| v.as_i64()),
        "deck_summary": {
            "size": deck_size,
            "upgraded_cards": upgraded_cards,
            "powers": power_count,
            "curses": curse_count,
        },
        "relic_ids": relic_ids,
        "potion_ids": potion_ids,
        "current_node": current_node,
        "rest_options": rest_options,
    })
}

fn summarize_card_like_entry(card: &Value) -> Value {
    json!({
        "id": card.get("id").and_then(|v| v.as_str()),
        "name": card.get("name").and_then(|v| v.as_str()),
        "upgrades": card.get("upgrades").and_then(|v| v.as_i64()).unwrap_or(0),
        "price": card.get("price").and_then(|v| v.as_i64()),
    })
}

fn summarize_reward_entry(reward: &Value) -> Value {
    let reward_type = reward
        .get("reward_type")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    match reward_type {
        "GOLD" => json!({
            "reward_type": reward_type,
            "gold": reward.get("gold").and_then(|v| v.as_i64()),
        }),
        "POTION" => json!({
            "reward_type": reward_type,
            "id": reward.get("potion").and_then(|p| p.get("id")).and_then(|v| v.as_str()),
        }),
        "RELIC" => json!({
            "reward_type": reward_type,
            "id": reward.get("relic").and_then(|r| r.get("id")).and_then(|v| v.as_str()),
        }),
        "CARD" => json!({
            "reward_type": reward_type,
            "cards": reward
                .get("cards")
                .and_then(|v| v.as_array())
                .map(|cards| cards.iter().map(summarize_card_like_entry).collect::<Vec<_>>())
                .unwrap_or_default(),
        }),
        _ => json!({
            "reward_type": reward_type,
        }),
    }
}

fn summarize_shop_card_entry(card: &Value) -> Value {
    json!({
        "id": card.get("id").and_then(|v| v.as_str()),
        "name": card.get("name").and_then(|v| v.as_str()),
        "upgrades": card.get("upgrades").and_then(|v| v.as_i64()).unwrap_or(0),
        "price": card.get("price").and_then(|v| v.as_i64()),
    })
}

fn summarize_shop_relic_entry(relic: &Value) -> Value {
    json!({
        "id": relic.get("id").and_then(|v| v.as_str()),
        "name": relic.get("name").and_then(|v| v.as_str()),
        "price": relic.get("price").and_then(|v| v.as_i64()),
    })
}

fn summarize_shop_potion_entry(potion: &Value) -> Value {
    json!({
        "id": potion.get("id").and_then(|v| v.as_str()),
        "name": potion.get("name").and_then(|v| v.as_str()),
        "price": potion.get("price").and_then(|v| v.as_i64()),
    })
}

fn write_noncombat_watch_sidecar(
    config: &LiveWatchCaptureConfig,
    runtime: &mut LiveWatchRuntime,
    root: &Value,
    frame_count: u64,
    matched: &LiveWatchMatch,
    log: &mut std::fs::File,
    watch_noncombat_audit: &mut std::fs::File,
) {
    let Some(response_id) = root
        .get("protocol_meta")
        .and_then(|m| m.get("response_id"))
        .and_then(|v| v.as_i64())
    else {
        return;
    };
    let safe_name = sanitize_capture_name(&matched.tags.join("__"));
    let out_path = config.out_dir.join(format!(
        "watch_noncombat_{}_{}.json",
        response_id, safe_name
    ));
    if let Err(err) = std::fs::create_dir_all(&config.out_dir) {
        let _ = writeln!(
            log,
            "[F{}] WATCH NONCOMBAT FAILED creating dir {}: {}",
            frame_count,
            config.out_dir.display(),
            err
        );
        return;
    }
    let sidecar = build_noncombat_watch_sidecar(response_id, frame_count, root, matched, &out_path);
    let sidecar_text = match serde_json::to_string_pretty(&sidecar) {
        Ok(text) => text,
        Err(err) => {
            let _ = writeln!(
                log,
                "[F{}] WATCH NONCOMBAT FAILED serializing sidecar: {}",
                frame_count, err
            );
            return;
        }
    };
    if let Err(err) = std::fs::write(&out_path, sidecar_text) {
        let _ = writeln!(
            log,
            "[F{}] WATCH NONCOMBAT FAILED writing {}: {}",
            frame_count,
            out_path.display(),
            err
        );
        return;
    }
    runtime.captured_response_ids.insert(response_id);
    runtime.captures_written += 1;
    let _ = writeln!(watch_noncombat_audit, "{}", sidecar);
    let _ = watch_noncombat_audit.flush();
    let _ = writeln!(
        log,
        "[F{}] WATCH NONCOMBAT CAPTURED {} tags={}",
        frame_count,
        out_path.display(),
        matched.tags.join(", ")
    );
}

fn contiguous_window_start(
    records: &BTreeMap<i64, Value>,
    end_response_id: i64,
    max_len: usize,
) -> i64 {
    let mut current = end_response_id;
    let mut start = end_response_id;
    let mut used = 1usize;
    while used < max_len {
        let prev = current - 1;
        if records.contains_key(&prev) {
            start = prev;
            current = prev;
            used += 1;
        } else {
            break;
        }
    }
    start
}

fn sanitize_capture_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, ':' | '_' | '-') {
            out.push('_');
        }
    }
    if out.is_empty() {
        "capture".to_string()
    } else {
        out
    }
}

fn dedupe_assertions(assertions: Vec<ScenarioAssertion>) -> Vec<ScenarioAssertion> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for assertion in assertions {
        if seen.insert(assertion.field.clone()) {
            out.push(assertion);
        }
    }
    out
}

fn canonical_card_watch_id(wanted: &str, raw: Option<&str>) -> String {
    card_id_from_java(wanted)
        .map(crate::content::cards::java_id)
        .map(|s| s.to_string())
        .or_else(|| raw.map(|s| s.to_string()))
        .unwrap_or_else(|| wanted.to_string())
}

fn canonical_relic_watch_id(wanted: &str, relics: &[Value]) -> String {
    relics
        .iter()
        .filter_map(|relic| relic.get("id").and_then(|v| v.as_str()))
        .find(|raw| watch_matches_relic(wanted, raw))
        .map(|s| s.to_string())
        .unwrap_or_else(|| wanted.to_string())
}

fn watch_matches_card(wanted: &str, raw_java_id: &str) -> bool {
    wanted.eq_ignore_ascii_case(raw_java_id)
        || matches!(
            (card_id_from_java(wanted), card_id_from_java(raw_java_id)),
            (Some(left), Some(right)) if left == right
        )
}

fn watch_matches_relic(wanted: &str, raw_java_id: &str) -> bool {
    wanted.eq_ignore_ascii_case(raw_java_id)
        || matches!(
            (relic_id_from_java(wanted), relic_id_from_java(raw_java_id)),
            (Some(left), Some(right)) if left == right
        )
}

fn watch_matches_power(wanted: &str, raw_java_id: &str) -> bool {
    wanted.eq_ignore_ascii_case(raw_java_id)
        || matches!(
            (power_id_from_java(wanted), power_id_from_java(raw_java_id)),
            (Some(left), Some(right)) if left == right
        )
}
