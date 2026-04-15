use super::live_comm_admin::{
    ensure_log_dirs, gc_runs, is_clean_label, rewrite_manifest, write_manifest, LiveArtifactRecord,
    LiveLogPaths, LiveProfileMetadata, LiveRetentionFlags, LiveRunArtifacts, LiveRunCounts,
    LiveRunManifest, LiveRunProvenance, LiveRunValidation,
};
use crate::diff::replay::{derive_combat_replay_view, verify_combat_replay_view};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const PROFILE_PATH: &str = r"d:\rust\sts_simulator\tools\live_comm\profile.json";

#[derive(Clone, Debug, Default)]
pub(crate) struct FinalizeRunInput {
    pub run_id: String,
    pub timestamp: String,
    pub build_tag: String,
    pub parity_mode: String,
    pub watch_enabled: bool,
    pub session_exit_reason: String,
    pub engine_bug_total: usize,
    pub content_gap_total: usize,
    pub timing_diff_total: usize,
    pub replay_failures: usize,
    pub game_over_seen: bool,
    pub final_victory: bool,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct FinalizeRunOutcome {
    pub run_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub classification_label: String,
    pub validation_status: String,
    pub gc_summary: String,
}

pub(crate) fn load_profile_metadata() -> LiveProfileMetadata {
    let Ok(text) = std::fs::read_to_string(PROFILE_PATH) else {
        return LiveProfileMetadata::default();
    };
    let Ok(root) = serde_json::from_str::<Value>(&text) else {
        return LiveProfileMetadata::default();
    };
    LiveProfileMetadata {
        profile_name: root
            .get("activated_profile")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        purpose: root
            .get("purpose")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        capture_policy: root
            .get("capture_policy")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
    }
}

fn env_opt(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn current_exe_path_string() -> Option<String> {
    std::env::current_exe()
        .ok()
        .map(|path| path.display().to_string())
}

fn current_exe_mtime_fallback() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let secs = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(format!("unix:{secs}"))
}

pub(crate) fn runtime_provenance() -> LiveRunProvenance {
    let profile = load_profile_metadata();
    let git_short = option_env!("LIVE_COMM_GIT_SHORT")
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| env_opt("LIVE_COMM_LAUNCH_GIT_SHORT"));
    let build_unix = option_env!("LIVE_COMM_BUILD_UNIX").and_then(|s| s.parse::<u64>().ok());

    LiveRunProvenance {
        exe_path: env_opt("LIVE_COMM_LAUNCH_EXE_PATH").or_else(current_exe_path_string),
        exe_mtime_utc: env_opt("LIVE_COMM_LAUNCH_EXE_MTIME_UTC")
            .or_else(current_exe_mtime_fallback),
        git_short_sha: git_short,
        build_unix,
        build_time_utc: build_unix.map(|secs| format!("unix:{secs}")),
        profile_path: env_opt("LIVE_COMM_LAUNCH_PROFILE_PATH")
            .or_else(|| Some(PROFILE_PATH.to_string())),
        profile_name: env_opt("LIVE_COMM_LAUNCH_PROFILE_NAME").or(profile.profile_name),
    }
}

fn file_contains(path: &Path, needle: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|text| text.contains(needle))
        .unwrap_or(false)
}

fn count_valid_jsonl_records(path: &Path) -> Result<usize, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read '{}': {err}", path.display()))?;
    let mut count = 0usize;
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        serde_json::from_str::<Value>(trimmed).map_err(|err| {
            format!(
                "invalid jsonl in '{}' at line {}: {err}",
                path.display(),
                idx + 1
            )
        })?;
        count += 1;
    }
    Ok(count)
}

fn raw_contains_event_frames(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(root) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let screen_type = root
            .get("game_state")
            .and_then(|state| state.get("screen_type").or_else(|| state.get("screen_name")))
            .and_then(Value::as_str);
        if screen_type.is_some_and(|screen| screen.eq_ignore_ascii_case("EVENT")) {
            return true;
        }
    }
    false
}

fn debug_bootstrap_protocol_ok(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    !text.contains("Invalid command: ready")
        && !text.contains("Invalid command: __LIVE_COMM_BOOTSTRAP__")
}

