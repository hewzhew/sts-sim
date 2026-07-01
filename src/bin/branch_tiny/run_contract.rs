use serde::{Deserialize, Serialize};

use super::{BranchStatus, TerminalOutcome};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RunObjective {
    FirstVictory,
    FirstTerminal,
    ExhaustFrontier,
}

#[derive(Clone, Copy)]
pub(super) struct CompletionReason(&'static str);

impl CompletionReason {
    pub(super) fn as_str(self) -> &'static str {
        self.0
    }
}

impl RunObjective {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "first-victory" | "first_victory" => Ok(Self::FirstVictory),
            "first-terminal" | "first_terminal" => Ok(Self::FirstTerminal),
            "exhaust-frontier" | "exhaust_frontier" => Ok(Self::ExhaustFrontier),
            _ => Err(format!(
                "invalid value for --objective: {value}; expected first-victory, first-terminal, or exhaust-frontier"
            )),
        }
    }
}

pub(super) fn default_run_objective() -> RunObjective {
    RunObjective::FirstVictory
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
