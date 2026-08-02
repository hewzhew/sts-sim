use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::eval::combat_case::{
    CombatCase, CombatCaseGap, CombatCaseRngSummary, CombatCaseRunSummary, CombatCaseSource,
};
use crate::eval::combat_case_context::capture_oracle_analysis_combat_case_production_context_v1;
use crate::eval::combat_guidance_bundle::CombatGuidanceBundleV1;
use crate::eval::combat_lab_v1::atomic_write_json;
use crate::eval::run_control::{
    exact_audit_run_progress_journal_policy_v1, expand_oracle_neow_candidates_v1,
    ordered_oracle_neow_root_candidate_ids_v1, seed_oracle_run_explorer_from_checkpoint_v1,
    seed_oracle_run_explorer_from_session_v1, seed_oracle_run_explorer_v1, NeowOracleExpansionV1,
    OracleAnalysisAdvanceReportV1, OracleAnalysisAdvanceRequestV1, OracleAnalysisNodeViewV1,
    OracleAnalysisSessionCheckpointV1, OracleAnalysisSessionV1, RunControlConfig,
    RunControlSession, RunDecisionAction,
};
use crate::state::core::ClientInput;

use super::oracle_run::{
    oracle_run_combat_budgets_v1, OracleRunBudget, OracleRunConfig, OracleRunContinuationV1,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat_guidance_bundle: Option<CombatGuidanceBundleV1>,
    pub session: OracleAnalysisSessionCheckpointV1,
}

pub struct OracleAnalysisWorkspaceV1 {
    pub seed: u64,
    pub ascension: u8,
    pub budget: OracleRunBudget,
    pub combat_guidance_bundle: Option<CombatGuidanceBundleV1>,
    pub session: OracleAnalysisSessionV1,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OracleAnalysisWorkspaceSaveTimingV1 {
    pub checkpoint_elapsed_ms: u64,
    pub write_elapsed_ms: u64,
}

impl OracleAnalysisWorkspaceV1 {
    pub fn new(config: OracleRunConfig) -> Result<Self, String> {
        Self::new_with_combat_guidance(config, None)
    }

    pub fn new_with_combat_guidance(
        config: OracleRunConfig,
        combat_guidance_bundle: Option<CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        validate_analysis_config(&config)?;
        validate_combat_guidance(&combat_guidance_bundle)?;
        let session = RunControlSession::new(RunControlConfig {
            seed: config.seed,
            ascension_level: config.ascension,
            final_act: false,
            reward_automation: super::oracle_run::oracle_reward_automation_config(),
            ..RunControlConfig::default()
        });
        let preferred_neow_roots = ordered_oracle_neow_root_candidate_ids_v1(
            &session,
            super::owner_audit::legacy_oracle_policy_prior_v1,
        )?;
        let expansion = expand_oracle_neow_candidates_v1(&session)
            .map_err(|error| format!("failed to materialize oracle Neow roots: {error}"))?;
        let preferred_neow_replay = preferred_neow_replay_v1(
            config.seed,
            config.ascension,
            &preferred_neow_roots,
            &expansion,
        )?;
        let explorer = seed_oracle_run_explorer_v1(
            expansion,
            Some(super::owner_audit::legacy_oracle_policy_prior_v1),
        )?;
        let first_root = preferred_neow_replay
            .as_ref()
            .and_then(|preferred| {
                explorer
                    .branches
                    .iter()
                    .find(|branch| neow_replay_matches(&branch.replay, preferred))
                    .map(|branch| branch.branch_id)
            })
            .or_else(|| explorer.branches.first().map(|branch| branch.branch_id));
        let combat_budgets = oracle_run_combat_budgets_v1(&config)
            .with_guidance_bundle(combat_guidance_bundle.clone());
        let analysis = OracleAnalysisSessionV1::from_explorer(
            explorer,
            first_root,
            combat_budgets,
            Some(super::owner_audit::legacy_oracle_policy_prior_v1),
            None,
        )?;
        Ok(Self {
            seed: config.seed,
            ascension: config.ascension,
            budget: config.budget,
            combat_guidance_bundle,
            session: analysis,
        })
    }

    pub fn from_continuation(
        config: OracleRunConfig,
        continuation: OracleRunContinuationV1,
    ) -> Result<Self, String> {
        Self::from_continuation_with_combat_guidance(config, continuation, None)
    }

