use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::eval::combat_lab_v1::atomic_write_json;
use crate::eval::run_control::{
    expand_oracle_neow_candidates_v1, seed_oracle_run_explorer_from_session_v1,
    seed_oracle_run_explorer_v1, OracleAnalysisAdvanceReportV1, OracleAnalysisAdvanceRequestV1,
    OracleAnalysisNodeViewV1, OracleAnalysisSessionCheckpointV1, OracleAnalysisSessionV1,
    RunControlConfig, RunControlSession,
};

use super::oracle_run::{
    oracle_combat_budgets, OracleRunBudget, OracleRunConfig, OracleRunContinuationV1,
};

pub const ORACLE_ANALYSIS_WORKSPACE_SCHEMA_NAME: &str = "OracleAnalysisWorkspace";
pub const ORACLE_ANALYSIS_WORKSPACE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisWorkspaceArtifactV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    pub ascension: u8,
    pub budget: OracleRunBudget,
    pub session: OracleAnalysisSessionCheckpointV1,
}

pub struct OracleAnalysisWorkspaceV1 {
    pub seed: u64,
    pub ascension: u8,
    pub budget: OracleRunBudget,
    pub session: OracleAnalysisSessionV1,
}

impl OracleAnalysisWorkspaceV1 {
    pub fn new(config: OracleRunConfig) -> Result<Self, String> {
        validate_analysis_config(&config)?;
        let session = RunControlSession::new(RunControlConfig {
            seed: config.seed,
            ascension_level: config.ascension,
            final_act: false,
            reward_automation: super::oracle_run::oracle_reward_automation_config(),
            ..RunControlConfig::default()
        });
        let expansion = expand_oracle_neow_candidates_v1(&session)
            .map_err(|error| format!("failed to materialize oracle Neow roots: {error}"))?;
        let explorer = seed_oracle_run_explorer_v1(
            expansion,
            Some(super::owner_audit::oracle_candidate_order),
        )?;
        let first_root = explorer.branches.first().map(|branch| branch.branch_id);
        let analysis = OracleAnalysisSessionV1::from_explorer(
            explorer,
            first_root,
            oracle_combat_budgets(&config),
            Some(super::owner_audit::oracle_candidate_order),
            Some(super::owner_audit::oracle_candidate_annotation),
        )?;
        Ok(Self {
            seed: config.seed,
            ascension: config.ascension,
            budget: config.budget,
            session: analysis,
        })
    }

    pub fn from_continuation(
        config: OracleRunConfig,
        continuation: OracleRunContinuationV1,
    ) -> Result<Self, String> {
        validate_analysis_config(&config)?;
        if continuation.seed != config.seed || continuation.ascension != config.ascension {
            return Err(format!(
                "oracle continuation is seed {} A{}, requested analysis is seed {} A{}",
                continuation.seed, continuation.ascension, config.seed, config.ascension
            ));
        }
        let combat_budgets = oracle_combat_budgets(&config);
        // Import the exact selected state and its committed journal. Historical
        // automatic frontier work is intentionally not treated as an editable
        // analysis tree; the workbench creates explicit variations from here.
        let explorer = seed_oracle_run_explorer_from_session_v1(
            continuation.session.into_session()?,
            continuation.journal,
            &combat_budgets,
            Some(super::owner_audit::oracle_candidate_order),
        )?;
        let cursor = explorer.branches.first().map(|branch| branch.branch_id);
        let analysis = OracleAnalysisSessionV1::from_explorer(
            explorer,
            cursor,
            combat_budgets,
            Some(super::owner_audit::oracle_candidate_order),
            Some(super::owner_audit::oracle_candidate_annotation),
        )?;
        Ok(Self {
            seed: config.seed,
            ascension: config.ascension,
            budget: config.budget,
            session: analysis,
        })
    }

    pub fn restore(artifact: OracleAnalysisWorkspaceArtifactV1) -> Result<Self, String> {
        if artifact.schema_name != ORACLE_ANALYSIS_WORKSPACE_SCHEMA_NAME
            || artifact.schema_version != ORACLE_ANALYSIS_WORKSPACE_SCHEMA_VERSION
        {
            return Err("unsupported oracle analysis workspace schema".to_string());
        }
        let config = OracleRunConfig {
            seed: artifact.seed,
            ascension: artifact.ascension,
            budget: artifact.budget,
        };
        validate_analysis_config(&config)?;
        let session = OracleAnalysisSessionV1::restore(
            artifact.session,
            oracle_combat_budgets(&config),
            Some(super::owner_audit::oracle_candidate_order),
            Some(super::owner_audit::oracle_candidate_annotation),
        )?;
        Ok(Self {
            seed: artifact.seed,
            ascension: artifact.ascension,
            budget: artifact.budget,
            session,
        })
    }

    pub fn artifact(&self) -> Result<OracleAnalysisWorkspaceArtifactV1, String> {
        Ok(OracleAnalysisWorkspaceArtifactV1 {
            schema_name: ORACLE_ANALYSIS_WORKSPACE_SCHEMA_NAME.to_string(),
            schema_version: ORACLE_ANALYSIS_WORKSPACE_SCHEMA_VERSION,
            seed: self.seed,
            ascension: self.ascension,
            budget: self.budget,
            session: self.session.checkpoint()?,
        })
    }

    pub fn view(&self) -> Result<OracleAnalysisNodeViewV1, String> {
        self.session.view_cursor()
    }

    pub fn try_choice(&mut self, choice_ref: &str) -> Result<OracleAnalysisNodeViewV1, String> {
        self.session.try_choice(choice_ref)?;
        self.view()
    }

    pub fn advance(
        &mut self,
        request: OracleAnalysisAdvanceRequestV1,
    ) -> Result<(OracleAnalysisAdvanceReportV1, OracleAnalysisNodeViewV1), String> {
        let report = self.session.advance_cursor(request)?;
        let view = self.view()?;
        Ok((report, view))
    }
}

pub fn save_oracle_analysis_workspace_v1(
    path: &Path,
    workspace: &OracleAnalysisWorkspaceV1,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create oracle analysis directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    atomic_write_json(path, &workspace.artifact()?)
}

pub fn load_oracle_analysis_workspace_v1(path: &Path) -> Result<OracleAnalysisWorkspaceV1, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let artifact = serde_json::from_slice::<OracleAnalysisWorkspaceArtifactV1>(&bytes)
        .map_err(|error| format!("failed to parse '{}': {error}", path.display()))?;
    OracleAnalysisWorkspaceV1::restore(artifact)
}

fn validate_analysis_config(config: &OracleRunConfig) -> Result<(), String> {
    if config.ascension > 20 {
        return Err(format!(
            "oracle analysis ascension must be in 0..=20, got {}",
            config.ascension
        ));
    }
    if config.budget.combat_quantum_nodes == 0 || config.budget.combat_quantum_ms == 0 {
        return Err("oracle analysis combat quantum must be positive".to_string());
    }
    Ok(())
}
