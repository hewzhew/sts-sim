use crate::eval::branch_experiment::{
    run_branch_experiment_from_session_after_prefix_with_snapshots_v1,
    run_branch_experiment_from_session_with_snapshots_v1, run_branch_experiment_with_snapshots_v1,
    BranchExperimentBossRelicCandidatePoolV1, BranchExperimentBranchReportV1,
    BranchExperimentCampfirePlanCandidatePoolV1, BranchExperimentConfigV1,
    BranchExperimentEventCandidatePoolV1, BranchExperimentRewardOptionPortfolioEntryV1,
    BranchExperimentRewardOptionPortfolioV1, BranchExperimentRouteCandidatePoolV1,
    BranchExperimentRouteDecisionV1, BranchExperimentRunResultV1,
    BranchExperimentShopPlanCandidateEntryV1, BranchExperimentShopPlanCandidatePoolV1,
    BranchExperimentStrategyRequestV1,
};
use crate::eval::campaign_journal::{
    campaign_journal_candidate_from_boss_relic_entry_v1,
    campaign_journal_candidate_from_campfire_entry_v1,
    campaign_journal_candidate_from_event_entry_v1, campaign_journal_candidate_from_route_entry_v1,
    reward_portfolio_from_journal_event_v1, CampaignJournalCandidateAdmissionStatusV1,
    CampaignJournalCandidateAdmissionTraceV1, CampaignJournalCandidateDispositionV1,
    CampaignJournalCandidateV1, CampaignJournalEventPayloadV1, CampaignJournalEventV1,
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
    campaign_branch_from_report_branch_v1, campaign_child_branch_id_v1,
    campaign_replay_commands_for_path_v1, maybe_attach_campaign_combat_lab_probe_v1,
    BranchCampaignBranchStatusV1, BranchCampaignBranchV1, BranchCampaignConfigV1,
    BranchCampaignDecisionObservationV1, BranchCampaignRouteEvidenceSummaryV1,
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
    pub(super) decision_parent_anchor_commands: Vec<Vec<String>>,
    pub(super) strategy_requests: Vec<BranchExperimentStrategyRequestV1>,
    pub(super) route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    pub(super) decision_observations: Vec<BranchCampaignDecisionObservationV1>,
    pub(super) journal_events: Vec<CampaignJournalEventV1>,
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
    let mut decision_parent_anchor_commands = Vec::new();
    let mut strategy_requests = Vec::new();
    let mut explored_branch_points = 0usize;
    let mut wall_limit_hit = false;
    let mut branch_limit_hit = false;
    let mut combat_budget_retries = 0usize;
    let mut route_evidence = BranchCampaignRouteEvidenceSummaryV1::default();
    let mut decision_observations = Vec::new();
    let mut journal_events = Vec::new();
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
        for (local_commands, snapshot) in result.decision_parent_sessions {
            let mut full_commands = parent.commands.clone();
            full_commands.extend(local_commands);
            state_store.insert_child_session(&parent.commands, full_commands.clone(), snapshot);
            decision_parent_anchor_commands.push(full_commands);
        }
        let combat_budget_retry_used = round_retry || parent_result.combat_budget_retry_used;
        let parent_journal_events = campaign_journal_events_from_report_v1(
            parent,
            parent_index,
            round_number,
            combat_budget_retry_used,
            &report,
        );
        decision_observations.extend(campaign_decision_observations_from_journal_events_v1(
            &parent_journal_events,
        ));
        journal_events.extend(parent_journal_events);
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
        decision_parent_anchor_commands,
        strategy_requests,
        route_evidence,
        decision_observations,
        journal_events,
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

fn campaign_journal_events_from_report_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    report: &crate::eval::branch_experiment::BranchExperimentReportV1,
) -> Vec<CampaignJournalEventV1> {
    let (parent_act, parent_floor) = parent
        .summary
        .as_ref()
        .map(|summary| (summary.act, summary.floor))
        .unwrap_or_default();
    let mut events = report
        .reward_option_portfolios
        .iter()
        .enumerate()
        .map(|(portfolio_index, portfolio)| {
            campaign_reward_portfolio_journal_event_v1(
                parent,
                parent_index,
                round_number,
                combat_budget_retry_used,
                parent_act,
                parent_floor,
                portfolio_index,
                portfolio,
            )
        })
        .collect::<Vec<_>>();
    events.extend(campaign_shop_branch_journal_events_v1(
        parent,
        parent_index,
        round_number,
        combat_budget_retry_used,
        parent_act,
        parent_floor,
        report,
    ));
    events.extend(campaign_campfire_branch_journal_events_v1(
        parent,
        parent_index,
        round_number,
        combat_budget_retry_used,
        parent_act,
        parent_floor,
        report,
    ));
    events.extend(campaign_event_branch_journal_events_v1(
        parent,
        parent_index,
        round_number,
        combat_budget_retry_used,
        parent_act,
        parent_floor,
        report,
    ));
    events.extend(campaign_boss_relic_branch_journal_events_v1(
        parent,
        parent_index,
        round_number,
        combat_budget_retry_used,
        parent_act,
        parent_floor,
        report,
    ));
    events.extend(campaign_route_candidate_pool_journal_events_v1(
        parent,
        parent_index,
        round_number,
        combat_budget_retry_used,
        parent_act,
        parent_floor,
        report,
    ));
    events.extend(campaign_route_decision_journal_events_v1(
        parent,
        parent_index,
        round_number,
        combat_budget_retry_used,
        parent_act,
        parent_floor,
        report,
    ));
    events
}

