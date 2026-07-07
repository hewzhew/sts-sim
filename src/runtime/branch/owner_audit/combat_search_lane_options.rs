use sts_simulator::ai::combat_search_v2::{
    CombatSearchAcceptancePluginId, CombatSearchArtifactPluginId, CombatSearchBudgetSpec,
    CombatSearchChildRolloutPluginId, CombatSearchFrontierPluginId, CombatSearchPhaseGuardPluginId,
    CombatSearchPluginStack, CombatSearchProfile, CombatSearchRolloutPluginId,
    CombatSearchTurnPlanPluginId, CombatSearchV2PotionPolicy,
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
    CombatSearchRecipe::from_profile(
        lane_profile(lane, request, session),
        request.args.auto_ops,
        request.args.wall_ms.is_some(),
    )
    .into_auto_step_options()
}

fn lane_profile(
    lane: CombatSearchLane,
    request: &CombatSearchRequest,
    session: &RunControlSession,
) -> CombatSearchProfile {
    let profile = match lane.kind() {
        CombatSearchLaneKind::Primary => profile_with_budget(
            lane.label(),
            request.args,
            LaneSearchBudget::Primary,
            CombatSearchChildRolloutPluginId::LazyOnPop,
        ),
        CombatSearchLaneKind::DiagnosticRescue => profile_with_budget(
            lane.label(),
            request.args,
            LaneSearchBudget::Rescue,
            CombatSearchChildRolloutPluginId::LazyOnPop,
        ),
        CombatSearchLaneKind::HallwayImmediateRescue => profile_with_budget(
            lane.label(),
            request.args,
            LaneSearchBudget::Rescue,
            CombatSearchChildRolloutPluginId::Immediate,
        )
        .with_max_potions_used(0),
        CombatSearchLaneKind::NonBossPotionRescue => profile_with_budget(
            lane.label(),
            request.args,
            LaneSearchBudget::Boss,
            CombatSearchChildRolloutPluginId::LazyOnPop,
        )
        .with_potion_policy(CombatSearchV2PotionPolicy::All)
        .with_max_potions_used(NONBOSS_POTION_RESCUE_MAX_POTIONS_USED),
        CombatSearchLaneKind::HallwayQualityPotionRescue => quality_profile(
            lane.label(),
            request.args,
            LaneSearchBudget::HallwayQuality,
            CombatSearchChildRolloutPluginId::Immediate,
            CombatSearchPhaseGuardPluginId::ChampSplitGuard,
        ),
        CombatSearchLaneKind::BossNoPotion => profile_with_budget(
            lane.label(),
            request.args,
            LaneSearchBudget::Boss,
            CombatSearchChildRolloutPluginId::LazyOnPop,
        )
        .with_rollout_plugin(CombatSearchRolloutPluginId::Disabled)
        .with_potion_policy(CombatSearchV2PotionPolicy::Never)
        .with_max_potions_used(0),
        CombatSearchLaneKind::BossPotionRescue => profile_with_budget(
            lane.label(),
            request.args,
            LaneSearchBudget::Boss,
            boss_potion_rescue_child_rollout_plugin(session),
        )
        .with_rollout_plugin(CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion)
        .with_potion_policy(CombatSearchV2PotionPolicy::All)
        .with_max_potions_used(boss_potion_budget(session)),
        CombatSearchLaneKind::BossTimeEaterClock => quality_profile(
            lane.label(),
            request.args,
            LaneSearchBudget::Boss,
            CombatSearchChildRolloutPluginId::LazyOnPop,
            CombatSearchPhaseGuardPluginId::TimeEaterClockHint,
        ),
        CombatSearchLaneKind::QualityRealHp => quality_profile(
            lane.label(),
            request.args,
            LaneSearchBudget::Boss,
            CombatSearchChildRolloutPluginId::Immediate,
            CombatSearchPhaseGuardPluginId::ChampSplitGuard,
        ),
    };
    profile.with_acceptance(lane.acceptance_plugin())
}

fn profile_with_budget(
    label: &'static str,
    args: Args,
    budget: LaneSearchBudget,
    child_rollout_plugin: CombatSearchChildRolloutPluginId,
) -> CombatSearchProfile {
    CombatSearchProfile {
        label,
        budget: CombatSearchBudgetSpec {
            max_nodes: budget.max_nodes(args),
            wall_ms: budget.wall_ms(args),
        },
        plugins: CombatSearchPluginStack {
            turn_plan: CombatSearchTurnPlanPluginId::DiagnosticOnly,
            child_rollout: child_rollout_plugin,
            ..CombatSearchPluginStack::default()
        },
        acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
        artifacts: CombatSearchArtifactPluginId::PortfolioAttempt,
    }
}

