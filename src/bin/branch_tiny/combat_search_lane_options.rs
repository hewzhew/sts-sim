use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::run_control::{RunControlAutoStepOptions, RunControlSession};

use super::combat_search_lanes::{CombatSearchLane, CombatSearchLaneKind, CombatSearchRequest};
use super::combat_search_recipe::CombatSearchRecipe;
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
    lane_recipe(lane, request, session).into_auto_step_options()
}

fn lane_recipe(
    lane: CombatSearchLane,
    request: &CombatSearchRequest,
    session: &RunControlSession,
) -> CombatSearchRecipe {
    match lane.kind() {
        CombatSearchLaneKind::Primary => base_recipe(
            request.args.search_nodes,
            request.args.search_ms,
            request.args.auto_ops,
            request.args.wall_ms.is_some(),
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
        ),
        CombatSearchLaneKind::DiagnosticRescue => base_recipe(
            request.args.rescue_search_nodes,
            request.args.rescue_search_ms,
            request.args.auto_ops,
            request.args.wall_ms.is_some(),
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
        ),
        CombatSearchLaneKind::HallwayImmediateRescue => base_recipe(
            request.args.rescue_search_nodes,
            request.args.rescue_search_ms,
            request.args.auto_ops,
            request.args.wall_ms.is_some(),
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2ChildRolloutPolicy::Immediate,
        )
        .with_max_potions_used(0),
        CombatSearchLaneKind::NonBossPotionRescue => base_recipe(
            request.args.boss_search_nodes,
            request.args.boss_search_ms,
            request.args.auto_ops,
            request.args.wall_ms.is_some(),
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
        )
        .with_potion_policy(CombatSearchV2PotionPolicy::All)
        .with_max_potions_used(NONBOSS_POTION_RESCUE_MAX_POTIONS_USED),
        CombatSearchLaneKind::HallwayQualityPotionRescue => base_recipe(
            hallway_quality_nodes(request.args),
            hallway_quality_ms(request.args),
            request.args.auto_ops,
            request.args.wall_ms.is_some(),
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2ChildRolloutPolicy::Immediate,
        )
        .with_rollout_policy(CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion)
        .with_frontier_policy(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets)
        .with_potion_policy(CombatSearchV2PotionPolicy::SemanticBudgeted)
        .with_max_potions_used(2)
        .with_phase_guard_policy(CombatSearchV2PhaseGuardPolicy::ChampSplitGuard),
        CombatSearchLaneKind::BossNoPotion => boss_budget_recipe(
            request.args,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
            CombatSearchV2RolloutPolicy::Disabled,
        )
        .with_potion_policy(CombatSearchV2PotionPolicy::Never)
        .with_max_potions_used(0),
        CombatSearchLaneKind::BossPotionRescue => boss_budget_recipe(
            request.args,
            boss_potion_rescue_child_rollout_policy(session),
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
        )
        .with_potion_policy(CombatSearchV2PotionPolicy::All)
        .with_max_potions_used(boss_potion_budget(session)),
        CombatSearchLaneKind::BossTimeEaterClock => boss_budget_recipe(
            request.args,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
        )
        .with_frontier_policy(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets)
        .with_potion_policy(CombatSearchV2PotionPolicy::SemanticBudgeted)
        .with_max_potions_used(2)
        .with_phase_guard_policy(CombatSearchV2PhaseGuardPolicy::TimeEaterClockHint),
        CombatSearchLaneKind::QualityRealHp => boss_budget_recipe(
            request.args,
            CombatSearchV2ChildRolloutPolicy::Immediate,
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
        )
        .with_frontier_policy(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets)
        .with_potion_policy(CombatSearchV2PotionPolicy::SemanticBudgeted)
        .with_max_potions_used(2)
        .with_phase_guard_policy(CombatSearchV2PhaseGuardPolicy::ChampSplitGuard),
    }
}

fn base_recipe(
    max_nodes: usize,
    wall_ms: u64,
    auto_ops: usize,
    wall_limited: bool,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
) -> CombatSearchRecipe {
    CombatSearchRecipe::new(
        max_nodes,
        wall_ms,
        auto_ops,
        wall_limited,
        turn_plan_policy,
        child_rollout_policy,
    )
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

fn boss_budget_recipe(
    args: Args,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    rollout_policy: CombatSearchV2RolloutPolicy,
) -> CombatSearchRecipe {
    base_recipe(
        args.boss_search_nodes,
        args.boss_search_ms,
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        child_rollout_policy,
    )
    .with_rollout_policy(rollout_policy)
}
