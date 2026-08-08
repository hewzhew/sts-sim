use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Subcommand;
use serde::Serialize;
use sts_oracle_runtime::eval::run_control::{
    BoundedRunDriveStopV1, BoundedRunDriver, BoundedRunStepControlV1,
    CombatLearningRootBatchArtifactV1, CombatLearningRootContextV1, CombatLearningRootIdentityV1,
    RunControlConfig, RunControlSession, RunControlSessionCheckpointV1,
};
use sts_oracle_runtime::runtime::branch::{
    apply_oracle_production_noncombat_step_v1, load_oracle_run_continuation_v1,
    OracleProductionNoncombatStepV1, ORACLE_RUN_CONTINUATION_SCHEMA_NAME,
    ORACLE_RUN_CONTINUATION_SCHEMA_VERSION,
};

const SUMMARY_SCHEMA_NAME: &str = "CombatLearningRootExportSummary";
const SUMMARY_SCHEMA_VERSION: u32 = 1;
const COLLECTION_SUMMARY_SCHEMA_NAME: &str = "CombatLearningRootCollectionSummary";
const COLLECTION_SUMMARY_SCHEMA_VERSION: u32 = 2;
const MAX_COLLECTED_ROOTS: usize = 64;

#[derive(Debug, Subcommand)]
pub(super) enum LearningRootCommand {
    /// Convert production continuations already at combat boundaries into one
    /// bounded opaque root batch.
    Export {
        #[arg(long, required = true)]
        continuation: Vec<PathBuf>,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 16 * 1024 * 1024)]
        max_bytes: usize,
    },
    /// Run current production owners to the first combat boundary and emit one
    /// opaque batch without intermediate continuation JSON.
    Collect {
        #[arg(long, required = true)]
        seed: Vec<u64>,
        #[arg(long, default_value_t = 0)]
        ascension: u8,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value_t = 64)]
        max_progress_steps: usize,
        #[arg(long, default_value_t = 10_000)]
        wall_ms: u64,
        #[arg(long, default_value_t = 16 * 1024 * 1024)]
        max_bytes: usize,
    },
}

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

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CombatLearningRootCollectionSummaryV2 {
    schema_name: &'static str,
    schema_version: u32,
    ascension: u8,
    total_applied_progress_steps: usize,
    output: PathBuf,
    payload_bytes: usize,
    roots: Vec<CombatLearningRootCollectedRootV2>,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct CombatLearningRootCollectedRootV2 {
    seed: u64,
    applied_progress_steps: usize,
    identity: CombatLearningRootIdentityV1,
    context: CombatLearningRootContextV1,
}

enum CombatLearningRootCollectionStop {
    CombatBoundary,
    AutomationGap(String),
}

pub(super) fn run(command: LearningRootCommand) -> Result<(), String> {
    match command {
        LearningRootCommand::Export {
            continuation,
            output,
            max_bytes,
        } => super::print_json(&export(&continuation, &output, max_bytes)?),
        LearningRootCommand::Collect {
            seed,
            ascension,
            output,
            max_progress_steps,
            wall_ms,
            max_bytes,
        } => super::print_json(&collect(
            &seed,
            ascension,
            &output,
            max_progress_steps,
            wall_ms,
            max_bytes,
        )?),
    }
}

pub(super) fn export(
    continuation_paths: &[PathBuf],
    output: &Path,
    max_bytes: usize,
) -> Result<CombatLearningRootExportSummaryV1, String> {
    if continuation_paths.is_empty() {
        return Err("learning root export requires at least one continuation".to_owned());
    }
    require_fresh_output(output)?;

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
    write_new_payload(output, &payload)?;

    Ok(CombatLearningRootExportSummaryV1 {
        schema_name: SUMMARY_SCHEMA_NAME,
        schema_version: SUMMARY_SCHEMA_VERSION,
        output: output.to_path_buf(),
        payload_bytes: payload.len(),
        roots,
    })
}

pub(super) fn collect(
    seeds: &[u64],
    ascension: u8,
    output: &Path,
    max_progress_steps: usize,
    wall_ms: u64,
    max_bytes: usize,
) -> Result<CombatLearningRootCollectionSummaryV2, String> {
    require_fresh_output(output)?;
    if ascension > 20 {
        return Err("learning root collection ascension must be at most 20".to_owned());
    }
    if seeds.is_empty() {
        return Err("learning root collection requires at least one seed".to_owned());
    }
    if seeds.len() > MAX_COLLECTED_ROOTS {
        return Err(format!(
            "learning root collection accepts at most {MAX_COLLECTED_ROOTS} seeds"
        ));
    }
    let distinct = seeds.iter().copied().collect::<BTreeSet<_>>();
    if distinct.len() != seeds.len() {
        return Err("learning root collection requires distinct seeds".to_owned());
    }

    let mut checkpoints = Vec::with_capacity(seeds.len());
    let mut collected = Vec::with_capacity(seeds.len());
    for &seed in seeds {
        let (checkpoint, applied_progress_steps) =
            collect_one(seed, ascension, max_progress_steps, wall_ms)
                .map_err(|error| format!("seed {seed}: {error}"))?;
        checkpoints.push(checkpoint);
        collected.push((seed, applied_progress_steps));
    }
    let artifact = CombatLearningRootBatchArtifactV1::from_checkpoints(checkpoints)?;
    let roots = artifact
        .roots()
        .iter()
        .zip(collected)
        .map(
            |(root, (seed, applied_progress_steps))| CombatLearningRootCollectedRootV2 {
                seed,
                applied_progress_steps,
                identity: root.identity().clone(),
                context: *root.context(),
            },
        )
        .collect::<Vec<_>>();
    let total_applied_progress_steps = roots.iter().map(|root| root.applied_progress_steps).sum();
    let payload = artifact.encode(max_bytes)?;
    write_new_payload(output, &payload)?;

    Ok(CombatLearningRootCollectionSummaryV2 {
        schema_name: COLLECTION_SUMMARY_SCHEMA_NAME,
        schema_version: COLLECTION_SUMMARY_SCHEMA_VERSION,
        ascension,
        total_applied_progress_steps,
        output: output.to_path_buf(),
        payload_bytes: payload.len(),
        roots,
    })
}

fn collect_one(
    seed: u64,
    ascension: u8,
    max_progress_steps: usize,
    wall_ms: u64,
) -> Result<(RunControlSessionCheckpointV1, usize), String> {
    let mut session = RunControlSession::new(RunControlConfig {
        seed,
        ascension_level: ascension,
        ..RunControlConfig::default()
    });
    let drive = BoundedRunDriver::new(max_progress_steps, Some(wall_ms))?
        .drive_with(&mut session, |session, _context| {
            match apply_oracle_production_noncombat_step_v1(session)? {
                OracleProductionNoncombatStepV1::Applied(step) => {
                    Ok(BoundedRunStepControlV1::Continue {
                        progress_steps: vec![step],
                    })
                }
                OracleProductionNoncombatStepV1::CombatBoundary => {
                    Ok(BoundedRunStepControlV1::Stop {
                        progress_steps: Vec::new(),
                        output: CombatLearningRootCollectionStop::CombatBoundary,
                    })
                }
                OracleProductionNoncombatStepV1::AutomationGap { reason } => {
                    Ok(BoundedRunStepControlV1::Stop {
                        progress_steps: Vec::new(),
                        output: CombatLearningRootCollectionStop::AutomationGap(reason),
                    })
                }
            }
        })
        .map_err(|error| error.message)?;
    let applied_progress_steps = drive.applied_progress_steps();
    match drive.stop {
        BoundedRunDriveStopV1::Step(CombatLearningRootCollectionStop::CombatBoundary) => {}
        BoundedRunDriveStopV1::Step(CombatLearningRootCollectionStop::AutomationGap(reason)) => {
            return Err(format!(
                "learning root collection did not reach a combat boundary: {reason}"
            ));
        }
        BoundedRunDriveStopV1::ProgressBudgetExhausted => {
            return Err(format!(
                "learning root collection did not reach a combat boundary: progress budget exhausted after {applied_progress_steps} steps"
            ));
        }
        BoundedRunDriveStopV1::WallDeadlineReached => {
            return Err(format!(
                "learning root collection did not reach a combat boundary: wall deadline reached after {applied_progress_steps} steps"
            ));
        }
        BoundedRunDriveStopV1::RunCompleted { victory } => {
            return Err(format!(
                "learning root collection did not reach a combat boundary: run completed with victory={victory}"
            ));
        }
    }
    Ok((
        RunControlSessionCheckpointV1::from_session(&session),
        applied_progress_steps,
    ))
}

fn require_fresh_output(output: &Path) -> Result<(), String> {
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
    Ok(())
}

fn write_new_payload(output: &Path, payload: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|error| format!("failed to create {}: {error}", output.display()))?;
    file.write_all(payload)
        .map_err(|error| format!("failed to write {}: {error}", output.display()))?;
    file.sync_all()
        .map_err(|error| format!("failed to flush {}: {error}", output.display()))
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
        let root = unique_temp_dir("export");
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

    #[test]
    fn current_owner_run_collects_first_combat_as_an_opaque_root() {
        let root = unique_temp_dir("collect");
        fs::create_dir(&root).expect("create test root");
        let output_path = root.join("root.bin");

        let summary = collect(&[11, 12], 0, &output_path, 32, 10_000, 16 * 1024 * 1024)
            .expect("collect first production combat root");
        let payload = fs::read(&output_path).expect("read collected root artifact");
        let artifact = CombatLearningRootBatchArtifactV1::decode(&payload, 2, 16 * 1024 * 1024)
            .expect("decode collected root");

        assert_eq!(summary.schema_name, COLLECTION_SUMMARY_SCHEMA_NAME);
        assert_eq!(summary.ascension, 0);
        assert!(summary.total_applied_progress_steps > 0);
        assert_eq!(summary.payload_bytes, payload.len());
        assert_eq!(artifact.roots().len(), 2);
        assert_eq!(
            summary
                .roots
                .iter()
                .map(|root| root.seed)
                .collect::<Vec<_>>(),
            vec![11, 12]
        );
        for (artifact_root, summary_root) in artifact.roots().iter().zip(&summary.roots) {
            assert!(summary_root.applied_progress_steps > 0);
            assert_eq!(artifact_root.identity(), &summary_root.identity);
            assert_eq!(artifact_root.context(), &summary_root.context);
        }

        fs::remove_dir_all(&root).expect("remove test root");
    }

    #[test]
    fn collection_budget_exhaustion_writes_no_artifact() {
        let root = unique_temp_dir("bounded");
        fs::create_dir(&root).expect("create test root");
        let output_path = root.join("root.bin");

        let error = collect(&[11], 0, &output_path, 1, 10_000, 16 * 1024 * 1024)
            .expect_err("one progress step must not reach combat");

        assert!(error.contains("did not reach a combat boundary"));
        assert!(!output_path.exists());
        fs::remove_dir_all(&root).expect("remove test root");
    }

    #[test]
    fn collection_rejects_duplicate_or_excessive_seed_batches_before_writing() {
        let root = unique_temp_dir("seed-bounds");
        fs::create_dir(&root).expect("create test root");
        let duplicate_output = root.join("duplicate.bin");
        let excessive_output = root.join("excessive.bin");

        assert!(collect(
            &[11, 11],
            0,
            &duplicate_output,
            32,
            10_000,
            16 * 1024 * 1024,
        )
        .is_err());
        assert!(collect(
            &(0..=MAX_COLLECTED_ROOTS as u64).collect::<Vec<_>>(),
            0,
            &excessive_output,
            32,
            10_000,
            16 * 1024 * 1024,
        )
        .is_err());
        assert!(!duplicate_output.exists());
        assert!(!excessive_output.exists());
        fs::remove_dir_all(&root).expect("remove test root");
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "sts_learning_root_{label}_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ))
    }
}
