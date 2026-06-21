use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use self::lineage::{branch_frontier, frontier_groups};
use self::strategy_request::branch_strategy_requests;
use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyFormationSummaryV2,
};
use crate::ai::opening_hand_target_plan_v1::opening_hand_target_debt_tags_v1;
use crate::ai::route_planner_v1::{MapDecisionPacketV1, RouteMoveCandidateV1, RouteSafetyFlagV1};
use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, compile_shop_decision_v1,
    compiled_shop_decision_has_executable_conversion_branch_v1, ShopCompileModeV1,
    ShopPolicyConfigV1,
};
use crate::ai::strategic::run_debt_ledger_v1;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::relics::RelicId;
use crate::eval::branch_experiment_boundary::{
    branch_boundary_available, current_branch_boundary, BranchBoundaryActionV1,
    BranchBoundaryConfigV1, BranchBoundaryIdV1, CardRewardPortfolioContext,
};
use crate::eval::branch_experiment_retention::{
    branch_retention_order_rank_key_v1, default_branch_retention_decision_v1,
    select_branch_retention_portfolio_v1, BranchRetentionCandidateInputV1, BranchRetentionConfigV1,
    BranchRetentionDecisionV1, BranchRetentionSlotV1,
};
use crate::eval::branch_experiment_trajectory::{
    summarize_branch_trajectory_v1, BranchTrajectorySignatureV1,
};
use crate::eval::run_control::CombatSearchPerformanceSnapshotV1;
#[cfg(test)]
use crate::eval::run_control::RunControlCommand;
use crate::eval::run_control::{
    build_decision_surface, canonical_player_class, combat_automation_trajectories_v1,
    load_session_trace_v1, parse_run_control_command, replay_session_trace,
    RunActionCardSnapshotV1, RunActionResultChangeV1, RunControlAutoStepOptions,
    RunControlCommandOutcome, RunControlConfig, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession, RunControlTraceAnnotationV1,
    SessionTraceReplayOptions, SessionTraceReplayStop,
};
use crate::state::core::{
    master_deck_card_is_bottled, master_deck_card_is_purgeable, EngineState, RunResult,
};
use crate::state::map::node::RoomType;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

mod lineage;
mod strategy_request;
mod types;

const BRANCH_EXPERIMENT_COMMAND_SEQUENCE_SEPARATOR: &str = " && ";
const COMBAT_TURN_SEGMENT_PROGRESS_STOP_REASON: &str =
    "combat turn segment progressed; continue next campaign round";

pub use types::{
    BranchExperimentBossCombatRecordV1, BranchExperimentBossRelicCandidateEntryV1,
    BranchExperimentBossRelicCandidatePoolV1, BranchExperimentBranchReportV1,
    BranchExperimentBranchStatusV1, BranchExperimentCampfirePlanCandidateEntryV1,
    BranchExperimentCampfirePlanCandidatePoolV1, BranchExperimentChoiceCardV1,
    BranchExperimentChoiceDecisionSignalV1, BranchExperimentChoiceV1, BranchExperimentConfigV1,
    BranchExperimentEventCandidateEntryV1, BranchExperimentEventCandidatePoolV1,
    BranchExperimentFirstEliteEvidenceV1, BranchExperimentFrontierGroupV1,
    BranchExperimentFrontierV1, BranchExperimentLineageV1, BranchExperimentPrunedBranchSummaryV1,
    BranchExperimentPrunedFirstPickCountV1, BranchExperimentReportV1,
    BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRewardOptionPortfolioV1,
    BranchExperimentRouteCandidateEntryV1, BranchExperimentRouteCandidatePoolV1,
    BranchExperimentRouteDecisionV1, BranchExperimentRunSummaryV1,
    BranchExperimentShopPlanCandidateEntryV1, BranchExperimentShopPlanCandidatePoolV1,
    BranchExperimentStrategyRequestV1, BranchExperimentWallLimitPhaseV1,
    BRANCH_EXPERIMENT_CARD_REWARD_STRATEGIC_TRACE_SIGNAL_SOURCE_V1, BRANCH_EXPERIMENT_SCHEMA_NAME,
    BRANCH_EXPERIMENT_SCHEMA_VERSION, BRANCH_EXPERIMENT_SHOP_ALTERNATIVE_PLAN_SIGNAL_SOURCE_V1,
    BRANCH_EXPERIMENT_SHOP_BRANCH_FRONTIER_SIGNAL_SOURCE_V1,
    BRANCH_EXPERIMENT_SHOP_COMPAT_SELECTED_PLAN_SIGNAL_SOURCE_V1,
};

pub(crate) const BRANCH_EXPERIMENT_REPLAY_ADVANCE_COMMAND: &str =
    "__branch_experiment_replay_advance";
pub const BRANCH_EXPERIMENT_DECISION_PARENT_COMMAND_PREFIX_V1: &str = "__decision_parent:";

#[derive(Clone, Debug)]
struct BranchWork {
    id: String,
    session: RunControlSession,
    choices: Vec<BranchExperimentChoiceV1>,
    status: BranchExperimentBranchStatusV1,
    stop_reason: String,
    retention: BranchRetentionDecisionV1,
    final_boss_combat_record: Option<BranchExperimentBossCombatRecordV1>,
}

