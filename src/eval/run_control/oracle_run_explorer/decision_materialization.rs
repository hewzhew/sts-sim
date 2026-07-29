use super::*;

impl OracleRunExplorerV1 {
    pub(super) fn materialize_decision(
        &mut self,
        work: LazyOracleRunDecisionV1,
        decision_annotation: Option<OracleRunDecisionAnnotationFnV1>,
    ) -> Result<Option<usize>, String> {
        let parent = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == work.parent_branch_id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "oracle decision references missing parent branch {}",
                    work.parent_branch_id
                )
            })?;
        if parent.state_fingerprint != work.parent_state_fingerprint {
            return Err(format!(
                "oracle decision parent fingerprint changed for branch {}",
                work.parent_branch_id
            ));
        }

        let annotation =
            decision_annotation.and_then(|annotate| annotate(&parent.session, &work.candidate_id));
        let successor = super::super::exact_run_decision_successor_v1(
            &parent.session,
            &work.candidate_id,
            work.action.clone(),
        )?;
        let mut session = successor.session;
        let mut transaction = successor.transaction;
        if let Some(annotation) = annotation {
            transaction.trace_annotations.push(annotation);
        }
        let forced_steps = settle_oracle_forced_transitions(&mut session)?;
        let successor_fingerprint = run_session_fingerprint_v2(&session);
        if successor_fingerprint == parent.state_fingerprint {
            return Err(format!(
                "oracle decision '{}' ({}) produced no state change at branch {}; \
                 executable decision surfaces must not expose no-op actions",
                work.label, work.candidate_id, parent.branch_id
            ));
        }
        let mut journal = parent.journal;
        journal.append_committed_steps(vec![RunProgressStepV1::Decision(transaction)])?;
        journal.append_committed_steps(forced_steps)?;
        let mut replay = parent.replay;
        replay.push(OracleRunReplayStepV1 {
            candidate_id: work.candidate_id,
            label: work.label,
            action: work.action,
        });
        let child = OracleRunBranchV1 {
            branch_id: self.next_branch_id,
            parent_branch_id: Some(parent.branch_id),
            neow_root_candidate_id: parent.neow_root_candidate_id,
            neow_root_label: parent.neow_root_label,
            state_fingerprint: successor_fingerprint,
            boundary: classify_run_boundary(&session),
            path_negative_log_policy: work.path_negative_log_policy,
            path_discrepancy: work.path_discrepancy,
            path_depth: work.path_depth,
            replay,
            journal,
            session,
        };
        self.next_branch_id = self.next_branch_id.saturating_add(1);
        Ok(self.accept_branch(child))
    }

    pub(in super::super) fn materialize_explicit_decision(
        &mut self,
        work: LazyOracleRunDecisionV1,
        decision_annotation: Option<OracleRunDecisionAnnotationFnV1>,
    ) -> Result<usize, String> {
        let duplicate_count = self.retired_exact_duplicates.len();
        if let Some(branch_id) = self.materialize_decision(work, decision_annotation)? {
            return Ok(branch_id);
        }
        self.retired_exact_duplicates
            .get(duplicate_count)
            .map(|duplicate| duplicate.survivor_branch_id)
            .ok_or_else(|| {
                "explicit oracle decision was discarded without an exact-duplicate record"
                    .to_string()
            })
    }
}

pub(super) fn settle_oracle_forced_transitions(
    session: &mut RunControlSession,
) -> Result<Vec<RunProgressStepV1>, String> {
    let mut steps = Vec::new();
    if matches!(session.engine_state, EngineState::Campfire)
        && crate::engine::campfire_handler::get_available_options(&session.run_state).is_empty()
    {
        let transition = session.execute_forced_transition(
            super::super::RunForcedTransitionKindV1::EmptyCampfireExit,
        )?;
        steps.push(RunProgressStepV1::ForcedTransition(transition));
    }
    Ok(steps)
}
