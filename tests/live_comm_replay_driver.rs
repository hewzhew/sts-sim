use sts_simulator::diff::replay::live_comm_replay::{
    derive_combat_replay_view, load_live_session_replay_path, verify_combat_replay_view,
};

#[test]
fn replay_single_live_comm_replay_from_env() {
    let Some(path) = std::env::var_os("LIVE_COMM_REPLAY_FILE") else {
        return;
    };
    run_replay_path(std::path::Path::new(&path));
}

#[test]
fn replay_archived_live_comm_replays() {
    let replay_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("logs/replays");
    if !replay_dir.exists() {
        return;
    }

    let mut found = false;
    for entry in std::fs::read_dir(&replay_dir).expect("read logs/replays dir") {
        let entry = entry.expect("read replay entry");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        found = true;
        run_replay_path(&path);
    }

    if !found {
        eprintln!(
            "no structured livecomm replays under {}",
            replay_dir.display()
        );
    }
}

fn run_replay_path(path: &std::path::Path) {
    let replay = load_live_session_replay_path(path).expect("load structured replay");
    let combat_view = derive_combat_replay_view(&replay);
    let report = verify_combat_replay_view(&combat_view, true).expect("verify combat replay");
    assert!(
        report.failures.is_empty(),
        "livecomm replay {} diverged at command_id={} response_id={:?} frame_id={:?}",
        path.display(),
        report.failures[0].command_id,
        report.failures[0].response_id,
        report.failures[0].state_frame_id
    );
}
