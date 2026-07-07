use super::{BranchStatus, TerminalOutcome};
pub(super) use sts_simulator::runtime::branch::{RunContract, RunObjective};

#[derive(Clone, Copy)]
pub(super) struct CompletionReason(&'static str);

impl CompletionReason {
    pub(super) fn as_str(self) -> &'static str {
        self.0
    }
}

pub(super) fn satisfied(
    objective: RunObjective,
    status: &BranchStatus,
) -> Option<CompletionReason> {
    match (objective, status) {
        (RunObjective::FirstVictory, BranchStatus::Terminal(TerminalOutcome::Victory)) => {
            Some(CompletionReason("victory_found"))
        }
        (RunObjective::FirstTerminal, BranchStatus::Terminal(_)) => {
            Some(CompletionReason("terminal_found"))
        }
        _ => None,
    }
}
