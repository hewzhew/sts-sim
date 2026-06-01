use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchTerminalLabel {
    Win,
    Loss,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCoverageStatus {
    Exhaustive,
    NodeBudgetLimited,
    TimeBudgetLimited,
    FrontierOpen,
}
