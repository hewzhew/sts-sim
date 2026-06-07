use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyDeckFormationNeedV1,
    StrategyDeckFormationStageV1, StrategyFormationSummaryV2, StrategyPackageIdV2,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::eval::branch_experiment_boundary::{
    active_or_visible_reward_cards, branch_boundary_available, card_offer_labels,
    current_branch_boundary, BranchBoundaryConfigV1, CardRewardPortfolioContext,
};
use crate::eval::branch_experiment_retention::{
    default_branch_retention_decision_v1, select_branch_retention_portfolio_v1,
    BranchRetentionCandidateInputV1, BranchRetentionConfigV1, BranchRetentionDecisionV1,
    BranchRetentionSlotV1,
};
use crate::eval::branch_experiment_trajectory::{
    summarize_branch_trajectory_v1, BranchTrajectorySignatureV1,
};
use crate::eval::run_control::{
    build_decision_surface, canonical_player_class, load_session_trace_v1,
    parse_run_control_command, replay_session_trace, RunControlAutoStepOptions, RunControlCommand,
    RunControlConfig, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession, SessionTraceReplayOptions,
    SessionTraceReplayStop,
};
use crate::state::core::{EngineState, RunResult};
use crate::state::rewards::{RewardCard, RewardScreenContext};

pub const BRANCH_EXPERIMENT_SCHEMA_NAME: &str = "BranchExperimentV1";
pub const BRANCH_EXPERIMENT_SCHEMA_VERSION: u32 = 9;

#[derive(Clone, Debug, PartialEq)]
pub struct BranchExperimentConfigV1 {
    pub seed: u64,
    pub ascension_level: u8,
    pub player_class: &'static str,
    pub final_act: bool,
    pub max_branches: usize,
    pub max_branches_per_frontier_group: Option<usize>,
    pub max_reward_options_per_branch: Option<usize>,
    pub max_campfire_options_per_branch: Option<usize>,
    pub max_depth: usize,
    pub auto_max_operations: usize,
    pub experiment_wall_ms: Option<u64>,
    pub search_max_nodes: Option<usize>,
    pub search_wall_ms: Option<u64>,
    pub search_max_hp_loss: Option<RunControlHpLossLimit>,
    pub include_skip: bool,
    pub prefix_commands: Vec<String>,
    pub replay_trace_path: Option<PathBuf>,
    pub replay_trace_max_steps: Option<usize>,
}

