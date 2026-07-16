use std::collections::VecDeque;

use sts_simulator::eval::run_control::{PlannerBoundaryCaptureSegmentV1, RunProgressJournalV1};
use sts_simulator::runtime::branch::{
    build_run_trajectory_segment_v1, RunTrajectoryHeadV1, RunTrajectoryPolicyLaneV1,
    RunTrajectorySegmentDispositionV1, RunTrajectorySegmentDraftV1,
};

use super::branch_model::{BranchStatus, TerminalOutcome};

#[derive(Clone, Default)]
pub(super) struct BranchTrajectoryState {
    run_id: Option<String>,
    head: Option<RunTrajectoryHeadV1>,
    pending: VecDeque<RunTrajectorySegmentDraftV1>,
}

impl BranchTrajectoryState {
    pub(super) fn from_checkpoint_head(head: Option<RunTrajectoryHeadV1>) -> Self {
        Self {
            run_id: None,
            head,
            pending: VecDeque::new(),
        }
    }

    pub(super) fn bind_run(
        &mut self,
        run_id: &str,
        branch_id: usize,
        policy_lane: RunTrajectoryPolicyLaneV1,
        generation: usize,
        status: &BranchStatus,
        journal: &RunProgressJournalV1,
        capture: &PlannerBoundaryCaptureSegmentV1,
    ) -> Result<(), String> {
        match self.run_id.as_deref() {
            Some(existing) if existing != run_id => {
                return Err(format!(
                    "branch trajectory run mismatch: expected {existing}, got {run_id}"
                ));
            }
            Some(_) => return Ok(()),
            None => self.run_id = Some(run_id.to_string()),
        }
        self.append_recent(branch_id, policy_lane, generation, status, journal, capture)
    }

    pub(super) fn append_recent(
        &mut self,
        branch_id: usize,
        policy_lane: RunTrajectoryPolicyLaneV1,
        generation: usize,
        status: &BranchStatus,
        journal: &RunProgressJournalV1,
        capture: &PlannerBoundaryCaptureSegmentV1,
    ) -> Result<(), String> {
        let Some(run_id) = self.run_id.as_deref() else {
            return Ok(());
        };
        let parent = self
            .pending
            .back()
            .map(|draft| draft.head())
            .or_else(|| self.head.clone());
        let draft = build_run_trajectory_segment_v1(
            run_id,
            branch_id as u64,
            policy_lane,
            generation as u64,
            parent.as_ref(),
            disposition(status),
            journal,
            capture,
        )
        .map_err(|gap| format!("trajectory integrity gap: {gap}"))?;
        if let Some(draft) = draft {
            self.pending.push_back(draft);
        }
        Ok(())
    }

    pub(super) fn checkpoint_head(&self) -> Result<Option<RunTrajectoryHeadV1>, String> {
        if !self.pending.is_empty() {
            return Err(format!(
                "branch trajectory has {} uncommitted segment(s)",
                self.pending.len()
            ));
        }
        Ok(self.head.clone())
    }

    pub(super) fn run_id(&self) -> Option<&str> {
        self.run_id.as_deref()
    }

    pub(super) fn committed_head(&self) -> Option<&RunTrajectoryHeadV1> {
        self.head.as_ref()
    }

    pub(super) fn pending_front(&self) -> Option<&RunTrajectorySegmentDraftV1> {
        self.pending.front()
    }

    pub(super) fn mark_front_committed(&mut self, segment_id: &str) -> Result<(), String> {
        let Some(front) = self.pending.front() else {
            return Err("cannot commit an empty branch trajectory queue".to_string());
        };
        if front.segment.segment_id != segment_id {
            return Err(format!(
                "trajectory commit order mismatch: expected {}, got {segment_id}",
                front.segment.segment_id
            ));
        }
        let head = front.head();
        self.pending.pop_front();
        self.head = Some(head);
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

fn disposition(status: &BranchStatus) -> RunTrajectorySegmentDispositionV1 {
    match status {
        BranchStatus::Terminal(TerminalOutcome::Victory) => {
            RunTrajectorySegmentDispositionV1::TerminalVictory
        }
        BranchStatus::Terminal(TerminalOutcome::Defeat) => {
            RunTrajectorySegmentDispositionV1::TerminalDefeat
        }
        status if status.is_resumable() => RunTrajectorySegmentDispositionV1::Resumable,
        _ => RunTrajectorySegmentDispositionV1::Stopped,
    }
}