    pub fn from_continuation_with_combat_guidance(
        config: OracleRunConfig,
        continuation: OracleRunContinuationV1,
        combat_guidance_bundle: Option<CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        validate_analysis_config(&config)?;
        validate_combat_guidance(&combat_guidance_bundle)?;
        if continuation.seed != config.seed || continuation.ascension != config.ascension {
            return Err(format!(
                "oracle continuation is seed {} A{}, requested analysis is seed {} A{}",
                continuation.seed, continuation.ascension, config.seed, config.ascension
            ));
        }
        let combat_budgets = oracle_run_combat_budgets_v1(&config)
            .with_guidance_bundle(combat_guidance_bundle.clone());
        // Import the exact selected state and its committed journal. Historical
        // automatic frontier work is intentionally not treated as an editable
        // analysis tree; the workbench creates explicit variations from here.
        let explorer = seed_oracle_run_explorer_from_session_v1(
            continuation.session.into_session()?,
            continuation.journal,
            &combat_budgets,
            Some(super::owner_audit::legacy_oracle_policy_prior_v1),
        )?;
        let cursor = explorer.branches.first().map(|branch| branch.branch_id);
        let analysis = OracleAnalysisSessionV1::from_explorer(
            explorer,
            cursor,
            combat_budgets,
            Some(super::owner_audit::legacy_oracle_policy_prior_v1),
            None,
        )?;
        Ok(Self {
            seed: config.seed,
            ascension: config.ascension,
            budget: config.budget,
            combat_guidance_bundle,
            session: analysis,
        })
    }

    pub fn from_continuation_branch(
        config: OracleRunConfig,
        continuation: OracleRunContinuationV1,
        branch_id: usize,
    ) -> Result<Self, String> {
        Self::from_continuation_branch_with_combat_guidance(config, continuation, branch_id, None)
    }

    pub fn from_continuation_branch_with_combat_guidance(
        config: OracleRunConfig,
        continuation: OracleRunContinuationV1,
        branch_id: usize,
        combat_guidance_bundle: Option<CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        validate_analysis_config(&config)?;
        validate_combat_guidance(&combat_guidance_bundle)?;
        if continuation.seed != config.seed || continuation.ascension != config.ascension {
            return Err(format!(
                "oracle continuation is seed {} A{}, requested analysis is seed {} A{}",
                continuation.seed, continuation.ascension, config.seed, config.ascension
            ));
        }
        let combat_budgets = oracle_run_combat_budgets_v1(&config)
            .with_guidance_bundle(combat_guidance_bundle.clone());
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
            Some(super::owner_audit::legacy_oracle_policy_prior_v1),
        )?;
        let cursor = explorer.branches.first().map(|branch| branch.branch_id);
        let analysis = OracleAnalysisSessionV1::from_explorer(
            explorer,
            cursor,
            combat_budgets,
            Some(super::owner_audit::legacy_oracle_policy_prior_v1),
            None,
        )?;
        Ok(Self {
            seed: config.seed,
            ascension: config.ascension,
            budget: config.budget,
            combat_guidance_bundle,
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
        validate_combat_guidance(&artifact.combat_guidance_bundle)?;
        let combat_budgets = oracle_run_combat_budgets_v1(&config)
            .with_guidance_bundle(artifact.combat_guidance_bundle.clone());
        let session = OracleAnalysisSessionV1::restore(
            artifact.session,
            combat_budgets,
            Some(super::owner_audit::legacy_oracle_policy_prior_v1),
            None,
        )?;
        Ok(Self {
            seed: artifact.seed,
            ascension: artifact.ascension,
            budget: artifact.budget,
            combat_guidance_bundle: artifact.combat_guidance_bundle,
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
            combat_guidance_bundle: self.combat_guidance_bundle.clone(),
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

    pub fn accept_smoke_bomb_escape(&mut self) -> Result<OracleAnalysisNodeViewV1, String> {
        self.session.accept_cursor_smoke_bomb_escape()?;
        self.view()
    }
}

fn preferred_neow_replay_v1(
    seed: u64,
    ascension: u8,
    preferred_roots: &[String],
    expansion: &NeowOracleExpansionV1,
) -> Result<Option<Vec<(String, RunDecisionAction)>>, String> {
    let mut ranked = Vec::with_capacity(expansion.completed.len());
    for (stable_index, candidate) in expansion.completed.iter().enumerate() {
        let root_rank = preferred_roots
            .iter()
            .position(|candidate_id| candidate_id == &candidate.root_candidate_id)
            .unwrap_or(usize::MAX);
        let audit = exact_audit_run_progress_journal_policy_v1(
            seed,
            ascension,
            &candidate.journal,
            &candidate.session,
            super::owner_audit::current_oracle_candidate_order_v1,
        )
        .map_err(|error| {
            format!(
                "failed to audit completed Neow candidate '{}': {error}",
                candidate.root_label
            )
        })?;
        ranked.push((
            (
                root_rank,
                audit.choices_absent_from_owner_preferences,
                audit.discrepancy_sum,
                audit.max_owner_rank.unwrap_or(0),
                stable_index,
            ),
            candidate
                .replay
                .iter()
                .map(|step| (step.candidate_id.clone(), step.action.clone()))
                .collect::<Vec<_>>(),
        ));
    }
    ranked.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(ranked.into_iter().next().map(|(_, replay)| replay))
}

fn neow_replay_matches(
    actual: &[crate::eval::run_control::OracleRunReplayStepV1],
    expected: &[(String, RunDecisionAction)],
) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(actual, (candidate_id, action))| {
                actual.candidate_id == *candidate_id && actual.action == *action
            })
}

fn validate_combat_guidance(guidance: &Option<CombatGuidanceBundleV1>) -> Result<(), String> {
    guidance
        .as_ref()
        .map(CombatGuidanceBundleV1::validate)
        .transpose()
        .map(|_| ())
}

pub fn save_oracle_analysis_workspace_v1(
    path: &Path,
    workspace: &OracleAnalysisWorkspaceV1,
) -> Result<(), String> {
    save_oracle_analysis_workspace_with_timing_v1(path, workspace).map(|_| ())
}

pub fn save_oracle_analysis_workspace_with_timing_v1(
    path: &Path,
    workspace: &OracleAnalysisWorkspaceV1,
) -> Result<OracleAnalysisWorkspaceSaveTimingV1, String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create oracle analysis directory '{}': {error}",
                parent.display()
            )
        })?;
    }
    let checkpoint_started = std::time::Instant::now();
    let artifact = workspace.artifact()?;
    let checkpoint_elapsed_ms = elapsed_millis(checkpoint_started);
    let write_started = std::time::Instant::now();
    atomic_write_json(path, &artifact)?;
    Ok(OracleAnalysisWorkspaceSaveTimingV1 {
        checkpoint_elapsed_ms,
        write_elapsed_ms: elapsed_millis(write_started),
    })
}

