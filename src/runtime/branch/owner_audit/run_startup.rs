use std::collections::VecDeque;
use std::path::Path;
use std::time::Instant;

use super::cli_args::{default_combat_gap_case_dir, parse_args};
use super::run_capsule::RunCapsule;
use super::run_cutpoint_store::RunCutpointStore;
use super::run_slice_request::RunSliceRequest;
use super::run_slice_result::{ArtifactWriteSummary, RunSliceRequestKind};
use super::{branch_runtime, event_owner_probe, frontier_checkpoint, run_chain, Branch};

pub(super) enum RunStartup {
    Delegated,
    Ready(RunSliceRequest),
}

pub(super) fn prepare() -> Result<RunStartup, String> {
    let (
        mut args,
        overrides,
        trace_path,
        mut combat_gap_case_dir,
        frontier_checkpoint_path,
        mut resume_frontier,
        mut run_capsule_path,
        resume_capsule_path,
        continue_capsule,
        event_owner_probe,
    ) = parse_args()?;
    if let Some(continue_capsule) = continue_capsule {
        run_chain::run(args, overrides, continue_capsule)?;
        return Ok(RunStartup::Delegated);
    }
    if let Some(path) = resume_capsule_path {
        if resume_frontier.is_some() || run_capsule_path.is_some() {
            return Err(
                "--resume-capsule cannot be combined with --resume-frontier or --run-capsule"
                    .to_string(),
            );
        }
        resume_frontier = Some(path.join("frontier.json"));
        run_capsule_path = Some(path);
    }
    if let Some(probe) = event_owner_probe {
        event_owner_probe::run(args, probe)?;
        return Ok(RunStartup::Delegated);
    }
    let run_capsule = run_capsule_path.map(RunCapsule::new);
    if combat_gap_case_dir.is_none() {
        combat_gap_case_dir = run_capsule
            .as_ref()
            .map(RunCapsule::combat_cases_dir)
            .or_else(|| {
                default_combat_gap_case_dir(
                    trace_path.as_ref(),
                    frontier_checkpoint_path.as_ref(),
                    resume_frontier.as_ref(),
                )
            });
    }
    let started = Instant::now();
    let mut generation_start = 0usize;
    let mut capsule_args;
    let (mut frontier, next_branch_id) = if let Some(path) = resume_frontier.as_ref() {
        let (checkpoint, frontier, next_branch_id) = load_resume_frontier(path)?;
        let requested_slice_generations =
            overrides.generations.unwrap_or(checkpoint.args.generations);
        capsule_args = checkpoint.args;
        overrides.apply_to(&mut capsule_args);
        args = checkpoint.args;
        overrides.apply_to(&mut args);
        generation_start = checkpoint.generation;
        args.generations = generation_start.saturating_add(requested_slice_generations);
        if args.wall_ms.is_none() {
            args.wall_ms = capsule_args.wall_ms;
        }
        (frontier, next_branch_id)
    } else {
        overrides.apply_to(&mut args);
        capsule_args = args;
        branch_runtime::BranchRuntime::initial_frontier(args, started)
    };
    let mut artifact_writes = ArtifactWriteSummary::default();
    if let Some(capsule) = run_capsule.as_ref() {
        capsule.prepare_trajectory_frontier(capsule_args, generation_start, &mut frontier)?;
        artifact_writes.merge(capsule.write_running_manifest(capsule_args)?);
    }
    Ok(RunStartup::Ready(RunSliceRequest {
        args,
        capsule_args,
        request_kind: if resume_frontier.is_some() {
            RunSliceRequestKind::ResumeFrontier
        } else {
            RunSliceRequestKind::Start
        },
        human_output: true,
        trace_path,
        combat_gap_case_dir,
        frontier_checkpoint_path,
        resume_frontier,
        run_capsule,
        artifact_writes,
        generation_start,
        frontier,
        next_branch_id,
        started,
    }))
}

fn load_resume_frontier(
    path: &Path,
) -> Result<
    (
        frontier_checkpoint::FrontierCheckpoint,
        VecDeque<Branch>,
        usize,
    ),
    String,
> {
    let checkpoint = frontier_checkpoint::load(path)?;
    let validation_copy = checkpoint.clone();
    let (frontier, next_branch_id) = validation_copy.into_frontier()?;
    RunCutpointStore::validate_resume_path(path, &frontier)?;
    Ok((checkpoint, frontier, next_branch_id))
}

#[cfg(test)]
mod tests {
    use crate::content::relics::RelicId;
    use crate::state::core::EngineState;
    use crate::state::rewards::BossRelicChoiceState;

    use super::super::run_cutpoint::{RunCutpointKind, RunCutpointSnapshot};
    use super::super::run_cutpoint_store::RunCutpointStore;
    use super::super::{BranchStatus, Owner};
    use super::*;

