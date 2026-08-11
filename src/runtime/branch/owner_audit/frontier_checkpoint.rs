use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::{
    AtomicCombatSearchTraceSummaryV2, RunControlSessionCheckpointV1,
};
use sts_simulator::runtime::branch::RunTrajectoryHeadV1;

use super::accepted_high_loss_diagnostic::AcceptedHighLossDiagnosticDraft;
use super::branch_path::BranchPathStep;
use super::branch_policy_lane::BranchPolicyLane;
use super::run_contract::RunContract;
use super::run_identity::{current_source_identity, SourceIdentity};
use super::{Args, Branch, BranchStatus};

const FRONTIER_CHECKPOINT_SCHEMA: &str = "branch_tiny_frontier_checkpoint_v3";

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct FrontierCheckpoint {
    schema: String,
    source_identity: SourceIdentity,
    pub(super) runtime_args: Args,
    pub(super) generation: usize,
    next_branch_id: usize,
    frontier: Vec<BranchCheckpoint>,
}

#[derive(Clone, Deserialize, Serialize)]
struct BranchCheckpoint {
    id: usize,
    parent_id: Option<usize>,
    path: Vec<BranchPathStep>,
    session: RunControlSessionCheckpointV1,
    status: BranchStatus,
    #[serde(default)]
    policy_lane: BranchPolicyLane,
    #[serde(default)]
    atomic_combat_search_history: Vec<AtomicCombatSearchTraceSummaryV2>,
    #[serde(default)]
    comparison_search_start: Option<usize>,
    #[serde(default)]
    accepted_high_loss_diagnostics: Vec<AcceptedHighLossDiagnosticDraft>,
    #[serde(default)]
    trajectory_head: Option<RunTrajectoryHeadV1>,
}

pub(super) fn save(
    path: &Path,
    args: Args,
    generation: usize,
    next_branch_id: usize,
    frontier: &VecDeque<Branch>,
) -> Result<(), String> {
    let checkpoint = FrontierCheckpoint {
        schema: FRONTIER_CHECKPOINT_SCHEMA.to_string(),
        source_identity: current_source_identity(),
        runtime_args: args,
        generation,
        next_branch_id,
        frontier: frontier
            .iter()
            .filter(|branch| branch.status.is_resumable())
            .map(BranchCheckpoint::from_branch)
            .collect::<Result<Vec<_>, _>>()?,
    };
    let value = serde_json::to_value(&checkpoint).map_err(|error| error.to_string())?;
    super::run_capsule_io::write_json(path, value)
}

pub(super) fn load(path: &Path) -> Result<FrontierCheckpoint, String> {
    let payload = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&payload)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("<missing>");
    if schema != FRONTIER_CHECKPOINT_SCHEMA {
        return Err(format!(
            "unsupported frontier checkpoint schema at {}: expected {FRONTIER_CHECKPOINT_SCHEMA}, got {schema}",
            path.display()
        ));
    }
    let checkpoint: FrontierCheckpoint = serde_json::from_value(value)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    let current_source = current_source_identity();
    if checkpoint.source_identity != current_source {
        return Err(format!(
            "frontier source identity mismatch at {}: checkpoint={:?}, current={current_source:?}",
            path.display(),
            checkpoint.source_identity
        ));
    }
    Ok(checkpoint)
}

impl FrontierCheckpoint {
    #[allow(dead_code)]
    pub(super) fn run_contract(&self) -> RunContract {
        RunContract::from_args(self.runtime_args)
    }

