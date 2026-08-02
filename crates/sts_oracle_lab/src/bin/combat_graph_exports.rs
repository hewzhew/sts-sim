//! Persistence orchestration for selected local combat graph paths.

use std::path::{Path, PathBuf};

use sts_combat_planner::TurnOptionAction;
use sts_oracle_runtime::eval::combat_case::CombatCase;
use sts_oracle_runtime::sim::combat::CombatPosition;
use sts_oracle_runtime::state::core::ClientInput;

use super::combat_evidence_export::{
    export_verified_combat_witness, CombatEvidenceManifestExport, VerifiedCombatWitnessExport,
};
use super::combat_evidence_manifest::CombatEvidenceProducerV1;
use super::combat_replay_tools::export_descendant_combat_case;

pub(super) struct LocalGraphExportPaths<'a> {
    pub(super) witness_actions: Option<&'a Path>,
    pub(super) deepest_survival_case: Option<&'a Path>,
    pub(super) deepest_progress_case: Option<&'a Path>,
}

pub(super) struct LocalGraphExportActions<'a> {
    pub(super) witness: Option<&'a [ClientInput]>,
    pub(super) witness_final_position: Option<&'a CombatPosition>,
    pub(super) deepest_survival: &'a [TurnOptionAction],
    pub(super) deepest_progress: &'a [TurnOptionAction],
}

pub(super) struct LocalGraphExports {
    pub(super) witness_actions: Option<PathBuf>,
    pub(super) witness_manifest: Option<PathBuf>,
    pub(super) deepest_survival_case: Option<PathBuf>,
    pub(super) deepest_survival_actions: Option<PathBuf>,
    pub(super) deepest_progress_case: Option<PathBuf>,
    pub(super) deepest_progress_actions: Option<PathBuf>,
}

