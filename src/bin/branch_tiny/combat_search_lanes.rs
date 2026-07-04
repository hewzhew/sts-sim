use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::run_control::{
    RunControlAutoStepOptions, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession,
};

use super::Args;

const BOSS_POTION_RESCUE_MAX_POTIONS_USED: u32 = 3;
const NONBOSS_POTION_RESCUE_MAX_POTIONS_USED: u32 = 1;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum CombatSearchStakes {
    Hallway,
    Elite,
    Boss,
}

#[derive(Clone, Copy)]
pub(super) enum CombatSearchLaneKind {
    Primary,
    DiagnosticRescue,
    HallwayImmediateRescue,
    NonBossPotionRescue,
    BossNoPotion,
    BossPotionRescue,
    QualityRealHp,
}

#[derive(Clone, Copy)]
pub(super) enum CombatSearchLaneCommitPolicy {
    AcceptedLineOnly,
    AcceptedLineOrPrimaryChunk,
}

#[derive(Clone, Copy)]
pub(super) struct CombatSearchLane {
    kind: CombatSearchLaneKind,
}

pub(super) struct CombatSearchRequest {
    pub(super) args: Args,
    pub(super) stakes: CombatSearchStakes,
}

impl CombatSearchRequest {
    pub(super) fn from_session(session: &RunControlSession, args: Args) -> Self {
        Self {
            args,
            stakes: combat_search_stakes(session),
        }
    }

    pub(super) fn portfolio_after_primary(
        &self,
        session: &RunControlSession,
    ) -> Vec<CombatSearchLane> {
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

    pub(super) fn should_report(&self) -> bool {
        self.stakes == CombatSearchStakes::Boss
    }

    pub(super) fn combat_budget_capped(&self) -> bool {
        match self.stakes {
            CombatSearchStakes::Boss => self.args.wall_capped_boss_budget,
            CombatSearchStakes::Elite | CombatSearchStakes::Hallway => {
                self.args.wall_capped_search_budget
            }
        }
    }
}

impl CombatSearchLane {
    pub(super) fn primary() -> Self {
        Self::new(CombatSearchLaneKind::Primary)
    }

    fn new(kind: CombatSearchLaneKind) -> Self {
        Self { kind }
    }

    pub(super) fn label(self) -> &'static str {
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

    pub(super) fn commit_policy(self) -> CombatSearchLaneCommitPolicy {
        match self.kind {
            CombatSearchLaneKind::Primary => {
                CombatSearchLaneCommitPolicy::AcceptedLineOrPrimaryChunk
            }
            _ => CombatSearchLaneCommitPolicy::AcceptedLineOnly,
        }
    }

    pub(super) fn rejects_new_curses(self) -> bool {
        matches!(self.kind, CombatSearchLaneKind::NonBossPotionRescue)
    }

    pub(super) fn options(
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
