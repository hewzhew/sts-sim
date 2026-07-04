use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::content::cards::{get_card_definition, CardType};
use sts_simulator::eval::run_control::{
    apply_owner_audit_auto_run, CombatAutomationTrajectorySource, CombatSearchTraceSummary,
    RunControlAutoAppliedStepV1, RunControlAutoStepOptions, RunControlAutoStopKind,
    RunControlCommandOutcome, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession, RunControlTraceAnnotationV1,
};

use super::boundary_router;
use super::render;
use super::{
    Args, BranchStatus, CombatSearchLaneReport, CombatSearchPortfolioReport,
    CombatSearchPortfolioStatus, TerminalOutcome,
};

const BOSS_POTION_RESCUE_MAX_POTIONS_USED: u32 = 3;
const NONBOSS_POTION_RESCUE_MAX_POTIONS_USED: u32 = 1;

#[derive(Clone, Copy, Eq, PartialEq)]
enum CombatSearchStakes {
    Hallway,
    Elite,
    Boss,
}

#[derive(Clone, Copy)]
enum CombatSearchLaneKind {
    Primary,
    DiagnosticRescue,
    HallwayImmediateRescue,
    NonBossPotionRescue,
    BossNoPotion,
    BossPotionRescue,
    QualityRealHp,
}

#[derive(Clone, Copy)]
struct CombatSearchLane {
    kind: CombatSearchLaneKind,
}

struct CombatSearchRequest {
    args: Args,
    stakes: CombatSearchStakes,
}

struct CombatSearchLaneAttempt {
    outcome: Option<RunControlCommandOutcome>,
    status: BranchStatus,
    label: &'static str,
    max_nodes: usize,
    wall_ms: u64,
    potion_policy: &'static str,
    max_potions_used: Option<u32>,
    action_keys: Vec<String>,
    committed: bool,
    auto_stop_kind: Option<RunControlAutoStopKind>,
    applied_operations: usize,
}

#[derive(Clone)]
pub(super) enum CombatSearchOutcome {
    AcceptedLine,
    CombatGap,
    BudgetLimited,
    OperationBudgetExhausted,
    Terminal,
    Failed,
}

pub(super) struct CombatSearchPortfolioResult {
    pub(super) status: BranchStatus,
    pub(super) outcome: CombatSearchOutcome,
    pub(super) report: Option<CombatSearchPortfolioReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
    pub(super) applied_operations: usize,
}

pub(super) fn combat_search_summaries(
    outcome: &RunControlCommandOutcome,
) -> Vec<CombatSearchTraceSummary> {
    sts_simulator::eval::run_control::combat_search_trace_summaries(&outcome.trace_annotations)
        .collect()
}

