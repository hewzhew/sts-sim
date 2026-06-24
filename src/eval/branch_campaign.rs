use crate::ai::strategic::{compact_branch_signature_data, BranchSignatureCompact};
use crate::content::cards::CardId;
use crate::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1,
    BRANCH_EXPERIMENT_REPLAY_ADVANCE_COMMAND,
};
use crate::eval::branch_experiment_boundary::branch_boundary_available;
use crate::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use crate::eval::campaign_journal::CampaignJournalV1;
use crate::eval::combat_lab_probe_v1::current_act_boss_preview_probe_v1;
use crate::eval::run_control::{
    apply_branch_experiment_auto_run, build_decision_surface, AutoCombatCaptureConfig,
    RunControlAutoStepOptions, RunControlCombatSegmentMode, RunControlConfig,
    RunControlHpLossLimit, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
    RunControlSession, RunControlSessionCheckpointV1,
};
use crate::state::core::EngineState;
use crate::state::rewards::{RewardCard, RewardState};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

mod branch_display;
mod discard_trace;
mod intervention;
mod lineage;
mod model;
mod parent_batch;
mod performance;
mod progress;
mod report_render;
mod retry;
mod route_evidence;
mod run_domain;
mod scheduler;
mod selection_key;
mod state_graph;
mod strategic_signals;
mod summary;
use branch_display::{compact_campaign_choice_label_metadata_v1, render_choice_path};
#[cfg(test)]
use intervention::campaign_strategy_next_step_v1;
use intervention::{
    abandoned_branches_intervention_request_v1, campaign_strategy_requests_are_fatal_v1,
    leading_abandoned_combat_intervention_request_v1, merge_campaign_strategy_request_queue_v1,
    merge_campaign_strategy_requests_v1, prune_resolved_campaign_strategy_requests_v1,
};
pub use model::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignBranchV1,
    BranchCampaignCheckpointActiveCombatRecordV1, BranchCampaignCheckpointCombatTrajectoryRecordV1,
    BranchCampaignCheckpointComponentRecordV1,
    BranchCampaignCheckpointRunStateEmittedEventsRecordV1,
    BranchCampaignCheckpointRunStateMapGraphRecordV1, BranchCampaignCheckpointRunStateMapRecordV1,
    BranchCampaignCheckpointRunStateMasterDeckRecordV1,
    BranchCampaignCheckpointRunStatePotionsRecordV1,
    BranchCampaignCheckpointRunStateRelicsRecordV1,
    BranchCampaignCheckpointRunStateScheduleRecordV1,
    BranchCampaignCheckpointRunStateScheduleRefsV1, BranchCampaignCheckpointScheduleComponentsV1,
    BranchCampaignCheckpointSessionV1, BranchCampaignCheckpointV1,
    BranchCampaignContinuationOriginV1, BranchCampaignContinuationTargetLaneV1,
    BranchCampaignDecisionObservationV1, BranchCampaignDiscardedBranchV1, BranchCampaignReportV1,
    BranchCampaignRoundSummaryV1, BranchCampaignRouteContinuationOriginV1,
    BranchCampaignRouteEvidenceExampleV1, BranchCampaignRouteEvidenceSummaryV1,
    BranchCampaignRouteFirstEliteContinuationOriginV1, BranchCampaignRoutePathContinuationOriginV1,
    BranchCampaignRunPreludeV1, BranchCampaignRunResultV1, BranchCampaignSelectionV1,
    BranchCampaignStateStoreSummaryV1, BranchCampaignStrategyRequestV1,
};
use parent_batch::run_campaign_parent_batch_v1;
#[cfg(test)]
use parent_batch::{
    campaign_parent_replay_error_is_retryable_v1, campaign_retry_timing_for_parent_v1,
};
pub use performance::{
    BranchCampaignCombatPerformanceExampleV1, BranchCampaignCombatPerformanceSummaryV1,
};
pub use progress::{
    render_branch_campaign_progress_event_v1, render_branch_campaign_progress_event_with_detail_v1,
    BranchCampaignProgressDetailV1, BranchCampaignProgressEventV1,
    BranchCampaignReplayStartSourceV1,
};
use report_render::boss_approach_floor_v1;
pub use report_render::{
    render_branch_campaign_compact_v1, render_branch_campaign_compact_with_detail_v1,
    BranchCampaignReportDetailV1,
};
#[cfg(test)]
use retry::{
    branch_report_needs_combat_budget_retry_v1, campaign_parent_should_retry_combat_budget_now_v1,
    try_consume_branch_report_act_boss_gate_retry_v1, BOSS_GATE_RETRY_ATTEMPTS_PER_GATE,
};
use retry::{
    campaign_round_should_retry_combat_budget_on_stall_v1, combat_retry_campaign_config_v1,
    BranchCampaignCombatRetryLedgerStateV1,
};
pub use retry::{
    BranchCampaignCombatRetryLedgerEntryV1, BranchCampaignCombatRetryLedgerV1,
    BranchCampaignCombatRetryPolicyV1,
};
use route_evidence::merge_campaign_route_evidence_summary_v1;
pub use run_domain::{
    branch_campaign_ascension_domain_label_v1, branch_campaign_ascension_domain_role_v1,
    branch_campaign_run_domain_v1, BranchCampaignRunDomainV1,
};
pub use scheduler::select_campaign_branches_v1;
use scheduler::{
    append_discarded_examples_v1, reschedule_campaign_existing_workset_v1,
    schedule_campaign_workset_for_config_v1,
};
use selection_key::campaign_branch_retention_key_v1;
use state_graph::{BranchStateSessionRetentionPolicyV1, BranchStateStoreV1};
use strategic_signals::campaign_strategic_signals_from_groups_v1;
pub use strategic_signals::{
    BranchCampaignStrategicSignalGroupV1, BranchCampaignStrategicSignalsV1,
};
use summary::{
    campaign_refresh_all_branch_summaries_from_state_store_v1,
    campaign_refresh_branch_summary_from_session_v1, campaign_summary_from_report_branch_v1,
};

pub const BRANCH_CAMPAIGN_SCHEMA_NAME: &str = "BranchCampaignV1";
pub const BRANCH_CAMPAIGN_SCHEMA_VERSION: u32 = 1;
pub const BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME: &str = "BranchCampaignCheckpointV2";
pub const BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION: u32 = 2;
const UNSPENT_GOLD_PRESSURE_THRESHOLD: i32 = 300;
const COMBAT_LAB_CAMPAIGN_BOSS_PROBE_MAX_NODES: usize = 50_000;
const COMBAT_LAB_CAMPAIGN_BOSS_PROBE_MAX_WALL_MS: u64 = 300;
#[derive(Clone, Debug, PartialEq)]
pub struct BranchCampaignConfigV1 {
    pub seed: u64,
    pub ascension_level: u8,
    pub player_class: &'static str,
    pub final_act: bool,
    pub max_rounds: usize,
    pub round_depth: usize,
    pub max_active: usize,
    pub max_frozen: usize,
    pub max_branches_per_active: usize,
    pub boss_relic_axis_isolation: bool,
    pub retention_budget_profile: BranchRetentionBudgetProfileV1,
    pub max_reward_options_per_branch: Option<usize>,
    pub max_campfire_options_per_branch: usize,
    pub auto_max_operations: usize,
    pub experiment_wall_ms: Option<u64>,
    pub search_max_nodes: Option<usize>,
    pub search_wall_ms: Option<u64>,
    pub search_max_hp_loss: Option<RunControlHpLossLimit>,
    pub search_options: RunControlSearchCombatOptions,
    pub auto_capture: AutoCombatCaptureConfig,
    pub combat_retry_policy: BranchCampaignCombatRetryPolicyV1,
    pub combat_retry_wall_ms: Option<u64>,
    pub include_event_reward_skip: bool,
    pub min_acceptable_victory_hp_percent: u8,
    pub prefix_commands: Vec<String>,
}

impl Default for BranchCampaignConfigV1 {
    fn default() -> Self {
        Self {
            seed: 1,
            ascension_level: 0,
            player_class: "Ironclad",
            final_act: false,
            max_rounds: 8,
            round_depth: 1,
            max_active: 8,
            max_frozen: 32,
            max_branches_per_active: 12,
            boss_relic_axis_isolation: false,
            retention_budget_profile: BranchRetentionBudgetProfileV1::Package,
            max_reward_options_per_branch: Some(2),
            max_campfire_options_per_branch: 3,
            auto_max_operations: 128,
            experiment_wall_ms: Some(10_000),
            search_max_nodes: Some(50_000),
            search_wall_ms: Some(200),
            search_max_hp_loss: None,
            search_options: RunControlSearchCombatOptions {
                segment_mode: Some(RunControlCombatSegmentMode::NonBossTurnBoundary),
                ..RunControlSearchCombatOptions::default()
            },
            auto_capture: AutoCombatCaptureConfig::default(),
            combat_retry_policy: BranchCampaignCombatRetryPolicyV1::OnStall,
            combat_retry_wall_ms: None,
            include_event_reward_skip: false,
            min_acceptable_victory_hp_percent: 20,
            prefix_commands: Vec::new(),
        }
    }
}

