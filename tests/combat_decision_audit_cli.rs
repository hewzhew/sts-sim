use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_178() -> &'static Path {
    Path::new(r"d:\rust\sts_simulator\tests\decision_audit\hexaghost_frame_178.json")
}

fn raw_214122() -> &'static Path {
    Path::new(r"d:\rust\sts_simulator\logs\raw\live_comm_raw_20260412_214122.jsonl")
}

#[test]
fn audit_fixture_cli_prints_summary() {
    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "audit-fixture",
            "--fixture",
            fixture_178().to_str().unwrap(),
            "--top-k",
            "1",
        ])
        .output()
        .expect("run audit-fixture");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("decision audit: hexaghost_frame_178"));
    assert!(stdout.contains("chosen_first_move=Play #1 Strike+ @1"));
}

#[test]
fn audit_frame_cli_resolves_hexaghost_frame_203() {
    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "audit-frame",
            "--raw",
            raw_214122().to_str().unwrap(),
            "--frame",
            "203",
            "--top-k",
            "1",
        ])
        .output()
        .expect("run audit-frame");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("before_frame_id=Some(203)"));
    assert!(stdout.contains("chosen_first_move=Play #3 Strike+ @1"));
}

#[test]
fn export_preferences_cli_writes_nonempty_jsonl() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let out_path = std::env::temp_dir().join(format!("combat_pref_{stamp}.jsonl"));
    let summary_path = std::env::temp_dir().join(format!("combat_pref_{stamp}.summary.json"));

    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "export-preferences",
            "--raw",
            raw_214122().to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--summary-out",
            summary_path.to_str().unwrap(),
            "--limit",
            "8",
            "--min-incoming",
            "6",
            "--max-hp-ratio",
            "0.6",
        ])
        .output()
        .expect("run export-preferences");
    assert!(output.status.success());
    let out = std::fs::read_to_string(&out_path).expect("preference jsonl");
    assert!(!out.trim().is_empty());
    let summary = std::fs::read_to_string(&summary_path).expect("summary json");
    assert!(summary.contains("\"exported_samples\""));
}

#[test]
fn export_preference_seed_set_cli_writes_requested_frame_bundle() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let out_path = std::env::temp_dir().join(format!("combat_pref_seed_{stamp}.jsonl"));
    let summary_path = std::env::temp_dir().join(format!("combat_pref_seed_{stamp}.summary.json"));

    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "export-preference-seed-set",
            "--raw",
            raw_214122().to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--summary-out",
            summary_path.to_str().unwrap(),
            "--frames",
            "153,178,202",
        ])
        .output()
        .expect("run export-preference-seed-set");
    assert!(output.status.success());
    let out = std::fs::read_to_string(&out_path).expect("preference seed jsonl");
    assert!(!out.trim().is_empty());
    let summary = std::fs::read_to_string(&summary_path).expect("seed summary json");
    assert!(summary.contains("\"requested_frames\""));
    assert!(summary.contains("\"exported_frame_ids\""));
}

#[test]
fn summarize_preferences_cli_reports_motifs() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let out_path = std::env::temp_dir().join(format!("combat_pref_seed_{stamp}.jsonl"));
    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "export-preference-seed-set",
            "--raw",
            raw_214122().to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--frames",
            "153,178,202",
        ])
        .output()
        .expect("run export-preference-seed-set");
    assert!(output.status.success());

    let summary_output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "summarize-preferences",
            "--in",
            out_path.to_str().unwrap(),
            "--top-examples",
            "3",
        ])
        .output()
        .expect("run summarize-preferences");
    assert!(summary_output.status.success());
    let stdout = String::from_utf8_lossy(&summary_output.stdout);
    assert!(stdout.contains("preference motif summary"));
    assert!(stdout.contains("motifs="));
    assert!(stdout.contains("top_examples:"));
}

