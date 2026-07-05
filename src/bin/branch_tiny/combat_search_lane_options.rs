use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::run_control::{
    RunControlAutoStepOptions, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession,
};

use super::combat_search_lanes::{CombatSearchLane, CombatSearchLaneKind, CombatSearchRequest};
use super::Args;

const BOSS_POTION_RESCUE_MAX_POTIONS_USED: u32 = 3;
const NONBOSS_POTION_RESCUE_MAX_POTIONS_USED: u32 = 1;
const HALLWAY_QUALITY_MAX_NODES: usize = 300_000;
const HALLWAY_QUALITY_MAX_MS: u64 = 5_000;

pub(super) fn lane_options(
    lane: CombatSearchLane,
    request: &CombatSearchRequest,
    session: &RunControlSession,
) -> RunControlAutoStepOptions {
    match lane.kind() {
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
        CombatSearchLaneKind::HallwayQualityPotionRescue => {
            let mut options = auto_step_options(
                hallway_quality_nodes(request.args),
                hallway_quality_ms(request.args),
                request.args.auto_ops,
                request.args.wall_ms.is_some(),
                CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
                CombatSearchV2ChildRolloutPolicy::Immediate,
            );
            options.search.rollout_policy =
                Some(CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion);
            options.search.frontier_policy =
                Some(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets);
            options.search.potion_policy = Some(CombatSearchV2PotionPolicy::SemanticBudgeted);
            options.search.max_potions_used = Some(2);
            options.search.phase_guard_policy =
                Some(CombatSearchV2PhaseGuardPolicy::ChampSplitGuard);
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

fn hallway_quality_nodes(args: Args) -> usize {
    args.boss_search_nodes
        .min(HALLWAY_QUALITY_MAX_NODES)
        .max(args.rescue_search_nodes)
}

fn hallway_quality_ms(args: Args) -> u64 {
    args.boss_search_ms
        .min(HALLWAY_QUALITY_MAX_MS)
        .max(args.rescue_search_ms)
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