struct BranchCampaignRunStateV1 {
    rounds_completed: usize,
    scheduled: Vec<BranchCampaignBranchV1>,
    parked: Vec<BranchCampaignBranchV1>,
    victories: Vec<BranchCampaignBranchV1>,
    dead: Vec<BranchCampaignBranchV1>,
    abandoned: Vec<BranchCampaignBranchV1>,
    stuck: Vec<BranchCampaignBranchV1>,
    discarded_count: usize,
    discarded_examples: Vec<String>,
    discarded_branches: Vec<BranchCampaignDiscardedBranchV1>,
    strategy_requests: Vec<BranchCampaignStrategyRequestV1>,
    route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1,
    rounds: Vec<BranchCampaignRoundSummaryV1>,
    journal: CampaignJournalV1,
    state_store: BranchStateStoreV1,
    decision_parent_anchor_commands: BTreeSet<Vec<String>>,
}

pub fn run_branch_campaign_v1(
    config: &BranchCampaignConfigV1,
) -> Result<BranchCampaignReportV1, String> {
    Ok(run_branch_campaign_with_checkpoint_v1(config)?.report)
}

pub fn run_branch_campaign_with_progress_v1<F>(
    config: &BranchCampaignConfigV1,
    progress: F,
) -> Result<BranchCampaignReportV1, String>
where
    F: FnMut(BranchCampaignProgressEventV1),
{
    Ok(run_branch_campaign_with_checkpoint_and_progress_v1(config, progress)?.report)
}

pub fn run_branch_campaign_with_checkpoint_v1(
    config: &BranchCampaignConfigV1,
) -> Result<BranchCampaignRunResultV1, String> {
    run_branch_campaign_with_checkpoint_and_progress_v1(config, |_| {})
}

pub fn run_branch_campaign_with_checkpoint_and_progress_v1<F>(
    config: &BranchCampaignConfigV1,
    progress: F,
) -> Result<BranchCampaignRunResultV1, String>
where
    F: FnMut(BranchCampaignProgressEventV1),
{
    run_branch_campaign_from_state_with_progress_v1(config, root_campaign_state_v1(), progress)
}

pub fn run_branch_campaign_ancestor_replay_self_check_v1(
) -> Result<BranchCampaignStateStoreSummaryV1, String> {
    let mut parent_session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    parent_session.engine_state = EngineState::RewardScreen(reward);

    let parent_commands = Vec::<String>::new();
    let child_commands = vec!["rp 0".to_string()];
    let mut parent_branch = root_campaign_branch_v1();
    parent_branch.branch_id = "ancestor-anchor".to_string();
    parent_branch.commands = parent_commands.clone();
    let mut child_branch = root_campaign_branch_v1();
    child_branch.branch_id = "ancestor-child".to_string();
    child_branch.commands = child_commands.clone();

    let mut state_store = BranchStateStoreV1::new();
    state_store.insert_session(parent_commands.clone(), parent_session);
    state_store.insert_child_session(
        &parent_commands,
        child_commands,
        RunControlSession::new(RunControlConfig::default()),
    );
    state_store.retain_for_branches_with_session_policy(
        &[parent_branch],
        &[child_branch.clone()],
        &[],
        &[],
        BranchStateSessionRetentionPolicyV1 {
            max_frozen_exact_sessions: 0,
            max_stuck_exact_sessions: 0,
            max_abandoned_exact_sessions: 0,
            max_anchor_exact_sessions: 0,
            max_suffix_commands_without_session: usize::MAX,
        },
    );

    let config = BranchCampaignConfigV1 {
        round_depth: 0,
        max_branches_per_active: 1,
        experiment_wall_ms: Some(1_000),
        search_wall_ms: Some(10),
        search_max_nodes: Some(100),
        ..BranchCampaignConfigV1::default()
    };
    let mut retry_ledger = BranchCampaignCombatRetryLedgerStateV1::default();
    let batch = run_campaign_parent_batch_v1(
        &config,
        &[child_branch],
        &mut state_store,
        &mut retry_ledger,
        1,
        false,
        &mut |_| {},
    )?;
    if batch.candidates.is_empty() {
        return Err("ancestor replay self-check produced no candidate branches".to_string());
    }

    let summary = state_store.summary();
    if summary.replay_exact_hits != 0
        || summary.replay_ancestor_hits != 1
        || summary.replay_misses != 0
        || summary.replay_suffix_commands_sum != 1
        || summary.replay_suffix_commands_max != 1
    {
        return Err(format!(
            "ancestor replay self-check counters mismatch: exact={} ancestor={} miss={} suffix_sum={} suffix_max={}",
            summary.replay_exact_hits,
            summary.replay_ancestor_hits,
            summary.replay_misses,
            summary.replay_suffix_commands_sum,
            summary.replay_suffix_commands_max
        ));
    }
    Ok(summary)
}

pub fn run_branch_campaign_from_report_v1(
    config: &BranchCampaignConfigV1,
    previous: &BranchCampaignReportV1,
) -> Result<BranchCampaignReportV1, String> {
    Ok(run_branch_campaign_from_report_with_checkpoint_v1(config, previous, None)?.report)
}

pub fn run_branch_campaign_from_report_with_progress_v1<F>(
    config: &BranchCampaignConfigV1,
    previous: &BranchCampaignReportV1,
    progress: F,
) -> Result<BranchCampaignReportV1, String>
where
    F: FnMut(BranchCampaignProgressEventV1),
{
    Ok(
        run_branch_campaign_from_report_with_checkpoint_and_progress_v1(
            config, previous, None, progress,
        )?
        .report,
    )
}

pub fn run_branch_campaign_from_report_with_checkpoint_v1(
    config: &BranchCampaignConfigV1,
    previous: &BranchCampaignReportV1,
    checkpoint: Option<&BranchCampaignCheckpointV1>,
) -> Result<BranchCampaignRunResultV1, String> {
    run_branch_campaign_from_report_with_checkpoint_and_progress_v1(
        config,
        previous,
        checkpoint,
        |_| {},
    )
}

pub fn run_branch_campaign_from_report_with_checkpoint_and_progress_v1<F>(
    config: &BranchCampaignConfigV1,
    previous: &BranchCampaignReportV1,
    checkpoint: Option<&BranchCampaignCheckpointV1>,
    progress: F,
) -> Result<BranchCampaignRunResultV1, String>
where
    F: FnMut(BranchCampaignProgressEventV1),
{
    validate_campaign_resume_report_v1(config, previous)?;
    let effective_config = campaign_config_with_report_prelude_v1(config, previous);
    let mut state = match checkpoint {
        Some(checkpoint) => campaign_state_from_report_and_checkpoint_v1(previous, checkpoint)?,
        None => campaign_state_from_report_v1(previous),
    };
    if checkpoint.is_some() {
        let recovered = recover_auto_advanceable_stuck_branches_v1(
            &mut state.scheduled,
            &mut state.parked,
            &mut state.stuck,
            &mut state.state_store,
            config.max_active,
            config.max_frozen,
        );
        if recovered > 0 {
            state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
                state.strategy_requests,
                &state.scheduled,
                &state.parked,
                &state.stuck,
                &state.abandoned,
            );
        }
    }
    run_branch_campaign_from_state_with_progress_v1(&effective_config, state, progress)
}

