use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::content::cards::{get_card_definition, CardType};
use sts_simulator::eval::run_control::{
    apply_owner_audit_auto_run, CombatAutomationTrajectorySource, CombatSearchTraceSummary,
    RunControlAutoAppliedStepV1, RunControlAutoStepOptions, RunControlAutoStopKind,
    RunControlCommandOutcome, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession, RunControlTraceAnnotationV1,
};

use super::boundary_router;
use super::owner_orchestrator::{orchestrate_owner_boundary, OwnerOrchestration};
use super::render;
use super::{
    Args, BossRetryAttemptReport, BossRetryReport, BossRetryStatus, BranchStatus, RunDeadline,
    TerminalOutcome,
};

const BOSS_RETRY_POTION_RESCUE_MAX_POTIONS_USED: u32 = 3;
const HALLWAY_POTION_RESCUE_MAX_POTIONS_USED: u32 = 1;

struct HallwayPotionRescueAttempt {
    outcome: RunControlCommandOutcome,
    status: BranchStatus,
    committed: bool,
}

pub(super) struct AdvanceResult {
    pub(super) status: BranchStatus,
    pub(super) boss_retry: Option<BossRetryReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
}

pub(super) fn advance_to_owner_or_gap(
    session: &mut RunControlSession,
    args: Args,
    deadline: RunDeadline,
) -> AdvanceResult {
    let mut policy_steps = 0usize;
    let mut auto_ops_used = 0usize;
    let mut auto_steps = Vec::new();
    let mut combat_search = Vec::new();
    loop {
        let run_args = deadline.cap_args(args, 1);
        match apply_owner_audit_auto_run(session, primary_auto_step_options(run_args)) {
            Ok(outcome) => {
                let stop_kind = outcome.auto_stop.as_ref().map(|stop| stop.kind);
                auto_ops_used = auto_ops_used.saturating_add(
                    outcome
                        .auto_stop
                        .as_ref()
                        .map(|stop| stop.applied_operations)
                        .unwrap_or(0),
                );
                combat_search.extend(combat_search_summaries(&outcome));
                auto_steps.extend(outcome.auto_applied_steps.clone());
                let mut status = classify_auto_outcome(session, &outcome);
                if stop_kind == Some(RunControlAutoStopKind::OperationBudgetExhausted)
                    && auto_ops_used < args.auto_ops
                    && !deadline.should_stop()
                {
                    continue;
                }
                let combat_gap = matches!(status, BranchStatus::CombatGap { .. });
                let boss_combat = is_boss_combat(session);
                let combat_budget_capped = if boss_combat {
                    args.wall_capped_boss_budget
                } else {
                    args.wall_capped_search_budget
                };
                if combat_gap && combat_budget_capped {
                    if boss_combat {
                        return advance_result(
                            awaiting_auto_boundary(
                                "Combat",
                                format!(
                                    "outer wall budget would cap boss retry; effective search={}ms rescue={}ms boss={}ms",
                                    args.search_ms, args.rescue_search_ms, args.boss_search_ms
                                ),
                            ),
                            None,
                            auto_steps,
                            combat_search,
                        );
                    }
                    return advance_result(
                        BranchStatus::BudgetGap {
                            boundary: "Combat".to_string(),
                            reason: format!(
                                "outer wall budget capped combat search; effective search={}ms rescue={}ms boss={}ms",
                                args.search_ms, args.rescue_search_ms, args.boss_search_ms
                            ),
                        },
                        None,
                        auto_steps,
                        combat_search,
                    );
                }
                if combat_gap && boss_combat {
                    if args.checkpoint_before_boss_retry {
                        return advance_result(
                            awaiting_auto_boundary(
                                "Combat",
                                "checkpoint before boss retry after primary search gap".to_string(),
                            ),
                            None,
                            auto_steps,
                            combat_search,
                        );
                    }
                    if let Some(result) = try_boss_retry(session, deadline.cap_args(args, 1)) {
                        combat_search.extend(result.2);
                        return advance_result(result.0, Some(result.1), auto_steps, combat_search);
                    }
                }
                if combat_gap && !boss_combat {
                    match apply_owner_audit_auto_run(
                        session,
                        diagnostic_rescue_auto_step_options(args),
                    ) {
                        Ok(rescue) => {
                            combat_search.extend(combat_search_summaries(&rescue));
                            auto_steps.extend(rescue.auto_applied_steps.clone());
                            status = classify_auto_outcome(session, &rescue);
                        }
                        Err(err) => {
                            return advance_result(
                                BranchStatus::AdvanceFailed(format!(
                                    "diagnostic combat rescue failed: {err}"
                                )),
                                None,
                                auto_steps,
                                combat_search,
                            );
                        }
                    }
                    if matches!(status, BranchStatus::CombatGap { .. })
                        && should_try_hallway_immediate_rescue(session)
                    {
                        match apply_owner_audit_auto_run(
                            session,
                            hallway_immediate_rescue_auto_step_options(args),
                        ) {
                            Ok(rescue) => {
                                combat_search.extend(combat_search_summaries(&rescue));
                                auto_steps.extend(rescue.auto_applied_steps.clone());
                                status = classify_auto_outcome(session, &rescue);
                            }
                            Err(err) => {
                                return advance_result(
                                    BranchStatus::AdvanceFailed(format!(
                                        "hallway immediate rescue failed: {err}"
                                    )),
                                    None,
                                    auto_steps,
                                    combat_search,
                                );
                            }
                        }
                    }
                    if matches!(status, BranchStatus::CombatGap { .. })
                        && should_try_hallway_potion_rescue(session)
                    {
                        match try_hallway_potion_rescue(session, args) {
                            Ok(rescue) => {
                                combat_search.extend(combat_search_summaries(&rescue.outcome));
                                if rescue.committed {
                                    auto_steps.extend(rescue.outcome.auto_applied_steps.clone());
                                }
                                status = rescue.status;
                            }
                            Err(err) => {
                                return advance_result(
                                    BranchStatus::AdvanceFailed(format!(
                                        "hallway potion rescue failed: {err}"
                                    )),
                                    None,
                                    auto_steps,
                                    combat_search,
                                );
                            }
                        }
                    }
                }
                if let BranchStatus::Terminal(result) = status {
                    return advance_result(
                        BranchStatus::Terminal(result),
                        None,
                        auto_steps,
                        combat_search,
                    );
                }
                let owner = match &status {
                    BranchStatus::Running { owner, .. } => *owner,
                    _ => return advance_result(status, None, auto_steps, combat_search),
                };
                match orchestrate_owner_boundary(session, owner, &mut policy_steps) {
                    OwnerOrchestration::StopAtCandidates => {
                        return advance_result(status, None, auto_steps, combat_search);
                    }
                    OwnerOrchestration::Stop(status) => {
                        return advance_result(status, None, auto_steps, combat_search);
                    }
                    OwnerOrchestration::AppliedRoutine(step) => {
                        auto_steps.push(step);
                    }
                }
            }
            Err(err) => {
                return advance_result(
                    BranchStatus::AdvanceFailed(err),
                    None,
                    auto_steps,
                    combat_search,
                )
            }
        }
    }
}