pub(super) fn export_local_graph_paths(
    base: &CombatCase,
    witness_manifest_case: Option<&Path>,
    paths: LocalGraphExportPaths<'_>,
    actions: LocalGraphExportActions<'_>,
    max_engine_steps_per_transition: usize,
) -> Result<LocalGraphExports, String> {
    let (witness_actions, witness_manifest) = match (
        paths.witness_actions,
        actions.witness,
        actions.witness_final_position,
    ) {
        (Some(path), Some(actions), Some(expected_final_position)) => {
            let manifest = export_verified_combat_witness(VerifiedCombatWitnessExport {
                root_position: &base.position,
                action_output: path,
                actions,
                expected_final_position,
                max_engine_steps_per_transition,
                manifest: witness_manifest_case.map(|case_path| CombatEvidenceManifestExport {
                    producer: CombatEvidenceProducerV1::LocalGraphSearch,
                    case_path,
                    evidence_id: "local_graph_complete_win",
                }),
            })?;
            (Some(path.to_path_buf()), manifest)
        }
        (Some(_), Some(_), None) => {
            return Err("local-graph witness export is missing its final position".to_string())
        }
        _ => (None, None),
    };
    let (deepest_survival_case, deepest_survival_actions) =
        if let Some(path) = paths.deepest_survival_case {
            let actions_path = export_descendant_combat_case(
                base,
                actions.deepest_survival,
                path,
                max_engine_steps_per_transition,
                "local_turn_graph_deepest_survival",
            )?;
            (Some(path.to_path_buf()), Some(actions_path))
        } else {
            (None, None)
        };
    let (deepest_progress_case, deepest_progress_actions) =
        if let Some(path) = paths.deepest_progress_case {
            let actions_path = export_descendant_combat_case(
                base,
                actions.deepest_progress,
                path,
                max_engine_steps_per_transition,
                "local_turn_graph_deepest_progress",
            )?;
            (Some(path.to_path_buf()), Some(actions_path))
        } else {
            (None, None)
        };

    Ok(LocalGraphExports {
        witness_actions,
        witness_manifest,
        deepest_survival_case,
        deepest_survival_actions,
        deepest_progress_case,
        deepest_progress_actions,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use sts_oracle_runtime::eval::combat_case::load_combat_case;

    use super::*;
    use crate::combat_evidence_manifest::{
        combat_evidence_manifest_path_for_actions, decode_combat_evidence_manifest,
    };
    use crate::combat_replay_tools::replay_combat_inputs;

    const MAX_ENGINE_STEPS_PER_TRANSITION: usize = 10_000;

    fn fixture_path(file_name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/oracle_witnesses")
            .join(file_name)
    }

    fn temp_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "sts-oracle-local-graph-export-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    fn fixture() -> (PathBuf, CombatCase, Vec<ClientInput>) {
        let case_path = fixture_path("seed20260713009_a0_slime_boss.combat-case.json");
        let actions_path =
            fixture_path("seed20260713009_a0_slime_boss.local-turn-graph.actions.json");
        let case = load_combat_case(&case_path).expect("fixture case should load");
        let actions = serde_json::from_slice::<Vec<ClientInput>>(
            &fs::read(actions_path).expect("fixture actions should load"),
        )
        .expect("fixture actions should decode");
        (case_path, case, actions)
    }

    fn export_fixture(
        case_path: Option<&Path>,
        case: &CombatCase,
        actions: &[ClientInput],
        final_position: &CombatPosition,
        action_output: &Path,
    ) -> LocalGraphExports {
        export_local_graph_paths(
            case,
            case_path,
            LocalGraphExportPaths {
                witness_actions: Some(action_output),
                deepest_survival_case: None,
                deepest_progress_case: None,
            },
            LocalGraphExportActions {
                witness: Some(actions),
                witness_final_position: Some(final_position),
                deepest_survival: &[],
                deepest_progress: &[],
            },
            MAX_ENGINE_STEPS_PER_TRANSITION,
        )
        .expect("local-graph witness should export")
    }

    #[test]
    fn ordinary_complete_win_exports_a_local_graph_manifest() {
        let (case_path, case, actions) = fixture();
        let final_position = replay_combat_inputs(
            case.position.clone(),
            &actions,
            MAX_ENGINE_STEPS_PER_TRANSITION,
        )
        .expect("fixture actions should replay");
        let directory = temp_directory("ordinary");
        let action_output = directory.join("win.actions.json");
        let exports = export_fixture(
            Some(&case_path),
            &case,
            &actions,
            &final_position,
            &action_output,
        );

        let manifest_path = exports
            .witness_manifest
            .expect("ordinary root should receive a manifest");
        let manifest = decode_combat_evidence_manifest(
            &manifest_path,
            &fs::read(&manifest_path).expect("manifest should exist"),
        )
        .expect("manifest should decode");
        assert_eq!(
            manifest.producer,
            CombatEvidenceProducerV1::LocalGraphSearch
        );
        assert_eq!(manifest.case_path, case_path);
        assert_eq!(manifest.entries[0].action_paths, vec![action_output]);

        fs::remove_dir_all(directory).expect("temporary export should clean up");
    }

    #[test]
    fn full_health_counterfactual_exports_actions_without_claiming_the_original_case() {
        let (_, mut case, actions) = fixture();
        case.position.combat.entities.player.current_hp =
            case.position.combat.entities.player.max_hp;
        let final_position = replay_combat_inputs(
            case.position.clone(),
            &actions,
            MAX_ENGINE_STEPS_PER_TRANSITION,
        )
        .expect("counterfactual fixture actions should replay");
        let directory = temp_directory("full-health");
        let action_output = directory.join("win.actions.json");
        let exports = export_fixture(None, &case, &actions, &final_position, &action_output);

        assert_eq!(exports.witness_actions, Some(action_output.clone()));
        assert!(exports.witness_manifest.is_none());
        assert!(!combat_evidence_manifest_path_for_actions(&action_output).exists());

        fs::remove_dir_all(directory).expect("temporary export should clean up");
    }
}
