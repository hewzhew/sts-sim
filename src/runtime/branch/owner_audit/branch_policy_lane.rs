use serde::{Deserialize, Serialize};
use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;
use sts_simulator::runtime::branch::RunTrajectoryPolicyLaneV1;

const MAX_CHALLENGER_IDENTITIES: u8 = 2;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum BranchPolicyLane {
    Baseline { issued_challengers: u8 },
    Challenger { policy: ChallengerPolicyState },
}

impl Default for BranchPolicyLane {
    fn default() -> Self {
        Self::Baseline {
            issued_challengers: 0,
        }
    }
}

impl BranchPolicyLane {
    pub(super) fn challenger(policy: ChallengerPolicyState) -> Self {
        Self::Challenger { policy }
    }

    pub(super) fn issue_challenger_id(&mut self) -> Option<u8> {
        let Self::Baseline { issued_challengers } = self else {
            return None;
        };
        if *issued_challengers >= MAX_CHALLENGER_IDENTITIES {
            return None;
        }
        *issued_challengers += 1;
        Some(*issued_challengers)
    }

    pub(super) fn challenger_policy(&self) -> Option<&ChallengerPolicyState> {
        match self {
            Self::Challenger { policy } => Some(policy),
            Self::Baseline { .. } => None,
        }
    }

    pub(super) fn label(&self) -> String {
        match self {
            Self::Baseline { .. } => "baseline".to_string(),
            Self::Challenger { policy } => format!("challenger-{}", policy.lane_id),
        }
    }

    pub(super) fn trajectory_lane(&self) -> RunTrajectoryPolicyLaneV1 {
        match self {
            Self::Baseline { .. } => RunTrajectoryPolicyLaneV1::Baseline,
            Self::Challenger { policy } => RunTrajectoryPolicyLaneV1::Challenger {
                lane_id: policy.lane_id,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;

    #[test]
    fn baseline_issues_at_most_two_challenger_identities() {
        let mut lane = BranchPolicyLane::default();

        assert_eq!(lane.issue_challenger_id(), Some(1));
        assert_eq!(lane.issue_challenger_id(), Some(2));
        assert_eq!(lane.issue_challenger_id(), None);
    }

    #[test]
    fn lane_identity_round_trips_through_json() {
        let lane = BranchPolicyLane::challenger(ChallengerPolicyState::new(2));

        let json = serde_json::to_string(&lane).expect("lane should serialize");
        let restored: BranchPolicyLane =
            serde_json::from_str(&json).expect("lane should deserialize");

        assert_eq!(restored, lane);
        assert_eq!(restored.label(), "challenger-2");
    }
}
