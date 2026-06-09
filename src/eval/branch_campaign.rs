use crate::eval::branch_experiment::{
    run_branch_experiment_from_session_with_snapshots_v1, run_branch_experiment_with_snapshots_v1,
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentConfigV1,
    BranchExperimentRouteDecisionV1, BranchExperimentRunResultV1,
    BranchExperimentStrategyRequestV1, BRANCH_EXPERIMENT_REPLAY_ADVANCE_COMMAND,
};
use crate::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use crate::eval::run_control::{
    RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession,
    RunControlSessionCheckpointV1,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const BRANCH_CAMPAIGN_SCHEMA_NAME: &str = "BranchCampaignV1";
pub const BRANCH_CAMPAIGN_SCHEMA_VERSION: u32 = 1;
pub const BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME: &str = "BranchCampaignCheckpointV1";
pub const BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION: u32 = 1;
const COMBAT_RETRY_NODE_MULTIPLIER: usize = 4;
const COMBAT_RETRY_WALL_MULTIPLIER: u64 = 4;
const COMBAT_RETRY_MIN_NODES: usize = 200_000;
const COMBAT_RETRY_MAX_NODES: usize = 500_000;
const COMBAT_RETRY_MIN_WALL_MS: u64 = 1_200;
const COMBAT_RETRY_MAX_WALL_MS: u64 = 5_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchCampaignCombatRetryPolicyV1 {
    /// Keep moving through available branches first. If all routes stall on combat,
    /// the campaign will surface that as an intervention instead of retrying every parent.
    OnStall,
    /// Legacy behavior: immediately rerun a parent with a larger combat budget when all
    /// produced children are pruned combat branches.
    Immediate,
    Disabled,
}

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
    pub include_event_reward_skip: bool,
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
            retention_budget_profile: BranchRetentionBudgetProfileV1::Package,
            max_reward_options_per_branch: Some(2),
            max_campfire_options_per_branch: 3,
            auto_max_operations: 128,
            experiment_wall_ms: Some(10_000),
            search_max_nodes: Some(50_000),
            search_wall_ms: Some(200),
            search_max_hp_loss: None,
            search_options: RunControlSearchCombatOptions::default(),
            combat_retry_policy: BranchCampaignCombatRetryPolicyV1::OnStall,
            include_event_reward_skip: false,
            prefix_commands: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignBranchStatusV1 {
    Active,
    Frozen,
    TerminalVictory,
    TerminalDefeat,
    Abandoned,
    Stuck,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignBranchSummaryV1 {
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_count: usize,
    pub formation_stage: String,
    pub formation_strengths: Vec<String>,
    pub formation_needs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignBranchV1 {
    pub branch_id: String,
    pub commands: Vec<String>,
    pub choice_labels: Vec<String>,
    pub summary: Option<BranchCampaignBranchSummaryV1>,
    pub frontier_title: String,
    pub status: BranchCampaignBranchStatusV1,
    pub rank_key: i32,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignSelectionV1 {
    pub active: Vec<BranchCampaignBranchV1>,
    pub frozen: Vec<BranchCampaignBranchV1>,
    pub victories: Vec<BranchCampaignBranchV1>,
    pub dead: Vec<BranchCampaignBranchV1>,
    pub abandoned: Vec<BranchCampaignBranchV1>,
    pub stuck: Vec<BranchCampaignBranchV1>,
    pub discarded_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignStrategyRequestV1 {
    pub kind: String,
    pub boundary_title: String,
    pub branch_count: usize,
    #[serde(default)]
    pub act: u8,
    #[serde(default)]
    pub floor: i32,
    #[serde(default)]
    pub stop_reasons: Vec<String>,
    pub examples: Vec<String>,
    #[serde(default)]
    pub next_card_reward_offer: Option<Vec<String>>,
    #[serde(default)]
    pub boundary_details: Vec<String>,
    pub suggested_action: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRoundSummaryV1 {
    pub round: usize,
    pub started_active: usize,
    pub produced_branches: usize,
    pub active_after: usize,
    pub frozen_added: usize,
    pub dead_added: usize,
    pub abandoned_added: usize,
    pub victories_added: usize,
    pub stuck_added: usize,
    pub discarded_added: usize,
    pub explored_branch_points: usize,
    pub wall_limit_hit: bool,
    pub branch_limit_hit: bool,
    pub combat_budget_retries: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRouteEvidenceSummaryV1 {
    pub decisions: usize,
    pub first_elite_forced: usize,
    pub first_elite_optional: usize,
    pub first_elite_none: usize,
    pub rest_bailout: usize,
    pub shop_bailout: usize,
    pub underprepared_first_elite: usize,
    pub avg_elite_prep_bp: i32,
    pub examples: Vec<BranchCampaignRouteEvidenceExampleV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRouteEvidenceExampleV1 {
    pub target: String,
    pub first_elite: String,
    pub elite_prep_bp: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    pub rounds_completed: usize,
    pub stop_reason: String,
    pub active: Vec<BranchCampaignBranchV1>,
    pub frozen: Vec<BranchCampaignBranchV1>,
    pub victories: Vec<BranchCampaignBranchV1>,
    pub dead: Vec<BranchCampaignBranchV1>,
    pub abandoned: Vec<BranchCampaignBranchV1>,
    pub stuck: Vec<BranchCampaignBranchV1>,
    pub discarded_count: usize,
    pub strategy_requests: Vec<BranchCampaignStrategyRequestV1>,
    #[serde(default)]
    pub route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    pub rounds: Vec<BranchCampaignRoundSummaryV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointSessionV1 {
    pub commands: Vec<String>,
    pub session: RunControlSessionCheckpointV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    pub rounds_completed: usize,
    pub sessions: Vec<BranchCampaignCheckpointSessionV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchCampaignRunResultV1 {
    pub report: BranchCampaignReportV1,
    pub checkpoint: BranchCampaignCheckpointV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BranchCampaignProgressEventV1 {
    CampaignStarted {
        seed: u64,
        max_rounds: usize,
        round_depth: usize,
        max_active: usize,
        max_frozen: usize,
    },
    RoundStarted {
        round: usize,
        max_rounds: usize,
        active_branches: usize,
        frozen_branches: usize,
    },
    BranchStarted {
        round: usize,
        branch_index: usize,
        branch_count: usize,
        choices: String,
    },
    BranchFinished {
        round: usize,
        branch_index: usize,
        branch_count: usize,
        produced_branches: usize,
        explored_branch_points: usize,
        elapsed_wall_ms: u64,
        start_elapsed_wall_ms: u64,
        combat_budget_retry_used: bool,
        wall_limit_hit: bool,
        branch_limit_hit: bool,
    },
    RoundFinished {
        round: usize,
        started_active: usize,
        produced_branches: usize,
        active_after: usize,
        frozen_added: usize,
        strategy_requests: usize,
    },
    FrozenPromoted {
        promoted: usize,
        active_after: usize,
        frozen_remaining: usize,
    },
    CampaignFinished {
        stop_reason: String,
        active: usize,
        frozen: usize,
        victories: usize,
        stuck: usize,
    },
}

struct BranchCampaignParentRoundResultV1 {
    result: BranchExperimentRunResultV1,
    combat_budget_retry_used: bool,
}

struct BranchCampaignParentBatchResultV1 {
    candidates: Vec<BranchCampaignBranchV1>,
    strategy_requests: Vec<BranchExperimentStrategyRequestV1>,
    route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    explored_branch_points: usize,
    wall_limit_hit: bool,
    branch_limit_hit: bool,
    combat_budget_retries: usize,
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
    strategy_requests: Vec<BranchCampaignStrategyRequestV1>,
    route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    rounds: Vec<BranchCampaignRoundSummaryV1>,
    snapshot_cache: BTreeMap<Vec<String>, RunControlSession>,
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
    let state = match checkpoint {
        Some(checkpoint) => campaign_state_from_report_and_checkpoint_v1(previous, checkpoint)?,
        None => campaign_state_from_report_v1(previous),
    };
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
        if state.active.is_empty() && state.victories.is_empty() && !state.frozen.is_empty() {
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
                });
            }
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
        let parents = std::mem::take(&mut state.active);
        let started_active = parents.len();
        let mut batch = run_campaign_parent_batch_v1(
            config,
            &parents,
            &mut state.snapshot_cache,
            round_number,
            false,
            &mut progress,
        )?;
        let mut produced_branches = batch.candidates.len();
        let mut selected = select_campaign_branches_v1(
            batch.candidates.clone(),
            config.max_active,
            config.max_frozen,
        );
        if campaign_round_should_retry_combat_budget_on_stall_v1(config, &selected) {
            if let Some(retry_config) = combat_retry_campaign_config_v1(config) {
                batch = run_campaign_parent_batch_v1(
                    &retry_config,
                    &parents,
                    &mut state.snapshot_cache,
                    round_number,
                    true,
                    &mut progress,
                )?;
                produced_branches = batch.candidates.len();
                selected = select_campaign_branches_v1(
                    batch.candidates.clone(),
                    config.max_active,
                    config.max_frozen,
                );
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
        );
        state.discarded_count = state
            .discarded_count
            .saturating_add(selected.discarded_count);
        let dead_added = selected.dead.len();
        let abandoned_added = selected.abandoned.len();
        let victories_added = selected.victories.len();
        let stuck_added = selected.stuck.len();
        state.active = selected.active;
        state.victories.extend(selected.victories);
        state.dead.extend(selected.dead);
        state.abandoned.extend(selected.abandoned);
        state.stuck.extend(selected.stuck);
        retain_campaign_snapshot_cache_v1(&mut state.snapshot_cache, &state.active, &state.frozen);
        let promoted_from_frozen = if state.active.is_empty() && state.victories.is_empty() {
            promote_frozen_to_active_v1(&mut state.active, &mut state.frozen, config.max_active)
        } else {
            0
        };
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
        };
        progress(BranchCampaignProgressEventV1::RoundFinished {
            round: round_number,
            started_active,
            produced_branches,
            active_after: state.active.len(),
            frozen_added,
            strategy_requests: state.strategy_requests.len(),
        });
        if promoted_from_frozen > 0 {
            progress(BranchCampaignProgressEventV1::FrozenPromoted {
                promoted: promoted_from_frozen,
                active_after: state.active.len(),
                frozen_remaining: state.frozen.len(),
            });
        }
        state.rounds.push(round_summary);
        state.rounds_completed = state.rounds_completed.saturating_add(1);

        if !state.victories.is_empty() {
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

    let checkpoint = campaign_checkpoint_from_state_v1(config, &state);
    let report = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: config.seed,
        rounds_completed: state.rounds_completed,
        stop_reason,
        active: state.active,
        frozen: state.frozen,
        victories: state.victories,
        dead: state.dead,
        abandoned: state.abandoned,
        stuck: state.stuck,
        discarded_count: state.discarded_count,
        strategy_requests: state.strategy_requests,
        route_evidence: state.route_evidence,
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
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        rounds: Vec::new(),
        snapshot_cache: BTreeMap::new(),
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
        strategy_requests: report.strategy_requests.clone(),
        route_evidence: report.route_evidence.clone(),
        rounds: report.rounds.clone(),
        snapshot_cache: BTreeMap::new(),
    }
}

fn campaign_state_from_report_and_checkpoint_v1(
    report: &BranchCampaignReportV1,
    checkpoint: &BranchCampaignCheckpointV1,
) -> Result<BranchCampaignRunStateV1, String> {
    validate_campaign_resume_checkpoint_v1(report, checkpoint)?;
    let mut state = campaign_state_from_report_v1(report);
    let keep = state
        .active
        .iter()
        .chain(state.frozen.iter())
        .map(|branch| branch.commands.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for entry in &checkpoint.sessions {
        if keep.contains(&entry.commands) {
            state.snapshot_cache.insert(
                entry.commands.clone(),
                entry.session.clone().into_session().map_err(|err| {
                    format!("failed to restore campaign checkpoint session: {err}")
                })?,
            );
        }
    }
    Ok(state)
}

fn campaign_checkpoint_from_state_v1(
    config: &BranchCampaignConfigV1,
    state: &BranchCampaignRunStateV1,
) -> BranchCampaignCheckpointV1 {
    let mut sessions = Vec::new();
    for branch in state.active.iter().chain(state.frozen.iter()) {
        if let Some(session) = state.snapshot_cache.get(&branch.commands) {
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
        rounds_completed: state.rounds_completed,
        sessions,
    }
}

fn retain_campaign_snapshot_cache_v1(
    snapshot_cache: &mut BTreeMap<Vec<String>, RunControlSession>,
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
) {
    let keep = active
        .iter()
        .chain(frozen.iter())
        .map(|branch| branch.commands.clone())
        .collect::<std::collections::BTreeSet<_>>();
    snapshot_cache.retain(|commands, _| keep.contains(commands));
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

pub fn render_branch_campaign_progress_event_v1(event: &BranchCampaignProgressEventV1) -> String {
    match event {
        BranchCampaignProgressEventV1::CampaignStarted {
            seed,
            max_rounds,
            round_depth,
            max_active,
            max_frozen,
        } => format!(
            "campaign start: seed={seed} rounds={max_rounds} round_depth={round_depth} active_cap={max_active} frozen_cap={max_frozen}"
        ),
        BranchCampaignProgressEventV1::RoundStarted {
            round,
            max_rounds,
            active_branches,
            frozen_branches,
        } => format!(
            "round {round}/{max_rounds}: advancing {active_branches} active branch(es), frozen={frozen_branches}"
        ),
        BranchCampaignProgressEventV1::BranchStarted {
            round,
            branch_index,
            branch_count,
            choices,
        } => format!(
            "round {round}: branch {branch_index}/{branch_count} running | choices: {choices}"
        ),
        BranchCampaignProgressEventV1::BranchFinished {
            round,
            branch_index,
            branch_count,
            produced_branches,
            explored_branch_points,
            elapsed_wall_ms,
            start_elapsed_wall_ms,
            combat_budget_retry_used,
            wall_limit_hit,
            branch_limit_hit,
        } => {
            let mut limits = Vec::new();
            if *branch_limit_hit {
                limits.push("branch");
            }
            if *wall_limit_hit {
                limits.push("wall");
            }
            let limits = if limits.is_empty() {
                "-".to_string()
            } else {
                limits.join(",")
            };
            let retry = if *combat_budget_retry_used {
                " retry=combat_budget"
            } else {
                ""
            };
            let start = if *start_elapsed_wall_ms > 0 {
                format!(" start_ms={start_elapsed_wall_ms}")
            } else {
                String::new()
            };
            format!(
                "round {round}: branch {branch_index}/{branch_count} done | produced={produced_branches} branch_points={explored_branch_points} elapsed_ms={elapsed_wall_ms}{start}{retry} limits=[{limits}]"
            )
        }
        BranchCampaignProgressEventV1::RoundFinished {
            round,
            started_active,
            produced_branches,
            active_after,
            frozen_added,
            strategy_requests,
        } => format!(
            "round {round} done: started={started_active} produced={produced_branches} active_after={active_after} frozen_added={frozen_added} strategy_requests={strategy_requests}"
        ),
        BranchCampaignProgressEventV1::FrozenPromoted {
            promoted,
            active_after,
            frozen_remaining,
        } => format!(
            "promoted {promoted} frozen branch(es) after active branches ran out; active_after={active_after} frozen={frozen_remaining}"
        ),
        BranchCampaignProgressEventV1::CampaignFinished {
            stop_reason,
            active,
            frozen,
            victories,
            stuck,
        } => format!(
            "campaign finished: stop={stop_reason} active={active} frozen={frozen} victories={victories} stuck={stuck}"
        ),
    }
}

pub fn render_branch_campaign_compact_v1(
    report: &BranchCampaignReportV1,
    branch_examples: usize,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} seed={} rounds={} stop={}",
        report.schema_name, report.seed, report.rounds_completed, report.stop_reason
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
    }
    if report.stop_reason == "max_rounds"
        && (!report.active.is_empty() || !report.frozen.is_empty())
    {
        lines.push(
            "Next: budget ended; use .\\tools\\campaign.ps1 -More or raise -MaxRounds to keep exploring this seed"
                .to_string(),
        );
    }
    if !report.strategy_requests.is_empty() {
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
        for (index, branch) in report.active.iter().take(branch_examples).enumerate() {
            lines.push(format!(
                "  {}. {} | {} | choices: {}",
                index + 1,
                render_campaign_branch_state(branch),
                branch.frontier_title,
                render_choice_path(&branch.choice_labels)
            ));
        }
    }
    if !report.frozen.is_empty() {
        lines.push(String::new());
        lines.push("Frozen examples:".to_string());
        for (index, branch) in report.frozen.iter().take(branch_examples).enumerate() {
            lines.push(format!(
                "  {}. {} | {} | choices: {}",
                index + 1,
                render_campaign_branch_state(branch),
                branch.frontier_title,
                render_choice_path(&branch.choice_labels)
            ));
        }
    }
    lines.join("\n")
}

fn format_bp(value: i32) -> String {
    format!("{:.2}", f64::from(value) / 100.0)
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
    matches!(
        report.stop_reason.as_str(),
        "needs_intervention" | "stuck" | "no_active_branch" | "no_progress"
    ) && report.active.is_empty()
        && report.frozen.is_empty()
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
        frontier_title: "start".to_string(),
        status: BranchCampaignBranchStatusV1::Active,
        rank_key: 0,
    }
}

fn run_campaign_parent_batch_v1<F>(
    config: &BranchCampaignConfigV1,
    parents: &[BranchCampaignBranchV1],
    snapshot_cache: &mut BTreeMap<Vec<String>, RunControlSession>,
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

    for (parent_index, parent) in parents.iter().enumerate() {
        progress(BranchCampaignProgressEventV1::BranchStarted {
            round: round_number,
            branch_index: parent_index + 1,
            branch_count: parent_count,
            choices: render_choice_path(&parent.choice_labels),
        });
        let parent_snapshot = snapshot_cache.get(&parent.commands).cloned();
        let parent_result = run_campaign_parent_round_v1(config, parent, parent_snapshot)?;
        let result = parent_result.result;
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
            let child = campaign_branch_from_report_branch_v1(parent, branch);
            if let Some(snapshot) = result.branch_sessions.get(&branch.branch_id) {
                snapshot_cache.insert(child.commands.clone(), snapshot.clone());
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
    })
}

fn run_campaign_parent_round_v1(
    config: &BranchCampaignConfigV1,
    parent: &BranchCampaignBranchV1,
    parent_snapshot: Option<RunControlSession>,
) -> Result<BranchCampaignParentRoundResultV1, String> {
    let result = match run_campaign_parent_round_once_v1(config, parent, parent_snapshot.clone()) {
        Ok(result) => result,
        Err(err)
            if campaign_parent_replay_error_is_retryable_v1(&err)
                && combat_retry_campaign_config_v1(config).is_some() =>
        {
            let retry_config = combat_retry_campaign_config_v1(config)
                .expect("retry config was checked before retrying parent replay");
            return run_campaign_parent_round_once_v1(&retry_config, parent, parent_snapshot)
                .map(|result| BranchCampaignParentRoundResultV1 {
                    result,
                    combat_budget_retry_used: true,
                })
                .map_err(|retry_err| {
                    format!(
                        "campaign parent replay failed before retry: {err}\nretry also failed: {retry_err}"
                    )
                });
        }
        Err(err) => return Err(err),
    };
    if !campaign_parent_should_retry_combat_budget_now_v1(config, &result.report.branches) {
        return Ok(BranchCampaignParentRoundResultV1 {
            result,
            combat_budget_retry_used: false,
        });
    }

    let Some(retry_config) = combat_retry_campaign_config_v1(config) else {
        return Ok(BranchCampaignParentRoundResultV1 {
            result,
            combat_budget_retry_used: false,
        });
    };
    let retry_result = run_campaign_parent_round_once_v1(&retry_config, parent, parent_snapshot)?;
    Ok(BranchCampaignParentRoundResultV1 {
        result: retry_result,
        combat_budget_retry_used: true,
    })
}

fn run_campaign_parent_round_once_v1(
    config: &BranchCampaignConfigV1,
    parent: &BranchCampaignBranchV1,
    parent_snapshot: Option<RunControlSession>,
) -> Result<BranchExperimentRunResultV1, String> {
    let mut experiment_config = campaign_branch_experiment_config_v1(config);
    if let Some(session) = parent_snapshot {
        experiment_config.prefix_commands.clear();
        return Ok(run_branch_experiment_from_session_with_snapshots_v1(
            session,
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
        ..BranchExperimentConfigV1::default()
    }
}

fn campaign_parent_replay_error_is_retryable_v1(error: &str) -> bool {
    error.contains("is not valid on the current screen")
        || error.contains("is only valid on a card reward item or card reward screen")
}

fn combat_retry_campaign_config_v1(
    config: &BranchCampaignConfigV1,
) -> Option<BranchCampaignConfigV1> {
    let retry_nodes = retry_node_budget_v1(config.search_max_nodes);
    let retry_wall_ms = retry_wall_budget_v1(config.search_wall_ms);
    if retry_nodes == config.search_max_nodes && retry_wall_ms == config.search_wall_ms {
        return None;
    }

    let mut retry_config = config.clone();
    retry_config.search_max_nodes = retry_nodes;
    retry_config.search_wall_ms = retry_wall_ms;
    retry_config.search_max_hp_loss = config
        .search_max_hp_loss
        .or(Some(RunControlHpLossLimit::Unlimited));
    Some(retry_config)
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
    matches!(
        config.combat_retry_policy,
        BranchCampaignCombatRetryPolicyV1::Immediate
    ) && branch_report_needs_combat_budget_retry_v1(branches)
}

fn campaign_round_should_retry_combat_budget_on_stall_v1(
    config: &BranchCampaignConfigV1,
    selection: &BranchCampaignSelectionV1,
) -> bool {
    matches!(
        config.combat_retry_policy,
        BranchCampaignCombatRetryPolicyV1::OnStall
    ) && combat_retry_campaign_config_v1(config).is_some()
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
) -> usize {
    let mut added = 0usize;
    for branch in new_frozen {
        if frozen.len() < max_frozen {
            frozen.push(branch);
            added = added.saturating_add(1);
        } else {
            *discarded_count = discarded_count.saturating_add(1);
        }
    }
    added
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
            "raise combat retry budget, inspect the combat, or provide a manual line".to_string()
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
        stop_reasons: vec!["all candidate route branches were abandoned".to_string()],
        examples,
        next_card_reward_offer: None,
        boundary_details: Vec::new(),
        suggested_action:
            "raise combat retry budget, provide a manual combat line, or abandon this route family"
                .to_string(),
    })
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
            "raise combat retry budget | provide a manual combat line | abandon this macro route family"
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

    active_candidates.sort_by(|left, right| {
        right
            .rank_key
            .cmp(&left.rank_key)
            .then_with(|| branch_progress_key(right).cmp(&branch_progress_key(left)))
            .then_with(|| left.branch_id.cmp(&right.branch_id))
    });

    for (index, mut branch) in active_candidates.into_iter().enumerate() {
        if index < max_active {
            branch.status = BranchCampaignBranchStatusV1::Active;
            selection.active.push(branch);
        } else if selection.frozen.len() < max_frozen {
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            selection.frozen.push(branch);
        } else {
            selection.discarded_count = selection.discarded_count.saturating_add(1);
        }
    }
    selection
}

fn promote_frozen_to_active_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> usize {
    frozen.sort_by(|left, right| {
        branch_progress_key(right)
            .cmp(&branch_progress_key(left))
            .then_with(|| right.rank_key.cmp(&left.rank_key))
            .then_with(|| left.branch_id.cmp(&right.branch_id))
    });
    let mut promoted = 0usize;
    while active.len() < max_active && !frozen.is_empty() {
        let mut branch = frozen.remove(0);
        branch.status = BranchCampaignBranchStatusV1::Active;
        active.push(branch);
        promoted = promoted.saturating_add(1);
    }
    promoted
}

fn render_campaign_branch_state(branch: &BranchCampaignBranchV1) -> String {
    branch
        .summary
        .as_ref()
        .map(|summary| {
            format!(
                "A{}F{} HP {}/{} gold {} deck {}",
                summary.act,
                summary.floor,
                summary.hp,
                summary.max_hp,
                summary.gold,
                summary.deck_count
            )
        })
        .unwrap_or_else(|| "start".to_string())
}

fn render_choice_path(labels: &[String]) -> String {
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(" -> ")
    }
}

fn campaign_strategy_next_step_v1(kind: &str) -> Option<&'static str> {
    match kind {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => Some(
            "try a deeper same-seed run, e.g. .\\tools\\campaign.ps1 -More; if it still stops, inspect the combat or provide a manual line",
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
        branch_id: branch.branch_id.clone(),
        commands,
        choice_labels,
        summary: Some(campaign_summary_from_report_branch_v1(branch)),
        frontier_title: branch.summary.boundary_title.clone(),
        status: campaign_status_from_report_status(branch.status),
        rank_key: branch.rank_key,
    }
}

fn branch_progress_key(branch: &BranchCampaignBranchV1) -> (u8, i32, i32) {
    branch
        .summary
        .as_ref()
        .map(|summary| (summary.act, summary.floor, summary.hp))
        .unwrap_or((0, 0, 0))
}

fn campaign_choice_label_v1(
    choice: &crate::eval::branch_experiment::BranchExperimentChoiceV1,
) -> String {
    if choice.effect_label.is_empty() {
        choice.label.clone()
    } else {
        choice.effect_label.clone()
    }
}

fn campaign_summary_from_report_branch_v1(
    branch: &BranchExperimentBranchReportV1,
) -> BranchCampaignBranchSummaryV1 {
    BranchCampaignBranchSummaryV1 {
        act: branch.summary.act,
        floor: branch.summary.floor,
        hp: branch.summary.hp,
        max_hp: branch.summary.max_hp,
        gold: branch.summary.gold,
        deck_count: branch.summary.deck_count,
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