#[test]
fn diagnose_search_frame_cli_prints_ranked_moves() {
    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "diagnose-search-frame",
            "--raw",
            raw_214122().to_str().unwrap(),
            "--frame",
            "178",
            "--depth-limit",
            "5",
            "--top-k",
            "3",
        ])
        .output()
        .expect("run diagnose-search-frame");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("search diagnosis: frame 178"));
    assert!(stdout.contains("top_moves:"));
    assert!(stdout.contains("reduced_legal_moves="));
    assert!(stdout.contains("equivalence_mode=safe"));
    assert!(stdout.contains("survival_window="));
    assert!(stdout.contains("exhaust_evidence="));
    assert!(stdout.contains("exhaust_block="));
    assert!(stdout.contains("exhaust_draw="));
}

#[test]
fn diagnose_search_frame_cli_prefers_sword_boomerang_on_178() {
    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "diagnose-search-frame",
            "--raw",
            raw_214122().to_str().unwrap(),
            "--frame",
            "178",
            "--depth-limit",
            "5",
            "--top-k",
            "5",
        ])
        .output()
        .expect("run diagnose-search-frame");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("chosen_move=Play #4 Sword Boomerang+"));
}

#[test]
fn diagnose_search_frame_cli_emits_profile_json() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let out_path = std::env::temp_dir().join(format!("search_profile_{stamp}.json"));

    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "diagnose-search-frame",
            "--raw",
            raw_214122().to_str().unwrap(),
            "--frame",
            "178",
            "--depth-limit",
            "5",
            "--emit-profile-json",
            out_path.to_str().unwrap(),
        ])
        .output()
        .expect("run diagnose-search-frame");
    assert!(output.status.success());
    let json = std::fs::read_to_string(&out_path).expect("profile json");
    assert!(json.contains("\"search_total_ms\""));
    assert!(json.contains("\"chosen_move\""));
}

#[test]
fn export_search_baseline_cli_writes_requested_frames() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let out_path = std::env::temp_dir().join(format!("search_baseline_{stamp}.json"));

    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "export-search-baseline",
            "--raw",
            raw_214122().to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--frames",
            "178,203",
            "--depth-limit",
            "5",
        ])
        .output()
        .expect("run export-search-baseline");
    assert!(output.status.success());
    let json = std::fs::read_to_string(&out_path).expect("baseline json");
    assert!(json.contains("\"frame\":178") || json.contains("\"frame\": 178"));
    assert!(json.contains("\"frame\":203") || json.contains("\"frame\": 203"));
}

#[test]
fn audit_recent_live_session_cli_reports_shortlist_and_suggestions() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_millis();
    let suspect_path = std::env::temp_dir().join(format!("combat_suspects_{stamp}.jsonl"));
    std::fs::write(
        &suspect_path,
        r#"{"frame_count":178,"response_id":178,"state_frame_id":178,"chosen_move":"Play #3 Strike+ @1","heuristic_move":"Play #3 Strike+ @1","search_move":"Play #4 Sword Boomerang+","top_gap":1.5,"sequence_bonus":-4288000.0,"survival_window_delta":-4288000.0,"exhaust_evidence_delta":0.0,"realized_exhaust_block":0,"realized_exhaust_draw":0,"heuristic_search_gap":true,"large_sequence_bonus":true,"tight_root_gap":true,"reasons":["heuristic_search_gap","large_sequence_bonus","tight_root_gap"]}"#,
    )
    .expect("write suspect jsonl");

    let output = Command::new(env!("CARGO_BIN_EXE_combat_decision_audit"))
        .args([
            "audit-recent-live-session",
            "--raw",
            raw_214122().to_str().unwrap(),
            "--suspects",
            suspect_path.to_str().unwrap(),
            "--depth-limit",
            "5",
            "--top-k",
            "2",
            "--limit",
            "3",
        ])
        .output()
        .expect("run audit-recent-live-session");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("recent live session audit"));
    assert!(stdout.contains("shortlist:"));
    assert!(stdout.contains("frame=178"));
    assert!(stdout.contains("survival_window="));
    assert!(stdout.contains("exhaust_evidence="));
    assert!(stdout.contains("next_step:"));
}
