use crate::ai::strategic::{
    compact_branch_signature_data, format_compact_branch_signature, run_debt_ledger_v1,
    BranchSignatureCompact,
};
use crate::eval::branch_experiment::{
    run_branch_experiment_from_session_after_prefix_with_snapshots_v1,
    run_branch_experiment_from_session_with_snapshots_v1, run_branch_experiment_with_snapshots_v1,
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentConfigV1,
    BranchExperimentRouteDecisionV1, BranchExperimentRunResultV1,
    BranchExperimentStrategyRequestV1, BRANCH_EXPERIMENT_REPLAY_ADVANCE_COMMAND,
};
use crate::eval::branch_experiment_boundary::branch_boundary_available;
use crate::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use crate::eval::branch_experiment_trajectory::{
    branch_trajectory_key_v1, BranchTrajectorySignatureV1,
};
use crate::eval::combat_lab_probe_v1::current_act_boss_preview_probe_v1;
use crate::eval::run_control::{
    apply_branch_experiment_auto_run, build_decision_surface, RunControlAutoStepOptions,
    RunControlCombatSegmentMode, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession, RunControlSessionCheckpointV1,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

mod model;
mod performance;
mod progress;
mod retry;
mod run_domain;
mod selection_key;
mod state_graph;
mod strategic_signals;
pub use model::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignBranchV1,
    BranchCampaignCheckpointSessionV1, BranchCampaignCheckpointV1, BranchCampaignReportV1,
    BranchCampaignRoundSummaryV1, BranchCampaignRouteEvidenceExampleV1,
    BranchCampaignRouteEvidenceSummaryV1, BranchCampaignRunResultV1, BranchCampaignSelectionV1,
    BranchCampaignStateStoreSummaryV1, BranchCampaignStrategyRequestV1,
};
use performance::{
    add_combat_performance_samples_v1, aggregate_campaign_combat_performance_v1,
    format_seconds_from_us_1dp_v1, render_campaign_combat_performance_v1,
};
pub use performance::{
    BranchCampaignCombatPerformanceExampleV1, BranchCampaignCombatPerformanceSummaryV1,
};
pub use progress::{render_branch_campaign_progress_event_v1, BranchCampaignProgressEventV1};
pub use retry::{
    BranchCampaignCombatRetryLedgerEntryV1, BranchCampaignCombatRetryLedgerV1,
    BranchCampaignCombatRetryPolicyV1,
};
pub use run_domain::{
    branch_campaign_ascension_domain_label_v1, branch_campaign_ascension_domain_role_v1,
    branch_campaign_run_domain_v1, BranchCampaignRunDomainV1,
};
use selection_key::{
    act_boss_floor_v1, campaign_branch_retention_key_v1, compare_campaign_branches_for_active_v1,
    compare_campaign_branches_for_promotion_v1, render_campaign_branch_selection_basis_v1,
};
use state_graph::{
    BranchStateReplayStartV1, BranchStateSessionRetentionPolicyV1, BranchStateStoreV1,
};
use strategic_signals::{
    campaign_strategic_signals_for_render_v1, campaign_strategic_signals_from_groups_v1,
    render_campaign_strategic_concern_v1, render_campaign_strategic_signals_v1,
};
pub use strategic_signals::{
    BranchCampaignStrategicSignalGroupV1, BranchCampaignStrategicSignalsV1,
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
const PROGRESS_ANCHOR_MAX_RANK_LAG: i32 = 1_000;
const SURVIVAL_ANCHOR_LOW_HP_PERCENT: i32 = 25;
const SURVIVAL_ANCHOR_NEARBY_MIN_HP_GAIN: i32 = 20;
const SURVIVAL_ANCHOR_HEALTHY_SALVAGE_HP_PERCENT: i32 = 50;
const SURVIVAL_ANCHOR_HEALTHY_SALVAGE_HP_GAIN: i32 = 40;
const SURVIVAL_ANCHOR_CRITICAL_HP_PERCENT: i32 = 15;
const SURVIVAL_ANCHOR_CRITICAL_SALVAGE_HP_PERCENT: i32 = 30;
const SURVIVAL_ANCHOR_CRITICAL_SALVAGE_HP_GAIN: i32 = 25;
const BOSS_RELIC_CHOICE_LABELS_V1: &[&str] = &[
    "Astrolabe",
    "BlackBlood",
    "BlackStar",
    "BustedCrown",
    "CallingBell",
    "CoffeeDripper",
    "CursedKey",
    "Ectoplasm",
    "EmptyCage",
    "FrozenCore",
    "FusionHammer",
    "HolyWater",
    "HoveringKite",
    "Inserter",
    "MarkOfPain",
    "NuclearBattery",
    "PandorasBox",
    "PhilosopherStone",
    "RingOfTheSerpent",
    "RunicCube",
    "RunicDome",
    "RunicPyramid",
    "SacredBark",
    "SlaversCollar",
    "SneckoEye",
    "Sozu",
    "TinyHouse",
    "VelvetChoker",
    "VioletLotus",
    "WristBlade",
];

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

pub fn render_branch_campaign_compact_v1(
    report: &BranchCampaignReportV1,
    branch_examples: usize,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} seed={} ascension=A{} domain={} role={} class={} rounds={} stop={}",
        report.schema_name,
        report.seed,
        report.run_domain.ascension_level,
        report.run_domain.label,
        report.run_domain.role,
        report.run_domain.player_class,
        report.rounds_completed,
        report.stop_reason
    ));
    lines.push(format!(
        "Active {} | Frozen {} | Dead {} | Abandoned {} | Victories {} | Stuck {} | Discarded {}",
        report.active.len(),
        report.frozen.len(),
        report.dead.len(),
        report.abandoned.len(),
        report.victories.len(),
        report.stuck.len(),
        report.discarded_count
    ));
    if let Some(round) = report.rounds.last() {
        lines.push(format!(
            "Last round: started={} produced={} branch_points={} active_after={} frozen_added={} discarded_added={} combat_retries={} limits=[{}{}]",
            round.started_active,
            round.produced_branches,
            round.explored_branch_points,
            round.active_after,
            round.frozen_added,
            round.discarded_added,
            round.combat_budget_retries,
            if round.branch_limit_hit { "branch" } else { "" },
            if round.wall_limit_hit {
                if round.branch_limit_hit { ",wall" } else { "wall" }
            } else {
                ""
            }
        ));
    }
    let total_round_elapsed_ms: u64 = report
        .rounds
        .iter()
        .map(|round| round.elapsed_wall_ms)
        .sum();
    let parent_elapsed_wall_ms_sum: u64 = report
        .rounds
        .iter()
        .map(|round| round.parent_elapsed_wall_ms_sum)
        .sum();
    let parent_elapsed_wall_ms_max = report
        .rounds
        .iter()
        .map(|round| round.parent_elapsed_wall_ms_max)
        .max()
        .unwrap_or_default();
    let combat_retry_elapsed_wall_ms_sum: u64 = report
        .rounds
        .iter()
        .map(|round| round.combat_retry_elapsed_wall_ms_sum)
        .sum();
    let combat_retry_elapsed_wall_ms_max = report
        .rounds
        .iter()
        .map(|round| round.combat_retry_elapsed_wall_ms_max)
        .max()
        .unwrap_or_default();
    if total_round_elapsed_ms > 0 || parent_elapsed_wall_ms_sum > 0 {
        lines.push(format!(
            "Timing: rounds={} parent_sum={} parent_max={} combat_retry_sum={} combat_retry_max={}",
            format_seconds_1dp_v1(total_round_elapsed_ms),
            format_seconds_1dp_v1(parent_elapsed_wall_ms_sum),
            format_seconds_1dp_v1(parent_elapsed_wall_ms_max),
            format_seconds_1dp_v1(combat_retry_elapsed_wall_ms_sum),
            format_seconds_1dp_v1(combat_retry_elapsed_wall_ms_max),
        ));
    }
    if !report.state_store.is_empty() {
        lines.push(format!(
            "State store: sessions={} nodes={} linked={} replay=exact:{} ancestor:{} miss:{} suffix=sum:{} max:{} cache=pruned:{} anchors:{} lookups={}/{} inserts={} retains={}",
            report.state_store.sessions,
            report.state_store.nodes,
            report.state_store.linked_nodes,
            report.state_store.replay_exact_hits,
            report.state_store.replay_ancestor_hits,
            report.state_store.replay_misses,
            report.state_store.replay_suffix_commands_sum,
            report.state_store.replay_suffix_commands_max,
            report.state_store.sessions_pruned,
            report.state_store.anchor_sessions_kept,
            report.state_store.lookup_hits,
            report.state_store.lookup_misses,
            report.state_store.inserts,
            report.state_store.retains,
        ));
    }
    let combat_performance = aggregate_campaign_combat_performance_v1(report);
    if combat_performance.samples > 0 {
        lines.push(render_campaign_combat_performance_v1(&combat_performance));
        if let Some(example) = combat_performance.slowest.first() {
            lines.push(format!(
                "  slowest: A{}F{} turn={} {} {} {} bucket={} status={}",
                example.act,
                example.floor,
                example.turn,
                example.combat_kind,
                example.enemies,
                format_seconds_from_us_1dp_v1(example.total_us),
                example.dominant_bucket,
                example.coverage_status
            ));
        }
    }
    if !report.combat_retry_ledger.is_empty() {
        lines.push(format!(
            "Combat retry ledger: boss_gate={}",
            render_combat_retry_ledger_v1(&report.combat_retry_ledger)
        ));
    }
    if report.discarded_count > 0 && !report.discarded_examples.is_empty() {
        lines.push(format!(
            "Branch pressure: discarded={} examples=[{}]",
            report.discarded_count,
            render_branch_pressure_examples_v1(&report.discarded_examples)
        ));
    }
    if !report.abandoned.is_empty() {
        lines.push(format!(
            "Abandoned examples: count={} reasons=[{}] examples=[{}]",
            report.abandoned.len(),
            render_campaign_branch_stop_reasons_v1(&report.abandoned, 3),
            render_campaign_branch_examples_v1(&report.abandoned, 3)
        ));
    }
    if let Some(final_boss_failures) =
        render_campaign_final_boss_failure_summary_v1(report, branch_examples)
    {
        lines.push(final_boss_failures);
    }
    if let Some(boss_relic_coverage) = render_campaign_boss_relic_coverage_v1(report) {
        lines.push(boss_relic_coverage);
    }
    if report.route_evidence.decisions > 0 {
        lines.push(format!(
            "Route evidence: decisions={} first_elite optional={} forced={} none={} avg_elite_prep={} underprepared={} bailouts=rest:{} shop:{}",
            report.route_evidence.decisions,
            report.route_evidence.first_elite_optional,
            report.route_evidence.first_elite_forced,
            report.route_evidence.first_elite_none,
            format_bp(report.route_evidence.avg_elite_prep_bp),
            report.route_evidence.underprepared_first_elite,
            report.route_evidence.rest_bailout,
            report.route_evidence.shop_bailout,
        ));
        if let Some(example) = report.route_evidence.examples.first() {
            lines.push(format!(
                "  example: {} | first_elite={} elite_prep={}",
                example.target,
                example.first_elite,
                format_bp(example.elite_prep_bp)
            ));
        }
        if report.route_evidence.underprepared_first_elite > 0 {
            lines.push(format!(
                "Route concern: forced_first_elite_underprepared={}/{} rest_bailout={} shop_bailout={}",
                report.route_evidence.underprepared_first_elite,
                report.route_evidence.decisions,
                report.route_evidence.rest_bailout,
                report.route_evidence.shop_bailout,
            ));
            if let Some(example) = report.route_evidence.underprepared_examples.first() {
                lines.push(format!(
                    "  concern example: {} | first_elite={} elite_prep={}",
                    example.target,
                    example.first_elite,
                    format_bp(example.elite_prep_bp)
                ));
            }
        }
    }
    if let Some(pressure) = campaign_unspent_gold_pressure_v1(report) {
        lines.push(format!(
            "Resource concern: high_unspent_gold_near_boss={} max_gold={} causes=[{}]",
            pressure.count, pressure.max_gold, pressure.cause_counts
        ));
        lines.push(format!("  resource example: {}", pressure.example));
    }
    if let Some(pressure) = campaign_boss_mechanic_pressure_v1(report) {
        lines.push(format!(
            "Boss pressure: bosses=[{}] signals=[{}]",
            pressure.boss_counts, pressure.signal_counts
        ));
        lines.push(format!("  boss example: {}", pressure.example));
    }
    if let Some(combat_lab) = render_campaign_combat_lab_probe_summary_v1(report) {
        lines.extend(combat_lab);
    }
    let strategic_signals = campaign_strategic_signals_for_render_v1(report);
    if let Some(strategic) = render_campaign_strategic_signals_v1(&strategic_signals) {
        lines.push(strategic);
    }
    if let Some(concern) = render_campaign_strategic_concern_v1(&strategic_signals) {
        lines.push(concern);
    }
    if let Some(choice_coverage) = render_campaign_choice_coverage_v1(report) {
        lines.push(choice_coverage);
    }
    if let Some(victory_lines) = render_campaign_victory_quality_lines_v1(report) {
        lines.push(String::new());
        lines.extend(victory_lines);
    }
    if report.stop_reason == "max_rounds"
        && (!report.active.is_empty() || !report.frozen.is_empty())
    {
        lines.push(
            "Next: budget ended; use .\\tools\\campaign.ps1 -More, or .\\tools\\campaign.ps1 -More -Rounds N to add a small fixed number of rounds"
                .to_string(),
        );
    }
    let render_strategy_requests = report.victories.is_empty()
        && !report.strategy_requests.is_empty()
        && (campaign_report_stop_needs_immediate_intervention_v1(report)
            || report.active.is_empty());
    if render_strategy_requests {
        lines.push(String::new());
        if campaign_report_stop_needs_immediate_intervention_v1(report) {
            lines.push("Needs intervention:".to_string());
        } else {
            lines.push("Deferred strategy notes:".to_string());
        }
        for request in report.strategy_requests.iter().take(4) {
            lines.push(format!(
                "  {} | {} | branches={}",
                request.kind, request.boundary_title, request.branch_count
            ));
            if let Some(reason) = request.stop_reasons.first() {
                lines.push(format!("    stop: {reason}"));
            }
            if let Some(example) = request.examples.first() {
                lines.push(format!("    example: {example}"));
            }
            lines.extend(render_campaign_strategy_context_v1(request));
            lines.push(format!("    needed: {}", request.suggested_action));
            if let Some(next_step) = campaign_strategy_next_step_v1(&request.kind) {
                lines.push(format!("    next: {next_step}"));
            }
            lines.extend(render_campaign_intervention_details_v2(report, request));
        }
    }
    if !report.active.is_empty() {
        lines.push(String::new());
        lines.push("Top active:".to_string());
        let shown = report
            .active
            .iter()
            .take(branch_examples)
            .collect::<Vec<_>>();
        let baseline = shown.first().copied();
        for (index, branch) in shown.into_iter().enumerate() {
            lines.push(format!(
                "  {}. {} | {} | choices: {}{}",
                index + 1,
                render_campaign_branch_state(branch),
                branch.frontier_title,
                render_compact_choice_path(&branch.choice_labels),
                render_campaign_branch_diff_suffix_v1(branch, baseline, index)
            ));
        }
    }
    if !report.frozen.is_empty() {
        lines.push(String::new());
        lines.push("Frozen examples:".to_string());
        let shown = report
            .frozen
            .iter()
            .take(branch_examples)
            .collect::<Vec<_>>();
        let baseline = shown.first().copied();
        for (index, branch) in shown.into_iter().enumerate() {
            lines.push(format!(
                "  {}. {} | {} | choices: {}{}",
                index + 1,
                render_campaign_branch_state(branch),
                branch.frontier_title,
                render_compact_choice_path(&branch.choice_labels),
                render_campaign_branch_diff_suffix_v1(branch, baseline, index)
            ));
        }
    }
    lines.join("\n")
}

