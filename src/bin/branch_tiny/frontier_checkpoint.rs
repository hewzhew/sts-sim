use std::collections::VecDeque;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sts_simulator::eval::run_control::RunControlSessionCheckpointV1;

use super::owners::ChoiceAnnotation;
use super::{Args, Branch, BranchPathStep, BranchStatus, DecisionKey};

#[derive(Deserialize, Serialize)]
pub(super) struct FrontierCheckpoint {
    schema: String,
    pub(super) args: Args,
    pub(super) generation: usize,
    next_branch_id: usize,
    frontier: Vec<BranchCheckpoint>,
}

#[derive(Deserialize, Serialize)]
struct BranchCheckpoint {
    id: usize,
    parent_id: Option<usize>,
    path: Vec<PathStepCheckpoint>,
    session: RunControlSessionCheckpointV1,
    status: BranchStatusCheckpoint,
}

#[derive(Deserialize, Serialize)]
struct PathStepCheckpoint {
    key: Option<DecisionKey>,
    action_debug: String,
    label: String,
}

#[derive(Deserialize, Serialize)]
enum BranchStatusCheckpoint {
    Running {
        boundary: String,
        owner: super::Owner,
    },
    Terminal(String),
    AutomationGap {
        boundary: String,
        site: super::BoundarySite,
    },
    CombatGap {
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

pub(super) fn save(
    path: &Path,
    args: Args,
    generation: usize,
    next_branch_id: usize,
    frontier: &VecDeque<Branch>,
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let checkpoint = FrontierCheckpoint {
        schema: "branch_tiny_frontier_checkpoint".to_string(),
        args,
        generation,
        next_branch_id,
        frontier: frontier.iter().map(BranchCheckpoint::from_branch).collect(),
    };
    let payload = serde_json::to_string_pretty(&checkpoint).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

pub(super) fn load(path: &Path) -> Result<FrontierCheckpoint, String> {
    let payload = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&payload)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

impl FrontierCheckpoint {
    pub(super) fn into_frontier(self) -> Result<(VecDeque<Branch>, usize), String> {
        let mut frontier = VecDeque::new();
        for branch in self.frontier {
            frontier.push_back(branch.into_branch()?);
        }
        Ok((frontier, self.next_branch_id))
    }
}

impl BranchCheckpoint {
    fn from_branch(branch: &Branch) -> Self {
        Self {
            id: branch.id,
            parent_id: branch.parent_id,
            path: branch
                .path
                .iter()
                .map(PathStepCheckpoint::from_step)
                .collect(),
            session: RunControlSessionCheckpointV1::from_session(&branch.session),
            status: BranchStatusCheckpoint::from_status(&branch.status),
        }
    }

    fn into_branch(self) -> Result<Branch, String> {
        Ok(Branch {
            id: self.id,
            parent_id: self.parent_id,
            path: self
                .path
                .into_iter()
                .map(PathStepCheckpoint::into_step)
                .collect(),
            session: self.session.into_session()?,
            status: self.status.into_status(),
            boss_retry: None,
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
        })
    }
}

impl PathStepCheckpoint {
    fn from_step(step: &BranchPathStep) -> Self {
        Self {
            key: step.key.clone(),
            action_debug: step.action_debug.clone(),
            label: step.label.clone(),
        }
    }

    fn into_step(self) -> BranchPathStep {
        BranchPathStep {
            key: self.key,
            action_debug: self.action_debug,
            label: self.label,
            annotation: ChoiceAnnotation::None,
        }
    }
}

impl BranchStatusCheckpoint {
    fn from_status(status: &BranchStatus) -> Self {
        match status {
            BranchStatus::Running { boundary, owner } => Self::Running {
                boundary: boundary.clone(),
                owner: *owner,
            },
            BranchStatus::Terminal(result) => Self::Terminal((*result).to_string()),
            BranchStatus::AutomationGap { boundary, site } => Self::AutomationGap {
                boundary: boundary.clone(),
                site: *site,
            },
            BranchStatus::CombatGap { boundary, reason } => Self::CombatGap {
                boundary: boundary.clone(),
                reason: reason.clone(),
            },
            BranchStatus::BudgetGap { boundary, reason } => Self::BudgetGap {
                boundary: boundary.clone(),
                reason: reason.clone(),
            },
            BranchStatus::ApplyFailed(reason) => Self::ApplyFailed(reason.clone()),
            BranchStatus::AdvanceFailed(reason) => Self::AdvanceFailed(reason.clone()),
        }
    }

    fn into_status(self) -> BranchStatus {
        match self {
            Self::Running { boundary, owner } => BranchStatus::Running { boundary, owner },
            Self::Terminal(result) => BranchStatus::Terminal(terminal_result(&result)),
            Self::AutomationGap { boundary, site } => {
                BranchStatus::AutomationGap { boundary, site }
            }
            Self::CombatGap { boundary, reason } => BranchStatus::CombatGap { boundary, reason },
            Self::BudgetGap { boundary, reason } => BranchStatus::BudgetGap { boundary, reason },
            Self::ApplyFailed(reason) => BranchStatus::ApplyFailed(reason),
            Self::AdvanceFailed(reason) => BranchStatus::AdvanceFailed(reason),
        }
    }
}

fn terminal_result(result: &str) -> &'static str {
    match result {
        "win" => "win",
        "loss" => "loss",
        "abandoned" => "abandoned",
        _ => "terminal",
    }
}
