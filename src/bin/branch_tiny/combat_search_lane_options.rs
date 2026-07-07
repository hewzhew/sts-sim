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

#[derive(Clone, Copy)]
enum LaneSearchBudget {
    Primary,
    Rescue,
    Boss,
    HallwayQuality,
}

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
        CombatSearchLaneKind::Primary => recipe_with_budget(
            request.args,
            LaneSearchBudget::Primary,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
        ),
        CombatSearchLaneKind::DiagnosticRescue => recipe_with_budget(
            request.args,
            LaneSearchBudget::Rescue,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
        ),
        CombatSearchLaneKind::HallwayImmediateRescue => recipe_with_budget(
            request.args,
            LaneSearchBudget::Rescue,
            CombatSearchV2ChildRolloutPolicy::Immediate,
        )
        .with_max_potions_used(0),
        CombatSearchLaneKind::NonBossPotionRescue => recipe_with_budget(
            request.args,
            LaneSearchBudget::Boss,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
        )
        .with_potion_policy(CombatSearchV2PotionPolicy::All)
        .with_max_potions_used(NONBOSS_POTION_RESCUE_MAX_POTIONS_USED),
        CombatSearchLaneKind::HallwayQualityPotionRescue => quality_recipe(
            request.args,
            LaneSearchBudget::HallwayQuality,
            CombatSearchV2ChildRolloutPolicy::Immediate,
            CombatSearchV2PhaseGuardPolicy::ChampSplitGuard,
        ),
        CombatSearchLaneKind::BossNoPotion => recipe_with_budget(
            request.args,
            LaneSearchBudget::Boss,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
        )
        .with_rollout_policy(CombatSearchV2RolloutPolicy::Disabled)
        .with_potion_policy(CombatSearchV2PotionPolicy::Never)
        .with_max_potions_used(0),
        CombatSearchLaneKind::BossPotionRescue => recipe_with_budget(
            request.args,
            LaneSearchBudget::Boss,
            boss_potion_rescue_child_rollout_policy(session),
        )
        .with_rollout_policy(CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion)
        .with_potion_policy(CombatSearchV2PotionPolicy::All)
        .with_max_potions_used(boss_potion_budget(session)),
        CombatSearchLaneKind::BossTimeEaterClock => quality_recipe(
            request.args,
            LaneSearchBudget::Boss,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
            CombatSearchV2PhaseGuardPolicy::TimeEaterClockHint,
        ),
        CombatSearchLaneKind::QualityRealHp => quality_recipe(
            request.args,
            LaneSearchBudget::Boss,
            CombatSearchV2ChildRolloutPolicy::Immediate,
            CombatSearchV2PhaseGuardPolicy::ChampSplitGuard,
        ),
    }
}

fn recipe_with_budget(
    args: Args,
    budget: LaneSearchBudget,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
) -> CombatSearchRecipe {
    CombatSearchRecipe::new(
        budget.max_nodes(args),
        budget.wall_ms(args),
        args.auto_ops,
        args.wall_ms.is_some(),
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
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

fn quality_recipe(
    args: Args,
    budget: LaneSearchBudget,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    phase_guard_policy: CombatSearchV2PhaseGuardPolicy,
) -> CombatSearchRecipe {
    recipe_with_budget(args, budget, child_rollout_policy)
        .with_rollout_policy(CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion)
        .with_frontier_policy(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets)
        .with_potion_policy(CombatSearchV2PotionPolicy::SemanticBudgeted)
        .with_max_potions_used(2)
        .with_phase_guard_policy(phase_guard_policy)
}

impl LaneSearchBudget {
    fn max_nodes(self, args: Args) -> usize {
        match self {
            LaneSearchBudget::Primary => args.search_nodes,
            LaneSearchBudget::Rescue => args.rescue_search_nodes,
            LaneSearchBudget::Boss => args.boss_search_nodes,
            LaneSearchBudget::HallwayQuality => args
                .boss_search_nodes
                .min(HALLWAY_QUALITY_MAX_NODES)
                .max(args.rescue_search_nodes),
        }
    }

    fn wall_ms(self, args: Args) -> u64 {
        match self {
            LaneSearchBudget::Primary => args.search_ms,
            LaneSearchBudget::Rescue => args.rescue_search_ms,
            LaneSearchBudget::Boss => args.boss_search_ms,
            LaneSearchBudget::HallwayQuality => args
                .boss_search_ms
                .min(HALLWAY_QUALITY_MAX_MS)
                .max(args.rescue_search_ms),
        }
    }
}