fn render_campaign_final_boss_failure_summary_v1(
    report: &BranchCampaignReportV1,
    branch_examples: usize,
) -> Option<String> {
    let failures = report
        .abandoned
        .iter()
        .filter(|branch| {
            branch.frontier_title == "Combat"
                && branch
                    .summary
                    .as_ref()
                    .is_some_and(|summary| summary.act == 3 && summary.floor >= 48)
        })
        .collect::<Vec<_>>();
    if failures.is_empty() {
        return None;
    }

    let mut boss_counts = BTreeMap::<String, usize>::new();
    let mut hp_min = i32::MAX;
    let mut hp_max = i32::MIN;
    let mut deck_min = usize::MAX;
    let mut deck_max = usize::MIN;
    for branch in &failures {
        if let Some(summary) = branch.summary.as_ref() {
            let boss = if summary.boss.is_empty() {
                "unknown".to_string()
            } else {
                summary.boss.clone()
            };
            *boss_counts.entry(boss).or_default() += 1;
            hp_min = hp_min.min(summary.hp);
            hp_max = hp_max.max(summary.hp);
            deck_min = deck_min.min(summary.deck_count);
            deck_max = deck_max.max(summary.deck_count);
        }
    }

    let bosses = boss_counts
        .into_iter()
        .map(|(boss, count)| format!("{boss}={count}"))
        .collect::<Vec<_>>()
        .join(" ");
    let examples = failures
        .iter()
        .take(branch_examples.max(1).min(3))
        .map(|branch| render_campaign_branch_state(branch))
        .collect::<Vec<_>>()
        .join(" | ");

    Some(format!(
        "Final boss failures: abandoned={} bosses=[{}] hp={}..{} deck={}..{} examples=[{}]",
        failures.len(),
        bosses,
        hp_min,
        hp_max,
        deck_min,
        deck_max,
        examples
    ))
}

fn render_campaign_boss_relic_coverage_v1(report: &BranchCampaignReportV1) -> Option<String> {
    let active = campaign_boss_relic_lineage_counts_v1(&report.active);
    let frozen = campaign_boss_relic_lineage_counts_v1(&report.frozen);
    let abandoned = campaign_boss_relic_lineage_counts_v1(&report.abandoned);
    if active.is_empty() && frozen.is_empty() && abandoned.is_empty() {
        return None;
    }

    let mut furthest = BTreeMap::<String, (u8, i32)>::new();
    for branch in report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.abandoned.iter())
        .chain(report.victories.iter())
        .chain(report.dead.iter())
        .chain(report.stuck.iter())
    {
        let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
            continue;
        };
        let Some(summary) = branch.summary.as_ref() else {
            continue;
        };
        let progress = (summary.act, summary.floor);
        furthest
            .entry(lineage)
            .and_modify(|existing| {
                if progress > *existing {
                    *existing = progress;
                }
            })
            .or_insert(progress);
    }

    Some(format!(
        "Boss relic coverage: active=[{}] frozen=[{}] abandoned=[{}] furthest=[{}]",
        render_string_counts_v1(&active),
        render_string_counts_v1(&frozen),
        render_string_counts_v1(&abandoned),
        furthest
            .into_iter()
            .map(|(lineage, (act, floor))| format!("{lineage}=A{act}F{floor}"))
            .collect::<Vec<_>>()
            .join(" ")
    ))
}

fn format_seconds_1dp_v1(ms: u64) -> String {
    format!("{:.1}s", ms as f64 / 1000.0)
}

fn format_bp(value: i32) -> String {
    format!("{:.2}", f64::from(value) / 100.0)
}

