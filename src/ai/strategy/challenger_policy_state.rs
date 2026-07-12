use serde::{Deserialize, Serialize};

use crate::ai::strategy::candidate_pressure_response::{
    CandidatePressureResponse, StrategyCommitmentKind,
};
use crate::ai::strategy::pressure_assessment::PressureHypothesis;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentRequirement {
    RepeatableSupply,
    Source,
    Payoff,
    Deployability,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentStatus {
    Active,
    Completed,
    Abandoned,
    Expired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentHorizon {
    DecisionBoundaries(u8),
    NextEliteOrBoss,
    CurrentActBoss,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyProgress {
    DecisionBoundary,
    EliteReached,
    BossReached,
    ActAdvanced,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StrategyCommitment {
    pub kind: StrategyCommitmentKind,
    pub status: CommitmentStatus,
    pub requirements: Vec<CommitmentRequirement>,
    pub horizon: CommitmentHorizon,
    pub burden_units: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChallengerPolicyState {
    pub lane_id: u8,
    pub active_pressure: Vec<PressureHypothesis>,
    pub commitments: Vec<StrategyCommitment>,
    pub divergence_count: u16,
    pub last_checkpoint_ref: Option<String>,
}

impl ChallengerPolicyState {
    pub fn new(lane_id: u8) -> Self {
        Self {
            lane_id,
            active_pressure: Vec::new(),
            commitments: Vec::new(),
            divergence_count: 0,
            last_checkpoint_ref: None,
        }
    }

    pub fn record_divergence(
        &mut self,
        checkpoint_ref: impl Into<String>,
        response: &CandidatePressureResponse,
    ) {
        self.divergence_count = self.divergence_count.saturating_add(1);
        self.last_checkpoint_ref = Some(checkpoint_ref.into());
        for kind in &response.opens_commitments {
            if !self.commitments.iter().any(|commitment| {
                commitment.kind == *kind && commitment.status == CommitmentStatus::Active
            }) {
                self.open_commitment(StrategyCommitment {
                    kind: *kind,
                    status: CommitmentStatus::Active,
                    requirements: default_requirements(*kind),
                    horizon: CommitmentHorizon::CurrentActBoss,
                    burden_units: 0,
                });
            }
        }
    }

    pub fn open_commitment(&mut self, commitment: StrategyCommitment) {
        self.commitments.push(commitment);
    }

    pub fn candidate_supports_active_commitment(
        &self,
        response: &CandidatePressureResponse,
    ) -> bool {
        self.commitments.iter().any(|commitment| {
            commitment.status == CommitmentStatus::Active
                && response.supports_commitments.contains(&commitment.kind)
        })
    }

    pub fn observe_requirement(
        &mut self,
        kind: StrategyCommitmentKind,
        requirement: CommitmentRequirement,
    ) {
        if let Some(commitment) = self.commitments.iter_mut().find(|commitment| {
            commitment.kind == kind && commitment.status == CommitmentStatus::Active
        }) {
            commitment.requirements.retain(|item| *item != requirement);
            if commitment.requirements.is_empty() {
                commitment.status = CommitmentStatus::Completed;
            }
        }
    }

    pub fn advance(&mut self, progress: PolicyProgress) {
        for commitment in &mut self.commitments {
            if commitment.status != CommitmentStatus::Active {
                continue;
            }
            let expires = match (&mut commitment.horizon, progress) {
                (
                    CommitmentHorizon::DecisionBoundaries(remaining),
                    PolicyProgress::DecisionBoundary,
                ) => {
                    *remaining = remaining.saturating_sub(1);
                    *remaining == 0
                }
                (
                    CommitmentHorizon::NextEliteOrBoss,
                    PolicyProgress::EliteReached | PolicyProgress::BossReached,
                ) => true,
                (
                    CommitmentHorizon::CurrentActBoss,
                    PolicyProgress::BossReached | PolicyProgress::ActAdvanced,
                ) => true,
                _ => false,
            };
            if expires {
                commitment.status = CommitmentStatus::Expired;
            }
        }
    }
}

fn default_requirements(kind: StrategyCommitmentKind) -> Vec<CommitmentRequirement> {
    match kind {
        StrategyCommitmentKind::ExhaustEngine => vec![CommitmentRequirement::Payoff],
        StrategyCommitmentKind::SelfDamageEngine => {
            vec![CommitmentRequirement::RepeatableSupply]
        }
        StrategyCommitmentKind::StrengthScaling => vec![CommitmentRequirement::Source],
        StrategyCommitmentKind::BlockEngine => vec![CommitmentRequirement::Source],
        StrategyCommitmentKind::UpgradeAccess => vec![CommitmentRequirement::Deployability],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategy::candidate_pressure_response::CandidatePressureResponse;

    #[test]
    fn challenger_remembers_multiple_sequential_divergences() {
        let mut state = ChallengerPolicyState::new(1);
        state.record_divergence("a2f19", &CandidatePressureResponse::default());
        state.record_divergence("a2f23", &CandidatePressureResponse::default());

        assert_eq!(state.divergence_count, 2);
        assert_eq!(state.last_checkpoint_ref.as_deref(), Some("a2f23"));
    }

    #[test]
    fn active_commitment_recognizes_later_support() {
        let mut state = ChallengerPolicyState::new(1);
        state.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::ExhaustEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::Payoff],
            horizon: CommitmentHorizon::DecisionBoundaries(3),
            burden_units: 1,
        });
        let response = CandidatePressureResponse {
            supports_commitments: vec![StrategyCommitmentKind::ExhaustEngine],
            ..CandidatePressureResponse::default()
        };

        assert!(state.candidate_supports_active_commitment(&response));
    }

    #[test]
    fn unsupported_commitment_expires_at_its_decision_horizon() {
        let mut state = ChallengerPolicyState::new(1);
        state.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::SelfDamageEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::RepeatableSupply],
            horizon: CommitmentHorizon::DecisionBoundaries(1),
            burden_units: 1,
        });

        state.advance(PolicyProgress::DecisionBoundary);

        assert_eq!(state.commitments[0].status, CommitmentStatus::Expired);
    }

    #[test]
    fn satisfying_last_requirement_completes_commitment() {
        let mut state = ChallengerPolicyState::new(1);
        state.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::SelfDamageEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::RepeatableSupply],
            horizon: CommitmentHorizon::CurrentActBoss,
            burden_units: 1,
        });

        state.observe_requirement(
            StrategyCommitmentKind::SelfDamageEngine,
            CommitmentRequirement::RepeatableSupply,
        );

        assert_eq!(state.commitments[0].status, CommitmentStatus::Completed);
    }

    #[test]
    fn challenger_policy_state_round_trips_through_json() {
        let mut state = ChallengerPolicyState::new(2);
        state.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::ExhaustEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::Payoff],
            horizon: CommitmentHorizon::CurrentActBoss,
            burden_units: 1,
        });

        let json = serde_json::to_string(&state).expect("policy state should serialize");
        let restored: ChallengerPolicyState =
            serde_json::from_str(&json).expect("policy state should deserialize");

        assert_eq!(restored, state);
    }
}
