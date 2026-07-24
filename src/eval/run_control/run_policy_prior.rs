use std::collections::{BTreeMap, BTreeSet};

use super::{RunControlSession, RunDecisionAction};

const RANKED_POLICY_UNIFORM_MIX_V1: f64 = 0.05;
const RANKED_POLICY_DECAY_V1: f64 = 0.25;
const NORMALIZED_PROBABILITY_TOLERANCE_V1: f64 = 1.0e-9;

/// One exact legal action presented to a run policy.
///
/// Policies may rank these actions, but they do not create, remove, or mutate
/// them. The exact model remains the sole source of action legality.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunPolicyCandidateV1<'a> {
    pub candidate_id: &'a str,
    pub label: &'a str,
    pub action: &'a RunDecisionAction,
}

/// Positive policy support for one exact legal action.
#[derive(Clone, Debug, PartialEq)]
pub struct RunActionPriorV1 {
    pub candidate_id: String,
    pub probability: f64,
}

/// A complete, normalized policy distribution over one exact decision surface.
///
/// Entry order is the policy rank used by discrepancy search. Every legal
/// candidate must appear exactly once with finite, strictly positive support.
#[derive(Clone, Debug, PartialEq)]
pub struct RunPolicyPriorV1 {
    pub entries: Vec<RunActionPriorV1>,
}

pub type RunPolicyPriorFnV1 =
    for<'a> fn(&RunControlSession, &[RunPolicyCandidateV1<'a>]) -> Result<RunPolicyPriorV1, String>;

impl RunPolicyPriorV1 {
    pub fn validate_for(&self, legal: &[RunPolicyCandidateV1<'_>]) -> Result<(), String> {
        if legal.is_empty() {
            return Err("run policy prior requires at least one legal candidate".to_string());
        }
        if self.entries.len() != legal.len() {
            return Err(format!(
                "run policy prior returned {} entries for {} legal candidates",
                self.entries.len(),
                legal.len()
            ));
        }

        let legal_ids = legal
            .iter()
            .map(|candidate| candidate.candidate_id)
            .collect::<BTreeSet<_>>();
        if legal_ids.len() != legal.len() {
            return Err("exact decision surface contains duplicate candidate ids".to_string());
        }

        let mut prior_ids = BTreeSet::new();
        let mut probability_sum = 0.0;
        for entry in &self.entries {
            if !legal_ids.contains(entry.candidate_id.as_str()) {
                return Err(format!(
                    "run policy prior returned non-legal candidate '{}'",
                    entry.candidate_id
                ));
            }
            if !prior_ids.insert(entry.candidate_id.as_str()) {
                return Err(format!(
                    "run policy prior duplicated candidate '{}'",
                    entry.candidate_id
                ));
            }
            if !entry.probability.is_finite() || entry.probability <= 0.0 {
                return Err(format!(
                    "run policy prior assigned non-positive or non-finite support to '{}'",
                    entry.candidate_id
                ));
            }
            probability_sum += entry.probability;
        }
        if (probability_sum - 1.0).abs() > NORMALIZED_PROBABILITY_TOLERANCE_V1 {
            return Err(format!(
                "run policy prior probabilities sum to {probability_sum}, expected 1"
            ));
        }
        Ok(())
    }
}

/// Converts a preferred candidate order into the current positive-support
/// ranked prior. Unknown and duplicate preferred ids are ignored; every legal
/// candidate is then appended in exact-surface order.
///
/// This helper preserves the existing oracle behavior while legacy owners are
/// migrated. New policies may return their own complete normalized priors.
pub fn positive_ranked_run_policy_prior_v1(
    legal: &[RunPolicyCandidateV1<'_>],
    preferred_candidate_ids: impl IntoIterator<Item = String>,
) -> Result<RunPolicyPriorV1, String> {
    if legal.is_empty() {
        return Err("cannot build a policy prior for an empty decision surface".to_string());
    }

    let mut legal_indices = BTreeMap::new();
    for (index, candidate) in legal.iter().enumerate() {
        if legal_indices
            .insert(candidate.candidate_id, index)
            .is_some()
        {
            return Err(format!(
                "exact decision surface duplicated candidate '{}'",
                candidate.candidate_id
            ));
        }
    }

    let mut ordered_indices = Vec::with_capacity(legal.len());
    let mut selected_indices = BTreeSet::new();
    for candidate_id in preferred_candidate_ids {
        if let Some(index) = legal_indices.get(candidate_id.as_str()).copied() {
            if selected_indices.insert(index) {
                ordered_indices.push(index);
            }
        }
    }
    for index in 0..legal.len() {
        if selected_indices.insert(index) {
            ordered_indices.push(index);
        }
    }

    let raw = (0..legal.len())
        .map(|rank| RANKED_POLICY_DECAY_V1.powi(rank as i32))
        .collect::<Vec<_>>();
    let raw_sum = raw.iter().sum::<f64>();
    let uniform = 1.0 / legal.len() as f64;
    let entries = ordered_indices
        .into_iter()
        .enumerate()
        .map(|(rank, index)| RunActionPriorV1 {
            candidate_id: legal[index].candidate_id.to_string(),
            probability: (1.0 - RANKED_POLICY_UNIFORM_MIX_V1) * raw[rank] / raw_sum
                + RANKED_POLICY_UNIFORM_MIX_V1 * uniform,
        })
        .collect();
    let prior = RunPolicyPriorV1 { entries };
    prior.validate_for(legal)?;
    Ok(prior)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::core::ClientInput;

    fn candidate<'a>(
        candidate_id: &'a str,
        action: &'a RunDecisionAction,
    ) -> RunPolicyCandidateV1<'a> {
        RunPolicyCandidateV1 {
            candidate_id,
            label: candidate_id,
            action,
        }
    }

    #[test]
    fn ranked_prior_keeps_every_legal_candidate_positive() {
        let action = RunDecisionAction::Input(ClientInput::Proceed);
        let legal = vec![
            candidate("first", &action),
            candidate("second", &action),
            candidate("third", &action),
        ];
        let prior = positive_ranked_run_policy_prior_v1(
            &legal,
            ["second".to_string(), "unknown".to_string()],
        )
        .expect("ranked prior");

        assert_eq!(prior.entries[0].candidate_id, "second");
        assert_eq!(prior.entries.len(), legal.len());
        assert!(prior.entries.iter().all(|entry| entry.probability > 0.0));
        assert!(
            (prior
                .entries
                .iter()
                .map(|entry| entry.probability)
                .sum::<f64>()
                - 1.0)
                .abs()
                < NORMALIZED_PROBABILITY_TOLERANCE_V1
        );
    }

    #[test]
    fn prior_validation_rejects_missing_or_zero_support() {
        let action = RunDecisionAction::Input(ClientInput::Proceed);
        let legal = vec![candidate("first", &action), candidate("second", &action)];
        let prior = RunPolicyPriorV1 {
            entries: vec![
                RunActionPriorV1 {
                    candidate_id: "first".to_string(),
                    probability: 1.0,
                },
                RunActionPriorV1 {
                    candidate_id: "second".to_string(),
                    probability: 0.0,
                },
            ],
        };

        assert!(prior.validate_for(&legal).is_err());
    }
}