fn awaiting_auto_boundary(boundary: impl Into<String>, reason: String) -> BranchStatus {
    BranchStatus::AwaitingAuto {
        boundary: boundary.into(),
        reason,
    }
}

fn classify_auto_outcome(
    session: &RunControlSession,
    outcome: &RunControlCommandOutcome,
) -> BranchStatus {
    if let Some(result) = boundary_router::terminal_outcome(session) {
        return BranchStatus::Terminal(result);
    }
    outcome
        .auto_stop
        .as_ref()
        .map(|stop| boundary_router::classify_boundary(session, stop))
        .unwrap_or_else(|| {
            BranchStatus::AdvanceFailed(
                "auto_run returned non-terminal success without auto_stop".to_string(),
            )
        })
}

fn advance_result(
    status: BranchStatus,
    boss_retry: Option<BossRetryReport>,
    auto_steps: Vec<RunControlAutoAppliedStepV1>,
    combat_search: Vec<CombatSearchTraceSummary>,
) -> AdvanceResult {
    AdvanceResult {
        status,
        boss_retry,
        auto_steps,
        combat_search,
    }
}

fn combat_search_summaries(outcome: &RunControlCommandOutcome) -> Vec<CombatSearchTraceSummary> {
    sts_simulator::eval::run_control::combat_search_trace_summaries(&outcome.trace_annotations)
        .collect()
}