fn render_combat_retry_ledger_v1(ledger: &BranchCampaignCombatRetryLedgerV1) -> String {
    ledger
        .boss_gate_attempts
        .iter()
        .map(|entry| {
            format!(
                "A{}F{}={}/{}",
                entry.act, entry.floor, entry.attempts, BOSS_GATE_RETRY_ATTEMPTS_PER_GATE
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

struct CampaignUnspentGoldPressureV1 {
    count: usize,
    max_gold: i32,
    cause_counts: String,
    example: String,
}

struct CampaignBossMechanicPressureV1 {
    boss_counts: String,
    signal_counts: String,
    example: String,
}

fn campaign_boss_mechanic_pressure_v1(
    report: &BranchCampaignReportV1,
) -> Option<CampaignBossMechanicPressureV1> {
    let branches = report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.victories.iter())
        .chain(report.abandoned.iter())
        .chain(report.stuck.iter())
        .chain(report.dead.iter())
        .filter(|branch| branch_has_boss_mechanic_pressure_v1(branch))
        .collect::<Vec<_>>();
    if branches.is_empty() {
        return None;
    }

    let mut boss_counts = BTreeMap::<String, usize>::new();
    let mut signal_counts = BTreeMap::<String, usize>::new();
    for branch in &branches {
        let Some(summary) = branch.summary.as_ref() else {
            continue;
        };
        *boss_counts.entry(summary.boss.clone()).or_default() += 1;
        for signal in &summary.boss_pressure {
            *signal_counts.entry(signal.clone()).or_default() += 1;
        }
    }

    let example = branches
        .iter()
        .max_by(|left, right| {
            boss_mechanic_pressure_key_v1(left).cmp(&boss_mechanic_pressure_key_v1(right))
        })
        .map(|branch| {
            let summary = branch
                .summary
                .as_ref()
                .expect("filtered branch has summary");
            format!(
                "A{}F{} HP {}/{} deck {} boss={} | {}",
                summary.act,
                summary.floor,
                summary.hp,
                summary.max_hp,
                summary.deck_count,
                summary.boss,
                summary.boss_pressure.join(" ")
            )
        })
        .unwrap_or_default();

    Some(CampaignBossMechanicPressureV1 {
        boss_counts: render_string_count_map_v1(&boss_counts, usize::MAX),
        signal_counts: render_string_count_map_v1(&signal_counts, 8),
        example,
    })
}

fn branch_has_boss_mechanic_pressure_v1(branch: &BranchCampaignBranchV1) -> bool {
    let Some(summary) = branch.summary.as_ref() else {
        return false;
    };
    !summary.boss.is_empty()
        && !summary.boss_pressure.is_empty()
        && summary.floor >= boss_approach_floor_v1(summary.act)
}

fn render_campaign_combat_lab_probe_summary_v1(
    report: &BranchCampaignReportV1,
) -> Option<Vec<String>> {
    let probes = report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.victories.iter())
        .chain(report.abandoned.iter())
        .chain(report.stuck.iter())
        .chain(report.dead.iter())
        .flat_map(|branch| branch.combat_lab_probes.iter())
        .collect::<Vec<_>>();
    if probes.is_empty() {
        return None;
    }

    let mut kind_counts = BTreeMap::<String, usize>::new();
    let mut result_counts = BTreeMap::<String, usize>::new();
    for probe in &probes {
        *kind_counts.entry(probe.kind.clone()).or_default() += 1;
        *result_counts.entry(probe.result.clone()).or_default() += 1;
    }
    let example = probes
        .iter()
        .find(|probe| probe.kind == "current_act_boss_preview")
        .or_else(|| probes.first())
        .expect("non-empty probe list");

    let mut lines = vec![
        format!(
            "Combat lab probes: {} {}",
            render_string_count_map_v1(&kind_counts, 6),
            render_string_count_map_v1(&result_counts, 6)
        ),
        format!(
            "  probe example: boss={} source={} boundary={} result={}",
            example.boss.as_deref().unwrap_or("unknown"),
            example.source,
            example.boundary,
            example.result
        ),
    ];
    if !example.diagnosis.is_empty() {
        lines.push(format!(
            "  probe diagnosis: {}/{} confidence={} signals={}",
            example.diagnosis.outcome_class,
            example.diagnosis.search_reason,
            example.diagnosis.confidence,
            render_probe_signal_list_v1(&example.diagnosis.signals)
        ));
    }
    Some(lines)
}

fn render_probe_signal_list_v1(signals: &[String]) -> String {
    if signals.is_empty() {
        "-".to_string()
    } else {
        signals.join(",")
    }
}

fn boss_mechanic_pressure_key_v1(branch: &BranchCampaignBranchV1) -> (i32, i32, i32) {
    branch
        .summary
        .as_ref()
        .map(|summary| {
            (
                summary.floor,
                summary.boss_pressure.len() as i32,
                summary.hp,
            )
        })
        .unwrap_or((0, 0, 0))
}

fn render_string_count_map_v1(counts: &BTreeMap<String, usize>, limit: usize) -> String {
    counts
        .iter()
        .take(limit)
        .map(|(label, count)| format!("{label}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn campaign_unspent_gold_pressure_v1(
    report: &BranchCampaignReportV1,
) -> Option<CampaignUnspentGoldPressureV1> {
    let pressured = report
        .active
        .iter()
        .chain(report.frozen.iter())
        .filter(|branch| branch_has_unspent_gold_pressure_v1(branch))
        .collect::<Vec<_>>();
    if pressured.is_empty() {
        return None;
    }
    let max_gold = pressured
        .iter()
        .filter_map(|branch| branch.summary.as_ref().map(|summary| summary.gold))
        .max()
        .unwrap_or(0);
    let cause_counts = render_unspent_gold_cause_counts_v1(&pressured);
    let example = pressured
        .iter()
        .max_by(|left, right| {
            unspent_gold_pressure_key_v1(left).cmp(&unspent_gold_pressure_key_v1(right))
        })
        .map(|branch| {
            let summary = branch
                .summary
                .as_ref()
                .expect("filtered branch has summary");
            format!(
                "A{}F{} gold {} cause={} | {}",
                summary.act,
                summary.floor,
                summary.gold,
                branch_unspent_gold_pressure_cause_v1(branch),
                render_compact_choice_path(&branch.choice_labels)
            )
        })
        .unwrap_or_default();
    Some(CampaignUnspentGoldPressureV1 {
        count: pressured.len(),
        max_gold,
        cause_counts,
        example,
    })
}

fn branch_has_unspent_gold_pressure_v1(branch: &BranchCampaignBranchV1) -> bool {
    let Some(summary) = branch.summary.as_ref() else {
        return false;
    };
    summary.gold >= UNSPENT_GOLD_PRESSURE_THRESHOLD
        && summary.floor >= boss_approach_floor_v1(summary.act)
}

fn unspent_gold_pressure_key_v1(branch: &BranchCampaignBranchV1) -> (i32, i32) {
    branch
        .summary
        .as_ref()
        .map(|summary| (summary.gold, summary.floor))
        .unwrap_or((0, 0))
}

fn branch_unspent_gold_pressure_cause_v1(branch: &BranchCampaignBranchV1) -> &'static str {
    let has_buy = branch
        .choice_labels
        .iter()
        .any(|label| is_campaign_shop_buy_label_v1(label));
    if has_buy {
        return "purchase_seen_gold_still_high";
    }
    let has_shop_leave = branch
        .choice_labels
        .iter()
        .any(|label| is_campaign_shop_leave_label_v1(label));
    if has_shop_leave {
        return "shop_leave_without_purchase";
    }
    let has_shop_signal = branch
        .choice_labels
        .iter()
        .any(|label| label.to_ascii_lowercase().contains("shop"));
    if has_shop_signal {
        return "shop_seen_without_purchase";
    }
    "no_shop_action_seen"
}

fn is_campaign_shop_buy_label_v1(label: &str) -> bool {
    let normalized = label.trim().to_ascii_lowercase();
    normalized.starts_with("buy ") || normalized.contains("| buy ")
}

fn is_campaign_shop_leave_label_v1(label: &str) -> bool {
    let normalized = label.to_ascii_lowercase();
    normalized.contains("leave shop")
        || normalized.contains("auto leave shop")
        || normalized.contains("decline selected shop purchase portfolio")
}

fn render_unspent_gold_cause_counts_v1(branches: &[&BranchCampaignBranchV1]) -> String {
    let mut counts = BTreeMap::<&'static str, usize>::new();
    for branch in branches {
        *counts
            .entry(branch_unspent_gold_pressure_cause_v1(branch))
            .or_default() += 1;
    }
    [
        "no_shop_action_seen",
        "shop_leave_without_purchase",
        "purchase_seen_gold_still_high",
        "shop_seen_without_purchase",
    ]
    .into_iter()
    .filter_map(|cause| counts.get(cause).map(|count| format!("{cause}={count}")))
    .collect::<Vec<_>>()
    .join(" ")
}

fn boss_approach_floor_v1(act: u8) -> i32 {
    match act {
        1 => 10,
        2 => 24,
        3 => 40,
        _ => i32::MAX,
    }
}

fn render_branch_pressure_examples_v1(examples: &[String]) -> String {
    unique_limited_strings(
        examples
            .iter()
            .map(|example| truncate_branch_pressure_example_v1(example)),
        3,
    )
    .join(" | ")
}

fn render_campaign_branch_examples_v1(
    branches: &[BranchCampaignBranchV1],
    max_examples: usize,
) -> String {
    unique_limited_strings(
        branches
            .iter()
            .map(render_campaign_discard_example_v1)
            .map(|example| truncate_branch_pressure_example_v1(&example)),
        max_examples,
    )
    .join(" | ")
}

fn render_campaign_branch_stop_reasons_v1(
    branches: &[BranchCampaignBranchV1],
    max_examples: usize,
) -> String {
    unique_limited_strings(
        branches
            .iter()
            .map(|branch| branch.stop_reason.trim())
            .filter(|reason| !reason.is_empty())
            .map(truncate_branch_pressure_example_v1),
        max_examples,
    )
    .join(" | ")
}

fn render_campaign_choice_coverage_v1(report: &BranchCampaignReportV1) -> Option<String> {
    if report.active.is_empty() && report.frozen.is_empty() {
        return None;
    }
    let active_first = render_campaign_choice_count_summary_v1(
        report
            .active
            .iter()
            .filter_map(campaign_branch_first_choice_v1),
    );
    let active_latest = render_campaign_choice_count_summary_v1(
        report
            .active
            .iter()
            .filter_map(campaign_branch_latest_choice_v1),
    );
    let frozen_first = render_campaign_choice_count_summary_v1(
        report
            .frozen
            .iter()
            .filter_map(campaign_branch_first_choice_v1),
    );
    let frozen_latest = render_campaign_choice_count_summary_v1(
        report
            .frozen
            .iter()
            .filter_map(campaign_branch_latest_choice_v1),
    );
    Some(format!(
        "Choice coverage: active_first=[{}] active_latest=[{}] frozen_first=[{}] frozen_latest=[{}]",
        active_first, active_latest, frozen_first, frozen_latest
    ))
}

fn render_campaign_choice_count_summary_v1<'a>(choices: impl Iterator<Item = &'a str>) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for choice in choices {
        let compact_choice = compact_campaign_choice_label_metadata_v1(choice);
        let choice = compact_choice.trim();
        if choice.is_empty() {
            continue;
        }
        *counts.entry(choice.to_string()).or_default() += 1;
    }
    if counts.is_empty() {
        return "-".to_string();
    }
    let mut entries = counts.into_iter().collect::<Vec<_>>();
    entries.sort_by(|(left_label, left_count), (right_label, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_label.cmp(right_label))
    });
    entries
        .into_iter()
        .take(3)
        .map(|(label, count)| format!("{}={count}", truncate_campaign_diff_label_v1(&label)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn campaign_branch_first_choice_v1(branch: &BranchCampaignBranchV1) -> Option<&str> {
    branch.choice_labels.first().map(String::as_str)
}

fn campaign_branch_latest_choice_v1(branch: &BranchCampaignBranchV1) -> Option<&str> {
    branch.choice_labels.last().map(String::as_str)
}

fn render_campaign_branch_diff_suffix_v1(
    branch: &BranchCampaignBranchV1,
    baseline: Option<&BranchCampaignBranchV1>,
    index: usize,
) -> String {
    if index == 0 {
        return String::new();
    }
    let Some(baseline) = baseline else {
        return String::new();
    };
    let mut parts = Vec::new();
    if let Some(choice_diff) = render_campaign_choice_diff_v1(branch, baseline) {
        parts.push(format!("choices {choice_diff}"));
    }
    if let (Some(summary), Some(base_summary)) =
        (branch.summary.as_ref(), baseline.summary.as_ref())
    {
        if summary.formation_stage != base_summary.formation_stage {
            parts.push(format!(
                "stage {}->{}",
                base_summary.formation_stage, summary.formation_stage
            ));
        }
        if let Some(diff) = render_string_set_diff_v1(
            &summary.formation_strengths,
            &base_summary.formation_strengths,
        ) {
            parts.push(format!("strengths {diff}"));
        }
        if let Some(diff) =
            render_string_set_diff_v1(&summary.formation_needs, &base_summary.formation_needs)
        {
            parts.push(format!("needs {diff}"));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" | diff: {}", parts.join("; "))
    }
}

fn render_campaign_choice_diff_v1(
    branch: &BranchCampaignBranchV1,
    baseline: &BranchCampaignBranchV1,
) -> Option<String> {
    let mut additions = Vec::new();
    let max_len = branch.choice_labels.len().max(baseline.choice_labels.len());
    for index in 0..max_len {
        let current = branch.choice_labels.get(index);
        let base = baseline.choice_labels.get(index);
        if current == base {
            continue;
        }
        if let Some(label) = current {
            let label = compact_campaign_choice_label_metadata_v1(label);
            additions.push(format!("+{}", truncate_campaign_diff_label_v1(&label)));
        }
        if additions.len() >= 3 {
            break;
        }
    }
    if additions.is_empty() {
        None
    } else {
        Some(additions.join(" "))
    }
}

fn render_string_set_diff_v1(current: &[String], baseline: &[String]) -> Option<String> {
    let mut added = current
        .iter()
        .filter(|value| !baseline.contains(value))
        .cloned()
        .collect::<Vec<_>>();
    let mut removed = baseline
        .iter()
        .filter(|value| !current.contains(value))
        .cloned()
        .collect::<Vec<_>>();
    added.sort();
    removed.sort();
    let mut parts = Vec::new();
    parts.extend(added.into_iter().take(3).map(|value| format!("+{value}")));
    parts.extend(removed.into_iter().take(3).map(|value| format!("-{value}")));
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn truncate_campaign_diff_label_v1(value: &str) -> String {
    const MAX_CHARS: usize = 48;
    if value.chars().count() <= MAX_CHARS {
        return value.to_string();
    }
    let prefix = value
        .chars()
        .take(MAX_CHARS.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

fn render_campaign_victory_quality_lines_v1(
    report: &BranchCampaignReportV1,
) -> Option<Vec<String>> {
    let first = report.victories.first()?;
    let best = report
        .victories
        .iter()
        .max_by(|left, right| {
            victory_quality_key_v1(left)
                .cmp(&victory_quality_key_v1(right))
                .then_with(|| left.branch_id.cmp(&right.branch_id).reverse())
        })
        .unwrap_or(first);

    let mut lines = Vec::new();
    if report.victories.len() == 1 || first.branch_id == best.branch_id {
        lines.push(render_campaign_victory_line_v1("Victory", first));
    } else {
        lines.push(render_campaign_victory_line_v1("First victory", first));
        lines.push(render_campaign_victory_line_v1("Best victory", best));
    }
    Some(lines)
}

fn render_campaign_victory_line_v1(label: &str, branch: &BranchCampaignBranchV1) -> String {
    format!(
        "{label}: {} | choices: {}",
        render_campaign_branch_state(branch),
        render_compact_choice_path(&branch.choice_labels)
    )
}

fn victory_quality_key_v1(branch: &BranchCampaignBranchV1) -> (i32, i32, i32) {
    branch
        .summary
        .as_ref()
        .map(|summary| {
            let hp_percent = if summary.max_hp > 0 {
                (summary.hp.max(0) * 1000) / summary.max_hp
            } else {
                1000
            };
            (hp_percent, summary.hp, summary.gold)
        })
        .unwrap_or((0, 0, 0))
}

fn unique_limited_strings<I>(items: I, limit: usize) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut result = Vec::new();
    for item in items {
        if result.len() >= limit {
            break;
        }
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

fn truncate_branch_pressure_example_v1(value: &str) -> String {
    const MAX_CHARS: usize = 96;
    let parts = value
        .split(" -> ")
        .map(compact_campaign_choice_label_metadata_v1)
        .collect::<Vec<_>>();
    let compressed = if parts.len() > 4 {
        format!(
            "{} -> {} -> ... -> {}",
            parts[0],
            parts[1],
            parts.last().map(String::as_str).unwrap_or_default()
        )
    } else {
        parts.join(" -> ")
    };
    if compressed.chars().count() <= MAX_CHARS {
        return compressed;
    }
    let prefix = compressed
        .chars()
        .take(MAX_CHARS.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

fn render_campaign_strategy_context_v1(request: &BranchCampaignStrategyRequestV1) -> Vec<String> {
    let mut lines = Vec::new();
    if request.act > 0 || request.floor > 0 {
        lines.push(format!("    context: A{}F{}", request.act, request.floor));
    }
    if let Some(offer) = &request.next_card_reward_offer {
        if !offer.is_empty() {
            lines.push(format!("    next reward offer: {}", offer.join(" | ")));
        }
    }
    for detail in request.boundary_details.iter().take(3) {
        lines.push(format!("    detail: {detail}"));
    }
    lines
}

fn campaign_report_stop_needs_immediate_intervention_v1(report: &BranchCampaignReportV1) -> bool {
    report.stop_reason == "needs_intervention"
        || (matches!(
            report.stop_reason.as_str(),
            "stuck" | "no_active_branch" | "no_progress"
        ) && report.active.is_empty()
            && report.frozen.is_empty())
}

fn merge_campaign_route_decisions_v1(
    summary: &mut BranchCampaignRouteEvidenceSummaryV1,
    decisions: &[BranchExperimentRouteDecisionV1],
) {
    for decision in decisions {
        add_campaign_route_decision_v1(summary, decision);
    }
}

fn merge_campaign_route_evidence_summary_v1(
    target: &mut BranchCampaignRouteEvidenceSummaryV1,
    incoming: BranchCampaignRouteEvidenceSummaryV1,
) {
    if incoming.decisions == 0 {
        return;
    }
    target.avg_elite_prep_bp = weighted_average_bp(
        target.avg_elite_prep_bp,
        target.decisions,
        incoming.avg_elite_prep_bp,
        incoming.decisions,
    );
    target.decisions = target.decisions.saturating_add(incoming.decisions);
    target.first_elite_forced = target
        .first_elite_forced
        .saturating_add(incoming.first_elite_forced);
    target.first_elite_optional = target
        .first_elite_optional
        .saturating_add(incoming.first_elite_optional);
    target.first_elite_none = target
        .first_elite_none
        .saturating_add(incoming.first_elite_none);
    target.rest_bailout = target.rest_bailout.saturating_add(incoming.rest_bailout);
    target.shop_bailout = target.shop_bailout.saturating_add(incoming.shop_bailout);
    target.underprepared_first_elite = target
        .underprepared_first_elite
        .saturating_add(incoming.underprepared_first_elite);
    for example in incoming.examples {
        if target.examples.len() >= 4 {
            break;
        }
        target.examples.push(example);
    }
    for example in incoming.underprepared_examples {
        if target.underprepared_examples.len() >= 4 {
            break;
        }
        target.underprepared_examples.push(example);
    }
}

fn add_campaign_route_decision_v1(
    summary: &mut BranchCampaignRouteEvidenceSummaryV1,
    decision: &BranchExperimentRouteDecisionV1,
) {
    summary.avg_elite_prep_bp = weighted_average_bp(
        summary.avg_elite_prep_bp,
        summary.decisions,
        decision.elite_prep_bp,
        1,
    );
    summary.decisions = summary.decisions.saturating_add(1);
    if decision.first_elite.paths_with_first_elite == 0 {
        summary.first_elite_none = summary.first_elite_none.saturating_add(1);
    } else if decision.first_elite.forced {
        summary.first_elite_forced = summary.first_elite_forced.saturating_add(1);
    } else if decision.first_elite.optional {
        summary.first_elite_optional = summary.first_elite_optional.saturating_add(1);
    }
    if decision.first_elite.can_bail_to_rest_before {
        summary.rest_bailout = summary.rest_bailout.saturating_add(1);
    }
    if decision.first_elite.can_bail_to_shop_before {
        summary.shop_bailout = summary.shop_bailout.saturating_add(1);
    }
    if route_decision_has_underprepared_first_elite_v1(decision) {
        summary.underprepared_first_elite = summary.underprepared_first_elite.saturating_add(1);
    }
    if summary.examples.len() < 4 {
        summary.examples.push(BranchCampaignRouteEvidenceExampleV1 {
            target: decision.target.clone(),
            first_elite: render_branch_campaign_first_elite_evidence_v1(decision),
            elite_prep_bp: decision.elite_prep_bp,
        });
    }
    if route_decision_has_underprepared_first_elite_v1(decision)
        && summary.underprepared_examples.len() < 4
    {
        summary
            .underprepared_examples
            .push(BranchCampaignRouteEvidenceExampleV1 {
                target: decision.target.clone(),
                first_elite: render_branch_campaign_first_elite_evidence_v1(decision),
                elite_prep_bp: decision.elite_prep_bp,
            });
    }
}

fn weighted_average_bp(
    left_avg: i32,
    left_count: usize,
    right_avg: i32,
    right_count: usize,
) -> i32 {
    let total_count = left_count.saturating_add(right_count);
    if total_count == 0 {
        return 0;
    }
    let total = i64::from(left_avg) * left_count as i64 + i64::from(right_avg) * right_count as i64;
    (total / total_count as i64) as i32
}

fn route_decision_has_underprepared_first_elite_v1(
    decision: &BranchExperimentRouteDecisionV1,
) -> bool {
    decision.first_elite.paths_with_first_elite > 0
        && decision.first_elite.forced
        && !decision.first_elite.can_bail_to_rest_before
        && !decision.first_elite.can_bail_to_shop_before
        && decision.first_elite.max_hallway_fights_before < 2
}

fn render_branch_campaign_first_elite_evidence_v1(
    decision: &BranchExperimentRouteDecisionV1,
) -> String {
    let first_elite = &decision.first_elite;
    if first_elite.paths_with_first_elite == 0 {
        return "none".to_string();
    }
    let kind = if first_elite.forced {
        "forced"
    } else if first_elite.optional {
        "optional"
    } else {
        "present"
    };
    format!(
        "{kind} hallways={}-{} fires={}-{} shops={}-{} rest_bailout={} shop_bailout={}",
        first_elite.min_hallway_fights_before,
        first_elite.max_hallway_fights_before,
        first_elite.min_fires_before,
        first_elite.max_fires_before,
        first_elite.min_shops_before,
        first_elite.max_shops_before,
        first_elite.can_bail_to_rest_before,
        first_elite.can_bail_to_shop_before
    )
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
    let mut parent_results = Vec::new();
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
        )? {
            Ok(result) => parent_results.push(Some(result)),
            Err(request) => {
                parent_results.push(None);
                retry_requests.push(request);
            }
        }
    }
    let retry_results = run_campaign_parent_retry_passes_parallel_v1(parents, retry_requests)?;
    for (parent_index, parent_result) in retry_results {
        if let Some(slot) = parent_results.get_mut(parent_index) {
            *slot = Some(parent_result);
        }
    }

    for (parent_index, parent_result) in parent_results.into_iter().enumerate() {
        let parent_result = parent_result.ok_or_else(|| {
            format!(
                "internal error: missing branch campaign parent result for parent {}",
                parent_index + 1
            )
        })?;
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
        max_frozen_exact_sessions: config.max_active.saturating_mul(2),
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

fn append_limited_frozen_v1(
    frozen: &mut Vec<BranchCampaignBranchV1>,
    new_frozen: Vec<BranchCampaignBranchV1>,
    max_frozen: usize,
    discarded_count: &mut usize,
    discarded_examples: &mut Vec<String>,
) -> usize {
    let mut added = 0usize;
    for branch in new_frozen {
        if let Some(existing_index) = frozen.iter().position(|existing| {
            campaign_branch_quality_key_v1(existing) == campaign_branch_quality_key_v1(&branch)
        }) {
            if campaign_branch_retention_key_v1(&branch)
                > campaign_branch_retention_key_v1(&frozen[existing_index])
            {
                let displaced = std::mem::replace(&mut frozen[existing_index], branch);
                record_campaign_duplicate_merge_v1(&displaced, discarded_count, discarded_examples);
                added = added.saturating_add(1);
            } else {
                record_campaign_duplicate_merge_v1(&branch, discarded_count, discarded_examples);
            }
            continue;
        }

        if frozen.len() < max_frozen {
            frozen.push(branch);
            added = added.saturating_add(1);
            continue;
        }

        let Some(worst_index) = frozen_replacement_index_v1(frozen, &branch) else {
            record_campaign_discard_v1(&branch, discarded_count, discarded_examples);
            continue;
        };
        let worst_branch = &frozen[worst_index];
        if campaign_branch_retention_key_v1(&branch)
            > campaign_branch_retention_key_v1(worst_branch)
            || branch_introduces_new_boss_relic_lineage_v1(frozen, &branch)
        {
            let displaced = std::mem::replace(&mut frozen[worst_index], branch);
            record_campaign_discard_v1(&displaced, discarded_count, discarded_examples);
            added = added.saturating_add(1);
        } else {
            record_campaign_discard_v1(&branch, discarded_count, discarded_examples);
        }
    }
    added
}

fn frozen_replacement_index_v1(
    frozen: &[BranchCampaignBranchV1],
    incoming: &BranchCampaignBranchV1,
) -> Option<usize> {
    let lineage_counts = campaign_boss_relic_lineage_counts_v1(frozen);
    if branch_introduces_new_boss_relic_lineage_v1(frozen, incoming) {
        if let Some((index, _)) = frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| {
                branch_is_replaceable_without_losing_boss_relic_lineage_v1(branch, &lineage_counts)
            })
            .min_by(|(_, left), (_, right)| {
                campaign_branch_retention_key_v1(left).cmp(&campaign_branch_retention_key_v1(right))
            })
        {
            return Some(index);
        }
        return None;
    }

    frozen
        .iter()
        .enumerate()
        .filter(|(_, branch)| {
            branch_is_replaceable_without_losing_boss_relic_lineage_v1(branch, &lineage_counts)
        })
        .min_by(|(_, left), (_, right)| {
            campaign_branch_retention_key_v1(left).cmp(&campaign_branch_retention_key_v1(right))
        })
        .map(|(index, _)| index)
}

fn branch_is_replaceable_without_losing_boss_relic_lineage_v1(
    branch: &BranchCampaignBranchV1,
    lineage_counts: &BTreeMap<String, usize>,
) -> bool {
    let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
        return true;
    };
    lineage_counts.get(&lineage).copied().unwrap_or_default() > 1
}

fn branch_introduces_new_boss_relic_lineage_v1(
    frozen: &[BranchCampaignBranchV1],
    incoming: &BranchCampaignBranchV1,
) -> bool {
    let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(incoming) else {
        return false;
    };
    !frozen
        .iter()
        .any(|branch| campaign_branch_boss_relic_lineage_key_v1(branch).as_ref() == Some(&lineage))
}

fn record_campaign_discard_v1(
    branch: &BranchCampaignBranchV1,
    discarded_count: &mut usize,
    discarded_examples: &mut Vec<String>,
) {
    *discarded_count = discarded_count.saturating_add(1);
    if discarded_examples.len() < 6 {
        discarded_examples.push(render_campaign_discard_example_v1(branch));
    }
}

fn record_campaign_duplicate_merge_v1(
    branch: &BranchCampaignBranchV1,
    discarded_count: &mut usize,
    discarded_examples: &mut Vec<String>,
) {
    *discarded_count = discarded_count.saturating_add(1);
    if discarded_examples.len() < 6 {
        discarded_examples.push(format!(
            "merged duplicate: {}",
            render_campaign_discard_example_v1(branch)
        ));
    }
}

fn merge_campaign_strategy_requests_v1(
    requests: Vec<BranchExperimentStrategyRequestV1>,
) -> Vec<BranchCampaignStrategyRequestV1> {
    let mut merged = BTreeMap::<(String, String, u8, i32), BranchCampaignStrategyRequestV1>::new();
    for request in requests {
        let key = (
            request.kind.clone(),
            request.boundary_title.clone(),
            request.act,
            request.floor,
        );
        merged
            .entry(key)
            .and_modify(|existing| {
                existing.branch_count = existing.branch_count.saturating_add(request.branch_count);
                if (request.act, request.floor) > (existing.act, existing.floor) {
                    existing.act = request.act;
                    existing.floor = request.floor;
                }
                if existing.next_card_reward_offer.is_none() {
                    existing.next_card_reward_offer = request.next_card_reward_offer.clone();
                }
                for example in &request.examples {
                    if existing.examples.len() < 4 && !existing.examples.contains(example) {
                        existing.examples.push(example.clone());
                    }
                }
                for reason in &request.stop_reasons {
                    if existing.stop_reasons.len() < 4 && !existing.stop_reasons.contains(reason) {
                        existing.stop_reasons.push(reason.clone());
                    }
                }
                for detail in &request.boundary_details {
                    if existing.boundary_details.len() < 8
                        && !existing.boundary_details.contains(detail)
                    {
                        existing.boundary_details.push(detail.clone());
                    }
                }
            })
            .or_insert_with(|| BranchCampaignStrategyRequestV1 {
                kind: request.kind.clone(),
                boundary_title: request.boundary_title,
                branch_count: request.branch_count,
                act: request.act,
                floor: request.floor,
                stop_reasons: request.stop_reasons.into_iter().take(4).collect(),
                examples: request.examples.into_iter().take(4).collect(),
                next_card_reward_offer: request.next_card_reward_offer,
                boundary_details: request.boundary_details.into_iter().take(8).collect(),
                suggested_action: campaign_suggested_action_v1(
                    &request.kind,
                    &request.suggested_action,
                ),
            });
    }
    merged.into_values().collect()
}

fn campaign_suggested_action_v1(kind: &str, suggested_action: &str) -> String {
    match kind {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => {
            "provide combat tactic or upstream route/reward strategy; raise budget only if search was clearly under-spent".to_string()
        }
        "card_reward_policy_gap" => {
            "provide reward family policy for this public offer and run context".to_string()
        }
        "event_strategy" => "provide event strategy for this event context".to_string(),
        "campfire_strategy" => {
            "provide campfire strategy for this deck and route context".to_string()
        }
        "boss_relic_strategy" => {
            "provide boss relic strategy for the current deck package".to_string()
        }
        "shop_strategy" => "provide shop strategy for this shop state".to_string(),
        "reward_claim_policy" => "provide reward claim policy for this context".to_string(),
        "route_policy_gap" => "provide route strategy for this map context".to_string(),
        _ => suggested_action.to_string(),
    }
}

fn merge_campaign_strategy_request_queue_v1(
    existing: Vec<BranchCampaignStrategyRequestV1>,
    incoming: Vec<BranchCampaignStrategyRequestV1>,
) -> Vec<BranchCampaignStrategyRequestV1> {
    let mut merged = BTreeMap::<(String, String, u8, i32), BranchCampaignStrategyRequestV1>::new();
    for mut request in existing.into_iter().chain(incoming) {
        request.suggested_action =
            campaign_suggested_action_v1(&request.kind, &request.suggested_action);
        let key = (
            request.kind.clone(),
            request.boundary_title.clone(),
            request.act,
            request.floor,
        );
        merged
            .entry(key)
            .and_modify(|current| {
                current.branch_count = current.branch_count.saturating_add(request.branch_count);
                if (request.act, request.floor) > (current.act, current.floor) {
                    current.act = request.act;
                    current.floor = request.floor;
                }
                if current.next_card_reward_offer.is_none() {
                    current.next_card_reward_offer = request.next_card_reward_offer.clone();
                }
                for reason in &request.stop_reasons {
                    if current.stop_reasons.len() < 4 && !current.stop_reasons.contains(reason) {
                        current.stop_reasons.push(reason.clone());
                    }
                }
                for example in &request.examples {
                    if current.examples.len() < 4 && !current.examples.contains(example) {
                        current.examples.push(example.clone());
                    }
                }
                for detail in &request.boundary_details {
                    if current.boundary_details.len() < 8
                        && !current.boundary_details.contains(detail)
                    {
                        current.boundary_details.push(detail.clone());
                    }
                }
            })
            .or_insert(request);
    }
    merged.into_values().collect()
}

fn campaign_strategy_requests_are_fatal_v1(
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
    strategy_requests: &[BranchCampaignStrategyRequestV1],
) -> bool {
    !strategy_requests.is_empty() && active.is_empty() && frozen.is_empty()
}

fn abandoned_branches_intervention_request_v1(
    abandoned: &[BranchCampaignBranchV1],
) -> Option<BranchCampaignStrategyRequestV1> {
    if abandoned.is_empty() {
        return None;
    }
    let examples = abandoned
        .iter()
        .map(|branch| {
            let choices = render_choice_path(&branch.choice_labels);
            if choices == "-" {
                render_campaign_branch_state(branch)
            } else {
                choices
            }
        })
        .take(4)
        .collect::<Vec<_>>();
    let stop_reasons = unique_limited_strings(
        abandoned
            .iter()
            .map(|branch| branch.stop_reason.trim())
            .filter(|reason| !reason.is_empty())
            .map(ToOwned::to_owned),
        4,
    );
    Some(BranchCampaignStrategyRequestV1 {
        kind: "combat_manual_or_budget".to_string(),
        boundary_title: "Combat".to_string(),
        branch_count: abandoned.len(),
        act: abandoned
            .iter()
            .filter_map(|branch| branch.summary.as_ref().map(|summary| summary.act))
            .max()
            .unwrap_or_default(),
        floor: abandoned
            .iter()
            .filter_map(|branch| branch.summary.as_ref().map(|summary| summary.floor))
            .max()
            .unwrap_or_default(),
        stop_reasons: if stop_reasons.is_empty() {
            vec!["all candidate route branches were abandoned".to_string()]
        } else {
            stop_reasons
        },
        examples,
        next_card_reward_offer: None,
        boundary_details: Vec::new(),
        suggested_action:
            "provide combat tactic or upstream route/reward strategy; raise budget only if search was clearly under-spent"
                .to_string(),
    })
}

fn leading_abandoned_combat_intervention_request_v1(
    frozen: &[BranchCampaignBranchV1],
    abandoned: &[BranchCampaignBranchV1],
) -> Option<BranchCampaignStrategyRequestV1> {
    let best_frozen_progress = frozen.iter().map(branch_progress_key).max();
    let best_abandoned_progress = abandoned
        .iter()
        .filter(|branch| is_combat_abandoned_branch_v1(branch))
        .map(branch_progress_key)
        .max()?;
    if best_frozen_progress.is_some_and(|progress| progress >= best_abandoned_progress) {
        return None;
    }

    let leading = abandoned
        .iter()
        .filter(|branch| {
            is_combat_abandoned_branch_v1(branch)
                && branch_progress_key(branch) == best_abandoned_progress
        })
        .cloned()
        .collect::<Vec<_>>();
    abandoned_branches_intervention_request_v1(&leading)
}

fn is_combat_abandoned_branch_v1(branch: &BranchCampaignBranchV1) -> bool {
    branch.status == BranchCampaignBranchStatusV1::Abandoned
        && normalized_campaign_boundary_title(&branch.frontier_title) == "combat"
}

fn render_campaign_intervention_details_v2(
    report: &BranchCampaignReportV1,
    request: &BranchCampaignStrategyRequestV1,
) -> Vec<String> {
    vec![
        format!(
            "    kind: {}",
            campaign_intervention_kind_v2(report, request)
        ),
        format!(
            "    tried: {}",
            campaign_intervention_tried_v2(report, request)
        ),
        format!(
            "    possible inputs: {}",
            campaign_intervention_options_v2(request)
        ),
    ]
}

fn campaign_intervention_kind_v2(
    report: &BranchCampaignReportV1,
    request: &BranchCampaignStrategyRequestV1,
) -> &'static str {
    match request.kind.as_str() {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => {
            if report
                .rounds
                .last()
                .map(|round| round.combat_budget_retries > 0)
                .unwrap_or(false)
            {
                "combat_unresolved_after_retry"
            } else {
                "combat_unresolved"
            }
        }
        "card_reward_policy_gap" => "card_reward_strategy_gap",
        "event_strategy" => "event_strategy_needed",
        "campfire_strategy" => "campfire_strategy_needed",
        "boss_relic_strategy" => "boss_relic_strategy_needed",
        "shop_strategy" => "shop_strategy_needed",
        "reward_claim_policy" => "reward_claim_strategy_needed",
        "route_policy_gap" => "route_strategy_gap",
        "engineering_issue" => "engineering_issue",
        _ => "strategy_needed",
    }
}

fn campaign_intervention_tried_v2(
    report: &BranchCampaignReportV1,
    request: &BranchCampaignStrategyRequestV1,
) -> String {
    match request.kind.as_str() {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => {
            let retries = report
                .rounds
                .last()
                .map(|round| round.combat_budget_retries)
                .unwrap_or(0);
            if retries > 0 {
                format!("campaign search budget; combat budget retry x{retries}")
            } else {
                "campaign search budget".to_string()
            }
        }
        "card_reward_policy_gap" => {
            "branch reward candidates; current autopick gate declined".to_string()
        }
        "event_strategy" => "event boundary detected; no narrow event policy accepted".to_string(),
        "campfire_strategy" => {
            "campfire options detected; no campfire priority accepted".to_string()
        }
        "shop_strategy" => "shop options detected; purchase portfolio did not resolve".to_string(),
        _ => "current campaign policy".to_string(),
    }
}

fn campaign_intervention_options_v2(request: &BranchCampaignStrategyRequestV1) -> &'static str {
    match request.kind.as_str() {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => {
            "switch macro branch | provide combat tactic | add upstream route/reward rule | raise retry budget only if under-spent"
        }
        "card_reward_policy_gap" => {
            "reward package rule | keep branching this reward family | force human judgment"
        }
        "event_strategy" => {
            "event rule | choose one event branch manually | blacklist this event branch"
        }
        "campfire_strategy" => {
            "smith/rest/recall rule | branch fewer smith targets | ask human at this campfire"
        }
        "shop_strategy" => {
            "buy/remove/leave rule | cap purchase portfolio | ask human at this shop"
        }
        "boss_relic_strategy" => {
            "boss relic package rule | preserve multiple relic branches | ask human"
        }
        "reward_claim_policy" => {
            "mark reward as safe claim | keep reward pending | ask human"
        }
        "route_policy_gap" => {
            "route rule for this context | provide one map choice | freeze this route family"
        }
        "engineering_issue" => {
            "fix simulator or command bug | rerun same seed | quarantine affected trace"
        }
        _ => "add a narrow strategy rule | keep branching | ask human",
    }
}

pub fn select_campaign_branches_v1(
    branches: Vec<BranchCampaignBranchV1>,
    max_active: usize,
    max_frozen: usize,
) -> BranchCampaignSelectionV1 {
    let mut active_candidates = Vec::new();
    let mut selection = BranchCampaignSelectionV1::default();
    for branch in branches {
        if campaign_stuck_branch_should_be_abandoned_for_combat_triage_v1(&branch) {
            selection.abandoned.push(branch);
            continue;
        }
        match branch.status {
            BranchCampaignBranchStatusV1::TerminalVictory => selection.victories.push(branch),
            BranchCampaignBranchStatusV1::TerminalDefeat => selection.dead.push(branch),
            BranchCampaignBranchStatusV1::Abandoned => selection.abandoned.push(branch),
            BranchCampaignBranchStatusV1::Stuck => selection.stuck.push(branch),
            BranchCampaignBranchStatusV1::Frozen | BranchCampaignBranchStatusV1::Active => {
                active_candidates.push(branch)
            }
        }
    }

    active_candidates.sort_by(compare_campaign_branches_for_active_v1);

    let mut retained_quality_keys = BTreeSet::new();
    for mut branch in active_candidates {
        let quality_key = campaign_branch_quality_key_v1(&branch);
        if !retained_quality_keys.insert(quality_key) {
            record_campaign_duplicate_merge_v1(
                &branch,
                &mut selection.discarded_count,
                &mut selection.discarded_examples,
            );
            continue;
        }

        if selection.active.len() < max_active
            && (campaign_branch_primary_active_eligible_v1(&branch) || selection.active.is_empty())
        {
            branch.status = BranchCampaignBranchStatusV1::Active;
            selection.active.push(branch);
        } else if selection.frozen.len() < max_frozen {
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            selection.frozen.push(branch);
        } else {
            selection.discarded_count = selection.discarded_count.saturating_add(1);
            if selection.discarded_examples.len() < 6 {
                selection
                    .discarded_examples
                    .push(render_campaign_discard_example_v1(&branch));
            }
        }
    }
    rebalance_active_progress_anchor_v1(&mut selection.active, &mut selection.frozen);
    rebalance_active_survival_anchor_v1(&mut selection.active, &mut selection.frozen);
    selection
}

fn select_campaign_branches_for_config_v1(
    branches: Vec<BranchCampaignBranchV1>,
    config: &BranchCampaignConfigV1,
) -> BranchCampaignSelectionV1 {
    let mut selection = select_campaign_branches_v1(branches, config.max_active, config.max_frozen);
    if config.active_lineage_diversity_slots > 0 {
        rebalance_active_lineage_diversity_v1(
            &mut selection.active,
            &mut selection.frozen,
            config.active_lineage_diversity_slots,
        );
    }
    selection
}

fn rebalance_active_lineage_diversity_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    target_unique_lineages: usize,
) -> usize {
    if target_unique_lineages == 0 || active.len() < 2 || frozen.is_empty() {
        return 0;
    }
    let mut swaps = rebalance_active_boss_relic_lineage_v1(active, frozen, target_unique_lineages);
    swaps = swaps.saturating_add(rebalance_active_lineage_spread_v1(
        active,
        frozen,
        target_unique_lineages,
        campaign_branch_first_lineage_key_v1,
    ));
    swaps = swaps.saturating_add(rebalance_active_unique_lineage_v1(
        active,
        frozen,
        target_unique_lineages,
        campaign_branch_path_lineage_key_v1,
    ));
    swaps
}

fn rebalance_active_boss_relic_lineage_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    target_lineage_slots: usize,
) -> usize {
    if target_lineage_slots == 0 || active.len() < 2 || frozen.is_empty() {
        return 0;
    }
    let available_lineage_count = campaign_boss_relic_lineage_counts_for_pool_v1(active, frozen)
        .len()
        .min(target_lineage_slots)
        .min(active.len());
    if available_lineage_count < 2 {
        return 0;
    }
    let max_per_lineage = active.len().div_ceil(available_lineage_count);
    let mut swaps = 0usize;
    loop {
        let active_lineages = campaign_boss_relic_lineage_counts_v1(active);
        let Some((overrepresented_key, overrepresented_count)) =
            active_lineages
                .iter()
                .max_by(|(left_key, left_count), (right_key, right_count)| {
                    left_count
                        .cmp(right_count)
                        .then_with(|| right_key.cmp(left_key))
                })
        else {
            break;
        };
        if *overrepresented_count <= max_per_lineage {
            break;
        }
        let Some((frozen_index, _)) = frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| {
                let Some(key) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
                    return false;
                };
                key != *overrepresented_key
                    && active_lineages.get(&key).copied().unwrap_or(0) < max_per_lineage
            })
            .min_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let Some((replace_index, _)) = active
            .iter()
            .enumerate()
            .filter(|(_, branch)| {
                campaign_branch_boss_relic_lineage_key_v1(branch).as_ref()
                    == Some(overrepresented_key)
            })
            .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let mut promoted = frozen.remove(frozen_index);
        promoted.status = BranchCampaignBranchStatusV1::Active;
        let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
        demoted.status = BranchCampaignBranchStatusV1::Frozen;
        frozen.push(demoted);
        active.sort_by(compare_campaign_branches_for_active_v1);
        frozen.sort_by(compare_campaign_branches_for_promotion_v1);
        swaps = swaps.saturating_add(1);
    }
    swaps
}

fn rebalance_active_unique_lineage_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    target_unique_lineages: usize,
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> usize {
    let target_unique_lineages = target_unique_lineages.min(active.len());
    let mut swaps = 0usize;
    while campaign_active_lineage_count_v1(active, key_fn) < target_unique_lineages {
        let active_lineages = campaign_active_lineage_counts_v1(active, key_fn);
        let Some((frozen_index, _)) = frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| !active_lineages.contains_key(&key_fn(branch)))
            .min_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let Some((replace_index, _)) = active
            .iter()
            .enumerate()
            .filter(|(_, branch)| active_lineages.get(&key_fn(branch)).copied().unwrap_or(0) > 1)
            .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let mut promoted = frozen.remove(frozen_index);
        promoted.status = BranchCampaignBranchStatusV1::Active;
        let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
        demoted.status = BranchCampaignBranchStatusV1::Frozen;
        frozen.push(demoted);
        active.sort_by(compare_campaign_branches_for_active_v1);
        frozen.sort_by(compare_campaign_branches_for_promotion_v1);
        swaps = swaps.saturating_add(1);
    }
    swaps
}