fn run_branch_campaign_from_state_with_progress_v1<F>(
    config: &BranchCampaignConfigV1,
    mut state: BranchCampaignRunStateV1,
    mut progress: F,
) -> Result<BranchCampaignRunResultV1, String>
where
    F: FnMut(BranchCampaignProgressEventV1),
{
    let round_offset = state.rounds_completed;
    let displayed_max_rounds = round_offset.saturating_add(config.max_rounds);
    progress(BranchCampaignProgressEventV1::CampaignStarted {
        seed: config.seed,
        max_rounds: displayed_max_rounds,
        round_depth: config.round_depth,
        max_scheduled: config.max_active,
        max_parked: config.max_frozen,
    });

    let mut stop_reason = "max_rounds".to_string();

    for local_round in 0..config.max_rounds {
        let recovered = recover_auto_advanceable_stuck_branches_v1(
            &mut state.scheduled,
            &mut state.parked,
            &mut state.stuck,
            &mut state.state_store,
            config.max_active,
            config.max_frozen,
        );
        if recovered > 0 {
            state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
                state.strategy_requests,
                &state.scheduled,
                &state.parked,
                &state.stuck,
                &state.abandoned,
            );
        }
        let existing_schedule = reschedule_campaign_existing_workset_v1(
            std::mem::take(&mut state.scheduled),
            std::mem::take(&mut state.parked),
            config,
        );
        let _existing_schedule_counts =
            apply_campaign_schedule_selection_to_state_v1(&mut state, existing_schedule);
        if state.scheduled.is_empty()
            && campaign_should_stop_after_victory_v1(
                config,
                &state.victories,
                &state.scheduled,
                &state.parked,
            )
        {
            stop_reason = "victory_found".to_string();
            break;
        }
        if state.scheduled.is_empty() {
            stop_reason = "no_scheduled_branch".to_string();
            break;
        }
        let round_number = round_offset.saturating_add(local_round).saturating_add(1);
        progress(BranchCampaignProgressEventV1::RoundStarted {
            round: round_number,
            max_rounds: displayed_max_rounds,
            scheduled_branches: state.scheduled.len(),
            parked_branches: state.parked.len(),
        });
        let round_started_at = Instant::now();
        let parents = std::mem::take(&mut state.scheduled);
        let started_scheduled = parents.len();
        let mut batch = run_campaign_parent_batch_v1(
            config,
            &parents,
            &mut state.state_store,
            &mut state.combat_retry_ledger,
            round_number,
            false,
            &mut progress,
        )?;
        let mut parent_elapsed_wall_ms_sum = batch.parent_elapsed_wall_ms_sum;
        let mut parent_elapsed_wall_ms_max = batch.parent_elapsed_wall_ms_max;
        let mut combat_retry_elapsed_wall_ms_sum = batch.combat_retry_elapsed_wall_ms_sum;
        let mut combat_retry_elapsed_wall_ms_max = batch.combat_retry_elapsed_wall_ms_max;
        let mut produced_branches = batch.candidates.len();
        let parked_before_schedule = state.parked.len();
        let mut selected = schedule_campaign_workset_for_config_v1(
            batch.candidates.clone(),
            state.parked.clone(),
            config,
        );
        if campaign_round_should_retry_combat_budget_on_stall_v1(
            config,
            &selected,
            state.parked.len(),
        ) {
            let retry_allowed = state
                .combat_retry_ledger
                .try_consume_selection_boss_gate_retry_v1(&selected);
            if retry_allowed {
                if let Some(retry_config) = combat_retry_campaign_config_v1(config) {
                    batch = run_campaign_parent_batch_v1(
                        &retry_config,
                        &parents,
                        &mut state.state_store,
                        &mut state.combat_retry_ledger,
                        round_number,
                        true,
                        &mut progress,
                    )?;
                    parent_elapsed_wall_ms_sum =
                        parent_elapsed_wall_ms_sum.saturating_add(batch.parent_elapsed_wall_ms_sum);
                    parent_elapsed_wall_ms_max =
                        parent_elapsed_wall_ms_max.max(batch.parent_elapsed_wall_ms_max);
                    combat_retry_elapsed_wall_ms_sum = combat_retry_elapsed_wall_ms_sum
                        .saturating_add(batch.combat_retry_elapsed_wall_ms_sum);
                    combat_retry_elapsed_wall_ms_max = combat_retry_elapsed_wall_ms_max
                        .max(batch.combat_retry_elapsed_wall_ms_max);
                    produced_branches = batch.candidates.len();
                    selected = schedule_campaign_workset_for_config_v1(
                        batch.candidates.clone(),
                        state.parked.clone(),
                        config,
                    );
                }
            }
        }
        state
            .decision_parent_anchor_commands
            .extend(batch.decision_parent_anchor_commands);
        state.strategy_requests = merge_campaign_strategy_request_queue_v1(
            state.strategy_requests,
            merge_campaign_strategy_requests_v1(batch.strategy_requests.clone()),
        );
        merge_campaign_route_evidence_summary_v1(&mut state.route_evidence, batch.route_evidence);
        let schedule_counts = apply_campaign_schedule_selection_to_state_v1(&mut state, selected);
        let parked_added = state.parked.len().saturating_sub(parked_before_schedule);
        let dead_added = schedule_counts.dead_added;
        let abandoned_added = schedule_counts.abandoned_added;
        let victories_added = schedule_counts.victories_added;
        let stuck_added = schedule_counts.stuck_added;
        recover_auto_advanceable_stuck_branches_v1(
            &mut state.scheduled,
            &mut state.parked,
            &mut state.stuck,
            &mut state.state_store,
            config.max_active,
            config.max_frozen,
        );
        state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
            state.strategy_requests,
            &state.scheduled,
            &state.parked,
            &state.stuck,
            &state.abandoned,
        );
        state
            .state_store
            .retain_for_branches_with_session_policy_and_anchors(
                &state.scheduled,
                &state.parked,
                &state.abandoned,
                &state.stuck,
                &state.decision_parent_anchor_commands,
                campaign_state_session_retention_policy_v1(config),
            );
        let leading_abandoned_request = if state.scheduled.is_empty() && state.victories.is_empty()
        {
            leading_abandoned_combat_intervention_request_v1(&state.parked, &state.abandoned)
        } else {
            None
        };
        if let Some(request) = leading_abandoned_request {
            state.strategy_requests =
                merge_campaign_strategy_request_queue_v1(state.strategy_requests, vec![request]);
        }
        let round_summary = BranchCampaignRoundSummaryV1 {
            round: round_number,
            started_scheduled,
            produced_branches,
            scheduled_after: state.scheduled.len(),
            parked_added,
            dead_added,
            abandoned_added,
            victories_added,
            stuck_added,
            discarded_added: schedule_counts.discarded_added,
            explored_branch_points: batch.explored_branch_points,
            wall_limit_hit: batch.wall_limit_hit,
            branch_limit_hit: batch.branch_limit_hit,
            combat_budget_retries: batch.combat_budget_retries,
            elapsed_wall_ms: campaign_elapsed_ms_u64(round_started_at),
            parent_elapsed_wall_ms_sum,
            parent_elapsed_wall_ms_max,
            combat_retry_elapsed_wall_ms_sum,
            combat_retry_elapsed_wall_ms_max,
            combat_performance: batch.combat_performance,
            decision_observations: batch.decision_observations,
        };
        state.journal.extend(batch.journal_events);
        progress(BranchCampaignProgressEventV1::RoundFinished {
            round: round_number,
            started_scheduled,
            produced_branches,
            scheduled_after: state.scheduled.len(),
            parked_added,
            strategy_requests: state.strategy_requests.len(),
        });
        state.rounds.push(round_summary);
        state.rounds_completed = state.rounds_completed.saturating_add(1);

        if campaign_should_stop_after_victory_v1(
            config,
            &state.victories,
            &state.scheduled,
            &state.parked,
        ) {
            stop_reason = "victory_found".to_string();
            break;
        }
        if state.scheduled.is_empty()
            && state.parked.is_empty()
            && !state.abandoned.is_empty()
            && state.strategy_requests.is_empty()
        {
            if let Some(request) = abandoned_branches_intervention_request_v1(&state.abandoned) {
                state.strategy_requests = vec![request];
                stop_reason = "needs_intervention".to_string();
                break;
            }
        }
        if campaign_strategy_requests_are_fatal_v1(
            &state.scheduled,
            &state.parked,
            &state.strategy_requests,
        ) {
            stop_reason = "needs_intervention".to_string();
            break;
        }
        if state.scheduled.is_empty() && state.parked.is_empty() && !state.stuck.is_empty() {
            stop_reason = "stuck".to_string();
            break;
        }
        if produced_branches == 0 {
            stop_reason = "no_progress".to_string();
            break;
        }
    }

    progress(BranchCampaignProgressEventV1::CampaignFinished {
        stop_reason: stop_reason.clone(),
        scheduled: state.scheduled.len(),
        parked: state.parked.len(),
        victories: state.victories.len(),
        stuck: state.stuck.len(),
    });

    campaign_refresh_all_branch_summaries_from_state_store_v1(&mut state);

    let strategic_signals = campaign_strategic_signals_from_groups_v1(
        &state.scheduled,
        &state.parked,
        &state.victories,
        &state.abandoned,
        &state.stuck,
    );
    let checkpoint = campaign_checkpoint_from_state_v1(config, &state);
    let mut journal = state.journal;
    journal.compact_for_campaign_artifact_v1();
    let report = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: config.seed,
        run_domain: branch_campaign_run_domain_v1(config.ascension_level, config.player_class),
        run_prelude: branch_campaign_run_prelude_v1(config),
        rounds_completed: state.rounds_completed,
        stop_reason,
        active: state.scheduled,
        frozen: state.parked,
        victories: state.victories,
        dead: state.dead,
        abandoned: state.abandoned,
        stuck: state.stuck,
        discarded_count: state.discarded_count,
        discarded_examples: state.discarded_examples,
        discarded_branches: state.discarded_branches,
        strategy_requests: state.strategy_requests,
        route_evidence: state.route_evidence,
        combat_retry_ledger: state.combat_retry_ledger.to_report_v1(),
        strategic_signals,
        state_store: state.state_store.summary(),
        journal,
        rounds: state.rounds,
    };
    Ok(BranchCampaignRunResultV1 { report, checkpoint })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CampaignScheduleApplyCountsV1 {
    dead_added: usize,
    abandoned_added: usize,
    victories_added: usize,
    stuck_added: usize,
    discarded_added: usize,
}