fn detect_reward_loop(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let mut signatures: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(root) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let Some(game_state) = root.get("game_state") else {
            continue;
        };
        let screen = game_state
            .get("screen_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let signature = match screen {
            "COMBAT_REWARD" => {
                let rewards = game_state
                    .get("screen_state")
                    .and_then(|v| v.get("rewards"))
                    .and_then(Value::as_array);
                let active = rewards
                    .into_iter()
                    .flatten()
                    .map(|reward| {
                        let reward_type = reward
                            .get("reward_type")
                            .and_then(Value::as_str)
                            .unwrap_or("UNKNOWN");
                        let choice_index = reward
                            .get("choice_index")
                            .and_then(Value::as_u64)
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "?".to_string());
                        format!("{reward_type}@{choice_index}")
                    })
                    .collect::<Vec<_>>();
                format!("COMBAT_REWARD:{}", active.join(","))
            }
            "CARD_REWARD" => {
                let cards = game_state
                    .get("screen_state")
                    .and_then(|v| v.get("cards"))
                    .and_then(Value::as_array);
                let offered = cards
                    .into_iter()
                    .flatten()
                    .filter_map(|card| {
                        card.get("id")
                            .and_then(Value::as_str)
                            .or_else(|| card.get("name").and_then(Value::as_str))
                    })
                    .collect::<Vec<_>>();
                format!("CARD_REWARD:{}", offered.join(","))
            }
            _ => {
                signatures.clear();
                continue;
            }
        };
        signatures.push(signature);
        if signatures.len() >= 6 {
            let n = signatures.len();
            if signatures[n - 1] == signatures[n - 3]
                && signatures[n - 2] == signatures[n - 4]
                && signatures[n - 3] == signatures[n - 5]
                && signatures[n - 4] == signatures[n - 6]
                && signatures[n - 1] != signatures[n - 2]
            {
                return true;
            }
        }
        if signatures.len() > 8 {
            signatures.drain(0..signatures.len() - 8);
        }
        if signatures.len() >= 4
            && signatures[signatures.len() - 1] == signatures[signatures.len() - 3]
            && signatures[signatures.len() - 2] == signatures[signatures.len() - 4]
            && signatures[signatures.len() - 1] != signatures[signatures.len() - 2]
        {
            return true;
        }
    }
    false
}

fn event_screen_validation(path: &Path) -> (bool, bool) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return (false, false);
    };
    let mut fields_present = true;
    let mut multistage_ok = true;
    let mut multistage_counts = std::collections::BTreeMap::new();
    let mut multistage_has_progress = std::collections::BTreeMap::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(root) = serde_json::from_str::<Value>(trimmed) else {
            fields_present = false;
            continue;
        };
        let Some(decision) = root.get("decision") else {
            fields_present = false;
            continue;
        };
        let has_index = decision.get("screen_index").is_some();
        let has_key = decision.get("screen_key").is_some_and(|v| !v.is_null());
        let has_source = decision.get("screen_source").is_some_and(|v| !v.is_null());
        if !has_index || !has_source {
            fields_present = false;
        }
        let event_name = decision
            .get("event_name")
            .and_then(Value::as_str)
            .unwrap_or("");
        if matches!(
            event_name,
            "Neow" | "Shining Light" | "Golden Idol" | "Knowing Skull" | "Living Wall" | "Big Fish"
        ) {
            *multistage_counts.entry(event_name.to_string()).or_insert(0usize) += 1;
            let progressed = decision
                .get("screen_index")
                .and_then(Value::as_u64)
                .is_some_and(|v| v > 0)
                || has_key;
            let entry = multistage_has_progress
                .entry(event_name.to_string())
                .or_insert(false);
            *entry |= progressed;
        }
    }
    for (event_name, count) in multistage_counts {
        if count > 1
            && !multistage_has_progress
                .get(&event_name)
                .copied()
                .unwrap_or(false)
        {
            multistage_ok = false;
        }
    }
    (fields_present, multistage_ok)
}

