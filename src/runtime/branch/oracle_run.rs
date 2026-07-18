use std::time::Instant;

use serde::Serialize;

use crate::ai::combat_search_v2::CombatSearchV2Satisfaction;
use crate::content::potions::Potion;
use crate::content::relics::RelicState;
use crate::eval::run_control::{
    drive_oracle_run_explorer_v1, expand_oracle_neow_candidates_v1, seed_oracle_run_explorer_v1,
    CombatAutomationActionV1, DecisionCandidateKey, NeowOracleExpansionV1,
    OraclePendingCombatSummaryV1, OracleRunCombatBudgetsV1, OracleRunExploreBudgetV1,
    OracleRunExploreResultV1, OracleRunExploreStopV1, RewardAutomationConfig, RunControlConfig,
    RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession, RunDecisionAction,
    RunProgressJournalV1, RunProgressStepV1,
};
use crate::runtime::combat::CombatCard;
use crate::state::core::EngineState;

pub const ORACLE_RUN_REPORT_SCHEMA_NAME: &str = "OracleRunReport";
pub const ORACLE_RUN_REPORT_SCHEMA_VERSION: u32 = 6;

#[derive(Clone, Copy, Debug)]
pub struct OracleRunBudget {
    pub max_work_items: usize,
    pub wall_ms: Option<u64>,
    pub hallway_nodes: usize,
    pub hallway_ms: u64,
    pub elite_nodes: usize,
    pub elite_ms: u64,
    pub boss_nodes: usize,
    pub boss_ms: u64,
    pub combat_quantum_nodes: usize,
    pub combat_quantum_ms: u64,
}

