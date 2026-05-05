use std::fs;
use std::path::Path;

use serde_json::Value;
use sts_simulator::bot::combat::diagnose_root_search_with_depth;
use sts_simulator::diff::state_sync::build_combat_state_from_snapshots;
use sts_simulator::state::EngineState;

fn load_sample(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("protocol_truth_samples")
        .join(name)
        .join("frame.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read sample {}: {err}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("failed to parse sample {}: {err}", path.display()))
}

fn build_state_from_frame(frame: &Value) -> sts_simulator::runtime::combat::CombatState {
    build_combat_state_from_snapshots(
        &frame["game_state"]["combat_truth"],
        &frame["game_state"]["combat_observation"],
        &frame["game_state"]["relics"],
    )
}

#[test]
fn sentry_live_comm_frame_root_search_returns() {
    let frame = load_sample("sentry_livecomm");
    let combat = build_state_from_frame(&frame);
    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    assert!(diagnostics.legal_moves > 0);
    assert_eq!(
        diagnostics.decision_audit["exact_turn_shadow"]["skipped"].as_bool(),
        Some(true)
    );
    assert_eq!(
        diagnostics.decision_audit["exact_turn_shadow"]["skip_reason"].as_str(),
        Some("high_root_branching")
    );
}