#[derive(Clone, Debug)]
struct BranchExperimentPreparedStart {
    branch: BranchWork,
    replay_trace_applied_steps: usize,
    replay_trace_stop: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BranchExperimentRunResultV1 {
    pub report: BranchExperimentReportV1,
    pub branch_sessions: BTreeMap<String, RunControlSession>,
    pub decision_parent_sessions: BTreeMap<Vec<String>, RunControlSession>,
    pub start_elapsed_wall_ms: u64,
    pub combat_performance_samples: Vec<CombatSearchPerformanceSnapshotV1>,
}

pub fn run_branch_experiment_v1(
    config: &BranchExperimentConfigV1,
) -> Result<BranchExperimentReportV1, String> {
    run_branch_experiment_with_snapshots_v1(config).map(|result| result.report)
}

pub fn run_branch_experiment_with_snapshots_v1(
    config: &BranchExperimentConfigV1,
) -> Result<BranchExperimentRunResultV1, String> {
    let started_at = Instant::now();
    let prepared = prepare_branch_experiment_start(config, false)?;
    let start_elapsed_wall_ms = elapsed_ms_u64(started_at);
    let mut result = run_branch_experiment_from_start_branch_with_replay_and_snapshots(
        prepared.branch,
        config,
        prepared.replay_trace_applied_steps,
        prepared.replay_trace_stop,
    );
    result.start_elapsed_wall_ms = start_elapsed_wall_ms;
    Ok(result)
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
        require_same!(search_options);
        require_same!(auto_capture);
        require_same!(include_skip);
        require_same!(include_event_reward_skip);
        require_same!(auto_leave_after_shop_purchase_branch);
        require_same!(defer_branch_settle);
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
        auto_capture: config.auto_capture.clone(),
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

    let prefix_final_boss_combat_record =
        apply_branch_experiment_prefix_commands_v1(&mut session, config, &config.prefix_commands)?;

    let mut branch = BranchWork {
        id: "root".to_string(),
        session,
        choices: Vec::new(),
        status: BranchExperimentBranchStatusV1::Active,
        stop_reason: "initial".to_string(),
        retention: default_branch_retention_decision_v1(),
        final_boss_combat_record: prefix_final_boss_combat_record,
    };
    if settle_to_first_boundary {
        let mut ignored_route_decisions = Vec::new();
        let mut ignored_route_candidate_pools = Vec::new();
        let mut ignored_combat_performance = Vec::new();
        settle_branch_to_frontier(
            &mut branch,
            config,
            &mut ignored_route_decisions,
            &mut ignored_route_candidate_pools,
            &mut BTreeMap::new(),
            &mut ignored_combat_performance,
        );
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
    run_branch_experiment_from_session_with_snapshots_v1(session, config).report
}

pub fn run_branch_experiment_from_session_with_snapshots_v1(
    session: RunControlSession,
    config: &BranchExperimentConfigV1,
) -> BranchExperimentRunResultV1 {
    run_branch_experiment_from_session_with_replay_and_snapshots(session, config, 0, None)
}

pub fn run_branch_experiment_from_session_after_prefix_with_snapshots_v1(
    mut session: RunControlSession,
    config: &BranchExperimentConfigV1,
    prefix_commands: &[String],
) -> Result<BranchExperimentRunResultV1, String> {
    session.set_auto_capture_config(config.auto_capture.clone());
    let prefix_final_boss_combat_record =
        apply_branch_experiment_prefix_commands_v1(&mut session, config, prefix_commands)?;
    Ok(
        run_branch_experiment_from_start_branch_with_replay_and_snapshots(
            BranchWork {
                id: "root".to_string(),
                session,
                choices: Vec::new(),
                status: BranchExperimentBranchStatusV1::Active,
                stop_reason: "initial".to_string(),
                retention: default_branch_retention_decision_v1(),
                final_boss_combat_record: prefix_final_boss_combat_record,
            },
            config,
            0,
            None,
        ),
    )
}

fn run_branch_experiment_from_session_with_replay_and_snapshots(
    mut session: RunControlSession,
    config: &BranchExperimentConfigV1,
    replay_trace_applied_steps: usize,
    replay_trace_stop: Option<String>,
) -> BranchExperimentRunResultV1 {
    session.set_auto_capture_config(config.auto_capture.clone());
    run_branch_experiment_from_start_branch_with_replay_and_snapshots(
        BranchWork {
            id: "root".to_string(),
            session,
            choices: Vec::new(),
            status: BranchExperimentBranchStatusV1::Active,
            stop_reason: "initial".to_string(),
            retention: default_branch_retention_decision_v1(),
            final_boss_combat_record: None,
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
    run_branch_experiment_from_start_branch_with_replay_and_snapshots(
        start_branch,
        config,
        replay_trace_applied_steps,
        replay_trace_stop,
    )
    .report
}

fn apply_branch_experiment_prefix_commands_v1(
    session: &mut RunControlSession,
    config: &BranchExperimentConfigV1,
    prefix_commands: &[String],
) -> Result<Option<BranchExperimentBossCombatRecordV1>, String> {
    let mut prefix_final_boss_combat_record = None;
    for command_line in prefix_commands {
        if command_line == BRANCH_EXPERIMENT_REPLAY_ADVANCE_COMMAND {
            let outcome = crate::eval::run_control::apply_branch_experiment_auto_run(
                session,
                RunControlAutoStepOptions {
                    search: branch_experiment_search_options(config),
                    max_operations: Some(config.auto_max_operations),
                    route: RunControlRouteAutomationMode::Planner,
                },
            )?;
            prefix_final_boss_combat_record =
                final_boss_combat_record_from_annotations_v1(session, &outcome.trace_annotations)
                    .or(prefix_final_boss_combat_record);
        } else {
            apply_branch_choice(session, command_line)?;
        }
    }
    Ok(prefix_final_boss_combat_record)
}

fn run_branch_experiment_from_start_branch_with_replay_and_snapshots(
    start_branch: BranchWork,
    config: &BranchExperimentConfigV1,
    replay_trace_applied_steps: usize,
    replay_trace_stop: Option<String>,
) -> BranchExperimentRunResultV1 {
    let started_at = Instant::now();
    let mut branches = vec![start_branch];
    let report_seed = branches[0].session.run_state.seed;
    let mut explored_branch_points = 0usize;
    let mut branch_limit_hit = false;
    let mut frontier_group_limit_hit = false;
    let mut wall_limit_hit = false;
    let mut wall_limit_phase = None;
    let mut pruned_branch_count = 0usize;
    let mut pruned_first_pick_counts = BTreeMap::<String, usize>::new();
    let mut pruned_branch_summary = BranchExperimentPrunedBranchSummaryV1::default();
    let mut reward_option_portfolios = Vec::new();
    let mut shop_plan_candidate_pools = Vec::new();
    let mut campfire_plan_candidate_pools = Vec::new();
    let mut event_candidate_pools = Vec::new();
    let mut boss_relic_candidate_pools = Vec::new();
    let mut route_decisions = Vec::new();
    let mut route_candidate_pools = Vec::new();
    let mut decision_parent_sessions = BTreeMap::<Vec<String>, RunControlSession>::new();
    let mut combat_performance_samples = Vec::new();

    for depth in 0..config.max_depth {
        if experiment_wall_limit_hit(started_at, config) {
            wall_limit_hit = true;
            wall_limit_phase.get_or_insert(BranchExperimentWallLimitPhaseV1::Expansion);
            break;
        }
        let mut next = Vec::new();
        let mut expanded_any = false;

        for mut branch in branches {
            if experiment_wall_limit_hit(started_at, config) {
                wall_limit_hit = true;
                wall_limit_phase.get_or_insert(BranchExperimentWallLimitPhaseV1::Expansion);
                next.push(branch);
                continue;
            }
            if branch.status != BranchExperimentBranchStatusV1::Active {
                next.push(branch);
                continue;
            }

            advance_to_experiment_boundary(
                &mut branch,
                config,
                &mut route_decisions,
                &mut route_candidate_pools,
                &mut decision_parent_sessions,
                &mut combat_performance_samples,
            );
            if branch.status != BranchExperimentBranchStatusV1::Active {
                next.push(branch);
                continue;
            }

            let boundary_config = BranchBoundaryConfigV1 {
                max_reward_options_per_branch: config.max_reward_options_per_branch,
                max_campfire_options_per_branch: config.max_campfire_options_per_branch,
                include_skip: config.include_skip,
                include_event_reward_skip: config.include_event_reward_skip,
            };
            let frontier_before_boundary = branch_frontier(&branch.session);
            let reward_portfolio_context =
                config
                    .max_reward_options_per_branch
                    .map(|_| CardRewardPortfolioContext {
                        depth,
                        frontier_key: frontier_before_boundary.key.clone(),
                        boundary_title: frontier_before_boundary.boundary_title.clone(),
                    });
            if let Some(boundary) =
                current_branch_boundary(&branch.session, boundary_config, reward_portfolio_context)
            {
                let decision_parent_commands = branch_decision_parent_commands_v1(
                    &branch,
                    depth,
                    boundary.id,
                    &frontier_before_boundary.key,
                );
                let decision_parent_choices = branch_choice_labels_v1(&branch.choices);
                decision_parent_sessions
                    .entry(decision_parent_commands.clone())
                    .or_insert_with(|| branch.session.clone());
                if let Some(mut portfolio) = boundary.reward_option_portfolio {
                    portfolio.branch_id = branch.id.clone();
                    portfolio.branch_choices = decision_parent_choices.clone();
                    portfolio.branch_commands = decision_parent_commands.clone();
                    reward_option_portfolios.push(portfolio);
                }
                if let Some(mut pool) = boundary.shop_plan_candidate_pool {
                    pool.branch_id = branch.id.clone();
                    pool.branch_choices = decision_parent_choices.clone();
                    pool.branch_commands = decision_parent_commands.clone();
                    pool.depth = depth;
                    pool.frontier_key = frontier_before_boundary.key.clone();
                    pool.boundary_title = frontier_before_boundary.boundary_title.clone();
                    shop_plan_candidate_pools.push(pool);
                }
                if let Some(mut pool) = boundary.campfire_plan_candidate_pool {
                    pool.branch_id = branch.id.clone();
                    pool.branch_choices = decision_parent_choices.clone();
                    pool.branch_commands = decision_parent_commands.clone();
                    pool.depth = depth;
                    pool.frontier_key = frontier_before_boundary.key.clone();
                    pool.boundary_title = frontier_before_boundary.boundary_title.clone();
                    campfire_plan_candidate_pools.push(pool);
                }
                if let Some(mut pool) = boundary.event_candidate_pool {
                    pool.branch_id = branch.id.clone();
                    pool.branch_choices = decision_parent_choices.clone();
                    pool.branch_commands = decision_parent_commands.clone();
                    pool.depth = depth;
                    pool.frontier_key = frontier_before_boundary.key.clone();
                    pool.boundary_title = frontier_before_boundary.boundary_title.clone();
                    event_candidate_pools.push(pool);
                }
                if let Some(mut pool) = boundary.boss_relic_candidate_pool {
                    pool.branch_id = branch.id.clone();
                    pool.branch_choices = decision_parent_choices.clone();
                    pool.branch_commands = decision_parent_commands.clone();
                    pool.depth = depth;
                    pool.frontier_key = frontier_before_boundary.key.clone();
                    pool.boundary_title = frontier_before_boundary.boundary_title.clone();
                    boss_relic_candidate_pools.push(pool);
                }
                if boundary.options.is_empty() {
                    branch.status = BranchExperimentBranchStatusV1::NeedsHumanBoundary;
                    branch.stop_reason = boundary.id.empty_portfolio_reason().to_string();
                    next.push(branch);
                    continue;
                }

                explored_branch_points = explored_branch_points.saturating_add(1);
                expanded_any = true;
                let boundary_title = current_boundary_title(&branch.session);
                for option in boundary.options {
                    next.push(expand_branch_choice(
                        &branch,
                        BranchChoiceDraft {
                            depth,
                            kind: option.kind,
                            boundary_title: boundary_title.clone(),
                            label: option.label,
                            command: option.command,
                            action: option.action,
                            card: option.card,
                            upgrades: option.upgrades,
                            selected_cards: option.selected_cards,
                            effect_kind: option.effect_kind,
                            effect_key: option.effect_key,
                            effect_label: option.effect_label,
                            representative_count: option.representative_count,
                            suppressed_count: option.suppressed_count,
                            decision_signal: option.decision_signal,
                            success_reason: option.success_reason,
                        },
                        config,
                    ));
                }
                continue;
            }

            mark_unbranchable_boundary(&mut branch);
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
            wall_limit_phase.get_or_insert(BranchExperimentWallLimitPhaseV1::FinalSettle);
            break;
        }
        settle_branch_to_frontier(
            branch,
            config,
            &mut route_decisions,
            &mut route_candidate_pools,
            &mut decision_parent_sessions,
            &mut combat_performance_samples,
        );
    }

    let mut branch_sessions = branches
        .iter()
        .map(|branch| (branch.id.clone(), branch.session.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut branch_reports = branches
        .into_iter()
        .map(|branch| {
            let summary = run_summary(&branch.session, &branch.choices);
            let frontier = branch_frontier(&branch.session);
            BranchExperimentBranchReportV1 {
                rank_key: branch_effective_rank_key(&branch),
                retention: branch.retention,
                branch_id: branch.id,
                status: branch.status,
                choices: branch.choices,
                stop_reason: branch.stop_reason,
                summary,
                frontier,
                final_boss_combat_record: branch.final_boss_combat_record,
                boundary_details: branch_boundary_details(&branch.session),
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

    let report = BranchExperimentReportV1 {
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
        wall_limit_phase,
        elapsed_wall_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        pruned_branch_count,
        pruned_first_pick_counts: pruned_first_pick_count_reports(pruned_first_pick_counts),
        pruned_branch_summary,
        reward_option_portfolios,
        shop_plan_candidate_pools,
        campfire_plan_candidate_pools,
        event_candidate_pools,
        boss_relic_candidate_pools,
        strategy_requests: branch_strategy_requests(&branch_reports),
        route_decisions,
        route_candidate_pools,
        frontier_groups: frontier_groups(&branch_reports),
        branches: branch_reports,
    };
    branch_sessions.retain(|branch_id, _| {
        report
            .branches
            .iter()
            .any(|branch| branch.branch_id == *branch_id)
    });
    BranchExperimentRunResultV1 {
        report,
        branch_sessions,
        decision_parent_sessions,
        start_elapsed_wall_ms: 0,
        combat_performance_samples,
    }
}

fn branch_boundary_details(session: &RunControlSession) -> Vec<String> {
    let surface = build_decision_surface(session);
    let mut details =
        crate::eval::event_boundary_packet_v1::event_boundary_packet_from_session_v1(session)
            .map(|packet| {
                crate::eval::event_boundary_packet_v1::event_boundary_detail_lines_v1(&packet, 3)
            })
            .unwrap_or_default();
    details.extend(
        crate::eval::reward_boundary_packet_v1::reward_boundary_packet_from_session_v1(session)
            .map(|packet| {
                crate::eval::reward_boundary_packet_v1::reward_boundary_detail_lines_v1(&packet, 4)
            })
            .unwrap_or_default(),
    );
    details.extend(surface.view.context.into_iter().take(3));
    details.extend(
        surface
            .view
            .candidates
            .into_iter()
            .take(8)
            .map(|candidate| {
                format!(
                    "{} | {} | {}",
                    candidate.id,
                    candidate.label,
                    candidate.action.command_hint()
                )
            }),
    );
    details
}

fn experiment_wall_limit_hit(started_at: Instant, config: &BranchExperimentConfigV1) -> bool {
    let Some(limit_ms) = config.experiment_wall_ms else {
        return false;
    };
    started_at.elapsed().as_millis() >= u128::from(limit_ms)
}

fn elapsed_ms_u64(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn advance_to_experiment_boundary(
    branch: &mut BranchWork,
    config: &BranchExperimentConfigV1,
    route_decisions: &mut Vec<BranchExperimentRouteDecisionV1>,
    route_candidate_pools: &mut Vec<BranchExperimentRouteCandidatePoolV1>,
    decision_parent_sessions: &mut BTreeMap<Vec<String>, RunControlSession>,
    combat_performance_samples: &mut Vec<CombatSearchPerformanceSnapshotV1>,
) {
    if is_terminal(&branch.session) || experiment_branch_options_available(&branch.session) {
        update_terminal_status(branch);
        return;
    }

    let outcome = crate::eval::run_control::apply_branch_experiment_auto_run(
        &mut branch.session,
        RunControlAutoStepOptions {
            search: branch_experiment_search_options(config),
            max_operations: Some(config.auto_max_operations),
            route: RunControlRouteAutomationMode::Planner,
        },
    );

    match outcome {
        Ok(outcome) => {
            let route_parent_commands =
                route_decision_parent_commands_from_snapshots_v1(branch, &outcome);
            for (commands, session) in route_parent_commands.values() {
                decision_parent_sessions
                    .entry(commands.clone())
                    .or_insert_with(|| session.clone());
            }
            route_decisions.extend(outcome.trace_annotations.iter().enumerate().filter_map(
                |(annotation_index, annotation)| {
                    branch_route_decision_from_annotation(
                        branch,
                        annotation,
                        route_parent_commands
                            .get(&annotation_index)
                            .map(|(commands, _)| commands.as_slice()),
                    )
                },
            ));
            route_candidate_pools.extend(outcome.trace_annotations.iter().enumerate().filter_map(
                |(annotation_index, annotation)| {
                    branch_route_candidate_pool_from_annotation(
                        branch,
                        annotation,
                        route_parent_commands
                            .get(&annotation_index)
                            .map(|(commands, _)| commands.as_slice()),
                    )
                },
            ));
            combat_performance_samples.extend(
                outcome
                    .trace_annotations
                    .iter()
                    .filter_map(combat_performance_snapshot_from_annotation),
            );
            branch.stop_reason =
                if outcome_has_combat_turn_segment_progress(&outcome.trace_annotations)
                    && normalized_boundary_title(&current_boundary_title(&branch.session))
                        == "combat"
                {
                    COMBAT_TURN_SEGMENT_PROGRESS_STOP_REASON.to_string()
                } else {
                    first_reason_line(&outcome.message)
                        .unwrap_or_else(|| current_boundary_title(&branch.session))
                };
            update_terminal_status(branch);
            if branch.status == BranchExperimentBranchStatusV1::TerminalVictory {
                branch.final_boss_combat_record = final_boss_combat_record_from_annotations_v1(
                    &branch.session,
                    &outcome.trace_annotations,
                )
                .or_else(|| branch.final_boss_combat_record.clone());
            }
        }
        Err(err) => {
            branch.status = BranchExperimentBranchStatusV1::Failed;
            branch.stop_reason = err;
        }
    }
}

fn branch_experiment_search_options(
    config: &BranchExperimentConfigV1,
) -> RunControlSearchCombatOptions {
    let mut options = config.search_options.clone();
    if config.search_max_nodes.is_some() {
        options.max_nodes = config.search_max_nodes;
    }
    if config.search_wall_ms.is_some() {
        options.wall_ms = config.search_wall_ms;
    }
    if config.search_max_hp_loss.is_some() {
        options.max_hp_loss = config.search_max_hp_loss;
    }
    options
}

fn route_decision_parent_commands_from_snapshots_v1(
    branch: &BranchWork,
    outcome: &RunControlCommandOutcome,
) -> BTreeMap<usize, (Vec<String>, RunControlSession)> {
    let mut snapshots = outcome
        .decision_parent_snapshots
        .iter()
        .filter(|snapshot| snapshot.source == "route_planner");
    let mut route_ordinal = 0usize;
    let mut result = BTreeMap::new();
    for (annotation_index, annotation) in outcome.trace_annotations.iter().enumerate() {
        if !matches!(
            annotation,
            RunControlTraceAnnotationV1::RoutePlannerSelection { .. }
        ) {
            continue;
        }
        let Some(snapshot) = snapshots.next() else {
            continue;
        };
        let Ok(session) = snapshot.snapshot.clone().into_session() else {
            continue;
        };
        let mut commands = branch_choice_commands_v1(&branch.choices);
        commands.push(route_decision_anchor_command_v1(
            route_ordinal,
            &snapshot.command,
        ));
        result.insert(annotation_index, (commands, session));
        route_ordinal = route_ordinal.saturating_add(1);
    }
    result
}

fn route_decision_anchor_command_v1(index: usize, command: &str) -> String {
    let command_key = command
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("__route_decision:{index}:{command_key}")
}

fn branch_route_decision_from_annotation(
    branch: &BranchWork,
    annotation: &RunControlTraceAnnotationV1,
    parent_commands: Option<&[String]>,
) -> Option<BranchExperimentRouteDecisionV1> {
    let RunControlTraceAnnotationV1::RoutePlannerSelection {
        selected_index,
        target_x,
        target_y,
        room_type,
        move_kind,
        safety,
        command,
        map_decision_packet,
        route_evidence: Some(evidence),
        ..
    } = annotation
    else {
        return None;
    };

    Some(BranchExperimentRouteDecisionV1 {
        branch_id: branch.id.clone(),
        branch_choices: branch_choice_labels_v1(&branch.choices),
        branch_commands: parent_commands
            .map(|commands| commands.to_vec())
            .unwrap_or_else(|| branch_choice_commands_v1(&branch.choices)),
        selected_index: *selected_index,
        selected_candidate_id: selected_index.and_then(|index| {
            map_decision_packet
                .as_ref()
                .and_then(|packet| packet.candidates.get(index))
                .map(|candidate| candidate.candidate_id.clone())
        }),
        target: format!("x={target_x} y={target_y} {room_type}"),
        move_kind: move_kind.clone(),
        safety: safety.clone(),
        command: command.clone(),
        elite_prep_bp: evidence.elite_prep_bp,
        first_elite: BranchExperimentFirstEliteEvidenceV1 {
            paths_with_first_elite: evidence.first_elite.paths_with_first_elite,
            forced: evidence.first_elite.forced,
            optional: evidence.first_elite.optional,
            min_hallway_fights_before: evidence.first_elite.min_hallway_fights_before,
            max_hallway_fights_before: evidence.first_elite.max_hallway_fights_before,
            min_unknowns_before: evidence.first_elite.min_unknowns_before,
            max_unknowns_before: evidence.first_elite.max_unknowns_before,
            min_fires_before: evidence.first_elite.min_fires_before,
            max_fires_before: evidence.first_elite.max_fires_before,
            min_shops_before: evidence.first_elite.min_shops_before,
            max_shops_before: evidence.first_elite.max_shops_before,
            can_bail_to_rest_before: evidence.first_elite.can_bail_to_rest_before,
            can_bail_to_shop_before: evidence.first_elite.can_bail_to_shop_before,
        },
    })
}

fn branch_route_candidate_pool_from_annotation(
    branch: &BranchWork,
    annotation: &RunControlTraceAnnotationV1,
    parent_commands: Option<&[String]>,
) -> Option<BranchExperimentRouteCandidatePoolV1> {
    let RunControlTraceAnnotationV1::RoutePlannerSelection {
        selected_index,
        candidate_count,
        candidate_pool,
        map_decision_packet,
        ..
    } = annotation
    else {
        return None;
    };
    if let Some(packet) = map_decision_packet {
        if packet.candidates.is_empty() {
            return None;
        }
        return Some(branch_route_candidate_pool_from_map_packet_v1(
            branch,
            packet,
            parent_commands,
        ));
    }
    if candidate_pool.is_empty() {
        return None;
    }

    Some(BranchExperimentRouteCandidatePoolV1 {
        branch_id: branch.id.clone(),
        branch_choices: branch_choice_labels_v1(&branch.choices),
        branch_commands: parent_commands
            .map(|commands| commands.to_vec())
            .unwrap_or_else(|| branch_choice_commands_v1(&branch.choices)),
        decision_id: format!("{}:route_candidate_pool", branch.id),
        boundary_title: "Map".to_string(),
        frontier_key: branch.id.clone(),
        depth: 0,
        candidate_count: *candidate_count,
        selected_index: *selected_index,
        candidate_pool_provenance: None,
        map_decision_packet: None,
        candidates: candidate_pool
            .iter()
            .map(|candidate| BranchExperimentRouteCandidateEntryV1 {
                candidate_id: format!("route:{}:{}", candidate.rank, candidate.command),
                rank: candidate.rank,
                selected: Some(candidate.rank) == *selected_index,
                target_node: None,
                target: format!(
                    "x={} y={} {}",
                    candidate.target_x, candidate.target_y, candidate.room_type
                ),
                room_type: candidate.room_type.clone(),
                move_kind: candidate.move_kind.clone(),
                safety_flag: None,
                safety: candidate.safety.clone(),
                score: candidate.score,
                score_terms: None,
                command: candidate.command.clone(),
                node_features: None,
                path_summary: None,
                needs: None,
                elite_prep_bp: candidate.elite_prep_bp,
                first_elite: BranchExperimentFirstEliteEvidenceV1 {
                    paths_with_first_elite: candidate.first_elite.paths_with_first_elite,
                    forced: candidate.first_elite.forced,
                    optional: candidate.first_elite.optional,
                    min_hallway_fights_before: candidate.first_elite.min_hallway_fights_before,
                    max_hallway_fights_before: candidate.first_elite.max_hallway_fights_before,
                    min_unknowns_before: candidate.first_elite.min_unknowns_before,
                    max_unknowns_before: candidate.first_elite.max_unknowns_before,
                    min_fires_before: candidate.first_elite.min_fires_before,
                    max_fires_before: candidate.first_elite.max_fires_before,
                    min_shops_before: candidate.first_elite.min_shops_before,
                    max_shops_before: candidate.first_elite.max_shops_before,
                    can_bail_to_rest_before: candidate.first_elite.can_bail_to_rest_before,
                    can_bail_to_shop_before: candidate.first_elite.can_bail_to_shop_before,
                },
                reasons: candidate.reasons.clone(),
                cautions: candidate.cautions.clone(),
            })
            .collect(),
    })
}

fn branch_route_candidate_pool_from_map_packet_v1(
    branch: &BranchWork,
    packet: &MapDecisionPacketV1,
    parent_commands: Option<&[String]>,
) -> BranchExperimentRouteCandidatePoolV1 {
    BranchExperimentRouteCandidatePoolV1 {
        branch_id: branch.id.clone(),
        branch_choices: branch_choice_labels_v1(&branch.choices),
        branch_commands: parent_commands
            .map(|commands| commands.to_vec())
            .unwrap_or_else(|| branch_choice_commands_v1(&branch.choices)),
        decision_id: format!("{}:route_candidate_pool", branch.id),
        boundary_title: "Map".to_string(),
        frontier_key: branch.id.clone(),
        depth: 0,
        candidate_count: packet.candidates.len(),
        selected_index: packet.selected_index,
        candidate_pool_provenance: Some(packet.candidate_pool.clone()),
        map_decision_packet: Some(packet.clone()),
        candidates: packet
            .candidates
            .iter()
            .map(|candidate| {
                branch_route_candidate_entry_from_map_packet_candidate_v1(
                    packet.selected_index,
                    candidate,
                )
            })
            .collect(),
    }
}

fn branch_route_candidate_entry_from_map_packet_candidate_v1(
    selected_index: Option<usize>,
    candidate: &RouteMoveCandidateV1,
) -> BranchExperimentRouteCandidateEntryV1 {
    let elite = &candidate.projection.path_summary.first_elite;
    BranchExperimentRouteCandidateEntryV1 {
        candidate_id: candidate.candidate_id.clone(),
        rank: candidate.rank,
        selected: Some(candidate.rank) == selected_index,
        target_node: Some(candidate.target.clone()),
        target: branch_route_target_label_v1(&candidate.target),
        room_type: branch_route_room_type_label_v1(candidate.target.room_type),
        move_kind: format!("{:?}", candidate.target.move_kind),
        safety_flag: Some(candidate.evaluation.safety),
        safety: branch_route_safety_label_v1(candidate.evaluation.safety).to_string(),
        score: candidate.evaluation.total_score,
        score_terms: Some(candidate.evaluation.score_terms.clone()),
        command: candidate.command.clone(),
        node_features: Some(candidate.features.clone()),
        path_summary: Some(candidate.projection.path_summary.clone()),
        needs: Some(candidate.needs.clone()),
        elite_prep_bp: score_to_basis_points(candidate.evaluation.score_terms.elite_prep),
        first_elite: BranchExperimentFirstEliteEvidenceV1 {
            paths_with_first_elite: elite.paths_with_first_elite,
            forced: elite.forced,
            optional: elite.optional,
            min_hallway_fights_before: elite.min_hallway_fights_before,
            max_hallway_fights_before: elite.max_hallway_fights_before,
            min_unknowns_before: elite.min_unknowns_before,
            max_unknowns_before: elite.max_unknowns_before,
            min_fires_before: elite.min_fires_before,
            max_fires_before: elite.max_fires_before,
            min_shops_before: elite.min_shops_before,
            max_shops_before: elite.max_shops_before,
            can_bail_to_rest_before: elite.can_bail_to_rest_before,
            can_bail_to_shop_before: elite.can_bail_to_shop_before,
        },
        reasons: candidate.evaluation.legacy_reasons.clone(),
        cautions: candidate.evaluation.legacy_cautions.clone(),
    }
}

fn branch_route_target_label_v1(target: &crate::ai::route_planner_v1::MapRouteTargetV1) -> String {
    format!(
        "x={} y={} {}",
        target.x,
        target.y,
        branch_route_room_type_label_v1(target.room_type)
    )
}

fn branch_route_room_type_label_v1(room_type: Option<RoomType>) -> String {
    match room_type {
        Some(RoomType::EventRoom) => "Event",
        Some(RoomType::MonsterRoom) => "Monster",
        Some(RoomType::MonsterRoomElite) => "Elite",
        Some(RoomType::MonsterRoomBoss) => "Boss",
        Some(RoomType::RestRoom) => "Rest",
        Some(RoomType::ShopRoom) => "Shop",
        Some(RoomType::TreasureRoom) => "Treasure",
        Some(RoomType::TrueVictoryRoom) => "Victory",
        None => "Unknown",
    }
    .to_string()
}

fn branch_route_safety_label_v1(safety: RouteSafetyFlagV1) -> &'static str {
    match safety {
        RouteSafetyFlagV1::Ok => "ok",
        RouteSafetyFlagV1::RiskyButAllowed => "risky_but_allowed",
        RouteSafetyFlagV1::RejectUnlessNoAlternative => "reject_unless_forced",
    }
}

fn score_to_basis_points(score: f32) -> i32 {
    (score * 100.0).round() as i32
}

fn branch_choice_labels_v1(choices: &[BranchExperimentChoiceV1]) -> Vec<String> {
    choices.iter().map(|choice| choice.label.clone()).collect()
}

fn branch_choice_commands_v1(choices: &[BranchExperimentChoiceV1]) -> Vec<String> {
    choices
        .iter()
        .map(|choice| choice.command.clone())
        .collect()
}

fn branch_decision_parent_commands_v1(
    branch: &BranchWork,
    depth: usize,
    boundary_id: BranchBoundaryIdV1,
    frontier_key: &str,
) -> Vec<String> {
    let mut commands = branch_choice_commands_v1(&branch.choices);
    commands.push(format!(
        "{}{}:{}:{}",
        BRANCH_EXPERIMENT_DECISION_PARENT_COMMAND_PREFIX_V1,
        depth,
        branch_boundary_id_label_v1(boundary_id),
        stable_text_hash_hex_v1(frontier_key)
    ));
    commands
}

fn branch_boundary_id_label_v1(boundary_id: BranchBoundaryIdV1) -> &'static str {
    match boundary_id {
        BranchBoundaryIdV1::CardReward => "reward",
        BranchBoundaryIdV1::Campfire => "campfire",
        BranchBoundaryIdV1::BossRelic => "boss_relic",
        BranchBoundaryIdV1::RunSelection => "run_selection",
        BranchBoundaryIdV1::Reward => "reward_claim",
        BranchBoundaryIdV1::Shop => "shop",
        BranchBoundaryIdV1::Event => "event",
    }
}

fn stable_text_hash_hex_v1(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn combat_performance_snapshot_from_annotation(
    annotation: &RunControlTraceAnnotationV1,
) -> Option<CombatSearchPerformanceSnapshotV1> {
    match annotation {
        RunControlTraceAnnotationV1::CombatSearchPerformance { snapshot } => Some(snapshot.clone()),
        _ => None,
    }
}

fn final_boss_combat_record_from_annotations_v1(
    session: &RunControlSession,
    annotations: &[RunControlTraceAnnotationV1],
) -> Option<BranchExperimentBossCombatRecordV1> {
    if !matches!(
        session.engine_state,
        EngineState::GameOver(RunResult::Victory)
    ) {
        return None;
    }
    if let Some(record) = session.last_completed_combat_automation_trajectory() {
        return Some(BranchExperimentBossCombatRecordV1 {
            source: record.source.clone(),
            action_count: record.action_count,
            actions: record.actions.clone(),
            label_role: record.label_role.clone(),
        });
    }
    combat_automation_trajectories_v1(annotations)
        .last()
        .map(|trajectory| BranchExperimentBossCombatRecordV1 {
            source: trajectory.source.to_string(),
            action_count: trajectory.action_count,
            actions: trajectory.actions.to_vec(),
            label_role: trajectory.label_role.to_string(),
        })
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

fn settle_branch_to_frontier(
    branch: &mut BranchWork,
    config: &BranchExperimentConfigV1,
    route_decisions: &mut Vec<BranchExperimentRouteDecisionV1>,
    route_candidate_pools: &mut Vec<BranchExperimentRouteCandidatePoolV1>,
    decision_parent_sessions: &mut BTreeMap<Vec<String>, RunControlSession>,
    combat_performance_samples: &mut Vec<CombatSearchPerformanceSnapshotV1>,
) {
    if branch.status != BranchExperimentBranchStatusV1::Active {
        return;
    }
    advance_to_experiment_boundary(
        branch,
        config,
        route_decisions,
        route_candidate_pools,
        decision_parent_sessions,
        combat_performance_samples,
    );
    if branch.status != BranchExperimentBranchStatusV1::Active || is_terminal(&branch.session) {
        return;
    }
    if !experiment_branch_options_available(&branch.session) {
        mark_unbranchable_boundary(branch);
    }
}

fn mark_unbranchable_boundary(branch: &mut BranchWork) {
    let boundary_title = current_boundary_title(&branch.session);
    if is_combat_turn_segment_progress_boundary(&boundary_title, &branch.stop_reason) {
        return;
    } else if is_budget_unresolved_combat_boundary(&boundary_title, &branch.stop_reason) {
        branch.status = BranchExperimentBranchStatusV1::Pruned;
    } else {
        branch.status = BranchExperimentBranchStatusV1::NeedsHumanBoundary;
        if branch.stop_reason == "initial" || branch.stop_reason.is_empty() {
            branch.stop_reason = boundary_title;
        }
    }
}

fn is_budget_unresolved_combat_boundary(boundary_title: &str, stop_reason: &str) -> bool {
    normalized_boundary_title(boundary_title) == "combat"
        && stop_reason
            .to_ascii_lowercase()
            .contains("combat search did not find an executable complete win")
}

fn is_combat_turn_segment_progress_boundary(boundary_title: &str, stop_reason: &str) -> bool {
    normalized_boundary_title(boundary_title) == "combat"
        && stop_reason.eq_ignore_ascii_case(COMBAT_TURN_SEGMENT_PROGRESS_STOP_REASON)
}

fn outcome_has_combat_turn_segment_progress(annotations: &[RunControlTraceAnnotationV1]) -> bool {
    combat_automation_trajectories_v1(annotations)
        .any(|trajectory| trajectory.source == "search_combat_turn_segment")
}

fn normalized_boundary_title(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn experiment_branch_options_available(session: &RunControlSession) -> bool {
    branch_boundary_available(session)
}

struct BranchChoiceDraft {
    depth: usize,
    kind: &'static str,
    boundary_title: String,
    label: String,
    command: String,
    action: BranchBoundaryActionV1,
    card: Option<CardId>,
    upgrades: Option<u8>,
    selected_cards: Vec<BranchExperimentChoiceCardV1>,
    effect_kind: String,
    effect_key: String,
    effect_label: String,
    representative_count: usize,
    suppressed_count: usize,
    decision_signal: Option<BranchExperimentChoiceDecisionSignalV1>,
    success_reason: &'static str,
}

fn expand_branch_choice(
    branch: &BranchWork,
    draft: BranchChoiceDraft,
    config: &BranchExperimentConfigV1,
) -> BranchWork {
    let mut child = branch.clone();
    child.id = format!("{}.{}", child.id, draft.command);
    let maybe_auto_leave_after_purchase =
        config.auto_leave_after_shop_purchase_branch && is_shop_purchase_effect(&draft.effect_kind);
    child.choices.push(BranchExperimentChoiceV1 {
        depth: draft.depth,
        kind: draft.kind.to_string(),
        boundary_title: draft.boundary_title,
        card: draft.card,
        upgrades: draft.upgrades,
        selected_cards: draft.selected_cards,
        effect_kind: draft.effect_kind,
        effect_key: draft.effect_key,
        effect_label: draft.effect_label,
        representative_count: draft.representative_count,
        suppressed_count: draft.suppressed_count,
        decision_signal: draft.decision_signal,
        label: draft.label,
        command: draft.command.clone(),
    });
    match apply_branch_action(&mut child.session, &draft.action).and_then(|changes| {
        if let Some(choice) = child.choices.last_mut() {
            append_branch_action_result_to_choice(choice, &changes);
        }
        if maybe_auto_leave_after_purchase && shop_should_auto_leave_after_purchase(&child.session)
        {
            let _ = apply_branch_choice(&mut child.session, "leave")?;
            if let Some(choice) = child.choices.last_mut() {
                choice.effect_label = format!("{} | auto leave shop", choice.effect_label);
            }
        }
        Ok(())
    }) {
        Ok(()) => {
            child.stop_reason = draft.success_reason.to_string();
            update_terminal_status(&mut child);
            if !config.defer_branch_settle {
                let mut ignored_route_decisions = Vec::new();
                let mut ignored_route_candidate_pools = Vec::new();
                let mut ignored_decision_parent_sessions = BTreeMap::new();
                let mut ignored_combat_performance = Vec::new();
                settle_branch_to_frontier(
                    &mut child,
                    config,
                    &mut ignored_route_decisions,
                    &mut ignored_route_candidate_pools,
                    &mut ignored_decision_parent_sessions,
                    &mut ignored_combat_performance,
                );
            }
        }
        Err(err) => {
            child.status = BranchExperimentBranchStatusV1::Failed;
            child.stop_reason = err;
        }
    }
    child
}

fn is_shop_purchase_effect(effect_kind: &str) -> bool {
    matches!(
        effect_kind,
        "shop_buy_card" | "shop_buy_relic" | "shop_buy_potion" | "shop_buy_combo"
    )
}

fn shop_should_auto_leave_after_purchase(session: &RunControlSession) -> bool {
    let EngineState::Shop(shop) = &session.engine_state else {
        return false;
    };
    if shop.pending_reward_overlay.is_some() {
        return false;
    }
    let context = build_shop_decision_context_v1(&session.run_state, shop);
    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::BranchTopK { max_plans: 4 },
    );
    !compiled_shop_decision_has_executable_conversion_branch_v1(&compiled)
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
        .map(|(index, branch)| branch_retention_candidate_input(index, branch))
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
        &branches,
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
            .then_with(|| applied_retention_rank_key(right).cmp(&applied_retention_rank_key(left)))
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

fn branch_retention_candidate_input(
    index: usize,
    branch: &BranchWork,
) -> BranchRetentionCandidateInputV1 {
    let choice_profiles = branch_choice_profiles(branch);
    let recent_choice_profiles = branch_recent_choice_profiles(branch);
    let choice_effect_keys = branch_choice_effect_keys(branch);
    let frontier = branch_frontier(&branch.session);
    let (hp, max_hp) = visible_player_hp(&branch.session);
    BranchRetentionCandidateInputV1 {
        index,
        act: branch.session.run_state.act_num,
        floor: branch.session.run_state.floor_num,
        frontier_key: frontier.key,
        rank_key: branch_rank_key(branch),
        hp,
        max_hp,
        gold: branch.session.run_state.gold,
        deck_count: branch.session.run_state.master_deck.len(),
        curse_count: branch_curse_count(&branch.session.run_state),
        strategy_formation: Some(strategy_formation_summary(&branch.session)),
        trajectory: summarize_branch_trajectory_v1(&choice_profiles),
        recent_choice_profiles,
        choice_profiles,
        choice_effect_keys,
        lineage_flags: frontier.lineage.sequence_breakers_present,
        decision_signals: branch
            .choices
            .iter()
            .filter_map(|choice| choice.decision_signal.clone())
            .collect(),
        strategic_debt_tags: branch_strategic_debt_tags(&branch.session),
        startup: crate::ai::deck_startup_profile_v1::deck_startup_profile_v1(
            &branch.session.run_state,
        ),
    }
}

fn branch_curse_count(run_state: &RunState) -> usize {
    run_state
        .master_deck
        .iter()
        .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
        .count()
}

fn branch_effective_rank_key(branch: &BranchWork) -> i32 {
    let candidate = branch_retention_candidate_input(0, branch);
    branch_retention_order_rank_key_v1(&candidate)
}

fn applied_retention_rank_key(branch: &BranchWork) -> i32 {
    let adjustment = &branch.retention.rank_adjustment;
    if adjustment.base_rank_key != 0 || adjustment.effective_rank_key != 0 {
        adjustment.effective_rank_key
    } else {
        branch_effective_rank_key(branch)
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
    branches: &[BranchWork],
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
        if let Some(branch) = branches.get(candidate.index) {
            for choice in &branch.choices {
                *summary
                    .choice_effect_counts
                    .entry(branch_choice_effect_key(choice).to_string())
                    .or_default() += 1;
            }
            for flag in branch_frontier(&branch.session)
                .lineage
                .sequence_breakers_present
            {
                *summary.lineage_flag_counts.entry(flag).or_default() += 1;
            }
        }
    }
    summary
}

fn branch_choice_effect_key(choice: &BranchExperimentChoiceV1) -> String {
    branch_experiment_choice_effect_key_v1(&choice.effect_kind)
}

fn branch_choice_effect_keys(branch: &BranchWork) -> Vec<String> {
    branch
        .choices
        .iter()
        .map(branch_choice_effect_key)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn branch_strategic_debt_tags(session: &RunControlSession) -> Vec<String> {
    let run_state = &session.run_state;
    let mut tags = run_debt_ledger_v1(run_state)
        .strategic_debt_tags()
        .into_iter()
        .collect::<BTreeSet<_>>();

    for relic in &run_state.relics {
        if !matches!(
            relic.id,
            RelicId::BottledFlame | RelicId::BottledLightning | RelicId::BottledTornado
        ) || relic.amount <= 0
        {
            continue;
        }
        let Some(card) = run_state
            .master_deck
            .iter()
            .find(|card| card.uuid as i32 == relic.amount)
        else {
            continue;
        };
        for tag in bottled_card_debt_tags_v1(run_state, relic.id, card.id, card.upgrades) {
            tags.insert(tag);
        }
    }

    tags.into_iter().collect()
}

fn bottled_card_debt_tags_v1(
    run_state: &RunState,
    relic: RelicId,
    card: CardId,
    upgrades: u8,
) -> Vec<String> {
    opening_hand_target_debt_tags_v1(run_state, relic, card, upgrades)
}

pub fn branch_experiment_choice_effect_key_v1(effect_kind: &str) -> String {
    if effect_kind.starts_with("boss_relic:") {
        return effect_kind.to_string();
    }
    match effect_kind {
        "" | "add_card" => "take_card",
        "skip_card_reward" => "skip_reward",
        "reward_skip_full_potion" => "reward_skip_full_potion",
        "singing_bowl" => "singing_bowl",
        "upgrade_card" => "upgrade_card",
        "rest" => "rest",
        "boss_relic" => "boss_relic",
        "event_choice" => "event_choice",
        "event_accept" => "event_accept",
        "event_card_reward" => "event_card_reward",
        "event_continue" => "event_continue",
        "event_decline" => "event_decline",
        "event_deck_operation" => "event_deck_operation",
        "event_duplicate_card" => "event_duplicate_card",
        "event_gain" => "event_gain",
        "event_gain_curse" => "event_gain_curse",
        "event_gain_gold" => "event_gain_gold",
        "event_gain_max_hp" => "event_gain_max_hp",
        "event_gain_potion" => "event_gain_potion",
        "event_gain_relic" => "event_gain_relic",
        "event_heal" => "event_heal",
        "event_leave" => "event_leave",
        "event_pay_resource" => "event_pay_resource",
        "event_remove_card" => "event_remove_card",
        "event_special" => "event_special",
        "event_start_combat" => "event_start_combat",
        "event_trade" => "event_trade",
        "event_transform_card" => "event_transform_card",
        "event_upgrade_card" => "event_upgrade_card",
        "remove_card" => "remove_card",
        "transform_card" => "transform_card",
        "duplicate_card" => "duplicate_card",
        "bottle_card" => "bottle_card",
        "shop_buy_card" => "shop_buy_card",
        "shop_buy_relic" => "shop_buy_relic",
        "shop_buy_potion" => "shop_buy_potion",
        "shop_buy_combo" => "shop_buy_combo",
        "shop_leave" => "shop_leave",
        "shop_purge" => "shop_purge",
        "dig" => "dig",
        "lift" => "lift",
        "recall" => "recall",
        _ => "other",
    }
    .to_string()
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
    merge_count_map(&mut total.choice_effect_counts, step.choice_effect_counts);
    merge_count_map(&mut total.lineage_flag_counts, step.lineage_flag_counts);
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

fn apply_branch_choice(
    session: &mut RunControlSession,
    command: &str,
) -> Result<Vec<RunActionResultChangeV1>, String> {
    let mut changes = Vec::new();
    for command in command
        .split(BRANCH_EXPERIMENT_COMMAND_SEQUENCE_SEPARATOR)
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let command = parse_run_control_command(command)?;
        let outcome = session.apply_command(command)?;
        changes.extend(branch_action_changes(&outcome));
    }
    Ok(changes)
}

fn apply_branch_action(
    session: &mut RunControlSession,
    action: &BranchBoundaryActionV1,
) -> Result<Vec<RunActionResultChangeV1>, String> {
    match action {
        BranchBoundaryActionV1::Command(command) => apply_branch_choice(session, command),
        BranchBoundaryActionV1::Inputs(inputs) => {
            let outcome = session.apply_command(
                crate::eval::run_control::RunControlCommand::InputSequence(inputs.clone()),
            )?;
            Ok(branch_action_changes(&outcome))
        }
    }
}

fn branch_action_changes(outcome: &RunControlCommandOutcome) -> Vec<RunActionResultChangeV1> {
    outcome
        .action_result
        .as_ref()
        .map(|result| result.changes.clone())
        .unwrap_or_default()
}

fn append_branch_action_result_to_choice(
    choice: &mut BranchExperimentChoiceV1,
    changes: &[RunActionResultChangeV1],
) {
    if choice.kind != "event" {
        return;
    }
    if !matches!(
        choice.effect_kind.as_str(),
        "remove_card" | "transform_card" | "upgrade_card"
    ) {
        return;
    }
    if choice.effect_label.contains("result:") {
        return;
    }
    let Some(summary) = branch_action_card_mutation_result_label(changes) else {
        return;
    };
    if choice.effect_label.is_empty() {
        choice.effect_label = format!("result: {summary}");
    } else {
        choice.effect_label = format!("{} | result: {summary}", choice.effect_label);
    }
}

fn branch_action_card_mutation_result_label(changes: &[RunActionResultChangeV1]) -> Option<String> {
    let mut parts = Vec::new();
    for change in changes {
        match change {
            RunActionResultChangeV1::CardRemoved { card } => {
                parts.push(format!("removed {}", branch_action_card_label(card)));
            }
            RunActionResultChangeV1::CardTransformed { before, after } => {
                parts.push(format!(
                    "{} -> {}",
                    branch_action_card_label(before),
                    branch_action_card_label(after)
                ));
            }
            RunActionResultChangeV1::CardUpgraded { before, after } => {
                parts.push(format!(
                    "upgraded {} -> {}",
                    branch_action_card_label(before),
                    branch_action_card_label(after)
                ));
            }
            _ => {}
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn branch_action_card_label(card: &RunActionCardSnapshotV1) -> String {
    let name = get_card_definition(card.id).name;
    if card.upgrades == 0 {
        name.to_string()
    } else {
        format!("{name}+{}", card.upgrades)
    }
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
            let (current_hp, _) = visible_player_hp(&branch.session);
            branch.session.run_state.act_num as i32 * 10_000
                + branch.session.run_state.floor_num * 100
                + current_hp * 10
                + branch_effective_gold_after_deck_debt(&branch.session)
        }
    }
}

fn branch_effective_gold_after_deck_debt(session: &RunControlSession) -> i32 {
    session
        .run_state
        .gold
        .saturating_sub(removable_curse_purge_debt_v1(&session.run_state))
}

fn removable_curse_purge_debt_v1(run_state: &RunState) -> i32 {
    let removable_curses = run_state
        .master_deck
        .iter()
        .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
        .filter(|card| master_deck_card_is_purgeable(card))
        .filter(|card| !master_deck_card_is_bottled(card, &run_state.relics))
        .count() as i32;
    if removable_curses <= 0 {
        return 0;
    }
    removable_curses
        .min(3)
        .saturating_mul(estimated_purge_cost_v1(run_state))
}

fn estimated_purge_cost_v1(run_state: &RunState) -> i32 {
    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::SmilingMask)
    {
        50
    } else {
        75 + run_state.shop_purge_count.saturating_mul(25)
    }
}

fn run_summary(
    session: &RunControlSession,
    choices: &[BranchExperimentChoiceV1],
) -> BranchExperimentRunSummaryV1 {
    let formation = strategy_formation_summary(session);
    let choice_profiles = choice_profiles_from_choices(choices);
    let (hp, max_hp) = visible_player_hp(session);
    BranchExperimentRunSummaryV1 {
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        hp,
        max_hp,
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

fn visible_player_hp(session: &RunControlSession) -> (i32, i32) {
    session.visible_player_hp()
}

fn branch_choice_profiles(
    branch: &BranchWork,
) -> Vec<crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1> {
    choice_profiles_from_choices(&branch.choices)
}

fn branch_recent_choice_profiles(
    branch: &BranchWork,
) -> Vec<crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1> {
    branch
        .choices
        .last()
        .map(choice_profiles_from_choice)
        .unwrap_or_default()
}

fn choice_profiles_from_choices(
    choices: &[BranchExperimentChoiceV1],
) -> Vec<crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1> {
    choices
        .iter()
        .flat_map(choice_profiles_from_choice)
        .collect()
}

fn choice_profiles_from_choice(
    choice: &BranchExperimentChoiceV1,
) -> Vec<crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1> {
    choice_profile_cards(choice)
        .into_iter()
        .map(|selected| {
            card_reward_semantic_profile_v1(&RewardCard::new(selected.card, selected.upgrades))
        })
        .collect()
}

fn choice_profile_cards(choice: &BranchExperimentChoiceV1) -> Vec<BranchExperimentChoiceCardV1> {
    if !branch_choice_effect_adds_card_profile(&choice.effect_kind) {
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

fn branch_choice_effect_adds_card_profile(effect_kind: &str) -> bool {
    matches!(
        effect_kind,
        "" | "add_card"
            | "duplicate_card"
            | "event_card_reward"
            | "event_duplicate_card"
            | "shop_buy_card"
            | "shop_buy_combo"
    )
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

#[cfg(test)]
mod tests;
