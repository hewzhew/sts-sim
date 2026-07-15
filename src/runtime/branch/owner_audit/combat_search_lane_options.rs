use sts_simulator::ai::combat_search_v2::{
    CombatSearchAcceptancePluginId, CombatSearchArtifactPluginId, CombatSearchAttemptPolicy,
    CombatSearchBudgetSpec, CombatSearchChildRolloutPluginId, CombatSearchEngineProfile,
    CombatSearchFrontierPluginId, CombatSearchPhaseGuardPluginId, CombatSearchPluginStack,
    CombatSearchProfile, CombatSearchRolloutPluginId, CombatSearchV2PotionPolicy,
};
use sts_simulator::eval::run_control::{
    RunControlAutoStepOptions, RunControlHpLossLimit, RunControlSession,
};

use super::combat_search_lanes::{
    CombatSearchLane, CombatSearchLaneKind, CombatSearchRequest, CombatSearchStakes,
};
use super::combat_search_recipe::CombatSearchRecipe;
use super::combat_search_survival::owner_audit_hp_loss_limit;
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
    let mut options = CombatSearchRecipe::from_profile(lane_profile(lane, request, session))
        .into_auto_step_options();
    options.search.potion_policy = options
        .search
        .profile
        .map(|profile| profile.to_config().potion_policy);
    options.search.max_hp_loss = Some(match lane.kind() {
        CombatSearchLaneKind::Primary => owner_audit_hp_loss_limit(session),
        _ => RunControlHpLossLimit::Unlimited,
    });
    options.search.disable_no_win_rescue = !lane_allows_internal_no_win_rescue(lane);
    options.search.allow_smoke_bomb_survival_fallback =
        lane_allows_smoke_bomb_survival_fallback(lane);
    options
}

fn lane_allows_internal_no_win_rescue(lane: CombatSearchLane) -> bool {
    matches!(
        lane.kind(),
        CombatSearchLaneKind::DiagnosticRescue
            | CombatSearchLaneKind::EliteSurvivalFallback
            | CombatSearchLaneKind::HallwaySurvivalFallback
    )
}

fn lane_allows_smoke_bomb_survival_fallback(lane: CombatSearchLane) -> bool {
    matches!(
        lane.kind(),
        CombatSearchLaneKind::EliteSurvivalFallback | CombatSearchLaneKind::HallwaySurvivalFallback
    )
}

