use std::collections::VecDeque;
use std::path::PathBuf;

use super::run_capsule::{RunCapsule, RunCapsuleSave};
use super::run_deadline::RunDeadline;
use super::run_slice_result::ArtifactWriteSummary;
use super::{frontier_checkpoint, Args, Branch};

pub(super) fn save_context_wall_stop(
    frontier_checkpoint_path: &Option<PathBuf>,
    resume_frontier: &Option<PathBuf>,
    capsule: Option<&RunCapsule>,
    args: Args,
    generation: usize,
    next_branch_id: usize,
    frontier: &VecDeque<Branch>,
    deadline: &RunDeadline,
    human_output: bool,
) -> Result<ArtifactWriteSummary, String> {
    save_wall_stop(
        frontier_checkpoint_output_path(frontier_checkpoint_path, resume_frontier, capsule),
        capsule,
        args,
        generation,
        next_branch_id,
        frontier,
        deadline,
        human_output,
    )
}

fn frontier_checkpoint_output_path<'a>(
    frontier_checkpoint_path: &'a Option<PathBuf>,
    resume_frontier: &'a Option<PathBuf>,
    capsule: Option<&RunCapsule>,
) -> Option<&'a PathBuf> {
    if frontier_checkpoint_path.is_some() {
        return frontier_checkpoint_path.as_ref();
    }
    if capsule.is_some() {
        return None;
    }
    resume_frontier.as_ref()
}

fn save_wall_stop(
    path: Option<&PathBuf>,
    capsule: Option<&RunCapsule>,
    args: Args,
    generation: usize,
    next_branch_id: usize,
    frontier: &VecDeque<Branch>,
    deadline: &RunDeadline,
    human_output: bool,
) -> Result<ArtifactWriteSummary, String> {
    let mut artifacts = ArtifactWriteSummary::default();
    if human_output {
        println!(
            "wall_soft_stop: generation={} remaining_ms={}",
            generation,
            deadline.remaining_ms().unwrap_or(0)
        );
    }
    if let Some(path) = path {
        let running = frontier
            .iter()
            .filter(|branch| branch.status.is_resumable())
            .count();
        if running == 0 {
            if human_output {
                println!("frontier_checkpoint skipped: no running branches");
            }
            return Ok(artifacts);
        }
        frontier_checkpoint::save(path, args, generation, next_branch_id, frontier)?;
        artifacts.merge(ArtifactWriteSummary::frontier_checkpoint_at(path.clone()));
        if human_output {
            println!(
                "frontier_checkpoint: {} running={}",
                path.display(),
                running
            );
        }
    } else if capsule.is_none() {
        if human_output {
            println!("wall_soft_stop reached without --frontier-checkpoint");
        }
    }
    if let Some(capsule) = capsule {
        let save = capsule.save_paused_recovery(
            args,
            generation,
            next_branch_id,
            frontier,
            "wall_deadline",
        )?;
        artifacts.merge(capsule.artifact_writes(save));
        print_capsule_save(save, capsule, human_output);
    }
    Ok(artifacts)
}

pub(super) fn print_capsule_save(
    save: RunCapsuleSave,
    capsule: &RunCapsule,
    human_output: bool,
) -> bool {
    match save {
        RunCapsuleSave::None => false,
        RunCapsuleSave::Frontier { running } => {
            if human_output {
                println!("run_capsule_frontier: running={running}");
            }
            true
        }
        RunCapsuleSave::Result => {
            if human_output {
                println!("run_capsule_result: {}", capsule.result_path().display());
            }
            true
        }
    }
}
