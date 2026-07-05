use std::collections::VecDeque;
use std::path::PathBuf;

use super::run_capsule::RunCapsule;
use super::run_deadline::RunDeadline;
use super::{run_persistence, Args, Branch};

pub(super) struct RunStopRecorder<'a> {
    frontier_checkpoint_path: &'a Option<PathBuf>,
    resume_frontier: &'a Option<PathBuf>,
    capsule: Option<&'a RunCapsule>,
    frontier_saved: bool,
}

impl<'a> RunStopRecorder<'a> {
    pub(super) fn new(
        frontier_checkpoint_path: &'a Option<PathBuf>,
        resume_frontier: &'a Option<PathBuf>,
        capsule: Option<&'a RunCapsule>,
    ) -> Self {
        Self {
            frontier_checkpoint_path,
            resume_frontier,
            capsule,
            frontier_saved: false,
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
        self.frontier_saved |= run_persistence::save_context_wall_stop(
            self.frontier_checkpoint_path,
            self.resume_frontier,
            self.capsule,
            args,
            generation,
            next_branch_id,
            frontier,
            deadline,
        )?;
        Ok(())
    }

    pub(super) fn save_recovery_if_needed(
        self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
    ) -> Result<(), String> {
        if let Some(capsule) = self.capsule.filter(|_| !self.frontier_saved) {
            run_persistence::print_capsule_save(
                capsule.save_recovery(args, generation, next_branch_id, frontier)?,
                capsule,
            );
        }
        Ok(())
    }
}
