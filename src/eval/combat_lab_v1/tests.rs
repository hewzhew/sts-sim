use super::{
    derive_shuffle_seed_v1, load_and_resolve_combat_lab_spec_v1, profile_config_v1,
    CombatLabShuffleGeneratorV1, CombatLabShuffleScheduleV1, CombatLabSpecV1,
};
use serde_json::json;
use std::fs;

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
