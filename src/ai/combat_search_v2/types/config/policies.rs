use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2SetupBiasPolicy {
    Default,
    KeyCardOnline,
}

impl Default for CombatSearchV2SetupBiasPolicy {
    fn default() -> Self {
        Self::Default
    }
}

impl CombatSearchV2SetupBiasPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::KeyCardOnline => "key_card_online",
        }
    }

    pub(in crate::ai::combat_search_v2) fn prioritizes_key_card_online(self) -> bool {
        matches!(self, Self::KeyCardOnline)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2PhaseGuardPolicy {
    Default,
    ChampSplitGuard,
    TimeEaterClockHint,
}

impl Default for CombatSearchV2PhaseGuardPolicy {
    fn default() -> Self {
        Self::Default
    }
}

impl CombatSearchV2PhaseGuardPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::ChampSplitGuard => "champ_split_guard",
            Self::TimeEaterClockHint => "time_eater_clock_hint",
        }
    }

    pub(in crate::ai::combat_search_v2) fn guards_champ_split(self) -> bool {
        matches!(self, Self::ChampSplitGuard)
    }

    pub(in crate::ai::combat_search_v2) fn guards_time_eater_clock(self) -> bool {
        matches!(self, Self::TimeEaterClockHint)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2PotionPolicy {
    Never,
    #[serde(alias = "all_legal_potion_actions")]
    All,
    #[serde(alias = "semantic_budgeted_potion_actions")]
    SemanticBudgeted,
}

impl CombatSearchV2PotionPolicy {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            CombatSearchV2PotionPolicy::Never => "never",
            CombatSearchV2PotionPolicy::All => "all_legal_potion_actions",
            CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic_budgeted_potion_actions",
        }
    }
}

const HIGH_STAKES_BOSS_MAX_POTIONS_USED: u32 = 2;
const HIGH_STAKES_ELITE_MAX_POTIONS_USED: u32 = 1;

pub fn high_stakes_semantic_potion_budget(
    combat: &crate::runtime::combat::CombatState,
) -> Option<u32> {
    if combat.meta.is_boss_fight {
        Some(HIGH_STAKES_BOSS_MAX_POTIONS_USED)
    } else if combat.meta.is_elite_fight {
        Some(HIGH_STAKES_ELITE_MAX_POTIONS_USED)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2ChildRolloutPolicy {
    Immediate,
    LazyOnPop,
}

impl Default for CombatSearchV2ChildRolloutPolicy {
    fn default() -> Self {
        Self::LazyOnPop
    }
}

impl CombatSearchV2ChildRolloutPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::LazyOnPop => "lazy_on_pop",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2RolloutPolicy {
    Disabled,
    #[serde(alias = "adaptive_no_potion")]
    EnemyMechanicsAdaptiveNoPotion,
    ConservativeNoPotion,
    PhaseAwareNoPotion,
    TurnBeamNoPotion,
}

impl Default for CombatSearchV2RolloutPolicy {
    fn default() -> Self {
        Self::EnemyMechanicsAdaptiveNoPotion
    }
}

impl CombatSearchV2RolloutPolicy {
    pub fn label(self) -> &'static str {
        match self {
            CombatSearchV2RolloutPolicy::Disabled => "disabled",
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion => {
                "enemy_mechanics_adaptive_no_potion"
            }
            CombatSearchV2RolloutPolicy::ConservativeNoPotion => "conservative_no_potion",
            CombatSearchV2RolloutPolicy::PhaseAwareNoPotion => "phase_aware_no_potion",
            CombatSearchV2RolloutPolicy::TurnBeamNoPotion => "turn_beam_no_potion",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2TurnPlanPolicy {
    DiagnosticOnly,
    RootFrontierSeed,
    TurnBoundaryFrontierSeed,
    #[serde(alias = "support_enemy_turn_boundary_frontier_seed")]
    TacticalEnemyTurnBoundaryFrontierSeed,
}

impl Default for CombatSearchV2TurnPlanPolicy {
    fn default() -> Self {
        Self::DiagnosticOnly
    }
}

impl CombatSearchV2TurnPlanPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::DiagnosticOnly => "diagnostic_only",
            Self::RootFrontierSeed => "root_frontier_seed",
            Self::TurnBoundaryFrontierSeed => "turn_boundary_frontier_seed",
            Self::TacticalEnemyTurnBoundaryFrontierSeed => {
                "tactical_enemy_turn_boundary_frontier_seed"
            }
        }
    }

    pub(in crate::ai::combat_search_v2) fn seeds_root_frontier(self) -> bool {
        matches!(
            self,
            Self::RootFrontierSeed | Self::TurnBoundaryFrontierSeed
        )
    }

    pub(in crate::ai::combat_search_v2) fn seeds_turn_boundary_frontier(self) -> bool {
        matches!(
            self,
            Self::TurnBoundaryFrontierSeed | Self::TacticalEnemyTurnBoundaryFrontierSeed
        )
    }

    pub(in crate::ai::combat_search_v2) fn requires_tactical_enemy_gate(self) -> bool {
        matches!(self, Self::TacticalEnemyTurnBoundaryFrontierSeed)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2FrontierPolicy {
    SingleQueue,
    RoundRobinEvalBuckets,
}

impl Default for CombatSearchV2FrontierPolicy {
    fn default() -> Self {
        Self::SingleQueue
    }
}

impl CombatSearchV2FrontierPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::SingleQueue => "single_queue",
            Self::RoundRobinEvalBuckets => "round_robin_eval_buckets",
        }
    }
}
