//! Resolved execution controls for the local turn-graph laboratory command.
//!
//! This module owns how orthogonal CLI controls compose into one executable
//! search profile. It deliberately does not load cases, advance search, write
//! artifacts, or render reports.

use serde::Serialize;
use sts_combat_planner::{
    combat_plan_selection_timing_policy_v1, combat_plan_state_guide_policy_v1, CombatDecisionRoot,
    LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessSession, SharedCombatActionPolicy,
};
use sts_oracle_runtime::eval::run_control::existing_combat_rollout_lookahead_v1;

use super::combat_policy_controls::{anchor_only_policy, root_turn_anchor_only_policy};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum LocalGraphGuideService {
    AnchorOnly,
    RootTurnAnchorThenGuides,
    AnchorAndGuides,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct LocalGraphExecutionProfile {
    guide_service: LocalGraphGuideService,
    rollout_lookahead: bool,
    typed_plan_state_guide: bool,
    plan_selection_timing: bool,
}

impl LocalGraphExecutionProfile {
    pub(super) fn from_controls(
        anchor_only: bool,
        root_turn_anchor_only: bool,
        rollout_lookahead: bool,
        typed_plan_state_guide: bool,
        plan_selection_timing: bool,
    ) -> Result<Self, String> {
        if anchor_only && root_turn_anchor_only {
            return Err("anchor-only conflicts with root-turn-anchor-only".to_owned());
        }
        if rollout_lookahead && (anchor_only || root_turn_anchor_only) {
            return Err(
                "rollout-lookahead requires ordinary guide service, not an anchor-only mode"
                    .to_owned(),
            );
        }
        if anchor_only && typed_plan_state_guide {
            return Err("typed-plan-guide conflicts with anchor-only".to_owned());
        }
        Ok(Self {
            guide_service: if anchor_only {
                LocalGraphGuideService::AnchorOnly
            } else if root_turn_anchor_only {
                LocalGraphGuideService::RootTurnAnchorThenGuides
            } else {
                LocalGraphGuideService::AnchorAndGuides
            },
            rollout_lookahead,
            typed_plan_state_guide,
            plan_selection_timing,
        })
    }

    /// Historical summary retained in the V1 report alongside the complete
    /// typed profile. It describes graph service, not every policy overlay.
    pub(super) fn scheduler_label(self) -> &'static str {
        match (self.guide_service, self.rollout_lookahead) {
            (LocalGraphGuideService::AnchorOnly, _) => "anchor_only",
            (LocalGraphGuideService::RootTurnAnchorThenGuides, _) => "root_turn_anchor_then_guides",
            (LocalGraphGuideService::AnchorAndGuides, true) => {
                "anchor_guides_and_lazy_rollout_lookahead"
            }
            (LocalGraphGuideService::AnchorAndGuides, false) => "anchor_and_guides",
        }
    }

    pub(super) fn prepare_session(
        self,
        root: CombatDecisionRoot,
        root_player_turn: u32,
        config: LocalTurnGraphWitnessConfig,
        base_policy: SharedCombatActionPolicy,
    ) -> LocalTurnGraphWitnessSession {
        let policy = self.decorate_policy(root_player_turn, base_policy);

        if self.rollout_lookahead {
            LocalTurnGraphWitnessSession::with_policy_and_lookahead(
                root,
                config,
                policy,
                existing_combat_rollout_lookahead_v1(),
            )
        } else {
            LocalTurnGraphWitnessSession::with_policy(root, config, policy)
        }
    }

    fn decorate_policy(
        self,
        root_player_turn: u32,
        base_policy: SharedCombatActionPolicy,
    ) -> SharedCombatActionPolicy {
        // Policy overlays are established before guide suppression so an
        // anchor-only service promise remains authoritative for every guide,
        // including plan-owned lanes.
        let policy = if self.typed_plan_state_guide {
            combat_plan_state_guide_policy_v1(base_policy)
        } else {
            base_policy
        };
        let policy = if self.plan_selection_timing {
            combat_plan_selection_timing_policy_v1(policy)
        } else {
            policy
        };
        match self.guide_service {
            LocalGraphGuideService::AnchorOnly => anchor_only_policy(policy),
            LocalGraphGuideService::RootTurnAnchorThenGuides => {
                root_turn_anchor_only_policy(root_player_turn, policy)
            }
            LocalGraphGuideService::AnchorAndGuides => policy,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use sts_combat_planner::{
        CombatActionPolicy, CombatGuideLaneId, CombatPolicyChoice, CombatStateGuide,
    };
    use sts_oracle_runtime::sim::combat::CombatPosition;
    use sts_oracle_runtime::state::core::EngineState;
    use sts_oracle_runtime::test_support::blank_test_combat;

    struct OneGuidePolicy;

    impl CombatActionPolicy for OneGuidePolicy {
        fn weights(
            &self,
            _position: &CombatPosition,
            choices: &[CombatPolicyChoice<'_>],
        ) -> Vec<f64> {
            vec![1.0; choices.len()]
        }

        fn state_guides(&self, _position: &CombatPosition) -> Vec<CombatStateGuide> {
            vec![CombatStateGuide::new(CombatGuideLaneId::new(7), vec![1])]
        }
    }

    #[test]
    fn execution_profile_reports_every_orthogonal_control() {
        let profile = LocalGraphExecutionProfile::from_controls(false, false, true, true, true)
            .expect("valid profile");
        let value = serde_json::to_value(profile).expect("serialize profile");

        assert_eq!(value["guide_service"], "anchor_and_guides");
        assert_eq!(value["rollout_lookahead"], true);
        assert_eq!(value["typed_plan_state_guide"], true);
        assert_eq!(value["plan_selection_timing"], true);
        assert_eq!(
            profile.scheduler_label(),
            "anchor_guides_and_lazy_rollout_lookahead"
        );
    }

    #[test]
    fn invalid_combinations_are_rejected_below_clap() {
        assert!(
            LocalGraphExecutionProfile::from_controls(true, true, false, false, false).is_err()
        );
        assert!(
            LocalGraphExecutionProfile::from_controls(false, true, true, false, false).is_err()
        );
        assert!(
            LocalGraphExecutionProfile::from_controls(true, false, false, true, false).is_err()
        );
    }

    #[test]
    fn root_turn_anchor_suppression_wraps_every_inner_guide() {
        let profile = LocalGraphExecutionProfile::from_controls(false, true, false, true, false)
            .expect("valid root-turn profile");
        let mut position = CombatPosition::new(EngineState::CombatPlayerTurn, blank_test_combat());
        let root_turn = position.combat.turn.turn_count;
        let policy = profile.decorate_policy(root_turn, Arc::new(OneGuidePolicy));

        assert!(policy.state_guides(&position).is_empty());
        position.combat.turn.turn_count += 1;
        assert!(!policy.state_guides(&position).is_empty());
    }
}