pub(super) fn run_combat_portfolio_step(
    session: &mut RunControlSession,
    args: Args,
) -> Result<CombatSearchPortfolioResult, String> {
    let request = CombatSearchRequest::from_session(session, args);
    let mut auto_steps = Vec::new();
    let mut combat_search = Vec::new();
    let mut attempts = Vec::new();
    let mut applied_operations = 0usize;

    let primary = run_lane_attempt(
        session,
        &request,
        CombatSearchLane::new(CombatSearchLaneKind::Primary),
    )
    .map_err(|err| format!("primary failed: {err}"))?;
    if let Some(outcome) = primary.outcome.as_ref() {
        applied_operations = applied_operations.saturating_add(primary.applied_operations);
        combat_search.extend(combat_search_summaries(outcome));
        if primary.committed {
            auto_steps.extend(outcome.auto_applied_steps.clone());
        }
    }
    let mut status = primary.status.clone();
    let primary_stop_kind = primary.auto_stop_kind;
    if primary_operation_budget_exhausted(&status, primary_stop_kind)
        && !matches!(status, BranchStatus::Terminal(_))
    {
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            None,
            auto_steps,
            combat_search,
            applied_operations,
        ));
    }
    let saw_primary_combat_gap = matches!(status, BranchStatus::CombatGap { .. });
    if request.should_report() && saw_primary_combat_gap {
        attempts.push(combat_portfolio_attempt_report(&primary));
    }
    if !saw_primary_combat_gap {
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            None,
            auto_steps,
            combat_search,
            applied_operations,
        ));
    }

    if request.combat_budget_capped() {
        status = if request.stakes == CombatSearchStakes::Boss {
            awaiting_auto_boundary(
                "Combat",
                format!(
                    "outer wall budget would cap combat portfolio; effective search={}ms rescue={}ms boss={}ms",
                    args.search_ms, args.rescue_search_ms, args.boss_search_ms
                ),
            )
        } else {
            BranchStatus::BudgetGap {
                boundary: "Combat".to_string(),
                reason: format!(
                    "outer wall budget capped combat search; effective search={}ms rescue={}ms boss={}ms",
                    args.search_ms, args.rescue_search_ms, args.boss_search_ms
                ),
            }
        };
        let report = request.report(status.clone(), attempts);
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            report,
            auto_steps,
            combat_search,
            applied_operations,
        ));
    }
    if request.stakes == CombatSearchStakes::Boss && args.checkpoint_before_combat_portfolio {
        status = awaiting_auto_boundary(
            "Combat",
            "checkpoint before combat portfolio after primary search gap".to_string(),
        );
        let report = request.report(status.clone(), attempts);
        return Ok(combat_search_result(
            status,
            primary_stop_kind,
            report,
            auto_steps,
            combat_search,
            applied_operations,
        ));
    }

    for lane in request.portfolio_after_primary(session) {
        let retry = run_lane_attempt(session, &request, lane)
            .map_err(|err| format!("{} failed: {err}", lane.label()))?;
        if let Some(outcome) = retry.outcome.as_ref() {
            applied_operations = applied_operations.saturating_add(retry.applied_operations);
            combat_search.extend(combat_search_summaries(outcome));
            if retry.committed {
                auto_steps.extend(outcome.auto_applied_steps.clone());
            }
        }
        if request.should_report() {
            attempts.push(combat_portfolio_attempt_report(&retry));
        }
        status = retry.status;
        if !matches!(status, BranchStatus::CombatGap { .. }) {
            break;
        }
    }

    let report = request.report(status.clone(), attempts);
    Ok(combat_search_result(
        status,
        primary_stop_kind,
        report,
        auto_steps,
        combat_search,
        applied_operations,
    ))
}

impl CombatSearchRequest {
    fn from_session(session: &RunControlSession, args: Args) -> Self {
        Self {
            args,
            stakes: combat_search_stakes(session),
        }
    }

    fn portfolio_after_primary(&self, session: &RunControlSession) -> Vec<CombatSearchLane> {
        let mut lanes = Vec::new();
        match self.stakes {
            CombatSearchStakes::Boss => {
                lanes.push(CombatSearchLane::new(CombatSearchLaneKind::BossNoPotion));
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::BossPotionRescue,
                ));
                lanes.push(CombatSearchLane::new(CombatSearchLaneKind::QualityRealHp));
            }
            CombatSearchStakes::Elite => {
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::DiagnosticRescue,
                ));
                if should_try_nonboss_potion_rescue(session) {
                    lanes.push(CombatSearchLane::new(
                        CombatSearchLaneKind::NonBossPotionRescue,
                    ));
                }
                lanes.push(CombatSearchLane::new(CombatSearchLaneKind::QualityRealHp));
            }
            CombatSearchStakes::Hallway => {
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::DiagnosticRescue,
                ));
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::HallwayImmediateRescue,
                ));
                if should_try_nonboss_potion_rescue(session) {
                    lanes.push(CombatSearchLane::new(
                        CombatSearchLaneKind::NonBossPotionRescue,
                    ));
                }
            }
        }
        lanes
    }

    fn should_report(&self) -> bool {
        self.stakes == CombatSearchStakes::Boss
    }

    fn combat_budget_capped(&self) -> bool {
        match self.stakes {
            CombatSearchStakes::Boss => self.args.wall_capped_boss_budget,
            CombatSearchStakes::Elite | CombatSearchStakes::Hallway => {
                self.args.wall_capped_search_budget
            }
        }
    }

    fn report(
        &self,
        status: BranchStatus,
        attempts: Vec<CombatSearchLaneReport>,
    ) -> Option<CombatSearchPortfolioReport> {
        self.should_report()
            .then(|| combat_portfolio_report(self.args, status, attempts))
    }
}

impl CombatSearchLane {
    fn new(kind: CombatSearchLaneKind) -> Self {
        Self { kind }
    }