fn is_boss_combat(session: &RunControlSession) -> bool {
    session
        .active_combat
        .as_ref()
        .is_some_and(|combat| combat.combat_state.meta.is_boss_fight)
}

fn should_try_hallway_potion_rescue(session: &RunControlSession) -> bool {
    let Some(active) = session.active_combat.as_ref() else {
        return false;
    };
    let meta = &active.combat_state.meta;
    let player = &active.combat_state.entities.player;
    !meta.is_boss_fight
        && !meta.is_elite_fight
        && (session.run_state.act_num >= 3 || player.current_hp * 2 <= player.max_hp)
}

fn should_try_hallway_immediate_rescue(session: &RunControlSession) -> bool {
    session.active_combat.as_ref().is_some_and(|active| {
        !active.combat_state.meta.is_boss_fight && !active.combat_state.meta.is_elite_fight
    })
}

fn try_hallway_potion_rescue(
    session: &mut RunControlSession,
    args: Args,
) -> Result<HallwayPotionRescueAttempt, String> {
    let before_curses = master_deck_curse_count(session);
    let mut trial = session.clone();
    let outcome =
        apply_owner_audit_auto_run(&mut trial, hallway_potion_rescue_auto_step_options(args))?;
    let status = classify_auto_outcome(&trial, &outcome);
    if matches!(status, BranchStatus::CombatGap { .. }) {
        return Ok(HallwayPotionRescueAttempt {
            outcome,
            status,
            committed: false,
        });
    }
    let gained_curses = master_deck_curse_count(&trial).saturating_sub(before_curses);
    if gained_curses > 0 {
        return Ok(HallwayPotionRescueAttempt {
            outcome,
            status: BranchStatus::CombatGap {
                boundary: "Combat".to_string(),
                reason: format!(
                    "hallway potion rescue rejected dirty win: gained {gained_curses} curse card(s)"
                ),
            },
            committed: false,
        });
    }
    *session = trial;
    Ok(HallwayPotionRescueAttempt {
        outcome,
        status,
        committed: true,
    })
}

fn master_deck_curse_count(session: &RunControlSession) -> usize {
    session
        .run_state
        .master_deck
        .iter()
        .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
        .count()
}

fn primary_auto_step_options(args: Args) -> RunControlAutoStepOptions {
    auto_step_options(
        args.search_nodes,
        args.search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        CombatSearchV2ChildRolloutPolicy::LazyOnPop,
    )
}

fn diagnostic_rescue_auto_step_options(args: Args) -> RunControlAutoStepOptions {
    auto_step_options(
        args.rescue_search_nodes,
        args.rescue_search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        CombatSearchV2ChildRolloutPolicy::LazyOnPop,
    )
}

fn hallway_immediate_rescue_auto_step_options(args: Args) -> RunControlAutoStepOptions {
    let mut options = auto_step_options(
        args.rescue_search_nodes,
        args.rescue_search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        CombatSearchV2ChildRolloutPolicy::Immediate,
    );
    options.search.max_potions_used = Some(0);
    options
}

