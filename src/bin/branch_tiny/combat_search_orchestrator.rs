use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::content::cards::{get_card_definition, CardType};
use sts_simulator::eval::run_control::{
    apply_owner_audit_auto_run, CombatAutomationTrajectorySource, CombatSearchTraceSummary,
    RunControlAutoAppliedStepV1, RunControlAutoStepOptions, RunControlCommandOutcome,
    RunControlHpLossLimit, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
    RunControlSession, RunControlTraceAnnotationV1,
};

use super::boundary_router;
use super::render;
use super::{
    Args, BossRetryAttemptReport, BossRetryReport, BossRetryStatus, BranchStatus, TerminalOutcome,
};

const BOSS_RETRY_POTION_RESCUE_MAX_POTIONS_USED: u32 = 3;
const NONBOSS_POTION_RESCUE_MAX_POTIONS_USED: u32 = 1;

#[derive(Clone, Copy, Eq, PartialEq)]
enum CombatSearchStakes {
    Hallway,
    Elite,
    Boss,
}

#[derive(Clone, Copy)]
enum CombatSearchLaneKind {
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
    action_keys: Vec<String>,
    committed: bool,
}

pub(super) struct CombatSearchPortfolioResult {
    pub(super) status: BranchStatus,
    pub(super) boss_retry: Option<BossRetryReport>,
    pub(super) auto_steps: Vec<RunControlAutoAppliedStepV1>,
    pub(super) combat_search: Vec<CombatSearchTraceSummary>,
}

pub(super) fn combat_search_summaries(
    outcome: &RunControlCommandOutcome,
) -> Vec<CombatSearchTraceSummary> {
    sts_simulator::eval::run_control::combat_search_trace_summaries(&outcome.trace_annotations)
        .collect()
}

pub(super) fn is_boss_combat(session: &RunControlSession) -> bool {
    session
        .active_combat
        .as_ref()
        .is_some_and(|combat| combat.combat_state.meta.is_boss_fight)
}

pub(super) fn primary_auto_step_options(args: Args) -> RunControlAutoStepOptions {
    auto_step_options(
        args.search_nodes,
        args.search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        CombatSearchV2ChildRolloutPolicy::LazyOnPop,
    )
}

pub(super) fn run_after_primary_gap(
    session: &mut RunControlSession,
    args: Args,
) -> Result<CombatSearchPortfolioResult, String> {
    let request = CombatSearchRequest::from_session(session, args);
    let mut auto_steps = Vec::new();
    let mut combat_search = Vec::new();
    let mut attempts = Vec::new();
    let mut status = BranchStatus::CombatGap {
        boundary: "Combat".to_string(),
        reason: "primary combat search gap".to_string(),
    };

    for lane in request.portfolio(session) {
        let retry = run_lane_attempt(session, &request, lane)
            .map_err(|err| format!("{} failed: {err}", lane.label()))?;
        if let Some(outcome) = retry.outcome.as_ref() {
            combat_search.extend(combat_search_summaries(outcome));
            if retry.committed {
                auto_steps.extend(outcome.auto_applied_steps.clone());
            }
        }
        if request.stakes == CombatSearchStakes::Boss {
            attempts.push(boss_retry_attempt_report(
                request.args,
                lane.label(),
                lane.potion_policy(),
                lane.max_potions_used(session),
                &retry.status,
                retry.action_keys.clone(),
            ));
        }
        status = retry.status;
        if !matches!(status, BranchStatus::CombatGap { .. }) {
            break;
        }
    }

    let boss_retry = if request.stakes == CombatSearchStakes::Boss {
        Some(boss_retry_report(request.args, status.clone(), attempts))
    } else {
        None
    };
    Ok(CombatSearchPortfolioResult {
        status,
        boss_retry,
        auto_steps,
        combat_search,
    })
}

impl CombatSearchRequest {
    fn from_session(session: &RunControlSession, args: Args) -> Self {
        Self {
            args,
            stakes: combat_search_stakes(session),
        }
    }

    fn portfolio(&self, session: &RunControlSession) -> Vec<CombatSearchLane> {
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
}

impl CombatSearchLane {
    fn new(kind: CombatSearchLaneKind) -> Self {
        Self { kind }
    }

    fn label(self) -> &'static str {
        match self.kind {
            CombatSearchLaneKind::DiagnosticRescue => "diagnostic_rescue",
            CombatSearchLaneKind::HallwayImmediateRescue => "hallway_immediate_rescue",
            CombatSearchLaneKind::NonBossPotionRescue => "nonboss_potion_rescue",
            CombatSearchLaneKind::BossNoPotion => "no_potion",
            CombatSearchLaneKind::BossPotionRescue => "potion_rescue",
            CombatSearchLaneKind::QualityRealHp => "quality_real_hp",
        }
    }

    fn potion_policy(self) -> CombatSearchV2PotionPolicy {
        match self.kind {
            CombatSearchLaneKind::BossNoPotion
            | CombatSearchLaneKind::HallwayImmediateRescue
            | CombatSearchLaneKind::DiagnosticRescue => CombatSearchV2PotionPolicy::Never,
            CombatSearchLaneKind::BossPotionRescue | CombatSearchLaneKind::NonBossPotionRescue => {
                CombatSearchV2PotionPolicy::All
            }
            CombatSearchLaneKind::QualityRealHp => CombatSearchV2PotionPolicy::SemanticBudgeted,
        }
    }

    fn max_potions_used(self, session: &RunControlSession) -> Option<u32> {
        match self.kind {
            CombatSearchLaneKind::DiagnosticRescue
            | CombatSearchLaneKind::HallwayImmediateRescue
            | CombatSearchLaneKind::BossNoPotion => Some(0),
            CombatSearchLaneKind::NonBossPotionRescue => {
                Some(NONBOSS_POTION_RESCUE_MAX_POTIONS_USED)
            }
            CombatSearchLaneKind::BossPotionRescue => Some(boss_potion_budget(session)),
            CombatSearchLaneKind::QualityRealHp => Some(2),
        }
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
    let outcome = match apply_owner_audit_auto_run(&mut trial, lane.options(request, session)) {
        Ok(outcome) => outcome,
        Err(err) => {
            return Ok(CombatSearchLaneAttempt {
                outcome: None,
                status: BranchStatus::AdvanceFailed(err),
                action_keys: Vec::new(),
                committed: false,
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
    let action_keys = retry_complete_search_action_keys(&outcome);
    let committed = lane.commits(&status);
    if committed {
        *session = trial;
    }
    Ok(CombatSearchLaneAttempt {
        outcome: Some(outcome),
        status,
        action_keys,
        committed,
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
        .max(BOSS_RETRY_POTION_RESCUE_MAX_POTIONS_USED)
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
