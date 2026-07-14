use std::collections::VecDeque;

use blake2::{Blake2b512, Digest};
use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::{
    build_decision_surface, DecisionCandidateKey, RunControlSessionCheckpointV1,
};

use super::{Branch, BranchStatus};

pub(super) const RUN_CUTPOINT_SCHEMA: &str = "branch_tiny_run_cutpoint_v1";
pub(super) const RUN_CUTPOINT_TRUST: &str = "exact_run_control_checkpoint_v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RunCutpointKind {
    PreCombatSearch,
    OwnerDecision,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RunCutpointManifestV1 {
    pub(super) schema: String,
    pub(super) kind: RunCutpointKind,
    pub(super) artifact_trust: String,
    pub(super) generation: usize,
    pub(super) branch_id: usize,
    pub(super) act: u8,
    pub(super) floor: i32,
    pub(super) boundary: String,
    pub(super) session_checkpoint_hash: String,
    pub(super) branch_control_hash: String,
    pub(super) candidate_count: usize,
    pub(super) candidate_set_hash: String,
    pub(super) candidate_order_hash: String,
}

#[derive(Clone)]
pub(super) struct RunCutpointSnapshot {
    pub(super) manifest: RunCutpointManifestV1,
    pub(super) branch: Branch,
}

impl RunCutpointSnapshot {
    pub(super) fn capture(
        kind: RunCutpointKind,
        generation: usize,
        branch: &Branch,
    ) -> Result<Self, String> {
        let candidate_keys = candidate_keys(branch);
        let mut sorted_keys = candidate_keys.clone();
        sorted_keys.sort_by_key(|key| serde_json::to_string(key).unwrap_or_default());
        let session_checkpoint_hash = checkpoint_hash(branch)?;
        let branch_control_hash = hash_serializable(&(
            branch.id,
            branch.parent_id,
            &branch.path,
            &branch.status,
            &branch.policy_lane,
            &session_checkpoint_hash,
        ))?;
        Ok(Self {
            manifest: RunCutpointManifestV1 {
                schema: RUN_CUTPOINT_SCHEMA.to_string(),
                kind,
                artifact_trust: RUN_CUTPOINT_TRUST.to_string(),
                generation,
                branch_id: branch.id,
                act: branch.session.run_state.act_num,
                floor: branch.session.run_state.floor_num,
                boundary: branch_boundary(&branch.status),
                session_checkpoint_hash,
                branch_control_hash,
                candidate_count: candidate_keys.len(),
                candidate_set_hash: hash_serializable(&sorted_keys)?,
                candidate_order_hash: hash_serializable(&candidate_keys)?,
            },
            branch: branch.clone(),
        })
    }

    pub(super) fn validate_branch(&self, branch: &Branch) -> Result<(), String> {
        if self.manifest.schema != RUN_CUTPOINT_SCHEMA
            || self.manifest.artifact_trust != RUN_CUTPOINT_TRUST
        {
            return Err("unsupported run cutpoint manifest".to_string());
        }
        let actual = Self::capture(self.manifest.kind, self.manifest.generation, branch)?;
        if actual.manifest.session_checkpoint_hash != self.manifest.session_checkpoint_hash {
            return Err(format!(
                "session checkpoint fingerprint mismatch: expected {}, got {}",
                self.manifest.session_checkpoint_hash, actual.manifest.session_checkpoint_hash
            ));
        }
        if actual.manifest.branch_control_hash != self.manifest.branch_control_hash {
            return Err(format!(
                "branch control fingerprint mismatch: expected {}, got {}",
                self.manifest.branch_control_hash, actual.manifest.branch_control_hash
            ));
        }
        if actual.manifest.act != self.manifest.act
            || actual.manifest.floor != self.manifest.floor
            || actual.manifest.boundary != self.manifest.boundary
        {
            return Err("cutpoint boundary mismatch".to_string());
        }
        if actual.manifest.candidate_count != self.manifest.candidate_count
            || actual.manifest.candidate_set_hash != self.manifest.candidate_set_hash
        {
            return Err("candidate set fingerprint mismatch".to_string());
        }
        if actual.manifest.candidate_order_hash != self.manifest.candidate_order_hash {
            return Err("candidate order fingerprint mismatch".to_string());
        }
        Ok(())
    }

    pub(super) fn into_resumable_frontier(self) -> VecDeque<Branch> {
        VecDeque::from([self.branch])
    }
}

