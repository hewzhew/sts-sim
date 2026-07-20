use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::eval::combat_lab_v1::atomic_write_json;
use crate::eval::combat_case::{
    CombatCase, CombatCaseGap, CombatCasePathStep, CombatCaseRngSummary, CombatCaseRunSummary,
    CombatCaseSource,
};
use crate::eval::run_control::{
    expand_oracle_neow_candidates_v1, seed_oracle_run_explorer_from_checkpoint_v1,
    seed_oracle_run_explorer_from_session_v1, seed_oracle_run_explorer_v1,
    OracleAnalysisAdvanceReportV1, OracleAnalysisAdvanceRequestV1, OracleAnalysisNodeViewV1,
    OracleAnalysisSessionCheckpointV1, OracleAnalysisSessionV1, RunControlConfig,
    RunControlSession,
};
use crate::state::core::ClientInput;

use super::oracle_run::{
    oracle_combat_budgets, OracleRunBudget, OracleRunConfig, OracleRunContinuationV1,
    ORACLE_RUN_CONTINUATION_SCHEMA_NAME, ORACLE_RUN_CONTINUATION_SCHEMA_VERSION,
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

    pub fn from_continuation_branch(
        config: OracleRunConfig,
        continuation: OracleRunContinuationV1,
        branch_id: usize,
    ) -> Result<Self, String> {
        validate_analysis_config(&config)?;
        if continuation.seed != config.seed || continuation.ascension != config.ascension {
            return Err(format!(
                "oracle continuation is seed {} A{}, requested analysis is seed {} A{}",
                continuation.seed, continuation.ascension, config.seed, config.ascension
            ));
        }
        let combat_budgets = oracle_combat_budgets(&config);
        let frontier = continuation.explorer_frontier.ok_or_else(|| {
            "oracle continuation has no retained frontier from which to import a branch".to_string()
        })?;
        let mut restored = seed_oracle_run_explorer_from_checkpoint_v1(frontier, &combat_budgets)?;
        let branch_index = restored
            .branches
            .iter()
            .position(|branch| branch.branch_id == branch_id)
            .ok_or_else(|| format!("oracle continuation does not retain branch {branch_id}"))?;
        let branch = restored.branches.swap_remove(branch_index);
        let explorer = seed_oracle_run_explorer_from_session_v1(
            branch.session,
            branch.journal,
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

    pub fn continuation(&self, node_id: usize) -> Result<OracleRunContinuationV1, String> {
        let (journal, session) = self.session.continuation_parts(node_id)?;
        Ok(OracleRunContinuationV1 {
            schema_name: ORACLE_RUN_CONTINUATION_SCHEMA_NAME.to_string(),
            schema_version: ORACLE_RUN_CONTINUATION_SCHEMA_VERSION,
            seed: self.seed,
            ascension: self.ascension,
            journal,
            session,
            explorer_frontier: None,
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

    pub fn accept_combat_incumbent(&mut self) -> Result<OracleAnalysisNodeViewV1, String> {
        self.session.accept_cursor_combat_incumbent()?;
        self.view()
    }

    pub fn accept_combat_actions(
        &mut self,
        actions: &[ClientInput],
    ) -> Result<OracleAnalysisNodeViewV1, String> {
        self.session.accept_cursor_combat_actions(actions)?;
        self.view()
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

/// Recover one exact combat from an analysis workspace whose unrelated
/// branches may no longer pass current whole-frontier validation.
///
/// The selected branch is still deserialized through the current checkpoint
/// types. This deliberately bypasses only cross-branch fingerprint validation;
/// it does not reinterpret or edit the saved combat state.
pub fn recover_oracle_analysis_combat_case_v1(
    path: &Path,
    branch_id: usize,
) -> Result<CombatCase, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let artifact = serde_json::from_slice::<OracleAnalysisWorkspaceArtifactV1>(&bytes)
        .map_err(|error| format!("failed to parse '{}': {error}", path.display()))?;
    if artifact.schema_name != ORACLE_ANALYSIS_WORKSPACE_SCHEMA_NAME
        || artifact.schema_version != ORACLE_ANALYSIS_WORKSPACE_SCHEMA_VERSION
    {
        return Err(format!(
            "unsupported oracle analysis workspace {} version {}",
            artifact.schema_name, artifact.schema_version
        ));
    }
    let saved = artifact
        .session
        .explorer
        .branches
        .into_iter()
        .find(|branch| branch.branch_id == branch_id)
        .ok_or_else(|| format!("oracle analysis workspace has no branch {branch_id}"))?;
    let source = CombatCaseSource {
        seed: artifact.seed,
        ascension: artifact.ascension,
        generation: saved.path_depth as usize,
        branch_id: saved.branch_id,
        parent_id: saved.parent_branch_id,
    };
    let path = saved
        .replay
        .iter()
        .map(|step| CombatCasePathStep {
            key: serde_json::to_value(&step.action).unwrap_or(serde_json::Value::Null),
            label: step.label.clone(),
            state_before: None,
            decision_evidence: Some(serde_json::json!({
                "candidate_id": step.candidate_id,
                "recovered_from_branch": branch_id,
            })),
        })
        .collect::<Vec<_>>();
    let session = saved.session.into_session()?;
    let position = session.current_active_combat_position()?;
    let (search_nodes, search_ms) = if position.combat.meta.is_boss_fight {
        (artifact.budget.boss_nodes, artifact.budget.boss_ms)
    } else if position.combat.meta.is_elite_fight {
        (artifact.budget.elite_nodes, artifact.budget.elite_ms)
    } else {
        (artifact.budget.hallway_nodes, artifact.budget.hallway_ms)
    };
    Ok(CombatCase::new(
        source,
        CombatCaseGap {
            boundary: format!(
                "Act {} Floor {} recovered oracle analysis combat",
                session.run_state.act_num, session.run_state.floor_num
            ),
            reason: "selected_branch_recovery".to_string(),
            search_nodes,
            search_ms,
            rescue_search_nodes: 0,
            rescue_search_ms: 0,
        },
        CombatCaseRunSummary {
            act: session.run_state.act_num,
            floor: session.run_state.floor_num,
            hp: session.run_state.current_hp,
            max_hp: session.run_state.max_hp,
            gold: session.run_state.gold,
            deck_size: session.run_state.master_deck.len(),
            relic_count: session.run_state.relics.len(),
            potion_slots: session.run_state.potions.len(),
        },
        Vec::new(),
        None,
        path,
        CombatCaseRngSummary::from_pool(&session.run_state.rng_pool),
        position,
    ))
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
