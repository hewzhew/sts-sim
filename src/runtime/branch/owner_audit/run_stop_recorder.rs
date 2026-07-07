use std::collections::VecDeque;
use std::path::PathBuf;

use super::run_capsule::RunCapsule;
use super::run_deadline::RunDeadline;
use super::run_slice_result::ArtifactWriteSummary;
use super::{run_persistence, Args, Branch};

pub(super) struct RunStopRecorder<'a> {
    frontier_checkpoint_path: &'a Option<PathBuf>,
    resume_frontier: &'a Option<PathBuf>,
    capsule: Option<&'a RunCapsule>,
    human_output: bool,
    frontier_saved: bool,
    artifact_writes: ArtifactWriteSummary,
}

impl<'a> RunStopRecorder<'a> {
    pub(super) fn new(
        frontier_checkpoint_path: &'a Option<PathBuf>,
        resume_frontier: &'a Option<PathBuf>,
        capsule: Option<&'a RunCapsule>,
        human_output: bool,
    ) -> Self {
        Self {
            frontier_checkpoint_path,
            resume_frontier,
            capsule,
            human_output,
            frontier_saved: false,
            artifact_writes: ArtifactWriteSummary::default(),
        }
    }

    pub(super) fn save_soft_wall(
        &mut self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
        deadline: &RunDeadline,
    ) -> Result<(), String> {
        let artifacts = run_persistence::save_context_wall_stop(
            self.frontier_checkpoint_path,
            self.resume_frontier,
            self.capsule,
            args,
            generation,
            next_branch_id,
            frontier,
            deadline,
            self.human_output,
        )?;
        self.frontier_saved |= artifacts.frontier_written || artifacts.result_written;
        self.artifact_writes.merge(artifacts);
        Ok(())
    }

    pub(super) fn save_recovery_if_needed(
        mut self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
    ) -> Result<ArtifactWriteSummary, String> {
        if let Some(capsule) = self.capsule.filter(|_| !self.frontier_saved) {
            let save = capsule.save_recovery(args, generation, next_branch_id, frontier)?;
            self.artifact_writes.merge(capsule.artifact_writes(save));
            run_persistence::print_capsule_save(save, capsule, self.human_output);
        }
        Ok(self.artifact_writes)
    }
}