fn last_failure_snapshot_frame(path: &Path) -> Option<u64> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return None;
    };
    text.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .filter_map(|root| root.get("frame").and_then(Value::as_u64))
        .last()
}

fn validate_run_artifacts(run_dir: &Path, manifest: &LiveRunManifest) -> LiveRunValidation {
    let focus_path = run_dir.join("focus.txt");
    let debug_path = run_dir.join("debug.txt");
    let raw_path = run_dir.join("raw.jsonl");
    let event_audit_path = run_dir.join("event_audit.jsonl");
    let failure_snapshots_path = run_dir.join("failure_snapshots.jsonl");

    let focus_has_event_trace = file_contains(&focus_path, "[EVENT]");
    let debug_has_event_policy = file_contains(&debug_path, "EVENT POLICY");
    let event_frames_present = raw_contains_event_frames(&raw_path);
    let event_audit_present = event_audit_path.exists();
    let event_audit_json_lines = count_valid_jsonl_records(&event_audit_path).unwrap_or(0);
    let reward_loop_detected = detect_reward_loop(&raw_path);
    let bootstrap_protocol_ok = debug_bootstrap_protocol_ok(&debug_path);
    let (event_screen_fields_present, event_screen_nonzero_or_keyed_for_multistage_events) =
        event_screen_validation(&event_audit_path);
    let manifest_lists_event_audit = manifest
        .artifacts
        .event_audit
        .as_ref()
        .is_some_and(|artifact| artifact.relative_path == "event_audit.jsonl");

    let trace_incomplete = event_frames_present
        && (!focus_has_event_trace
            || !debug_has_event_policy
            || !event_audit_present
            || event_audit_json_lines == 0
            || reward_loop_detected
            || !bootstrap_protocol_ok
            || !event_screen_fields_present
            || !event_screen_nonzero_or_keyed_for_multistage_events
            || !manifest_lists_event_audit);

    let mut errors = Vec::new();
    if event_frames_present && !focus_has_event_trace {
        errors.push("event frames present but focus.txt has no [EVENT] trace".to_string());
    }
    if event_frames_present && !debug_has_event_policy {
        errors.push("event frames present but debug.txt has no EVENT POLICY trace".to_string());
    }
    if !manifest_lists_event_audit {
        errors.push("manifest is missing event_audit artifact record".to_string());
    }
    if event_frames_present && !event_audit_present {
        errors.push("event frames present but event_audit.jsonl is missing".to_string());
    }
    if event_frames_present && event_audit_present && event_audit_json_lines == 0 {
        errors.push("event frames present but event_audit.jsonl has no valid records".to_string());
    }
    if reward_loop_detected {
        errors.push("reward loop detected in raw protocol stream".to_string());
    }
    if !bootstrap_protocol_ok {
        errors.push("bootstrap protocol leaked into normal command stream".to_string());
    }
    if event_frames_present && !event_screen_fields_present {
        errors.push(
            "event trace present but screen_index/screen_source fields are missing".to_string(),
        );
    }
    if event_frames_present && !event_screen_nonzero_or_keyed_for_multistage_events {
        errors.push("multistage events never reported a keyed or advanced screen".to_string());
    }

    let status = if trace_incomplete || reward_loop_detected || !bootstrap_protocol_ok {
        "trace_incomplete"
    } else if event_frames_present {
        "ok"
    } else {
        "ok_no_events"
    };

    LiveRunValidation {
        status: status.to_string(),
        event_frames_present,
        focus_has_event_trace,
        debug_has_event_policy,
        event_audit_present,
        event_audit_json_lines,
        manifest_lists_event_audit,
        reward_loop_detected,
        bootstrap_protocol_ok,
        event_screen_fields_present,
        event_screen_nonzero_or_keyed_for_multistage_events,
        trace_incomplete,
        latest_failure_snapshot_frame: last_failure_snapshot_frame(&failure_snapshots_path),
        errors,
    }
}