impl Default for BranchExperimentConfigV1 {
    fn default() -> Self {
        Self {
            seed: 1,
            ascension_level: 0,
            player_class: "Ironclad",
            final_act: false,
            max_branches: 12,
            max_branches_per_frontier_group: None,
            max_reward_options_per_branch: None,
            max_campfire_options_per_branch: Some(3),
            max_depth: 4,
            auto_max_operations: 128,
            experiment_wall_ms: None,
            search_max_nodes: None,
            search_wall_ms: Some(100),
            search_max_hp_loss: None,
            include_skip: false,
            prefix_commands: Vec::new(),
            replay_trace_path: None,
            replay_trace_max_steps: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub policy_quality_claim: bool,
    pub seed: u64,
    pub replay_trace_path: Option<String>,
    pub replay_trace_applied_steps: usize,
    pub replay_trace_stop: Option<String>,
    pub max_branches: usize,
    pub max_depth: usize,
    pub explored_branch_points: usize,
    pub branch_limit_hit: bool,
    pub frontier_group_limit_hit: bool,
    pub wall_limit_hit: bool,
    pub elapsed_wall_ms: u64,
    pub pruned_branch_count: usize,
    pub pruned_first_pick_counts: Vec<BranchExperimentPrunedFirstPickCountV1>,
    pub reward_option_portfolios: Vec<BranchExperimentRewardOptionPortfolioV1>,
    pub frontier_groups: Vec<BranchExperimentFrontierGroupV1>,
    pub branches: Vec<BranchExperimentBranchReportV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentPrunedFirstPickCountV1 {
    pub first_pick: String,
    pub count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRewardOptionPortfolioV1 {
    pub depth: usize,
    pub frontier_key: String,
    pub boundary_title: String,
    pub max_reward_options_per_branch: usize,
    pub original_count: usize,
    pub selected_count: usize,
    pub selected_options: Vec<BranchExperimentRewardOptionPortfolioEntryV1>,
    pub pruned_options: Vec<BranchExperimentRewardOptionPortfolioEntryV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRewardOptionPortfolioEntryV1 {
    pub command: String,
    pub label: String,
    pub semantic_class: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentBranchReportV1 {
    pub branch_id: String,
    pub status: BranchExperimentBranchStatusV1,
    pub rank_key: i32,
    pub retention: BranchRetentionDecisionV1,
    pub choices: Vec<BranchExperimentChoiceV1>,
    pub stop_reason: String,
    pub summary: BranchExperimentRunSummaryV1,
    pub frontier: BranchExperimentFrontierV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchExperimentBranchStatusV1 {
    Active,
    TerminalVictory,
    TerminalDefeat,
    NeedsHumanBoundary,
    Failed,
    Pruned,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentChoiceV1 {
    pub depth: usize,
    pub kind: String,
    pub card: Option<CardId>,
    pub upgrades: Option<u8>,
    pub label: String,
    pub command: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRunSummaryV1 {
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_count: usize,
    pub relic_count: usize,
    pub potion_count: usize,
    pub formation_stage: StrategyDeckFormationStageV1,
    pub formation_needs: Vec<StrategyDeckFormationNeedV1>,
    pub formation_strengths: Vec<StrategyPackageIdV2>,
    pub trajectory: BranchTrajectorySignatureV1,
    pub boundary_title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentFrontierV1 {
    pub key: String,
    pub act: u8,
    pub floor: i32,
    pub boundary_title: String,
    pub card_rng_counter: u32,
    pub card_blizz_randomizer: i32,
    pub next_card_reward_offer: Option<Vec<String>>,
    pub lineage: BranchExperimentLineageV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentLineageV1 {
    pub visibility: String,
    pub public_policy_input: bool,
    pub direct_pick_consumes_card_rng: bool,
    pub same_reward_offer_lineage_key: String,
    pub reward_screen_context: String,
    pub reward_count_modifiers: Vec<String>,
    pub card_pool_modifiers: Vec<String>,
    pub rarity_modifiers: Vec<String>,
    pub preview_modifiers: Vec<String>,
    pub sequence_breakers_present: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentFrontierGroupV1 {
    pub key: String,
    pub branch_count: usize,
    pub representative_branch_id: String,
    pub boundary_title: String,
    pub next_card_reward_offer: Option<Vec<String>>,
    pub lineage_flags: Vec<String>,
}

#[derive(Clone, Debug)]
struct BranchWork {
    id: String,
    session: RunControlSession,
    choices: Vec<BranchExperimentChoiceV1>,
    status: BranchExperimentBranchStatusV1,
    stop_reason: String,
    retention: BranchRetentionDecisionV1,
}

pub fn run_branch_experiment_v1(
    config: &BranchExperimentConfigV1,
) -> Result<BranchExperimentReportV1, String> {
    let replay_trace = config
        .replay_trace_path
        .as_ref()
        .map(|path| load_session_trace_v1(path))
        .transpose()?;
    let player_class = replay_trace
        .as_ref()
        .map(|trace| canonical_player_class(&trace.run_config.player_class))
        .transpose()?
        .unwrap_or(config.player_class);
    let mut session = RunControlSession::new(RunControlConfig {
        seed: replay_trace
            .as_ref()
            .map(|trace| trace.run_config.seed)
            .unwrap_or(config.seed),
        ascension_level: replay_trace
            .as_ref()
            .map(|trace| trace.run_config.ascension_level)
            .unwrap_or(config.ascension_level),
        final_act: replay_trace
            .as_ref()
            .map(|trace| trace.run_config.final_act)
            .unwrap_or(config.final_act),
        player_class,
        search_max_nodes: config.search_max_nodes,
        search_wall_ms: config.search_wall_ms,
        ..RunControlConfig::default()
    });
    let mut replay_applied_steps = 0usize;
    let mut replay_stop = None;
    if let Some(trace) = replay_trace.as_ref() {
        let report = replay_session_trace(
            &mut session,
            trace,
            SessionTraceReplayOptions {
                max_steps: config.replay_trace_max_steps,
            },
        );
        replay_applied_steps = report.applied_steps.len();
        replay_stop = Some(format!("{:?}", report.stop));
        match report.stop {
            SessionTraceReplayStop::TraceEnd | SessionTraceReplayStop::MaxSteps { .. } => {}
            _ => {
                return Err(format!(
                    "replay trace stopped before a usable prefix: {:?}",
                    report.stop
                ));
            }
        }
    }

    for command_line in &config.prefix_commands {
        let command = parse_run_control_command(command_line)?;
        session.apply_command(command)?;
    }

    Ok(run_branch_experiment_from_session_with_replay(
        session,
        config,
        replay_applied_steps,
        replay_stop,
    ))
}

#[cfg(test)]
fn run_branch_experiment_from_session(
    session: RunControlSession,
    config: &BranchExperimentConfigV1,
) -> BranchExperimentReportV1 {
    run_branch_experiment_from_session_with_replay(session, config, 0, None)
}

fn run_branch_experiment_from_session_with_replay(
    session: RunControlSession,
    config: &BranchExperimentConfigV1,
    replay_trace_applied_steps: usize,
    replay_trace_stop: Option<String>,
) -> BranchExperimentReportV1 {
    let started_at = Instant::now();
    let mut branches = vec![BranchWork {
        id: "root".to_string(),
        session,
        choices: Vec::new(),
        status: BranchExperimentBranchStatusV1::Active,
        stop_reason: "initial".to_string(),
        retention: default_branch_retention_decision_v1(),
    }];
    let report_seed = branches[0].session.run_state.seed;
    let mut explored_branch_points = 0usize;
    let mut branch_limit_hit = false;
    let mut frontier_group_limit_hit = false;
    let mut wall_limit_hit = false;
    let mut pruned_branch_count = 0usize;
    let mut pruned_first_pick_counts = BTreeMap::<String, usize>::new();
    let mut reward_option_portfolios = Vec::new();

    for depth in 0..config.max_depth {
        if experiment_wall_limit_hit(started_at, config) {
            wall_limit_hit = true;
            break;
        }
        let mut next = Vec::new();
        let mut expanded_any = false;

        for mut branch in branches {
            if experiment_wall_limit_hit(started_at, config) {
                wall_limit_hit = true;
                next.push(branch);
                continue;
            }
            if branch.status != BranchExperimentBranchStatusV1::Active {
                next.push(branch);
                continue;
            }

            advance_to_experiment_boundary(&mut branch, config);
            if branch.status != BranchExperimentBranchStatusV1::Active {
                next.push(branch);
                continue;
            }

            let boundary_config = BranchBoundaryConfigV1 {
                max_reward_options_per_branch: config.max_reward_options_per_branch,
                max_campfire_options_per_branch: config.max_campfire_options_per_branch,
            };
            let reward_portfolio_context = config.max_reward_options_per_branch.map(|_| {
                let frontier = branch_frontier(&branch.session);
                CardRewardPortfolioContext {
                    depth,
                    frontier_key: frontier.key,
                    boundary_title: frontier.boundary_title,
                }
            });
            if let Some(boundary) =
                current_branch_boundary(&branch.session, boundary_config, reward_portfolio_context)
            {
                if let Some(portfolio) = boundary.reward_option_portfolio {
                    reward_option_portfolios.push(portfolio);
                }
                if boundary.options.is_empty() {
                    branch.status = BranchExperimentBranchStatusV1::NeedsHumanBoundary;
                    branch.stop_reason = boundary.id.empty_portfolio_reason().to_string();
                    next.push(branch);
                    continue;
                }

                explored_branch_points = explored_branch_points.saturating_add(1);
                expanded_any = true;
                for option in boundary.options {
                    next.push(expand_branch_choice(
                        &branch,
                        BranchChoiceDraft {
                            depth,
                            kind: option.kind,
                            label: option.label,
                            command: option.command,
                            card: option.card,
                            upgrades: option.upgrades,
                            success_reason: option.success_reason,
                        },
                        config,
                    ));
                }
                continue;
            }

            branch.status = BranchExperimentBranchStatusV1::NeedsHumanBoundary;
            branch.stop_reason = current_boundary_title(&branch.session);
            next.push(branch);
        }

        let retention = apply_branch_retention(next, config);
        next = retention.branches;
        branch_limit_hit |= retention.branch_limit_hit;
        frontier_group_limit_hit |= retention.frontier_group_limit_hit;
        pruned_branch_count = pruned_branch_count.saturating_add(retention.pruned_count);
        merge_pruned_first_pick_counts(
            &mut pruned_first_pick_counts,
            retention.pruned_first_pick_counts,
        );

        branches = next;
        if !expanded_any {
            break;
        }
    }
    for branch in &mut branches {
        if experiment_wall_limit_hit(started_at, config) {
            wall_limit_hit = true;
            break;
        }
        settle_branch_to_frontier(branch, config);
    }

    let mut branch_reports = branches
        .into_iter()
        .map(|branch| {
            let summary = run_summary(&branch.session, &branch.choices);
            let frontier = branch_frontier(&branch.session);
            BranchExperimentBranchReportV1 {
                rank_key: branch_rank_key(&branch),
                retention: branch.retention,
                branch_id: branch.id,
                status: branch.status,
                choices: branch.choices,
                stop_reason: branch.stop_reason,
                summary,
                frontier,
            }
        })
        .collect::<Vec<_>>();
    branch_reports.sort_by(|left, right| {
        retention_report_slot_priority(left.retention.primary_slot)
            .cmp(&retention_report_slot_priority(
                right.retention.primary_slot,
            ))
            .then_with(|| right.rank_key.cmp(&left.rank_key))
            .then_with(|| left.branch_id.cmp(&right.branch_id))
    });

    BranchExperimentReportV1 {
        schema_name: BRANCH_EXPERIMENT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_EXPERIMENT_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        policy_quality_claim: false,
        seed: report_seed,
        replay_trace_path: config
            .replay_trace_path
            .as_ref()
            .map(|path| path.display().to_string()),
        replay_trace_applied_steps,
        replay_trace_stop,
        max_branches: config.max_branches,
        max_depth: config.max_depth,
        explored_branch_points,
        branch_limit_hit,
        frontier_group_limit_hit,
        wall_limit_hit,
        elapsed_wall_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        pruned_branch_count,
        pruned_first_pick_counts: pruned_first_pick_count_reports(pruned_first_pick_counts),
        reward_option_portfolios,
        frontier_groups: frontier_groups(&branch_reports),
        branches: branch_reports,
    }
}

fn experiment_wall_limit_hit(started_at: Instant, config: &BranchExperimentConfigV1) -> bool {
    let Some(limit_ms) = config.experiment_wall_ms else {
        return false;
    };
    started_at.elapsed().as_millis() >= u128::from(limit_ms)
}

fn advance_to_experiment_boundary(branch: &mut BranchWork, config: &BranchExperimentConfigV1) {
    if is_terminal(&branch.session) || experiment_branch_options_available(&branch.session) {
        update_terminal_status(branch);
        return;
    }

    let outcome =
        branch
            .session
            .apply_command(RunControlCommand::AutoRun(RunControlAutoStepOptions {
                search: RunControlSearchCombatOptions {
                    max_nodes: config.search_max_nodes,
                    wall_ms: config.search_wall_ms,
                    max_hp_loss: config.search_max_hp_loss,
                    ..RunControlSearchCombatOptions::default()
                },
                max_operations: Some(config.auto_max_operations),
                route: RunControlRouteAutomationMode::Planner,
            }));

    match outcome {
        Ok(outcome) => {
            branch.stop_reason = first_reason_line(&outcome.message)
                .unwrap_or_else(|| current_boundary_title(&branch.session));
            update_terminal_status(branch);
        }
        Err(err) => {
            branch.status = BranchExperimentBranchStatusV1::Failed;
            branch.stop_reason = err;
        }
    }
}

fn update_terminal_status(branch: &mut BranchWork) {
    match &branch.session.engine_state {
        EngineState::GameOver(RunResult::Victory) => {
            branch.status = BranchExperimentBranchStatusV1::TerminalVictory;
            branch.stop_reason = "victory".to_string();
        }
        EngineState::GameOver(RunResult::Defeat) => {
            branch.status = BranchExperimentBranchStatusV1::TerminalDefeat;
            branch.stop_reason = "defeat".to_string();
        }
        _ => {}
    }
}

fn settle_branch_to_frontier(branch: &mut BranchWork, config: &BranchExperimentConfigV1) {
    if branch.status != BranchExperimentBranchStatusV1::Active {
        return;
    }
    advance_to_experiment_boundary(branch, config);
    if branch.status != BranchExperimentBranchStatusV1::Active || is_terminal(&branch.session) {
        return;
    }
    if !experiment_branch_options_available(&branch.session) {
        branch.status = BranchExperimentBranchStatusV1::NeedsHumanBoundary;
        branch.stop_reason = current_boundary_title(&branch.session);
    }
}

fn experiment_branch_options_available(session: &RunControlSession) -> bool {
    branch_boundary_available(session)
}

struct BranchChoiceDraft {
    depth: usize,
    kind: &'static str,
    label: String,
    command: String,
    card: Option<CardId>,
    upgrades: Option<u8>,
    success_reason: &'static str,
}

fn expand_branch_choice(
    branch: &BranchWork,
    draft: BranchChoiceDraft,
    config: &BranchExperimentConfigV1,
) -> BranchWork {
    let mut child = branch.clone();
    child.id = format!("{}.{}", child.id, draft.command);
    child.choices.push(BranchExperimentChoiceV1 {
        depth: draft.depth,
        kind: draft.kind.to_string(),
        card: draft.card,
        upgrades: draft.upgrades,
        label: draft.label,
        command: draft.command.clone(),
    });
    match apply_branch_choice(&mut child.session, &draft.command) {
        Ok(()) => {
            child.stop_reason = draft.success_reason.to_string();
            settle_branch_to_frontier(&mut child, config);
        }
        Err(err) => {
            child.status = BranchExperimentBranchStatusV1::Failed;
            child.stop_reason = err;
        }
    }
    child
}

#[derive(Clone, Debug)]
struct BranchRetentionApplyResult {
    branches: Vec<BranchWork>,
    branch_limit_hit: bool,
    frontier_group_limit_hit: bool,
    pruned_count: usize,
    pruned_first_pick_counts: BTreeMap<String, usize>,
}

fn apply_branch_retention(
    mut branches: Vec<BranchWork>,
    config: &BranchExperimentConfigV1,
) -> BranchRetentionApplyResult {
    let before_len = branches.len();
    let candidates = branches
        .iter()
        .enumerate()
        .map(|(index, branch)| {
            let choice_profiles = branch_choice_profiles(branch);
            BranchRetentionCandidateInputV1 {
                index,
                frontier_key: branch_frontier(&branch.session).key,
                rank_key: branch_rank_key(branch),
                hp: branch.session.run_state.current_hp,
                max_hp: branch.session.run_state.max_hp,
                gold: branch.session.run_state.gold,
                deck_count: branch.session.run_state.master_deck.len(),
                strategy_formation: Some(strategy_formation_summary(&branch.session)),
                trajectory: summarize_branch_trajectory_v1(&choice_profiles),
                choice_profiles,
            }
        })
        .collect::<Vec<_>>();
    let selection = select_branch_retention_portfolio_v1(
        &candidates,
        BranchRetentionConfigV1 {
            max_total: config.max_branches,
            max_per_frontier: config.max_branches_per_frontier_group,
        },
    );

    for (index, branch) in branches.iter_mut().enumerate() {
        branch.retention = selection
            .decisions_by_index
            .get(&index)
            .cloned()
            .unwrap_or_else(default_branch_retention_decision_v1);
    }

    let keep_indices = selection
        .keep_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let pruned_first_pick_counts = pruned_first_pick_counts_for_selection(&branches, &keep_indices);

    let mut branches = branches
        .into_iter()
        .enumerate()
        .filter_map(|(index, branch)| keep_indices.contains(&index).then_some(branch))
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        retention_report_slot_priority(left.retention.primary_slot)
            .cmp(&retention_report_slot_priority(
                right.retention.primary_slot,
            ))
            .then_with(|| branch_rank_key(right).cmp(&branch_rank_key(left)))
            .then_with(|| left.id.cmp(&right.id))
    });

    BranchRetentionApplyResult {
        branches,
        branch_limit_hit: selection.total_limit_hit,
        frontier_group_limit_hit: selection.frontier_limit_hit,
        pruned_count: before_len.saturating_sub(selection.keep_indices.len()),
        pruned_first_pick_counts,
    }
}

fn pruned_first_pick_counts_for_selection(
    branches: &[BranchWork],
    keep_indices: &BTreeSet<usize>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::<String, usize>::new();
    for (index, branch) in branches.iter().enumerate() {
        if keep_indices.contains(&index) {
            continue;
        }
        *counts.entry(branch_first_pick_label(branch)).or_default() += 1;
    }
    counts
}

fn branch_first_pick_label(branch: &BranchWork) -> String {
    branch
        .choices
        .first()
        .map(|choice| choice.label.clone())
        .unwrap_or_else(|| "no_card_reward_choice".to_string())
}

fn merge_pruned_first_pick_counts(
    total: &mut BTreeMap<String, usize>,
    step_counts: BTreeMap<String, usize>,
) {
    for (label, count) in step_counts {
        *total.entry(label).or_default() += count;
    }
}

fn pruned_first_pick_count_reports(
    counts: BTreeMap<String, usize>,
) -> Vec<BranchExperimentPrunedFirstPickCountV1> {
    let mut reports = counts
        .into_iter()
        .map(|(first_pick, count)| BranchExperimentPrunedFirstPickCountV1 { first_pick, count })
        .collect::<Vec<_>>();
    reports.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.first_pick.cmp(&right.first_pick))
    });
    reports
}

fn retention_report_slot_priority(slot: BranchRetentionSlotV1) -> usize {
    match slot {
        BranchRetentionSlotV1::Package => 0,
        BranchRetentionSlotV1::EngineSetup => 1,
        BranchRetentionSlotV1::Scaling => 2,
        BranchRetentionSlotV1::DefenseEngine => 3,
        BranchRetentionSlotV1::Survival => 4,
        BranchRetentionSlotV1::Frontload => 5,
        BranchRetentionSlotV1::CleanDeck => 6,
        BranchRetentionSlotV1::Diversity => 7,
    }
}

fn apply_branch_choice(session: &mut RunControlSession, command: &str) -> Result<(), String> {
    let command = parse_run_control_command(command)?;
    session.apply_command(command).map(|_| ())
}

fn current_boundary_title(session: &RunControlSession) -> String {
    build_decision_surface(session).view.header.title
}

fn first_reason_line(message: &str) -> Option<String> {
    message
        .lines()
        .find_map(|line| line.strip_prefix("Reason: ").map(str::to_string))
}

fn is_terminal(session: &RunControlSession) -> bool {
    matches!(session.engine_state, EngineState::GameOver(_))
}

fn branch_rank_key(branch: &BranchWork) -> i32 {
    match branch.status {
        BranchExperimentBranchStatusV1::TerminalVictory => 1_000_000,
        BranchExperimentBranchStatusV1::TerminalDefeat => -1_000_000,
        BranchExperimentBranchStatusV1::Failed => -900_000,
        BranchExperimentBranchStatusV1::Pruned => -800_000,
        BranchExperimentBranchStatusV1::Active
        | BranchExperimentBranchStatusV1::NeedsHumanBoundary => {
            branch.session.run_state.act_num as i32 * 10_000
                + branch.session.run_state.floor_num * 100
                + branch.session.run_state.current_hp * 10
                + branch.session.run_state.gold
        }
    }
}

fn run_summary(
    session: &RunControlSession,
    choices: &[BranchExperimentChoiceV1],
) -> BranchExperimentRunSummaryV1 {
    let formation = strategy_formation_summary(session);
    let choice_profiles = choice_profiles_from_choices(choices);
    BranchExperimentRunSummaryV1 {
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        hp: session.run_state.current_hp,
        max_hp: session.run_state.max_hp,
        gold: session.run_state.gold,
        deck_count: session.run_state.master_deck.len(),
        relic_count: session.run_state.relics.len(),
        potion_count: session
            .run_state
            .potions
            .iter()
            .filter(|potion| potion.is_some())
            .count(),
        formation_stage: formation.stage,
        formation_needs: formation.needs,
        formation_strengths: formation.strengths,
        trajectory: summarize_branch_trajectory_v1(&choice_profiles),
        boundary_title: current_boundary_title(session),
    }
}

fn strategy_formation_summary(session: &RunControlSession) -> StrategyFormationSummaryV2 {
    build_run_strategy_snapshot_from_run_state_v2(&session.run_state).formation_summary()
}

fn branch_choice_profiles(
    branch: &BranchWork,
) -> Vec<crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1> {
    choice_profiles_from_choices(&branch.choices)
}

fn choice_profiles_from_choices(
    choices: &[BranchExperimentChoiceV1],
) -> Vec<crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1> {
    choices
        .iter()
        .filter_map(|choice| {
            let card = choice.card?;
            let upgrades = choice.upgrades.unwrap_or_default();
            Some(card_reward_semantic_profile_v1(&RewardCard::new(
                card, upgrades,
            )))
        })
        .collect()
}

fn branch_frontier(session: &RunControlSession) -> BranchExperimentFrontierV1 {
    let next_card_reward_offer = active_or_visible_reward_cards(session).map(card_offer_labels);
    let boundary_title = current_boundary_title(session);
    let lineage = branch_lineage(session, &boundary_title, next_card_reward_offer.as_ref());
    let key = format!(
        "act{}:floor{}:{}:{}",
        session.run_state.act_num,
        session.run_state.floor_num,
        boundary_title,
        lineage.same_reward_offer_lineage_key
    );
    BranchExperimentFrontierV1 {
        key,
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        boundary_title,
        card_rng_counter: session.run_state.rng_pool.card_rng.counter,
        card_blizz_randomizer: session.run_state.card_blizz_randomizer,
        next_card_reward_offer,
        lineage,
    }
}

fn branch_lineage(
    session: &RunControlSession,
    boundary_title: &str,
    next_card_reward_offer: Option<&Vec<String>>,
) -> BranchExperimentLineageV1 {
    let reward_screen_context = reward_screen_context_label(session)
        .map(str::to_string)
        .unwrap_or_else(|| "none".to_string());
    let reward_count_modifiers = reward_count_modifiers(session);
    let card_pool_modifiers = card_pool_modifiers(session);
    let rarity_modifiers = rarity_modifiers(session);
    let preview_modifiers = preview_modifiers(session);
    let sequence_breakers_present = sequence_breakers_present(
        &reward_count_modifiers,
        &card_pool_modifiers,
        &rarity_modifiers,
        &preview_modifiers,
    );
    let same_reward_offer_lineage_key = format!(
        "card_rng{}:blizz{}:context{}:count{}:pool{}:rarity{}:preview{}:offer{}",
        session.run_state.rng_pool.card_rng.counter,
        session.run_state.card_blizz_randomizer,
        reward_screen_context,
        join_key_parts(&reward_count_modifiers),
        join_key_parts(&card_pool_modifiers),
        join_key_parts(&rarity_modifiers),
        join_key_parts(&preview_modifiers),
        next_card_reward_offer
            .map(|offer| offer.join("|"))
            .unwrap_or_else(|| "-".to_string())
    );

    BranchExperimentLineageV1 {
        visibility: "privileged_simulator_diagnostic".to_string(),
        public_policy_input: false,
        direct_pick_consumes_card_rng: false,
        same_reward_offer_lineage_key,
        reward_screen_context: format!("{reward_screen_context}@{boundary_title}"),
        reward_count_modifiers,
        card_pool_modifiers,
        rarity_modifiers,
        preview_modifiers,
        sequence_breakers_present,
    }
}

fn reward_screen_context_label(session: &RunControlSession) -> Option<&'static str> {
    let context = match &session.engine_state {
        EngineState::RewardScreen(reward) => reward.screen_context,
        EngineState::RewardOverlay { reward_state, .. } => reward_state.screen_context,
        _ => return None,
    };
    Some(match context {
        RewardScreenContext::Standard => "standard",
        RewardScreenContext::TreasureRoom => "treasure_room",
        RewardScreenContext::MuggedCombat => "mugged_combat",
        RewardScreenContext::SmokedCombat => "smoked_combat",
    })
}

fn reward_count_modifiers(session: &RunControlSession) -> Vec<String> {
    relic_flags(
        session,
        &[
            (RelicId::BustedCrown, "busted_crown_reward_count_minus_2"),
            (RelicId::QuestionCard, "question_card_reward_count_plus_1"),
            (
                RelicId::PrayerWheel,
                "prayer_wheel_extra_normal_combat_card_reward",
            ),
        ],
    )
}

fn card_pool_modifiers(session: &RunControlSession) -> Vec<String> {
    relic_flags(
        session,
        &[(RelicId::PrismaticShard, "prismatic_shard_any_color_pool")],
    )
}

fn rarity_modifiers(session: &RunControlSession) -> Vec<String> {
    relic_flags(
        session,
        &[(RelicId::NlothsGift, "nloths_gift_triple_rare_chance")],
    )
}

fn preview_modifiers(session: &RunControlSession) -> Vec<String> {
    let mut modifiers = relic_flags(
        session,
        &[
            (RelicId::MoltenEgg, "molten_egg_upgrade_attack_previews"),
            (RelicId::ToxicEgg, "toxic_egg_upgrade_skill_previews"),
            (RelicId::FrozenEgg, "frozen_egg_upgrade_power_previews"),
        ],
    );
    if session.run_state.card_upgraded_chance > 0.0 {
        modifiers.push(format!(
            "card_upgrade_chance_rng_{:.3}",
            session.run_state.card_upgraded_chance
        ));
    }
    modifiers
}

fn relic_flags(session: &RunControlSession, flags: &[(RelicId, &str)]) -> Vec<String> {
    flags
        .iter()
        .filter_map(|(relic_id, label)| {
            session
                .run_state
                .relics
                .iter()
                .any(|relic| relic.id == *relic_id)
                .then_some((*label).to_string())
        })
        .collect()
}

fn sequence_breakers_present(
    reward_count_modifiers: &[String],
    card_pool_modifiers: &[String],
    rarity_modifiers: &[String],
    preview_modifiers: &[String],
) -> Vec<String> {
    reward_count_modifiers
        .iter()
        .chain(card_pool_modifiers.iter())
        .chain(rarity_modifiers.iter())
        .chain(preview_modifiers.iter())
        .cloned()
        .collect()
}

fn join_key_parts(parts: &[String]) -> String {
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("+")
    }
}

fn frontier_groups(
    branches: &[BranchExperimentBranchReportV1],
) -> Vec<BranchExperimentFrontierGroupV1> {
    let mut groups = BTreeMap::<String, BranchExperimentFrontierGroupV1>::new();
    for branch in branches {
        groups
            .entry(branch.frontier.key.clone())
            .and_modify(|group| group.branch_count += 1)
            .or_insert_with(|| BranchExperimentFrontierGroupV1 {
                key: branch.frontier.key.clone(),
                branch_count: 1,
                representative_branch_id: branch.branch_id.clone(),
                boundary_title: branch.frontier.boundary_title.clone(),
                next_card_reward_offer: branch.frontier.next_card_reward_offer.clone(),
                lineage_flags: branch.frontier.lineage.sequence_breakers_present.clone(),
            });
    }
    let mut groups = groups.into_values().collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        right
            .branch_count
            .cmp(&left.branch_count)
            .then_with(|| left.key.cmp(&right.key))
    });
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use crate::ai::noncombat_strategy_v1::{
        StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1,
    };
    use crate::content::cards::CardId;
    use crate::content::relics::RelicState;
    use crate::state::rewards::{BossRelicChoiceState, RewardState};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn branch_experiment_expands_pending_card_reward_choices() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                ..BranchExperimentConfigV1::default()
            },
        );

        assert_eq!(report.explored_branch_points, 1);
        assert_eq!(report.branches.len(), 2);
        assert!(report.branches.iter().any(|branch| {
            branch.choices[0].command == "rp 0" && branch.choices[0].label == "Twin Strike"
        }));
        assert!(report.branches.iter().any(|branch| {
            branch.choices[0].command == "rp 1" && branch.choices[0].label == "Cleave"
        }));
    }

