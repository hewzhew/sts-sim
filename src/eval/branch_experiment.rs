use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyFormationSummaryV2,
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
    RunControlConfig, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
    RunControlSession, SessionTraceReplayOptions, SessionTraceReplayStop,
};
use crate::state::core::{EngineState, RunResult};
use crate::state::rewards::{RewardCard, RewardScreenContext};

mod types;

pub use types::{
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentChoiceCardV1,
    BranchExperimentChoiceV1, BranchExperimentConfigV1, BranchExperimentFrontierGroupV1,
    BranchExperimentFrontierV1, BranchExperimentLineageV1, BranchExperimentPrunedBranchSummaryV1,
    BranchExperimentPrunedFirstPickCountV1, BranchExperimentReportV1,
    BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRewardOptionPortfolioV1,
    BranchExperimentRunSummaryV1, BRANCH_EXPERIMENT_SCHEMA_NAME, BRANCH_EXPERIMENT_SCHEMA_VERSION,
};
#[derive(Clone, Debug)]
struct BranchWork {
    id: String,
    session: RunControlSession,
    choices: Vec<BranchExperimentChoiceV1>,
    status: BranchExperimentBranchStatusV1,
    stop_reason: String,
    retention: BranchRetentionDecisionV1,
}

#[derive(Clone, Debug)]
struct BranchExperimentPreparedStart {
    branch: BranchWork,
    replay_trace_applied_steps: usize,
    replay_trace_stop: Option<String>,
}

pub fn run_branch_experiment_v1(
    config: &BranchExperimentConfigV1,
) -> Result<BranchExperimentReportV1, String> {
    let prepared = prepare_branch_experiment_start(config, false)?;
    Ok(run_branch_experiment_from_start_branch_with_replay(
        prepared.branch,
        config,
        prepared.replay_trace_applied_steps,
        prepared.replay_trace_stop,
    ))
}

pub fn run_branch_experiment_profiles_from_shared_start_v1(
    configs: &[BranchExperimentConfigV1],
) -> Result<Vec<BranchExperimentReportV1>, String> {
    let Some(first_config) = configs.first() else {
        return Ok(Vec::new());
    };
    validate_shared_start_configs(configs)?;
    let prepared = prepare_branch_experiment_start(first_config, true)?;
    configs
        .iter()
        .map(|config| {
            Ok(run_branch_experiment_from_start_branch_with_replay(
                prepared.branch.clone(),
                config,
                prepared.replay_trace_applied_steps,
                prepared.replay_trace_stop.clone(),
            ))
        })
        .collect()
}

fn validate_shared_start_configs(configs: &[BranchExperimentConfigV1]) -> Result<(), String> {
    let Some(first) = configs.first() else {
        return Ok(());
    };
    for config in configs.iter().skip(1) {
        macro_rules! require_same {
            ($field:ident) => {
                ensure_shared_start_field(stringify!($field), &first.$field, &config.$field)?;
            };
        }

        require_same!(seed);
        require_same!(ascension_level);
        require_same!(player_class);
        require_same!(final_act);
        require_same!(max_branches);
        require_same!(max_branches_per_frontier_group);
        require_same!(max_reward_options_per_branch);
        require_same!(max_campfire_options_per_branch);
        require_same!(max_depth);
        require_same!(auto_max_operations);
        require_same!(experiment_wall_ms);
        require_same!(search_max_nodes);
        require_same!(search_wall_ms);
        require_same!(search_max_hp_loss);
        require_same!(include_skip);
        require_same!(prefix_commands);
        require_same!(replay_trace_path);
        require_same!(replay_trace_max_steps);
    }
    Ok(())
}

fn ensure_shared_start_field<T: PartialEq + ?Sized>(
    field: &str,
    expected: &T,
    actual: &T,
) -> Result<(), String> {
    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "shared-start profile configs differ in {field}; only retention_budget_profile may vary"
        ))
    }
}

