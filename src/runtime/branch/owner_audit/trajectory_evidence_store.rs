use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sts_simulator::ai::strategy::trajectory_comparison::{
    TrajectoryDeployabilityEvidence, TrajectorySnapshot,
};

use super::branch_model::Branch;
use super::run_capsule_io::write_json;
use super::trajectory_snapshot::{
    trajectory_evaluation_from_snapshots, trajectory_snapshot_with_deployability,
    FrontierTrajectoryEvaluation,
};

const TRAJECTORY_STATE_SCHEMA: &str = "branch_tiny_trajectory_state_v0";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct TrajectoryObservation {
    pub(super) generation: usize,
    pub(super) branch_id: usize,
    pub(super) parent_id: Option<usize>,
    pub(super) status: Value,
    pub(super) snapshot: TrajectorySnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct TrajectoryEvidenceState {
    pub(super) schema: String,
    pub(super) observations: Vec<TrajectoryObservation>,
    pub(super) evaluation: FrontierTrajectoryEvaluation,
}

impl TrajectoryEvidenceState {
    fn empty() -> Self {
        Self {
            schema: TRAJECTORY_STATE_SCHEMA.to_string(),
            observations: Vec::new(),
            evaluation: trajectory_evaluation_from_snapshots(Vec::new()),
        }
    }

    fn observe(
        &mut self,
        generation: usize,
        branch: &Branch,
        deployability: TrajectoryDeployabilityEvidence,
    ) -> Result<(), String> {
        let observation = TrajectoryObservation {
            generation,
            branch_id: branch.id,
            parent_id: branch.parent_id,
            status: serde_json::to_value(&branch.status)
                .map_err(|error| format!("serialize trajectory branch status: {error}"))?,
            snapshot: trajectory_snapshot_with_deployability(branch, deployability),
        };
        let lane = observation.snapshot.lane.as_str();
        match self
            .observations
            .iter_mut()
            .find(|existing| existing.snapshot.lane == lane)
        {
            Some(existing)
                if (observation.generation, observation.branch_id)
                    >= (existing.generation, existing.branch_id) =>
            {
                *existing = observation;
            }
            Some(_) => {}
            None => self.observations.push(observation),
        }
        self.refresh();
        Ok(())
    }

    fn refresh(&mut self) {
        self.observations.sort_by(|left, right| {
            lane_sort_key(&left.snapshot.lane).cmp(&lane_sort_key(&right.snapshot.lane))
        });
        self.evaluation = trajectory_evaluation_from_snapshots(
            self.observations
                .iter()
                .map(|observation| observation.snapshot.clone())
                .collect(),
        );
    }
}

pub(super) fn read_state(path: &Path) -> Result<TrajectoryEvidenceState, String> {
    let payload = match fs::read_to_string(path) {
        Ok(payload) => payload,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(TrajectoryEvidenceState::empty());
        }
        Err(error) => return Err(format!("read {}: {error}", path.display())),
    };
    let mut state: TrajectoryEvidenceState = serde_json::from_str(&payload)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    if state.schema != TRAJECTORY_STATE_SCHEMA {
        return Err(format!(
            "expected {TRAJECTORY_STATE_SCHEMA} in {}, got {}",
            path.display(),
            state.schema
        ));
    }
    state.refresh();
    Ok(state)
}

pub(super) fn record_frontier(
    path: &Path,
    generation: usize,
    frontier: &VecDeque<Branch>,
    deployment_by_branch: &BTreeMap<usize, TrajectoryDeployabilityEvidence>,
) -> Result<TrajectoryEvidenceState, String> {
    let mut state = read_state(path)?;
    for branch in frontier {
        state.observe(
            generation,
            branch,
            deployment_by_branch
                .get(&branch.id)
                .copied()
                .unwrap_or(TrajectoryDeployabilityEvidence::Unknown),
        )?;
    }
    persist(path, &state)?;
    Ok(state)
}

pub(super) fn record_branch(
    path: &Path,
    generation: usize,
    branch: &Branch,
    deployability: TrajectoryDeployabilityEvidence,
) -> Result<TrajectoryEvidenceState, String> {
    let mut state = read_state(path)?;
    state.observe(generation, branch, deployability)?;
    persist(path, &state)?;
    Ok(state)
}

fn persist(path: &Path, state: &TrajectoryEvidenceState) -> Result<(), String> {
    let value = serde_json::to_value(state)
        .map_err(|error| format!("serialize {}: {error}", path.display()))?;
    write_json(path, value)
}

