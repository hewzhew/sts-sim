use std::sync::Arc;

use sts_combat_planner::{CombatActionPolicy, CombatPolicyChoice, CombatStateGuideRank};

use crate::sim::combat::CombatPosition;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ExistingCombatKnowledgePolicy;

pub fn existing_combat_knowledge_policy_v1() -> sts_combat_planner::SharedCombatActionPolicy {
    Arc::new(ExistingCombatKnowledgePolicy)
}

impl CombatActionPolicy for ExistingCombatKnowledgePolicy {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        let atomic_inputs = choices
            .iter()
            .filter_map(|choice| match choice {
                CombatPolicyChoice::Atomic(input) => Some((*input).clone()),
                CombatPolicyChoice::StructuredSelection(_) => None,
            })
            .collect::<Vec<_>>();
        let mut atomic_weights =
            crate::ai::combat_search_v2::oracle_action_policy::oracle_atomic_action_policy_weights(
                position,
                &atomic_inputs,
            )
            .into_iter();
        choices
            .iter()
            .map(|choice| match choice {
                CombatPolicyChoice::Atomic(_) => atomic_weights.next().unwrap_or(1.0),
                CombatPolicyChoice::StructuredSelection(_) => 1.0,
            })
            .collect()
    }

    fn state_guide_rank(&self, position: &CombatPosition) -> Option<CombatStateGuideRank> {
        Some(CombatStateGuideRank::new(
            crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(
                position,
            ),
        ))
    }

    fn state_guide_ranks(&self, position: &CombatPosition) -> Vec<CombatStateGuideRank> {
        vec![
            CombatStateGuideRank::new(
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(
                    position,
                ),
            ),
            CombatStateGuideRank::new(
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(
                    position,
                ),
            ),
        ]
    }
}
