use std::sync::Arc;

use std::time::{Duration, Instant};

use sts_combat_planner::{
    CombatActionPolicy, CombatPolicyChoice, CombatPolicyWitnessProposal, CombatStateGuideRank,
    TurnOptionAction,
};

use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};

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
            CombatStateGuideRank::new(
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(
                    position,
                ),
            ),
            CombatStateGuideRank::new(
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(
                    position,
                ),
            ),
        ]
    }

    fn turn_generation_guide_ranks(&self, position: &CombatPosition) -> Vec<CombatStateGuideRank> {
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
            CombatStateGuideRank::new(
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_turn_generation_guide_components(
                    position,
                ),
            ),
            CombatStateGuideRank::new(
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(
                    position,
                ),
            ),
        ]
    }

    fn witness_proposal(
        &self,
        position: &CombatPosition,
        deadline: Option<Instant>,
    ) -> Option<CombatPolicyWitnessProposal> {
        const QUICK_ROLLOUT_BUDGET: Duration = Duration::from_millis(10);
        const MAX_DONOR_SEARCH_BUDGET: Duration = Duration::from_millis(2_000);
        const DONOR_DEADLINE_SLACK: Duration = Duration::from_millis(2_000);
        const POLICY_REPLAY_BUDGET: Duration = Duration::from_millis(500);
        const PLANNER_REPLAY_RESERVE: Duration = Duration::from_millis(500);

        let rollout_deadline = deadline.map(|deadline| {
            deadline.min(
                Instant::now()
                    .checked_add(QUICK_ROLLOUT_BUDGET)
                    .unwrap_or(deadline),
            )
        });
        let rollout = crate::ai::combat_search_v2::oracle_rollout_witness_proposal_v1(
            position,
            80,
            rollout_deadline,
        );
        let player = &position.combat.entities.player;
        // A line that spends a quarter of max HP can still be catastrophic
        // when the run arrives wounded.  Escalate according to the resources
        // actually available at this encounter.
        let material_loss = (player.current_hp / 4).max(6);
        let needs_search = rollout.as_ref().is_none_or(|proposal| {
            player.current_hp.saturating_sub(proposal.final_hp_hint) >= material_loss
        });
        // The policy materializes exact successor hashes, then the witness
        // planner independently replays those actions before accepting a
        // witness. Reserve time for both stages inside the granted quantum;
        // otherwise a donor can consume the whole combat allowance, return a
        // valid line, and leave the authoritative planner no work with which
        // to accept it.
        let proposal_deadline = deadline
            .and_then(|deadline| {
                deadline.checked_sub(
                    DONOR_DEADLINE_SLACK + POLICY_REPLAY_BUDGET + PLANNER_REPLAY_RESERVE,
                )
            })
            .map(|proposal_deadline| {
                proposal_deadline.min(
                    Instant::now()
                        .checked_add(MAX_DONOR_SEARCH_BUDGET)
                        .unwrap_or(proposal_deadline),
                )
            })
            .filter(|proposal_deadline| *proposal_deadline > Instant::now());
        let searched = needs_search
            .then(|| {
                crate::ai::combat_search_v2::oracle_search_witness_proposal_v1(
                    position,
                    800_000,
                    proposal_deadline,
                )
            })
            .flatten();
        let proposal = match (rollout, searched) {
            (Some(rollout), Some(searched)) if searched.final_hp_hint > rollout.final_hp_hint => {
                searched
            }
            (Some(rollout), _) => rollout,
            (None, Some(searched)) => searched,
            (None, None) => return None,
        };
        let replay_deadline = Instant::now()
            .checked_add(POLICY_REPLAY_BUDGET)
            .map_or_else(Instant::now, |candidate| {
                deadline.map_or(candidate, |deadline| candidate.min(deadline))
            });
        let stepper = EngineCombatStepper;
        let mut current = position.clone();
        let mut actions = Vec::with_capacity(proposal.actions.len());
        for input in proposal.actions {
            if Instant::now() >= replay_deadline {
                return None;
            }
            let step = stepper.apply_to_stable(
                &current,
                input.clone(),
                CombatStepLimits {
                    max_engine_steps: 512,
                    deadline: Some(replay_deadline),
                },
            );
            if step.truncated {
                return None;
            }
            actions.push(TurnOptionAction {
                input,
                expected_successor_hash: crate::ai::combat_state_key::combat_exact_state_hash_v1(
                    &step.position.engine,
                    &step.position.combat,
                ),
                engine_steps: step.engine_steps,
            });
            current = step.position;
        }
        // `CombatTerminal` predates typed escape outcomes and therefore also
        // calls a post-Smoke-Bomb reward screen `Win`.  Such a line belongs
        // to run-control's escape channel, never to the exact-victory donor.
        (stepper.terminal(&current) == CombatTerminal::Win && !current.combat.runtime.combat_smoked)
            .then_some(CombatPolicyWitnessProposal {
                actions,
                final_hp_hint: proposal.final_hp_hint,
            })
    }
}
