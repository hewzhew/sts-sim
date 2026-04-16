use std::fs;
use std::path::Path;

use serde_json::Value;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::content::powers::store::{has_power, power_amount, powers_for};
use sts_simulator::content::powers::PowerId;
use sts_simulator::diff::state_sync::{build_combat_state, snapshot_uuid};

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
    build_combat_state(
        &frame["game_state"]["combat_state"],
        &frame["game_state"]["relics"],
    )
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

    assert_eq!(
        power_amount(&state, guardian.id, PowerId::GuardianThreshold),
        30
    );
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
        .and_then(|powers| {
            powers
                .iter()
                .find(|power| power.power_type == PowerId::Combust)
        })
        .expect("combust sample should import Combust onto the player");

    assert_eq!(combust.amount, 1);
    assert_eq!(combust.extra_data, 1);
}

#[test]
fn stasis_sample_imports_runtime_state() {
    let frame = load_sample("stasis");
    let state = build_state_from_frame(&frame);
    let bronze_orb = state
        .entities
        .monsters
        .iter()
        .find(|monster| {
            monster.monster_type == EnemyId::BronzeOrb as usize
                && powers_for(&state, monster.id)
                    .map(|powers| {
                        powers
                            .iter()
                            .any(|power| power.power_type == PowerId::Stasis)
                    })
                    .unwrap_or(false)
        })
        .expect("stasis sample should contain a BronzeOrb with Stasis");
    let stasis = powers_for(&state, bronze_orb.id)
        .and_then(|powers| {
            powers
                .iter()
                .find(|power| power.power_type == PowerId::Stasis)
        })
        .expect("stasis sample should import Stasis onto BronzeOrb");
    let expected_uuid = frame["game_state"]["combat_state"]["monsters"]
        .as_array()
        .and_then(|monsters| {
            monsters.iter().find_map(|monster| {
                let is_bronze_orb = monster.get("id").and_then(|v| v.as_str()) == Some("BronzeOrb");
                let card_uuid =
                    monster
                        .get("powers")
                        .and_then(|v| v.as_array())
                        .and_then(|powers| {
                            powers.iter().find_map(|power| {
                                (power.get("id").and_then(|v| v.as_str()) == Some("Stasis"))
                                    .then(|| power["runtime_state"]["card_uuid"].clone())
                            })
                        });
                if is_bronze_orb {
                    card_uuid
                } else {
                    None
                }
            })
        })
        .expect("stasis sample should include power.runtime_state.card_uuid");

    assert_eq!(stasis.amount, -1);
    assert_eq!(stasis.extra_data, snapshot_uuid(&expected_uuid, 0) as i32);
}
