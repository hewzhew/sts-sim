use std::process::Command;

use serde::{Deserialize, Serialize};

use super::run_contract::RunContract;
use super::Args;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct RunIdentity {
    pub(super) schema: String,
    pub(super) run_contract: RunContract,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct SourceIdentity {
    pub(super) git_commit: Option<String>,
    pub(super) git_dirty: Option<bool>,
}

impl RunIdentity {
    pub(super) fn from_args(args: Args) -> Self {
        Self {
            schema: "branch_tiny_run_identity_v1".to_string(),
            run_contract: RunContract::from_args(args),
        }
    }
}

pub(super) fn current_source_identity() -> SourceIdentity {
    SourceIdentity {
        git_commit: current_git_commit(),
        git_dirty: current_git_dirty(),
    }
}

fn current_git_commit() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|commit| !commit.is_empty())
}

fn current_git_dirty() -> Option<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| !String::from_utf8_lossy(&output.stdout).trim().is_empty())
}
