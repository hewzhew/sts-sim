mod no_potion;
mod node;

#[cfg(test)]
use super::rollout_pending_choice::RolloutPendingChoiceProgress;
#[cfg(test)]
use super::*;

pub(super) const DEFAULT_ROLLOUT_MAX_EVALUATIONS: usize = 384;
pub(super) const DEFAULT_ROLLOUT_MAX_ACTIONS: usize = 80;

pub(super) use no_potion::{conservative_no_potion_rollout, phase_aware_no_potion_rollout};

#[cfg(test)]
mod tests;
