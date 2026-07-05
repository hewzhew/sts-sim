use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::RunControlSessionCheckpointV1;

use super::branch_path::BranchPathStep;
use super::{Args, Branch, BranchStatus};

#[derive(Deserialize, Serialize)]
pub(super) struct FrontierCheckpoint {
    schema: String,
    pub(super) args: Args,
    pub(super) generation: usize,
    next_branch_id: usize,
    frontier: Vec<BranchCheckpoint>,
}

#[derive(Deserialize, Serialize)]
struct BranchCheckpoint {
    id: usize,
    parent_id: Option<usize>,
    path: Vec<BranchPathStep>,
    session: RunControlSessionCheckpointV1,
    status: BranchStatus,
}

pub(super) fn save(
    path: &Path,
    args: Args,
    generation: usize,
    next_branch_id: usize,
    frontier: &VecDeque<Branch>,
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let checkpoint = FrontierCheckpoint {
        schema: "branch_tiny_frontier_checkpoint".to_string(),
        args,
        generation,
        next_branch_id,
        frontier: frontier
            .iter()
            .filter(|branch| branch.status.is_resumable())
            .map(BranchCheckpoint::from_branch)
            .collect(),
    };
    let payload = serde_json::to_string_pretty(&checkpoint).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

pub(super) fn load(path: &Path) -> Result<FrontierCheckpoint, String> {
    let payload = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&payload)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

impl FrontierCheckpoint {
    pub(super) fn into_frontier(self) -> Result<(VecDeque<Branch>, usize), String> {
        let mut frontier = VecDeque::new();
        for branch in self.frontier {
            frontier.push_back(branch.into_branch()?);
        }
        Ok((frontier, self.next_branch_id))
    }
}

impl BranchCheckpoint {
    fn from_branch(branch: &Branch) -> Self {
        let mut session = RunControlSessionCheckpointV1::from_session(&branch.session);
        session.clear_combat_diagnostics_for_external_checkpoint();
        Self {
            id: branch.id,
            parent_id: branch.parent_id,
            path: branch.path.clone(),
            session,
            status: branch.status.clone(),
        }
    }

    fn into_branch(self) -> Result<Branch, String> {
        Ok(Branch {
            id: self.id,
            parent_id: self.parent_id,
            path: self.path,
            session: self.session.into_session()?,
            status: self.status,
            combat_portfolio: None,
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
        })
    }
}
