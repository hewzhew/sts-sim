//! Engine-neutral combat-search vocabulary.
//!
//! The repository has two intentionally different exact combat solvers:
//! atomic best-first search and the production complete-turn portfolio.
//! Shared request concepts and durable engine identity belong here; each
//! engine still validates its own supported satisfaction modes. Rollout,
//! turn-plan, frontier, and plugin controls remain owned by the atomic engine.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatWitnessSatisfactionV1 {
    BudgetOrExhaustion,
    ZeroLossOrBudget,
    FirstCompleteWin,
    HpLossAtMost(u32),
    /// Requires an improvement in the compatibility score over persistent
    /// effects already materialized by the combat. This is not run value.
    #[serde(alias = "persistent_run_value_gain")]
    MaterializedPersistentPayoffGain,
    FirstCompleteWinWithoutNewExternalBurden,
    HpLossAtMostWithoutNewExternalBurden(u32),
    PotionFreeHpLossAtMostWithoutNewExternalBurden(u32),
}

impl CombatWitnessSatisfactionV1 {
    pub fn label(self) -> &'static str {
        match self {
            Self::BudgetOrExhaustion => "budget_or_exhaustion",
            Self::ZeroLossOrBudget => "zero_loss_or_budget",
            Self::FirstCompleteWin => "first_complete_win",
            Self::HpLossAtMost(_) => "hp_loss_at_most",
            Self::MaterializedPersistentPayoffGain => "materialized_persistent_payoff_gain",
            Self::FirstCompleteWinWithoutNewExternalBurden => {
                "first_complete_win_without_new_external_burden"
            }
            Self::HpLossAtMostWithoutNewExternalBurden(_) => {
                "hp_loss_at_most_without_new_external_burden"
            }
            Self::PotionFreeHpLossAtMostWithoutNewExternalBurden(_) => {
                "potion_free_hp_loss_at_most_without_new_external_burden"
            }
        }
    }

    pub fn hp_loss_limit(self) -> Option<u32> {
        match self {
            Self::HpLossAtMost(limit)
            | Self::HpLossAtMostWithoutNewExternalBurden(limit)
            | Self::PotionFreeHpLossAtMostWithoutNewExternalBurden(limit) => Some(limit),
            Self::BudgetOrExhaustion
            | Self::ZeroLossOrBudget
            | Self::FirstCompleteWin
            | Self::MaterializedPersistentPayoffGain
            | Self::FirstCompleteWinWithoutNewExternalBurden => None,
        }
    }
}

impl Default for CombatWitnessSatisfactionV1 {
    fn default() -> Self {
        Self::ZeroLossOrBudget
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatWitnessPotionPolicyV1 {
    Never,
    #[serde(alias = "all_legal_potion_actions")]
    All,
    #[serde(alias = "semantic_budgeted_potion_actions")]
    SemanticBudgeted,
}

impl CombatWitnessPotionPolicyV1 {
    pub fn label(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::All => "all_legal_potion_actions",
            Self::SemanticBudgeted => "semantic_budgeted_potion_actions",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatWitnessEngineV1 {
    /// `src/ai/combat_search_v2`: atomic-action best-first fixed-root search.
    AtomicExactV2,
    /// Production resident portfolio over complete-turn local graph and
    /// policy-discrepancy search.
    TurnGraphPortfolioV1,
}

/// Shared owner default for the explicitly opened high-stakes semantic potion
/// lane. This is admission policy, not a property of either search engine.
pub fn high_stakes_semantic_witness_potion_budget_v1(
    combat: &crate::runtime::combat::CombatState,
) -> Option<u32> {
    const BOSS_MAX_POTIONS_USED: u32 = 2;
    const ELITE_MAX_POTIONS_USED: u32 = 1;

    if combat.meta.is_boss_fight {
        Some(BOSS_MAX_POTIONS_USED)
    } else if combat.meta.is_elite_fight {
        Some(ELITE_MAX_POTIONS_USED)
    } else {
        None
    }
}
