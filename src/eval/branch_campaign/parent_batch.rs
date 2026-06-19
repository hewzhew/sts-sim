use crate::eval::branch_experiment::{
    run_branch_experiment_from_session_after_prefix_with_snapshots_v1,
    run_branch_experiment_from_session_with_snapshots_v1, run_branch_experiment_with_snapshots_v1,
    BranchExperimentConfigV1, BranchExperimentRunResultV1, BranchExperimentStrategyRequestV1,
};

use super::branch_display::render_compact_choice_path;
use super::performance::{
    add_combat_performance_samples_v1, BranchCampaignCombatPerformanceSummaryV1,
};
use super::progress::BranchCampaignProgressEventV1;
use super::retry::{
    campaign_parent_should_retry_combat_budget_now_v1, combat_retry_campaign_config_v1,
    try_consume_branch_report_act_boss_gate_retry_v1, BranchCampaignCombatRetryLedgerStateV1,
};
use super::route_evidence::merge_campaign_route_decisions_v1;
use super::state_graph::{BranchStateReplayStartV1, BranchStateStoreV1};
use super::summary::campaign_refresh_branch_summary_from_session_v1;
use super::{
    campaign_branch_from_report_branch_v1, campaign_replay_commands_for_path_v1,
    maybe_attach_campaign_combat_lab_probe_v1, BranchCampaignBranchStatusV1,
    BranchCampaignBranchV1, BranchCampaignConfigV1, BranchCampaignRouteEvidenceSummaryV1,
};

struct BranchCampaignParentRoundResultV1 {
    result: BranchExperimentRunResultV1,
    combat_budget_retry_used: bool,
    elapsed_wall_ms_sum: u64,
    elapsed_wall_ms_max: u64,
    combat_retry_elapsed_wall_ms_sum: u64,
    combat_retry_elapsed_wall_ms_max: u64,
}

struct BranchCampaignParentBaseResultV1 {
    parent_index: usize,
    result: Result<BranchExperimentRunResultV1, String>,
}

struct BranchCampaignParentRetryRequestV1 {
    parent_index: usize,
    parent_replay_start: Option<BranchStateReplayStartV1>,
    retry_config: BranchCampaignConfigV1,
    initial_elapsed_wall_ms: Option<u64>,
    original_error: Option<String>,
}

pub(super) struct BranchCampaignParentBatchResultV1 {
    pub(super) candidates: Vec<BranchCampaignBranchV1>,
    pub(super) strategy_requests: Vec<BranchExperimentStrategyRequestV1>,
    pub(super) route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    pub(super) explored_branch_points: usize,
    pub(super) wall_limit_hit: bool,
    pub(super) branch_limit_hit: bool,
    pub(super) combat_budget_retries: usize,
    pub(super) parent_elapsed_wall_ms_sum: u64,
    pub(super) parent_elapsed_wall_ms_max: u64,
    pub(super) combat_retry_elapsed_wall_ms_sum: u64,
    pub(super) combat_retry_elapsed_wall_ms_max: u64,
    pub(super) combat_performance: BranchCampaignCombatPerformanceSummaryV1,
}

