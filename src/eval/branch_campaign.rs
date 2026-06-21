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
use std::collections::BTreeSet;
use std::time::Instant;

mod active_lineage;
mod active_rebalance;
mod active_selection;
mod branch_display;
mod frozen_pool;
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
mod selection_key;
mod state_graph;
mod strategic_signals;
mod summary;
use active_lineage::{
    rebalance_active_lineage_diversity_v1, refill_active_boss_relic_axes_from_frozen_v1,
};
use active_rebalance::{
    branch_is_rehydrated_checkpointed_combat_failure_v1, campaign_progress_is_clearly_ahead_v1,
    promote_frozen_to_active_v1, promote_rehydrated_combat_failures_to_active_on_stall_v1,
    rebalance_active_with_stronger_frozen_v1,
};
pub use active_selection::select_campaign_branches_v1;
use active_selection::{append_discarded_examples_v1, select_campaign_branches_for_config_v1};
use branch_display::{compact_campaign_choice_label_metadata_v1, render_choice_path};
use frozen_pool::append_axis_limited_frozen_v1;
use frozen_pool::append_limited_frozen_v1;
#[cfg(test)]
use intervention::campaign_strategy_next_step_v1;
use intervention::{
    abandoned_branches_intervention_request_v1, campaign_strategy_requests_are_fatal_v1,
    leading_abandoned_combat_intervention_request_v1, merge_campaign_strategy_request_queue_v1,
    merge_campaign_strategy_requests_v1, prune_resolved_campaign_strategy_requests_v1,
};
#[cfg(test)]
use lineage::campaign_branch_boss_relic_lineage_key_v1;
pub use model::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignBranchV1,
    BranchCampaignCheckpointSessionV1, BranchCampaignCheckpointV1,
    BranchCampaignDecisionObservationV1, BranchCampaignReportV1, BranchCampaignRoundSummaryV1,
    BranchCampaignRouteEvidenceExampleV1, BranchCampaignRouteEvidenceSummaryV1,
    BranchCampaignRunPreludeV1, BranchCampaignRunResultV1, BranchCampaignSelectionV1,
    BranchCampaignStateStoreSummaryV1, BranchCampaignStrategyRequestV1,
};
use parent_batch::run_campaign_parent_batch_v1;
#[cfg(test)]
use parent_batch::{
    campaign_branch_experiment_config_v1, campaign_parent_replay_error_is_retryable_v1,
    campaign_retry_timing_for_parent_v1,
};
pub use performance::{
    BranchCampaignCombatPerformanceExampleV1, BranchCampaignCombatPerformanceSummaryV1,
};
pub use progress::{
    render_branch_campaign_progress_event_v1, render_branch_campaign_progress_event_with_detail_v1,
    BranchCampaignProgressDetailV1, BranchCampaignProgressEventV1,
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
use selection_key::{campaign_branch_retention_key_v1, compare_campaign_branches_for_promotion_v1};
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
    pub active_lineage_diversity_slots: usize,
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
            active_lineage_diversity_slots: 0,
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
    active: Vec<BranchCampaignBranchV1>,
    frozen: Vec<BranchCampaignBranchV1>,
    victories: Vec<BranchCampaignBranchV1>,
    dead: Vec<BranchCampaignBranchV1>,
    abandoned: Vec<BranchCampaignBranchV1>,
    stuck: Vec<BranchCampaignBranchV1>,
    discarded_count: usize,
    discarded_examples: Vec<String>,
    strategy_requests: Vec<BranchCampaignStrategyRequestV1>,
    route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1,
    rounds: Vec<BranchCampaignRoundSummaryV1>,
    journal: CampaignJournalV1,
    state_store: BranchStateStoreV1,
    recovered_checkpoint_failure_commands: BTreeSet<Vec<String>>,
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
        let rehydrated = rehydrate_checkpoint_failures_on_resume_v1(
            &mut state,
            config.max_active,
            config.max_frozen,
        );
        if rehydrated > 0 {
            state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
                state.strategy_requests,
                &state.active,
                &state.frozen,
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
        max_active: config.max_active,
        max_frozen: config.max_frozen,
    });

    let mut stop_reason = "max_rounds".to_string();

    for local_round in 0..config.max_rounds {
        let recovered = recover_auto_advanceable_stuck_branches_v1(
            &mut state.active,
            &mut state.frozen,
            &mut state.stuck,
            &mut state.state_store,
            config.max_active,
            config.max_frozen,
        );
        if recovered > 0 {
            state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
                state.strategy_requests,
                &state.active,
                &state.frozen,
                &state.stuck,
                &state.abandoned,
            );
            let promoted = rebalance_active_with_stronger_frozen_v1(
                &mut state.active,
                &mut state.frozen,
                config.max_active,
            );
            if promoted > 0 {
                progress(BranchCampaignProgressEventV1::FrozenPromoted {
                    promoted,
                    active_after: state.active.len(),
                    frozen_remaining: state.frozen.len(),
                    filled_active: 0,
                    stronger_rebalanced: promoted,
                    diversity_rebalanced: 0,
                    rehydrated_recovered: 0,
                    checkpoint_recovered: 0,
                });
            }
        }
        if state.active.is_empty()
            && !campaign_should_stop_after_victory_v1(
                config,
                &state.victories,
                &state.active,
                &state.frozen,
            )
            && !state.frozen.is_empty()
        {
            let promoted = promote_frozen_to_active_v1(
                &mut state.active,
                &mut state.frozen,
                config.max_active,
            );
            if promoted > 0 {
                progress(BranchCampaignProgressEventV1::FrozenPromoted {
                    promoted,
                    active_after: state.active.len(),
                    frozen_remaining: state.frozen.len(),
                    filled_active: promoted,
                    stronger_rebalanced: 0,
                    diversity_rebalanced: 0,
                    rehydrated_recovered: 0,
                    checkpoint_recovered: 0,
                });
            }
        }
        if state.active.is_empty()
            && !campaign_should_stop_after_victory_v1(
                config,
                &state.victories,
                &state.active,
                &state.frozen,
            )
        {
            let promoted = promote_rehydrated_combat_failures_to_active_on_stall_v1(
                &mut state.active,
                &mut state.frozen,
                config.max_active,
            );
            if promoted > 0 {
                progress(BranchCampaignProgressEventV1::FrozenPromoted {
                    promoted,
                    active_after: state.active.len(),
                    frozen_remaining: state.frozen.len(),
                    filled_active: 0,
                    stronger_rebalanced: 0,
                    diversity_rebalanced: 0,
                    rehydrated_recovered: promoted,
                    checkpoint_recovered: 0,
                });
            }
        }
        if state.active.is_empty()
            && state.frozen.is_empty()
            && !campaign_should_stop_after_victory_v1(
                config,
                &state.victories,
                &state.active,
                &state.frozen,
            )
        {
            let recovered =
                recover_checkpointed_combat_failures_on_stall_v1(&mut state, config.max_active);
            if recovered > 0 {
                state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
                    state.strategy_requests,
                    &state.active,
                    &state.frozen,
                    &state.stuck,
                    &state.abandoned,
                );
                progress(BranchCampaignProgressEventV1::FrozenPromoted {
                    promoted: recovered,
                    active_after: state.active.len(),
                    frozen_remaining: state.frozen.len(),
                    filled_active: 0,
                    stronger_rebalanced: 0,
                    diversity_rebalanced: 0,
                    rehydrated_recovered: 0,
                    checkpoint_recovered: recovered,
                });
            }
        }
        if state.active.is_empty()
            && campaign_should_stop_after_victory_v1(
                config,
                &state.victories,
                &state.active,
                &state.frozen,
            )
        {
            stop_reason = "victory_found".to_string();
            break;
        }
        if state.active.is_empty() {
            stop_reason = "no_active_branch".to_string();
            break;
        }
        let round_number = round_offset.saturating_add(local_round).saturating_add(1);
        progress(BranchCampaignProgressEventV1::RoundStarted {
            round: round_number,
            max_rounds: displayed_max_rounds,
            active_branches: state.active.len(),
            frozen_branches: state.frozen.len(),
        });
        let round_started_at = Instant::now();
        let parents = std::mem::take(&mut state.active);
        let started_active = parents.len();
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
        let mut selected = select_campaign_branches_for_config_v1(batch.candidates.clone(), config);
        if campaign_round_should_retry_combat_budget_on_stall_v1(
            config,
            &selected,
            state.frozen.len(),
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
                    selected =
                        select_campaign_branches_for_config_v1(batch.candidates.clone(), config);
                }
            }
        }
        state.strategy_requests = merge_campaign_strategy_request_queue_v1(
            state.strategy_requests,
            merge_campaign_strategy_requests_v1(batch.strategy_requests.clone()),
        );
        merge_campaign_route_evidence_summary_v1(&mut state.route_evidence, batch.route_evidence);
        let frozen_added = if config.boss_relic_axis_isolation {
            append_axis_limited_frozen_v1(
                &mut state.frozen,
                selected.frozen,
                config.max_frozen,
                &mut state.discarded_count,
                &mut state.discarded_examples,
            )
        } else {
            append_limited_frozen_v1(
                &mut state.frozen,
                selected.frozen,
                config.max_frozen,
                &mut state.discarded_count,
                &mut state.discarded_examples,
            )
        };
        state.discarded_count = state
            .discarded_count
            .saturating_add(selected.discarded_count);
        append_discarded_examples_v1(&mut state.discarded_examples, selected.discarded_examples);
        let dead_added = selected.dead.len();
        let abandoned_added = selected.abandoned.len();
        let victories_added = selected.victories.len();
        let stuck_added = selected.stuck.len();
        state.active = selected.active;
        state.victories.extend(selected.victories);
        state.dead.extend(selected.dead);
        state.abandoned.extend(selected.abandoned);
        state.stuck.extend(selected.stuck);
        recover_auto_advanceable_stuck_branches_v1(
            &mut state.active,
            &mut state.frozen,
            &mut state.stuck,
            &mut state.state_store,
            config.max_active,
            config.max_frozen,
        );
        let rebalanced_from_frozen = rebalance_active_with_stronger_frozen_v1(
            &mut state.active,
            &mut state.frozen,
            config.max_active,
        );
        let diversity_rebalanced_from_frozen = if config.active_lineage_diversity_slots > 0 {
            rebalance_active_lineage_diversity_v1(
                &mut state.active,
                &mut state.frozen,
                config.active_lineage_diversity_slots,
            )
        } else {
            0
        };
        let axis_refilled_from_frozen = if config.boss_relic_axis_isolation {
            refill_active_boss_relic_axes_from_frozen_v1(
                &mut state.active,
                &mut state.frozen,
                config.max_active,
            )
        } else {
            0
        };
        state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
            state.strategy_requests,
            &state.active,
            &state.frozen,
            &state.stuck,
            &state.abandoned,
        );
        state.state_store.retain_for_branches_with_session_policy(
            &state.active,
            &state.frozen,
            &state.abandoned,
            &state.stuck,
            campaign_state_session_retention_policy_v1(config),
        );
        let leading_abandoned_request = if state.active.is_empty() && state.victories.is_empty() {
            leading_abandoned_combat_intervention_request_v1(&state.frozen, &state.abandoned)
        } else {
            None
        };
        if let Some(request) = leading_abandoned_request {
            state.strategy_requests =
                merge_campaign_strategy_request_queue_v1(state.strategy_requests, vec![request]);
        }
        let promoted_from_frozen = if state.active.is_empty()
            && !campaign_should_stop_after_victory_v1(
                config,
                &state.victories,
                &state.active,
                &state.frozen,
            ) {
            promote_frozen_to_active_v1(&mut state.active, &mut state.frozen, config.max_active)
        } else {
            0
        };
        let promoted_rehydrated_from_frozen = if state.active.is_empty()
            && !campaign_should_stop_after_victory_v1(
                config,
                &state.victories,
                &state.active,
                &state.frozen,
            ) {
            promote_rehydrated_combat_failures_to_active_on_stall_v1(
                &mut state.active,
                &mut state.frozen,
                config.max_active,
            )
        } else {
            0
        };
        let recovered_from_abandoned = if state.active.is_empty()
            && state.frozen.is_empty()
            && !campaign_should_stop_after_victory_v1(
                config,
                &state.victories,
                &state.active,
                &state.frozen,
            ) {
            recover_checkpointed_combat_failures_on_stall_v1(&mut state, config.max_active)
        } else {
            0
        };
        if recovered_from_abandoned > 0 {
            state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
                state.strategy_requests,
                &state.active,
                &state.frozen,
                &state.stuck,
                &state.abandoned,
            );
        }
        let total_promoted_from_frozen = promoted_from_frozen
            .saturating_add(rebalanced_from_frozen)
            .saturating_add(diversity_rebalanced_from_frozen)
            .saturating_add(axis_refilled_from_frozen)
            .saturating_add(promoted_rehydrated_from_frozen)
            .saturating_add(recovered_from_abandoned);
        let round_summary = BranchCampaignRoundSummaryV1 {
            round: round_number,
            started_active,
            produced_branches,
            active_after: state.active.len(),
            frozen_added,
            dead_added,
            abandoned_added,
            victories_added,
            stuck_added,
            discarded_added: selected.discarded_count,
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
            started_active,
            produced_branches,
            active_after: state.active.len(),
            frozen_added,
            strategy_requests: state.strategy_requests.len(),
        });
        if total_promoted_from_frozen > 0 {
            progress(BranchCampaignProgressEventV1::FrozenPromoted {
                promoted: total_promoted_from_frozen,
                active_after: state.active.len(),
                frozen_remaining: state.frozen.len(),
                filled_active: promoted_from_frozen,
                stronger_rebalanced: rebalanced_from_frozen,
                diversity_rebalanced: diversity_rebalanced_from_frozen,
                rehydrated_recovered: promoted_rehydrated_from_frozen,
                checkpoint_recovered: recovered_from_abandoned,
            });
        }
        state.rounds.push(round_summary);
        state.rounds_completed = state.rounds_completed.saturating_add(1);

        if campaign_should_stop_after_victory_v1(
            config,
            &state.victories,
            &state.active,
            &state.frozen,
        ) {
            stop_reason = "victory_found".to_string();
            break;
        }
        if state.active.is_empty()
            && state.frozen.is_empty()
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
            &state.active,
            &state.frozen,
            &state.strategy_requests,
        ) {
            stop_reason = "needs_intervention".to_string();
            break;
        }
        if state.active.is_empty() && state.frozen.is_empty() && !state.stuck.is_empty() {
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
        active: state.active.len(),
        frozen: state.frozen.len(),
        victories: state.victories.len(),
        stuck: state.stuck.len(),
    });

    campaign_refresh_all_branch_summaries_from_state_store_v1(&mut state);

    let strategic_signals = campaign_strategic_signals_from_groups_v1(
        &state.active,
        &state.frozen,
        &state.victories,
        &state.abandoned,
        &state.stuck,
    );
    let checkpoint = campaign_checkpoint_from_state_v1(config, &state);
    let report = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: config.seed,
        run_domain: branch_campaign_run_domain_v1(config.ascension_level, config.player_class),
        run_prelude: branch_campaign_run_prelude_v1(config),
        rounds_completed: state.rounds_completed,
        stop_reason,
        active: state.active,
        frozen: state.frozen,
        victories: state.victories,
        dead: state.dead,
        abandoned: state.abandoned,
        stuck: state.stuck,
        discarded_count: state.discarded_count,
        discarded_examples: state.discarded_examples,
        strategy_requests: state.strategy_requests,
        route_evidence: state.route_evidence,
        combat_retry_ledger: state.combat_retry_ledger.to_report_v1(),
        strategic_signals,
        state_store: state.state_store.summary(),
        journal: state.journal,
        rounds: state.rounds,
    };
    Ok(BranchCampaignRunResultV1 { report, checkpoint })
}

