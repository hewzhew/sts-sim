use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2Config, CombatSearchV2FrontierPolicy,
    CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CombatSearchBudgetSpec {
    pub max_nodes: usize,
    pub wall_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CombatSearchProfile {
    pub label: &'static str,
    pub budget: CombatSearchBudgetSpec,
    pub plugins: CombatSearchPluginStack,
    pub acceptance: CombatSearchAcceptancePluginId,
    pub artifacts: CombatSearchArtifactPluginId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CombatSearchPluginStack {
    pub action_prior: CombatSearchActionPriorPluginId,
    pub node_evaluator: CombatSearchNodeEvaluatorPluginId,
    pub turn_plan: CombatSearchTurnPlanPluginId,
    pub child_rollout: CombatSearchChildRolloutPluginId,
    pub rollout: CombatSearchRolloutPluginId,
    pub frontier: CombatSearchFrontierPluginId,
    pub potion: CombatSearchPotionPlugin,
    pub phase_guard: CombatSearchPhaseGuardPluginId,
}

impl Default for CombatSearchPluginStack {
    fn default() -> Self {
        Self {
            action_prior: CombatSearchActionPriorPluginId::Default,
            node_evaluator: CombatSearchNodeEvaluatorPluginId::CombatOutcomeScore,
            turn_plan: CombatSearchTurnPlanPluginId::DiagnosticOnly,
            child_rollout: CombatSearchChildRolloutPluginId::LazyOnPop,
            rollout: CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion,
            frontier: CombatSearchFrontierPluginId::RoundRobinEvalBuckets,
            potion: CombatSearchPotionPlugin::default(),
            phase_guard: CombatSearchPhaseGuardPluginId::Default,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CombatSearchPotionPlugin {
    pub policy: CombatSearchV2PotionPolicy,
    pub max_potions_used: Option<u32>,
}

impl Default for CombatSearchPotionPlugin {
    fn default() -> Self {
        Self {
            policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchActionPriorPluginId {
    Default,
    KeyCardOnline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchNodeEvaluatorPluginId {
    CombatOutcomeScore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchTurnPlanPluginId {
    DiagnosticOnly,
    RootFrontierSeed,
    TurnBoundaryFrontierSeed,
    TacticalEnemyTurnBoundaryFrontierSeed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchChildRolloutPluginId {
    Immediate,
    LazyOnPop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchRolloutPluginId {
    Disabled,
    EnemyMechanicsAdaptiveNoPotion,
    ConservativeNoPotion,
    PhaseAwareNoPotion,
    TurnBeamNoPotion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchFrontierPluginId {
    SingleQueue,
    RoundRobinEvalBuckets,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchPhaseGuardPluginId {
    Default,
    ChampSplitGuard,
    TimeEaterClockHint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchAcceptancePluginId {
    AcceptedLineOnly,
    AcceptedLineOrPrimaryChunk,
    CleanAcceptedLineNoNewCurse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchArtifactPluginId {
    None,
    PortfolioAttempt,
    DecisionMicroscope,
    FullTrace,
}

pub trait CombatSearchActionPriorPlugin {
    fn id(&self) -> CombatSearchActionPriorPluginId;
}

pub trait CombatSearchNodeEvaluatorPlugin {
    fn id(&self) -> CombatSearchNodeEvaluatorPluginId;
}

pub trait CombatSearchTurnPlanPlugin {
    fn id(&self) -> CombatSearchTurnPlanPluginId;
}

pub trait CombatSearchChildRolloutPlugin {
    fn id(&self) -> CombatSearchChildRolloutPluginId;
}

pub trait CombatSearchRolloutPlugin {
    fn id(&self) -> CombatSearchRolloutPluginId;
}

pub trait CombatSearchFrontierPlugin {
    fn id(&self) -> CombatSearchFrontierPluginId;
}

pub trait CombatSearchPhaseGuardPlugin {
    fn id(&self) -> CombatSearchPhaseGuardPluginId;
}

pub trait CombatSearchAcceptancePlugin {
    fn id(&self) -> CombatSearchAcceptancePluginId;
}

pub trait CombatSearchArtifactPlugin {
    fn id(&self) -> CombatSearchArtifactPluginId;
}

impl CombatSearchActionPriorPlugin for CombatSearchActionPriorPluginId {
    fn id(&self) -> CombatSearchActionPriorPluginId {
        *self
    }
}

impl CombatSearchNodeEvaluatorPlugin for CombatSearchNodeEvaluatorPluginId {
    fn id(&self) -> CombatSearchNodeEvaluatorPluginId {
        *self
    }
}

impl CombatSearchTurnPlanPlugin for CombatSearchTurnPlanPluginId {
    fn id(&self) -> CombatSearchTurnPlanPluginId {
        *self
    }
}

impl CombatSearchChildRolloutPlugin for CombatSearchChildRolloutPluginId {
    fn id(&self) -> CombatSearchChildRolloutPluginId {
        *self
    }
}

impl CombatSearchRolloutPlugin for CombatSearchRolloutPluginId {
    fn id(&self) -> CombatSearchRolloutPluginId {
        *self
    }
}

impl CombatSearchFrontierPlugin for CombatSearchFrontierPluginId {
    fn id(&self) -> CombatSearchFrontierPluginId {
        *self
    }
}

impl CombatSearchPhaseGuardPlugin for CombatSearchPhaseGuardPluginId {
    fn id(&self) -> CombatSearchPhaseGuardPluginId {
        *self
    }
}

impl CombatSearchAcceptancePlugin for CombatSearchAcceptancePluginId {
    fn id(&self) -> CombatSearchAcceptancePluginId {
        *self
    }
}

impl CombatSearchArtifactPlugin for CombatSearchArtifactPluginId {
    fn id(&self) -> CombatSearchArtifactPluginId {
        *self
    }
}

impl CombatSearchProfile {
    pub fn with_acceptance(mut self, acceptance: CombatSearchAcceptancePluginId) -> Self {
        self.acceptance = acceptance;
        self
    }

    pub fn with_rollout_plugin(mut self, rollout: CombatSearchRolloutPluginId) -> Self {
        self.plugins.rollout = rollout;
        self
    }

    pub fn with_frontier_plugin(mut self, frontier: CombatSearchFrontierPluginId) -> Self {
        self.plugins.frontier = frontier;
        self
    }

    pub fn with_potion_policy(mut self, policy: CombatSearchV2PotionPolicy) -> Self {
        self.plugins.potion.policy = policy;
        self
    }

    pub fn with_max_potions_used(mut self, max_potions_used: u32) -> Self {
        self.plugins.potion.max_potions_used = Some(max_potions_used);
        self
    }

    pub fn with_phase_guard_plugin(mut self, phase_guard: CombatSearchPhaseGuardPluginId) -> Self {
        self.plugins.phase_guard = phase_guard;
        self
    }

    pub fn to_config(self) -> CombatSearchV2Config {
        let defaults = CombatSearchV2Config::default();
        CombatSearchV2Config {
            max_nodes: self.budget.max_nodes,
            wall_time: Some(Duration::from_millis(self.budget.wall_ms)),
            potion_policy: self.plugins.potion.policy,
            max_potions_used: self.plugins.potion.max_potions_used,
            rollout_policy: self.plugins.rollout.into(),
            child_rollout_policy: self.plugins.child_rollout.into(),
            turn_plan_policy: self.plugins.turn_plan.into(),
            frontier_policy: self.plugins.frontier.into(),
            phase_guard_policy: self.plugins.phase_guard.into(),
            setup_bias_policy: self.plugins.action_prior.into(),
            ..defaults
        }
    }
}

impl From<CombatSearchActionPriorPluginId> for CombatSearchV2SetupBiasPolicy {
    fn from(plugin: CombatSearchActionPriorPluginId) -> Self {
        match plugin {
            CombatSearchActionPriorPluginId::Default => Self::Default,
            CombatSearchActionPriorPluginId::KeyCardOnline => Self::KeyCardOnline,
        }
    }
}

impl From<CombatSearchTurnPlanPluginId> for CombatSearchV2TurnPlanPolicy {
    fn from(plugin: CombatSearchTurnPlanPluginId) -> Self {
        match plugin {
            CombatSearchTurnPlanPluginId::DiagnosticOnly => Self::DiagnosticOnly,
            CombatSearchTurnPlanPluginId::RootFrontierSeed => Self::RootFrontierSeed,
            CombatSearchTurnPlanPluginId::TurnBoundaryFrontierSeed => {
                Self::TurnBoundaryFrontierSeed
            }
            CombatSearchTurnPlanPluginId::TacticalEnemyTurnBoundaryFrontierSeed => {
                Self::TacticalEnemyTurnBoundaryFrontierSeed
            }
        }
    }
}

impl From<CombatSearchChildRolloutPluginId> for CombatSearchV2ChildRolloutPolicy {
    fn from(plugin: CombatSearchChildRolloutPluginId) -> Self {
        match plugin {
            CombatSearchChildRolloutPluginId::Immediate => Self::Immediate,
            CombatSearchChildRolloutPluginId::LazyOnPop => Self::LazyOnPop,
        }
    }
}

impl From<CombatSearchRolloutPluginId> for CombatSearchV2RolloutPolicy {
    fn from(plugin: CombatSearchRolloutPluginId) -> Self {
        match plugin {
            CombatSearchRolloutPluginId::Disabled => Self::Disabled,
            CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion => {
                Self::EnemyMechanicsAdaptiveNoPotion
            }
            CombatSearchRolloutPluginId::ConservativeNoPotion => Self::ConservativeNoPotion,
            CombatSearchRolloutPluginId::PhaseAwareNoPotion => Self::PhaseAwareNoPotion,
            CombatSearchRolloutPluginId::TurnBeamNoPotion => Self::TurnBeamNoPotion,
        }
    }
}

impl From<CombatSearchFrontierPluginId> for CombatSearchV2FrontierPolicy {
    fn from(plugin: CombatSearchFrontierPluginId) -> Self {
        match plugin {
            CombatSearchFrontierPluginId::SingleQueue => Self::SingleQueue,
            CombatSearchFrontierPluginId::RoundRobinEvalBuckets => Self::RoundRobinEvalBuckets,
        }
    }
}

impl From<CombatSearchPhaseGuardPluginId> for CombatSearchV2PhaseGuardPolicy {
    fn from(plugin: CombatSearchPhaseGuardPluginId) -> Self {
        match plugin {
            CombatSearchPhaseGuardPluginId::Default => Self::Default,
            CombatSearchPhaseGuardPluginId::ChampSplitGuard => Self::ChampSplitGuard,
            CombatSearchPhaseGuardPluginId::TimeEaterClockHint => Self::TimeEaterClockHint,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_ids_implement_their_role_traits() {
        fn action_prior_id(
            plugin: impl CombatSearchActionPriorPlugin,
        ) -> CombatSearchActionPriorPluginId {
            plugin.id()
        }
        fn evaluator_id(
            plugin: impl CombatSearchNodeEvaluatorPlugin,
        ) -> CombatSearchNodeEvaluatorPluginId {
            plugin.id()
        }
        fn frontier_id(plugin: impl CombatSearchFrontierPlugin) -> CombatSearchFrontierPluginId {
            plugin.id()
        }
        fn rollout_id(plugin: impl CombatSearchRolloutPlugin) -> CombatSearchRolloutPluginId {
            plugin.id()
        }
        fn acceptance_id(
            plugin: impl CombatSearchAcceptancePlugin,
        ) -> CombatSearchAcceptancePluginId {
            plugin.id()
        }

        assert_eq!(
            action_prior_id(CombatSearchActionPriorPluginId::KeyCardOnline),
            CombatSearchActionPriorPluginId::KeyCardOnline
        );
        assert_eq!(
            evaluator_id(CombatSearchNodeEvaluatorPluginId::CombatOutcomeScore),
            CombatSearchNodeEvaluatorPluginId::CombatOutcomeScore
        );
        assert_eq!(
            frontier_id(CombatSearchFrontierPluginId::RoundRobinEvalBuckets),
            CombatSearchFrontierPluginId::RoundRobinEvalBuckets
        );
        assert_eq!(
            rollout_id(CombatSearchRolloutPluginId::TurnBeamNoPotion),
            CombatSearchRolloutPluginId::TurnBeamNoPotion
        );
        assert_eq!(
            acceptance_id(CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse),
            CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse
        );
    }
}
