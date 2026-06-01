use std::time::Instant;

use crate::sim::combat::CombatStepper;

use super::*;

mod score;
use score::{probe_action_score, probe_upgrade_reason, RolloutActionProbeScore};

pub(super) fn choose_by_one_step_probe(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    ordered: &[IndexedActionChoice],
    allow_nonterminal_upgrade: bool,
) -> Option<(IndexedActionChoice, &'static str)> {
    let fallback_score = probe_action_score(node, stepper, config, deadline, ordered.first()?, 0)?;
    let mut best: Option<(RolloutActionProbeScore, IndexedActionChoice, &'static str)> = None;
    for (ordered_index, choice) in ordered
        .iter()
        .take(super::rollout_policy::CONSERVATIVE_ROLLOUT_PROBE_ACTION_LIMIT)
        .enumerate()
    {
        let Some(score) =
            probe_action_score(node, stepper, config, deadline, choice, ordered_index)
        else {
            continue;
        };
        let Some(reason) = probe_upgrade_reason(score, fallback_score, allow_nonterminal_upgrade)
        else {
            continue;
        };
        let replace = best
            .as_ref()
            .map(|(best_score, _, _)| score > *best_score)
            .unwrap_or(true);
        if replace {
            best = Some((score, choice.clone(), reason));
        }
    }
    let (_, best_choice, reason) = best?;
    Some((best_choice, reason))
}

#[cfg(test)]
#[path = "rollout_probe_tests.rs"]
mod tests;
