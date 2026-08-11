use serde::{Deserialize, Serialize};

use super::oracle_analysis_session::OracleAnalysisSessionCheckpointV1;
use crate::eval::combat_guidance_bundle::CombatGuidanceBundleV1;

use super::oracle_run::OracleRunBudget;

pub const ORACLE_ANALYSIS_WORKSPACE_SCHEMA_NAME: &str = "OracleAnalysisWorkspace";
pub const ORACLE_ANALYSIS_WORKSPACE_SCHEMA_VERSION: u32 = 1;

/// Durable workspace envelope. Live analysis/search objects never enter this
/// contract; their run-control owners first project typed checkpoints.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisWorkspaceArtifactV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    pub ascension: u8,
    pub budget: OracleRunBudget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat_guidance_bundle: Option<CombatGuidanceBundleV1>,
    pub session: OracleAnalysisSessionCheckpointV1,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OracleAnalysisWorkspaceSaveTimingV1 {
    pub checkpoint_elapsed_ms: u64,
    pub write_elapsed_ms: u64,
}
