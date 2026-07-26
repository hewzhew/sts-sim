//! Shared tactical priors used by both run control and lightweight combat tools.

use std::sync::Arc;
use std::time::Instant;

use sts_combat_legacy::ai::combat_search_v2::oracle_action_policy;
use sts_combat_planner::{
    CombatActionPolicy, CombatGuideLaneId, CombatLookaheadEvaluation, CombatLookaheadEvaluator,
    CombatPolicyChoice, CombatStateGuide,
};
use sts_core::sim::combat::CombatPosition;
use sts_core::sim::combat_action_surface::CombatSelectionActionFamilyV2;
use sts_core::state::core::ClientInput;

#[derive(Clone, Copy, Debug, Default)]
pub struct ExistingCombatKnowledgePolicy;

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
                CombatPolicyChoice::Atomic(input) => Some((*input).clone()),
                CombatPolicyChoice::StructuredSelection(_) => None,
            })
            .collect::<Vec<_>>();
        let mut atomic_weights =
            oracle_action_policy::oracle_atomic_action_policy_weights(position, &atomic_inputs)
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
