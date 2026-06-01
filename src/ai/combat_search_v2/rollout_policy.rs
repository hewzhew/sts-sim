use std::time::Instant;

use crate::sim::combat::CombatStepper;

use super::rollout_probe::choose_by_one_step_probe;
use super::*;

pub(super) const ROLLOUT_ACTION_REASON_CONSERVATIVE_ORDERING_FIRST: &str =
    "conservative_policy_selected_first_semantic_ordered_no_potion_action";
pub(super) const ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_PROBE: &str =
    "conservative_policy_selected_bounded_one_step_probe_no_potion_action";
pub(super) const ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_SURVIVAL_VALUE: &str =
    "conservative_policy_selected_bounded_one_step_survival_value_no_potion_action";
pub(super) const ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_PHASE_VALUE: &str =
    "conservative_policy_selected_bounded_one_step_phase_value_no_potion_action";
pub(super) const ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_ACTION_FACTS_VALUE: &str =
    "conservative_policy_selected_bounded_one_step_action_facts_value_no_potion_action";

pub(super) const CONSERVATIVE_ROLLOUT_PROBE_ACTION_LIMIT: usize = 6;

#[derive(Clone, Debug)]
pub(super) struct RolloutPolicySelection {
    pub(super) choice: IndexedActionChoice,
    pub(super) reason: &'static str,
}

pub(super) fn filtered_rollout_legal_actions(
    policy: CombatSearchV2RolloutPolicy,
    legal: Vec<CombatActionChoice>,
    combat: &CombatState,
) -> Vec<CombatActionChoice> {
    match policy {
        CombatSearchV2RolloutPolicy::Disabled => Vec::new(),
        CombatSearchV2RolloutPolicy::ConservativeNoPotion
        | CombatSearchV2RolloutPolicy::PhaseAwareNoPotion => {
            filtered_legal_actions(legal, CombatSearchV2PotionPolicy::Never, combat)
        }
    }
}

pub(super) fn choose_rollout_action(
    policy: CombatSearchV2RolloutPolicy,
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    engine: &EngineState,
    combat: &CombatState,
    legal: Vec<CombatActionChoice>,
) -> Option<RolloutPolicySelection> {
    match policy {
        CombatSearchV2RolloutPolicy::Disabled => None,
        CombatSearchV2RolloutPolicy::ConservativeNoPotion => choose_conservative_no_potion_action(
            false, node, stepper, config, deadline, engine, combat, legal,
        ),
        CombatSearchV2RolloutPolicy::PhaseAwareNoPotion => choose_conservative_no_potion_action(
            true, node, stepper, config, deadline, engine, combat, legal,
        ),
    }
}

fn choose_conservative_no_potion_action(
    allow_nonterminal_probe_upgrade: bool,
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    engine: &EngineState,
    combat: &CombatState,
    legal: Vec<CombatActionChoice>,
) -> Option<RolloutPolicySelection> {
    let choices = legal
        .into_iter()
        .enumerate()
        .map(|(original_action_id, choice)| IndexedActionChoice {
            original_action_id,
            choice,
        })
        .collect();
    let ordered = order_indexed_action_choices(engine, combat, choices);
    let fallback = ordered.choices.first().cloned()?;
    if ordered.choices.len() == 1 {
        return Some(RolloutPolicySelection {
            choice: fallback,
            reason: ROLLOUT_ACTION_REASON_CONSERVATIVE_ORDERING_FIRST,
        });
    }
    let probed = choose_by_one_step_probe(
        node,
        stepper,
        config,
        deadline,
        &ordered.choices,
        allow_nonterminal_probe_upgrade,
    );
    Some(match probed {
        Some((choice, reason)) => RolloutPolicySelection { choice, reason },
        None => RolloutPolicySelection {
            choice: fallback,
            reason: ROLLOUT_ACTION_REASON_CONSERVATIVE_ORDERING_FIRST,
        },
    })
}

#[cfg(test)]
mod tests;