    fn label(self) -> &'static str {
        match self.kind {
            CombatSearchLaneKind::Primary => "primary",
            CombatSearchLaneKind::DiagnosticRescue => "diagnostic_rescue",
            CombatSearchLaneKind::HallwayImmediateRescue => "hallway_immediate_rescue",
            CombatSearchLaneKind::NonBossPotionRescue => "nonboss_potion_rescue",
            CombatSearchLaneKind::BossNoPotion => "no_potion",
            CombatSearchLaneKind::BossPotionRescue => "potion_rescue",
            CombatSearchLaneKind::QualityRealHp => "quality_real_hp",
        }
    }

    fn is_primary(self) -> bool {
        matches!(self.kind, CombatSearchLaneKind::Primary)
    }
}

fn should_try_nonboss_potion_rescue(session: &RunControlSession) -> bool {
    let Some(active) = session.active_combat.as_ref() else {
        return false;
    };
    let meta = &active.combat_state.meta;
    let player = &active.combat_state.entities.player;
    let has_usable_potion = active
        .combat_state
        .entities
        .potions
        .iter()
        .flatten()
        .any(|potion| potion.can_use);
    !meta.is_boss_fight
        && has_usable_potion
        && (meta.is_elite_fight
            || session.run_state.act_num >= 3
            || player.current_hp * 2 <= player.max_hp)
}

fn master_deck_curse_count(session: &RunControlSession) -> usize {
    session
        .run_state
        .master_deck
        .iter()
        .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
        .count()
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

fn combat_search_stakes(session: &RunControlSession) -> CombatSearchStakes {
    session
        .active_combat
        .as_ref()
        .map(|active| {
            if active.combat_state.meta.is_boss_fight {
                CombatSearchStakes::Boss
            } else if active.combat_state.meta.is_elite_fight {
                CombatSearchStakes::Elite
            } else {
                CombatSearchStakes::Hallway
            }
        })
        .unwrap_or(CombatSearchStakes::Hallway)
}

fn run_lane_attempt(
    session: &mut RunControlSession,
    request: &CombatSearchRequest,
    lane: CombatSearchLane,
) -> Result<CombatSearchLaneAttempt, String> {
    let before_curses = master_deck_curse_count(session);
    let mut trial = session.clone();
    let options = lane.options(request, session);
    let max_nodes = options.search.max_nodes.unwrap_or_default();
    let wall_ms = options.search.wall_ms.unwrap_or_default();
    let potion_policy = potion_policy_label(options.search.potion_policy);
    let max_potions_used = options.search.max_potions_used;
    let outcome = match apply_owner_audit_auto_run(&mut trial, options) {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(CombatSearchLaneAttempt {
                outcome: None,
                status: BranchStatus::AdvanceFailed(err),
                label: lane.label(),
                max_nodes,
                wall_ms,
                potion_policy,
                max_potions_used,
                action_keys: Vec::new(),
                committed: false,
                auto_stop_kind: None,
                applied_operations: 0,
            });
        }
    };
    let mut status = lane_status(&trial, &outcome);
    if lane.rejects_new_curses()
        && !matches!(status, BranchStatus::CombatGap { .. })
        && master_deck_curse_count(&trial) > before_curses
    {
        let gained_curses = master_deck_curse_count(&trial).saturating_sub(before_curses);
        status = BranchStatus::CombatGap {
            boundary: "Combat".to_string(),
            reason: format!(
                "{} rejected dirty win: gained {gained_curses} curse card(s)",
                lane.label()
            ),
        };
    }
    let auto_stop_kind = outcome.auto_stop.as_ref().map(|stop| stop.kind);
    let applied_operations = outcome
        .auto_stop
        .as_ref()
        .map(|stop| stop.applied_operations)
        .unwrap_or(0);
    let action_keys = retry_complete_search_action_keys(&outcome);
    let committed = lane.commits(&status)
        || (lane.is_primary() && primary_operation_budget_exhausted(&status, auto_stop_kind));
    if committed {
        *session = trial;
    }
    Ok(CombatSearchLaneAttempt {
        outcome: Some(outcome),
        status,
        label: lane.label(),
        max_nodes,
        wall_ms,
        potion_policy,
        max_potions_used,
        action_keys,
        committed,
        auto_stop_kind,
        applied_operations,
    })
}

fn lane_status(session: &RunControlSession, outcome: &RunControlCommandOutcome) -> BranchStatus {
    if let Some(outcome) = boundary_router::terminal_outcome(session) {
        BranchStatus::Terminal(outcome)
    } else {
        boundary_router::classify_auto_outcome(session, outcome)
    }
}

