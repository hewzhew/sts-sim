use super::{
    derive_shuffle_seed_v1, load_and_resolve_combat_lab_spec_v1, preflight_combat_lab_scenario_v1,
    profile_config_v1, CombatLabArtifactStoreV1, CombatLabManifestV1, CombatLabShuffleGeneratorV1,
    CombatLabShuffleScheduleV1, CombatLabSpecV1,
};
use crate::eval::fingerprint::combat_state_fingerprint_v1;
use crate::fixtures::combat_start_spec::compile_combat_start_spec;
use serde_json::json;
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;

#[test]
fn artifact_new_run_writes_manifest_before_cells() {
    let (directory, resolved) = resolved_lab_fixture("artifact_new_run_manifest_first");
    let output = directory.join("run");
    let expected = artifact_manifest(resolved, 123);

    let store = CombatLabArtifactStoreV1::create_or_resume(&output, expected)
        .expect("create artifact store");

    assert!(output.join("manifest.json").is_file());
    assert!(!output.join("cells.jsonl").exists());
    assert!(store.cells().is_empty());
    assert_eq!(store.manifest().created_at_unix_ms, 123);
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn resume_does_not_duplicate_cells() {
    let (directory, resolved) = resolved_lab_fixture("resume_does_not_duplicate_cells");
    let output = directory.join("run");
    let manifest = artifact_manifest(resolved.clone(), 100);
    let mut store = CombatLabArtifactStoreV1::create_or_resume(&output, manifest.clone())
        .expect("create artifact store");
    let cell = artifact_cell(&resolved, 0);

    store.append_cell(&cell).expect("append first cell");
    let duplicate = store
        .append_cell(&cell)
        .expect_err("duplicate cell key must be rejected");
    assert!(duplicate.contains("duplicate cell key"), "{duplicate}");
    drop(store);

    let reopened =
        CombatLabArtifactStoreV1::create_or_resume(&output, artifact_manifest(resolved, 200))
            .expect("resume artifact store");
    assert_eq!(reopened.cells().len(), 1);
    assert!(reopened.contains_cell(&cell.cell_key));

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn artifact_truncated_final_line_is_ignored_and_pending_cell_can_append() {
    let (directory, resolved) = resolved_lab_fixture("artifact_truncated_final_line");
    let output = directory.join("run");
    let manifest = artifact_manifest(resolved.clone(), 100);
    let first = artifact_cell(&resolved, 0);
    let pending = artifact_cell(&resolved, 1);
    let mut store = CombatLabArtifactStoreV1::create_or_resume(&output, manifest.clone())
        .expect("create artifact store");
    store.append_cell(&first).expect("append first cell");
    drop(store);

    let pending_bytes = serde_json::to_vec(&pending).expect("serialize pending cell");
    let partial_len = pending_bytes.len() / 2;
    let mut journal = fs::OpenOptions::new()
        .append(true)
        .open(output.join("cells.jsonl"))
        .expect("open journal for interrupted append");
    journal
        .write_all(&pending_bytes[..partial_len])
        .expect("write partial final line");
    journal.sync_data().expect("sync partial final line");
    drop(journal);

    let mut resumed = CombatLabArtifactStoreV1::create_or_resume(&output, manifest.clone())
        .expect("ignore partial final line");
    assert_eq!(resumed.cells().len(), 1);
    assert!(resumed.contains_cell(&first.cell_key));
    assert!(!resumed.contains_cell(&pending.cell_key));
    resumed
        .append_cell(&pending)
        .expect("append exact pending cell after tail repair");
    drop(resumed);

    let reopened = CombatLabArtifactStoreV1::create_or_resume(&output, manifest)
        .expect("reopen repaired journal");
    assert_eq!(reopened.cells().len(), 2);
    assert!(reopened.contains_cell(&pending.cell_key));
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn artifact_malformed_newline_terminated_journal_entry_is_error() {
    let (directory, resolved) = resolved_lab_fixture("artifact_malformed_terminated_line");
    let output = directory.join("run");
    let manifest = artifact_manifest(resolved, 100);
    drop(
        CombatLabArtifactStoreV1::create_or_resume(&output, manifest.clone())
            .expect("create artifact store"),
    );
    fs::write(output.join("cells.jsonl"), b"{not-json}\n")
        .expect("write malformed terminated entry");

    let error = CombatLabArtifactStoreV1::create_or_resume(&output, manifest)
        .err()
        .expect("terminated malformed entry must fail resume");
    assert!(error.contains("line 1"), "{error}");
    assert!(error.contains("malformed"), "{error}");
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn artifact_empty_newline_terminated_journal_entry_is_error() {
    let (directory, resolved) = resolved_lab_fixture("artifact_empty_terminated_line");
    let output = directory.join("run");
    let manifest = artifact_manifest(resolved, 100);
    drop(
        CombatLabArtifactStoreV1::create_or_resume(&output, manifest.clone())
            .expect("create artifact store"),
    );
    fs::write(output.join("cells.jsonl"), b"\n").expect("write empty terminated entry");

    let error = CombatLabArtifactStoreV1::create_or_resume(&output, manifest)
        .err()
        .expect("empty terminated entry must fail resume");
    assert!(error.contains("line 1"), "{error}");
    assert!(error.contains("malformed"), "{error}");
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn artifact_checkpoint_disagreement_is_repaired_from_journal() {
    let (directory, resolved) = resolved_lab_fixture("artifact_checkpoint_repair");
    let output = directory.join("run");
    let manifest = artifact_manifest(resolved.clone(), 100);
    let cell = artifact_cell(&resolved, 0);
    let mut store = CombatLabArtifactStoreV1::create_or_resume(&output, manifest.clone())
        .expect("create artifact store");
    store.append_cell(&cell).expect("append cell");
    store
        .checkpoint_sample_boundary(1)
        .expect("write checkpoint");
    drop(store);

    let bad_checkpoint = super::CombatLabCheckpointV1 {
        schema_version: 1,
        journal_digest: "invented".to_string(),
        completed_cell_keys: BTreeSet::from(["ghost-cell".to_string()]),
        next_sample_hint: 99,
    };
    fs::write(
        output.join("checkpoint.json"),
        serde_json::to_vec(&bad_checkpoint).expect("serialize bad checkpoint"),
    )
    .expect("replace checkpoint with disagreement");

    let resumed = CombatLabArtifactStoreV1::create_or_resume(&output, manifest)
        .expect("repair checkpoint from journal");
    assert!(resumed.contains_cell(&cell.cell_key));
    let repaired: super::CombatLabCheckpointV1 = serde_json::from_slice(
        &fs::read(output.join("checkpoint.json")).expect("read repaired checkpoint"),
    )
    .expect("parse repaired checkpoint");
    assert_eq!(
        repaired.completed_cell_keys,
        BTreeSet::from([cell.cell_key])
    );
    assert_eq!(
        repaired.journal_digest,
        artifact_journal_digest(&fs::read(output.join("cells.jsonl")).expect("read journal"))
    );
    assert_ne!(repaired.next_sample_hint, 99);
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn resume_mismatch_reports_profile_budget_scenario_and_code_fields() {
    let (directory, resolved) = resolved_lab_fixture("resume_field_mismatch");
    let output = directory.join("run");
    let manifest = artifact_manifest(resolved, 100);
    drop(
        CombatLabArtifactStoreV1::create_or_resume(&output, manifest.clone())
            .expect("create artifact store"),
    );

    let mut profile = manifest.clone();
    profile.resolved_spec.profiles[0].profile_hash = "changed-profile".to_string();
    profile.resolved_spec.experiment_hash = "changed-experiment".to_string();
    profile.experiment_hash = "changed-experiment".to_string();
    assert_resume_mismatch(&output, profile, "resolved_spec.profiles[0].profile_hash");

    let mut budget = manifest.clone();
    budget.resolved_spec.budget_hash = "changed-budget".to_string();
    budget.resolved_spec.experiment_hash = "changed-experiment".to_string();
    budget.experiment_hash = "changed-experiment".to_string();
    assert_resume_mismatch(&output, budget, "resolved_spec.budget_hash");

    let mut scenario = manifest.clone();
    scenario.resolved_spec.scenario_hash = "changed-scenario".to_string();
    scenario.resolved_spec.experiment_hash = "changed-experiment".to_string();
    scenario.experiment_hash = "changed-experiment".to_string();
    assert_resume_mismatch(&output, scenario, "resolved_spec.scenario_hash");

    let mut commit = manifest.clone();
    commit.source_identity.git_commit = Some("changed-commit".to_string());
    assert_resume_mismatch(&output, commit, "source_identity.git_commit");

    let mut dirty = manifest.clone();
    dirty.source_identity.git_dirty = Some(true);
    assert_resume_mismatch(&output, dirty, "source_identity.git_dirty");

    for (field, mut environment) in [
        ("environment.package_version", manifest.clone()),
        ("environment.target_os", manifest.clone()),
        ("environment.target_arch", manifest.clone()),
    ] {
        match field {
            "environment.package_version" => {
                environment.environment.package_version = "changed".to_string()
            }
            "environment.target_os" => environment.environment.target_os = "changed".to_string(),
            "environment.target_arch" => {
                environment.environment.target_arch = "changed".to_string()
            }
            _ => unreachable!(),
        }
        assert_resume_mismatch(&output, environment, field);
    }

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn resume_target_extension_returns_only_new_or_missing_cells() {
    let (directory, resolved) = resolved_lab_fixture("resume_target_extension");
    let output = directory.join("run");
    let manifest = artifact_manifest(resolved.clone(), 100);
    let requested = (0..3)
        .map(|sample_index| artifact_cell(&resolved, sample_index))
        .collect::<Vec<_>>();
    let mut store = CombatLabArtifactStoreV1::create_or_resume(&output, manifest.clone())
        .expect("create artifact store");
    store.append_cell(&requested[0]).expect("append old cell");
    drop(store);

    let resumed = CombatLabArtifactStoreV1::create_or_resume(&output, manifest)
        .expect("resume for larger requested target");
    let missing = requested
        .iter()
        .filter(|cell| !resumed.contains_cell(&cell.cell_key))
        .map(|cell| cell.sample_index)
        .collect::<Vec<_>>();
    assert_eq!(missing, vec![1, 2]);
    assert!(resumed.contains_cell(&requested[0].cell_key));
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn resume_target_decrease_deletes_no_journal_cells() {
    let (directory, resolved) = resolved_lab_fixture("resume_target_decrease");
    let output = directory.join("run");
    let manifest = artifact_manifest(resolved.clone(), 100);
    let cells = (0..3)
        .map(|sample_index| artifact_cell(&resolved, sample_index))
        .collect::<Vec<_>>();
    let mut store = CombatLabArtifactStoreV1::create_or_resume(&output, manifest.clone())
        .expect("create artifact store");
    for cell in &cells {
        store.append_cell(cell).expect("append cell");
    }
    drop(store);

    let resumed = CombatLabArtifactStoreV1::create_or_resume(&output, manifest)
        .expect("resume for smaller requested target");
    let requested_sample_count = 1_u64;
    let pending_within_decreased_target = cells
        .iter()
        .filter(|cell| cell.sample_index < requested_sample_count)
        .filter(|cell| !resumed.contains_cell(&cell.cell_key))
        .count();
    assert_eq!(pending_within_decreased_target, 0);
    assert_eq!(resumed.cells().len(), 3);
    assert!(cells
        .iter()
        .all(|cell| resumed.contains_cell(&cell.cell_key)));
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn artifact_atomic_json_replacement_overwrites_existing_destination_on_windows() {
    let (directory, resolved) = resolved_lab_fixture("artifact_replace_existing");
    let output = directory.join("run");
    let store =
        CombatLabArtifactStoreV1::create_or_resume(&output, artifact_manifest(resolved, 100))
            .expect("create artifact store");
    store
        .write_summary(&json!({"generation": 1}))
        .expect("write first summary");
    store
        .write_summary(&json!({"generation": 2}))
        .expect("replace existing summary");

    let summary: serde_json::Value = serde_json::from_slice(
        &fs::read(output.join("summary.json")).expect("read replaced summary"),
    )
    .expect("parse replaced summary");
    assert_eq!(summary["generation"], 2);
    assert!(fs::read_dir(&output)
        .expect("list artifact directory")
        .all(|entry| !entry
            .expect("read artifact entry")
            .file_name()
            .to_string_lossy()
            .contains(".tmp-")));
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn sample_is_shared_across_profiles() {
    let directory = test_directory("sample_is_shared_across_profiles");
    fs::create_dir_all(&directory).expect("create test directory");
    let mut lab = valid_lab_spec_value();
    let mut second_profile = lab["profiles"][0].clone();
    second_profile["id"] = json!("comparison");
    second_profile["label"] = json!("Comparison");
    lab["profiles"] = json!([lab["profiles"][0].clone(), second_profile]);
    write_json(&directory.join("start.json"), &valid_start_spec_value());
    write_json(&directory.join("lab.json"), &lab);
    let resolved = load_and_resolve_combat_lab_spec_v1(&directory.join("lab.json"))
        .expect("resolve laboratory spec");
    let compiler = preflight_combat_lab_scenario_v1(&resolved).expect("preflight scenario");

    let sample = compiler.compile_sample(0).expect("compile sample once");
    let mut profile_starts = resolved
        .profiles
        .iter()
        .map(|_| sample.start.clone())
        .collect::<Vec<_>>();

    assert_eq!(profile_starts.len(), 2);
    assert_eq!(profile_starts[0], profile_starts[1]);
    profile_starts[0].combat.entities.player.current_hp = 1;
    assert_eq!(profile_starts[1], sample.start);

    let baseline = compile_combat_start_spec(&resolved.start_spec_snapshot)
        .map(|(engine, combat)| crate::sim::combat::CombatPosition::new(engine, combat))
        .expect("compile no-override baseline");
    let baseline_fingerprint = combat_state_fingerprint_v1(&baseline);
    let changed_rng_streams = baseline_fingerprint
        .rng_boundary
        .streams
        .iter()
        .zip(&sample.state_fingerprint.rng_boundary.streams)
        .filter_map(|(baseline, sampled)| (baseline != sampled).then_some(baseline.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(changed_rng_streams, vec!["shuffle_rng"]);

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn sample_invariant_error_names_first_field() {
    let spec: crate::fixtures::combat_start_spec::CombatStartSpec =
        serde_json::from_value(valid_start_spec_value()).expect("parse start spec");
    let baseline = compile_combat_start_spec(&spec)
        .map(|(engine, combat)| crate::sim::combat::CombatPosition::new(engine, combat))
        .expect("compile baseline");
    let mut sampled = baseline.clone();
    sampled.combat.entities.monsters[0].monster_type += 1;
    sampled.combat.entities.monsters[0].current_hp += 1;

    let error = super::scenario::validate_combat_lab_sample_invariants_v1(&baseline, &sampled)
        .expect_err("changed monster identity must fail the invariant check");

    assert!(error.contains("monsters[0].monster_type"), "{error}");
}

#[test]
fn cell_coverage_limits_are_never_resolved_losses() {
    use crate::ai::combat_search_v2::{SearchCoverageStatus, SearchTerminalLabel};

    for coverage in [
        SearchCoverageStatus::NodeBudgetLimited,
        SearchCoverageStatus::TimeBudgetLimited,
        SearchCoverageStatus::FrontierOpen,
    ] {
        for terminal in [SearchTerminalLabel::Unresolved, SearchTerminalLabel::Loss] {
            assert_eq!(
                super::replay::classify_combat_lab_outcome_v1(coverage, Some(terminal), false,),
                super::CombatLabOutcomeClassV1::CoverageLimited
            );
        }
    }
    assert_eq!(
        super::replay::classify_combat_lab_outcome_v1(
            SearchCoverageStatus::Exhaustive,
            Some(SearchTerminalLabel::Loss),
            false,
        ),
        super::CombatLabOutcomeClassV1::ResolvedLoss
    );
}

#[test]
fn cell_replayed_win_carries_metrics_actions_and_draws() {
    use crate::ai::combat_search_v2::{replay_combat_search_witness_line_v1, SearchCoverageStatus};

    let (directory, resolved) = resolved_lab_fixture("cell_replayed_win");
    let (sample, trajectory) = replayable_win_sample();
    let mut stats = crate::ai::combat_search_v2::CombatSearchV2Stats::default();
    stats.nodes_expanded = 11;
    stats.nodes_generated = 17;
    stats.nodes_to_first_win = Some(9);

    let record = super::replay::combat_lab_cell_record_from_trajectory_with_replayer_v1(
        &resolved,
        &sample,
        &resolved.profiles[0],
        SearchCoverageStatus::AcceptedCompleteCandidate,
        Some(&trajectory),
        &stats,
        replay_combat_search_witness_line_v1,
    );

    assert_eq!(
        record.outcome_class,
        super::CombatLabOutcomeClassV1::ResolvedWin
    );
    assert!(record.replay_validated);
    assert_eq!(record.final_hp, Some(80));
    assert_eq!(record.hp_loss, Some(0));
    assert_eq!(record.actions, Some(2));
    assert_eq!(record.action_history.len(), 2);
    assert!(!record.draw_history.is_empty());
    assert!(record.outcome_order_key.is_some());
    assert_eq!(record.expanded_nodes, 11);
    assert_eq!(record.generated_nodes, 17);
    assert_eq!(record.nodes_to_first_win, Some(9));
    assert!(record.search_terminal.is_some());
    assert!(record.coverage_status.is_some());
    assert!(record.error.is_none());
    let trajectory_json = serde_json::to_value(&trajectory).expect("serialize trajectory report");
    assert!(trajectory_json.get("outcome_order_key").is_some());
    assert_eq!(
        crate::ai::combat_search_v2::COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION,
        12
    );

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn cell_replay_mismatch_is_halting_execution_error() {
    use crate::ai::combat_search_v2::SearchCoverageStatus;

    let (directory, resolved) = resolved_lab_fixture("cell_replay_mismatch");
    let (sample, trajectory) = replayable_win_sample();
    let stats = crate::ai::combat_search_v2::CombatSearchV2Stats::default();

    let record = super::replay::combat_lab_cell_record_from_trajectory_with_replayer_v1(
        &resolved,
        &sample,
        &resolved.profiles[0],
        SearchCoverageStatus::NodeBudgetLimited,
        Some(&trajectory),
        &stats,
        |_, _, _| Err("forced replay mismatch".to_string()),
    );

    assert_eq!(
        record.outcome_class,
        super::CombatLabOutcomeClassV1::ExecutionError
    );
    assert!(!record.replay_validated);
    assert!(record.search_terminal.is_some());
    assert!(record.coverage_status.is_some());
    assert!(record.outcome_order_key.is_none());
    let error = record.error.expect("replay mismatch should be recorded");
    assert_eq!(error.stage, super::CombatLabCellErrorStageV1::ExactReplay);
    assert_eq!(error.code, "exact_replay_invariant_mismatch");
    assert!(error.halt_experiment);
    assert!(error.message.contains("forced replay mismatch"));

    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn cell_key_is_stable_and_commits_every_identity_input() {
    let baseline = super::combat_lab_cell_key_v1("experiment", 3, 5, "profile", "p-hash", "b-hash");
    assert_eq!(
        baseline,
        super::combat_lab_cell_key_v1("experiment", 3, 5, "profile", "p-hash", "b-hash")
    );
    for changed in [
        super::combat_lab_cell_key_v1("other", 3, 5, "profile", "p-hash", "b-hash"),
        super::combat_lab_cell_key_v1("experiment", 4, 5, "profile", "p-hash", "b-hash"),
        super::combat_lab_cell_key_v1("experiment", 3, 6, "profile", "p-hash", "b-hash"),
        super::combat_lab_cell_key_v1("experiment", 3, 5, "other", "p-hash", "b-hash"),
        super::combat_lab_cell_key_v1("experiment", 3, 5, "profile", "other", "b-hash"),
        super::combat_lab_cell_key_v1("experiment", 3, 5, "profile", "p-hash", "other"),
    ] {
        assert_ne!(baseline, changed);
    }
}

#[test]
fn schedule_is_frozen() {
    let schedule = CombatLabShuffleScheduleV1 {
        generator: CombatLabShuffleGeneratorV1::SplitMix64V1,
        seed: 42,
    };

    assert_eq!(
        derive_shuffle_seed_v1(&schedule, 0),
        13_679_457_532_755_275_413
    );
    assert_eq!(
        derive_shuffle_seed_v1(&schedule, 1),
        2_949_826_092_126_892_291
    );
    assert_eq!(
        derive_shuffle_seed_v1(&schedule, 2),
        5_139_283_748_462_763_858
    );
}

#[test]
fn profile_local_resource_budget_is_rejected() {
    let mut value = valid_lab_spec_value();
    value["profiles"][0]["max_nodes"] = json!(123);

    let error = serde_json::from_value::<CombatLabSpecV1>(value)
        .expect_err("profile-local resource budgets must be rejected");

    assert!(error.to_string().contains("unknown field `max_nodes`"));
}

#[test]
fn empty_profile_list_is_rejected() {
    let directory = test_directory("empty_profiles");
    fs::create_dir_all(&directory).expect("create test directory");
    let mut value = valid_lab_spec_value();
    value["profiles"] = json!([]);
    let lab_spec_path = directory.join("lab.json");
    fs::write(
        &lab_spec_path,
        serde_json::to_vec(&value).expect("serialize lab spec"),
    )
    .expect("write lab spec");

    let error = load_and_resolve_combat_lab_spec_v1(&lab_spec_path)
        .expect_err("an experiment must contain at least one profile");

    assert!(error.contains("at least one profile"), "{error}");
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn duplicate_profile_ids_are_rejected() {
    let directory = test_directory("duplicate_profiles");
    fs::create_dir_all(&directory).expect("create test directory");
    let mut value = valid_lab_spec_value();
    let profile = value["profiles"][0].clone();
    value["profiles"] = json!([profile.clone(), profile]);
    let lab_spec_path = directory.join("lab.json");
    fs::write(
        &lab_spec_path,
        serde_json::to_vec(&value).expect("serialize lab spec"),
    )
    .expect("write lab spec");

    let error = load_and_resolve_combat_lab_spec_v1(&lab_spec_path)
        .expect_err("profile IDs must be unique");

    assert!(error.contains("duplicate profile id 'baseline'"), "{error}");
    fs::remove_dir_all(directory).expect("remove test directory");
}

#[test]
fn canonical_hashes_ignore_json_key_order_and_provenance_paths() {
    let first_directory = test_directory("canonical_hashes_first");
    let second_directory = test_directory("canonical_hashes_second");
    fs::create_dir_all(&first_directory).expect("create first test directory");
    fs::create_dir_all(&second_directory).expect("create second test directory");
    let lab_value = valid_lab_spec_value();
    let start_value = valid_start_spec_value();
    let ordinary_lab_json = serde_json::to_string_pretty(&lab_value).expect("serialize lab spec");
    let reordered_lab_json = json_with_reversed_object_keys(&lab_value);
    let ordinary_start_json =
        serde_json::to_string_pretty(&start_value).expect("serialize start spec");
    let reordered_start_json = json_with_reversed_object_keys(&start_value);
    assert_ne!(ordinary_lab_json, reordered_lab_json);
    assert_ne!(ordinary_start_json, reordered_start_json);
    fs::write(first_directory.join("lab.json"), ordinary_lab_json).expect("write first lab");
    fs::write(second_directory.join("lab.json"), reordered_lab_json).expect("write second lab");
    fs::write(first_directory.join("start.json"), ordinary_start_json).expect("write first start");
    fs::write(second_directory.join("start.json"), reordered_start_json)
        .expect("write second start");

    let first = load_and_resolve_combat_lab_spec_v1(&first_directory.join("lab.json"))
        .expect("resolve first spec");
    let second = load_and_resolve_combat_lab_spec_v1(&second_directory.join("lab.json"))
        .expect("resolve reordered spec");

    assert_eq!(first.scenario_hash, second.scenario_hash);
    assert_eq!(
        first.profiles[0].profile_hash,
        second.profiles[0].profile_hash
    );
    assert_eq!(first.budget_hash, second.budget_hash);
    assert_eq!(first.experiment_hash, second.experiment_hash);
    for hash in [
        &first.scenario_hash,
        &first.profiles[0].profile_hash,
        &first.budget_hash,
        &first.experiment_hash,
    ] {
        assert_eq!(hash.len(), 64);
        assert!(
            hash.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "hash must use lowercase hex: {hash}"
        );
    }
    assert_ne!(first.lab_spec_path, second.lab_spec_path);
    assert_ne!(first.start_spec_path, second.start_spec_path);
    fs::remove_dir_all(first_directory).expect("remove first test directory");
    fs::remove_dir_all(second_directory).expect("remove second test directory");
}

#[test]
fn changed_start_snapshot_changes_scenario_and_experiment_hashes() {
    let first_directory = test_directory("scenario_hash_first");
    let second_directory = test_directory("scenario_hash_second");
    fs::create_dir_all(&first_directory).expect("create first test directory");
    fs::create_dir_all(&second_directory).expect("create second test directory");
    let lab_value = valid_lab_spec_value();
    let first_start = valid_start_spec_value();
    let mut second_start = first_start.clone();
    second_start["player_current_hp"] = json!(71);
    write_json(&first_directory.join("lab.json"), &lab_value);
    write_json(&second_directory.join("lab.json"), &lab_value);
    write_json(&first_directory.join("start.json"), &first_start);
    write_json(&second_directory.join("start.json"), &second_start);

    let first = load_and_resolve_combat_lab_spec_v1(&first_directory.join("lab.json"))
        .expect("resolve first spec");
    let second = load_and_resolve_combat_lab_spec_v1(&second_directory.join("lab.json"))
        .expect("resolve second spec");

    assert_ne!(first.scenario_hash, second.scenario_hash);
    assert_ne!(first.experiment_hash, second.experiment_hash);
    assert_eq!(
        first.profiles[0].profile_hash,
        second.profiles[0].profile_hash
    );
    assert_eq!(first.budget_hash, second.budget_hash);
    fs::remove_dir_all(first_directory).expect("remove first test directory");
    fs::remove_dir_all(second_directory).expect("remove second test directory");
}

#[test]
fn changed_profile_policy_changes_profile_and_experiment_hashes() {
    let first_directory = test_directory("profile_hash_first");
    let second_directory = test_directory("profile_hash_second");
    fs::create_dir_all(&first_directory).expect("create first test directory");
    fs::create_dir_all(&second_directory).expect("create second test directory");
    let first_lab = valid_lab_spec_value();
    let mut second_lab = first_lab.clone();
    second_lab["profiles"][0]["rollout_policy"] = json!("turn_beam_no_potion");
    let start = valid_start_spec_value();
    write_json(&first_directory.join("lab.json"), &first_lab);
    write_json(&second_directory.join("lab.json"), &second_lab);
    write_json(&first_directory.join("start.json"), &start);
    write_json(&second_directory.join("start.json"), &start);

    let first = load_and_resolve_combat_lab_spec_v1(&first_directory.join("lab.json"))
        .expect("resolve first spec");
    let second = load_and_resolve_combat_lab_spec_v1(&second_directory.join("lab.json"))
        .expect("resolve second spec");

    assert_ne!(
        first.profiles[0].profile_hash,
        second.profiles[0].profile_hash
    );
    assert_ne!(first.experiment_hash, second.experiment_hash);
    assert_eq!(first.scenario_hash, second.scenario_hash);
    assert_eq!(first.budget_hash, second.budget_hash);
    fs::remove_dir_all(first_directory).expect("remove first test directory");
    fs::remove_dir_all(second_directory).expect("remove second test directory");
}

#[test]
fn every_serialized_profile_policy_is_hashed() {
    let baseline_directory = test_directory("all_profile_policies_baseline");
    fs::create_dir_all(&baseline_directory).expect("create baseline test directory");
    let baseline_lab = valid_lab_spec_value();
    let start = valid_start_spec_value();
    write_json(&baseline_directory.join("lab.json"), &baseline_lab);
    write_json(&baseline_directory.join("start.json"), &start);
    let baseline = load_and_resolve_combat_lab_spec_v1(&baseline_directory.join("lab.json"))
        .expect("resolve baseline spec");

    for (field, changed_value) in [
        ("potion_policy", "all"),
        ("child_rollout_policy", "lazy_on_pop"),
        ("turn_plan_policy", "root_frontier_seed"),
        ("frontier_policy", "round_robin_eval_buckets"),
        ("phase_guard_policy", "champ_split_guard"),
        ("setup_bias_policy", "key_card_online"),
    ] {
        let directory = test_directory(field);
        fs::create_dir_all(&directory).expect("create variant test directory");
        let mut lab = baseline_lab.clone();
        lab["profiles"][0][field] = json!(changed_value);
        write_json(&directory.join("lab.json"), &lab);
        write_json(&directory.join("start.json"), &start);
        let changed = load_and_resolve_combat_lab_spec_v1(&directory.join("lab.json"))
            .expect("resolve changed profile policy");

        assert_ne!(
            baseline.profiles[0].profile_hash, changed.profiles[0].profile_hash,
            "profile hash omitted {field}"
        );
        fs::remove_dir_all(directory).expect("remove variant test directory");
    }

    fs::remove_dir_all(baseline_directory).expect("remove baseline test directory");
}

#[test]
fn changed_common_budget_changes_budget_and_experiment_hashes() {
    let first_directory = test_directory("budget_hash_first");
    let second_directory = test_directory("budget_hash_second");
    fs::create_dir_all(&first_directory).expect("create first test directory");
    fs::create_dir_all(&second_directory).expect("create second test directory");
    let first_lab = valid_lab_spec_value();
    let mut second_lab = first_lab.clone();
    second_lab["common_budget"]["max_nodes"] = json!(1001);
    let start = valid_start_spec_value();
    write_json(&first_directory.join("lab.json"), &first_lab);
    write_json(&second_directory.join("lab.json"), &second_lab);
    write_json(&first_directory.join("start.json"), &start);
    write_json(&second_directory.join("start.json"), &start);

    let first = load_and_resolve_combat_lab_spec_v1(&first_directory.join("lab.json"))
        .expect("resolve first spec");
    let second = load_and_resolve_combat_lab_spec_v1(&second_directory.join("lab.json"))
        .expect("resolve second spec");

    assert_ne!(first.budget_hash, second.budget_hash);
    assert_ne!(first.experiment_hash, second.experiment_hash);
    assert_eq!(first.scenario_hash, second.scenario_hash);
    assert_eq!(
        first.profiles[0].profile_hash,
        second.profiles[0].profile_hash
    );
    fs::remove_dir_all(first_directory).expect("remove first test directory");
    fs::remove_dir_all(second_directory).expect("remove second test directory");
}

#[test]
fn every_common_budget_field_is_hashed() {
    let baseline_directory = test_directory("all_budget_fields_baseline");
    fs::create_dir_all(&baseline_directory).expect("create baseline test directory");
    let baseline_lab = valid_lab_spec_value();
    let start = valid_start_spec_value();
    write_json(&baseline_directory.join("lab.json"), &baseline_lab);
    write_json(&baseline_directory.join("start.json"), &start);
    let baseline = load_and_resolve_combat_lab_spec_v1(&baseline_directory.join("lab.json"))
        .expect("resolve baseline spec");

    for (field, changed_value) in [
        ("max_actions_per_line", json!(101)),
        ("max_engine_steps_per_action", json!(201)),
        ("wall_ms", json!(1)),
        ("stop_on_win_hp_loss_at_most", json!(2)),
        ("min_win_candidates_before_stop", json!(2)),
        ("max_potions_used", json!(1)),
        ("rollout_max_evaluations", json!(11)),
        ("rollout_max_actions", json!(21)),
        ("rollout_beam_width", json!(4)),
        ("turn_plan_probe_max_inner_nodes", json!(5)),
        ("turn_plan_probe_max_end_states", json!(6)),
        ("turn_plan_probe_per_bucket_limit", json!(7)),
    ] {
        let directory = test_directory(field);
        fs::create_dir_all(&directory).expect("create variant test directory");
        let mut lab = baseline_lab.clone();
        lab["common_budget"][field] = changed_value;
        write_json(&directory.join("lab.json"), &lab);
        write_json(&directory.join("start.json"), &start);
        let changed = load_and_resolve_combat_lab_spec_v1(&directory.join("lab.json"))
            .expect("resolve changed common budget");

        assert_ne!(
            baseline.budget_hash, changed.budget_hash,
            "budget hash omitted {field}"
        );
        fs::remove_dir_all(directory).expect("remove variant test directory");
    }

    fs::remove_dir_all(baseline_directory).expect("remove baseline test directory");
}

#[test]
fn profile_identity_is_excluded_from_profile_hash_but_included_in_experiment_hash() {
    let first_directory = test_directory("profile_identity_first");
    let second_directory = test_directory("profile_identity_second");
    fs::create_dir_all(&first_directory).expect("create first test directory");
    fs::create_dir_all(&second_directory).expect("create second test directory");
    let first_lab = valid_lab_spec_value();
    let mut second_lab = first_lab.clone();
    second_lab["profiles"][0]["id"] = json!("renamed");
    second_lab["profiles"][0]["label"] = json!("Renamed Profile");
    let start = valid_start_spec_value();
    write_json(&first_directory.join("lab.json"), &first_lab);
    write_json(&second_directory.join("lab.json"), &second_lab);
    write_json(&first_directory.join("start.json"), &start);
    write_json(&second_directory.join("start.json"), &start);

    let first = load_and_resolve_combat_lab_spec_v1(&first_directory.join("lab.json"))
        .expect("resolve first spec");
    let second = load_and_resolve_combat_lab_spec_v1(&second_directory.join("lab.json"))
        .expect("resolve second spec");

    assert_eq!(
        first.profiles[0].profile_hash,
        second.profiles[0].profile_hash
    );
    assert_ne!(first.experiment_hash, second.experiment_hash);
    fs::remove_dir_all(first_directory).expect("remove first test directory");
    fs::remove_dir_all(second_directory).expect("remove second test directory");
}

#[test]
fn experiment_identity_and_schedule_are_hashed() {
    let baseline_directory = test_directory("experiment_identity_baseline");
    fs::create_dir_all(&baseline_directory).expect("create baseline test directory");
    let baseline_lab = valid_lab_spec_value();
    let start = valid_start_spec_value();
    write_json(&baseline_directory.join("lab.json"), &baseline_lab);
    write_json(&baseline_directory.join("start.json"), &start);
    let baseline = load_and_resolve_combat_lab_spec_v1(&baseline_directory.join("lab.json"))
        .expect("resolve baseline spec");

    for (label, pointer, changed_value) in [
        (
            "experiment_id",
            "/experiment_id",
            json!("experiment_changed"),
        ),
        ("scenario_id", "/scenario_id", json!("scenario_changed")),
        ("schedule_seed", "/schedule/seed", json!(43)),
    ] {
        let directory = test_directory(label);
        fs::create_dir_all(&directory).expect("create variant test directory");
        let mut lab = baseline_lab.clone();
        *lab.pointer_mut(pointer).expect("fixture path should exist") = changed_value;
        write_json(&directory.join("lab.json"), &lab);
        write_json(&directory.join("start.json"), &start);
        let changed = load_and_resolve_combat_lab_spec_v1(&directory.join("lab.json"))
            .expect("resolve changed experiment identity");

        assert_ne!(
            baseline.experiment_hash, changed.experiment_hash,
            "experiment hash omitted {label}"
        );
        assert_eq!(baseline.scenario_hash, changed.scenario_hash);
        assert_eq!(baseline.budget_hash, changed.budget_hash);
        assert_eq!(
            baseline.profiles[0].profile_hash,
            changed.profiles[0].profile_hash
        );
        fs::remove_dir_all(directory).expect("remove variant test directory");
    }

    fs::remove_dir_all(baseline_directory).expect("remove baseline test directory");
}

#[test]
fn profile_config_assigns_the_complete_contract() {
    let mut value = valid_lab_spec_value();
    value["common_budget"]["wall_ms"] = json!(250);
    value["common_budget"]["stop_on_win_hp_loss_at_most"] = json!(4);
    value["common_budget"]["max_potions_used"] = json!(2);
    value["common_budget"]["turn_plan_probe_max_inner_nodes"] = json!(31);
    value["common_budget"]["turn_plan_probe_max_end_states"] = json!(32);
    value["common_budget"]["turn_plan_probe_per_bucket_limit"] = json!(33);
    let spec: CombatLabSpecV1 = serde_json::from_value(value).expect("parse lab spec");

    let config = profile_config_v1(&spec.experiment_id, &spec.profiles[0], &spec.common_budget);

    assert_eq!(
        config.input_label.as_deref(),
        Some("combat_lab_v1/experiment/baseline")
    );
    assert_eq!(config.max_nodes, 1000);
    assert_eq!(config.max_actions_per_line, 100);
    assert_eq!(config.max_engine_steps_per_action, 200);
    assert_eq!(
        config.wall_time,
        Some(std::time::Duration::from_millis(250))
    );
    assert_eq!(config.stop_on_win_hp_loss_at_most, Some(4));
    assert_eq!(config.min_win_candidates_before_stop, 1);
    assert_eq!(config.potion_policy, spec.profiles[0].potion_policy);
    assert_eq!(config.max_potions_used, Some(2));
    assert_eq!(config.rollout_policy, spec.profiles[0].rollout_policy);
    assert_eq!(
        config.child_rollout_policy,
        spec.profiles[0].child_rollout_policy
    );
    assert_eq!(config.rollout_max_evaluations, 10);
    assert_eq!(config.rollout_max_actions, 20);
    assert_eq!(config.rollout_beam_width, 3);
    assert_eq!(config.turn_plan_policy, spec.profiles[0].turn_plan_policy);
    assert_eq!(config.frontier_policy, spec.profiles[0].frontier_policy);
    assert_eq!(
        config.phase_guard_policy,
        spec.profiles[0].phase_guard_policy
    );
    assert_eq!(config.setup_bias_policy, spec.profiles[0].setup_bias_policy);
    assert_eq!(config.turn_plan_probe_max_inner_nodes, Some(31));
    assert_eq!(config.turn_plan_probe_max_end_states, Some(32));
    assert_eq!(config.turn_plan_probe_per_bucket_limit, Some(33));
    assert!(config.root_action_prior.is_none());
    assert!(config.turn_plan_prior.is_none());
}

#[test]
fn unsupported_nested_start_entry_fails_resolution_preflight() {
    let directory = test_directory("unknown_nested_start_field");
    fs::create_dir_all(&directory).expect("create test directory");
    let lab = valid_lab_spec_value();
    let mut start = valid_start_spec_value();
    start["master_deck"] = json!([{
        "id": "Bash",
        "unsupported": true
    }]);
    write_json(&directory.join("lab.json"), &lab);
    write_json(&directory.join("start.json"), &start);

    let error = load_and_resolve_combat_lab_spec_v1(&directory.join("lab.json"))
        .expect_err("unknown nested start fields must fail preflight");

    assert!(
        error.contains("failed to parse combat start spec"),
        "{error}"
    );
    assert!(
        error.contains("did not match any variant of untagged enum StartSpecCardSpec"),
        "{error}"
    );
    fs::remove_dir_all(directory).expect("remove test directory");
}

fn valid_lab_spec_value() -> serde_json::Value {
    json!({
        "schema_version": 1,
        "experiment_id": "experiment",
        "scenario_id": "scenario",
        "start_spec": "start.json",
        "schedule": {
            "generator": "split_mix64_v1",
            "seed": 42
        },
        "profiles": [{
            "id": "baseline",
            "label": "Baseline",
            "information_scope": "exact_state_oracle",
            "potion_policy": "never",
            "rollout_policy": "disabled",
            "child_rollout_policy": "immediate",
            "turn_plan_policy": "disabled",
            "frontier_policy": "single_queue",
            "phase_guard_policy": "default",
            "setup_bias_policy": "default"
        }],
        "common_budget": {
            "max_nodes": 1000,
            "max_actions_per_line": 100,
            "max_engine_steps_per_action": 200,
            "wall_ms": null,
            "stop_on_win_hp_loss_at_most": null,
            "min_win_candidates_before_stop": 1,
            "max_potions_used": null,
            "rollout_max_evaluations": 10,
            "rollout_max_actions": 20,
            "rollout_beam_width": 3,
            "turn_plan_probe_max_inner_nodes": null,
            "turn_plan_probe_max_end_states": null,
            "turn_plan_probe_per_bucket_limit": null
        }
    })
}

fn valid_start_spec_value() -> serde_json::Value {
    json!({
        "name": "combat_lab_start",
        "player_class": "Ironclad",
        "ascension_level": 0,
        "encounter_id": "JawWorm",
        "room_type": "monster",
        "seed": 123,
        "player_current_hp": 72,
        "player_max_hp": 80,
        "relics": ["Burning Blood"],
        "potions": [],
        "master_deck": ["Bash"]
    })
}

fn resolved_lab_fixture(label: &str) -> (std::path::PathBuf, super::ResolvedCombatLabSpecV1) {
    let directory = test_directory(label);
    fs::create_dir_all(&directory).expect("create test directory");
    write_json(&directory.join("start.json"), &valid_start_spec_value());
    write_json(&directory.join("lab.json"), &valid_lab_spec_value());
    let resolved = load_and_resolve_combat_lab_spec_v1(&directory.join("lab.json"))
        .expect("resolve laboratory fixture");
    (directory, resolved)
}

fn replayable_win_sample() -> (
    super::CombatLabCompiledSampleV1,
    crate::ai::combat_search_v2::CombatSearchV2TrajectoryReport,
) {
    use crate::ai::combat_search_v2::{trajectory_from_state, CombatSearchV2ActionTrace};
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
    use crate::sim::combat::{
        apply_combat_input_to_stable_observed_v1, CombatPosition, CombatStepLimits, CombatTerminal,
    };
    use crate::state::core::{ClientInput, EngineState};
    use crate::test_support::{blank_test_combat, test_monster};

    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.current_hp = 1;
    monster.max_hp = 1;
    let target = monster.id;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::BattleTrance, 1),
        CombatCard::new(CardId::Strike, 2),
    ];
    combat.zones.draw_pile = vec![CombatCard::new(CardId::Defend, 3)];
    let start = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
    let inputs = vec![
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(target),
        },
    ];
    let mut final_position = start.clone();
    for input in &inputs {
        let observed = apply_combat_input_to_stable_observed_v1(
            &final_position,
            input.clone(),
            CombatStepLimits {
                max_engine_steps: 100,
                deadline: None,
            },
        );
        assert!(!observed.step.truncated);
        assert!(!observed.step.timed_out);
        final_position = observed.step.position;
    }
    assert_eq!(
        crate::sim::combat::combat_terminal(&final_position.engine, &final_position.combat),
        CombatTerminal::Win
    );
    let actions = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| CombatSearchV2ActionTrace {
            step_index: index,
            action_id: index,
            action_key: format!("action-{index}"),
            action_debug: format!("{input:?}"),
            input: input.clone(),
        })
        .collect();
    let trajectory = trajectory_from_state(
        final_position.engine,
        final_position.combat,
        start.combat.entities.player.current_hp,
        actions,
        0,
        0,
        2,
        false,
    );
    let sample = super::CombatLabCompiledSampleV1 {
        sample_index: 7,
        shuffle_seed: 99,
        state_fingerprint: combat_state_fingerprint_v1(&start),
        start,
        non_shuffle_rng_hash: "non-shuffle".to_string(),
        shuffle_rng_hash: "shuffle".to_string(),
        monster_snapshot_hash: "monsters".to_string(),
    };
    (sample, trajectory)
}

fn json_with_reversed_object_keys(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(object) => {
            let fields = object
                .iter()
                .rev()
                .map(|(key, value)| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).expect("serialize object key"),
                        json_with_reversed_object_keys(value)
                    )
                })
                .collect::<Vec<_>>();
            format!("{{{}}}", fields.join(","))
        }
        serde_json::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(json_with_reversed_object_keys)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => serde_json::to_string(value).expect("serialize scalar JSON value"),
    }
}

fn write_json(path: &std::path::Path, value: &serde_json::Value) {
    fs::write(
        path,
        serde_json::to_vec(value).expect("serialize fixture JSON"),
    )
    .expect("write fixture JSON");
}

fn artifact_manifest(
    resolved: super::ResolvedCombatLabSpecV1,
    created_at_unix_ms: u64,
) -> CombatLabManifestV1 {
    CombatLabManifestV1::from_resolved_v1(
        resolved,
        crate::runtime::branch::SourceIdentity {
            git_commit: Some("test-commit".to_string()),
            git_dirty: Some(false),
        },
        created_at_unix_ms,
    )
}

fn artifact_cell(
    resolved: &super::ResolvedCombatLabSpecV1,
    sample_index: u64,
) -> super::CombatLabCellRecordV1 {
    use crate::ai::combat_search_v2::SearchCoverageStatus;

    let (mut sample, trajectory) = replayable_win_sample();
    sample.sample_index = sample_index;
    sample.shuffle_seed = derive_shuffle_seed_v1(&resolved.schedule, sample_index);
    super::replay::combat_lab_cell_record_from_trajectory_with_replayer_v1(
        resolved,
        &sample,
        &resolved.profiles[0],
        SearchCoverageStatus::AcceptedCompleteCandidate,
        Some(&trajectory),
        &crate::ai::combat_search_v2::CombatSearchV2Stats::default(),
        crate::ai::combat_search_v2::replay_combat_search_witness_line_v1,
    )
}

fn assert_resume_mismatch(
    output: &std::path::Path,
    manifest: CombatLabManifestV1,
    differing_field: &str,
) {
    let error = CombatLabArtifactStoreV1::create_or_resume(output, manifest)
        .err()
        .expect("resume identity mismatch must be rejected");
    assert!(error.contains(differing_field), "{error}");
}

fn artifact_journal_digest(bytes: &[u8]) -> String {
    use blake2::{Blake2b512, Digest};

    let mut hasher = Blake2b512::new();
    hasher.update(bytes);
    hasher.finalize()[..32]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn test_directory(label: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "sts_simulator_combat_lab_v1_{label}_{}_{}",
        std::process::id(),
        nonce
    ))
}