fn campaign_reward_portfolio_journal_event_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    parent_act: u8,
    parent_floor: i32,
    portfolio_index: usize,
    portfolio: &BranchExperimentRewardOptionPortfolioV1,
) -> CampaignJournalEventV1 {
    let decision_id = format!(
        "{}:round{}:reward_portfolio{}",
        parent.branch_id, round_number, portfolio_index
    );
    let branch_id = campaign_pool_branch_id_v1(parent, &portfolio.branch_id);
    let branch_choices = campaign_pool_branch_choices_v1(parent, &portfolio.branch_choices);
    let branch_commands = campaign_pool_branch_commands_v1(parent, &portfolio.branch_commands);
    CampaignJournalEventV1 {
        event_id: format!("{decision_id}:candidate_set"),
        round: round_number,
        branch_id,
        branch_index: parent_index,
        branch_frontier_title: parent.frontier_title.clone(),
        act: parent_act,
        floor: parent_floor,
        branch_choices,
        branch_commands,
        combat_budget_retry_used,
        payload: CampaignJournalEventPayloadV1::RewardCandidateSet {
            decision_id,
            boundary_title: portfolio.boundary_title.clone(),
            frontier_key: portfolio.frontier_key.clone(),
            depth: portfolio.depth,
            max_reward_options_per_branch: portfolio.max_reward_options_per_branch,
            original_count: portfolio.original_count,
            selected_count: portfolio.selected_count,
            candidates: reward_portfolio_candidates_v1(portfolio),
        },
    }
}

fn reward_portfolio_candidates_v1(
    portfolio: &BranchExperimentRewardOptionPortfolioV1,
) -> Vec<CampaignJournalCandidateV1> {
    portfolio
        .selected_options
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            journal_candidate_from_reward_entry_v1(
                "kept",
                index,
                entry,
                CampaignJournalCandidateDispositionV1::Kept,
            )
        })
        .chain(
            portfolio
                .pruned_options
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    journal_candidate_from_reward_entry_v1(
                        "pruned",
                        index,
                        entry,
                        CampaignJournalCandidateDispositionV1::Pruned,
                    )
                }),
        )
        .collect()
}

fn journal_candidate_from_reward_entry_v1(
    group: &str,
    index: usize,
    entry: &BranchExperimentRewardOptionPortfolioEntryV1,
    disposition: CampaignJournalCandidateDispositionV1,
) -> CampaignJournalCandidateV1 {
    CampaignJournalCandidateV1 {
        candidate_id: format!("{group}:{index}:{}", entry.command),
        command: entry.command.clone(),
        label: entry.label.clone(),
        semantic_class: entry.semantic_class.clone(),
        admission: CampaignJournalCandidateAdmissionTraceV1::from_disposition(
            disposition,
            "reward_portfolio",
            group,
        )
        .with_lane(group),
        disposition,
    }
}