fn hallway_potion_rescue_auto_step_options(args: Args) -> RunControlAutoStepOptions {
    let mut options = auto_step_options(
        args.boss_search_nodes,
        args.boss_search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        CombatSearchV2ChildRolloutPolicy::LazyOnPop,
    );
    options.search.potion_policy = Some(CombatSearchV2PotionPolicy::All);
    options.search.max_potions_used = Some(HALLWAY_POTION_RESCUE_MAX_POTIONS_USED);
    options
}

fn auto_step_options(
    max_nodes: usize,
    wall_ms: u64,
    auto_ops: usize,
    wall_limited: bool,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
) -> RunControlAutoStepOptions {
    RunControlAutoStepOptions {
        search: RunControlSearchCombatOptions {
            max_nodes: Some(max_nodes),
            wall_ms: Some(wall_ms),
            max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
            turn_plan_policy: Some(turn_plan_policy),
            child_rollout_policy: Some(child_rollout_policy),
            ..Default::default()
        },
        max_operations: Some(auto_run_chunk_ops(auto_ops, wall_limited)),
        route: RunControlRouteAutomationMode::Planner,
    }
}

fn auto_run_chunk_ops(auto_ops: usize, wall_limited: bool) -> usize {
    if wall_limited {
        1
    } else {
        auto_ops
    }
}

fn try_boss_retry(
    session: &mut RunControlSession,
    args: Args,
) -> Option<(BranchStatus, BossRetryReport, Vec<CombatSearchTraceSummary>)> {
    let mut all_search = Vec::new();
    let mut attempts = Vec::new();
    let no_potion = boss_retry_options(
        args,
        CombatSearchV2PotionPolicy::Never,
        Some(0),
        CombatSearchV2ChildRolloutPolicy::LazyOnPop,
        CombatSearchV2RolloutPolicy::Disabled,
    );
    let (status, attempt, search) = run_boss_retry_attempt(session, args, "no_potion", no_potion);
    all_search.extend(search);
    attempts.push(attempt);
    if !matches!(status, BranchStatus::CombatGap { .. }) {
        let report = boss_retry_report(args, status.clone(), attempts);
        return Some((status, report, all_search));
    }

    let max_potions = session
        .active_combat
        .as_ref()
        .and_then(|active| {
            sts_simulator::ai::combat_search_v2::high_stakes_semantic_potion_budget(
                &active.combat_state,
            )
        })
        .unwrap_or(1)
        .max(BOSS_RETRY_POTION_RESCUE_MAX_POTIONS_USED);
    let rescue = boss_retry_options(
        args,
        CombatSearchV2PotionPolicy::All,
        Some(max_potions),
        boss_potion_rescue_child_rollout_policy(session),
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
    );
    let (status, attempt, search) = run_boss_retry_attempt(session, args, "potion_rescue", rescue);
    all_search.extend(search);
    attempts.push(attempt);
    let report = boss_retry_report(args, status.clone(), attempts);
    Some((status, report, all_search))
}

fn boss_potion_rescue_child_rollout_policy(
    session: &RunControlSession,
) -> CombatSearchV2ChildRolloutPolicy {
    if session.run_state.act_num >= 3 {
        CombatSearchV2ChildRolloutPolicy::LazyOnPop
    } else {
        CombatSearchV2ChildRolloutPolicy::Immediate
    }
}

fn boss_retry_options(
    args: Args,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    rollout_policy: CombatSearchV2RolloutPolicy,
) -> RunControlAutoStepOptions {
    let mut options = auto_step_options(
        args.boss_search_nodes,
        args.boss_search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        child_rollout_policy,
    );
    options.search.potion_policy = Some(potion_policy);
    options.search.max_potions_used = max_potions_used;
    options.search.rollout_policy = Some(rollout_policy);
    options
}

