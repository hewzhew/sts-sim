use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2Satisfaction {
    /// Never treat an incumbent as sufficient; stop only at a real work or
    /// frontier boundary.
    BudgetOrExhaustion,
    /// Preserve the old quality-search behavior: only a zero-loss win with no
    /// remaining external-payoff opportunity is self-evidently complete.
    ZeroLossOrBudget,
    /// Stop on the first exact, replayable whole-combat win.
    FirstCompleteWin,
    /// Stop once an exact whole-combat win meets the owner's explicit loss target.
    HpLossAtMost(u32),
    /// Stop on the first exact whole-combat win that did not create a
    /// combat-external burden such as Writhing Mass's curse.
    FirstCompleteWinWithoutNewExternalBurden,
    /// Stop once an exact clean whole-combat win also meets the owner's
    /// explicit loss target.
    HpLossAtMostWithoutNewExternalBurden(u32),
}

impl CombatSearchV2Satisfaction {
    pub fn label(self) -> &'static str {
        match self {
            Self::BudgetOrExhaustion => "budget_or_exhaustion",
            Self::ZeroLossOrBudget => "zero_loss_or_budget",
            Self::FirstCompleteWin => "first_complete_win",
            Self::HpLossAtMost(_) => "hp_loss_at_most",
            Self::FirstCompleteWinWithoutNewExternalBurden => {
                "first_complete_win_without_new_external_burden"
            }
            Self::HpLossAtMostWithoutNewExternalBurden(_) => {
                "hp_loss_at_most_without_new_external_burden"
            }
        }
    }

    pub fn hp_loss_limit(self) -> Option<u32> {
        match self {
            Self::HpLossAtMost(limit) | Self::HpLossAtMostWithoutNewExternalBurden(limit) => {
                Some(limit)
            }
            Self::BudgetOrExhaustion
            | Self::ZeroLossOrBudget
            | Self::FirstCompleteWin
            | Self::FirstCompleteWinWithoutNewExternalBurden => None,
        }
    }
}

impl Default for CombatSearchV2Satisfaction {
    fn default() -> Self {
        Self::ZeroLossOrBudget
    }
}
