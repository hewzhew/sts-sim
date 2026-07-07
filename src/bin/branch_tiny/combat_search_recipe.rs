use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::run_control::{
    RunControlAutoStepOptions, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions,
};

#[derive(Clone, Copy)]
pub(super) struct CombatSearchRecipe {
    max_nodes: usize,
    wall_ms: u64,
    auto_ops: usize,
    wall_limited: bool,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    rollout_policy: Option<CombatSearchV2RolloutPolicy>,
    frontier_policy: Option<CombatSearchV2FrontierPolicy>,
    potion_policy: Option<CombatSearchV2PotionPolicy>,
    max_potions_used: Option<u32>,
    phase_guard_policy: Option<CombatSearchV2PhaseGuardPolicy>,
}

impl CombatSearchRecipe {
    pub(super) fn new(
        max_nodes: usize,
        wall_ms: u64,
        auto_ops: usize,
        wall_limited: bool,
        turn_plan_policy: CombatSearchV2TurnPlanPolicy,
        child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    ) -> Self {
        Self {
            max_nodes,
            wall_ms,
            auto_ops,
            wall_limited,
            turn_plan_policy,
            child_rollout_policy,
            rollout_policy: None,
            frontier_policy: None,
            potion_policy: None,
            max_potions_used: None,
            phase_guard_policy: None,
        }
    }

    pub(super) fn with_rollout_policy(mut self, policy: CombatSearchV2RolloutPolicy) -> Self {
        self.rollout_policy = Some(policy);
        self
    }

    pub(super) fn with_frontier_policy(mut self, policy: CombatSearchV2FrontierPolicy) -> Self {
        self.frontier_policy = Some(policy);
        self
    }

    pub(super) fn with_potion_policy(mut self, policy: CombatSearchV2PotionPolicy) -> Self {
        self.potion_policy = Some(policy);
        self
    }

    pub(super) fn with_max_potions_used(mut self, max_potions_used: u32) -> Self {
        self.max_potions_used = Some(max_potions_used);
        self
    }

    pub(super) fn with_phase_guard_policy(
        mut self,
        policy: CombatSearchV2PhaseGuardPolicy,
    ) -> Self {
        self.phase_guard_policy = Some(policy);
        self
    }

    pub(super) fn into_auto_step_options(self) -> RunControlAutoStepOptions {
        RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions {
                max_nodes: Some(self.max_nodes),
                wall_ms: Some(self.wall_ms),
                max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
                turn_plan_policy: Some(self.turn_plan_policy),
                child_rollout_policy: Some(self.child_rollout_policy),
                rollout_policy: self.rollout_policy,
                frontier_policy: self.frontier_policy,
                potion_policy: self.potion_policy,
                max_potions_used: self.max_potions_used,
                phase_guard_policy: self.phase_guard_policy,
                ..Default::default()
            },
            max_operations: Some(auto_run_chunk_ops(self.auto_ops, self.wall_limited)),
            route: RunControlRouteAutomationMode::Planner,
        }
    }
}

fn auto_run_chunk_ops(auto_ops: usize, wall_limited: bool) -> usize {
    if wall_limited {
        1
    } else {
        auto_ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_materializes_core_search_options() {
        let options = CombatSearchRecipe::new(
            123,
            456,
            7,
            false,
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2ChildRolloutPolicy::LazyOnPop,
        )
        .with_rollout_policy(CombatSearchV2RolloutPolicy::Disabled)
        .with_frontier_policy(CombatSearchV2FrontierPolicy::SingleQueue)
        .with_potion_policy(CombatSearchV2PotionPolicy::Never)
        .with_max_potions_used(0)
        .with_phase_guard_policy(CombatSearchV2PhaseGuardPolicy::ChampSplitGuard)
        .into_auto_step_options();

        assert_eq!(options.search.max_nodes, Some(123));
        assert_eq!(options.search.wall_ms, Some(456));
        assert_eq!(options.max_operations, Some(7));
        assert_eq!(options.route, RunControlRouteAutomationMode::Planner);
        assert_eq!(
            options.search.turn_plan_policy,
            Some(CombatSearchV2TurnPlanPolicy::DiagnosticOnly)
        );
        assert_eq!(
            options.search.child_rollout_policy,
            Some(CombatSearchV2ChildRolloutPolicy::LazyOnPop)
        );
        assert_eq!(
            options.search.rollout_policy,
            Some(CombatSearchV2RolloutPolicy::Disabled)
        );
        assert_eq!(
            options.search.frontier_policy,
            Some(CombatSearchV2FrontierPolicy::SingleQueue)
        );
        assert_eq!(
            options.search.potion_policy,
            Some(CombatSearchV2PotionPolicy::Never)
        );
        assert_eq!(options.search.max_potions_used, Some(0));
        assert_eq!(
            options.search.phase_guard_policy,
            Some(CombatSearchV2PhaseGuardPolicy::ChampSplitGuard)
        );
    }

    #[test]
    fn wall_limited_recipe_uses_single_operation_chunk() {
        let options = CombatSearchRecipe::new(
            10,
            20,
            99,
            true,
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2ChildRolloutPolicy::Immediate,
        )
        .into_auto_step_options();

        assert_eq!(options.max_operations, Some(1));
    }
}
