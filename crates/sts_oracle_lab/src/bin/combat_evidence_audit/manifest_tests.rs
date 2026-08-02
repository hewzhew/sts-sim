use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::artifacts::declared_manifest_pairs;
use super::replay::replay_pair;
use super::{PairCandidate, ReplayExpectations};

fn tracked_slime_boss_pair() -> PairCandidate {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixture_root = repository.join("fixtures/oracle_witnesses");
    PairCandidate {
        case_path: fixture_root.join("seed20260713009_a0_slime_boss.combat-case.json"),
        action_paths: vec![
            fixture_root.join("seed20260713009_a0_slime_boss.local-turn-graph.actions.json")
        ],
        provenance: BTreeSet::from(["test_fixture".to_string()]),
        source_paths: BTreeSet::new(),
        expectations: ReplayExpectations::default(),
    }
}

#[test]
fn typed_manifest_producers_resolve_external_case_and_revalidate_identity() {
    let baseline = tracked_slime_boss_pair();
    let expected = replay_pair(&baseline, 250).expect("tracked witness should replay");
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "sts-combat-evidence-manifest-{}-{suffix}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let runtime_fingerprint = "0".repeat(128);
    let current_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    for producer in [
        "historical_combat_witness_export",
        "policy_discrepancy_search",
        "potion_expenditure_audit",
    ] {
        let manifest_path = root.join(format!("{producer}.combat-evidence-manifest.json"));
        let value = serde_json::json!({
            "schema_name": "CombatEvidenceManifestV1",
            "schema_version": 1,
            "producer": producer,
            "runtime": {},
            "runtime_source_content_fingerprint": runtime_fingerprint,
            "root_exact_state_hash": expected.root_exact_state_hash,
            "case_path": baseline.case_path.canonicalize().unwrap(),
            "entries": [{
                "evidence_id": producer,
                "action_paths": [baseline.action_paths[0].canonicalize().unwrap()],
                "action_sequence_blake2b_512": expected.action_sequence_blake2b_512,
                "supplied_action_count": expected.supplied_action_count,
                "expected_terminal": expected.final_terminal,
                "expected_final_player_hp": expected.final_player_hp
            }]
        });
        fs::write(&manifest_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();

        let candidates = declared_manifest_pairs(&manifest_path, &root, &current_dir)
            .expect("typed manifest should resolve");
        assert_eq!(candidates.len(), 1);
        let replayed =
            replay_pair(&candidates[0], 250).expect("manifest identity should revalidate");
        assert!(replayed.provenance.contains("typed_evidence_manifest"));
        assert_eq!(replayed.record_id, expected.record_id);
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn manifest_outcome_mismatch_is_rejected_after_exact_replay() {
    let mut candidate = tracked_slime_boss_pair();
    let expected = replay_pair(&candidate, 250).expect("tracked witness should replay");
    candidate
        .expectations
        .final_player_hps
        .insert(expected.final_player_hp + 1);

    let error = replay_pair(&candidate, 250).expect_err("tampered outcome must be rejected");
    assert!(error.contains("manifest final player HP"));
}
