use crate::ai::strategic::{compact_branch_signature_data, BranchSignatureCompact};
use crate::content::cards::CardId;
use crate::eval::branch_experiment::{
    run_branch_experiment_from_session_after_prefix_with_snapshots_v1,
    run_branch_experiment_from_session_with_snapshots_v1, run_branch_experiment_with_snapshots_v1,
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentConfigV1,
    BranchExperimentRunResultV1, BranchExperimentStrategyRequestV1,
    BRANCH_EXPERIMENT_REPLAY_ADVANCE_COMMAND,
};
use crate::eval::branch_experiment_boundary::branch_boundary_available;
use crate::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use crate::eval::combat_lab_probe_v1::current_act_boss_preview_probe_v1;
use crate::eval::run_control::{
    apply_branch_experiment_auto_run, build_decision_surface, RunControlAutoStepOptions,
    RunControlCombatSegmentMode, RunControlConfig, RunControlHpLossLimit,
    RunControlRouteAutomationMode, RunControlSearchCombatOptions, RunControlSession,
    RunControlSessionCheckpointV1,
};
use crate::state::core::EngineState;
use crate::state::rewards::{RewardCard, RewardState};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

mod active_selection;
mod branch_display;
mod frozen_pool;
mod intervention;
mod lineage;
mod model;
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
pub use active_selection::select_campaign_branches_v1;
use active_selection::{
    append_discarded_examples_v1, branch_is_rehydrated_checkpointed_combat_failure_v1,
    campaign_progress_is_clearly_ahead_v1, promote_frozen_to_active_v1,
    promote_rehydrated_combat_failures_to_active_on_stall_v1,
    rebalance_active_lineage_diversity_v1, rebalance_active_with_stronger_frozen_v1,
    select_campaign_branches_for_config_v1,
};
use branch_display::{
    compact_campaign_choice_label_metadata_v1, render_choice_path, render_compact_choice_path,
};
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
    BranchCampaignCheckpointSessionV1, BranchCampaignCheckpointV1, BranchCampaignReportV1,
    BranchCampaignRoundSummaryV1, BranchCampaignRouteEvidenceExampleV1,
    BranchCampaignRouteEvidenceSummaryV1, BranchCampaignRunResultV1, BranchCampaignSelectionV1,
    BranchCampaignStateStoreSummaryV1, BranchCampaignStrategyRequestV1,
};
use performance::add_combat_performance_samples_v1;
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
pub use retry::{
    BranchCampaignCombatRetryLedgerEntryV1, BranchCampaignCombatRetryLedgerV1,
    BranchCampaignCombatRetryPolicyV1,
};
use route_evidence::{merge_campaign_route_decisions_v1, merge_campaign_route_evidence_summary_v1};
pub use run_domain::{
    branch_campaign_ascension_domain_label_v1, branch_campaign_ascension_domain_role_v1,
    branch_campaign_run_domain_v1, BranchCampaignRunDomainV1,
};
use selection_key::{
    act_boss_floor_v1, campaign_branch_retention_key_v1, compare_campaign_branches_for_promotion_v1,
};
use state_graph::{
    BranchStateReplayStartV1, BranchStateSessionRetentionPolicyV1, BranchStateStoreV1,
};
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
const COMBAT_RETRY_NODE_MULTIPLIER: usize = 4;
const COMBAT_RETRY_WALL_MULTIPLIER: u64 = 4;
const COMBAT_RETRY_MIN_NODES: usize = 200_000;
const COMBAT_RETRY_MAX_NODES: usize = 500_000;
const COMBAT_RETRY_MIN_WALL_MS: u64 = 1_000;
const COMBAT_RETRY_MAX_WALL_MS: u64 = 1_000;
const BOSS_GATE_RETRY_ATTEMPTS_PER_GATE: usize = 2;
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
    pub retention_budget_profile: BranchRetentionBudgetProfileV1,
    pub max_reward_options_per_branch: Option<usize>,
    pub max_campfire_options_per_branch: usize,
    pub auto_max_operations: usize,
    pub experiment_wall_ms: Option<u64>,
    pub search_max_nodes: Option<usize>,
    pub search_wall_ms: Option<u64>,
    pub search_max_hp_loss: Option<RunControlHpLossLimit>,
    pub search_options: RunControlSearchCombatOptions,
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
            combat_retry_policy: BranchCampaignCombatRetryPolicyV1::OnStall,
            combat_retry_wall_ms: None,
            include_event_reward_skip: false,
            min_acceptable_victory_hp_percent: 20,
            prefix_commands: Vec::new(),
        }
    }
}

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