fn rebalance_active_lineage_spread_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    target_lineage_slots: usize,
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> usize {
    if target_lineage_slots == 0 || active.len() < 2 || frozen.is_empty() {
        return 0;
    }
    let available_lineage_count =
        campaign_branch_lineage_counts_for_pool_v1(active, frozen, key_fn)
            .len()
            .min(target_lineage_slots)
            .min(active.len());
    if available_lineage_count < 2 {
        return 0;
    }
    let max_per_lineage = active.len().div_ceil(available_lineage_count);
    let mut swaps = 0usize;
    loop {
        let active_lineages = campaign_active_lineage_counts_v1(active, key_fn);
        let Some((overrepresented_key, overrepresented_count)) =
            active_lineages
                .iter()
                .max_by(|(left_key, left_count), (right_key, right_count)| {
                    left_count
                        .cmp(right_count)
                        .then_with(|| right_key.cmp(left_key))
                })
        else {
            break;
        };
        if *overrepresented_count <= max_per_lineage {
            break;
        }
        let Some((frozen_index, _)) = frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| {
                let key = key_fn(branch);
                key != *overrepresented_key
                    && active_lineages.get(&key).copied().unwrap_or(0) < max_per_lineage
            })
            .min_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let Some((replace_index, _)) = active
            .iter()
            .enumerate()
            .filter(|(_, branch)| key_fn(branch) == *overrepresented_key)
            .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let mut promoted = frozen.remove(frozen_index);
        promoted.status = BranchCampaignBranchStatusV1::Active;
        let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
        demoted.status = BranchCampaignBranchStatusV1::Frozen;
        frozen.push(demoted);
        active.sort_by(compare_campaign_branches_for_active_v1);
        frozen.sort_by(compare_campaign_branches_for_promotion_v1);
        swaps = swaps.saturating_add(1);
    }
    swaps
}