fn boss_potion_rescue_child_rollout_plugin(
    session: &RunControlSession,
) -> CombatSearchChildRolloutPluginId {
    if session.run_state.act_num >= 3 {
        CombatSearchChildRolloutPluginId::LazyOnPop
    } else {
        CombatSearchChildRolloutPluginId::Immediate
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

#[cfg(test)]
fn quality_recipe(
    args: Args,
    budget: LaneSearchBudget,
    child_rollout_plugin: CombatSearchChildRolloutPluginId,
    phase_guard_plugin: CombatSearchPhaseGuardPluginId,
) -> CombatSearchRecipe {
    CombatSearchRecipe::from_profile(
        quality_profile(
            "test_quality",
            args,
            budget,
            child_rollout_plugin,
            phase_guard_plugin,
        ),
        args.auto_ops,
        args.wall_ms.is_some(),
    )
}

fn quality_profile(
    label: &'static str,
    args: Args,
    budget: LaneSearchBudget,
    child_rollout_plugin: CombatSearchChildRolloutPluginId,
    phase_guard_plugin: CombatSearchPhaseGuardPluginId,
) -> CombatSearchProfile {
    profile_with_budget(label, args, budget, child_rollout_plugin)
        .with_rollout_plugin(CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion)
        .with_frontier_plugin(CombatSearchFrontierPluginId::RoundRobinEvalBuckets)
        .with_potion_policy(CombatSearchV2PotionPolicy::SemanticBudgeted)
        .with_max_potions_used(2)
        .with_phase_guard_plugin(phase_guard_plugin)
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

#[cfg(test)]
mod tests {
    use super::super::run_contract::RunObjective;
    use super::*;
    use sts_simulator::ai::combat_search_v2::{
        CombatSearchV2FrontierPolicy, CombatSearchV2PhaseGuardPolicy, CombatSearchV2RolloutPolicy,
    };

    fn test_args() -> Args {
        Args {
            seed: 1,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 1,
            max_branches: 1,
            auto_ops: 7,
            search_nodes: 11,
            search_ms: 101,
            rescue_search_nodes: 22,
            rescue_search_ms: 202,
            boss_search_nodes: 999,
            boss_search_ms: 9_999,
            wall_ms: None,
            checkpoint_before_combat_portfolio: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    #[test]
    fn lane_budget_selects_expected_search_budget() {
        let args = test_args();

        assert_eq!(LaneSearchBudget::Primary.max_nodes(args), 11);
        assert_eq!(LaneSearchBudget::Primary.wall_ms(args), 101);
        assert_eq!(LaneSearchBudget::Rescue.max_nodes(args), 22);
        assert_eq!(LaneSearchBudget::Rescue.wall_ms(args), 202);
        assert_eq!(LaneSearchBudget::Boss.max_nodes(args), 999);
        assert_eq!(LaneSearchBudget::Boss.wall_ms(args), 9_999);
        assert_eq!(LaneSearchBudget::HallwayQuality.max_nodes(args), 999);
        assert_eq!(LaneSearchBudget::HallwayQuality.wall_ms(args), 5_000);
    }

    #[test]
    fn quality_recipe_sets_quality_search_modifiers() {
        let options = quality_recipe(
            test_args(),
            LaneSearchBudget::Boss,
            sts_simulator::ai::combat_search_v2::CombatSearchChildRolloutPluginId::Immediate,
            sts_simulator::ai::combat_search_v2::CombatSearchPhaseGuardPluginId::ChampSplitGuard,
        )
        .into_auto_step_options();
        let config = options.search.profile.expect("profile").to_config();

        assert_eq!(
            config.rollout_policy,
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
        );
        assert_eq!(
            config.frontier_policy,
            CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets
        );
        assert_eq!(
            config.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(2));
        assert_eq!(
            config.phase_guard_policy,
            CombatSearchV2PhaseGuardPolicy::ChampSplitGuard
        );
    }

    #[test]
    fn profile_with_budget_records_budget_and_child_rollout_plugin() {
        let profile = profile_with_budget(
            "profile_test",
            test_args(),
            LaneSearchBudget::Rescue,
            sts_simulator::ai::combat_search_v2::CombatSearchChildRolloutPluginId::Immediate,
        );

        assert_eq!(profile.label, "profile_test");
        assert_eq!(profile.budget.max_nodes, 22);
        assert_eq!(profile.budget.wall_ms, 202);
        assert_eq!(
            profile.plugins.child_rollout,
            sts_simulator::ai::combat_search_v2::CombatSearchChildRolloutPluginId::Immediate
        );
        assert_eq!(
            profile.acceptance,
            sts_simulator::ai::combat_search_v2::CombatSearchAcceptancePluginId::AcceptedLineOnly
        );
    }
}