impl CombatSearchLane {
    fn options(
        self,
        request: &CombatSearchRequest,
        session: &RunControlSession,
    ) -> RunControlAutoStepOptions {
        match self.kind {
            CombatSearchLaneKind::Primary => auto_step_options(
                request.args.search_nodes,
                request.args.search_ms,
                request.args.auto_ops,
                request.args.wall_ms.is_some(),
                CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
                CombatSearchV2ChildRolloutPolicy::LazyOnPop,
            ),
            CombatSearchLaneKind::DiagnosticRescue => auto_step_options(
                request.args.rescue_search_nodes,
                request.args.rescue_search_ms,
                request.args.auto_ops,
                request.args.wall_ms.is_some(),
                CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
                CombatSearchV2ChildRolloutPolicy::LazyOnPop,
            ),
            CombatSearchLaneKind::HallwayImmediateRescue => {
                let mut options = auto_step_options(
                    request.args.rescue_search_nodes,
                    request.args.rescue_search_ms,
                    request.args.auto_ops,
                    request.args.wall_ms.is_some(),
                    CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
                    CombatSearchV2ChildRolloutPolicy::Immediate,
                );
                options.search.max_potions_used = Some(0);
                options
            }
            CombatSearchLaneKind::NonBossPotionRescue => {
                let mut options = auto_step_options(
                    request.args.boss_search_nodes,
                    request.args.boss_search_ms,
                    request.args.auto_ops,
                    request.args.wall_ms.is_some(),
                    CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
                    CombatSearchV2ChildRolloutPolicy::LazyOnPop,
                );
                options.search.potion_policy = Some(CombatSearchV2PotionPolicy::All);
                options.search.max_potions_used = Some(NONBOSS_POTION_RESCUE_MAX_POTIONS_USED);
                options
            }
            CombatSearchLaneKind::BossNoPotion => {
                let mut options = boss_budget_options(
                    request.args,
                    CombatSearchV2ChildRolloutPolicy::LazyOnPop,
                    CombatSearchV2RolloutPolicy::Disabled,
                );
                options.search.potion_policy = Some(CombatSearchV2PotionPolicy::Never);
                options.search.max_potions_used = Some(0);
                options
            }
            CombatSearchLaneKind::BossPotionRescue => {
                let mut options = boss_budget_options(
                    request.args,
                    boss_potion_rescue_child_rollout_policy(session),
                    CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
                );
                options.search.potion_policy = Some(CombatSearchV2PotionPolicy::All);
                options.search.max_potions_used = Some(boss_potion_budget(session));
                options
            }
            CombatSearchLaneKind::QualityRealHp => {
                let mut options = boss_budget_options(
                    request.args,
                    CombatSearchV2ChildRolloutPolicy::Immediate,
                    CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
                );
                options.search.frontier_policy =
                    Some(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets);
                options.search.potion_policy = Some(CombatSearchV2PotionPolicy::SemanticBudgeted);
                options.search.max_potions_used = Some(2);
                options.search.phase_guard_policy =
                    Some(CombatSearchV2PhaseGuardPolicy::ChampSplitGuard);
                options
            }
        }
    }

    fn rejects_new_curses(self) -> bool {
        matches!(self.kind, CombatSearchLaneKind::NonBossPotionRescue)
    }

