use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::{CombatSearchTraceSummary, RunControlSessionCheckpointV1};

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::branch_path::BranchPathStep;
use super::run_contract::RunContract;
use super::{Args, Branch, BranchStatus};

#[derive(Deserialize, Serialize)]
pub(super) struct FrontierCheckpoint {
    schema: String,
    pub(super) args: Args,
    #[serde(default)]
    run_contract: Option<RunContract>,
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
    #[serde(default)]
    combat_search_history: Vec<CombatSearchTraceSummary>,
    #[serde(default)]
    accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
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
        run_contract: Some(RunContract::from_args(args)),
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
    #[allow(dead_code)]
    pub(super) fn run_contract(&self) -> RunContract {
        self.run_contract
            .unwrap_or_else(|| RunContract::from_args(self.args))
    }

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
            combat_search_history: branch.combat_search_history.clone(),
            accepted_high_loss_diagnostics: branch.accepted_high_loss_diagnostics.clone(),
        }
    }

    fn into_branch(self) -> Result<Branch, String> {
        Ok(Branch {
            id: self.id,
            parent_id: self.parent_id,
            path: self.path,
            session: self.session.into_session()?,
            status: self.status,
            policy_lane: super::branch_policy_lane::BranchPolicyLane::default(),
            combat_portfolio: None,
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
            combat_search_history: self.combat_search_history,
            accepted_high_loss_diagnostics: self.accepted_high_loss_diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_checkpoint_json() -> String {
        serde_json::json!({
            "schema": "branch_tiny_frontier_checkpoint",
            "args": {
                "seed": 44,
                "ascension": 2,
                "objective": "first_victory",
                "generations": 6,
                "max_branches": 4,
                "auto_ops": 9,
                "search_nodes": 10,
                "search_ms": 20,
                "rescue_search_nodes": 30,
                "rescue_search_ms": 40,
                "boss_search_nodes": 50,
                "boss_search_ms": 60,
                "wall_ms": 70
            },
            "generation": 1,
            "next_branch_id": 2,
            "frontier": []
        })
        .to_string()
    }

    #[test]
    fn legacy_checkpoint_without_run_contract_loads_contract_from_args() {
        let path = std::env::temp_dir().join("branch_tiny_legacy_frontier_checkpoint.json");
        fs::write(&path, legacy_checkpoint_json()).unwrap();

        let checkpoint = load(&path).unwrap();
        let contract = checkpoint.run_contract();

        assert_eq!(contract.game.seed, 44);
        assert_eq!(contract.game.ascension, 2);
        assert_eq!(contract.branching.generations, 6);
        assert_eq!(contract.slice.slice_ms, Some(70));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn checkpoint_writer_includes_run_contract() {
        let args = Args {
            seed: 45,
            ascension: 1,
            objective: super::super::run_contract::RunObjective::FirstVictory,
            generations: 2,
            max_branches: 1,
            auto_ops: 3,
            search_nodes: 4,
            search_ms: 5,
            rescue_search_nodes: 6,
            rescue_search_ms: 7,
            boss_search_nodes: 8,
            boss_search_ms: 9,
            wall_ms: Some(10),
            checkpoint_before_combat_portfolio: false,
            shop_boss_preview_bundle_limit: 0,
            shop_boss_preview_target_floor: None,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        };
        let path = std::env::temp_dir().join("branch_tiny_frontier_checkpoint_contract.json");
        let frontier = VecDeque::new();

        save(&path, args, 0, 1, &frontier).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(value["run_contract"]["game"]["seed"], 45);
        assert_eq!(value["run_contract"]["slice"]["slice_ms"], 10);
        assert_eq!(value["args"]["wall_ms"], 10);

        let _ = fs::remove_file(path);
    }
}