pub(super) fn run_campaign_parent_batch_v1<F>(
    config: &BranchCampaignConfigV1,
    parents: &[BranchCampaignBranchV1],
    state_store: &mut BranchStateStoreV1,
    combat_retry_ledger: &mut BranchCampaignCombatRetryLedgerStateV1,
    round_number: usize,
    round_retry: bool,
    progress: &mut F,
) -> Result<BranchCampaignParentBatchResultV1, String>
where
    F: FnMut(BranchCampaignProgressEventV1),
{
    let parent_count = parents.len();
    let mut candidates = Vec::new();
    let mut strategy_requests = Vec::new();
    let mut explored_branch_points = 0usize;
    let mut wall_limit_hit = false;
    let mut branch_limit_hit = false;
    let mut combat_budget_retries = 0usize;
    let mut route_evidence = BranchCampaignRouteEvidenceSummaryV1::default();
    let mut parent_elapsed_wall_ms_sum = 0u64;
    let mut parent_elapsed_wall_ms_max = 0u64;
    let mut combat_retry_elapsed_wall_ms_sum = 0u64;
    let mut combat_retry_elapsed_wall_ms_max = 0u64;
    let mut combat_performance = BranchCampaignCombatPerformanceSummaryV1::default();

    for (parent_index, parent) in parents.iter().enumerate() {
        progress(BranchCampaignProgressEventV1::BranchStarted {
            round: round_number,
            branch_index: parent_index + 1,
            branch_count: parent_count,
            choices: render_compact_choice_path(&parent.choice_labels),
        });
    }

    let parent_replay_starts = parents
        .iter()
        .map(|parent| state_store.replay_start_for_commands(&parent.commands))
        .collect::<Vec<_>>();
    let base_results =
        run_campaign_parent_base_passes_parallel_v1(config, parents, &parent_replay_starts)?;
    let mut parent_results: Vec<Option<BranchCampaignParentRoundResultV1>> =
        std::iter::repeat_with(|| None)
            .take(parents.len())
            .collect();
    let mut retry_requests = Vec::new();
    for base_result in base_results {
        let parent_index = base_result.parent_index;
        match campaign_parent_retry_request_or_result_v1(
            config,
            base_result,
            parent_replay_starts
                .get(parent_index)
                .cloned()
                .unwrap_or(None),
            combat_retry_ledger,
            !round_retry,
        ) {
            Ok(Ok(result)) => {
                if let Some(slot) = parent_results.get_mut(parent_index) {
                    *slot = Some(result);
                }
            }
            Ok(Err(request)) => {
                retry_requests.push(request);
            }
            Err(err) if campaign_parent_replay_error_is_branch_invalid_v1(&err) => {
                if let Some(parent) = parents.get(parent_index) {
                    candidates.push(campaign_branch_from_parent_replay_error_v1(parent, &err));
                }
            }
            Err(err) => return Err(err),
        }
    }
    let retry_results = run_campaign_parent_retry_passes_parallel_v1(parents, retry_requests)?;
    for (parent_index, parent_result) in retry_results {
        if let Some(slot) = parent_results.get_mut(parent_index) {
            *slot = Some(parent_result);
        }
    }

    for (parent_index, parent_result) in parent_results.into_iter().enumerate() {
        let Some(parent_result) = parent_result else {
            continue;
        };
        let parent = &parents[parent_index];
        parent_elapsed_wall_ms_sum =
            parent_elapsed_wall_ms_sum.saturating_add(parent_result.elapsed_wall_ms_sum);
        parent_elapsed_wall_ms_max =
            parent_elapsed_wall_ms_max.max(parent_result.elapsed_wall_ms_max);
        let (parent_retry_elapsed_sum, parent_retry_elapsed_max) =
            campaign_retry_timing_for_parent_v1(
                round_retry,
                parent_result.elapsed_wall_ms_sum,
                parent_result.elapsed_wall_ms_max,
                parent_result.combat_retry_elapsed_wall_ms_sum,
                parent_result.combat_retry_elapsed_wall_ms_max,
            );
        combat_retry_elapsed_wall_ms_sum =
            combat_retry_elapsed_wall_ms_sum.saturating_add(parent_retry_elapsed_sum);
        combat_retry_elapsed_wall_ms_max =
            combat_retry_elapsed_wall_ms_max.max(parent_retry_elapsed_max);
        let result = parent_result.result;
        add_combat_performance_samples_v1(
            &mut combat_performance,
            &result.combat_performance_samples,
        );
        let report = result.report;
        let combat_budget_retry_used = round_retry || parent_result.combat_budget_retry_used;
        if combat_budget_retry_used {
            combat_budget_retries = combat_budget_retries.saturating_add(1);
        }
        explored_branch_points =
            explored_branch_points.saturating_add(report.explored_branch_points);
        wall_limit_hit |= report.wall_limit_hit;
        branch_limit_hit |= report.branch_limit_hit || report.frontier_group_limit_hit;
        merge_campaign_route_decisions_v1(&mut route_evidence, &report.route_decisions);
        progress(BranchCampaignProgressEventV1::BranchFinished {
            round: round_number,
            branch_index: parent_index + 1,
            branch_count: parent_count,
            produced_branches: report.branches.len(),
            explored_branch_points: report.explored_branch_points,
            elapsed_wall_ms: report.elapsed_wall_ms,
            start_elapsed_wall_ms: result.start_elapsed_wall_ms,
            combat_budget_retry_used,
            wall_limit_hit: report.wall_limit_hit,
            branch_limit_hit: report.branch_limit_hit || report.frontier_group_limit_hit,
        });
        strategy_requests.extend(report.strategy_requests.iter().cloned());
        for branch in &report.branches {
            let mut child = campaign_branch_from_report_branch_v1(parent, branch);
            if let Some(snapshot) = result.branch_sessions.get(&branch.branch_id) {
                campaign_refresh_branch_summary_from_session_v1(&mut child, snapshot);
                maybe_attach_campaign_combat_lab_probe_v1(config, &mut child, snapshot);
                state_store.insert_child_session(
                    &parent.commands,
                    child.commands.clone(),
                    snapshot.clone(),
                );
            }
            candidates.push(child);
        }
    }

    Ok(BranchCampaignParentBatchResultV1 {
        candidates,
        strategy_requests,
        route_evidence,
        explored_branch_points,
        wall_limit_hit,
        branch_limit_hit,
        combat_budget_retries,
        parent_elapsed_wall_ms_sum,
        parent_elapsed_wall_ms_max,
        combat_retry_elapsed_wall_ms_sum,
        combat_retry_elapsed_wall_ms_max,
        combat_performance,
    })
}

