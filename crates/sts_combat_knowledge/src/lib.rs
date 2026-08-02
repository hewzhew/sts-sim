//! Shared tactical priors used by both run control and lightweight combat tools.

use std::sync::Arc;
use std::time::Instant;

use sts_combat_legacy::ai::combat_search_v2::oracle_action_policy;
use sts_combat_planner::{
    CombatActionPolicy, CombatGuideLaneId, CombatLookaheadEvaluation, CombatLookaheadEvaluator,
    CombatPolicyChoice, CombatStateGuide, CombatStateGuideRank, SharedCombatActionPolicy,
};
use sts_core::sim::combat::CombatPosition;
use sts_core::sim::combat_action_surface::CombatSelectionActionFamilyV2;
use sts_core::state::core::ClientInput;

#[derive(Clone, Copy, Debug, Default)]
pub struct ExistingCombatKnowledgePolicy;

/// Gives an owner-authorized concrete potion lane one real root challenge.
///
/// The ordinary policy deliberately assigns near-zero mass to potion actions
/// which lack an immediate tactical reason. That remains correct for conserved
/// and mixed search, but it can make a later exact-slot quality lane repeat the
/// no-potion search without meaningfully testing the resource it was opened
/// for. This wrapper changes only action guidance at the unchanged lane root;
/// legality, later timing, witness replay, and owner acceptance stay exact.
pub struct AuthorizedPotionTrialPolicyV1 {
    base: SharedCombatActionPolicy,
    root: CombatPosition,
    allowed_potion_slots: u64,
}

const GUIDE_PROGRESS: CombatGuideLaneId = CombatGuideLaneId::new(1);
const GUIDE_SURVIVAL: CombatGuideLaneId = CombatGuideLaneId::new(2);
const GUIDE_HORIZON: CombatGuideLaneId = CombatGuideLaneId::new(3);
const GUIDE_SETUP: CombatGuideLaneId = CombatGuideLaneId::new(4);
const GUIDE_TURN_DEPTH: CombatGuideLaneId = CombatGuideLaneId::new(5);
const GUIDE_ROLLOUT_LOOKAHEAD: CombatGuideLaneId = CombatGuideLaneId::new(6);

#[derive(Clone, Copy, Debug, Default)]
struct ExistingCombatRolloutLookaheadV1;

pub fn existing_combat_knowledge_policy_v1() -> sts_combat_planner::SharedCombatActionPolicy {
    Arc::new(ExistingCombatKnowledgePolicy)
}

pub fn authorized_potion_trial_policy_v1(
    base: SharedCombatActionPolicy,
    root: CombatPosition,
    allowed_potion_slots: u64,
) -> SharedCombatActionPolicy {
    Arc::new(AuthorizedPotionTrialPolicyV1 {
        base,
        root,
        allowed_potion_slots,
    })
}

pub fn existing_combat_rollout_lookahead_v1() -> sts_combat_planner::SharedCombatLookaheadEvaluator
{
    Arc::new(ExistingCombatRolloutLookaheadV1)
}

impl CombatLookaheadEvaluator for ExistingCombatRolloutLookaheadV1 {
    fn pending_guide(&self, _position: &CombatPosition) -> Option<CombatStateGuide> {
        Some(CombatStateGuide::new(
            GUIDE_ROLLOUT_LOOKAHEAD,
            // Live + no evidence. Evaluated non-winning rollouts remain live
            // heuristic samples; only a simulated win supplies positive
            // existence evidence.
            vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ))
    }

    fn admit_atomic_state(
        &self,
        position: &CombatPosition,
        atomic_expand_service_ordinal: usize,
    ) -> bool {
        let player = &position.combat.entities.player;
        let living_enemy_hp = position
            .combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| monster.current_hp.max(0))
            .sum::<i32>();
        player.current_hp.saturating_mul(3) <= player.max_hp
            || living_enemy_hp <= 45
            || (atomic_expand_service_ordinal > 0
                && atomic_expand_service_ordinal.is_multiple_of(256))
    }

    fn evaluate(
        &self,
        position: &CombatPosition,
        max_work: usize,
        deadline: Option<Instant>,
    ) -> Option<CombatLookaheadEvaluation> {
        if max_work == 0 || deadline.is_some_and(|limit| Instant::now() >= limit) {
            return None;
        }
        let rollout =
            oracle_action_policy::oracle_combat_rollout_guide_v1(position, max_work, deadline);
        Some(CombatLookaheadEvaluation {
            guide: CombatStateGuide::new(GUIDE_ROLLOUT_LOOKAHEAD, rollout.components),
            work: rollout.actions_simulated.max(1),
        })
    }
}