fn apply_campaign_schedule_selection_to_state_v1(
    state: &mut BranchCampaignRunStateV1,
    selection: BranchCampaignSelectionV1,
) -> CampaignScheduleApplyCountsV1 {
    let counts = CampaignScheduleApplyCountsV1 {
        dead_added: selection.dead.len(),
        abandoned_added: selection.abandoned.len(),
        victories_added: selection.victories.len(),
        stuck_added: selection.stuck.len(),
        discarded_added: selection.discarded_count,
    };
    state.scheduled = selection.scheduled;
    state.parked = selection.parked;
    state.victories.extend(selection.victories);
    state.dead.extend(selection.dead);
    state.abandoned.extend(selection.abandoned);
    state.stuck.extend(selection.stuck);
    state.discarded_count = state
        .discarded_count
        .saturating_add(selection.discarded_count);
    append_discarded_examples_v1(&mut state.discarded_examples, selection.discarded_examples);
    state
        .discarded_branches
        .extend(selection.discarded_branches);
    counts
}

fn root_campaign_state_v1() -> BranchCampaignRunStateV1 {
    BranchCampaignRunStateV1 {
        rounds_completed: 0,
        scheduled: vec![root_campaign_branch_v1()],
        parked: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::default(),
        rounds: Vec::new(),
        journal: CampaignJournalV1::new(),
        state_store: BranchStateStoreV1::new(),
        decision_parent_anchor_commands: BTreeSet::new(),
    }
}

fn campaign_state_from_report_v1(report: &BranchCampaignReportV1) -> BranchCampaignRunStateV1 {
    BranchCampaignRunStateV1 {
        rounds_completed: report.rounds_completed,
        scheduled: report.active.clone(),
        parked: report.frozen.clone(),
        victories: report.victories.clone(),
        dead: report.dead.clone(),
        abandoned: report.abandoned.clone(),
        stuck: report.stuck.clone(),
        discarded_count: report.discarded_count,
        discarded_examples: report.discarded_examples.clone(),
        discarded_branches: report.discarded_branches.clone(),
        strategy_requests: report.strategy_requests.clone(),
        route_evidence: report.route_evidence.clone(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::from_report_v1(
            &report.combat_retry_ledger,
        ),
        rounds: report.rounds.clone(),
        journal: report.journal.clone(),
        state_store: BranchStateStoreV1::new(),
        decision_parent_anchor_commands: BTreeSet::new(),
    }
}

fn campaign_state_from_report_and_checkpoint_v1(
    report: &BranchCampaignReportV1,
    checkpoint: &BranchCampaignCheckpointV1,
) -> Result<BranchCampaignRunStateV1, String> {
    validate_campaign_resume_checkpoint_v1(report, checkpoint)?;
    let mut state = campaign_state_from_report_v1(report);
    state
        .state_store
        .restore_checkpoint_nodes(&checkpoint.nodes)?;
    let keep = state
        .scheduled
        .iter()
        .chain(state.parked.iter())
        .chain(state.abandoned.iter())
        .chain(state.stuck.iter())
        .map(|branch| branch.commands.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for entry in &checkpoint.sessions {
        let entry_commands = checkpoint.session_commands_v1(entry)?;
        state.state_store.insert_session(
            entry_commands.clone(),
            checkpoint
                .hydrated_session_checkpoint_v1(entry)?
                .into_session()
                .map_err(|err| format!("failed to restore campaign checkpoint session: {err}"))?,
        );
        if !keep.contains(&entry_commands) {
            state.decision_parent_anchor_commands.insert(entry_commands);
        }
    }
    state.decision_parent_anchor_commands.extend(
        checkpoint
            .resolved_decision_parent_anchor_commands_v1()?
            .into_iter(),
    );
    state
        .stuck
        .retain(|branch| state.state_store.contains_commands(&branch.commands));
    state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
        state.strategy_requests,
        &state.scheduled,
        &state.parked,
        &state.stuck,
        &state.abandoned,
    );
    Ok(state)
}

fn campaign_checkpoint_from_state_v1(
    config: &BranchCampaignConfigV1,
    state: &BranchCampaignRunStateV1,
) -> BranchCampaignCheckpointV1 {
    let mut sessions = Vec::new();
    let mut run_state_map_graphs = Vec::<BranchCampaignCheckpointRunStateMapGraphRecordV1>::new();
    let mut run_state_map_graph_indexes = BTreeMap::<String, usize>::new();
    let mut run_state_maps = Vec::<BranchCampaignCheckpointRunStateMapRecordV1>::new();
    let mut run_state_map_indexes = BTreeMap::<String, usize>::new();
    let mut run_state_master_decks =
        Vec::<BranchCampaignCheckpointRunStateMasterDeckRecordV1>::new();
    let mut run_state_master_deck_indexes = BTreeMap::<String, usize>::new();
    let mut run_state_relics = Vec::<BranchCampaignCheckpointRunStateRelicsRecordV1>::new();
    let mut run_state_relic_indexes = BTreeMap::<String, usize>::new();
    let mut run_state_potions = Vec::<BranchCampaignCheckpointRunStatePotionsRecordV1>::new();
    let mut run_state_potion_indexes = BTreeMap::<String, usize>::new();
    let mut run_state_schedules = Vec::<BranchCampaignCheckpointRunStateScheduleRecordV1>::new();
    let mut run_state_schedule_indexes = BTreeMap::<String, usize>::new();
    let mut run_state_schedule_components = BranchCampaignCheckpointScheduleComponentsV1::default();
    let mut run_state_emitted_events =
        Vec::<BranchCampaignCheckpointRunStateEmittedEventsRecordV1>::new();
    let mut run_state_emitted_events_indexes = BTreeMap::<String, usize>::new();
    let mut combat_automation_trajectories =
        Vec::<BranchCampaignCheckpointCombatTrajectoryRecordV1>::new();
    let mut combat_automation_trajectory_indexes = BTreeMap::<String, usize>::new();
    let mut active_combats = Vec::<BranchCampaignCheckpointActiveCombatRecordV1>::new();
    let mut active_combat_indexes = BTreeMap::<String, usize>::new();
    let mut exported_commands = BTreeSet::new();
    for branch in state
        .scheduled
        .iter()
        .chain(state.parked.iter())
        .chain(state.abandoned.iter())
        .chain(state.stuck.iter())
    {
        if !exported_commands.insert(branch.commands.clone()) {
            continue;
        }
        if let Some(session) = state.state_store.get_session(&branch.commands) {
            let externalized = campaign_checkpoint_session_with_external_refs_v1(
                &branch.commands,
                session,
                &mut run_state_map_graphs,
                &mut run_state_map_graph_indexes,
                &mut run_state_maps,
                &mut run_state_map_indexes,
                &mut run_state_master_decks,
                &mut run_state_master_deck_indexes,
                &mut run_state_relics,
                &mut run_state_relic_indexes,
                &mut run_state_potions,
                &mut run_state_potion_indexes,
                &mut run_state_schedules,
                &mut run_state_schedule_indexes,
                &mut run_state_schedule_components,
                &mut run_state_emitted_events,
                &mut run_state_emitted_events_indexes,
                &mut combat_automation_trajectories,
                &mut combat_automation_trajectory_indexes,
                &mut active_combats,
                &mut active_combat_indexes,
            );
            let node_id = state
                .state_store
                .node_id_for_commands(&branch.commands)
                .map(|id| id.as_usize());
            sessions.push(BranchCampaignCheckpointSessionV1 {
                node_id,
                commands: if node_id.is_some() {
                    Vec::new()
                } else {
                    branch.commands.clone()
                },
                run_state_map_id: externalized.run_state_map_id,
                run_state_master_deck_id: externalized.run_state_master_deck_id,
                run_state_relics_id: externalized.run_state_relics_id,
                run_state_potions_id: externalized.run_state_potions_id,
                run_state_schedule_id: externalized.run_state_schedule_id,
                run_state_emitted_events_id: externalized.run_state_emitted_events_id,
                active_combat_id: externalized.active_combat_id,
                session: externalized.session,
            });
        }
    }
    for commands in &state.decision_parent_anchor_commands {
        if !exported_commands.insert(commands.clone()) {
            continue;
        }
        if let Some(session) = state.state_store.get_session(commands) {
            let externalized = campaign_checkpoint_session_with_external_refs_v1(
                commands,
                session,
                &mut run_state_map_graphs,
                &mut run_state_map_graph_indexes,
                &mut run_state_maps,
                &mut run_state_map_indexes,
                &mut run_state_master_decks,
                &mut run_state_master_deck_indexes,
                &mut run_state_relics,
                &mut run_state_relic_indexes,
                &mut run_state_potions,
                &mut run_state_potion_indexes,
                &mut run_state_schedules,
                &mut run_state_schedule_indexes,
                &mut run_state_schedule_components,
                &mut run_state_emitted_events,
                &mut run_state_emitted_events_indexes,
                &mut combat_automation_trajectories,
                &mut combat_automation_trajectory_indexes,
                &mut active_combats,
                &mut active_combat_indexes,
            );
            let node_id = state
                .state_store
                .node_id_for_commands(commands)
                .map(|id| id.as_usize());
            sessions.push(BranchCampaignCheckpointSessionV1 {
                node_id,
                commands: if node_id.is_some() {
                    Vec::new()
                } else {
                    commands.clone()
                },
                run_state_map_id: externalized.run_state_map_id,
                run_state_master_deck_id: externalized.run_state_master_deck_id,
                run_state_relics_id: externalized.run_state_relics_id,
                run_state_potions_id: externalized.run_state_potions_id,
                run_state_schedule_id: externalized.run_state_schedule_id,
                run_state_emitted_events_id: externalized.run_state_emitted_events_id,
                active_combat_id: externalized.active_combat_id,
                session: externalized.session,
            });
        }
    }
    BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: config.seed,
        run_domain: branch_campaign_run_domain_v1(config.ascension_level, config.player_class),
        run_prelude: branch_campaign_run_prelude_v1(config),
        rounds_completed: state.rounds_completed,
        nodes: state.state_store.checkpoint_nodes(),
        decision_parent_anchor_commands: state
            .decision_parent_anchor_commands
            .iter()
            .filter(|commands| state.state_store.node_id_for_commands(commands).is_none())
            .cloned()
            .collect(),
        decision_parent_anchor_node_ids: state
            .decision_parent_anchor_commands
            .iter()
            .filter_map(|commands| {
                state
                    .state_store
                    .node_id_for_commands(commands)
                    .map(|id| id.as_usize())
            })
            .collect(),
        run_state_map_graphs,
        run_state_maps,
        run_state_master_decks,
        run_state_relics,
        run_state_potions,
        run_state_schedules,
        run_state_schedule_components,
        run_state_emitted_events,
        combat_automation_trajectories,
        active_combats,
        sessions,
    }
}

