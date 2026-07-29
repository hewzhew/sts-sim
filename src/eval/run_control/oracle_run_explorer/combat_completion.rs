use serde::{Deserialize, Serialize};

use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleRunCombatEvidenceKindV1 {
    BudgetUnknown,
    ExhaustiveRefutation,
    SetupOrMechanicsError,
}

impl OracleRunCombatEvidenceKindV1 {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BudgetUnknown => "budget_unknown",
            Self::ExhaustiveRefutation => "exhaustive_refutation",
            Self::SetupOrMechanicsError => "setup_or_mechanics_error",
        }
    }
}

pub(super) enum FinishedOracleCombatV1 {
    Resolved(usize),
    ExactDuplicate,
    Unresolved(OracleRunUnresolvedCombatV1),
}

pub(super) fn classify_unresolved_combat_evidence(
    last_status: Option<&str>,
    generation_gap_count: usize,
) -> OracleRunCombatEvidenceKindV1 {
    match last_status {
        Some("frontier_exhausted") if generation_gap_count == 0 => {
            OracleRunCombatEvidenceKindV1::ExhaustiveRefutation
        }
        Some("mechanics_gap") | Some("replay_mismatch") => {
            OracleRunCombatEvidenceKindV1::SetupOrMechanicsError
        }
        _ => OracleRunCombatEvidenceKindV1::BudgetUnknown,
    }
}

impl OracleRunExplorerV1 {
    pub(super) fn finish_combat(
        &mut self,
        pending: PendingOracleCombatV1,
    ) -> Result<FinishedOracleCombatV1, String> {
        let parent = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == pending.branch_id)
            .cloned()
            .ok_or_else(|| format!("missing oracle combat branch {}", pending.branch_id))?;
        let progress = pending.work.progress();
        let mut session = parent.session.clone();
        // Deadlines bound search advancement. Once a verified witness is
        // ready, its exact replay is an atomic commit and is never interrupted
        // by a wall-clock deadline.
        let outcome = pending
            .work
            .finish_and_apply(&mut session)
            .map_err(|error| {
                format!(
                    "oracle combat branch {} at Act {} Floor {} failed to commit its witness: {error}",
                    parent.branch_id,
                    parent.session.run_state.act_num,
                    parent.session.run_state.floor_num
                )
            })?;
        if outcome.progress_steps.is_empty() {
            let rejection = outcome.combat_search_rejection.ok_or_else(|| {
                format!(
                    "oracle combat branch {} made no progress without typed rejection",
                    parent.branch_id
                )
            })?;
            let unresolved = OracleRunUnresolvedCombatV1 {
                branch_id: parent.branch_id,
                rejection,
                evidence_kind: classify_unresolved_combat_evidence(
                    progress.last_status,
                    progress.generation_gap_count,
                ),
                last_status: progress.last_status.map(str::to_string),
                generation_work: progress.generation_work,
                exact_states: progress.exact_states,
                applied_action_transitions: progress.applied_action_transitions,
                unique_successor_states: progress.unique_successor_states,
                duplicate_exact_successors: progress.duplicate_exact_successors,
                completed_turn_options: progress.completed_turn_options,
                retained_state_work: progress.retained_state_work,
                max_player_turn: progress.max_player_turn,
                max_path_atomic_depth: progress.max_path_atomic_depth,
                generation_gap_count: progress.generation_gap_count,
                incumbent_final_hp: progress.incumbent_final_hp,
            };
            return Ok(FinishedOracleCombatV1::Unresolved(unresolved));
        }
        self.accept_resolved_combat_branch(parent, session, outcome.progress_steps)
    }

    pub(super) fn accept_resolved_combat_branch(
        &mut self,
        parent: OracleRunBranchV1,
        session: RunControlSession,
        progress_steps: Vec<RunProgressStepV1>,
    ) -> Result<FinishedOracleCombatV1, String> {
        if progress_steps.len() != 1 {
            return Err(format!(
                "oracle combat branch {} committed {} progress steps; expected one",
                parent.branch_id,
                progress_steps.len()
            ));
        }
        let mut journal = parent.journal;
        journal.append_committed_steps(progress_steps)?;
        let child = OracleRunBranchV1 {
            branch_id: self.next_branch_id,
            parent_branch_id: Some(parent.branch_id),
            neow_root_candidate_id: parent.neow_root_candidate_id,
            neow_root_label: parent.neow_root_label,
            state_fingerprint: run_session_fingerprint_v2(&session),
            boundary: classify_run_boundary(&session),
            path_negative_log_policy: parent.path_negative_log_policy,
            path_discrepancy: parent.path_discrepancy,
            path_depth: parent.path_depth.saturating_add(1),
            replay: parent.replay,
            journal,
            session,
        };
        self.next_branch_id = self.next_branch_id.saturating_add(1);
        Ok(match self.accept_branch(child) {
            Some(branch_id) => FinishedOracleCombatV1::Resolved(branch_id),
            None => FinishedOracleCombatV1::ExactDuplicate,
        })
    }
}