fn campaign_shop_branch_journal_events_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    parent_act: u8,
    parent_floor: i32,
    report: &crate::eval::branch_experiment::BranchExperimentReportV1,
) -> Vec<CampaignJournalEventV1> {
    report
        .shop_plan_candidate_pools
        .iter()
        .enumerate()
        .map(|(group_index, pool)| {
            let decision_id = format!(
                "{}:round{}:shop_candidate_pool{}",
                parent.branch_id, round_number, group_index
            );
            let branch_id = campaign_pool_branch_id_v1(parent, &pool.branch_id);
            let branch_choices = campaign_pool_branch_choices_v1(parent, &pool.branch_choices);
            let branch_commands = campaign_pool_branch_commands_v1(parent, &pool.branch_commands);
            CampaignJournalEventV1 {
                event_id: format!("{decision_id}:candidate_set"),
                round: round_number,
                branch_id,
                branch_index: parent_index,
                branch_frontier_title: parent.frontier_title.clone(),
                act: parent_act,
                floor: parent_floor,
                branch_choices,
                branch_commands,
                combat_budget_retry_used,
                payload: CampaignJournalEventPayloadV1::ShopCandidatePool {
                    decision_id,
                    boundary_title: pool.boundary_title.clone(),
                    frontier_key: pool.frontier_key.clone(),
                    depth: pool.depth,
                    candidate_count: pool.candidate_count,
                    branch_frontier_count: pool.branch_frontier_count,
                    rollout_head_plan_id: pool.rollout_head_plan_id.clone(),
                    candidates: shop_candidate_pool_candidates_v1(pool),
                },
            }
        })
        .collect()
}

fn shop_candidate_pool_candidates_v1(
    pool: &BranchExperimentShopPlanCandidatePoolV1,
) -> Vec<CampaignJournalCandidateV1> {
    pool.candidates
        .iter()
        .map(journal_candidate_from_shop_candidate_entry_v1)
        .collect()
}

fn journal_candidate_from_shop_candidate_entry_v1(
    candidate: &BranchExperimentShopPlanCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    CampaignJournalCandidateV1 {
        candidate_id: candidate.plan_id.clone(),
        command: candidate.command.clone(),
        label: candidate.label.clone(),
        semantic_class: shop_candidate_semantic_class_v1(candidate),
        admission: CampaignJournalCandidateAdmissionTraceV1::new(
            shop_candidate_admission_status_v1(&candidate.branch_admission),
            "shop_candidate_pool",
            candidate.branch_admission.clone(),
        )
        .with_lane(candidate.lane.clone())
        .with_counts(0, candidate.suppressed_count),
        disposition: if candidate.branch_admission == "Admit" {
            CampaignJournalCandidateDispositionV1::Kept
        } else {
            CampaignJournalCandidateDispositionV1::Pruned
        },
    }
}

fn shop_candidate_semantic_class_v1(
    candidate: &BranchExperimentShopPlanCandidateEntryV1,
) -> String {
    let mut parts = vec![
        format!("role:{}", candidate.role),
        format!("source:{}", candidate.source),
        format!("kind:{}", candidate.kind),
        format!("lane:{}", candidate.lane),
        format!("verdict:{}", candidate.verdict),
        format!("rollout:{}", candidate.rollout_admission),
        format!("branch:{}", candidate.branch_admission),
        format!("tier:{}", candidate.tier),
        format!("score:{}", candidate.score),
        format!("confidence_milli:{}", candidate.confidence_milli),
        format!("component_net_rank:{}", candidate.component_net_rank),
        format!("gold:{}", candidate.total_gold_spent),
    ];
    if let Some(priority) = candidate.legacy_priority {
        parts.push(format!("legacy_priority:{priority}"));
    }
    if candidate.suppressed_count > 0 {
        parts.push(format!("suppressed:{}", candidate.suppressed_count));
    }
    if !candidate.projection_roles.is_empty() {
        parts.push(format!(
            "projection:{}",
            candidate.projection_roles.join("+")
        ));
    }
    parts.join(" ")
}