fn root_campaign_state_v1() -> BranchCampaignRunStateV1 {
    BranchCampaignRunStateV1 {
        rounds_completed: 0,
        active: vec![root_campaign_branch_v1()],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::default(),
        rounds: Vec::new(),
        journal: CampaignJournalV1::new(),
        state_store: BranchStateStoreV1::new(),
        recovered_checkpoint_failure_commands: BTreeSet::new(),
    }
}

fn campaign_state_from_report_v1(report: &BranchCampaignReportV1) -> BranchCampaignRunStateV1 {
    BranchCampaignRunStateV1 {
        rounds_completed: report.rounds_completed,
        active: report.active.clone(),
        frozen: report.frozen.clone(),
        victories: report.victories.clone(),
        dead: report.dead.clone(),
        abandoned: report.abandoned.clone(),
        stuck: report.stuck.clone(),
        discarded_count: report.discarded_count,
        discarded_examples: report.discarded_examples.clone(),
        strategy_requests: report.strategy_requests.clone(),
        route_evidence: report.route_evidence.clone(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::from_report_v1(
            &report.combat_retry_ledger,
        ),
        rounds: report.rounds.clone(),
        journal: report.journal.clone(),
        state_store: BranchStateStoreV1::new(),
        recovered_checkpoint_failure_commands: BTreeSet::new(),
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
        .active
        .iter()
        .chain(state.frozen.iter())
        .chain(state.abandoned.iter())
        .chain(state.stuck.iter())
        .map(|branch| branch.commands.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for entry in &checkpoint.sessions {
        if keep.contains(&entry.commands) {
            state.state_store.insert_session(
                entry.commands.clone(),
                entry.session.clone().into_session().map_err(|err| {
                    format!("failed to restore campaign checkpoint session: {err}")
                })?,
            );
        }
    }
    state
        .stuck
        .retain(|branch| state.state_store.contains_commands(&branch.commands));
    state.strategy_requests = prune_resolved_campaign_strategy_requests_v1(
        state.strategy_requests,
        &state.active,
        &state.frozen,
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
    for branch in state
        .active
        .iter()
        .chain(state.frozen.iter())
        .chain(state.abandoned.iter())
        .chain(state.stuck.iter())
    {
        if let Some(session) = state.state_store.get_session(&branch.commands) {
            sessions.push(BranchCampaignCheckpointSessionV1 {
                commands: branch.commands.clone(),
                session: RunControlSessionCheckpointV1::from_session(session),
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
        sessions,
    }
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
        status: BranchCampaignBranchStatusV1::Active,
        stop_reason: "initial".to_string(),
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
        max_suffix_commands_without_session: 6,
    }
}

fn campaign_elapsed_ms_u64(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn recover_auto_advanceable_stuck_branches_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
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
                active,
                frozen,
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

fn recover_checkpointed_combat_failures_on_stall_v1(
    state: &mut BranchCampaignRunStateV1,
    max_active: usize,
) -> usize {
    if max_active == 0 || !state.active.is_empty() || !state.frozen.is_empty() {
        return 0;
    }
    if state.state_store.is_empty() || state.abandoned.is_empty() {
        return 0;
    }

    let mut candidates = Vec::new();
    let mut remaining = Vec::new();
    for branch in std::mem::take(&mut state.abandoned) {
        if state
            .recovered_checkpoint_failure_commands
            .contains(&branch.commands)
        {
            remaining.push(branch);
            continue;
        }
        if campaign_checkpoint_failure_is_combat_resume_candidate_v1(
            &branch,
            match state.state_store.get_session(&branch.commands) {
                Some(session) => session,
                None => {
                    remaining.push(branch);
                    continue;
                }
            },
        ) {
            candidates.push(branch);
        } else {
            remaining.push(branch);
        }
    }
    candidates.sort_by(compare_campaign_branches_for_promotion_v1);

    let mut recovered = 0usize;
    for branch in candidates {
        if recovered >= max_active {
            remaining.push(branch);
            continue;
        }
        match try_rehydrate_checkpoint_failure_branch_v1(&branch, &state.state_store) {
            Some(mut recovered_branch) => {
                recovered_branch.status = BranchCampaignBranchStatusV1::Active;
                state
                    .recovered_checkpoint_failure_commands
                    .insert(branch.commands.clone());
                state.active.push(recovered_branch);
                recovered = recovered.saturating_add(1);
            }
            None => remaining.push(branch),
        }
    }
    state.abandoned = remaining;
    recovered
}

fn rehydrate_checkpoint_failures_on_resume_v1(
    state: &mut BranchCampaignRunStateV1,
    max_active: usize,
    max_frozen: usize,
) -> usize {
    if state.state_store.is_empty() {
        return 0;
    }

    let mut recovered = 0usize;
    let abandoned = std::mem::take(&mut state.abandoned);
    state.abandoned = rehydrate_checkpoint_failure_list_v1(
        abandoned,
        &mut state.active,
        &mut state.frozen,
        &state.state_store,
        &mut state.recovered_checkpoint_failure_commands,
        max_active,
        max_frozen,
        max_active,
        &mut recovered,
    );
    let stuck = std::mem::take(&mut state.stuck);
    state.stuck = rehydrate_checkpoint_failure_list_v1(
        stuck,
        &mut state.active,
        &mut state.frozen,
        &state.state_store,
        &mut state.recovered_checkpoint_failure_commands,
        max_active,
        max_frozen,
        max_active,
        &mut recovered,
    );
    recovered = recovered.saturating_add(recover_auto_advanceable_stuck_branches_v1(
        &mut state.active,
        &mut state.frozen,
        &mut state.stuck,
        &mut state.state_store,
        max_active,
        max_frozen,
    ));
    recovered
}

fn rehydrate_checkpoint_failure_list_v1(
    branches: Vec<BranchCampaignBranchV1>,
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    state_store: &BranchStateStoreV1,
    recovered_commands: &mut BTreeSet<Vec<String>>,
    max_active: usize,
    max_frozen: usize,
    max_recovered: usize,
    recovered_count: &mut usize,
) -> Vec<BranchCampaignBranchV1> {
    let mut remaining = Vec::new();
    let mut candidates = Vec::<(BranchCampaignBranchV1, BranchCampaignBranchV1)>::new();
    for branch in branches {
        if let Some(recovered) = try_rehydrate_checkpoint_failure_branch_v1(&branch, state_store) {
            candidates.push((branch, recovered));
        } else {
            remaining.push(branch);
        }
    }
    candidates
        .sort_by(|(_, left), (_, right)| compare_campaign_branches_for_promotion_v1(left, right));

    for (branch, recovered) in candidates {
        if *recovered_count < max_recovered
            && place_recovered_campaign_branch_v1(active, frozen, recovered, max_active, max_frozen)
        {
            recovered_commands.insert(branch.commands.clone());
            *recovered_count = recovered_count.saturating_add(1);
        } else {
            remaining.push(branch);
        }
    }

    remaining
}

fn try_rehydrate_checkpoint_failure_branch_v1(
    branch: &BranchCampaignBranchV1,
    state_store: &BranchStateStoreV1,
) -> Option<BranchCampaignBranchV1> {
    let session = state_store.get_session(&branch.commands)?;
    if !campaign_checkpoint_failure_is_combat_resume_candidate_v1(branch, session) {
        return None;
    }

    let mut recovered = branch.clone();
    let previous_status = format!("{:?}", recovered.status);
    campaign_refresh_branch_summary_from_session_v1(&mut recovered, session);
    recovered.status = BranchCampaignBranchStatusV1::Active;
    recovered.stop_reason = if recovered.stop_reason.trim().is_empty() {
        format!("rehydrated checkpointed {previous_status} combat branch")
    } else {
        format!(
            "rehydrated checkpointed {previous_status} combat branch: {}",
            recovered.stop_reason
        )
    };
    Some(recovered)
}

fn campaign_checkpoint_failure_is_combat_resume_candidate_v1(
    branch: &BranchCampaignBranchV1,
    session: &RunControlSession,
) -> bool {
    if !matches!(
        branch.status,
        BranchCampaignBranchStatusV1::Abandoned | BranchCampaignBranchStatusV1::Stuck
    ) {
        return false;
    }
    let frontier = normalized_campaign_boundary_title(&branch.frontier_title);
    if frontier.starts_with("combat") {
        return true;
    }
    if campaign_session_is_combat_boundary_v1(session) {
        return true;
    }
    let stop = branch.stop_reason.to_ascii_lowercase();
    stop.contains("combat search")
        || stop.contains("search-combat")
        || stop.contains("hp-loss")
        || stop.contains("max_hp_loss")
        || stop.contains("high-stakes combat")
        || stop.contains("complete_winning_candidate")
}

fn campaign_session_is_combat_boundary_v1(session: &RunControlSession) -> bool {
    matches!(
        session.engine_state,
        crate::state::core::EngineState::CombatStart(_)
            | crate::state::core::EngineState::CombatPlayerTurn
            | crate::state::core::EngineState::CombatProcessing
            | crate::state::core::EngineState::PendingChoice(_)
    )
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
        BranchCampaignBranchStatusV1::Active | BranchCampaignBranchStatusV1::Stuck
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
        recovered.status = BranchCampaignBranchStatusV1::Active;
        recovered.stop_reason = "recovered from checkpoint frontier drift".to_string();
        campaign_refresh_branch_summary_from_session_v1(&mut recovered, &session);
        return Some(recovered);
    }
    if campaign_stale_empty_portfolio_branch_is_now_available_v1(branch, &session) {
        let mut recovered = branch.clone();
        recovered.frontier_title = checkpoint_frontier;
        recovered.status = BranchCampaignBranchStatusV1::Active;
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
    recovered.status = BranchCampaignBranchStatusV1::Active;
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
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    mut recovered: BranchCampaignBranchV1,
    max_active: usize,
    max_frozen: usize,
) -> bool {
    let recovered_is_behind_active = active.iter().any(|branch| {
        campaign_progress_is_clearly_ahead_v1(
            branch_progress_key(branch),
            branch_progress_key(&recovered),
        )
    });
    if active.len() < max_active
        && !recovered_is_behind_active
        && !branch_is_rehydrated_checkpointed_combat_failure_v1(&recovered)
    {
        recovered.status = BranchCampaignBranchStatusV1::Active;
        active.push(recovered);
        return true;
    }
    recovered.status = BranchCampaignBranchStatusV1::Frozen;
    if frozen.len() < max_frozen {
        frozen.push(recovered);
        return true;
    }
    let Some((worst_index, worst_branch)) =
        frozen.iter().enumerate().min_by(|(_, left), (_, right)| {
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
    frozen[worst_index] = recovered;
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
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: branch.rank_key,
        final_boss_combat_record: branch.final_boss_combat_record.clone(),
        combat_lab_probes: Vec::new(),
    }
}

fn campaign_child_branch_id_v1(parent_id: &str, child_id: &str) -> String {
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
        BranchExperimentBranchStatusV1::Active => BranchCampaignBranchStatusV1::Active,
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