fn lane_profile(
    lane: CombatSearchLane,
    request: &CombatSearchRequest,
    session: &RunControlSession,
) -> CombatSearchProfile {
    let profile = match lane.kind() {
        CombatSearchLaneKind::Primary => primary_profile(lane.label(), request),
        CombatSearchLaneKind::DiagnosticRescue => profile_with_budget(
            lane.label(),
            request.args,
            LaneSearchBudget::Rescue,
            CombatSearchChildRolloutPluginId::LazyOnPop,
        ),
        CombatSearchLaneKind::PrimaryImmediateEscalation => profile_with_budget(
            lane.label(),
            request.args,
            LaneSearchBudget::Rescue,
            CombatSearchChildRolloutPluginId::Immediate,
        )
        .with_max_potions_used(0),
        CombatSearchLaneKind::EliteSurvivalFallback => quality_profile(
            lane.label(),
            request.args,
            LaneSearchBudget::HallwayQuality,
            CombatSearchChildRolloutPluginId::Immediate,
            CombatSearchPhaseGuardPluginId::ChampSplitGuard,
        )
        .with_max_potions_used(NONBOSS_POTION_RESCUE_MAX_POTIONS_USED),
        CombatSearchLaneKind::HallwayQualityPotionRescue => quality_profile(
            lane.label(),
            request.args,
            LaneSearchBudget::HallwayQuality,
            CombatSearchChildRolloutPluginId::Immediate,
            CombatSearchPhaseGuardPluginId::ChampSplitGuard,
        ),
        CombatSearchLaneKind::HallwaySurvivalFallback => quality_profile(
            lane.label(),
            request.args,
            LaneSearchBudget::HallwayQuality,
            CombatSearchChildRolloutPluginId::LazyOnPop,
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
            CombatSearchChildRolloutPluginId::LazyOnPop,
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

fn primary_profile(label: &'static str, request: &CombatSearchRequest) -> CombatSearchProfile {
    match request.stakes() {
        CombatSearchStakes::Boss => high_stakes_primary_profile(
            label,
            request.args,
            LaneSearchBudget::Boss,
            CombatSearchChildRolloutPluginId::Immediate,
            CombatSearchV2PotionPolicy::All,
            BOSS_POTION_RESCUE_MAX_POTIONS_USED,
        ),
        CombatSearchStakes::Elite => high_stakes_primary_profile(
            label,
            request.args,
            LaneSearchBudget::Rescue,
            CombatSearchChildRolloutPluginId::Immediate,
            CombatSearchV2PotionPolicy::SemanticBudgeted,
            NONBOSS_POTION_RESCUE_MAX_POTIONS_USED,
        ),
        CombatSearchStakes::Hallway => profile_with_budget(
            label,
            request.args,
            LaneSearchBudget::Primary,
            CombatSearchChildRolloutPluginId::Immediate,
        )
        .with_potion_policy(CombatSearchV2PotionPolicy::SemanticBudgeted)
        .with_max_potions_used(NONBOSS_POTION_RESCUE_MAX_POTIONS_USED),
    }
}

fn high_stakes_primary_profile(
    label: &'static str,
    args: Args,
    budget: LaneSearchBudget,
    child_rollout_plugin: CombatSearchChildRolloutPluginId,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: u32,
) -> CombatSearchProfile {
    profile_with_budget(label, args, budget, child_rollout_plugin)
        .with_potion_policy(potion_policy)
        .with_max_potions_used(max_potions_used)
}

fn profile_with_budget(
    label: &'static str,
    args: Args,
    budget: LaneSearchBudget,
    child_rollout_plugin: CombatSearchChildRolloutPluginId,
) -> CombatSearchProfile {
    CombatSearchProfile {
        label,
        engine: CombatSearchEngineProfile {
            budget: CombatSearchBudgetSpec {
                max_nodes: budget.max_nodes(args),
                wall_ms: budget.wall_ms(args),
            },
            plugins: CombatSearchPluginStack {
                child_rollout: child_rollout_plugin,
                ..CombatSearchPluginStack::default()
            },
        },
        policy: CombatSearchAttemptPolicy {
            acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            artifacts: CombatSearchArtifactPluginId::PortfolioAttempt,
        },
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
    CombatSearchRecipe::from_profile(quality_profile(
        "test_quality",
        args,
        budget,
        child_rollout_plugin,
        phase_guard_plugin,
    ))
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
    use sts_simulator::eval::run_control::{
        RunControlConfig, RunControlHpLossLimit, RunControlSession,
    };
    use sts_simulator::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use sts_simulator::state::map::node::RoomType;

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
            shop_boss_preview_bundle_limit: 0,
            shop_boss_preview_target_floor: None,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    fn session_with_combat_stakes(is_boss_fight: bool, is_elite_fight: bool) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = is_boss_fight;
        combat.meta.is_elite_fight = is_elite_fight;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: if is_boss_fight {
                    RoomType::MonsterRoomBoss
                } else if is_elite_fight {
                    RoomType::MonsterRoomElite
                } else {
                    RoomType::MonsterRoom
                },
            }),
        ));
        session
    }

    #[test]
    fn lane_options_attach_survival_hp_loss_gate() {
        for (current_hp, max_hp, expected_max_loss) in [
            (80, 80, 60),
            (54, 80, 34),
            (29, 79, 10),
            (54, 85, 33),
            (17, 85, 0),
        ] {
            let mut session = session_with_combat_stakes(false, false);
            let player = &mut session
                .active_combat
                .as_mut()
                .expect("active combat")
                .combat_state
                .entities
                .player;
            player.current_hp = current_hp;
            player.max_hp = max_hp;
            let request = CombatSearchRequest::from_session(&session, test_args());

            let options = lane_options(CombatSearchLane::primary(), &request, &session);

            assert_eq!(
                options.search.max_hp_loss,
                Some(RunControlHpLossLimit::Limit(expected_max_loss)),
                "visible hp {current_hp}/{max_hp}"
            );
        }
    }

    #[test]
    fn all_post_primary_hallway_lanes_generate_relaxed_candidates() {
        let mut session = session_with_combat_stakes(false, false);
        let player = &mut session
            .active_combat
            .as_mut()
            .expect("active combat")
            .combat_state
            .entities
            .player;
        player.current_hp = 20;
        player.max_hp = 74;
        let request = CombatSearchRequest::from_session(&session, test_args());

        let quality = lane_options(
            CombatSearchLane::new(CombatSearchLaneKind::HallwayQualityPotionRescue),
            &request,
            &session,
        );
        let fallback = lane_options(
            CombatSearchLane::new(CombatSearchLaneKind::HallwaySurvivalFallback),
            &request,
            &session,
        );
        let fallback_config = fallback.search.profile.expect("profile").to_config();

        assert_eq!(
            quality.search.max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(
            fallback.search.max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(
            fallback_config.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(fallback_config.max_potions_used, Some(2));
    }

    #[test]
    fn hallway_quality_and_survival_use_complementary_engines_at_same_budget() {
        let session = session_with_combat_stakes(false, false);
        let request = CombatSearchRequest::from_session(&session, test_args());
        let quality = lane_options(
            CombatSearchLane::new(CombatSearchLaneKind::HallwayQualityPotionRescue),
            &request,
            &session,
        )
        .search
        .profile
        .expect("quality profile");
        let survival = lane_options(
            CombatSearchLane::new(CombatSearchLaneKind::HallwaySurvivalFallback),
            &request,
            &session,
        )
        .search
        .profile
        .expect("survival profile");

        assert_eq!(
            quality.engine.plugins.child_rollout,
            CombatSearchChildRolloutPluginId::Immediate
        );
        assert_eq!(
            survival.engine.plugins.child_rollout,
            CombatSearchChildRolloutPluginId::LazyOnPop
        );
        assert_eq!(quality.engine.budget, survival.engine.budget);
        assert_ne!(quality.engine_fingerprint(), survival.engine_fingerprint());
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
        assert_eq!(profile.engine.budget.max_nodes, 22);
        assert_eq!(profile.engine.budget.wall_ms, 202);
        assert_eq!(
            profile.engine.plugins.child_rollout,
            sts_simulator::ai::combat_search_v2::CombatSearchChildRolloutPluginId::Immediate
        );
        assert_eq!(
            profile.engine.plugins.turn_plan,
            sts_simulator::ai::combat_search_v2::CombatSearchTurnPlanPluginId::Disabled
        );
        assert_eq!(
            profile.policy.acceptance,
            sts_simulator::ai::combat_search_v2::CombatSearchAcceptancePluginId::AcceptedLineOnly
        );
    }

    #[test]
    fn primary_boss_lane_uses_boss_budget_and_all_potions() {
        let session = session_with_combat_stakes(true, false);
        let request = CombatSearchRequest::from_session(&session, test_args());
        let options = lane_options(CombatSearchLane::primary(), &request, &session);
        let config = options.search.profile.expect("profile").to_config();

        assert_eq!(config.max_nodes, test_args().boss_search_nodes);
        assert_eq!(
            config.wall_time.map(|duration| duration.as_millis() as u64),
            Some(test_args().boss_search_ms)
        );
        assert_eq!(
            config.potion_policy,
            sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::All
        );
        assert_eq!(config.max_potions_used, Some(3));
        assert_eq!(
            config.child_rollout_policy,
            sts_simulator::ai::combat_search_v2::CombatSearchV2ChildRolloutPolicy::Immediate
        );
    }

    #[test]
    fn act2_boss_potion_rescue_uses_lazy_complete_win_profile() {
        let mut session = session_with_combat_stakes(true, false);
        session.run_state.act_num = 2;
        let request = CombatSearchRequest::from_session(&session, test_args());
        let lane = CombatSearchLane::new(CombatSearchLaneKind::BossPotionRescue);
        let options = lane_options(lane, &request, &session);
        let config = options.search.profile.expect("profile").to_config();

        assert_eq!(config.max_nodes, test_args().boss_search_nodes);
        assert_eq!(
            config.wall_time.map(|duration| duration.as_millis() as u64),
            Some(test_args().boss_search_ms)
        );
        assert_eq!(
            config.child_rollout_policy,
            sts_simulator::ai::combat_search_v2::CombatSearchV2ChildRolloutPolicy::LazyOnPop
        );
        assert_eq!(
            config.rollout_policy,
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
        );
        assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::All);
        assert_eq!(config.max_potions_used, Some(3));
        assert_eq!(
            lane.acceptance_plugin(),
            sts_simulator::ai::combat_search_v2::CombatSearchAcceptancePluginId::AcceptedLineOnly
        );
    }

    #[test]
    fn primary_elite_lane_uses_rescue_budget_and_one_semantic_potion() {
        let session = session_with_combat_stakes(false, true);
        let request = CombatSearchRequest::from_session(&session, test_args());
        let options = lane_options(CombatSearchLane::primary(), &request, &session);
        assert_eq!(
            options.search.potion_policy,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            "owner-audit must pin its profile policy so run-control does not stage another portfolio"
        );
        let config = options.search.profile.expect("profile").to_config();

        assert_eq!(config.max_nodes, test_args().rescue_search_nodes);
        assert_eq!(
            config.wall_time.map(|duration| duration.as_millis() as u64),
            Some(test_args().rescue_search_ms)
        );
        assert_eq!(
            config.potion_policy,
            sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(1));
        assert_eq!(
            config.child_rollout_policy,
            sts_simulator::ai::combat_search_v2::CombatSearchV2ChildRolloutPolicy::Immediate
        );
    }

    #[test]
    fn primary_hallway_lane_uses_primary_budget_immediate_rollout_and_one_semantic_potion() {
        let session = session_with_combat_stakes(false, false);
        let request = CombatSearchRequest::from_session(&session, test_args());
        let options = lane_options(CombatSearchLane::primary(), &request, &session);
        let config = options.search.profile.expect("profile").to_config();

        assert_eq!(config.max_nodes, test_args().search_nodes);
        assert_eq!(
            config.wall_time.map(|duration| duration.as_millis() as u64),
            Some(test_args().search_ms)
        );
        assert_eq!(
            config.child_rollout_policy,
            sts_simulator::ai::combat_search_v2::CombatSearchV2ChildRolloutPolicy::Immediate
        );
        assert_eq!(
            config.potion_policy,
            sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(1));
    }

    #[test]
    fn elite_survival_fallback_uses_bounded_quality_profile() {
        let mut session = session_with_combat_stakes(false, true);
        let player = &mut session
            .active_combat
            .as_mut()
            .expect("active combat")
            .combat_state
            .entities
            .player;
        player.current_hp = 45;
        player.max_hp = 74;
        let request = CombatSearchRequest::from_session(&session, test_args());
        let primary = lane_options(CombatSearchLane::primary(), &request, &session);
        let lane = CombatSearchLane::new(CombatSearchLaneKind::EliteSurvivalFallback);
        let fallback = lane_options(lane, &request, &session);
        let config = fallback.search.profile.expect("profile").to_config();

        assert_eq!(
            primary.search.max_hp_loss,
            Some(RunControlHpLossLimit::Limit(27))
        );
        assert_eq!(
            fallback.search.max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(
            lane.acceptance_plugin(),
            sts_simulator::ai::combat_search_v2::CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse
        );
        assert_eq!(config.max_nodes, test_args().boss_search_nodes);
        assert_eq!(
            config.wall_time.map(|duration| duration.as_millis() as u64),
            Some(5_000)
        );
        assert_eq!(
            config.child_rollout_policy,
            sts_simulator::ai::combat_search_v2::CombatSearchV2ChildRolloutPolicy::Immediate
        );
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
        assert_eq!(config.max_potions_used, Some(1));
    }

    #[test]
    fn only_survival_lanes_allow_escape_only_fallback() {
        assert!(!lane_allows_internal_no_win_rescue(
            CombatSearchLane::primary()
        ));
        assert!(lane_allows_internal_no_win_rescue(CombatSearchLane::new(
            CombatSearchLaneKind::DiagnosticRescue
        )));
        assert!(lane_allows_internal_no_win_rescue(CombatSearchLane::new(
            CombatSearchLaneKind::EliteSurvivalFallback
        )));
        assert!(lane_allows_internal_no_win_rescue(CombatSearchLane::new(
            CombatSearchLaneKind::HallwaySurvivalFallback
        )));
        assert!(!lane_allows_internal_no_win_rescue(CombatSearchLane::new(
            CombatSearchLaneKind::BossNoPotion
        )));

        assert!(!lane_allows_smoke_bomb_survival_fallback(
            CombatSearchLane::primary()
        ));
        assert!(lane_allows_smoke_bomb_survival_fallback(
            CombatSearchLane::new(CombatSearchLaneKind::EliteSurvivalFallback)
        ));
        assert!(lane_allows_smoke_bomb_survival_fallback(
            CombatSearchLane::new(CombatSearchLaneKind::HallwaySurvivalFallback)
        ));
        assert!(!lane_allows_smoke_bomb_survival_fallback(
            CombatSearchLane::new(CombatSearchLaneKind::HallwayQualityPotionRescue)
        ));
        assert!(!lane_allows_smoke_bomb_survival_fallback(
            CombatSearchLane::new(CombatSearchLaneKind::BossPotionRescue)
        ));
    }
}