fn shop_candidate_admission_status_v1(
    branch_admission: &str,
) -> CampaignJournalCandidateAdmissionStatusV1 {
    match branch_admission.to_ascii_lowercase().as_str() {
        "admit" => CampaignJournalCandidateAdmissionStatusV1::Scheduled,
        "reject" => CampaignJournalCandidateAdmissionStatusV1::Rejected,
        _ => CampaignJournalCandidateAdmissionStatusV1::Deferred,
    }
}

fn campaign_campfire_branch_journal_events_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    parent_act: u8,
    parent_floor: i32,
    report: &crate::eval::branch_experiment::BranchExperimentReportV1,
) -> Vec<CampaignJournalEventV1> {
    report
        .campfire_plan_candidate_pools
        .iter()
        .enumerate()
        .map(|(group_index, pool)| {
            let decision_id = format!(
                "{}:round{}:campfire_candidate_pool{}",
                parent.branch_id, round_number, group_index
            );
            let branch_id = campaign_pool_branch_id_v1(parent, &pool.branch_id);
            let branch_choices = campaign_pool_branch_choices_v1(parent, &pool.branch_choices);
            let branch_commands = campaign_pool_branch_commands_v1(parent, &pool.branch_commands);
            CampaignJournalEventV1 {
                event_id: format!("{decision_id}:candidate_set"),
                round: round_number,
                branch_id,
                branch_index: parent_index,
                branch_frontier_title: parent.frontier_title.clone(),
                act: parent_act,
                floor: parent_floor,
                branch_choices,
                branch_commands,
                combat_budget_retry_used,
                payload: CampaignJournalEventPayloadV1::CampfireCandidatePool {
                    decision_id,
                    boundary_title: pool.boundary_title.clone(),
                    frontier_key: pool.frontier_key.clone(),
                    depth: pool.depth,
                    candidate_count: pool.candidate_count,
                    branch_option_count: pool.branch_option_count,
                    selected_plan_id: pool.selected_plan_id.clone(),
                    candidates: campfire_candidate_pool_candidates_v1(pool),
                },
            }
        })
        .collect()
}

fn campfire_candidate_pool_candidates_v1(
    pool: &BranchExperimentCampfirePlanCandidatePoolV1,
) -> Vec<CampaignJournalCandidateV1> {
    pool.candidates
        .iter()
        .map(campaign_journal_candidate_from_campfire_entry_v1)
        .collect()
}

fn campaign_event_branch_journal_events_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    parent_act: u8,
    parent_floor: i32,
    report: &crate::eval::branch_experiment::BranchExperimentReportV1,
) -> Vec<CampaignJournalEventV1> {
    report
        .event_candidate_pools
        .iter()
        .enumerate()
        .map(|(group_index, pool)| {
            let decision_id = format!(
                "{}:round{}:event_candidate_pool{}",
                parent.branch_id, round_number, group_index
            );
            let branch_id = campaign_pool_branch_id_v1(parent, &pool.branch_id);
            let branch_choices = campaign_pool_branch_choices_v1(parent, &pool.branch_choices);
            let branch_commands = campaign_pool_branch_commands_v1(parent, &pool.branch_commands);
            CampaignJournalEventV1 {
                event_id: format!("{decision_id}:candidate_set"),
                round: round_number,
                branch_id,
                branch_index: parent_index,
                branch_frontier_title: parent.frontier_title.clone(),
                act: parent_act,
                floor: parent_floor,
                branch_choices,
                branch_commands,
                combat_budget_retry_used,
                payload: CampaignJournalEventPayloadV1::EventCandidatePool {
                    decision_id,
                    boundary_title: pool.boundary_title.clone(),
                    frontier_key: pool.frontier_key.clone(),
                    depth: pool.depth,
                    game_event_id: pool.event_id.clone(),
                    candidate_count: pool.candidate_count,
                    branch_option_count: pool.branch_option_count,
                    candidates: event_candidate_pool_candidates_v1(pool),
                },
            }
        })
        .collect()
}

