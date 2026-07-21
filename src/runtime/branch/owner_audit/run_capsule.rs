use std::collections::VecDeque;
use std::path::PathBuf;

use super::capsule_artifact_store::CapsuleArtifactStore;
use super::run_slice_result::{ArtifactWriteSummary, RunSliceRequestKind, RunSliceResult};
use super::{Args, Branch, BranchStatus, TerminalOutcome};

pub(super) struct RunCapsule {
    store: CapsuleArtifactStore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RunCapsuleSave {
    None,
    Frontier { running: usize },
    Result,
}

impl RunCapsule {
    pub(super) fn new(root: PathBuf) -> Self {
        Self {
            store: CapsuleArtifactStore::new(root),
        }
    }

    pub(super) fn combat_cases_dir(&self) -> PathBuf {
        self.store.combat_cases_dir()
    }

    pub(super) fn cutpoints_dir(&self) -> PathBuf {
        self.store.root_path().join("cutpoints")
    }

    pub(super) fn result_path(&self) -> PathBuf {
        self.store.result_path()
    }

    pub(super) fn prepare_trajectory_frontier(
        &self,
        args: Args,
        generation: usize,
        frontier: &mut VecDeque<Branch>,
    ) -> Result<(), String> {
        let run_id = self.store.trajectory_run_id(args)?;
        for branch in frontier {
            branch.bind_trajectory_run(&run_id, generation)?;
            self.store.verify_branch_trajectory(&run_id, branch)?;
        }
        Ok(())
    }

    pub(super) fn commit_branch_trajectory(
        &self,
        branch: &mut Branch,
    ) -> Result<ArtifactWriteSummary, String> {
        self.store.commit_branch_trajectory(branch)
    }

    pub(super) fn commit_frontier_trajectories(
        &self,
        frontier: &mut VecDeque<Branch>,
    ) -> Result<ArtifactWriteSummary, String> {
        let mut summary = ArtifactWriteSummary::default();
        for branch in frontier {
            summary.merge(self.commit_branch_trajectory(branch)?);
        }
        Ok(summary)
    }

    pub(super) fn project_branch_trajectory(
        &self,
        branch: &Branch,
    ) -> Result<Option<super::trajectory_projector::RunTrajectoryProjectionBundleV1>, String> {
        self.store.project_branch_trajectory(branch)
    }

    pub(super) fn write_running_manifest(
        &self,
        args: Args,
    ) -> Result<ArtifactWriteSummary, String> {
        self.store.write_running_manifest(args)?;
        Ok(self.store.running_manifest_summary())
    }

    pub(super) fn artifact_writes(&self, save: RunCapsuleSave) -> ArtifactWriteSummary {
        match save {
            RunCapsuleSave::None => ArtifactWriteSummary::default(),
            RunCapsuleSave::Frontier { .. } => self.store.frontier_summary(),
            RunCapsuleSave::Result => self.store.result_summary(),
        }
    }

    pub(super) fn save_recovery(
        &self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
    ) -> Result<RunCapsuleSave, String> {
        if let Some(branch) = frontier.iter().find(|branch| {
            matches!(
                branch.status,
                BranchStatus::Terminal(TerminalOutcome::Victory)
            )
        }) {
            self.save_completed_result(args, generation, branch, "victory_found")?;
            return Ok(RunCapsuleSave::Result);
        }
        if let Some(save) =
            self.save_frontier(args, generation, next_branch_id, frontier, "running", None)?
        {
            return Ok(save);
        }
        if let Some(branch) = frontier.front() {
            self.save_result(args, generation, branch)?;
            return Ok(RunCapsuleSave::Result);
        }
        Ok(RunCapsuleSave::None)
    }

    pub(super) fn save_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
    ) -> Result<(), String> {
        self.store.write_result(args, generation, branch)
    }

    pub(super) fn save_completed_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
        reason: &'static str,
    ) -> Result<(), String> {
        self.store
            .write_completed_result(args, generation, branch, reason)
    }

    pub(super) fn save_paused_recovery(
        &self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
        reason: &'static str,
    ) -> Result<RunCapsuleSave, String> {
        if let Some(save) = self.save_frontier(
            args,
            generation,
            next_branch_id,
            frontier,
            "paused",
            Some(reason),
        )? {
            return Ok(save);
        }
        self.save_recovery(args, generation, next_branch_id, frontier)
    }

    pub(super) fn save_terminal_result(
        &self,
        args: Args,
        generation: usize,
        branch: &Branch,
    ) -> Result<ArtifactWriteSummary, String> {
        if self
            .store
            .append_terminal_result(args, generation, branch)?
        {
            return Ok(self.store.terminal_summary());
        }
        Ok(ArtifactWriteSummary::default())
    }

    pub(super) fn record_stopped_trajectory(
        &self,
        generation: usize,
        branch: &Branch,
    ) -> Result<ArtifactWriteSummary, String> {
        self.store.record_stopped_trajectory(generation, branch)?;
        Ok(self.store.trajectory_evidence_summary())
    }

    pub(super) fn append_slice_ledger(&self, result: &RunSliceResult) -> Result<(), String> {
        self.store.append_slice_ledger(result)
    }

    pub(super) fn append_slice_started_ledger(
        &self,
        args: Args,
        request_kind: RunSliceRequestKind,
        generation_start: usize,
        artifacts: &ArtifactWriteSummary,
    ) -> Result<(), String> {
        self.store
            .append_slice_started_ledger(args, request_kind, generation_start, artifacts)
    }

    fn save_frontier(
        &self,
        args: Args,
        generation: usize,
        next_branch_id: usize,
        frontier: &VecDeque<Branch>,
        capsule_status: &'static str,
        reason: Option<&'static str>,
    ) -> Result<Option<RunCapsuleSave>, String> {
        Ok(self
            .store
            .write_frontier(
                args,
                generation,
                next_branch_id,
                frontier,
                capsule_status,
                reason,
            )?
            .map(|running| RunCapsuleSave::Frontier { running }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cutpoints_live_under_the_capsule_root() {
        let root = std::env::temp_dir().join("sts_capsule_cutpoint_root");

        let capsule = RunCapsule::new(root.clone());

        assert_eq!(capsule.cutpoints_dir(), root.join("cutpoints"));
    }
}
