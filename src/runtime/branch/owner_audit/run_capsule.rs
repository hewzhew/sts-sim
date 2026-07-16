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
    use super::super::frontier_checkpoint;
    use super::*;

    #[test]
    fn cutpoints_live_under_the_capsule_root() {
        let root = std::env::temp_dir().join("sts_capsule_cutpoint_root");

        let capsule = RunCapsule::new(root.clone());

        assert_eq!(capsule.cutpoints_dir(), root.join("cutpoints"));
    }

    #[test]
    fn resumed_capsule_preserves_and_extends_the_same_trajectory_head() {
        let root = std::env::temp_dir().join(format!(
            "sts_capsule_trajectory_resume_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let args = crate::runtime::branch::default_branch_args(20260716001);
        let capsule = RunCapsule::new(root.clone());
        let (mut frontier, next_branch_id) =
            super::super::branch_runtime::BranchRuntime::initial_frontier(
                args,
                std::time::Instant::now(),
            );
        let journal = frontier.front().unwrap().recent_progress_journal.clone();
        let capture = frontier.front().unwrap().recent_planner_capture.clone();
        capsule
            .prepare_trajectory_frontier(args, 0, &mut frontier)
            .unwrap();
        capsule.write_running_manifest(args).unwrap();
        capsule.commit_frontier_trajectories(&mut frontier).unwrap();
        let first_head = frontier
            .front()
            .unwrap()
            .trajectory
            .committed_head()
            .unwrap()
            .clone();
        capsule
            .save_paused_recovery(args, 0, next_branch_id, &frontier, "test_slice_end")
            .unwrap();

        let checkpoint = frontier_checkpoint::load(&root.join("frontier.json")).unwrap();
        let (mut resumed, _) = checkpoint.into_frontier().unwrap();
        let resumed_capsule = RunCapsule::new(root.clone());
        resumed_capsule
            .prepare_trajectory_frontier(args, 0, &mut resumed)
            .unwrap();
        assert_eq!(
            resumed.front().unwrap().trajectory.committed_head(),
            Some(&first_head)
        );
        resumed.front_mut().unwrap().recent_progress_journal = journal;
        resumed.front_mut().unwrap().recent_planner_capture = capture;
        resumed
            .front_mut()
            .unwrap()
            .capture_recent_trajectory(1)
            .unwrap();
        resumed_capsule
            .commit_frontier_trajectories(&mut resumed)
            .unwrap();
        let second_head = resumed
            .front()
            .unwrap()
            .trajectory
            .committed_head()
            .unwrap();
        assert_eq!(second_head.depth, first_head.depth + 1);
        assert_ne!(second_head.segment_id, first_head.segment_id);
        let run_id = resumed_capsule.store.trajectory_run_id(args).unwrap();
        resumed_capsule
            .store
            .verify_branch_trajectory(&run_id, resumed.front().unwrap())
            .unwrap();

        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["trajectory_run_id"], run_id);
        let _ = std::fs::remove_dir_all(root);
    }
}