fn lane_sort_key(lane: &str) -> (u8, u16, &str) {
    if lane == "baseline" {
        return (0, 0, lane);
    }
    let number = lane
        .strip_prefix("challenger-")
        .and_then(|value| value.parse().ok())
        .unwrap_or(u16::MAX);
    (1, number, lane)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;
    use sts_simulator::ai::strategy::trajectory_comparison::{
        TrajectoryPairEligibility, TrajectorySearchComparabilityStatus, TrajectoryTerminal,
    };
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

    use super::*;
    use crate::runtime::branch::owner_audit::branch_model::{BranchStatus, Owner};
    use crate::runtime::branch::owner_audit::branch_policy_lane::BranchPolicyLane;

    fn test_branch(id: usize, policy_lane: BranchPolicyLane) -> Branch {
        Branch {
            id,
            parent_id: Some(id.saturating_sub(1)),
            path: Vec::new(),
            session: RunControlSession::new(RunControlConfig::default()),
            status: BranchStatus::Running {
                owner: Owner::CardReward,
                boundary: "test".to_string(),
            },
            policy_lane,
            combat_portfolio: None,
            recent_progress_journal: Default::default(),
            recent_planner_capture: Default::default(),
            trajectory: Default::default(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            comparison_search_start: None,
            accepted_high_loss_diagnostics: Vec::new(),
        }
    }

    #[test]
    fn stopped_lanes_survive_reopen_and_form_final_comparison() {
        let root = std::env::temp_dir().join("trajectory_state_cross_slice");
        let path = root.join("trajectory_state.json");
        let _ = std::fs::remove_dir_all(&root);

        let mut baseline = test_branch(1, BranchPolicyLane::default());
        let mut challenger = test_branch(
            2,
            BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
        );
        let frontier = VecDeque::from([baseline.clone(), challenger.clone()]);
        record_frontier(&path, 10, &frontier, &BTreeMap::new()).unwrap();

        challenger.status = BranchStatus::CombatGap {
            boundary: "boss".to_string(),
            reason: "no win".to_string(),
        };
        challenger.session.run_state.current_hp = 47;
        record_branch(
            &path,
            40,
            &challenger,
            TrajectoryDeployabilityEvidence::Unknown,
        )
        .unwrap();

        baseline.status = BranchStatus::CombatGap {
            boundary: "boss".to_string(),
            reason: "no win".to_string(),
        };
        baseline.session.run_state.current_hp = 42;
        record_branch(
            &path,
            42,
            &baseline,
            TrajectoryDeployabilityEvidence::Unknown,
        )
        .unwrap();

        let state = read_state(&path).unwrap();
        assert_eq!(state.observations.len(), 2);
        assert_eq!(state.evaluation.snapshots.len(), 2);
        assert_eq!(state.evaluation.comparisons.len(), 1);
        assert_eq!(state.evaluation.snapshots[0].resources.hp, 42);
        assert_eq!(state.evaluation.snapshots[1].resources.hp, 47);
        assert!(state
            .evaluation
            .snapshots
            .iter()
            .all(|snapshot| snapshot.terminal == TrajectoryTerminal::CoverageLimited));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn projected_deployability_replaces_the_live_unknown_snapshot_layer() {
        let root = std::env::temp_dir().join("trajectory_state_projected_deployability");
        let path = root.join("trajectory_state.json");
        let _ = std::fs::remove_dir_all(&root);
        let branch = test_branch(1, BranchPolicyLane::default());
        let frontier = VecDeque::from([branch]);
        let evidence = TrajectoryDeployabilityEvidence::Comparable {
            claimed_answers: 6,
            timely_playable: 4,
        };
        let deployment_by_branch = BTreeMap::from([(1, evidence)]);

        let state = record_frontier(&path, 10, &frontier, &deployment_by_branch).unwrap();

        assert_eq!(state.evaluation.snapshots[0].deployability, evidence);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_trajectory_state_loads_and_refreshes_fail_closed() {
        let root = std::env::temp_dir().join("trajectory_state_legacy_comparability");
        let path = root.join("trajectory_state.json");
        let _ = std::fs::remove_dir_all(&root);
        let frontier = VecDeque::from([
            test_branch(1, BranchPolicyLane::default()),
            test_branch(
                2,
                BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
            ),
        ]);
        record_frontier(&path, 10, &frontier, &BTreeMap::new()).expect("record current state");

        let mut value: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("read current state"))
                .expect("parse current state");
        for observation in value["observations"].as_array_mut().expect("observations") {
            observation["snapshot"]
                .as_object_mut()
                .expect("observation snapshot")
                .remove("search_comparability");
            observation["snapshot"]
                .as_object_mut()
                .expect("observation snapshot")
                .remove("full_search_comparability");
        }
        for snapshot in value["evaluation"]["snapshots"]
            .as_array_mut()
            .expect("evaluation snapshots")
        {
            snapshot
                .as_object_mut()
                .expect("evaluation snapshot")
                .remove("search_comparability");
            snapshot
                .as_object_mut()
                .expect("evaluation snapshot")
                .remove("full_search_comparability");
        }
        for comparison in value["evaluation"]["comparisons"]
            .as_array_mut()
            .expect("evaluation comparisons")
        {
            comparison
                .as_object_mut()
                .expect("comparison")
                .remove("eligibility");
        }
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&value).expect("serialize legacy state"),
        )
        .expect("write legacy state");

        let restored = read_state(&path).expect("load legacy state");

        assert!(restored.evaluation.snapshots.iter().all(|snapshot| {
            snapshot.search_comparability.status
                == TrajectorySearchComparabilityStatus::InsufficientEvidence
        }));
        assert!(restored.evaluation.snapshots.iter().all(|snapshot| {
            snapshot.full_search_comparability.status
                == TrajectorySearchComparabilityStatus::InsufficientEvidence
        }));
        assert_eq!(
            restored.evaluation.comparisons[0].eligibility,
            TrajectoryPairEligibility::ExcludedInsufficientEvidence
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn malformed_existing_state_is_not_silently_replaced() {
        let root = std::env::temp_dir().join("trajectory_state_malformed");
        let path = root.join("trajectory_state.json");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&path, "not-json").unwrap();

        let error = record_branch(
            &path,
            1,
            &test_branch(1, BranchPolicyLane::default()),
            TrajectoryDeployabilityEvidence::Unknown,
        )
        .unwrap_err();

        assert!(error.contains("trajectory_state.json"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not-json");
        let _ = std::fs::remove_dir_all(root);
    }
}