pub(crate) fn finalize_live_run(
    paths: &LiveLogPaths,
    input: FinalizeRunInput,
) -> Result<FinalizeRunOutcome, String> {
    ensure_log_dirs(paths).map_err(|err| format!("failed to ensure log dirs: {err}"))?;
    let run_dir = paths.run_dir(&input.run_id);
    std::fs::create_dir_all(&run_dir)
        .map_err(|err| format!("failed to create run dir '{}': {err}", run_dir.display()))?;

    let classification_label = classify_run(&input);
    let is_clean = is_clean_label(&classification_label);
    let retain_replay = matches!(
        classification_label.as_str(),
        "strict_fail" | "survey_tainted" | "loss_tainted" | "victory_tainted"
    );
    let raw_present = copy_if_exists(&paths.current_raw(), &run_dir.join("raw.jsonl"))?;
    let focus_present = copy_if_exists(&paths.current_focus(), &run_dir.join("focus.txt"))?;
    let signatures_present = copy_if_exists(
        &paths.current_signatures(),
        &run_dir.join("signatures.jsonl"),
    )?;
    let combat_suspects_present = copy_if_nonempty(
        &paths.current_combat_suspects(),
        &run_dir.join("combat_suspects.jsonl"),
    )?;
    let failure_snapshots_present = copy_if_exists(
        &paths.current_failure_snapshots(),
        &run_dir.join("failure_snapshots.jsonl"),
    )?;
    let debug_present = copy_if_exists(&paths.current_debug(), &run_dir.join("debug.txt"))?;
    let reward_audit_present = copy_if_nonempty(
        &paths.current_reward_audit(),
        &run_dir.join("reward_audit.jsonl"),
    )?;
    let event_audit_present = copy_if_exists(
        &paths.current_event_audit(),
        &run_dir.join("event_audit.jsonl"),
    )?;
    let sidecar_shadow_present = copy_if_nonempty(
        &paths.current_sidecar_shadow(),
        &run_dir.join("sidecar_shadow.jsonl"),
    )?;
    let watch_audit_present = if input.watch_enabled {
        copy_if_nonempty(
            &paths.current_watch_audit(),
            &run_dir.join("watch_audit.jsonl"),
        )?
    } else {
        false
    };
    let watch_noncombat_present = if input.watch_enabled {
        copy_if_nonempty(
            &paths.current_watch_noncombat(),
            &run_dir.join("watch_noncombat.jsonl"),
        )?
    } else {
        false
    };
    let replay_present = if retain_replay {
        copy_if_exists(&paths.current_replay(), &run_dir.join("replay.json"))?
    } else {
        false
    };

    let manifest = LiveRunManifest {
        run_id: input.run_id.clone(),
        timestamp: input.timestamp.clone(),
        build_tag: input.build_tag,
        parity_mode: input.parity_mode,
        watch_enabled: input.watch_enabled,
        session_exit_reason: input.session_exit_reason,
        classification_label: classification_label.clone(),
        profile: load_profile_metadata(),
        provenance: runtime_provenance(),
        counts: LiveRunCounts {
            engine_bugs: input.engine_bug_total,
            content_gaps: input.content_gap_total,
            timing_diffs: input.timing_diff_total,
            replay_failures: input.replay_failures,
        },
        artifacts: LiveRunArtifacts {
            raw: Some(LiveArtifactRecord::new("raw.jsonl", raw_present)),
            focus: Some(LiveArtifactRecord::new("focus.txt", focus_present)),
            signatures: Some(LiveArtifactRecord::new(
                "signatures.jsonl",
                signatures_present,
            )),
            combat_suspects: Some(LiveArtifactRecord::new(
                "combat_suspects.jsonl",
                combat_suspects_present,
            )),
            failure_snapshots: Some(LiveArtifactRecord::new(
                "failure_snapshots.jsonl",
                failure_snapshots_present,
            )),
            debug: Some(LiveArtifactRecord::new("debug.txt", debug_present)),
            replay: Some(LiveArtifactRecord::new("replay.json", replay_present)),
            reward_audit: Some(LiveArtifactRecord::new(
                "reward_audit.jsonl",
                reward_audit_present,
            )),
            event_audit: Some(LiveArtifactRecord::new(
                "event_audit.jsonl",
                event_audit_present,
            )),
            sidecar_shadow: Some(LiveArtifactRecord::new(
                "sidecar_shadow.jsonl",
                sidecar_shadow_present,
            )),
            validation: Some(LiveArtifactRecord::new("validation.json", false)),
            watch_audit: Some(LiveArtifactRecord::new(
                "watch_audit.jsonl",
                watch_audit_present,
            )),
            watch_noncombat: Some(LiveArtifactRecord::new(
                "watch_noncombat.jsonl",
                watch_noncombat_present,
            )),
        },
        retention: LiveRetentionFlags {
            pinned: false,
            cache_only: is_clean,
            retain_debug: debug_present,
            retain_replay,
        },
        validation: None,
    };

    let mut manifest = manifest;
    let manifest_path = write_manifest(&run_dir.join("manifest.json"), &manifest)?;
    let validation = validate_run_artifacts(&run_dir, &manifest);
    let validation_path = run_dir.join("validation.json");
    let validation_text = serde_json::to_string_pretty(&validation)
        .map_err(|err| format!("failed to serialize validation: {err}"))?;
    std::fs::write(&validation_path, validation_text).map_err(|err| {
        format!(
            "failed to write validation '{}': {err}",
            validation_path.display()
        )
    })?;
    let _ = std::fs::copy(&validation_path, paths.current_validation());
    manifest.validation = Some(validation.clone());
    manifest.artifacts.validation = Some(LiveArtifactRecord::new("validation.json", true));
    rewrite_manifest(&manifest_path, &manifest)?;
    let _ = write_manifest(&paths.current_manifest(), &manifest);
    let gc_summary = gc_runs(paths)?;

    Ok(FinalizeRunOutcome {
        run_dir,
        manifest_path,
        classification_label,
        validation_status: validation.status,
        gc_summary: format!(
            "pruned_runs={} pruned_debug={} pruned_replay={} pruned_watch={}",
            gc_summary.pruned_run_artifacts,
            gc_summary.pruned_debug,
            gc_summary.pruned_replay,
            gc_summary.pruned_watch
        ),
    })
}