fn elapsed_millis(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
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
    let session = saved.session.into_session()?;
    let position = session.current_active_combat_position()?;
    let (search_nodes, search_ms) = if position.combat.meta.is_boss_fight {
        (artifact.budget.boss_nodes, artifact.budget.boss_ms)
    } else if position.combat.meta.is_elite_fight {
        (artifact.budget.elite_nodes, artifact.budget.elite_ms)
    } else {
        (artifact.budget.hallway_nodes, artifact.budget.hallway_ms)
    };
    let mut case = CombatCase::new(
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
        Vec::new(),
        CombatCaseRngSummary::from_pool(&session.run_state.rng_pool),
        position,
    );
    let owner_budgets = oracle_run_combat_budgets_v1(&OracleRunConfig {
        seed: artifact.seed,
        ascension: artifact.ascension,
        budget: artifact.budget,
    })
    .with_guidance_bundle(artifact.combat_guidance_bundle.clone());
    case.production_context = Some(capture_oracle_analysis_combat_case_production_context_v1(
        &case,
        &session,
        &owner_budgets,
    )?);
    Ok(case)
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

#[cfg(test)]
mod tests {
    use super::{OracleAnalysisWorkspaceV1, OracleRunBudget, OracleRunConfig};
    use crate::content::relics::RelicId;

    #[test]
    fn seed007_starts_from_the_owner_preferred_exact_neow_root() {
        let workspace = OracleAnalysisWorkspaceV1::new(OracleRunConfig {
            seed: 20260713007,
            ascension: 0,
            budget: OracleRunBudget::default(),
        })
        .expect("seed007 oracle workspace");
        let view = workspace.view().expect("seed007 initial view");

        assert!(view.neow_root_label.contains("random rare relic"));
        assert!(view
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::Pocketwatch));
    }

    #[test]
    fn seed014_starts_from_the_owner_preferred_nested_neow_reward() {
        let workspace = OracleAnalysisWorkspaceV1::new(OracleRunConfig {
            seed: 20260713014,
            ascension: 0,
            budget: OracleRunBudget::default(),
        })
        .expect("seed014 oracle workspace");
        let view = workspace.view().expect("seed014 initial view");

        assert!(view.neow_root_label.contains("colorless card"));
        assert!(view
            .deck
            .iter()
            .any(|card| card.id == crate::content::cards::CardId::Blind));
        assert!(!view
            .deck
            .iter()
            .any(|card| card.id == crate::content::cards::CardId::JackOfAllTrades));
    }
}
