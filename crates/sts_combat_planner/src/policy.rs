use std::sync::Arc;

use sts_core::sim::combat::CombatPosition;
use sts_core::sim::combat_action_surface::CombatSelectionActionFamilyV2;
use sts_core::state::core::ClientInput;

/// One exact choice on a concrete simulator action surface.
///
/// Structured selections remain a family here: their (potentially enormous)
/// member language is scheduled lazily by the selection transaction.
#[derive(Clone, Copy)]
pub enum CombatPolicyChoice<'a> {
    Atomic(&'a ClientInput),
    StructuredSelection(&'a CombatSelectionActionFamilyV2),
}

/// Opaque, lexicographically ordered domain guidance for an exact combat
/// state. The planner does not assign meaning or units to the components; it
/// only uses the rank in an explicitly non-authoritative guide queue. Higher
/// components are preferred.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct CombatStateGuideRank(Vec<i32>);

impl CombatStateGuideRank {
    pub fn new(components: impl Into<Vec<i32>>) -> Self {
        Self(components.into())
    }
}

/// Supplies search guidance only. Returning a small weight never changes
/// legality, and invalid weights are treated as neutral rather than trusted.
pub trait CombatActionPolicy: Send + Sync {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64>;

    /// Optional state guidance for choosing between already materialized exact
    /// turn-boundary states. It never changes legality, duplicate ownership,
    /// or terminal claims. Search retains a policy-only anchor queue even when
    /// this rank is available.
    fn state_guide_rank(&self, _position: &CombatPosition) -> Option<CombatStateGuideRank> {
        None
    }

    /// Independent guide queues over one shared exact-state graph. Keeping
    /// ranks separate avoids inventing a calibration between unlike domain
    /// heuristics (for example, progress and survival).
    fn state_guide_ranks(&self, position: &CombatPosition) -> Vec<CombatStateGuideRank> {
        self.state_guide_rank(position).into_iter().collect()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UniformCombatActionPolicy;

impl CombatActionPolicy for UniformCombatActionPolicy {
    fn weights(&self, _position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        vec![1.0; choices.len()]
    }
}

pub type SharedCombatActionPolicy = Arc<dyn CombatActionPolicy>;

pub(crate) fn uniform_policy() -> SharedCombatActionPolicy {
    Arc::new(UniformCombatActionPolicy)
}

pub(crate) fn normalized_probabilities(
    weights: impl IntoIterator<Item = f64>,
    uniform_exploration_ppm: u32,
) -> Vec<f64> {
    let weights = weights
        .into_iter()
        .map(|weight| {
            if weight.is_finite() && weight > 0.0 {
                weight
            } else {
                1.0
            }
        })
        .collect::<Vec<_>>();
    if weights.is_empty() {
        return Vec::new();
    }
    let total = weights.iter().sum::<f64>();
    let uniform = 1.0 / weights.len() as f64;
    let epsilon = (uniform_exploration_ppm.min(1_000_000) as f64) / 1_000_000.0;
    weights
        .into_iter()
        .map(|weight| {
            ((1.0 - epsilon) * (weight / total) + epsilon * uniform).max(f64::MIN_POSITIVE)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixed_distribution_is_positive_and_normalized() {
        let distribution = normalized_probabilities([100.0, 1.0, 0.0, f64::NAN], 50_000);
        assert!(distribution.iter().all(|probability| *probability > 0.0));
        assert!((distribution.iter().sum::<f64>() - 1.0).abs() < 1.0e-12);
        assert!(distribution[0] > distribution[1]);
    }

    #[test]
    fn full_uniform_mix_ignores_expert_weight() {
        let distribution = normalized_probabilities([100.0, 1.0], 1_000_000);
        assert_eq!(distribution, vec![0.5, 0.5]);
    }
}