fn prepare_branch_experiment_start(
    config: &BranchExperimentConfigV1,
    settle_to_first_boundary: bool,
) -> Result<BranchExperimentPreparedStart, String> {
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

    let mut branch = BranchWork {
        id: "root".to_string(),
        session,
        choices: Vec::new(),
        status: BranchExperimentBranchStatusV1::Active,
        stop_reason: "initial".to_string(),
        retention: default_branch_retention_decision_v1(),
    };
    if settle_to_first_boundary {
        settle_branch_to_frontier(&mut branch, config);
    }

    Ok(BranchExperimentPreparedStart {
        branch,
        replay_trace_applied_steps: replay_applied_steps,
        replay_trace_stop: replay_stop,
    })
}

#[cfg(test)]
fn run_branch_experiment_from_session(
    session: RunControlSession,
    config: &BranchExperimentConfigV1,
) -> BranchExperimentReportV1 {
    run_branch_experiment_from_session_with_replay(session, config, 0, None)
}

#[cfg(test)]
fn run_branch_experiment_from_session_with_replay(
    session: RunControlSession,
    config: &BranchExperimentConfigV1,
    replay_trace_applied_steps: usize,
    replay_trace_stop: Option<String>,
) -> BranchExperimentReportV1 {
    run_branch_experiment_from_start_branch_with_replay(
        BranchWork {
            id: "root".to_string(),
            session,
            choices: Vec::new(),
            status: BranchExperimentBranchStatusV1::Active,
            stop_reason: "initial".to_string(),
            retention: default_branch_retention_decision_v1(),
        },
        config,
        replay_trace_applied_steps,
        replay_trace_stop,
    )
}