fn run_boss_retry_attempt(
    session: &mut RunControlSession,
    args: Args,
    label: &'static str,
    options: RunControlAutoStepOptions,
) -> (
    BranchStatus,
    BossRetryAttemptReport,
    Vec<CombatSearchTraceSummary>,
) {
    let potion_policy = options
        .search
        .potion_policy
        .unwrap_or(CombatSearchV2PotionPolicy::Never);
    let max_potions_used = options.search.max_potions_used;
    let outcome = match apply_owner_audit_auto_run(session, options) {
        Ok(outcome) => outcome,
        Err(err) => {
            let status = BranchStatus::AdvanceFailed(err);
            let attempt = boss_retry_attempt_report(
                args,
                label,
                potion_policy,
                max_potions_used,
                &status,
                Vec::new(),
            );
            return (status, attempt, Vec::new());
        }
    };
    let combat_search = combat_search_summaries(&outcome);
    let status = if let Some(outcome) = boundary_router::terminal_outcome(session) {
        BranchStatus::Terminal(outcome)
    } else if let Some(stop) = outcome.auto_stop.as_ref() {
        boundary_router::classify_boundary(session, stop)
    } else {
        BranchStatus::AdvanceFailed(
            "boss retry returned non-terminal success without auto_stop".to_string(),
        )
    };
    let action_keys = retry_complete_search_action_keys(&outcome);
    let attempt = boss_retry_attempt_report(
        args,
        label,
        potion_policy,
        max_potions_used,
        &status,
        action_keys,
    );
    (status, attempt, combat_search)
}

fn boss_retry_report(
    args: Args,
    status: BranchStatus,
    attempts: Vec<BossRetryAttemptReport>,
) -> BossRetryReport {
    let action_keys = attempts
        .last()
        .map(|attempt| attempt.action_keys.clone())
        .unwrap_or_default();
    let status = boss_retry_status(&status);
    BossRetryReport {
        status,
        max_nodes: args.boss_search_nodes,
        wall_ms: args.boss_search_ms,
        action_keys,
        attempts,
    }
}

fn boss_retry_attempt_report(
    args: Args,
    label: &'static str,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
    status: &BranchStatus,
    action_keys: Vec<String>,
) -> BossRetryAttemptReport {
    BossRetryAttemptReport {
        label,
        status: boss_retry_status(status),
        max_nodes: args.boss_search_nodes,
        wall_ms: args.boss_search_ms,
        potion_policy: potion_policy_label(potion_policy),
        max_potions_used,
        action_keys,
    }
}

fn boss_retry_status(status: &BranchStatus) -> BossRetryStatus {
    match status {
        BranchStatus::CombatGap { reason, .. } => BossRetryStatus::Failed(reason.clone()),
        BranchStatus::ApplyFailed(err)
        | BranchStatus::AdvanceFailed(err)
        | BranchStatus::BudgetGap { reason: err, .. } => BossRetryStatus::Failed(err.clone()),
        BranchStatus::Terminal(TerminalOutcome::Defeat) => {
            BossRetryStatus::Failed("retry ended in defeat".to_string())
        }
        BranchStatus::Terminal(result) => BossRetryStatus::Terminal(*result),
        _ => BossRetryStatus::Advanced(render::status_boundary(status).to_string()),
    }
}

fn potion_policy_label(policy: CombatSearchV2PotionPolicy) -> &'static str {
    match policy {
        CombatSearchV2PotionPolicy::Never => "never",
        CombatSearchV2PotionPolicy::All => "all",
        CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic",
    }
}

fn retry_complete_search_action_keys(outcome: &RunControlCommandOutcome) -> Vec<String> {
    outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                source, actions, ..
            } if matches!(
                source,
                CombatAutomationTrajectorySource::SearchCombat
                    | CombatAutomationTrajectorySource::CompleteLineSolver
            ) =>
            {
                Some(
                    actions
                        .iter()
                        .map(|action| action.action_key.clone())
                        .collect::<Vec<_>>(),
                )
            }
            _ => None,
        })
        .unwrap_or_default()
}