    #[test]
    fn branch_experiment_replay_trace_uses_trace_run_config() {
        let trace_path = write_trace_fixture(
            "branch_experiment_trace_config",
            &crate::eval::run_control::SessionTraceV1::new(&RunControlSession::new(
                RunControlConfig {
                    seed: 777,
                    ..RunControlConfig::default()
                },
            )),
        );

        let report = run_branch_experiment_v1(&BranchExperimentConfigV1 {
            seed: 1,
            replay_trace_path: Some(trace_path.clone()),
            max_depth: 0,
            ..BranchExperimentConfigV1::default()
        })
        .expect("empty trace should replay");

        assert_eq!(report.seed, 777);
        assert_eq!(
            report.replay_trace_path,
            Some(trace_path.display().to_string())
        );
        assert_eq!(report.replay_trace_applied_steps, 0);
        assert_eq!(report.replay_trace_stop, Some("TraceEnd".to_string()));

        let _ = fs::remove_dir_all(trace_path.parent().unwrap());
    }

    #[test]
    fn branch_experiment_report_counts_replayed_trace_steps() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let trace_path = unique_temp_path("branch_experiment_trace_steps").join("trace.json");
        let mut recorder =
            crate::eval::run_control::SessionTraceRecorder::new(trace_path.clone(), &session);
        let command = RunControlCommand::DefaultCandidate;
        let pending =
            crate::eval::run_control::SessionTraceRecorder::prepare_step(&session, "", &command);
        let outcome = session
            .apply_command(command)
            .expect("default candidate applies");
        recorder
            .record_action_step(
                pending,
                &session,
                outcome
                    .action_result
                    .as_ref()
                    .expect("command should change state"),
                &outcome.trace_annotations,
            )
            .expect("trace records");
        let trace = recorder.trace().clone();
        let trace_path = write_trace_fixture("branch_experiment_trace_steps", &trace);

