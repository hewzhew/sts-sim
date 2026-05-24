use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::eval::combat_capture::{capture_combat_position_v1, save_combat_capture_v1};
use crate::eval::run_control::registry::{add_case_to_benchmark_registry, BenchmarkCasePaths};
use crate::fixtures::combat_start_spec::{compile_combat_start_spec, CombatStartSpec};
use crate::sim::combat::CombatPosition;

use super::{audit_artifacts, ArtifactAuditStatus};

#[test]
fn audit_accepts_registered_capture_suite() {
    let root = unique_temp_dir("artifact_doctor_registered_capture");
    let paths = BenchmarkCasePaths::for_case(&root, "jaw");
    let capture = capture_combat_position_v1(Some("jaw".to_string()), &jaw_worm_position())
        .expect("capture should build");
    save_combat_capture_v1(&paths.capture_path, &capture).expect("capture should save");
    add_case_to_benchmark_registry(&root, "jaw").expect("registry should update");

    let report = audit_artifacts(&root);

    assert_eq!(report.summary.suites_found, 1);
    assert_eq!(report.summary.cases_found, 1);
    assert_eq!(report.summary.checks_error, 0);
    assert!(report.checks.iter().any(|check| {
        check.check_id == "case:root:jaw:capture_load"
            && check.status == ArtifactAuditStatus::Ok
            && check.artifact_hash.is_some()
    }));
    assert!(report.checks.iter().any(|check| {
        check.check_id == "case:root:jaw:search_input_load"
            && check.status == ArtifactAuditStatus::Ok
            && check.code == "search_input_load_ok"
    }));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn audit_warns_about_unregistered_baseline_files() {
    let root = unique_temp_dir("artifact_doctor_unregistered_baseline");
    let paths = BenchmarkCasePaths::for_case(&root, "jaw");
    let capture = capture_combat_position_v1(Some("jaw".to_string()), &jaw_worm_position())
        .expect("capture should build");
    save_combat_capture_v1(&paths.capture_path, &capture).expect("capture should save");
    add_case_to_benchmark_registry(&root, "jaw").expect("registry should update");
    fs::create_dir_all(paths.baseline_path.parent().unwrap()).expect("baseline dir");
    fs::write(
        &paths.baseline_path,
        r#"{
            "schema_name": "CombatBaselineOutcomeV1",
            "schema_version": 1,
            "case_id": "jaw",
            "terminal": "win",
            "start_hp": 80,
            "final_hp": 70,
            "hp_loss": 10,
            "turns": 4,
            "potions_used": 0,
            "potions_discarded": 0,
            "cards_played": 9
        }"#,
    )
    .expect("baseline should write");

    let report = audit_artifacts(&root);

    assert_eq!(report.summary.checks_error, 0);
    assert!(report.checks.iter().any(|check| {
        check.code == "baseline_file_not_registered" && check.status == ArtifactAuditStatus::Warn
    }));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn audit_reports_missing_root_without_panicking() {
    let root = unique_temp_dir("artifact_doctor_missing_root");

    let report = audit_artifacts(&root);

    assert_eq!(report.summary.checks_error, 1);
    assert!(report
        .checks
        .iter()
        .any(|check| check.code == "root_missing"));
}

fn jaw_worm_position() -> CombatPosition {
    let spec: CombatStartSpec = serde_json::from_str(
        r#"{
            "name": "jaw_worm_starter",
            "player_class": "Ironclad",
            "ascension_level": 0,
            "encounter_id": "JawWorm",
            "room_type": "monster",
            "seed": 1,
            "player_current_hp": 80,
            "player_max_hp": 80,
            "master_deck": [
                {"id": "Strike_R", "count": 5},
                {"id": "Defend_R", "count": 4},
                "Bash"
            ]
        }"#,
    )
    .expect("test start spec should parse");
    let (engine, combat) =
        compile_combat_start_spec(&spec).expect("test start spec should compile");
    CombatPosition::new(engine, combat)
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{label}_{}_{}", std::process::id(), nanos))
}