struct CampaignCheckpointExternalizedSessionV1 {
    session: RunControlSessionCheckpointV1,
    run_state_map_id: Option<String>,
    run_state_master_deck_id: Option<String>,
    run_state_relics_id: Option<String>,
    run_state_potions_id: Option<String>,
    run_state_schedule_id: Option<String>,
    run_state_emitted_events_id: Option<String>,
    active_combat_id: Option<String>,
}

fn campaign_checkpoint_session_with_external_refs_v1(
    _commands: &[String],
    session: &RunControlSession,
    run_state_map_graphs: &mut Vec<BranchCampaignCheckpointRunStateMapGraphRecordV1>,
    run_state_map_graph_indexes: &mut BTreeMap<String, usize>,
    run_state_maps: &mut Vec<BranchCampaignCheckpointRunStateMapRecordV1>,
    run_state_map_indexes: &mut BTreeMap<String, usize>,
    run_state_master_decks: &mut Vec<BranchCampaignCheckpointRunStateMasterDeckRecordV1>,
    run_state_master_deck_indexes: &mut BTreeMap<String, usize>,
    run_state_relics: &mut Vec<BranchCampaignCheckpointRunStateRelicsRecordV1>,
    run_state_relic_indexes: &mut BTreeMap<String, usize>,
    run_state_potions: &mut Vec<BranchCampaignCheckpointRunStatePotionsRecordV1>,
    run_state_potion_indexes: &mut BTreeMap<String, usize>,
    run_state_schedules: &mut Vec<BranchCampaignCheckpointRunStateScheduleRecordV1>,
    run_state_schedule_indexes: &mut BTreeMap<String, usize>,
    run_state_schedule_components: &mut BranchCampaignCheckpointScheduleComponentsV1,
    _run_state_emitted_events: &mut Vec<BranchCampaignCheckpointRunStateEmittedEventsRecordV1>,
    _run_state_emitted_events_indexes: &mut BTreeMap<String, usize>,
    _combat_automation_trajectories: &mut Vec<BranchCampaignCheckpointCombatTrajectoryRecordV1>,
    _combat_automation_trajectory_indexes: &mut BTreeMap<String, usize>,
    active_combats: &mut Vec<BranchCampaignCheckpointActiveCombatRecordV1>,
    active_combat_indexes: &mut BTreeMap<String, usize>,
) -> CampaignCheckpointExternalizedSessionV1 {
    let mut checkpoint = RunControlSessionCheckpointV1::from_session(session);
    let mut map = checkpoint.take_run_state_map_for_external_ref();
    let map_key = campaign_checkpoint_run_state_map_key_v1(&map);
    let map_index = if let Some(index) = run_state_map_indexes.get(&map_key).copied() {
        index
    } else {
        let index = run_state_maps.len();
        run_state_map_indexes.insert(map_key, index);
        let graph = std::mem::take(&mut map.graph);
        let run_state_map_graph_id = if graph.is_empty() {
            None
        } else {
            let graph_key = campaign_checkpoint_run_state_map_graph_key_v1(&graph);
            let graph_index =
                if let Some(index) = run_state_map_graph_indexes.get(&graph_key).copied() {
                    index
                } else {
                    let index = run_state_map_graphs.len();
                    run_state_map_graph_indexes.insert(graph_key, index);
                    run_state_map_graphs.push(BranchCampaignCheckpointRunStateMapGraphRecordV1 {
                        graph_id: format!("run_state_map_graph:{index}"),
                        graph,
                    });
                    index
                };
            run_state_map_graphs
                .get(graph_index)
                .map(|record| record.graph_id.clone())
        };
        run_state_maps.push(BranchCampaignCheckpointRunStateMapRecordV1 {
            map_id: format!("run_state_map:{index}"),
            run_state_map_graph_id,
            map,
        });
        index
    };
    let run_state_map_id = run_state_maps
        .get(map_index)
        .map(|record| record.map_id.clone());

    let master_deck = checkpoint.take_run_state_master_deck_for_external_ref();
    let run_state_master_deck_id = if master_deck.is_empty() {
        None
    } else {
        let deck_key = campaign_checkpoint_run_state_master_deck_key_v1(&master_deck);
        let deck_index = if let Some(index) = run_state_master_deck_indexes.get(&deck_key).copied()
        {
            index
        } else {
            let index = run_state_master_decks.len();
            run_state_master_deck_indexes.insert(deck_key, index);
            run_state_master_decks.push(BranchCampaignCheckpointRunStateMasterDeckRecordV1 {
                deck_id: format!("run_state_master_deck:{index}"),
                master_deck,
            });
            index
        };
        run_state_master_decks
            .get(deck_index)
            .map(|record| record.deck_id.clone())
    };

    let relics = checkpoint.take_run_state_relics_for_external_ref();
    let run_state_relics_id = if relics.is_empty() {
        None
    } else {
        let relics_key = campaign_checkpoint_run_state_relics_key_v1(&relics);
        let relics_index = if let Some(index) = run_state_relic_indexes.get(&relics_key).copied() {
            index
        } else {
            let index = run_state_relics.len();
            run_state_relic_indexes.insert(relics_key, index);
            run_state_relics.push(BranchCampaignCheckpointRunStateRelicsRecordV1 {
                relics_id: format!("run_state_relics:{index}"),
                relics,
            });
            index
        };
        run_state_relics
            .get(relics_index)
            .map(|record| record.relics_id.clone())
    };

    let potions = checkpoint.take_run_state_potions_for_external_ref();
    let run_state_potions_id = if potions.is_empty() {
        None
    } else {
        let potions_key = campaign_checkpoint_run_state_potions_key_v1(&potions);
        let potions_index = if let Some(index) = run_state_potion_indexes.get(&potions_key).copied()
        {
            index
        } else {
            let index = run_state_potions.len();
            run_state_potion_indexes.insert(potions_key, index);
            run_state_potions.push(BranchCampaignCheckpointRunStatePotionsRecordV1 {
                potions_id: format!("run_state_potions:{index}"),
                potions,
            });
            index
        };
        run_state_potions
            .get(potions_index)
            .map(|record| record.potions_id.clone())
    };

    let schedule = checkpoint.take_run_state_schedule_for_external_ref();
    let schedule_key = campaign_checkpoint_run_state_schedule_key_v1(&schedule);
    let schedule_index = if let Some(index) = run_state_schedule_indexes.get(&schedule_key).copied()
    {
        index
    } else {
        let index = run_state_schedules.len();
        run_state_schedule_indexes.insert(schedule_key, index);
        let schedule_refs = campaign_checkpoint_run_state_schedule_refs_v1(
            run_state_schedule_components,
            &schedule,
        );
        run_state_schedules.push(BranchCampaignCheckpointRunStateScheduleRecordV1 {
            schedule_id: format!("run_state_schedule:{index}"),
            schedule: None,
            schedule_refs: Some(schedule_refs),
        });
        index
    };
    let run_state_schedule_id = run_state_schedules
        .get(schedule_index)
        .map(|record| record.schedule_id.clone());

    let _ = checkpoint.take_run_state_emitted_events_for_external_ref();
    let run_state_emitted_events_id = None;
    checkpoint.clear_combat_diagnostics_for_external_checkpoint();

    let active_combat = checkpoint.take_active_combat_for_external_ref();
    let active_combat_id = if let Some(active_combat) = active_combat {
        let active_combat_key = campaign_checkpoint_active_combat_key_v1(&active_combat);
        let active_combat_index =
            if let Some(index) = active_combat_indexes.get(&active_combat_key).copied() {
                index
            } else {
                let index = active_combats.len();
                active_combat_indexes.insert(active_combat_key, index);
                active_combats.push(BranchCampaignCheckpointActiveCombatRecordV1 {
                    active_combat_id: format!("active_combat:{index}"),
                    active_combat,
                });
                index
            };
        active_combats
            .get(active_combat_index)
            .map(|record| record.active_combat_id.clone())
    } else {
        None
    };

    let _ = checkpoint.take_last_combat_automation_trajectory_record();
    CampaignCheckpointExternalizedSessionV1 {
        session: checkpoint,
        run_state_map_id,
        run_state_master_deck_id,
        run_state_relics_id,
        run_state_potions_id,
        run_state_schedule_id,
        run_state_emitted_events_id,
        active_combat_id,
    }
}