fn checkpoint_hash(branch: &Branch) -> Result<String, String> {
    let mut checkpoint = RunControlSessionCheckpointV1::from_session(&branch.session);
    checkpoint.clear_combat_diagnostics_for_external_checkpoint();
    hash_serializable(&checkpoint)
}

fn candidate_keys(branch: &Branch) -> Vec<Option<DecisionCandidateKey>> {
    build_decision_surface(&branch.session)
        .view
        .candidates
        .into_iter()
        .map(|candidate| candidate.key)
        .collect()
}

fn branch_boundary(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { boundary, .. }
        | BranchStatus::AwaitingAuto { boundary, .. }
        | BranchStatus::AutomationGap { boundary, .. }
        | BranchStatus::CombatGap { boundary, .. }
        | BranchStatus::OperationBudgetExhausted { boundary, .. }
        | BranchStatus::BudgetGap { boundary, .. } => boundary.clone(),
        BranchStatus::Terminal(_) => "Terminal".to_string(),
        BranchStatus::ApplyFailed(_) | BranchStatus::AdvanceFailed(_) => "Failure".to_string(),
    }
}

fn hash_serializable<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    let mut hasher = Blake2b512::new();
    hasher.update(bytes);
    Ok(hasher.finalize()[..32]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use crate::content::relics::RelicId;
    use crate::state::core::EngineState;
    use crate::state::rewards::BossRelicChoiceState;

    use super::super::{Branch, BranchStatus, Owner};
    use super::*;

    fn boss_relic_branch() -> Branch {
        let args = crate::runtime::branch::default_branch_args(20260713006);
        let (mut frontier, _) = super::super::branch_runtime::BranchRuntime::initial_frontier(
            args,
            std::time::Instant::now(),
        );
        let mut branch = frontier.pop_front().unwrap();
        branch.session.run_state.act_num = 2;
        branch.session.run_state.floor_num = 32;
        branch.session.run_state.current_hp = 13;
        branch.session.run_state.max_hp = 101;
        branch.session.run_state.gold = 167;
        branch.session.engine_state =
            EngineState::BossRelicSelect(BossRelicChoiceState::new(vec![
                RelicId::BlackBlood,
                RelicId::CoffeeDripper,
                RelicId::PhilosopherStone,
            ]));
        branch.status = BranchStatus::Running {
            boundary: "Boss Relic".to_string(),
            owner: Owner::BossRelic,
        };
        branch
    }

    #[test]
    fn snapshot_fingerprints_full_session_and_candidate_order() {
        let branch = boss_relic_branch();
        let snapshot =
            RunCutpointSnapshot::capture(RunCutpointKind::OwnerDecision, 29, &branch).unwrap();

        assert_eq!(snapshot.manifest.act, 2);
        assert_eq!(snapshot.manifest.floor, 32);
        assert_eq!(snapshot.manifest.boundary, "Boss Relic");
        assert_eq!(snapshot.manifest.candidate_count, 4);
        assert!(!snapshot.manifest.session_checkpoint_hash.is_empty());
        assert!(!snapshot.manifest.branch_control_hash.is_empty());
        assert!(!snapshot.manifest.candidate_order_hash.is_empty());
        snapshot.validate_branch(&branch).unwrap();
    }

    #[test]
    fn validation_rejects_persistent_state_drift() {
        let branch = boss_relic_branch();
        let snapshot =
            RunCutpointSnapshot::capture(RunCutpointKind::OwnerDecision, 29, &branch).unwrap();
        let mut changed = branch.clone();
        changed.session.run_state.gold += 1;

        let error = snapshot.validate_branch(&changed).unwrap_err();
        assert!(error.contains("session checkpoint fingerprint mismatch"));
    }

    #[test]
    fn validation_rejects_candidate_order_drift() {
        let branch = boss_relic_branch();
        let mut snapshot =
            RunCutpointSnapshot::capture(RunCutpointKind::OwnerDecision, 29, &branch).unwrap();
        snapshot.manifest.candidate_order_hash = "tampered".to_string();

        let error = snapshot.validate_branch(&branch).unwrap_err();
        assert!(error.contains("candidate order fingerprint mismatch"));
    }

    #[test]
    fn validation_rejects_branch_history_drift() {
        let branch = boss_relic_branch();
        let snapshot =
            RunCutpointSnapshot::capture(RunCutpointKind::OwnerDecision, 29, &branch).unwrap();
        let mut changed = branch.clone();
        changed.parent_id = Some(999);

        let error = snapshot.validate_branch(&changed).unwrap_err();
        assert!(error.contains("branch control fingerprint mismatch"));
    }
}