fn event_candidate_pool_candidates_v1(
    pool: &BranchExperimentEventCandidatePoolV1,
) -> Vec<CampaignJournalCandidateV1> {
    pool.candidates
        .iter()
        .map(campaign_journal_candidate_from_event_entry_v1)
        .collect()
}

fn campaign_boss_relic_branch_journal_events_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    parent_act: u8,
    parent_floor: i32,
    report: &crate::eval::branch_experiment::BranchExperimentReportV1,
) -> Vec<CampaignJournalEventV1> {
    report
        .boss_relic_candidate_pools
        .iter()
        .enumerate()
        .map(|(group_index, pool)| {
            let decision_id = format!(
                "{}:round{}:boss_relic_candidate_pool{}",
                parent.branch_id, round_number, group_index
            );
            let branch_id = campaign_pool_branch_id_v1(parent, &pool.branch_id);
            let branch_choices = campaign_pool_branch_choices_v1(parent, &pool.branch_choices);
            let branch_commands = campaign_pool_branch_commands_v1(parent, &pool.branch_commands);
            CampaignJournalEventV1 {
                event_id: format!("{decision_id}:candidate_set"),
                round: round_number,
                branch_id,
                branch_index: parent_index,
                branch_frontier_title: parent.frontier_title.clone(),
                act: parent_act,
                floor: parent_floor,
                branch_choices,
                branch_commands,
                combat_budget_retry_used,
                payload: CampaignJournalEventPayloadV1::BossRelicCandidatePool {
                    decision_id,
                    boundary_title: pool.boundary_title.clone(),
                    frontier_key: pool.frontier_key.clone(),
                    depth: pool.depth,
                    candidate_count: pool.candidate_count,
                    branch_option_count: pool.branch_option_count,
                    candidates: boss_relic_candidate_pool_candidates_v1(pool),
                },
            }
        })
        .collect()
}

fn boss_relic_candidate_pool_candidates_v1(
    pool: &BranchExperimentBossRelicCandidatePoolV1,
) -> Vec<CampaignJournalCandidateV1> {
    pool.candidates
        .iter()
        .map(campaign_journal_candidate_from_boss_relic_entry_v1)
        .collect()
}

fn campaign_route_decision_journal_events_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    parent_act: u8,
    parent_floor: i32,
    report: &crate::eval::branch_experiment::BranchExperimentReportV1,
) -> Vec<CampaignJournalEventV1> {
    report
        .route_decisions
        .iter()
        .enumerate()
        .map(|(route_index, decision)| {
            let route_branch = report
                .branches
                .iter()
                .find(|branch| branch.branch_id == decision.branch_id);
            campaign_route_decision_journal_event_v1(
                parent,
                parent_index,
                round_number,
                combat_budget_retry_used,
                parent_act,
                parent_floor,
                route_index,
                decision,
                route_branch,
            )
        })
        .collect()
}

fn campaign_route_candidate_pool_journal_events_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    parent_act: u8,
    parent_floor: i32,
    report: &crate::eval::branch_experiment::BranchExperimentReportV1,
) -> Vec<CampaignJournalEventV1> {
    report
        .route_candidate_pools
        .iter()
        .enumerate()
        .map(|(pool_index, pool)| {
            let route_branch = report
                .branches
                .iter()
                .find(|branch| branch.branch_id == pool.branch_id);
            campaign_route_candidate_pool_journal_event_v1(
                parent,
                parent_index,
                round_number,
                combat_budget_retry_used,
                parent_act,
                parent_floor,
                pool_index,
                pool,
                route_branch,
            )
        })
        .collect()
}