struct BranchCampaignParentBatchResultV1 {
    candidates: Vec<BranchCampaignBranchV1>,
    strategy_requests: Vec<BranchExperimentStrategyRequestV1>,
    route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    explored_branch_points: usize,
    wall_limit_hit: bool,
    branch_limit_hit: bool,
    combat_budget_retries: usize,
    parent_elapsed_wall_ms_sum: u64,
    parent_elapsed_wall_ms_max: u64,
    combat_retry_elapsed_wall_ms_sum: u64,
    combat_retry_elapsed_wall_ms_max: u64,
    combat_performance: BranchCampaignCombatPerformanceSummaryV1,
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
    state_store: BranchStateStoreV1,
    recovered_checkpoint_failure_commands: BTreeSet<Vec<String>>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BranchCampaignBossGateRetryKeyV1 {
    act: u8,
    floor: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct BranchCampaignCombatRetryLedgerStateV1 {
    boss_gate_attempts: BTreeMap<BranchCampaignBossGateRetryKeyV1, usize>,
}

impl BranchCampaignCombatRetryLedgerStateV1 {
    fn from_report_v1(report: &BranchCampaignCombatRetryLedgerV1) -> Self {
        let mut ledger = Self::default();
        for entry in &report.boss_gate_attempts {
            let key = BranchCampaignBossGateRetryKeyV1 {
                act: entry.act,
                floor: entry.floor,
            };
            ledger
                .boss_gate_attempts
                .insert(key, entry.attempts.min(BOSS_GATE_RETRY_ATTEMPTS_PER_GATE));
        }
        ledger
    }

    fn to_report_v1(&self) -> BranchCampaignCombatRetryLedgerV1 {
        BranchCampaignCombatRetryLedgerV1 {
            boss_gate_attempts: self
                .boss_gate_attempts
                .iter()
                .map(|(key, attempts)| BranchCampaignCombatRetryLedgerEntryV1 {
                    act: key.act,
                    floor: key.floor,
                    attempts: *attempts,
                })
                .collect(),
        }
    }

    fn try_consume_boss_gate_retry_v1(&mut self, key: BranchCampaignBossGateRetryKeyV1) -> bool {
        let attempts = self.boss_gate_attempts.entry(key).or_default();
        if *attempts >= BOSS_GATE_RETRY_ATTEMPTS_PER_GATE {
            return false;
        }
        *attempts = attempts.saturating_add(1);
        true
    }
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
    run_branch_campaign_from_state_with_progress_v1(config, state, progress)
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
            let retry_gate_key = campaign_selection_act_boss_gate_retry_key_v1(&selected);
            let retry_allowed = retry_gate_key
                .map(|key| {
                    state
                        .combat_retry_ledger
                        .try_consume_boss_gate_retry_v1(key)
                })
                .unwrap_or(true);
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
        let frozen_added = append_limited_frozen_v1(
            &mut state.frozen,
            selected.frozen,
            config.max_frozen,
            &mut state.discarded_count,
            &mut state.discarded_examples,
        );
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
        };
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
        rounds_completed: state.rounds_completed,
        nodes: state.state_store.checkpoint_nodes(),
        sessions,
    }
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

fn run_campaign_parent_batch_v1<F>(
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
    if let Some(gate_key) = branch_report_act_boss_gate_retry_key_v1(&result.report.branches) {
        if !combat_retry_ledger.try_consume_boss_gate_retry_v1(gate_key) {
            return Ok(Ok(parent_round_result_without_retry_v1(result)));
        }
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

fn campaign_retry_timing_for_parent_v1(
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

fn campaign_branch_experiment_config_v1(
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
        include_skip: true,
        include_event_reward_skip: config.include_event_reward_skip,
        auto_leave_after_shop_purchase_branch: true,
        ..BranchExperimentConfigV1::default()
    }
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

fn campaign_parent_replay_error_is_retryable_v1(error: &str) -> bool {
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

fn combat_retry_campaign_config_v1(
    config: &BranchCampaignConfigV1,
) -> Option<BranchCampaignConfigV1> {
    let retry_nodes = retry_node_budget_v1(config.search_max_nodes);
    let retry_wall_ms = config
        .combat_retry_wall_ms
        .or_else(|| retry_wall_budget_v1(config.search_wall_ms));
    if retry_nodes == config.search_max_nodes && retry_wall_ms == config.search_wall_ms {
        return None;
    }

    let mut retry_config = config.clone();
    retry_config.search_max_nodes = retry_nodes;
    retry_config.search_wall_ms = retry_wall_ms;
    retry_config.max_branches_per_active = combat_retry_branch_width_v1(config);
    retry_config.search_max_hp_loss = config
        .search_max_hp_loss
        .or(Some(RunControlHpLossLimit::Unlimited));
    Some(retry_config)
}

fn combat_retry_branch_width_v1(config: &BranchCampaignConfigV1) -> usize {
    config.max_branches_per_active.min(config.max_active.max(1))
}

fn retry_node_budget_v1(current: Option<usize>) -> Option<usize> {
    let base = current.unwrap_or(COMBAT_RETRY_MIN_NODES);
    Some(
        base.saturating_mul(COMBAT_RETRY_NODE_MULTIPLIER)
            .max(COMBAT_RETRY_MIN_NODES)
            .min(COMBAT_RETRY_MAX_NODES),
    )
}

fn retry_wall_budget_v1(current: Option<u64>) -> Option<u64> {
    let base = current.unwrap_or(COMBAT_RETRY_MIN_WALL_MS);
    Some(
        base.saturating_mul(COMBAT_RETRY_WALL_MULTIPLIER)
            .max(COMBAT_RETRY_MIN_WALL_MS)
            .min(COMBAT_RETRY_MAX_WALL_MS),
    )
}

fn branch_report_needs_combat_budget_retry_v1(branches: &[BranchExperimentBranchReportV1]) -> bool {
    !branches.is_empty()
        && branches
            .iter()
            .all(|branch| branch.status == BranchExperimentBranchStatusV1::Pruned)
        && branches.iter().all(|branch| {
            normalized_campaign_boundary_title(&branch.summary.boundary_title) == "combat"
        })
}

fn campaign_parent_should_retry_combat_budget_now_v1(
    config: &BranchCampaignConfigV1,
    branches: &[BranchExperimentBranchReportV1],
) -> bool {
    if matches!(
        config.combat_retry_policy,
        BranchCampaignCombatRetryPolicyV1::Disabled
    ) {
        return false;
    }
    if !branch_report_needs_combat_budget_retry_v1(branches) {
        return false;
    }
    matches!(
        config.combat_retry_policy,
        BranchCampaignCombatRetryPolicyV1::Immediate
    ) || branch_report_is_act_boss_gate_combat_retry_candidate_v1(branches)
}

fn branch_report_is_act_boss_gate_combat_retry_candidate_v1(
    branches: &[BranchExperimentBranchReportV1],
) -> bool {
    branch_report_act_boss_gate_retry_key_v1(branches).is_some()
}

fn branch_report_act_boss_gate_retry_key_v1(
    branches: &[BranchExperimentBranchReportV1],
) -> Option<BranchCampaignBossGateRetryKeyV1> {
    branches.iter().find_map(|branch| {
        let act = branch.summary.act;
        let floor = branch.summary.floor;
        (normalized_campaign_boundary_title(&branch.summary.boundary_title) == "combat"
            && floor >= act_boss_floor_v1(act))
        .then_some(BranchCampaignBossGateRetryKeyV1 {
            act,
            floor: act_boss_floor_v1(act),
        })
    })
}

fn campaign_selection_act_boss_gate_retry_key_v1(
    selection: &BranchCampaignSelectionV1,
) -> Option<BranchCampaignBossGateRetryKeyV1> {
    selection.abandoned.iter().find_map(|branch| {
        let summary = branch.summary.as_ref()?;
        (normalized_campaign_boundary_title(&branch.frontier_title) == "combat"
            && summary.floor >= act_boss_floor_v1(summary.act))
        .then_some(BranchCampaignBossGateRetryKeyV1 {
            act: summary.act,
            floor: act_boss_floor_v1(summary.act),
        })
    })
}

fn campaign_round_should_retry_combat_budget_on_stall_v1(
    config: &BranchCampaignConfigV1,
    selection: &BranchCampaignSelectionV1,
    existing_frozen_branches: usize,
) -> bool {
    matches!(
        config.combat_retry_policy,
        BranchCampaignCombatRetryPolicyV1::OnStall
    ) && combat_retry_campaign_config_v1(config).is_some()
        && existing_frozen_branches == 0
        && selection.active.is_empty()
        && selection.frozen.is_empty()
        && selection.victories.is_empty()
        && selection.dead.is_empty()
        && selection.stuck.is_empty()
        && !selection.abandoned.is_empty()
        && selection
            .abandoned
            .iter()
            .all(|branch| normalized_campaign_boundary_title(&branch.frontier_title) == "combat")
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
