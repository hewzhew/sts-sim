use super::value::{combat_eval_from_rollout_estimate, CombatEvalV2};
use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CombatSearchRolloutValueV1 {
    pub(super) evaluated: i32,
    pub(super) eval: CombatEvalV2,
}

impl Ord for CombatSearchRolloutValueV1 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.evaluated
            .cmp(&other.evaluated)
            .then_with(|| self.eval.cmp(&other.eval))
    }
}

impl PartialOrd for CombatSearchRolloutValueV1 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn rollout_priority_value(estimate: RolloutNodeEstimate) -> CombatSearchRolloutValueV1 {
    CombatSearchRolloutValueV1 {
        evaluated: i32::from(estimate.evaluated),
        eval: combat_eval_from_rollout_estimate(estimate),
    }
}

#[cfg(test)]
mod tests;