fn campaign_checkpoint_run_state_map_key_v1(map: &crate::state::map::state::MapState) -> String {
    serde_json::to_string(map).unwrap_or_else(|_| format!("{map:?}"))
}

fn campaign_checkpoint_run_state_map_graph_key_v1(graph: &crate::state::map::node::Map) -> String {
    serde_json::to_string(graph).unwrap_or_else(|_| format!("{graph:?}"))
}

fn campaign_checkpoint_run_state_master_deck_key_v1(
    master_deck: &[crate::runtime::combat::CombatCard],
) -> String {
    serde_json::to_string(master_deck).unwrap_or_else(|_| format!("{master_deck:?}"))
}

fn campaign_checkpoint_run_state_relics_key_v1(
    relics: &[crate::content::relics::RelicState],
) -> String {
    serde_json::to_string(relics).unwrap_or_else(|_| format!("{relics:?}"))
}

fn campaign_checkpoint_run_state_potions_key_v1(
    potions: &[Option<crate::content::potions::Potion>],
) -> String {
    serde_json::to_string(potions).unwrap_or_else(|_| format!("{potions:?}"))
}

fn campaign_checkpoint_run_state_schedule_refs_v1(
    components: &mut BranchCampaignCheckpointScheduleComponentsV1,
    schedule: &crate::state::run::RunStateScheduleCheckpointV1,
) -> BranchCampaignCheckpointRunStateScheduleRefsV1 {
    BranchCampaignCheckpointRunStateScheduleRefsV1 {
        rng_pool: schedule.rng_pool.clone(),
        neow_rng_id: campaign_checkpoint_component_id_v1(
            &mut components.neow_rngs,
            "schedule_neow_rng",
            &schedule.neow_rng,
        ),
        event_generator_id: campaign_checkpoint_component_id_v1(
            &mut components.event_generators,
            "schedule_event_generator",
            &schedule.event_generator,
        ),
        common_relic_pool_id: campaign_checkpoint_component_id_v1(
            &mut components.common_relic_pools,
            "schedule_common_relic_pool",
            &schedule.common_relic_pool,
        ),
        uncommon_relic_pool_id: campaign_checkpoint_component_id_v1(
            &mut components.uncommon_relic_pools,
            "schedule_uncommon_relic_pool",
            &schedule.uncommon_relic_pool,
        ),
        rare_relic_pool_id: campaign_checkpoint_component_id_v1(
            &mut components.rare_relic_pools,
            "schedule_rare_relic_pool",
            &schedule.rare_relic_pool,
        ),
        shop_relic_pool_id: campaign_checkpoint_component_id_v1(
            &mut components.shop_relic_pools,
            "schedule_shop_relic_pool",
            &schedule.shop_relic_pool,
        ),
        boss_relic_pool_id: campaign_checkpoint_component_id_v1(
            &mut components.boss_relic_pools,
            "schedule_boss_relic_pool",
            &schedule.boss_relic_pool,
        ),
        monster_list_id: campaign_checkpoint_component_id_v1(
            &mut components.monster_lists,
            "schedule_monster_list",
            &schedule.monster_list,
        ),
        elite_monster_list_id: campaign_checkpoint_component_id_v1(
            &mut components.elite_monster_lists,
            "schedule_elite_monster_list",
            &schedule.elite_monster_list,
        ),
        boss_key: schedule.boss_key,
        boss_list_id: campaign_checkpoint_component_id_v1(
            &mut components.boss_lists,
            "schedule_boss_list",
            &schedule.boss_list,
        ),
    }
}

fn campaign_checkpoint_component_id_v1<T: Clone + PartialEq>(
    records: &mut Vec<BranchCampaignCheckpointComponentRecordV1<T>>,
    prefix: &str,
    value: &T,
) -> String {
    if let Some(record) = records.iter().find(|record| record.value == *value) {
        return record.component_id.clone();
    }
    let component_id = format!("{prefix}:{}", records.len());
    records.push(BranchCampaignCheckpointComponentRecordV1 {
        component_id: component_id.clone(),
        value: value.clone(),
    });
    component_id
}

fn campaign_checkpoint_run_state_schedule_key_v1(
    schedule: &crate::state::run::RunStateScheduleCheckpointV1,
) -> String {
    serde_json::to_string(schedule).unwrap_or_else(|_| format!("{schedule:?}"))
}

fn campaign_checkpoint_active_combat_key_v1(
    active_combat: &crate::state::core::ActiveCombat,
) -> String {
    serde_json::to_string(active_combat).unwrap_or_else(|_| format!("{active_combat:?}"))
}

fn branch_campaign_run_prelude_v1(config: &BranchCampaignConfigV1) -> BranchCampaignRunPreludeV1 {
    BranchCampaignRunPreludeV1 {
        replay_root_id: "campaign_root_after_prelude".to_string(),
        branch_command_coordinate: "relative_to_run_prelude".to_string(),
        prefix_commands: config.prefix_commands.clone(),
    }
}

fn campaign_config_with_report_prelude_v1(
    config: &BranchCampaignConfigV1,
    report: &BranchCampaignReportV1,
) -> BranchCampaignConfigV1 {
    if report.run_prelude.is_empty() {
        return config.clone();
    }
    let mut effective = config.clone();
    effective.prefix_commands = report.run_prelude.prefix_commands.clone();
    effective
}