fn run_branch_experiment_from_start_branch_with_replay(
    start_branch: BranchWork,
    config: &BranchExperimentConfigV1,
    replay_trace_applied_steps: usize,
    replay_trace_stop: Option<String>,
) -> BranchExperimentReportV1 {
    let started_at = Instant::now();
    let mut branches = vec![start_branch];
    let report_seed = branches[0].session.run_state.seed;
    let mut explored_branch_points = 0usize;
    let mut branch_limit_hit = false;
    let mut frontier_group_limit_hit = false;
    let mut wall_limit_hit = false;
    let mut pruned_branch_count = 0usize;
    let mut pruned_first_pick_counts = BTreeMap::<String, usize>::new();
    let mut pruned_branch_summary = BranchExperimentPrunedBranchSummaryV1::default();
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
                include_skip: config.include_skip,
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
                            selected_cards: option.selected_cards,
                            effect_kind: option.effect_kind,
                            effect_key: option.effect_key,
                            effect_label: option.effect_label,
                            representative_count: option.representative_count,
                            suppressed_count: option.suppressed_count,
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
        merge_pruned_branch_summary(&mut pruned_branch_summary, retention.pruned_branch_summary);

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
        retention_report_slot_priority(retention_report_slot(left.retention.selected_by_slot))
            .cmp(&retention_report_slot_priority(retention_report_slot(
                right.retention.selected_by_slot,
            )))
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
        retention_profile: config.retention_budget_profile,
        explored_branch_points,
        branch_limit_hit,
        frontier_group_limit_hit,
        wall_limit_hit,
        elapsed_wall_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        pruned_branch_count,
        pruned_first_pick_counts: pruned_first_pick_count_reports(pruned_first_pick_counts),
        pruned_branch_summary,
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
    selected_cards: Vec<BranchExperimentChoiceCardV1>,
    effect_kind: String,
    effect_key: String,
    effect_label: String,
    representative_count: usize,
    suppressed_count: usize,
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
        selected_cards: draft.selected_cards,
        effect_kind: draft.effect_kind,
        effect_key: draft.effect_key,
        effect_label: draft.effect_label,
        representative_count: draft.representative_count,
        suppressed_count: draft.suppressed_count,
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
    pruned_branch_summary: BranchExperimentPrunedBranchSummaryV1,
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
            budget_profile: config.retention_budget_profile,
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
    let pruned_branch_summary = pruned_branch_summary_for_selection(
        &candidates,
        &selection.decisions_by_index,
        &keep_indices,
    );

    let mut branches = branches
        .into_iter()
        .enumerate()
        .filter_map(|(index, branch)| keep_indices.contains(&index).then_some(branch))
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        retention_report_slot_priority(retention_report_slot(left.retention.selected_by_slot))
            .cmp(&retention_report_slot_priority(retention_report_slot(
                right.retention.selected_by_slot,
            )))
            .then_with(|| branch_rank_key(right).cmp(&branch_rank_key(left)))
            .then_with(|| left.id.cmp(&right.id))
    });

    BranchRetentionApplyResult {
        branches,
        branch_limit_hit: selection.total_limit_hit,
        frontier_group_limit_hit: selection.frontier_limit_hit,
        pruned_count: before_len.saturating_sub(selection.keep_indices.len()),
        pruned_first_pick_counts,
        pruned_branch_summary,
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

fn pruned_branch_summary_for_selection(
    candidates: &[BranchRetentionCandidateInputV1],
    decisions_by_index: &BTreeMap<usize, BranchRetentionDecisionV1>,
    keep_indices: &BTreeSet<usize>,
) -> BranchExperimentPrunedBranchSummaryV1 {
    let mut summary = BranchExperimentPrunedBranchSummaryV1::default();
    for candidate in candidates {
        if keep_indices.contains(&candidate.index) {
            continue;
        }
        let Some(decision) = decisions_by_index.get(&candidate.index) else {
            continue;
        };
        *summary
            .primary_slot_counts
            .entry(decision.primary_slot)
            .or_default() += 1;
        for slot in &decision.slots {
            *summary.eligible_slot_counts.entry(*slot).or_default() += 1;
        }
        for state in branch_trajectory_package_state_tags(&candidate.trajectory) {
            *summary.package_state_counts.entry(state).or_default() += 1;
        }
    }
    summary
}

fn branch_trajectory_package_state_tags(trajectory: &BranchTrajectorySignatureV1) -> Vec<String> {
    let setup_keys = trajectory
        .setup_keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let package_keys = trajectory
        .package_keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut tags = Vec::new();
    for key in setup_keys.intersection(&package_keys) {
        tags.push(format!("closed:{key}"));
    }
    for key in setup_keys.difference(&package_keys) {
        tags.push(format!("open:{key}"));
    }
    for key in package_keys.difference(&setup_keys) {
        tags.push(format!("payoff_only:{key}"));
    }
    tags
}

fn branch_first_pick_label(branch: &BranchWork) -> String {
    branch
        .choices
        .first()
        .map(branch_choice_display_label)
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

fn merge_pruned_branch_summary(
    total: &mut BranchExperimentPrunedBranchSummaryV1,
    step: BranchExperimentPrunedBranchSummaryV1,
) {
    merge_count_map(&mut total.primary_slot_counts, step.primary_slot_counts);
    merge_count_map(&mut total.eligible_slot_counts, step.eligible_slot_counts);
    merge_count_map(&mut total.package_state_counts, step.package_state_counts);
}

fn merge_count_map<K: Ord>(total: &mut BTreeMap<K, usize>, step: BTreeMap<K, usize>) {
    for (key, count) in step {
        *total.entry(key).or_default() += count;
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

fn retention_report_slot(slot: Option<BranchRetentionSlotV1>) -> BranchRetentionSlotV1 {
    slot.unwrap_or(BranchRetentionSlotV1::Diversity)
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
        .flat_map(|choice| {
            choice_profile_cards(choice)
                .into_iter()
                .map(|selected| {
                    card_reward_semantic_profile_v1(&RewardCard::new(
                        selected.card,
                        selected.upgrades,
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn choice_profile_cards(choice: &BranchExperimentChoiceV1) -> Vec<BranchExperimentChoiceCardV1> {
    if !choice.effect_kind.is_empty()
        && !matches!(choice.effect_kind.as_str(), "add_card" | "duplicate_card")
    {
        return Vec::new();
    }
    if !choice.selected_cards.is_empty() {
        return choice.selected_cards.clone();
    }
    choice
        .card
        .map(|card| {
            vec![BranchExperimentChoiceCardV1 {
                card,
                upgrades: choice.upgrades.unwrap_or_default(),
            }]
        })
        .unwrap_or_default()
}

fn branch_choice_display_label(choice: &BranchExperimentChoiceV1) -> String {
    let base = if choice.effect_label.is_empty() {
        choice.label.clone()
    } else {
        choice.effect_label.clone()
    };
    let count = choice.representative_count;
    if count > 1 {
        format!("{base} (covers {count})")
    } else {
        base
    }
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
mod tests;
