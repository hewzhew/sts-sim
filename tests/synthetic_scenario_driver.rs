use sts_simulator::testing::author_spec::{compile_combat_author_spec, CombatAuthorSpec};
use sts_simulator::testing::scenario::assert_fixture;

#[test]
fn replay_synthetic_scenarios() {
    let fixture_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/synthetic_scenarios");
    if !fixture_dir.exists() {
        return;
    }

    let mut found = false;
    for entry in std::fs::read_dir(&fixture_dir).expect("read synthetic_scenarios dir") {
        let entry = entry.expect("read fixture entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        found = true;
        run_scenario_path(&path);
    }

    if !found {
        eprintln!("no synthetic scenarios under {}", fixture_dir.display());
    }
}

#[test]
fn replay_single_synthetic_scenario_from_env() {
    let Some(path) = std::env::var_os("SYNTHETIC_SCENARIO") else {
        return;
    };
    run_scenario_path(std::path::Path::new(&path));
}

fn run_scenario_path(path: &std::path::Path) {
    let text = std::fs::read_to_string(path).expect("read synthetic scenario");
    let spec: CombatAuthorSpec = serde_json::from_str(&text).expect("parse synthetic scenario");
    let fixture = compile_combat_author_spec(&spec).expect("compile synthetic scenario");
    assert_fixture(&fixture).unwrap_or_else(|err| panic!("{err}"));
}
