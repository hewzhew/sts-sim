use serde::{Deserialize, Serialize};

use super::run_contract::RunContract;
use super::Args;
pub(super) use sts_simulator::runtime::branch::{current_source_identity, SourceIdentity};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct RunIdentity {
    pub(super) schema: String,
    pub(super) run_contract: RunContract,
}

impl RunIdentity {
    pub(super) fn from_args(args: Args) -> Self {
        Self {
            schema: "branch_tiny_run_identity_v1".to_string(),
            run_contract: RunContract::from_args(args),
        }
    }
}