    pub(super) fn source_identity(&self) -> &SourceIdentity {
        &self.source_identity
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
    fn from_branch(branch: &Branch) -> Result<Self, String> {
        let mut session = RunControlSessionCheckpointV1::from_session(&branch.session);
        session.clear_combat_diagnostics_for_external_checkpoint();
        Ok(Self {
            id: branch.id,
            parent_id: branch.parent_id,
            path: branch.path.clone(),
            session,
            status: branch.status.clone(),
            policy_lane: branch.policy_lane.clone(),
            atomic_combat_search_history: branch.atomic_combat_search_history.clone(),
            comparison_search_start: branch.comparison_search_start,
            accepted_high_loss_diagnostics: branch.accepted_high_loss_diagnostics.clone(),
            trajectory_head: branch.trajectory.checkpoint_head()?,
        })
    }

    fn into_branch(self) -> Result<Branch, String> {
        Ok(Branch {
            id: self.id,
            parent_id: self.parent_id,
            path: self.path,
            session: self.session.into_session()?,
            status: self.status,
            policy_lane: self.policy_lane,
            atomic_combat_search_session: None,
            recent_progress_journal: Default::default(),
            recent_planner_capture: Default::default(),
            trajectory: super::branch_trajectory::BranchTrajectoryState::from_checkpoint_head(
                self.trajectory_head,
            ),
            atomic_combat_search_attempts: Vec::new(),
            atomic_combat_search_history: self.atomic_combat_search_history,
            comparison_search_start: self.comparison_search_start,
            accepted_high_loss_diagnostics: self.accepted_high_loss_diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use sts_simulator::ai::strategy::candidate_pressure_response::CandidatePressureResponse;
    use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;

    #[test]
    fn checkpoint_writer_uses_v3_runtime_args_source_identity_and_derived_contract() {
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
            checkpoint_before_atomic_combat_search_session: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        };
        let path = std::env::temp_dir().join("branch_tiny_frontier_checkpoint_contract.json");
        let frontier = VecDeque::new();

        save(&path, args, 0, 1, &frontier).unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        assert_eq!(value["schema"], FRONTIER_CHECKPOINT_SCHEMA);
        assert_eq!(
            value["source_identity"],
            serde_json::to_value(current_source_identity()).unwrap()
        );
        assert_eq!(value["runtime_args"]["wall_ms"], 10);
        assert!(value.get("args").is_none());
        assert!(value.get("run_contract").is_none());

        let checkpoint = load(&path).unwrap();
        let contract = checkpoint.run_contract();
        assert_eq!(contract.game.seed, 45);
        assert_eq!(contract.slice.slice_ms, Some(10));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn v2_checkpoint_is_rejected_instead_of_silently_upgraded() {
        let path = std::env::temp_dir().join("branch_tiny_frontier_checkpoint_v2_rejected.json");
        fs::write(&path, r#"{"schema":"branch_tiny_frontier_checkpoint_v2"}"#).unwrap();

        let error = match load(&path) {
            Ok(_) => panic!("V2 checkpoint unexpectedly loaded"),
            Err(error) => error,
        };

        assert!(error.contains("unsupported frontier checkpoint schema"));
        assert!(error.contains("branch_tiny_frontier_checkpoint_v2"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn mismatched_source_identity_is_rejected_before_resume() {
        let args = crate::runtime::branch::default_branch_args(47);
        let path = std::env::temp_dir().join("branch_tiny_frontier_source_mismatch.json");
        save(&path, args, 0, 1, &VecDeque::new()).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        value["source_identity"]["git_commit"] = serde_json::json!("different-source");
        fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();

        let error = match load(&path) {
            Ok(_) => panic!("mismatched source checkpoint unexpectedly loaded"),
            Err(error) => error,
        };

        assert!(error.contains("frontier source identity mismatch"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn challenger_policy_survives_frontier_checkpoint_round_trip() {
        let args = Args {
            seed: 46,
            ascension: 0,
            objective: super::super::run_contract::RunObjective::FirstVictory,
            generations: 2,
            max_branches: 3,
            auto_ops: 3,
            search_nodes: 4,
            search_ms: 5,
            rescue_search_nodes: 6,
            rescue_search_ms: 7,
            boss_search_nodes: 8,
            boss_search_ms: 9,
            wall_ms: None,
            checkpoint_before_atomic_combat_search_session: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        };
        let path = std::env::temp_dir().join("branch_tiny_challenger_policy_round_trip.json");
        let (mut frontier, next_branch_id) =
            super::super::branch_runtime::BranchRuntime::initial_frontier(args, Instant::now());
        let mut policy = ChallengerPolicyState::new(1);
        policy.record_divergence("a1f5", &CandidatePressureResponse::default());
        policy.record_divergence("a1f7", &CandidatePressureResponse::default());
        frontier.front_mut().unwrap().policy_lane =
            super::super::branch_policy_lane::BranchPolicyLane::challenger(policy);
        frontier.front_mut().unwrap().comparison_search_start = Some(7);

        save(&path, args, 2, next_branch_id, &frontier).unwrap();
        let (restored, _) = load(&path).unwrap().into_frontier().unwrap();

        let restored_policy = restored
            .front()
            .unwrap()
            .policy_lane
            .challenger_policy()
            .expect("challenger lane should survive checkpoint");
        assert_eq!(restored_policy.divergence_count, 2);
        assert_eq!(restored_policy.last_checkpoint_ref.as_deref(), Some("a1f7"));
        assert_eq!(restored.front().unwrap().comparison_search_start, Some(7));

        let _ = fs::remove_file(path);
    }
}