        let report = run_branch_experiment_v1(&BranchExperimentConfigV1 {
            replay_trace_path: Some(trace_path.clone()),
            replay_trace_max_steps: Some(1),
            max_depth: 0,
            ..BranchExperimentConfigV1::default()
        })
        .expect("one step trace should replay");

        assert_eq!(report.replay_trace_applied_steps, 1);
        assert_eq!(report.replay_trace_stop, Some("TraceEnd".to_string()));

        let _ = fs::remove_dir_all(trace_path.parent().unwrap());
    }

    #[test]
    fn branch_experiment_can_limit_reward_options_by_semantic_portfolio() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                max_reward_options_per_branch: Some(2),
                auto_max_operations: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        let picked_labels = report
            .branches
            .iter()
            .map(|branch| branch.choices[0].label.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(report.branches.len(), 2);
        assert!(
            picked_labels.contains("Shrug It Off"),
            "non-transition defense/draw candidate should not be crowded out"
        );
        assert_eq!(
            picked_labels
                .iter()
                .filter(|label| **label == "Twin Strike" || **label == "Cleave")
                .count(),
            1,
            "pure transition options should be represented, not exhaustively expanded"
        );
    }

    #[test]
    fn branch_experiment_reports_reward_option_portfolio_pruning() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                max_reward_options_per_branch: Some(2),
                auto_max_operations: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        assert_eq!(report.reward_option_portfolios.len(), 1);
        let portfolio = &report.reward_option_portfolios[0];
        assert_eq!(portfolio.depth, 0);
        assert_eq!(portfolio.original_count, 3);
        assert_eq!(portfolio.selected_count, 2);
        assert_eq!(portfolio.pruned_options.len(), 1);
        assert!(portfolio
            .selected_options
            .iter()
            .any(|option| option.label == "Shrug It Off"));
        assert!(portfolio
            .pruned_options
            .iter()
            .any(|option| option.semantic_class == "pure_transition_frontload"));
    }

    #[test]
    fn branch_experiment_expands_campfire_choices() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                max_campfire_options_per_branch: Some(2),
                ..BranchExperimentConfigV1::default()
            },
        );

        assert_eq!(report.explored_branch_points, 1);
        assert!(report
            .branches
            .iter()
            .any(|branch| branch.choices.iter().any(|choice| {
                choice.kind == "campfire" && choice.command == "rest" && choice.card.is_none()
            })));
        assert!(report
            .branches
            .iter()
            .any(|branch| branch.choices.iter().any(|choice| {
                choice.kind == "campfire"
                    && choice.command.starts_with("smith ")
                    && choice.card.is_some()
            })));
    }

    #[test]
    fn branch_experiment_expands_boss_relic_choices() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::BossRelicSelect(BossRelicChoiceState::new(vec![
            RelicId::BlackStar,
            RelicId::EmptyCage,
            RelicId::TinyHouse,
        ]));

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                auto_max_operations: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        let choices = report
            .branches
            .iter()
            .flat_map(|branch| &branch.choices)
            .collect::<Vec<_>>();
        assert_eq!(report.explored_branch_points, 1);
        assert_eq!(choices.len(), 3);
        assert!(choices.iter().all(|choice| {
            choice.kind == "boss_relic" && choice.card.is_none() && choice.upgrades.is_none()
        }));
    }

    #[test]
    fn pruned_first_pick_count_reports_sort_for_stable_comparison() {
        let reports = pruned_first_pick_count_reports(BTreeMap::from([
            ("Shockwave".to_string(), 2),
            ("Armaments".to_string(), 4),
            ("Clash".to_string(), 4),
        ]));

        assert_eq!(
            reports,
            vec![
                BranchExperimentPrunedFirstPickCountV1 {
                    first_pick: "Armaments".to_string(),
                    count: 4,
                },
                BranchExperimentPrunedFirstPickCountV1 {
                    first_pick: "Clash".to_string(),
                    count: 4,
                },
                BranchExperimentPrunedFirstPickCountV1 {
                    first_pick: "Shockwave".to_string(),
                    count: 2,
                },
            ]
        );
    }

    #[test]
    fn recorded_card_reward_pick_does_not_consume_card_reward_rng() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);
        let card_rng_counter_before = session.run_state.rng_pool.card_rng.counter;

        session
            .apply_command(RunControlCommand::RecordedCardRewardPick(0))
            .expect("recorded pick applies");

        assert_eq!(
            session.run_state.rng_pool.card_rng.counter, card_rng_counter_before,
            "card reward choices are generated before the player picks; picking a card must not consume card reward RNG"
        );
    }

    #[test]
    fn branch_lineage_is_privileged_and_not_public_policy_input() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![RewardCard::new(CardId::TwinStrike, 0)]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        let lineage = &report.branches[0].frontier.lineage;
        assert_eq!(lineage.visibility, "privileged_simulator_diagnostic");
        assert!(!lineage.public_policy_input);
        assert!(!lineage.direct_pick_consumes_card_rng);
        assert!(lineage.sequence_breakers_present.is_empty());
    }

    #[test]
    fn branch_lineage_reports_reward_sequence_breakers() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.relics.clear();
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::QuestionCard));
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::PrayerWheel));
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::PrismaticShard));
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::NlothsGift));
        session.run_state.card_upgraded_chance = 0.25;
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![RewardCard::new(CardId::TwinStrike, 1)]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        let lineage = &report.branches[0].frontier.lineage;
        assert!(lineage
            .reward_count_modifiers
            .contains(&"question_card_reward_count_plus_1".to_string()));
        assert!(lineage
            .reward_count_modifiers
            .contains(&"prayer_wheel_extra_normal_combat_card_reward".to_string()));
        assert!(lineage
            .card_pool_modifiers
            .contains(&"prismatic_shard_any_color_pool".to_string()));
        assert!(lineage
            .rarity_modifiers
            .contains(&"nloths_gift_triple_rare_chance".to_string()));
        assert!(lineage
            .preview_modifiers
            .contains(&"card_upgrade_chance_rng_0.250".to_string()));
        assert_eq!(
            report.frontier_groups[0].lineage_flags,
            lineage.sequence_breakers_present
        );
    }

    #[test]
    fn branch_report_exposes_strategy_formation_summary_used_for_retention() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![RewardCard::new(CardId::TwinStrike, 0)]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 2,
                auto_max_operations: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        let summary = &report.branches[0].summary;
        assert_eq!(
            summary.formation_stage,
            StrategyDeckFormationStageV1::StarterShell
        );
        assert!(summary
            .formation_needs
            .contains(&StrategyDeckFormationNeedV1::Frontload));
        assert_eq!(summary.trajectory.frontload_picks, 1);
        assert_eq!(summary.trajectory.transition_frontload_picks, 1);
    }

    #[test]
    fn branch_experiment_settles_after_last_depth_choice() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                auto_max_operations: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        assert!(
            report
                .branches
                .iter()
                .all(|branch| branch.stop_reason != "card reward branch applied"),
            "depth-exhausted branch results should be settled to a readable frontier, not left at an internal transition"
        );
    }

    #[test]
    fn branch_experiment_prunes_to_max_branches() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 2,
                ..BranchExperimentConfigV1::default()
            },
        );

        assert!(report.branch_limit_hit);
        assert_eq!(report.branches.len(), 2);
    }

    #[test]
    fn branch_experiment_caps_same_frontier_group_variants() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                max_branches_per_frontier_group: Some(1),
                auto_max_operations: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        assert!(report.frontier_group_limit_hit);
        assert_eq!(report.pruned_branch_count, 1);
        assert_eq!(report.branches.len(), 1);
        assert_eq!(report.frontier_groups.len(), 1);
    }

    fn write_trace_fixture(
        label: &str,
        trace: &crate::eval::run_control::SessionTraceV1,
    ) -> PathBuf {
        let path = unique_temp_path(label).join("trace.json");
        fs::create_dir_all(path.parent().unwrap()).expect("trace parent should exist");
        fs::write(
            &path,
            serde_json::to_string_pretty(trace).expect("trace should serialize"),
        )
        .expect("trace fixture should write");
        path
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "sts_simulator_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should be available")
                .as_nanos()
        ));
        path
    }
}
