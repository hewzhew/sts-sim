use std::fs;
use std::path::Path;

use serde_json::Value;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::content::powers::PowerId;
use sts_simulator::content::powers::store::{has_power, power_amount, powers_for};
use sts_simulator::diff::state_sync::build_combat_state;

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
    build_combat_state(&frame["game_state"]["combat_state"], &frame["game_state"]["relics"])
}

#[test]
fn guardian_threshold_sample_imports_runtime_state() {
    let frame = load_sample("guardian_threshold");
    let state = build_state_from_frame(&frame);
    let guardian = state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.monster_type == EnemyId::TheGuardian as usize)
        .expect("guardian sample should contain TheGuardian");

    assert_eq!(power_amount(&state, guardian.id, PowerId::GuardianThreshold), 30);
    assert!(has_power(&state, guardian.id, PowerId::ModeShift));
    assert_eq!(power_amount(&state, guardian.id, PowerId::ModeShift), 30);
}

#[test]
fn angry_sample_imports_runtime_state() {
    let frame = load_sample("angry");
    let state = build_state_from_frame(&frame);
    let gremlin = state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.monster_type == EnemyId::GremlinWarrior as usize)
        .expect("angry sample should contain GremlinWarrior");

    assert_eq!(power_amount(&state, gremlin.id, PowerId::Angry), 1);
}

#[test]
fn combust_sample_imports_runtime_state() {
    let frame = load_sample("combust");
    let state = build_state_from_frame(&frame);
    let combust = powers_for(&state, 0)
        .and_then(|powers| powers.iter().find(|power| power.power_type == PowerId::Combust))
        .expect("combust sample should import Combust onto the player");

    assert_eq!(combust.amount, 1);
    assert_eq!(combust.extra_data, 1);
}