fn campaign_active_lineage_count_v1(
    active: &[BranchCampaignBranchV1],
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> usize {
    campaign_active_lineage_counts_v1(active, key_fn).len()
}

fn campaign_active_lineage_counts_v1(
    active: &[BranchCampaignBranchV1],
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for branch in active {
        *counts.entry(key_fn(branch)).or_insert(0) += 1;
    }
    counts
}

fn campaign_branch_lineage_counts_for_pool_v1(
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for branch in active.iter().chain(frozen.iter()) {
        *counts.entry(key_fn(branch)).or_insert(0) += 1;
    }
    counts
}

fn campaign_branch_first_lineage_key_v1(branch: &BranchCampaignBranchV1) -> String {
    branch
        .choice_labels
        .first()
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| branch.branch_id.clone())
}

fn campaign_branch_path_lineage_key_v1(branch: &BranchCampaignBranchV1) -> String {
    let first = branch
        .choice_labels
        .first()
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| branch.branch_id.clone());
    let Some(latest) = branch.choice_labels.last() else {
        return first;
    };
    if latest == &first || latest.trim().is_empty() {
        return first;
    }
    format!("{first} | latest={latest}")
}

fn campaign_branch_boss_relic_lineage_key_v1(branch: &BranchCampaignBranchV1) -> Option<String> {
    let relics = branch
        .choice_labels
        .iter()
        .filter_map(|label| campaign_boss_relic_label_v1(label))
        .collect::<Vec<_>>();
    (!relics.is_empty()).then(|| relics.join(">"))
}

