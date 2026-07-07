use serde::{Deserialize, Serialize};

use crate::state::events::EventId;

#[derive(Clone, Deserialize, Serialize)]
pub enum BranchStatus {
    Running {
        boundary: String,
        owner: Owner,
    },
    AwaitingAuto {
        boundary: String,
        reason: String,
    },
    Terminal(TerminalOutcome),
    AutomationGap {
        boundary: String,
        site: BoundarySite,
    },
    CombatGap {
        boundary: String,
        reason: String,
    },
    OperationBudgetExhausted {
        boundary: String,
        reason: String,
    },
    BudgetGap {
        boundary: String,
        reason: String,
    },
    ApplyFailed(String),
    AdvanceFailed(String),
}

impl BranchStatus {
    pub fn is_resumable(&self) -> bool {
        matches!(
            self,
            BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. }
        )
    }

    pub fn is_expandable_now(&self) -> bool {
        matches!(self, BranchStatus::Running { .. })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TerminalOutcome {
    Victory,
    Defeat,
}

impl TerminalOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Victory => "victory",
            Self::Defeat => "defeat",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Owner {
    NeowStart,
    CardReward,
    BossRelic,
    Event(EventId),
    RewardTiny,
    ShopTiny,
    Campfire,
    RunChoice,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BoundarySite {
    Event(EventId),
    Reward,
    Shop,
    Route,
    Campfire,
    BossRelic,
    RunChoice,
    Treasure,
    Terminal,
    Unknown,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn branch_status_keeps_runtime_lifecycle_predicates() {
        let running = BranchStatus::Running {
            boundary: "Reward".to_string(),
            owner: Owner::CardReward,
        };
        let awaiting = BranchStatus::AwaitingAuto {
            boundary: "Combat".to_string(),
            reason: "auto".to_string(),
        };
        let terminal = BranchStatus::Terminal(TerminalOutcome::Victory);

        assert!(running.is_resumable());
        assert!(running.is_expandable_now());
        assert!(awaiting.is_resumable());
        assert!(!awaiting.is_expandable_now());
        assert!(!terminal.is_resumable());
        assert!(!terminal.is_expandable_now());
    }

    #[test]
    fn branch_status_serializes_as_structured_runtime_data() {
        let status = BranchStatus::AutomationGap {
            boundary: "A1F3 Event".to_string(),
            site: BoundarySite::Event(sts_simulator::state::events::EventId::DeadAdventurer),
        };

        let value = serde_json::to_value(status).unwrap();

        assert_eq!(value["AutomationGap"]["boundary"], json!("A1F3 Event"));
        assert_eq!(TerminalOutcome::Defeat.as_str(), "defeat");
    }
}
