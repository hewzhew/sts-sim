use std::sync::Arc;

use sts_core::ai::combat_state_key::CombatExactStateKey;
use sts_core::state::core::ClientInput;

use crate::types::TurnOptionGeneratorConfig;

use super::{
    CombatDecisionRoot, SharedCombatActionPolicy, TurnOptionAction, TurnOptionGeneratorSession,
};

/// Exact search identity augmented by caller-owned finite resource use.
///
/// `None` preserves ordinary simulator-state transposition when no potion
/// contract exists. Under a finite contract, equal simulator states reached
/// with different spent allowances remain distinct.
#[derive(Clone, Eq, Hash, PartialEq)]
pub(super) struct ConstrainedExactStateKey {
    exact: Arc<CombatExactStateKey>,
    potion_expenditures: Option<u32>,
}

impl ConstrainedExactStateKey {
    pub(super) fn new(
        exact: Arc<CombatExactStateKey>,
        finite_limit: Option<u32>,
        potion_expenditures: u32,
    ) -> Self {
        Self {
            exact,
            potion_expenditures: finite_limit.map(|_| potion_expenditures),
        }
    }
}

pub(super) fn actions_potion_expenditures(actions: &[TurnOptionAction]) -> u32 {
    actions
        .iter()
        .filter(|action| {
            matches!(
                action.input,
                ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
            )
        })
        .count()
        .try_into()
        .unwrap_or(u32::MAX)
}

pub(super) fn turn_generator_for_potion_budget(
    root: CombatDecisionRoot,
    generator_config: TurnOptionGeneratorConfig,
    policy: SharedCombatActionPolicy,
    max_potions_used: Option<u32>,
    already_spent: u32,
) -> TurnOptionGeneratorSession {
    let remaining = max_potions_used.map(|limit| limit.saturating_sub(already_spent));
    TurnOptionGeneratorSession::with_policy_and_potion_limit(
        root,
        generator_config,
        policy,
        remaining,
    )
}