fn campaign_boss_relic_label_v1(label: &str) -> Option<String> {
    let trimmed = label.trim();
    if BOSS_RELIC_CHOICE_LABELS_V1.contains(&trimmed) {
        Some(trimmed.to_string())
    } else if let Some(first_token) = trimmed.split_whitespace().next() {
        BOSS_RELIC_CHOICE_LABELS_V1
            .contains(&first_token)
            .then(|| first_token.to_string())
    } else {
        None
    }
}

fn campaign_boss_relic_lineage_counts_v1(
    branches: &[BranchCampaignBranchV1],
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for branch in branches {
        let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
            continue;
        };
        *counts.entry(lineage).or_insert(0) += 1;
    }
    counts
}

fn campaign_boss_relic_lineage_counts_for_pool_v1(
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for branch in active.iter().chain(frozen.iter()) {
        let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
            continue;
        };
        *counts.entry(lineage).or_insert(0) += 1;
    }
    counts
}

fn render_string_counts_v1(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn rebalance_active_progress_anchor_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
) -> bool {
    if active.len() < 2 || frozen.is_empty() {
        return false;
    }

    let Some((frozen_index, frozen_branch)) =
        frozen.iter().enumerate().max_by(|(_, left), (_, right)| {
            branch_progress_key(left)
                .cmp(&branch_progress_key(right))
                .then_with(|| compare_campaign_branches_for_active_v1(left, right).reverse())
        })
    else {
        return false;
    };
    let frozen_progress = branch_progress_key(frozen_branch);

    let duplicate_keys = active
        .iter()
        .map(campaign_branch_local_frontier_key_v1)
        .fold(BTreeMap::<String, usize>::new(), |mut counts, key| {
            *counts.entry(key).or_default() += 1;
            counts
        });

    let Some((replace_index, _)) = active
        .iter()
        .enumerate()
        .filter(|(_, branch)| {
            duplicate_keys
                .get(&campaign_branch_local_frontier_key_v1(branch))
                .copied()
                .unwrap_or(0)
                > 1
                && campaign_progress_is_clearly_ahead_v1(
                    frozen_progress,
                    branch_progress_key(branch),
                )
                && campaign_active_swap_respects_survival_v1(frozen_branch, branch)
                && campaign_progress_anchor_rank_close_enough_v1(frozen_branch, branch)
        })
        .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
    else {
        return false;
    };

    let mut promoted = frozen.remove(frozen_index);
    promoted.status = BranchCampaignBranchStatusV1::Active;
    let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
    demoted.status = BranchCampaignBranchStatusV1::Frozen;
    frozen.push(demoted);
    active.sort_by(compare_campaign_branches_for_active_v1);
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    true
}

fn campaign_progress_anchor_rank_close_enough_v1(
    candidate: &BranchCampaignBranchV1,
    replaced: &BranchCampaignBranchV1,
) -> bool {
    candidate
        .rank_key
        .saturating_add(PROGRESS_ANCHOR_MAX_RANK_LAG)
        >= replaced.rank_key
}

fn rebalance_active_survival_anchor_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
) -> bool {
    if active.is_empty() || frozen.is_empty() {
        return false;
    }

    let Some((replace_index, replace_hp)) = active
        .iter()
        .enumerate()
        .filter_map(|(idx, branch)| campaign_branch_hp_percent_v1(branch).map(|hp| (idx, hp)))
        .filter(|(_, hp)| *hp < SURVIVAL_ANCHOR_LOW_HP_PERCENT)
        .min_by_key(|(_, hp)| *hp)
    else {
        return false;
    };
    if campaign_branch_is_act_clear_transition_v1(&active[replace_index]) {
        return false;
    }

    let maybe_nearby = frozen
        .iter()
        .enumerate()
        .filter(|(_, branch)| !branch_is_rehydrated_checkpointed_combat_failure_v1(branch))
        .filter(|(_, branch)| {
            campaign_progress_is_nearby_v1(
                branch_progress_key(branch),
                branch_progress_key(&active[replace_index]),
            )
        })
        .filter_map(|(idx, branch)| {
            let hp = campaign_branch_hp_percent_v1(branch)?;
            (hp >= replace_hp.saturating_add(SURVIVAL_ANCHOR_NEARBY_MIN_HP_GAIN)
                && campaign_survival_anchor_respects_low_max_hp_risk_v1(
                    branch,
                    &active[replace_index],
                ))
            .then_some((idx, hp))
        })
        .max_by(|(left_idx, left_hp), (right_idx, right_hp)| {
            left_hp.cmp(right_hp).then_with(|| {
                campaign_branch_retention_key_v1(&frozen[*left_idx])
                    .cmp(&campaign_branch_retention_key_v1(&frozen[*right_idx]))
            })
        });

    let maybe_salvage = || {
        frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| !branch_is_rehydrated_checkpointed_combat_failure_v1(branch))
            .filter(|(_, branch)| {
                campaign_progress_is_survival_salvage_checkpoint_v1(
                    branch_progress_key(branch),
                    branch_progress_key(&active[replace_index]),
                )
            })
            .filter_map(|(idx, branch)| {
                let hp = campaign_branch_hp_percent_v1(branch)?;
                (campaign_branch_is_survival_salvage_v1(hp, replace_hp)
                    && campaign_survival_anchor_respects_low_max_hp_risk_v1(
                        branch,
                        &active[replace_index],
                    ))
                .then_some((idx, hp))
            })
            .max_by(|(left_idx, left_hp), (right_idx, right_hp)| {
                left_hp.cmp(right_hp).then_with(|| {
                    campaign_branch_retention_key_v1(&frozen[*left_idx])
                        .cmp(&campaign_branch_retention_key_v1(&frozen[*right_idx]))
                })
            })
    };

    let Some((frozen_index, _)) = maybe_nearby.or_else(maybe_salvage) else {
        return false;
    };

    let mut promoted = frozen.remove(frozen_index);
    promoted.status = BranchCampaignBranchStatusV1::Active;
    let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
    demoted.status = BranchCampaignBranchStatusV1::Frozen;
    frozen.push(demoted);
    active.sort_by(compare_campaign_branches_for_active_v1);
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    true
}