    #[test]
    fn manifested_resume_rejects_candidate_order_tampering() {
        let args = crate::runtime::branch::default_branch_args(20260713006);
        let (mut frontier, next_branch_id) =
            super::super::branch_runtime::BranchRuntime::initial_frontier(
                args,
                std::time::Instant::now(),
            );
        let mut branch = frontier.pop_front().unwrap();
        branch.session.run_state.act_num = 2;
        branch.session.run_state.floor_num = 32;
        branch.session.engine_state =
            EngineState::BossRelicSelect(BossRelicChoiceState::new(vec![
                RelicId::BlackBlood,
                RelicId::CoffeeDripper,
                RelicId::PhilosopherStone,
            ]));
        branch.status = BranchStatus::Running {
            boundary: "Boss Relic".to_string(),
            owner: Owner::BossRelic,
        };
        let store = RunCutpointStore::new(unique_root().join("cutpoints"));
        let snapshot =
            RunCutpointSnapshot::capture(RunCutpointKind::OwnerDecision, 29, &branch).unwrap();
        let frontier_path = store
            .write_boss_relic(args, next_branch_id, snapshot)
            .unwrap();
        let manifest_path = store.boss_relic_manifest_path(2, 32);
        let payload = std::fs::read_to_string(&manifest_path).unwrap();
        let mut manifest: serde_json::Value = serde_json::from_str(&payload).unwrap();
        manifest["candidate_order_hash"] = serde_json::Value::String("tampered".to_string());
        super::super::run_capsule_io::write_json(&manifest_path, manifest).unwrap();

        let error = match load_resume_frontier(&frontier_path) {
            Ok(_) => panic!("tampered cutpoint unexpectedly resumed"),
            Err(error) => error,
        };

        assert!(error.contains("candidate order fingerprint mismatch"));
    }

    #[test]
    fn legacy_frontier_without_manifest_still_resumes() {
        let args = crate::runtime::branch::default_branch_args(20260713006);
        let (frontier, next_branch_id) =
            super::super::branch_runtime::BranchRuntime::initial_frontier(
                args,
                std::time::Instant::now(),
            );
        let path = unique_root().join("legacy_frontier.json");
        frontier_checkpoint::save(&path, args, 7, next_branch_id, &frontier).unwrap();

        let (checkpoint, restored, restored_next_id) = load_resume_frontier(&path).unwrap();

        assert_eq!(checkpoint.generation, 7);
        assert_eq!(restored.len(), frontier.len());
        assert_eq!(restored_next_id, next_branch_id);
    }

    #[test]
    fn exact_boss_relic_cutpoint_round_trip_preserves_session_and_candidates() {
        let mut args = crate::runtime::branch::default_branch_args(20260713006);
        args.generations = 29;
        args.max_branches = 4;
        let (mut frontier, next_branch_id) =
            super::super::branch_runtime::BranchRuntime::initial_frontier(
                args,
                std::time::Instant::now(),
            );
        let mut original = frontier.pop_front().unwrap();
        original.session.run_state.act_num = 2;
        original.session.run_state.floor_num = 32;
        original.session.run_state.current_hp = 13;
        original.session.run_state.max_hp = 101;
        original.session.run_state.gold = 167;
        original.session.engine_state =
            EngineState::BossRelicSelect(BossRelicChoiceState::new(vec![
                RelicId::BlackBlood,
                RelicId::CoffeeDripper,
                RelicId::PhilosopherStone,
            ]));
        original.status = BranchStatus::Running {
            boundary: "Boss Relic".to_string(),
            owner: Owner::BossRelic,
        };
        let store = RunCutpointStore::new(unique_root().join("cutpoints"));
        let deadline =
            super::super::run_deadline::RunDeadline::new(std::time::Instant::now(), None);

        let _ = super::super::branch_scheduler::prepare_branch_work(
            original.clone(),
            args,
            args.generations,
            deadline,
            Some(&store),
            None,
            next_branch_id,
        )
        .unwrap();
        let path = store.boss_relic_frontier_path(2, 32);
        let (checkpoint, restored_frontier, _) = load_resume_frontier(&path).unwrap();
        let restored = restored_frontier.front().unwrap();

        assert_eq!(restored.session.run_state, original.session.run_state);
        assert_eq!(restored.session.engine_state, original.session.engine_state);
        assert_eq!(
            sts_simulator::eval::run_control::build_decision_surface(&restored.session)
                .view
                .candidates,
            sts_simulator::eval::run_control::build_decision_surface(&original.session)
                .view
                .candidates,
        );
        let restored_state_before_override = restored.session.run_state.clone();
        let restored_engine_before_override = restored.session.engine_state.clone();
        let mut overridden_args = checkpoint.args;
        overridden_args.max_branches = 1;
        assert_eq!(overridden_args.max_branches, 1);
        assert_eq!(restored.session.run_state, restored_state_before_override);
        assert_eq!(
            restored.session.engine_state,
            restored_engine_before_override
        );
    }

    fn unique_root() -> std::path::PathBuf {
        static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let ordinal = NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "sts_manifested_resume_{}_{}_{ordinal}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ))
    }
}
