use std::collections::VecDeque;
use std::path::PathBuf;

use super::capsule_artifact_store::CapsuleArtifactStore;
use super::run_slice_result::ArtifactWriteSummary;
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

impl RunCapsuleSave {
    pub(super) fn artifact_writes(self) -> ArtifactWriteSummary {
        match self {
            Self::None => ArtifactWriteSummary::default(),
            Self::Frontier { .. } => ArtifactWriteSummary {
                manifest_written: true,
                frontier_written: true,
                summary_written: true,
                ..ArtifactWriteSummary::default()
            },
            Self::Result => ArtifactWriteSummary {
                manifest_written: true,
                result_written: true,
                path_written: true,
                summary_written: true,
                ..ArtifactWriteSummary::default()
            },
        }
    }
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

    pub(super) fn result_path(&self) -> PathBuf {
        self.store.result_path()
    }

    pub(super) fn write_running_manifest(&self, args: Args) -> Result<(), String> {
        self.store.write_running_manifest(args)
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
            return Ok(ArtifactWriteSummary {
                terminal_written: true,
                ..ArtifactWriteSummary::default()
            });
        }
        Ok(ArtifactWriteSummary::default())
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
