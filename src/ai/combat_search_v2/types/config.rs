use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default)]
pub struct CombatSearchV2RootActionPrior {
    scores_by_state: Arc<HashMap<String, HashMap<String, f64>>>,
    duplicate_hint_count: usize,
}

impl CombatSearchV2RootActionPrior {
    pub fn from_scores(scores_by_state: HashMap<String, HashMap<String, f64>>) -> Self {
        Self::from_scores_with_duplicate_count(scores_by_state, 0)
    }

    pub fn from_scores_with_duplicate_count(
        scores_by_state: HashMap<String, HashMap<String, f64>>,
        duplicate_hint_count: usize,
    ) -> Self {
        Self {
            scores_by_state: Arc::new(scores_by_state),
            duplicate_hint_count,
        }
    }

    pub fn score(&self, exact_state_hash: &str, action_key: &str) -> Option<f64> {
        self.scores_by_state
            .get(exact_state_hash)
            .and_then(|scores| scores.get(action_key))
            .copied()
    }

    pub fn is_empty(&self) -> bool {
        self.scores_by_state.is_empty()
    }

    pub fn duplicate_hint_count(&self) -> usize {
        self.duplicate_hint_count
    }
}

#[derive(Clone, Debug, Default)]
pub struct CombatSearchV2TurnPlanPrior {
    scores_by_state: Arc<HashMap<String, HashMap<String, f64>>>,
    duplicate_hint_count: usize,
}

impl CombatSearchV2TurnPlanPrior {
    pub fn from_scores(scores_by_state: HashMap<String, HashMap<String, f64>>) -> Self {
        Self::from_scores_with_duplicate_count(scores_by_state, 0)
    }

    pub fn from_plan_scores<I, J>(scores_by_state: I) -> Self
    where
        I: IntoIterator<Item = (String, J)>,
        J: IntoIterator<Item = (Vec<String>, f64)>,
    {
        let mut keyed_scores = HashMap::new();
        for (state_hash, plan_scores) in scores_by_state {
            let mut state_scores = HashMap::new();
            for (action_keys, score) in plan_scores {
                if score.is_finite() {
                    state_scores.insert(turn_plan_action_sequence_key(&action_keys), score);
                }
            }
            keyed_scores.insert(state_hash, state_scores);
        }
        Self::from_scores(keyed_scores)
    }

    pub fn from_scores_with_duplicate_count(
        scores_by_state: HashMap<String, HashMap<String, f64>>,
        duplicate_hint_count: usize,
    ) -> Self {
        Self {
            scores_by_state: Arc::new(scores_by_state),
            duplicate_hint_count,
        }
    }

    pub fn score_for_action_keys(
        &self,
        exact_state_hash: &str,
        action_keys: &[String],
    ) -> Option<f64> {
        self.scores_by_state
            .get(exact_state_hash)
            .and_then(|scores| scores.get(&turn_plan_action_sequence_key(action_keys)))
            .copied()
    }

    pub fn has_hints_for_state(&self, exact_state_hash: &str) -> bool {
        self.scores_by_state
            .get(exact_state_hash)
            .is_some_and(|scores| !scores.is_empty())
    }

    pub fn is_empty(&self) -> bool {
        self.scores_by_state.is_empty()
    }

    pub fn duplicate_hint_count(&self) -> usize {
        self.duplicate_hint_count
    }
}

pub fn turn_plan_action_sequence_key(action_keys: &[String]) -> String {
    action_keys.join("\u{1f}")
}

#[derive(Clone, Debug)]
pub struct CombatSearchV2Config {
    pub max_nodes: usize,
    pub max_actions_per_line: usize,
    pub max_engine_steps_per_action: usize,
    pub wall_time: Option<Duration>,
    pub stop_on_win_hp_loss_at_most: Option<u32>,
    pub min_win_candidates_before_stop: usize,
    pub input_label: Option<String>,
    pub potion_policy: CombatSearchV2PotionPolicy,
    pub max_potions_used: Option<u32>,
    pub rollout_policy: CombatSearchV2RolloutPolicy,
    pub child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    pub rollout_max_evaluations: usize,
    pub rollout_max_actions: usize,
    pub rollout_beam_width: usize,
    pub turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    pub frontier_policy: CombatSearchV2FrontierPolicy,
    pub phase_guard_policy: CombatSearchV2PhaseGuardPolicy,
    pub turn_plan_probe_max_inner_nodes: Option<usize>,
    pub turn_plan_probe_max_end_states: Option<usize>,
    pub turn_plan_probe_per_bucket_limit: Option<usize>,
    pub root_action_prior: Option<CombatSearchV2RootActionPrior>,
    pub turn_plan_prior: Option<CombatSearchV2TurnPlanPrior>,
}

impl Default for CombatSearchV2Config {
    fn default() -> Self {
        Self {
            max_nodes: 50_000,
            max_actions_per_line: 200,
            max_engine_steps_per_action: 250,
            wall_time: None,
            stop_on_win_hp_loss_at_most: None,
            min_win_candidates_before_stop: 1,
            input_label: None,
            potion_policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: None,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::default(),
            rollout_max_evaluations: super::super::rollout::DEFAULT_ROLLOUT_MAX_EVALUATIONS,
            rollout_max_actions: super::super::rollout::DEFAULT_ROLLOUT_MAX_ACTIONS,
            rollout_beam_width: super::super::rollout::DEFAULT_TURN_BEAM_WIDTH,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
            turn_plan_probe_max_inner_nodes: None,
            turn_plan_probe_max_end_states: None,
            turn_plan_probe_per_bucket_limit: None,
            root_action_prior: None,
            turn_plan_prior: None,
        }
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
