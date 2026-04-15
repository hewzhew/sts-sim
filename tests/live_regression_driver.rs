use sts_simulator::testing::fixtures::scenario::{assert_fixture, ScenarioFixture};

#[test]
fn replay_fixtures() {
    let fixture_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/live_regressions");
    if !fixture_dir.exists() {
        return;
    }

    let mut found = false;
    for entry in std::fs::read_dir(&fixture_dir).expect("read live_regressions dir") {
        let entry = entry.expect("read fixture entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|name| name.ends_with(".min.json"))
        {
            continue;
        }
        found = true;
        run_fixture_path(&path);
    }

    if !found {
        eprintln!(
            "no live regression fixtures under {}",
            fixture_dir.display()
        );
    }
}

#[test]
fn replay_single_fixture_from_env() {
    let Some(path) = std::env::var_os("LIVE_REGRESSION_FIXTURE") else {
        return;
    };
    run_fixture_path(std::path::Path::new(&path));
}

fn run_fixture_path(path: &std::path::Path) {
    let text = std::fs::read_to_string(path).expect("read fixture");
    let fixture: ScenarioFixture = serde_json::from_str(&text).expect("parse fixture");
    assert_fixture(&fixture).unwrap_or_else(|err| panic!("{err}"));
}