impl CombatActionPolicy for ExistingCombatKnowledgePolicy {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        let atomic_inputs = choices
            .iter()
            .filter_map(|choice| match choice {
                CombatPolicyChoice::Atomic(input) => Some(*input),
                CombatPolicyChoice::StructuredSelection(_) => None,
            })
            .collect::<Vec<_>>();
        let mut atomic_weights =
            oracle_action_policy::oracle_legal_atomic_action_policy_weights_for_refs(
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

    fn structured_selection_member_weights(
        &self,
        position: &CombatPosition,
        _family: &CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        oracle_action_policy::oracle_atomic_action_policy_weights(position, members)
    }

    fn state_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        let guides = oracle_action_policy::oracle_combat_guide_bundle_v1(position);
        vec![
            CombatStateGuide::new(GUIDE_PROGRESS, guides.progress),
            CombatStateGuide::new(GUIDE_SURVIVAL, guides.survival),
            CombatStateGuide::new(GUIDE_HORIZON, guides.horizon),
            CombatStateGuide::new(GUIDE_SETUP, guides.setup),
        ]
    }

    fn turn_generation_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        let guides = oracle_action_policy::oracle_combat_guide_bundle_v1(position);
        vec![
            CombatStateGuide::new(GUIDE_PROGRESS, guides.progress),
            CombatStateGuide::new(GUIDE_SURVIVAL, guides.survival),
            CombatStateGuide::new(GUIDE_TURN_DEPTH, guides.turn_generation),
            CombatStateGuide::new(GUIDE_SETUP, guides.setup),
        ]
    }
}

impl CombatActionPolicy for AuthorizedPotionTrialPolicyV1 {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        let mut weights = self.base.weights(position, choices);
        if position != &self.root || weights.len() != choices.len() {
            return weights;
        }
        let Some(root_trial_weight) = weights
            .iter()
            .copied()
            .filter(|weight| weight.is_finite() && *weight > 0.0)
            .max_by(f64::total_cmp)
        else {
            return weights;
        };
        for (weight, choice) in weights.iter_mut().zip(choices) {
            let CombatPolicyChoice::Atomic(ClientInput::UsePotion { potion_index, .. }) = choice
            else {
                continue;
            };
            let Ok(slot) = u32::try_from(*potion_index) else {
                continue;
            };
            if 1_u64
                .checked_shl(slot)
                .is_some_and(|slot_mask| self.allowed_potion_slots & slot_mask != 0)
            {
                *weight = weight.max(root_trial_weight);
            }
        }
        weights
    }

    fn structured_selection_member_weights(
        &self,
        position: &CombatPosition,
        family: &CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        self.base
            .structured_selection_member_weights(position, family, members)
    }

    fn state_guide_rank(&self, position: &CombatPosition) -> Option<CombatStateGuideRank> {
        self.base.state_guide_rank(position)
    }

    fn state_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.base.state_guides(position)
    }

    fn turn_generation_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.base.turn_generation_guides(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_core::state::core::EngineState;

    struct FixedPolicy;

    impl CombatActionPolicy for FixedPolicy {
        fn weights(
            &self,
            _position: &CombatPosition,
            choices: &[CombatPolicyChoice<'_>],
        ) -> Vec<f64> {
            vec![1.0e-6, 1.0, 0.5][..choices.len()].to_vec()
        }
    }

    #[test]
    fn authorized_potion_trial_only_floors_the_exact_root_and_slot() {
        let root = CombatPosition::new(
            EngineState::CombatPlayerTurn,
            sts_core::test_support::blank_test_combat(),
        );
        let allowed = ClientInput::UsePotion {
            potion_index: 2,
            target: None,
        };
        let disallowed = ClientInput::UsePotion {
            potion_index: 1,
            target: None,
        };
        let end_turn = ClientInput::EndTurn;
        let choices = [
            CombatPolicyChoice::Atomic(&allowed),
            CombatPolicyChoice::Atomic(&end_turn),
            CombatPolicyChoice::Atomic(&disallowed),
        ];
        let policy = authorized_potion_trial_policy_v1(Arc::new(FixedPolicy), root.clone(), 1 << 2);

        assert_eq!(policy.weights(&root, &choices), vec![1.0, 1.0, 0.5]);

        let mut later = root;
        later.combat.entities.player.block = 1;
        assert_eq!(policy.weights(&later, &choices), vec![1.0e-6, 1.0, 0.5]);
    }
}