fn campaign_branch_is_survival_salvage_v1(candidate_hp: i32, replaced_hp: i32) -> bool {
    let healthy_salvage = candidate_hp >= SURVIVAL_ANCHOR_HEALTHY_SALVAGE_HP_PERCENT
        && candidate_hp >= replaced_hp.saturating_add(SURVIVAL_ANCHOR_HEALTHY_SALVAGE_HP_GAIN);
    let critical_salvage = replaced_hp < SURVIVAL_ANCHOR_CRITICAL_HP_PERCENT
        && candidate_hp >= SURVIVAL_ANCHOR_CRITICAL_SALVAGE_HP_PERCENT
        && candidate_hp >= replaced_hp.saturating_add(SURVIVAL_ANCHOR_CRITICAL_SALVAGE_HP_GAIN);
    healthy_salvage || critical_salvage
}

fn campaign_survival_anchor_respects_low_max_hp_risk_v1(
    candidate: &BranchCampaignBranchV1,
    replaced: &BranchCampaignBranchV1,
) -> bool {
    let (Some(candidate_summary), Some(replaced_summary)) =
        (candidate.summary.as_ref(), replaced.summary.as_ref())
    else {
        return true;
    };
    let candidate_max_hp = candidate_summary.max_hp.max(0);
    let replaced_max_hp = replaced_summary.max_hp.max(0);
    if candidate_max_hp == 0 || replaced_max_hp == 0 {
        return true;
    }
    if candidate_max_hp.saturating_mul(3) >= replaced_max_hp.saturating_mul(2) {
        return true;
    }
    candidate
        .rank_key
        .saturating_add(PROGRESS_ANCHOR_MAX_RANK_LAG)
        >= replaced.rank_key
}

fn campaign_branch_local_frontier_key_v1(branch: &BranchCampaignBranchV1) -> String {
    let (act, floor, _) = branch_progress_key(branch);
    format!(
        "a{act}f{floor}|{}",
        normalized_campaign_boundary_title(&branch.frontier_title)
    )
}

fn campaign_progress_is_clearly_ahead_v1(left: (u8, i32, i32), right: (u8, i32, i32)) -> bool {
    if left.0 > right.0 {
        return true;
    }
    left.0 == right.0 && left.1 >= right.1.saturating_add(2)
}

fn campaign_active_swap_respects_survival_v1(
    candidate: &BranchCampaignBranchV1,
    replaced: &BranchCampaignBranchV1,
) -> bool {
    if campaign_branch_is_act_clear_transition_v1(candidate)
        && !campaign_branch_is_act_clear_transition_v1(replaced)
    {
        return true;
    }

    let Some(candidate_hp_percent) = campaign_branch_hp_percent_v1(candidate) else {
        return true;
    };
    let Some(replaced_hp_percent) = campaign_branch_hp_percent_v1(replaced) else {
        return true;
    };
    let candidate_progress = branch_progress_key(candidate);
    let replaced_progress = branch_progress_key(replaced);
    if candidate_progress.0 == replaced_progress.0
        && candidate_progress.1 >= replaced_progress.1
        && candidate_progress.1.saturating_sub(replaced_progress.1) <= 8
        && candidate_hp_percent < SURVIVAL_ANCHOR_LOW_HP_PERCENT
        && campaign_branch_is_survival_salvage_v1(replaced_hp_percent, candidate_hp_percent)
    {
        return false;
    }
    if !campaign_progress_is_nearby_v1(candidate_progress, replaced_progress) {
        return true;
    }
    !(candidate_hp_percent < SURVIVAL_ANCHOR_LOW_HP_PERCENT
        && replaced_hp_percent
            >= candidate_hp_percent.saturating_add(SURVIVAL_ANCHOR_NEARBY_MIN_HP_GAIN))
}

fn campaign_branch_is_act_clear_transition_v1(branch: &BranchCampaignBranchV1) -> bool {
    let Some(summary) = branch.summary.as_ref() else {
        return false;
    };
    summary.act < 3
        && summary.floor >= act_boss_floor_v1(summary.act)
        && matches!(
            normalized_campaign_boundary_title(&branch.frontier_title).as_str(),
            "bossrelic" | "rewardscreen"
        )
}

fn campaign_progress_is_nearby_v1(left: (u8, i32, i32), right: (u8, i32, i32)) -> bool {
    left.0 == right.0 && (left.1 - right.1).abs() <= 2
}

fn campaign_progress_is_survival_salvage_checkpoint_v1(
    candidate: (u8, i32, i32),
    replaced: (u8, i32, i32),
) -> bool {
    candidate.0 == replaced.0
        && candidate.1 <= replaced.1
        && replaced.1.saturating_sub(candidate.1) <= 8
}

fn campaign_branch_hp_percent_v1(branch: &BranchCampaignBranchV1) -> Option<i32> {
    let summary = branch.summary.as_ref()?;
    if summary.max_hp <= 0 {
        return None;
    }
    Some(summary.hp.max(0).saturating_mul(100) / summary.max_hp)
}

fn campaign_stuck_branch_should_be_abandoned_for_combat_triage_v1(
    branch: &BranchCampaignBranchV1,
) -> bool {
    if branch.status != BranchCampaignBranchStatusV1::Stuck {
        return false;
    }
    if !normalized_campaign_boundary_title(&branch.frontier_title).starts_with("combat") {
        return false;
    }
    let stop = branch.stop_reason.to_ascii_lowercase();
    stop.contains("combat search")
        || stop.contains("search-combat")
        || stop.contains("hp-loss")
        || stop.contains("max_hp_loss")
        || stop.contains("high-stakes combat")
}

fn append_discarded_examples_v1(target: &mut Vec<String>, incoming: Vec<String>) {
    for example in incoming {
        if target.len() >= 6 {
            break;
        }
        if !target.contains(&example) {
            target.push(example);
        }
    }
}

fn render_campaign_discard_example_v1(branch: &BranchCampaignBranchV1) -> String {
    let choices = render_choice_path(&branch.choice_labels);
    if choices == "-" {
        render_campaign_branch_state(branch)
    } else {
        choices
    }
}

fn promote_frozen_to_active_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> usize {
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    let mut promoted = 0usize;
    while active.len() < max_active && !frozen.is_empty() {
        let require_primary_eligible = !active.is_empty();
        let Some(promote_index) = frozen.iter().position(|branch| {
            !branch_is_rehydrated_checkpointed_combat_failure_v1(branch)
                && (!require_primary_eligible || campaign_branch_primary_active_eligible_v1(branch))
        }) else {
            break;
        };
        let mut branch = frozen.remove(promote_index);
        branch.status = BranchCampaignBranchStatusV1::Active;
        active.push(branch);
        promoted = promoted.saturating_add(1);
    }
    promoted
}

fn campaign_branch_primary_active_eligible_v1(branch: &BranchCampaignBranchV1) -> bool {
    branch.rank_key >= 0
}

fn promote_rehydrated_combat_failures_to_active_on_stall_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> usize {
    if max_active == 0 || !active.is_empty() {
        return 0;
    }
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    let mut promoted = 0usize;
    while active.len() < max_active {
        let Some(promote_index) = frozen
            .iter()
            .position(branch_is_rehydrated_checkpointed_combat_failure_v1)
        else {
            break;
        };
        let mut branch = frozen.remove(promote_index);
        branch.status = BranchCampaignBranchStatusV1::Active;
        active.push(branch);
        promoted = promoted.saturating_add(1);
    }
    promoted
}

fn rebalance_active_with_stronger_frozen_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> usize {
    let mut total = 0usize;
    let max_iterations = active.len().saturating_add(frozen.len()).saturating_add(1);
    for _ in 0..max_iterations {
        let promoted = rebalance_active_with_stronger_frozen_once_v1(active, frozen, max_active);
        if promoted == 0 {
            break;
        }
        total = total.saturating_add(promoted);
    }
    total
}

fn rebalance_active_with_stronger_frozen_once_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> usize {
    if max_active == 0 || frozen.is_empty() {
        return 0;
    }
    if active.len() < max_active {
        return promote_frozen_to_active_v1(active, frozen, max_active);
    }
    if active.is_empty() {
        return 0;
    }

    if rebalance_active_survival_anchor_v1(active, frozen) {
        return 1;
    }

    let Some((worst_active_index, worst_active)) = active
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
    else {
        return 0;
    };
    let Some((best_frozen_index, best_frozen)) = frozen
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
    else {
        return 0;
    };

    if branch_is_rehydrated_checkpointed_combat_failure_v1(best_frozen)
        && active.iter().any(|branch| {
            campaign_progress_is_clearly_ahead_v1(
                branch_progress_key(branch),
                branch_progress_key(best_frozen),
            )
        })
    {
        return 0;
    }

    if active.iter().any(|branch| {
        campaign_branch_local_frontier_key_v1(branch)
            == campaign_branch_local_frontier_key_v1(best_frozen)
    }) && active.iter().any(|branch| {
        campaign_progress_is_clearly_ahead_v1(
            branch_progress_key(branch),
            branch_progress_key(best_frozen),
        )
    }) {
        return 0;
    }

    if !campaign_active_swap_respects_survival_v1(best_frozen, worst_active) {
        return 0;
    }

    if compare_campaign_branches_for_active_v1(best_frozen, worst_active)
        != std::cmp::Ordering::Less
    {
        return 0;
    }

    let mut promoted = frozen.remove(best_frozen_index);
    promoted.status = BranchCampaignBranchStatusV1::Active;
    let mut demoted = std::mem::replace(&mut active[worst_active_index], promoted);
    demoted.status = BranchCampaignBranchStatusV1::Frozen;
    frozen.push(demoted);
    active.sort_by(compare_campaign_branches_for_active_v1);
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    1
}

