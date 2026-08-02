use std::hash::{Hash, Hasher};
use std::sync::Arc;

use rustc_hash::FxHasher;
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
#[derive(Clone, Eq, PartialEq)]
pub(super) struct ConstrainedExactStateKey {
    /// Process-local bucket hash computed once when this search identity is
    /// constructed. Equality still compares the complete typed key below, so
    /// collisions cannot merge simulator states.
    structural_hash: u64,
    exact: Arc<CombatExactStateKey>,
    potion_expenditures: Option<u32>,
}

impl Hash for ConstrainedExactStateKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.structural_hash.hash(state);
        self.potion_expenditures.hash(state);
    }
}

impl ConstrainedExactStateKey {
    pub(super) fn new(
        exact: Arc<CombatExactStateKey>,
        finite_limit: Option<u32>,
        potion_expenditures: u32,
    ) -> Self {
        let mut hasher = FxHasher::default();
        exact.hash(&mut hasher);
        Self {
            structural_hash: hasher.finish(),
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

pub(super) fn policy_line_input_respects_potion_contract(
    input: &ClientInput,
    generator_config: TurnOptionGeneratorConfig,
    max_potions_used: Option<u32>,
    already_spent: u32,
) -> bool {
    let is_expenditure = matches!(
        input,
        ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
    );
    if matches!(input, ClientInput::DiscardPotion(_)) && !generator_config.allow_potion_discard {
        return false;
    }
    !is_expenditure
        || generator_config.allow_potion_expenditure
            && crate::witness::potion_input_uses_allowed_slot(
                input,
                generator_config.allowed_potion_slots,
            )
            && max_potions_used.is_none_or(|limit| already_spent < limit)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_line_uses_the_same_finite_potion_contract_as_generation() {
        let use_slot_one = ClientInput::UsePotion {
            potion_index: 1,
            target: None,
        };
        let config = TurnOptionGeneratorConfig {
            allowed_potion_slots: Some(1_u64 << 1),
            ..TurnOptionGeneratorConfig::default()
        };

        assert!(policy_line_input_respects_potion_contract(
            &use_slot_one,
            config,
            Some(1),
            0
        ));
        assert!(!policy_line_input_respects_potion_contract(
            &use_slot_one,
            config,
            Some(1),
            1
        ));
        assert!(!policy_line_input_respects_potion_contract(
            &ClientInput::DiscardPotion(0),
            config,
            Some(1),
            0
        ));
        let semantic = TurnOptionGeneratorConfig {
            allow_potion_discard: false,
            allowed_potion_slots: Some(1_u64 << 1),
            ..TurnOptionGeneratorConfig::default()
        };
        assert!(!policy_line_input_respects_potion_contract(
            &ClientInput::DiscardPotion(1),
            semantic,
            Some(1),
            0
        ));
        assert!(policy_line_input_respects_potion_contract(
            &ClientInput::EndTurn,
            config,
            Some(0),
            0
        ));
    }
}