fn validate_campaign_resume_checkpoint_v1(
    report: &BranchCampaignReportV1,
    checkpoint: &BranchCampaignCheckpointV1,
) -> Result<(), String> {
    if checkpoint.schema_name != BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME {
        return Err(format!(
            "campaign checkpoint schema mismatch: expected {}, found {}",
            BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME, checkpoint.schema_name
        ));
    }
    if checkpoint.schema_version != BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION {
        return Err(format!(
            "campaign checkpoint schema version mismatch: expected {}, found {}",
            BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION, checkpoint.schema_version
        ));
    }
    if checkpoint.seed != report.seed {
        return Err(format!(
            "campaign checkpoint seed mismatch: report seed {} does not match checkpoint seed {}",
            report.seed, checkpoint.seed
        ));
    }
    if checkpoint.rounds_completed != report.rounds_completed {
        return Err(format!(
            "campaign checkpoint rounds mismatch: report rounds {} does not match checkpoint rounds {}",
            report.rounds_completed, checkpoint.rounds_completed
        ));
    }
    Ok(())
}

fn validate_campaign_resume_report_v1(
    config: &BranchCampaignConfigV1,
    report: &BranchCampaignReportV1,
) -> Result<(), String> {
    if report.schema_name != BRANCH_CAMPAIGN_SCHEMA_NAME {
        return Err(format!(
            "campaign resume schema mismatch: expected {}, found {}",
            BRANCH_CAMPAIGN_SCHEMA_NAME, report.schema_name
        ));
    }
    if report.schema_version != BRANCH_CAMPAIGN_SCHEMA_VERSION {
        return Err(format!(
            "campaign resume schema version mismatch: expected {}, found {}",
            BRANCH_CAMPAIGN_SCHEMA_VERSION, report.schema_version
        ));
    }
    if report.seed != config.seed {
        return Err(format!(
            "campaign resume seed mismatch: config seed {} does not match report seed {}",
            config.seed, report.seed
        ));
    }
    Ok(())
}

fn root_campaign_branch_v1() -> BranchCampaignBranchV1 {
    BranchCampaignBranchV1 {
        branch_id: "root".to_string(),
        commands: Vec::new(),
        choice_labels: Vec::new(),
        summary: None,
        strategic_summary: BranchSignatureCompact::default(),
        frontier_title: "start".to_string(),
        status: BranchCampaignBranchStatusV1::Scheduled,
        stop_reason: "initial".to_string(),
        continuation_origin: None,
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: 0,
        final_boss_combat_record: None,
        combat_lab_probes: Vec::new(),
    }
}

