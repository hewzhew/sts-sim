use sts_simulator::fixtures::combat_case::{assert_case, CombatCase};

#[test]
fn replay_combat_cases() {
    let case_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/combat_cases");
    if !case_dir.exists() {
        return;
    }

    let mut found = false;
    for entry in std::fs::read_dir(&case_dir).expect("read combat_cases dir") {
        let entry = entry.expect("read case entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        found = true;
        run_case_path(&path);
    }

    if !found {
        eprintln!("no combat cases under {}", case_dir.display());
    }
}

#[test]
fn replay_single_combat_case_from_env() {
    let Some(path) = std::env::var_os("COMBAT_CASE") else {
        return;
    };
    run_case_path(std::path::Path::new(&path));
}

fn run_case_path(path: &std::path::Path) {
    let text = std::fs::read_to_string(path).expect("read combat case");
    let case: CombatCase = serde_json::from_str(&text).expect("parse combat case");
    assert_case(&case).unwrap_or_else(|err| panic!("{err}"));
}