    fn commits(self, status: &BranchStatus) -> bool {
        !matches!(
            status,
            BranchStatus::CombatGap { .. }
                | BranchStatus::BudgetGap { .. }
                | BranchStatus::ApplyFailed(_)
                | BranchStatus::AdvanceFailed(_)
                | BranchStatus::Terminal(TerminalOutcome::Defeat)
        )
    }
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

fn boss_potion_budget(session: &RunControlSession) -> u32 {
    session
        .active_combat
        .as_ref()
        .and_then(|active| {
            sts_simulator::ai::combat_search_v2::high_stakes_semantic_potion_budget(
                &active.combat_state,
            )
        })
        .unwrap_or(1)
        .max(BOSS_POTION_RESCUE_MAX_POTIONS_USED)
}

fn boss_budget_options(
    args: Args,
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
    options.search.rollout_policy = Some(rollout_policy);
    options
}

fn combat_portfolio_report(
    args: Args,
    status: BranchStatus,
    attempts: Vec<CombatSearchLaneReport>,
) -> CombatSearchPortfolioReport {
    let action_keys = attempts
        .last()
        .map(|attempt| attempt.action_keys.clone())
        .unwrap_or_default();
    let status = combat_portfolio_status(&status);
    CombatSearchPortfolioReport {
        status,
        max_nodes: args.boss_search_nodes,
        wall_ms: args.boss_search_ms,
        action_keys,
        attempts,
    }
}

fn combat_portfolio_attempt_report(attempt: &CombatSearchLaneAttempt) -> CombatSearchLaneReport {
    CombatSearchLaneReport {
        label: attempt.label,
        status: combat_portfolio_status(&attempt.status),
        max_nodes: attempt.max_nodes,
        wall_ms: attempt.wall_ms,
        potion_policy: attempt.potion_policy,
        max_potions_used: attempt.max_potions_used,
        action_keys: attempt.action_keys.clone(),
    }
}

fn combat_portfolio_status(status: &BranchStatus) -> CombatSearchPortfolioStatus {
    match status {
        BranchStatus::CombatGap { reason, .. } => {
            CombatSearchPortfolioStatus::Failed(reason.clone())
        }
        BranchStatus::ApplyFailed(err)
        | BranchStatus::AdvanceFailed(err)
        | BranchStatus::BudgetGap { reason: err, .. } => {
            CombatSearchPortfolioStatus::Failed(err.clone())
        }
        BranchStatus::Terminal(TerminalOutcome::Defeat) => {
            CombatSearchPortfolioStatus::Failed("combat portfolio ended in defeat".to_string())
        }
        BranchStatus::Terminal(result) => CombatSearchPortfolioStatus::Terminal(*result),
        _ => CombatSearchPortfolioStatus::Advanced(render::status_boundary(status).to_string()),
    }
}

fn potion_policy_label(policy: Option<CombatSearchV2PotionPolicy>) -> &'static str {
    match policy {
        Some(CombatSearchV2PotionPolicy::Never) => "never",
        Some(CombatSearchV2PotionPolicy::All) => "all",
        Some(CombatSearchV2PotionPolicy::SemanticBudgeted) => "semantic",
        None => "default",
    }
}

fn combat_search_result(
    status: BranchStatus,
    primary_stop_kind: Option<RunControlAutoStopKind>,
    report: Option<CombatSearchPortfolioReport>,
    auto_steps: Vec<RunControlAutoAppliedStepV1>,
    combat_search: Vec<CombatSearchTraceSummary>,
    applied_operations: usize,
) -> CombatSearchPortfolioResult {
    let outcome = combat_search_outcome(&status, primary_stop_kind);
    CombatSearchPortfolioResult {
        status,
        outcome,
        report,
        auto_steps,
        combat_search,
        applied_operations,
    }
}

fn combat_search_outcome(
    status: &BranchStatus,
    primary_stop_kind: Option<RunControlAutoStopKind>,
) -> CombatSearchOutcome {
    match status {
        BranchStatus::Terminal(_) => CombatSearchOutcome::Terminal,
        _ if primary_operation_budget_exhausted(status, primary_stop_kind) => {
            CombatSearchOutcome::OperationBudgetExhausted
        }
        BranchStatus::CombatGap { .. } => CombatSearchOutcome::CombatGap,
        BranchStatus::BudgetGap { .. } => CombatSearchOutcome::BudgetLimited,
        BranchStatus::ApplyFailed(_) | BranchStatus::AdvanceFailed(_) => {
            CombatSearchOutcome::Failed
        }
        _ => CombatSearchOutcome::AcceptedLine,
    }
}

fn primary_operation_budget_exhausted(
    status: &BranchStatus,
    primary_stop_kind: Option<RunControlAutoStopKind>,
) -> bool {
    primary_stop_kind == Some(RunControlAutoStopKind::OperationBudgetExhausted)
        || matches!(
            status,
            BranchStatus::BudgetGap { reason, .. }
                if reason.starts_with("operation budget exhausted")
        )
}

fn awaiting_auto_boundary(boundary: impl Into<String>, reason: String) -> BranchStatus {
    BranchStatus::AwaitingAuto {
        boundary: boundary.into(),
        reason,
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
                    | CombatAutomationTrajectorySource::LineLabTurnPoolRescue
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