fn run_campaign_parent_base_passes_parallel_v1(
    config: &BranchCampaignConfigV1,
    parents: &[BranchCampaignBranchV1],
    parent_replay_starts: &[Option<BranchStateReplayStartV1>],
) -> Result<Vec<BranchCampaignParentBaseResultV1>, String> {
    let joined = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for (parent_index, parent) in parents.iter().enumerate() {
            let parent_replay_start = parent_replay_starts
                .get(parent_index)
                .cloned()
                .unwrap_or(None);
            handles.push(scope.spawn(move || BranchCampaignParentBaseResultV1 {
                parent_index,
                result: run_campaign_parent_round_once_v1(config, parent, parent_replay_start),
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join())
            .collect::<Vec<_>>()
    });
    let mut results = joined
        .into_iter()
        .map(|result| result.map_err(|_| "branch campaign parent worker panicked".to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    results.sort_by_key(|result| result.parent_index);
    Ok(results)
}

fn campaign_parent_retry_request_or_result_v1(
    config: &BranchCampaignConfigV1,
    base_result: BranchCampaignParentBaseResultV1,
    parent_replay_start: Option<BranchStateReplayStartV1>,
    combat_retry_ledger: &mut BranchCampaignCombatRetryLedgerStateV1,
    parent_combat_retry_enabled: bool,
) -> Result<Result<BranchCampaignParentRoundResultV1, BranchCampaignParentRetryRequestV1>, String> {
    let parent_index = base_result.parent_index;
    let result = match base_result.result {
        Ok(result) => result,
        Err(err) if campaign_parent_replay_error_is_branch_invalid_v1(&err) => {
            return Err(err);
        }
        Err(err)
            if parent_combat_retry_enabled
                && campaign_parent_replay_error_is_retryable_v1(&err)
                && combat_retry_campaign_config_v1(config).is_some() =>
        {
            let retry_config = combat_retry_campaign_config_v1(config)
                .expect("retry config was checked before retrying parent replay");
            return Ok(Err(BranchCampaignParentRetryRequestV1 {
                parent_index,
                parent_replay_start,
                retry_config,
                initial_elapsed_wall_ms: None,
                original_error: Some(err),
            }));
        }
        Err(err) => return Err(err),
    };

    if !parent_combat_retry_enabled
        || !campaign_parent_should_retry_combat_budget_now_v1(config, &result.report.branches)
    {
        return Ok(Ok(parent_round_result_without_retry_v1(result)));
    }

    let Some(retry_config) = combat_retry_campaign_config_v1(config) else {
        return Ok(Ok(parent_round_result_without_retry_v1(result)));
    };
    if !try_consume_branch_report_act_boss_gate_retry_v1(
        combat_retry_ledger,
        &result.report.branches,
    ) {
        return Ok(Ok(parent_round_result_without_retry_v1(result)));
    }
    let initial_elapsed_wall_ms = result.report.elapsed_wall_ms;
    Ok(Err(BranchCampaignParentRetryRequestV1 {
        parent_index,
        parent_replay_start,
        retry_config,
        initial_elapsed_wall_ms: Some(initial_elapsed_wall_ms),
        original_error: None,
    }))
}

fn parent_round_result_without_retry_v1(
    result: BranchExperimentRunResultV1,
) -> BranchCampaignParentRoundResultV1 {
    let elapsed = result.report.elapsed_wall_ms;
    BranchCampaignParentRoundResultV1 {
        result,
        combat_budget_retry_used: false,
        elapsed_wall_ms_sum: elapsed,
        elapsed_wall_ms_max: elapsed,
        combat_retry_elapsed_wall_ms_sum: 0,
        combat_retry_elapsed_wall_ms_max: 0,
    }
}

fn run_campaign_parent_retry_passes_parallel_v1(
    parents: &[BranchCampaignBranchV1],
    requests: Vec<BranchCampaignParentRetryRequestV1>,
) -> Result<Vec<(usize, BranchCampaignParentRoundResultV1)>, String> {
    if requests.is_empty() {
        return Ok(Vec::new());
    }
    let joined = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for request in requests {
            let parent = &parents[request.parent_index];
            handles.push(scope.spawn(move || {
                let parent_index = request.parent_index;
                let initial_elapsed_wall_ms = request.initial_elapsed_wall_ms;
                let original_error = request.original_error;
                let result = run_campaign_parent_round_once_v1(
                    &request.retry_config,
                    parent,
                    request.parent_replay_start,
                );
                (
                    parent_index,
                    initial_elapsed_wall_ms,
                    original_error,
                    result,
                )
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join())
            .collect::<Vec<_>>()
    });
    let mut results = Vec::new();
    for joined_result in joined {
        let (parent_index, initial_elapsed_wall_ms, original_error, retry_result) =
            joined_result
                .map_err(|_| "branch campaign parent retry worker panicked".to_string())?;
        let result = retry_result
            .map_err(|retry_err| {
                original_error
                    .as_ref()
                    .map(|err| {
                        format!(
                            "campaign parent replay failed before retry: {err}\nretry also failed: {retry_err}"
                        )
                    })
                    .unwrap_or(retry_err)
            })?;
        let retry_elapsed = result.report.elapsed_wall_ms;
        let elapsed_wall_ms_sum = initial_elapsed_wall_ms
            .map(|initial| initial.saturating_add(retry_elapsed))
            .unwrap_or(retry_elapsed);
        let elapsed_wall_ms_max = initial_elapsed_wall_ms
            .map(|initial| initial.max(retry_elapsed))
            .unwrap_or(retry_elapsed);
        results.push((
            parent_index,
            BranchCampaignParentRoundResultV1 {
                result,
                combat_budget_retry_used: true,
                elapsed_wall_ms_sum,
                elapsed_wall_ms_max,
                combat_retry_elapsed_wall_ms_sum: retry_elapsed,
                combat_retry_elapsed_wall_ms_max: retry_elapsed,
            },
        ));
    }
    results.sort_by_key(|(parent_index, _)| *parent_index);
    Ok(results)
}

pub(super) fn campaign_retry_timing_for_parent_v1(
    round_retry: bool,
    parent_elapsed_wall_ms_sum: u64,
    parent_elapsed_wall_ms_max: u64,
    parent_retry_elapsed_wall_ms_sum: u64,
    parent_retry_elapsed_wall_ms_max: u64,
) -> (u64, u64) {
    if round_retry {
        (parent_elapsed_wall_ms_sum, parent_elapsed_wall_ms_max)
    } else {
        (
            parent_retry_elapsed_wall_ms_sum,
            parent_retry_elapsed_wall_ms_max,
        )
    }
}

fn run_campaign_parent_round_once_v1(
    config: &BranchCampaignConfigV1,
    parent: &BranchCampaignBranchV1,
    parent_replay_start: Option<BranchStateReplayStartV1>,
) -> Result<BranchExperimentRunResultV1, String> {
    let mut experiment_config = campaign_branch_experiment_config_v1(config);
    if let Some(replay_start) = parent_replay_start {
        experiment_config.prefix_commands.clear();
        if !replay_start.suffix_commands.is_empty() {
            return run_branch_experiment_from_session_after_prefix_with_snapshots_v1(
                replay_start.session,
                &experiment_config,
                &campaign_replay_commands_for_path_v1(&replay_start.suffix_commands),
            );
        }
        return Ok(run_branch_experiment_from_session_with_snapshots_v1(
            replay_start.session,
            &experiment_config,
        ));
    }
    experiment_config.prefix_commands = config.prefix_commands.clone();
    experiment_config
        .prefix_commands
        .extend(campaign_replay_commands_for_path_v1(&parent.commands));
    run_branch_experiment_with_snapshots_v1(&experiment_config)
}

pub(super) fn campaign_branch_experiment_config_v1(
    config: &BranchCampaignConfigV1,
) -> BranchExperimentConfigV1 {
    BranchExperimentConfigV1 {
        seed: config.seed,
        ascension_level: config.ascension_level,
        player_class: config.player_class,
        final_act: config.final_act,
        max_branches: config.max_branches_per_active,
        retention_budget_profile: config.retention_budget_profile,
        max_reward_options_per_branch: config.max_reward_options_per_branch,
        max_campfire_options_per_branch: Some(config.max_campfire_options_per_branch),
        max_depth: config.round_depth,
        auto_max_operations: config.auto_max_operations,
        experiment_wall_ms: config.experiment_wall_ms,
        search_max_nodes: config.search_max_nodes,
        search_wall_ms: config.search_wall_ms,
        search_max_hp_loss: config.search_max_hp_loss,
        search_options: config.search_options.clone(),
        auto_capture: config.auto_capture.clone(),
        include_skip: true,
        include_event_reward_skip: config.include_event_reward_skip,
        auto_leave_after_shop_purchase_branch: true,
        ..BranchExperimentConfigV1::default()
    }
}

pub(super) fn campaign_parent_replay_error_is_retryable_v1(error: &str) -> bool {
    error.contains("is not valid on the current screen")
        || error.contains("is only valid on a card reward item or card reward screen")
}

fn campaign_parent_replay_error_is_branch_invalid_v1(error: &str) -> bool {
    error.contains("branch-skip-card-reward is only valid on a reward screen")
        || error.contains("rp <idx> is only valid on a card reward item or card reward screen")
}

fn campaign_branch_from_parent_replay_error_v1(
    parent: &BranchCampaignBranchV1,
    error: &str,
) -> BranchCampaignBranchV1 {
    let mut branch = parent.clone();
    branch.branch_id = format!("{}.replay-error", parent.branch_id);
    branch.status = BranchCampaignBranchStatusV1::Abandoned;
    branch.stop_reason = format!("parent replay failed: {error}");
    branch.rank_key = -900_000;
    branch.final_boss_combat_record = None;
    branch.combat_lab_probes.clear();
    branch
}