fn campaign_route_candidate_pool_journal_event_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    parent_act: u8,
    parent_floor: i32,
    pool_index: usize,
    pool: &BranchExperimentRouteCandidatePoolV1,
    route_branch: Option<&BranchExperimentBranchReportV1>,
) -> CampaignJournalEventV1 {
    let decision_id = format!(
        "{}:round{}:route_candidate_pool{}:{}",
        parent.branch_id, round_number, pool_index, pool.branch_id
    );
    let branch_id = campaign_child_branch_id_v1(&parent.branch_id, &pool.branch_id);
    let branch_frontier_title = route_branch
        .map(|branch| branch.frontier.boundary_title.clone())
        .unwrap_or_else(|| pool.boundary_title.clone());
    let act = route_branch
        .map(|branch| branch.frontier.act)
        .unwrap_or(parent_act);
    let floor = route_branch
        .map(|branch| branch.frontier.floor)
        .unwrap_or(parent_floor);
    let branch_choices = combine_campaign_path_v1(&parent.choice_labels, &pool.branch_choices)
        .or_else(|| {
            route_branch.and_then(|branch| {
                combine_campaign_path_v1(
                    &parent.choice_labels,
                    &branch_experiment_choice_labels_v1(branch),
                )
            })
        })
        .unwrap_or_else(|| parent.choice_labels.clone());
    let branch_commands = combine_campaign_path_v1(&parent.commands, &pool.branch_commands)
        .or_else(|| {
            route_branch.and_then(|branch| {
                combine_campaign_path_v1(
                    &parent.commands,
                    &branch_experiment_choice_commands_v1(branch),
                )
            })
        })
        .unwrap_or_else(|| parent.commands.clone());
    CampaignJournalEventV1 {
        event_id: format!("{decision_id}:candidate_set"),
        round: round_number,
        branch_id,
        branch_index: parent_index,
        branch_frontier_title,
        act,
        floor,
        branch_choices,
        branch_commands,
        combat_budget_retry_used,
        payload: CampaignJournalEventPayloadV1::RouteCandidatePool {
            decision_id,
            boundary_title: pool.boundary_title.clone(),
            frontier_key: pool.frontier_key.clone(),
            depth: pool.depth,
            candidate_count: pool.candidate_count,
            selected_index: pool.selected_index,
            candidate_pool_provenance: pool.candidate_pool_provenance.clone(),
            map_decision_packet: pool.map_decision_packet.clone(),
            candidates: route_candidate_pool_candidates_v1(pool),
        },
    }
}

fn route_candidate_pool_candidates_v1(
    pool: &BranchExperimentRouteCandidatePoolV1,
) -> Vec<CampaignJournalCandidateV1> {
    pool.candidates
        .iter()
        .map(campaign_journal_candidate_from_route_entry_v1)
        .collect()
}

fn combine_campaign_path_v1(parent_path: &[String], local_path: &[String]) -> Option<Vec<String>> {
    if local_path.is_empty() {
        return None;
    }
    let mut combined = parent_path.to_vec();
    combined.extend(local_path.iter().cloned());
    Some(combined)
}

fn campaign_pool_branch_id_v1(parent: &BranchCampaignBranchV1, local_branch_id: &str) -> String {
    if local_branch_id.is_empty() {
        parent.branch_id.clone()
    } else {
        campaign_child_branch_id_v1(&parent.branch_id, local_branch_id)
    }
}

fn campaign_pool_branch_choices_v1(
    parent: &BranchCampaignBranchV1,
    local_choices: &[String],
) -> Vec<String> {
    combine_campaign_path_v1(&parent.choice_labels, local_choices)
        .unwrap_or_else(|| parent.choice_labels.clone())
}

fn campaign_pool_branch_commands_v1(
    parent: &BranchCampaignBranchV1,
    local_commands: &[String],
) -> Vec<String> {
    combine_campaign_path_v1(&parent.commands, local_commands)
        .unwrap_or_else(|| parent.commands.clone())
}

fn branch_experiment_choice_labels_v1(branch: &BranchExperimentBranchReportV1) -> Vec<String> {
    branch
        .choices
        .iter()
        .map(|choice| choice.label.clone())
        .collect()
}

fn branch_experiment_choice_commands_v1(branch: &BranchExperimentBranchReportV1) -> Vec<String> {
    branch
        .choices
        .iter()
        .map(|choice| choice.command.clone())
        .collect()
}