impl Default for OracleRunBudget {
    fn default() -> Self {
        Self {
            max_work_items: 2_048,
            wall_ms: None,
            hallway_nodes: 250_000,
            hallway_ms: 5_000,
            elite_nodes: 750_000,
            elite_ms: 15_000,
            boss_nodes: 2_000_000,
            boss_ms: 30_000,
            combat_quantum_nodes: 50_000,
            combat_quantum_ms: 1_000,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OracleRunConfig {
    pub seed: u64,
    pub ascension: u8,
    pub budget: OracleRunBudget,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleDecisionSummaryV1 {
    pub decision_step: u64,
    pub title: String,
    pub location: String,
    pub source: String,
    pub selected_label: String,
    pub candidate_labels: Vec<String>,
    pub action: RunDecisionAction,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleCombatResolutionSummaryV1 {
    pub combat_sequence: u64,
    pub title: String,
    pub location: String,
    pub kind: String,
    pub source: String,
    pub action_count: usize,
    pub final_hp: Option<i32>,
    pub actions: Vec<CombatAutomationActionV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleNeowReplayStepSummaryV1 {
    pub candidate_id: String,
    pub label: String,
    pub action: RunDecisionAction,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleCompletedNeowCandidateSummaryV1 {
    pub root_candidate_id: String,
    pub root_candidate_key: DecisionCandidateKey,
    pub root_label: String,
    pub replay: Vec<OracleNeowReplayStepSummaryV1>,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck: Vec<CombatCard>,
    pub relics: Vec<RelicState>,
    pub potions: Vec<Option<Potion>>,
    pub engine_state: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleUnresolvedNeowCandidateSummaryV1 {
    pub root_candidate_id: String,
    pub root_label: String,
    pub replay: Vec<OracleNeowReplayStepSummaryV1>,
    pub boundary: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleNeowFrontierSummaryV1 {
    pub completed: Vec<OracleCompletedNeowCandidateSummaryV1>,
    pub unresolved: Vec<OracleUnresolvedNeowCandidateSummaryV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleUnresolvedCombatSummaryV1 {
    pub branch_id: usize,
    pub rejection: String,
    pub nodes_expanded: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunExplorerSummaryV1 {
    pub stop: OracleRunExploreStopV1,
    pub work_items: usize,
    pub combat_quanta: usize,
    pub decision_service_ms: u64,
    pub combat_service_ms: u64,
    pub elapsed_ms: u64,
    pub materialized_branches: usize,
    pub pending_decisions: usize,
    pub pending_combats: Vec<OraclePendingCombatSummaryV1>,
    pub exact_duplicates: usize,
    pub unresolved_combats: Vec<OracleUnresolvedCombatSummaryV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    pub ascension: u8,
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub keys: [bool; 3],
    pub deck: Vec<CombatCard>,
    pub relics: Vec<RelicState>,
    pub potions: Vec<Option<Potion>>,
    pub engine_state: String,
    pub elapsed_ms: u64,
    pub initial_neow_frontier: OracleNeowFrontierSummaryV1,
    pub explorer: OracleRunExplorerSummaryV1,
    pub committed_progress_steps: usize,
    pub decisions: Vec<OracleDecisionSummaryV1>,
    pub combat_resolutions: Vec<OracleCombatResolutionSummaryV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub victory_witness: Option<RunProgressJournalV1>,
}

impl OracleRunReportV1 {
    pub fn victory(&self) -> bool {
        matches!(self.explorer.stop, OracleRunExploreStopV1::Victory { .. })
    }
}

pub fn run_oracle_run(config: OracleRunConfig) -> Result<OracleRunReportV1, String> {
    validate_config(&config)?;
    let started = Instant::now();
    let session = RunControlSession::new(RunControlConfig {
        seed: config.seed,
        ascension_level: config.ascension,
        final_act: false,
        reward_automation: RewardAutomationConfig {
            claim_gold: true,
            claim_potion_with_empty_slot: true,
            claim_safe_relic_without_sapphire_key: true,
        },
        ..RunControlConfig::default()
    });
    let neow_expansion = expand_oracle_neow_candidates_v1(&session)
        .map_err(|error| format!("failed to materialize initial Neow frontier: {error}"))?;
    let initial_neow_frontier = oracle_neow_frontier_summary(neow_expansion.clone());
    let explorer = seed_oracle_run_explorer_v1(neow_expansion)
        .map_err(|error| format!("failed to seed oracle run explorer: {error}"))?;
    let explored = drive_oracle_run_explorer_v1(
        explorer,
        OracleRunExploreBudgetV1 {
            max_work_items: config.budget.max_work_items,
            wall_ms: config.budget.wall_ms,
            combat: oracle_combat_budgets(&config),
            combat_quantum_nodes: config.budget.combat_quantum_nodes,
            combat_quantum_ms: Some(config.budget.combat_quantum_ms),
            decision_order: Some(super::owner_audit::oracle_candidate_order),
        },
    )
    .map_err(|error| format!("oracle run explorer failed: {error}"))?;

    finish_oracle_run_report(&config, &started, initial_neow_frontier, explored)
}

fn finish_oracle_run_report(
    config: &OracleRunConfig,
    started: &Instant,
    initial_neow_frontier: OracleNeowFrontierSummaryV1,
    explored: OracleRunExploreResultV1,
) -> Result<OracleRunReportV1, String> {
    let selected = explored
        .witness()
        .or_else(|| explored.furthest_branch())
        .ok_or_else(|| "oracle run produced no materialized branch".to_string())?;
    let journal = selected.journal.clone();
    let victory_witness =
        matches!(explored.stop, OracleRunExploreStopV1::Victory { .. }).then_some(journal.clone());
    let pending_combats = explored.explorer.pending_combat_summaries()?;
    let explorer_summary = OracleRunExplorerSummaryV1 {
        stop: explored.stop,
        work_items: explored.work_items,
        combat_quanta: explored.combat_quanta,
        decision_service_ms: explored.decision_service_ms,
        combat_service_ms: explored.combat_service_ms,
        elapsed_ms: explored.elapsed_ms,
        materialized_branches: explored.explorer.branches.len(),
        pending_decisions: explored.explorer.pending_decisions.len(),
        pending_combats,
        exact_duplicates: explored.explorer.retired_exact_duplicates.len(),
        unresolved_combats: explored
            .explorer
            .unresolved_combats
            .iter()
            .map(|combat| OracleUnresolvedCombatSummaryV1 {
                branch_id: combat.branch_id,
                rejection: format!("{:?}", combat.rejection),
                nodes_expanded: combat.nodes_expanded,
            })
            .collect(),
    };

    Ok(OracleRunReportV1 {
        schema_name: ORACLE_RUN_REPORT_SCHEMA_NAME.to_string(),
        schema_version: ORACLE_RUN_REPORT_SCHEMA_VERSION,
        seed: config.seed,
        ascension: config.ascension,
        act: selected.session.run_state.act_num,
        floor: selected.session.run_state.floor_num,
        current_hp: selected.session.run_state.current_hp,
        max_hp: selected.session.run_state.max_hp,
        gold: selected.session.run_state.gold,
        keys: selected.session.run_state.keys,
        deck: selected.session.run_state.master_deck.clone(),
        relics: selected.session.run_state.relics.clone(),
        potions: selected.session.run_state.potions.clone(),
        engine_state: engine_state_name(&selected.session.engine_state).to_string(),
        elapsed_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        initial_neow_frontier,
        explorer: explorer_summary,
        committed_progress_steps: journal.len(),
        decisions: decision_summaries(&journal),
        combat_resolutions: combat_resolution_summaries(&journal),
        victory_witness,
    })
}

fn oracle_neow_frontier_summary(expansion: NeowOracleExpansionV1) -> OracleNeowFrontierSummaryV1 {
    OracleNeowFrontierSummaryV1 {
        completed: expansion
            .completed
            .into_iter()
            .map(|candidate| OracleCompletedNeowCandidateSummaryV1 {
                root_candidate_id: candidate.root_candidate_id,
                root_candidate_key: candidate.root_candidate_key,
                root_label: candidate.root_label,
                replay: candidate
                    .replay
                    .into_iter()
                    .map(|step| OracleNeowReplayStepSummaryV1 {
                        candidate_id: step.candidate_id,
                        label: step.label,
                        action: step.action,
                    })
                    .collect(),
                current_hp: candidate.session.run_state.current_hp,
                max_hp: candidate.session.run_state.max_hp,
                gold: candidate.session.run_state.gold,
                deck: candidate.session.run_state.master_deck,
                relics: candidate.session.run_state.relics,
                potions: candidate.session.run_state.potions,
                engine_state: engine_state_name(&candidate.session.engine_state).to_string(),
            })
            .collect(),
        unresolved: expansion
            .unresolved
            .into_iter()
            .map(|candidate| OracleUnresolvedNeowCandidateSummaryV1 {
                root_candidate_id: candidate.root_candidate_id,
                root_label: candidate.root_label,
                replay: candidate
                    .replay
                    .into_iter()
                    .map(|step| OracleNeowReplayStepSummaryV1 {
                        candidate_id: step.candidate_id,
                        label: step.label,
                        action: step.action,
                    })
                    .collect(),
                boundary: candidate.boundary,
                reason: candidate.reason,
            })
            .collect(),
    }
}

fn combat_resolution_summaries(
    journal: &RunProgressJournalV1,
) -> Vec<OracleCombatResolutionSummaryV1> {
    journal
        .entries()
        .iter()
        .filter_map(RunProgressStepV1::as_combat_resolution)
        .map(|resolution| OracleCombatResolutionSummaryV1 {
            combat_sequence: resolution.before.combat_sequence,
            title: resolution.before.title.clone(),
            location: resolution.before.location.clone(),
            kind: format!("{:?}", resolution.kind),
            source: resolution.trajectory.source.to_string(),
            action_count: resolution.trajectory.action_count,
            final_hp: resolution
                .trajectory
                .actions
                .iter()
                .rev()
                .find_map(|action| action.combat_after.as_ref().map(|state| state.player_hp)),
            actions: resolution.trajectory.actions.clone(),
        })
        .collect()
}

fn decision_summaries(journal: &RunProgressJournalV1) -> Vec<OracleDecisionSummaryV1> {
    journal
        .entries()
        .iter()
        .filter_map(RunProgressStepV1::as_decision)
        .map(|transaction| {
            let selected_label = transaction
                .before
                .candidates
                .iter()
                .find(|candidate| candidate.candidate_id == transaction.selection.candidate_id)
                .map(|candidate| candidate.label.clone())
                .unwrap_or_else(|| "<selected candidate missing>".to_string());
            OracleDecisionSummaryV1 {
                decision_step: transaction.before.decision_step,
                title: transaction.before.title.clone(),
                location: transaction.before.location.clone(),
                source: format!("{:?}", transaction.selection.source),
                selected_label,
                candidate_labels: transaction
                    .before
                    .candidates
                    .iter()
                    .map(|candidate| candidate.label.clone())
                    .collect(),
                action: transaction.action.clone(),
            }
        })
        .collect()
}

fn validate_config(config: &OracleRunConfig) -> Result<(), String> {
    if config.budget.max_work_items == 0 {
        return Err("oracle run requires at least one work item".to_string());
    }
    if config.budget.combat_quantum_nodes == 0 || config.budget.combat_quantum_ms == 0 {
        return Err("oracle run combat quantum must be positive".to_string());
    }
    if config.ascension > 20 {
        return Err(format!(
            "oracle run ascension must be in 0..=20, got {}",
            config.ascension
        ));
    }
    Ok(())
}

fn oracle_combat_budgets(config: &OracleRunConfig) -> OracleRunCombatBudgetsV1 {
    OracleRunCombatBudgetsV1 {
        hallway: RunControlSearchCombatOptions {
            max_nodes: Some(config.budget.hallway_nodes),
            wall_ms: Some(config.budget.hallway_ms),
            satisfaction: Some(CombatSearchV2Satisfaction::ZeroLossOrBudget),
            max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
            enable_legacy_no_win_rescue: false,
            ..RunControlSearchCombatOptions::default()
        },
        elite: RunControlSearchCombatOptions {
            max_nodes: Some(config.budget.elite_nodes),
            wall_ms: Some(config.budget.elite_ms),
            satisfaction: Some(CombatSearchV2Satisfaction::ZeroLossOrBudget),
            max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
            enable_legacy_no_win_rescue: false,
            ..RunControlSearchCombatOptions::default()
        },
        boss: RunControlSearchCombatOptions {
            max_nodes: Some(config.budget.boss_nodes),
            wall_ms: Some(config.budget.boss_ms),
            satisfaction: Some(CombatSearchV2Satisfaction::BudgetOrExhaustion),
            max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
            enable_legacy_no_win_rescue: false,
            ..RunControlSearchCombatOptions::default()
        },
    }
}

fn engine_state_name(state: &EngineState) -> &'static str {
    match state {
        EngineState::MapNavigation => "map_navigation",
        EngineState::MapOverlay { .. } => "map_overlay",
        EngineState::CombatStart(_) => "combat_start",
        EngineState::CombatProcessing => "combat_processing",
        EngineState::CombatPlayerTurn => "combat_player_turn",
        EngineState::PendingChoice(_) => "pending_choice",
        EngineState::RewardScreen(_) => "reward_screen",
        EngineState::RewardOverlay { .. } => "reward_overlay",
        EngineState::EventRoom => "event_room",
        EngineState::Shop(_) => "shop",
        EngineState::Campfire => "campfire",
        EngineState::RunPendingChoice(_) => "run_pending_choice",
        EngineState::TreasureRoom(_) => "treasure_room",
        EngineState::BossRelicSelect(_) => "boss_relic_select",
        EngineState::GameOver(_) => "game_over",
    }
}
