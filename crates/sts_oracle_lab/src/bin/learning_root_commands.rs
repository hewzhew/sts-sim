use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sts_oracle_runtime::eval::run_control::{
    CombatLearningRootBatchArtifactV1, CombatLearningRootContextV1, CombatLearningRootIdentityV1,
};
use sts_oracle_runtime::runtime::branch::{
    load_oracle_run_continuation_v1, ORACLE_RUN_CONTINUATION_SCHEMA_NAME,
    ORACLE_RUN_CONTINUATION_SCHEMA_VERSION,
};

const SUMMARY_SCHEMA_NAME: &str = "CombatLearningRootExportSummary";
const SUMMARY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CombatLearningRootExportSummaryV1 {
    schema_name: &'static str,
    schema_version: u32,
    output: PathBuf,
    payload_bytes: usize,
    roots: Vec<CombatLearningRootExportedRootV1>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct CombatLearningRootExportedRootV1 {
    identity: CombatLearningRootIdentityV1,
    context: CombatLearningRootContextV1,
}

pub(super) fn export(
    continuation_paths: &[PathBuf],
    output: &Path,
    max_bytes: usize,
) -> Result<CombatLearningRootExportSummaryV1, String> {
    if continuation_paths.is_empty() {
        return Err("learning root export requires at least one continuation".to_owned());
    }
    if output.exists() {
        return Err(format!(
            "learning root output already exists: {}",
            output.display()
        ));
    }
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| format!("learning root output has no parent: {}", output.display()))?;
    if !parent.is_dir() {
        return Err(format!(
            "learning root output parent does not exist: {}",
            parent.display()
        ));
    }

    let mut checkpoints = Vec::with_capacity(continuation_paths.len());
    for path in continuation_paths {
        let continuation = load_oracle_run_continuation_v1(path)?;
        if continuation.schema_name != ORACLE_RUN_CONTINUATION_SCHEMA_NAME
            || continuation.schema_version != ORACLE_RUN_CONTINUATION_SCHEMA_VERSION
        {
            return Err(format!(
                "unsupported oracle continuation schema in {}",
                path.display()
            ));
        }
        checkpoints.push(continuation.session);
    }

    let artifact = CombatLearningRootBatchArtifactV1::from_checkpoints(checkpoints)?;
    let roots = artifact
        .roots()
        .iter()
        .map(|root| CombatLearningRootExportedRootV1 {
            identity: root.identity().clone(),
            context: *root.context(),
        })
        .collect();
    let payload = artifact.encode(max_bytes)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    file.write_all(&payload)
        .map_err(|error| format!("failed to write {}: {error}", output.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to flush {}: {error}", output.display()))?;

    Ok(CombatLearningRootExportSummaryV1 {
        schema_name: SUMMARY_SCHEMA_NAME,
        schema_version: SUMMARY_SCHEMA_VERSION,
        output: output.to_path_buf(),
        payload_bytes: payload.len(),
        roots,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use sts_oracle_runtime::content::cards::CardId;
    use sts_oracle_runtime::eval::run_control::{
        RunControlSession, RunControlSessionCheckpointV1, RunProgressJournalV1,
    };
    use sts_oracle_runtime::runtime::branch::{
        save_oracle_run_continuation_v1, OracleRunContinuationV1,
    };
    use sts_oracle_runtime::state::core::{
        ActiveCombat, CombatContext, DiscoveryChoiceState, EngineState, PendingChoice,
        RoomCombatContext,
    };
    use sts_oracle_runtime::state::map::node::RoomType;

    use super::*;

    #[test]
    fn public_continuation_exports_one_bridge_decodable_root() {
        let root = std::env::temp_dir().join(format!(
            "sts_learning_root_export_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        fs::create_dir(&root).expect("create test root");
        let continuation_path = root.join("source.continuation.json");
        let output_path = root.join("roots.bin");

        let mut session = RunControlSession::new(Default::default());
        let choice = PendingChoice::DiscoverySelect(DiscoveryChoiceState {
            cards: vec![CardId::Bash, CardId::FiendFire],
            colorless: false,
            card_type: None,
            amount: 1,
            can_skip: true,
        });
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            sts_oracle_runtime::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        save_oracle_run_continuation_v1(
            &continuation_path,
            &OracleRunContinuationV1 {
                schema_name: ORACLE_RUN_CONTINUATION_SCHEMA_NAME.to_owned(),
                schema_version: ORACLE_RUN_CONTINUATION_SCHEMA_VERSION,
                seed: 0,
                ascension: 0,
                journal: RunProgressJournalV1::default(),
                session: RunControlSessionCheckpointV1::from_session(&session),
                explorer_frontier: None,
            },
        )
        .expect("write current continuation");

        let summary = export(
            std::slice::from_ref(&continuation_path),
            &output_path,
            1024 * 1024,
        )
        .expect("export combat root");
        let payload = fs::read(&output_path).expect("read opaque root artifact");
        let artifact = CombatLearningRootBatchArtifactV1::decode(&payload, 1, 1024 * 1024)
            .expect("decode exported root");

        assert_eq!(summary.roots.len(), 1);
        assert_eq!(summary.payload_bytes, payload.len());
        assert_eq!(artifact.roots().len(), 1);
        assert!(export(
            std::slice::from_ref(&continuation_path),
            &output_path,
            1024 * 1024,
        )
        .is_err());

        fs::remove_dir_all(&root).expect("remove test root");
    }
}