fn campaign_route_decision_journal_event_v1(
    parent: &BranchCampaignBranchV1,
    parent_index: usize,
    round_number: usize,
    combat_budget_retry_used: bool,
    parent_act: u8,
    parent_floor: i32,
    route_index: usize,
    decision: &BranchExperimentRouteDecisionV1,
    route_branch: Option<&BranchExperimentBranchReportV1>,
) -> CampaignJournalEventV1 {
    let decision_id = format!(
        "{}:round{}:route_decision{}:{}",
        parent.branch_id, round_number, route_index, decision.branch_id
    );
    let branch_id = campaign_child_branch_id_v1(&parent.branch_id, &decision.branch_id);
    let branch_frontier_title = route_branch
        .map(|branch| branch.frontier.boundary_title.clone())
        .unwrap_or_else(|| parent.frontier_title.clone());
    let act = route_branch
        .map(|branch| branch.frontier.act)
        .unwrap_or(parent_act);
    let floor = route_branch
        .map(|branch| branch.frontier.floor)
        .unwrap_or(parent_floor);
    let branch_choices = combine_campaign_path_v1(&parent.choice_labels, &decision.branch_choices)
        .or_else(|| {
            route_branch.and_then(|branch| {
                combine_campaign_path_v1(
                    &parent.choice_labels,
                    &branch_experiment_choice_labels_v1(branch),
                )
            })
        })
        .unwrap_or_else(|| parent.choice_labels.clone());
    let branch_commands = combine_campaign_path_v1(&parent.commands, &decision.branch_commands)
        .or_else(|| {
            route_branch.and_then(|branch| {
                combine_campaign_path_v1(
                    &parent.commands,
                    &branch_experiment_choice_commands_v1(branch),
                )
            })
        })
        .unwrap_or_else(|| parent.commands.clone());
    CampaignJournalEventV1 {
        event_id: format!("{decision_id}:route"),
        round: round_number,
        branch_id,
        branch_index: parent_index,
        branch_frontier_title,
        act,
        floor,
        branch_choices,
        branch_commands,
        combat_budget_retry_used,
        payload: CampaignJournalEventPayloadV1::RouteDecision {
            decision_id,
            route_branch_id: decision.branch_id.clone(),
            selected_index: decision.selected_index,
            selected_candidate_id: decision.selected_candidate_id.clone(),
            target: decision.target.clone(),
            move_kind: decision.move_kind.clone(),
            safety: decision.safety.clone(),
            command: decision.command.clone(),
            elite_prep_bp: decision.elite_prep_bp,
            first_elite: decision.first_elite.clone(),
        },
    }
}

fn campaign_decision_observations_from_journal_events_v1(
    events: &[CampaignJournalEventV1],
) -> Vec<BranchCampaignDecisionObservationV1> {
    events
        .iter()
        .filter_map(|event| {
            let portfolio = reward_portfolio_from_journal_event_v1(event)?;
            Some(BranchCampaignDecisionObservationV1 {
                round: event.round,
                parent_index: event.branch_index,
                parent_branch_id: event.branch_id.clone(),
                parent_frontier_title: event.branch_frontier_title.clone(),
                parent_act: event.act,
                parent_floor: event.floor,
                parent_choices: event.branch_choices.clone(),
                parent_commands: event.branch_commands.clone(),
                combat_budget_retry_used: event.combat_budget_retry_used,
                portfolio,
            })
        })
        .collect()
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
                result:
                    run_campaign_parent_round_once_v1(config, parent, parent_replay_start).map_err(
                        |err| {
                            format!(
                                "parent={} commands={} failed: {err}",
                                parent.branch_id,
                                render_parent_commands_for_error_v1(&parent.commands)
                            )
                        },
                    ),
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

fn render_parent_commands_for_error_v1(commands: &[String]) -> String {
    if commands.is_empty() {
        "root".to_string()
    } else {
        commands.join(" -> ")
    }
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
                &campaign_replay_commands_for_replay_start_v1(
                    &parent.commands,
                    &replay_start.suffix_commands,
                ),
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

fn campaign_replay_commands_for_replay_start_v1(
    parent_commands: &[String],
    suffix_commands: &[String],
) -> Vec<String> {
    if parent_commands
        .iter()
        .any(|command| command.starts_with("__route_decision:"))
    {
        return suffix_commands.to_vec();
    }
    campaign_replay_commands_for_path_v1(suffix_commands)
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