fn normalized_campaign_boundary_title(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

pub fn campaign_replay_commands_for_path_v1(commands: &[String]) -> Vec<String> {
    let mut replay = Vec::with_capacity(commands.len().saturating_mul(2));
    for command in commands {
        replay.push(BRANCH_EXPERIMENT_REPLAY_ADVANCE_COMMAND.to_string());
        replay.push(command.clone());
    }
    replay
}

fn campaign_state_session_retention_policy_v1(
    config: &BranchCampaignConfigV1,
) -> BranchStateSessionRetentionPolicyV1 {
    BranchStateSessionRetentionPolicyV1 {
        max_frozen_exact_sessions: config.max_frozen,
        max_stuck_exact_sessions: config.max_active,
        max_abandoned_exact_sessions: 0,
        max_anchor_exact_sessions: config.max_active.saturating_add(config.max_frozen),
        max_suffix_commands_without_session: 6,
    }
}

fn campaign_elapsed_ms_u64(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn recover_auto_advanceable_stuck_branches_v1(
    scheduled: &mut Vec<BranchCampaignBranchV1>,
    parked: &mut Vec<BranchCampaignBranchV1>,
    stuck: &mut Vec<BranchCampaignBranchV1>,
    state_store: &mut BranchStateStoreV1,
    max_active: usize,
    max_frozen: usize,
) -> usize {
    if stuck.is_empty() {
        return 0;
    }

    let mut remaining = Vec::with_capacity(stuck.len());
    let mut recovered = 0usize;
    for branch in stuck.drain(..) {
        if let Some(recovered_branch) =
            try_recover_auto_advanceable_stuck_branch_v1(&branch, state_store)
        {
            let _placed = place_recovered_campaign_branch_v1(
                scheduled,
                parked,
                recovered_branch,
                max_active,
                max_frozen,
            );
            recovered = recovered.saturating_add(1);
        } else {
            remaining.push(branch);
        }
    }
    *stuck = remaining;
    recovered
}

fn campaign_progress_is_clearly_ahead_v1(left: (u8, i32, i32), right: (u8, i32, i32)) -> bool {
    if left.0 > right.0 {
        return true;
    }
    left.0 == right.0 && left.1 >= right.1.saturating_add(2)
}

fn maybe_attach_campaign_combat_lab_probe_v1(
    config: &BranchCampaignConfigV1,
    branch: &mut BranchCampaignBranchV1,
    session: &RunControlSession,
) {
    if !campaign_branch_should_probe_current_act_boss_v1(branch) {
        return;
    }
    let options = campaign_combat_lab_boss_probe_search_options_v1(config);
    let packet = current_act_boss_preview_probe_v1(session, &options, "campaign_key_boundary");
    branch.combat_lab_probes.push(packet);
}

fn campaign_branch_should_probe_current_act_boss_v1(branch: &BranchCampaignBranchV1) -> bool {
    if branch
        .combat_lab_probes
        .iter()
        .any(|probe| probe.kind == "current_act_boss_preview")
    {
        return false;
    }
    if !matches!(
        branch.status,
        BranchCampaignBranchStatusV1::Scheduled | BranchCampaignBranchStatusV1::Stuck
    ) {
        return false;
    }
    let Some(summary) = branch.summary.as_ref() else {
        return false;
    };
    if summary.act < 2
        || summary.boss.is_empty()
        || summary.floor < combat_lab_boss_probe_start_floor_v1(summary.act)
    {
        return false;
    }
    matches!(
        normalized_campaign_boundary_title(&branch.frontier_title).as_str(),
        "shop" | "campfire" | "cardreward" | "rewardscreen" | "rewardoverlay"
    )
}

fn combat_lab_boss_probe_start_floor_v1(act: u8) -> i32 {
    boss_approach_floor_v1(act).saturating_sub(1)
}

fn campaign_combat_lab_boss_probe_search_options_v1(
    config: &BranchCampaignConfigV1,
) -> RunControlSearchCombatOptions {
    let mut options = config.search_options.clone();
    let max_nodes = options
        .max_nodes
        .or(config.search_max_nodes)
        .unwrap_or(COMBAT_LAB_CAMPAIGN_BOSS_PROBE_MAX_NODES)
        .min(COMBAT_LAB_CAMPAIGN_BOSS_PROBE_MAX_NODES);
    let wall_ms = options
        .wall_ms
        .or(config.search_wall_ms)
        .unwrap_or(COMBAT_LAB_CAMPAIGN_BOSS_PROBE_MAX_WALL_MS)
        .min(COMBAT_LAB_CAMPAIGN_BOSS_PROBE_MAX_WALL_MS);
    options.max_nodes = Some(max_nodes);
    options.wall_ms = Some(wall_ms);
    options.max_hp_loss = Some(RunControlHpLossLimit::Unlimited);
    options.evidence = None;
    options
}

fn try_recover_auto_advanceable_stuck_branch_v1(
    branch: &BranchCampaignBranchV1,
    state_store: &mut BranchStateStoreV1,
) -> Option<BranchCampaignBranchV1> {
    let original_frontier = branch.frontier_title.clone();
    let mut session = state_store.get_session(&branch.commands)?.clone();
    let checkpoint_frontier = build_decision_surface(&session).view.header.title;
    if checkpoint_frontier != original_frontier && branch_boundary_available(&session) {
        let mut recovered = branch.clone();
        recovered.frontier_title = checkpoint_frontier;
        recovered.status = BranchCampaignBranchStatusV1::Scheduled;
        recovered.stop_reason = "recovered from checkpoint frontier drift".to_string();
        campaign_refresh_branch_summary_from_session_v1(&mut recovered, &session);
        return Some(recovered);
    }
    if campaign_stale_empty_portfolio_branch_is_now_available_v1(branch, &session) {
        let mut recovered = branch.clone();
        recovered.frontier_title = checkpoint_frontier;
        recovered.status = BranchCampaignBranchStatusV1::Scheduled;
        recovered.stop_reason = "recovered from current branch boundary".to_string();
        campaign_refresh_branch_summary_from_session_v1(&mut recovered, &session);
        state_store.insert_session(branch.commands.clone(), session);
        return Some(recovered);
    }

    let outcome = apply_branch_experiment_auto_run(
        &mut session,
        RunControlAutoStepOptions {
            max_operations: Some(1),
            route: RunControlRouteAutomationMode::Planner,
            ..Default::default()
        },
    )
    .ok()?;
    if outcome.action_result.is_none() {
        return None;
    }
    let new_frontier = build_decision_surface(&session).view.header.title;
    if new_frontier == original_frontier {
        return None;
    }

    let mut recovered = branch.clone();
    recovered.frontier_title = new_frontier;
    recovered.status = BranchCampaignBranchStatusV1::Scheduled;
    recovered.stop_reason = "recovered by one-step auto-advance".to_string();
    campaign_refresh_branch_summary_from_session_v1(&mut recovered, &session);
    state_store.insert_session(branch.commands.clone(), session);
    Some(recovered)
}

fn campaign_stale_empty_portfolio_branch_is_now_available_v1(
    branch: &BranchCampaignBranchV1,
    session: &RunControlSession,
) -> bool {
    branch
        .stop_reason
        .to_ascii_lowercase()
        .contains("option portfolio is empty")
        && branch_boundary_available(session)
}

fn place_recovered_campaign_branch_v1(
    scheduled: &mut Vec<BranchCampaignBranchV1>,
    parked: &mut Vec<BranchCampaignBranchV1>,
    mut recovered: BranchCampaignBranchV1,
    max_active: usize,
    max_frozen: usize,
) -> bool {
    let recovered_is_behind_scheduled = scheduled.iter().any(|branch| {
        campaign_progress_is_clearly_ahead_v1(
            branch_progress_key(branch),
            branch_progress_key(&recovered),
        )
    });
    if scheduled.len() < max_active && !recovered_is_behind_scheduled {
        recovered.status = BranchCampaignBranchStatusV1::Scheduled;
        scheduled.push(recovered);
        return true;
    }
    recovered.status = BranchCampaignBranchStatusV1::Parked;
    if parked.len() < max_frozen {
        parked.push(recovered);
        return true;
    }
    let Some((worst_index, worst_branch)) =
        parked.iter().enumerate().min_by(|(_, left), (_, right)| {
            campaign_branch_retention_key_v1(left).cmp(&campaign_branch_retention_key_v1(right))
        })
    else {
        return false;
    };
    if campaign_branch_retention_key_v1(&recovered)
        <= campaign_branch_retention_key_v1(worst_branch)
    {
        return false;
    }
    parked[worst_index] = recovered;
    true
}

pub(super) fn campaign_branch_quality_key_v1(branch: &BranchCampaignBranchV1) -> String {
    let frontier = normalized_campaign_boundary_title(&branch.frontier_title);
    let Some(summary) = branch.summary.as_ref() else {
        return format!(
            "frontier={frontier}|summary=-|choices={}",
            render_choice_path(&branch.choice_labels)
        );
    };
    let strengths = sorted_string_key_v1(&summary.formation_strengths);
    let needs = sorted_string_key_v1(&summary.formation_needs);
    let trajectory = if summary.trajectory_key.trim().is_empty() {
        format!(
            "recorded_choices={}",
            render_choice_path(&branch.choice_labels)
        )
    } else {
        summary.trajectory_key.clone()
    };
    let deck_identity = if summary.deck_key.trim().is_empty() {
        summary.deck_count.to_string()
    } else {
        summary.deck_key.clone()
    };
    format!(
        "frontier={frontier}|a{}f{}|hp={}/{}|gold={}|deck={}|stage={}|strengths={}|needs={}|traj={}",
        summary.act,
        summary.floor,
        summary.hp,
        summary.max_hp,
        summary.gold,
        deck_identity,
        summary.formation_stage,
        strengths,
        needs,
        trajectory
    )
}

fn sorted_string_key_v1(values: &[String]) -> String {
    if values.is_empty() {
        return "-".to_string();
    }
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted.join("+")
}

pub fn campaign_branch_from_report_branch_v1(
    parent: &BranchCampaignBranchV1,
    branch: &BranchExperimentBranchReportV1,
) -> BranchCampaignBranchV1 {
    let mut commands = parent.commands.clone();
    commands.extend(branch.choices.iter().map(|choice| choice.command.clone()));
    let mut choice_labels = parent.choice_labels.clone();
    choice_labels.extend(branch.choices.iter().map(campaign_choice_label_v1));
    BranchCampaignBranchV1 {
        branch_id: campaign_child_branch_id_v1(&parent.branch_id, &branch.branch_id),
        commands,
        choice_labels,
        summary: Some(campaign_summary_from_report_branch_v1(parent, branch)),
        strategic_summary: compact_branch_signature_data(&branch.retention.strategic_signature),
        frontier_title: branch.summary.boundary_title.clone(),
        status: campaign_status_from_report_status(branch.status),
        stop_reason: branch.stop_reason.clone(),
        continuation_origin: parent.continuation_origin.clone(),
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: branch.rank_key,
        final_boss_combat_record: branch.final_boss_combat_record.clone(),
        combat_lab_probes: Vec::new(),
    }
}

pub(super) fn campaign_child_branch_id_v1(parent_id: &str, child_id: &str) -> String {
    if parent_id.trim().is_empty() || parent_id == "root" {
        if child_id.starts_with("root") {
            return child_id.to_string();
        }
        return format!("root.{child_id}");
    }

    let suffix = child_id
        .strip_prefix("root.")
        .or_else(|| child_id.strip_prefix("root"))
        .unwrap_or(child_id)
        .trim_start_matches('.');
    if suffix.is_empty() {
        parent_id.to_string()
    } else {
        format!("{parent_id}.{suffix}")
    }
}

fn branch_progress_key(branch: &BranchCampaignBranchV1) -> (u8, i32, i32) {
    branch
        .summary
        .as_ref()
        .map(|summary| (summary.act, summary.floor, summary.hp))
        .unwrap_or((0, 0, 0))
}

fn campaign_should_stop_after_victory_v1(
    config: &BranchCampaignConfigV1,
    victories: &[BranchCampaignBranchV1],
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
) -> bool {
    if victories.is_empty() {
        return false;
    }
    if victories
        .iter()
        .any(|branch| branch_meets_victory_quality_v1(config, branch))
    {
        return true;
    }
    active.is_empty() && frozen.is_empty()
}

fn branch_meets_victory_quality_v1(
    config: &BranchCampaignConfigV1,
    branch: &BranchCampaignBranchV1,
) -> bool {
    let threshold = config.min_acceptable_victory_hp_percent as i64;
    if threshold == 0 {
        return true;
    }
    let Some(summary) = branch.summary.as_ref() else {
        return true;
    };
    if summary.max_hp <= 0 {
        return true;
    }
    (summary.hp.max(0) as i64) * 100 >= threshold * summary.max_hp as i64
}

fn campaign_choice_label_v1(
    choice: &crate::eval::branch_experiment::BranchExperimentChoiceV1,
) -> String {
    let label = if choice.effect_label.is_empty() {
        choice.label.clone()
    } else {
        choice.effect_label.clone()
    };
    let label = if choice.kind == "event" {
        label
            .split(" | event_eval ")
            .next()
            .unwrap_or(&label)
            .to_string()
    } else {
        label
    };
    let label = compact_campaign_choice_label_metadata_v1(&label);
    if choice.kind == "event" && label.starts_with('[') && !choice.boundary_title.trim().is_empty()
    {
        format!("{}: {}", choice.boundary_title, label)
    } else {
        label
    }
}

fn campaign_status_from_report_status(
    status: BranchExperimentBranchStatusV1,
) -> BranchCampaignBranchStatusV1 {
    match status {
        BranchExperimentBranchStatusV1::Active => BranchCampaignBranchStatusV1::Scheduled,
        BranchExperimentBranchStatusV1::TerminalVictory => {
            BranchCampaignBranchStatusV1::TerminalVictory
        }
        BranchExperimentBranchStatusV1::TerminalDefeat => {
            BranchCampaignBranchStatusV1::TerminalDefeat
        }
        BranchExperimentBranchStatusV1::Pruned => BranchCampaignBranchStatusV1::Abandoned,
        BranchExperimentBranchStatusV1::NeedsHumanBoundary
        | BranchExperimentBranchStatusV1::Failed => BranchCampaignBranchStatusV1::Stuck,
    }
}

#[cfg(test)]
mod tests;