fn classify_run(input: &FinalizeRunInput) -> String {
    let tainted = input.engine_bug_total > 0
        || input.content_gap_total > 0
        || input.timing_diff_total > 0
        || input.replay_failures > 0
        || input.session_exit_reason == "PARITY_FAIL";

    if input.final_victory {
        return if tainted {
            "victory_tainted".to_string()
        } else {
            "victory_clean".to_string()
        };
    }
    if input.game_over_seen {
        return if tainted {
            "loss_tainted".to_string()
        } else {
            "loss_clean".to_string()
        };
    }
    if input.parity_mode.eq_ignore_ascii_case("strict") {
        if tainted {
            "strict_fail".to_string()
        } else {
            "strict_ok".to_string()
        }
    } else if tainted {
        "survey_tainted".to_string()
    } else {
        "survey_clean".to_string()
    }
}

fn copy_if_exists(source: &Path, target: &Path) -> Result<bool, String> {
    if !source.exists() {
        return Ok(false);
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create '{}': {err}", parent.display()))?;
    }
    std::fs::copy(source, target).map_err(|err| {
        format!(
            "failed to copy '{}' to '{}': {err}",
            source.display(),
            target.display()
        )
    })?;
    Ok(true)
}

fn copy_if_nonempty(source: &Path, target: &Path) -> Result<bool, String> {
    if !std::fs::metadata(source)
        .map(|meta| meta.is_file() && meta.len() > 0)
        .unwrap_or(false)
    {
        return Ok(false);
    }
    copy_if_exists(source, target)
}

pub(crate) fn verify_replay_counts(replay_path: &Path) -> Result<(usize, usize), String> {
    let replay = crate::diff::replay::load_live_session_replay_path(replay_path)?;
    let view = derive_combat_replay_view(&replay);
    let report = verify_combat_replay_view(&view, false)?;
    let mut timing = 0;
    for failure in &report.failures {
        for diff in &failure.diffs {
            if diff.category.to_string() == "TIMING" {
                timing += 1;
            }
        }
    }
    Ok((report.failures.len(), timing))
}
