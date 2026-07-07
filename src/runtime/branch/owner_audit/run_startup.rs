use std::time::Instant;

use super::cli_args::{default_combat_gap_case_dir, parse_args};
use super::run_capsule::RunCapsule;
use super::run_slice_request::RunSliceRequest;
use super::run_slice_result::{ArtifactWriteSummary, RunSliceRequestKind};
use super::{branch_runtime, event_owner_probe, frontier_checkpoint, run_chain};

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
    let mut artifact_writes = ArtifactWriteSummary::default();
    if let Some(capsule) = run_capsule.as_ref() {
        artifact_writes.merge(capsule.write_running_manifest(args)?);
    }
    let started = Instant::now();
    let mut generation_start = 0usize;
    let (frontier, next_branch_id) = if let Some(path) = resume_frontier.as_ref() {
        let checkpoint = frontier_checkpoint::load(path)?;
        args = checkpoint.args;
        overrides.apply_to(&mut args);
        generation_start = checkpoint.generation;
        checkpoint.into_frontier()?
    } else {
        branch_runtime::BranchRuntime::initial_frontier(args, started)
    };
    Ok(RunStartup::Ready(RunSliceRequest {
        args,
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
