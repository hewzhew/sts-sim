use crate::ai::combat_search_v2::{
    CombatSearchProfile, CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy,
    CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2SetupBiasPolicy,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunControlSearchCombatOptions {
    pub profile: Option<CombatSearchProfile>,
    pub max_nodes: Option<usize>,
    pub max_actions_per_line: Option<usize>,
    pub max_engine_steps_per_action: Option<usize>,
    pub wall_ms: Option<u64>,
    pub max_hp_loss: Option<RunControlHpLossLimit>,
    pub potion_policy: Option<CombatSearchV2PotionPolicy>,
    pub max_potions_used: Option<u32>,
    pub rollout_policy: Option<CombatSearchV2RolloutPolicy>,
    pub child_rollout_policy: Option<CombatSearchV2ChildRolloutPolicy>,
    pub rollout_max_evaluations: Option<usize>,
    pub rollout_max_actions: Option<usize>,
    pub rollout_beam_width: Option<usize>,
    pub turn_plan_policy: Option<crate::ai::combat_search_v2::CombatSearchV2TurnPlanPolicy>,
    pub frontier_policy: Option<CombatSearchV2FrontierPolicy>,
    pub phase_guard_policy: Option<CombatSearchV2PhaseGuardPolicy>,
    pub setup_bias_policy: Option<CombatSearchV2SetupBiasPolicy>,
    pub segment_mode: Option<RunControlCombatSegmentMode>,
    pub disable_no_win_rescue: bool,
    pub allow_smoke_bomb_survival_fallback: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunControlCombatSegmentMode {
    TurnBoundary,
    NonBossTurnBoundary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunControlHpLossLimit {
    Limit(u32),
    Unlimited,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunControlAutoStepOptions {
    pub search: RunControlSearchCombatOptions,
    pub route: RunControlRouteAutomationMode,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RunControlRouteAutomationMode {
    #[default]
    Manual,
    Planner,
}