fn branch_is_rehydrated_checkpointed_combat_failure_v1(branch: &BranchCampaignBranchV1) -> bool {
    normalized_campaign_boundary_title(&branch.frontier_title).starts_with("combat")
        && branch
            .stop_reason
            .to_ascii_lowercase()
            .contains("rehydrated checkpointed")
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

fn campaign_refresh_branch_summary_from_session_v1(
    branch: &mut BranchCampaignBranchV1,
    session: &RunControlSession,
) {
    let Some(summary) = branch.summary.as_mut() else {
        return;
    };
    summary.act = session.run_state.act_num;
    summary.floor = session.run_state.floor_num;
    let (hp, max_hp) = session.visible_player_hp();
    summary.hp = hp;
    summary.max_hp = max_hp;
    summary.gold = session.run_state.gold;
    summary.deck_count = session.run_state.master_deck.len();
    summary.deck_key = campaign_deck_key_v1(session);
    summary.boss = branch_campaign_boss_label_v1(session);
    summary.boss_pressure = branch_campaign_boss_pressure_labels_v1(session);
    summary.run_debt = branch_campaign_run_debt_labels_v1(session);
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

fn campaign_deck_key_v1(session: &RunControlSession) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for card in &session.run_state.master_deck {
        *counts
            .entry(format!("{:?}+{}", card.id, card.upgrades))
            .or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(card, count)| format!("{card}x{count}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn branch_campaign_boss_label_v1(session: &RunControlSession) -> String {
    branch_campaign_boss_v1(session)
        .map(|boss| format!("{boss:?}"))
        .unwrap_or_default()
}

fn branch_campaign_boss_pressure_labels_v1(session: &RunControlSession) -> Vec<String> {
    let Some(boss) = branch_campaign_boss_v1(session) else {
        return Vec::new();
    };
    crate::ai::boss_mechanics_v1::boss_mechanic_pressure_profile_v1(&session.run_state, boss)
        .summary_labels()
}

fn branch_campaign_run_debt_labels_v1(session: &RunControlSession) -> Vec<String> {
    run_debt_ledger_v1(&session.run_state).compact_labels()
}

fn branch_campaign_boss_v1(
    session: &RunControlSession,
) -> Option<crate::content::monsters::factory::EncounterId> {
    session
        .run_state
        .boss_key
        .or_else(|| session.run_state.boss_list.first().copied())
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

fn campaign_branch_quality_key_v1(branch: &BranchCampaignBranchV1) -> String {
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

fn prune_resolved_campaign_strategy_requests_v1(
    requests: Vec<BranchCampaignStrategyRequestV1>,
    _active: &[BranchCampaignBranchV1],
    _frozen: &[BranchCampaignBranchV1],
    stuck: &[BranchCampaignBranchV1],
    abandoned: &[BranchCampaignBranchV1],
) -> Vec<BranchCampaignStrategyRequestV1> {
    requests
        .into_iter()
        .filter(|request| {
            stuck
                .iter()
                .chain(abandoned.iter())
                .any(|branch| campaign_strategy_request_matches_branch_v1(request, branch))
        })
        .collect()
}

fn campaign_strategy_request_matches_branch_v1(
    request: &BranchCampaignStrategyRequestV1,
    branch: &BranchCampaignBranchV1,
) -> bool {
    normalized_campaign_boundary_title(&request.boundary_title)
        == normalized_campaign_boundary_title(&branch.frontier_title)
        && (request.act == 0
            || branch
                .summary
                .as_ref()
                .is_some_and(|summary| summary.act == request.act))
        && (request.floor == 0
            || branch
                .summary
                .as_ref()
                .is_some_and(|summary| summary.floor == request.floor))
        && (request.stop_reasons.is_empty()
            || request
                .stop_reasons
                .iter()
                .any(|reason| branch.stop_reason.contains(reason)))
}

fn render_campaign_branch_state(branch: &BranchCampaignBranchV1) -> String {
    let state = branch
        .summary
        .as_ref()
        .map(|summary| {
            let deck_shape = render_campaign_branch_deck_shape_v1(summary)
                .map(|value| format!(" {value}"))
                .unwrap_or_default();
            let run_debt = render_campaign_branch_run_debt_v1(summary)
                .map(|value| format!(" debt=[{value}]"))
                .unwrap_or_default();
            format!(
                "A{}F{} HP {}/{} gold {} deck {}{}{}",
                summary.act,
                summary.floor,
                summary.hp,
                summary.max_hp,
                summary.gold,
                summary.deck_count,
                deck_shape,
                run_debt
            )
        })
        .unwrap_or_else(|| "start".to_string());
    let selection_basis = render_campaign_branch_selection_basis_v1(branch);
    let strategic_summary = format_compact_branch_signature(&branch.strategic_summary);
    if strategic_summary.is_empty() {
        format!("{state} {selection_basis}")
    } else {
        format!("{state} {selection_basis} strat=[{}]", strategic_summary)
    }
}

fn render_campaign_branch_run_debt_v1(summary: &BranchCampaignBranchSummaryV1) -> Option<String> {
    if summary.run_debt.is_empty() {
        return None;
    }
    let shown = summary
        .run_debt
        .iter()
        .take(2)
        .map(|label| truncate_campaign_run_debt_label_v1(label, 64))
        .collect::<Vec<_>>();
    let suffix = summary.run_debt.len().saturating_sub(shown.len());
    if suffix == 0 {
        Some(shown.join(" | "))
    } else {
        Some(format!("{} +{} more", shown.join(" | "), suffix))
    }
}

fn truncate_campaign_run_debt_label_v1(label: &str, max_chars: usize) -> String {
    if label.chars().count() <= max_chars {
        return label.to_string();
    }
    let prefix = label
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CampaignDeckKeyEntryV1 {
    card: String,
    upgrades: usize,
    count: usize,
}

fn render_campaign_branch_deck_shape_v1(summary: &BranchCampaignBranchSummaryV1) -> Option<String> {
    let entries = parse_campaign_deck_key_entries_v1(&summary.deck_key);
    if entries.is_empty() {
        return None;
    }
    let strike_count = entries
        .iter()
        .filter(|entry| campaign_deck_entry_is_strike_v1(&entry.card))
        .map(|entry| entry.count)
        .sum::<usize>();
    let defend_count = entries
        .iter()
        .filter(|entry| campaign_deck_entry_is_defend_v1(&entry.card))
        .map(|entry| entry.count)
        .sum::<usize>();
    let starter_count = entries
        .iter()
        .filter(|entry| campaign_deck_entry_is_starter_v1(&entry.card))
        .map(|entry| entry.count)
        .sum::<usize>();
    let upgraded_count = entries
        .iter()
        .filter(|entry| entry.upgrades > 0)
        .map(|entry| entry.count)
        .sum::<usize>();
    let additions = entries
        .iter()
        .filter(|entry| !campaign_deck_entry_is_starter_v1(&entry.card))
        .map(format_campaign_deck_key_entry_v1)
        .take(4)
        .collect::<Vec<_>>();
    let additions = if additions.is_empty() {
        "-".to_string()
    } else {
        additions.join(",")
    };
    Some(format!(
        "[S{strike_count} D{defend_count} starter{starter_count} add:{additions} upg{upgraded_count}]"
    ))
}

fn parse_campaign_deck_key_entries_v1(deck_key: &str) -> Vec<CampaignDeckKeyEntryV1> {
    deck_key
        .split(';')
        .filter_map(|part| {
            let part = part.trim();
            let (card_and_upgrade, count) = part.rsplit_once('x')?;
            let (card, upgrades) = card_and_upgrade.rsplit_once('+')?;
            Some(CampaignDeckKeyEntryV1 {
                card: card.trim().to_string(),
                upgrades: upgrades.trim().parse().ok()?,
                count: count.trim().parse().ok()?,
            })
        })
        .collect()
}

fn format_campaign_deck_key_entry_v1(entry: &CampaignDeckKeyEntryV1) -> String {
    let upgrade = match entry.upgrades {
        0 => String::new(),
        1 => "+".to_string(),
        value => format!("+{value}"),
    };
    let count = if entry.count > 1 {
        format!("x{}", entry.count)
    } else {
        String::new()
    };
    format!("{}{}{}", entry.card, upgrade, count)
}

fn campaign_deck_entry_is_starter_v1(card: &str) -> bool {
    campaign_deck_entry_is_strike_v1(card)
        || campaign_deck_entry_is_defend_v1(card)
        || matches!(
            card,
            "Bash" | "Neutralize" | "Survivor" | "Zap" | "Dualcast" | "Eruption" | "Vigilance"
        )
}

fn campaign_deck_entry_is_strike_v1(card: &str) -> bool {
    matches!(card, "Strike" | "StrikeG" | "StrikeB" | "StrikeP")
}

fn campaign_deck_entry_is_defend_v1(card: &str) -> bool {
    matches!(card, "Defend" | "DefendG" | "DefendB" | "DefendP")
}

fn render_choice_path(labels: &[String]) -> String {
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels
            .iter()
            .map(|label| compact_campaign_choice_label_metadata_v1(label))
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

fn render_compact_choice_path(labels: &[String]) -> String {
    const MAX_CHARS: usize = 140;
    if labels.is_empty() {
        return "-".to_string();
    }
    let compact = if labels.len() > 5 {
        let mut parts = Vec::new();
        parts.extend(labels.iter().take(2).cloned());
        parts.push("...".to_string());
        parts.extend(labels.iter().skip(labels.len().saturating_sub(3)).cloned());
        parts.join(" -> ")
    } else {
        render_choice_path(labels)
    };
    if compact.chars().count() <= MAX_CHARS {
        return compact;
    }
    let prefix = compact
        .chars()
        .take(MAX_CHARS.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

fn campaign_strategy_next_step_v1(kind: &str) -> Option<&'static str> {
    match kind {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => Some(
            "campaign should switch remaining macro branches first; if all are exhausted, provide a combat tactic or upstream route/reward rule",
        ),
        "card_reward_policy_gap" => {
            Some("decide whether this reward family should be branched, auto-picked, skipped, or kept for human judgment")
        }
        "event_strategy" => {
            Some("write a narrow event rule or choose one branch manually, then rerun the campaign")
        }
        "campfire_strategy" => {
            Some("choose rest/smith/recall priority for this deck state, then encode only the stable part")
        }
        "boss_relic_strategy" => {
            Some("choose the boss relic package direction, then keep the other branches frozen if still plausible")
        }
        "shop_strategy" => {
            Some("choose buy/remove/leave priorities; avoid expanding every affordable purchase blindly")
        }
        "reward_claim_policy" => {
            Some("decide which remaining rewards are safe automatic claims before continuing")
        }
        "route_policy_gap" => {
            Some("provide a route rule for this context, or provide a one-step map choice before continuing")
        }
        "engineering_issue" => {
            Some("fix the command/state bug before trusting this campaign branch")
        }
        _ => None,
    }
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

fn compact_campaign_choice_label_metadata_v1(label: &str) -> String {
    let parts = label
        .split(" | ")
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .filter(|part| {
            !part.starts_with("source=")
                && !part.starts_with("shop_legacy_estimate=")
                && !part.starts_with("deck mutation role=")
                && !part.starts_with("event_eval ")
                && !part.starts_with("total ")
                && *part != "auto leave shop"
        })
        .map(|part| part.replace(" gold then ", "g then ").replace(" gold", "g"))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        label.to_string()
    } else {
        parts.join(" ")
    }
}

fn campaign_summary_from_report_branch_v1(
    parent: &BranchCampaignBranchV1,
    branch: &BranchExperimentBranchReportV1,
) -> BranchCampaignBranchSummaryV1 {
    let trajectory_key = campaign_trajectory_key_from_report_branch_v1(parent, branch);
    BranchCampaignBranchSummaryV1 {
        act: branch.summary.act,
        floor: branch.summary.floor,
        hp: branch.summary.hp,
        max_hp: branch.summary.max_hp,
        gold: branch.summary.gold,
        deck_count: branch.summary.deck_count,
        deck_key: String::new(),
        formation_stage: format!("{:?}", branch.summary.formation_stage),
        formation_strengths: branch
            .summary
            .formation_strengths
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        formation_needs: branch
            .summary
            .formation_needs
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        trajectory_key,
        boss: String::new(),
        boss_pressure: Vec::new(),
        run_debt: Vec::new(),
    }
}

fn campaign_trajectory_key_from_report_branch_v1(
    parent: &BranchCampaignBranchV1,
    branch: &BranchExperimentBranchReportV1,
) -> String {
    let mut trajectory = parent
        .summary
        .as_ref()
        .and_then(|summary| parse_branch_trajectory_key_for_campaign_v1(&summary.trajectory_key))
        .unwrap_or_default();
    merge_campaign_branch_trajectory_v1(&mut trajectory, &branch.summary.trajectory);
    branch_trajectory_key_v1(&trajectory)
}

fn merge_campaign_branch_trajectory_v1(
    target: &mut BranchTrajectorySignatureV1,
    source: &BranchTrajectorySignatureV1,
) {
    target.frontload_picks = target
        .frontload_picks
        .saturating_add(source.frontload_picks);
    target.transition_frontload_picks = target
        .transition_frontload_picks
        .saturating_add(source.transition_frontload_picks);
    target.scaling_picks = target.scaling_picks.saturating_add(source.scaling_picks);
    target.defense_picks = target.defense_picks.saturating_add(source.defense_picks);
    target.engine_generator_picks = target
        .engine_generator_picks
        .saturating_add(source.engine_generator_picks);
    target.engine_payoff_picks = target
        .engine_payoff_picks
        .saturating_add(source.engine_payoff_picks);
    target.draw_energy_picks = target
        .draw_energy_picks
        .saturating_add(source.draw_energy_picks);
    merge_campaign_trajectory_keys_v1(&mut target.setup_keys, &source.setup_keys);
    merge_campaign_trajectory_keys_v1(&mut target.package_keys, &source.package_keys);
}

fn merge_campaign_trajectory_keys_v1(target: &mut Vec<String>, source: &[String]) {
    for key in source {
        if !target.iter().any(|existing| existing == key) {
            target.push(key.clone());
        }
    }
    target.sort();
}

fn parse_branch_trajectory_key_for_campaign_v1(key: &str) -> Option<BranchTrajectorySignatureV1> {
    if key.trim().is_empty() {
        return None;
    }
    let mut signature = BranchTrajectorySignatureV1::default();
    for part in key.split('|') {
        if let Some(value) = part.strip_prefix("setup=") {
            signature.setup_keys = parse_campaign_trajectory_key_list_v1(value);
        } else if let Some(value) = part.strip_prefix("pkg=") {
            signature.package_keys = parse_campaign_trajectory_key_list_v1(value);
        } else if let Some(value) = part.strip_prefix("frontload=") {
            signature.frontload_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("transition=") {
            signature.transition_frontload_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("scaling=") {
            signature.scaling_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("defense=") {
            signature.defense_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("engine_gen=") {
            signature.engine_generator_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("engine_payoff=") {
            signature.engine_payoff_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("draw_energy=") {
            signature.draw_energy_picks = value.parse().ok()?;
        }
    }
    signature.setup_keys.sort();
    signature.package_keys.sort();
    Some(signature)
}

fn parse_campaign_trajectory_key_list_v1(value: &str) -> Vec<String> {
    if value == "-" || value.is_empty() {
        return Vec::new();
    }
    value
        .split('+')
        .filter(|key| !key.is_empty())
        .map(str::to_string)
        .collect()
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
