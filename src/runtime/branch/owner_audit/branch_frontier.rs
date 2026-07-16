use std::collections::VecDeque;

use sts_simulator::ai::strategy::challenger_signature::{
    ChallengerLaneSnapshot, ChallengerSignature, DeckBurdenBand, DeployabilityBand,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::deck_strategic_deficit::{
    StrategicBurdenLevel, StrategicDeficitLevel,
};

use super::{Branch, BranchStatus, TerminalOutcome};

pub(super) fn retain_frontier(frontier: &mut VecDeque<Branch>, limit: usize) {
    if limit == 0 {
        frontier.clear();
        return;
    }
    let mut baseline = None::<Branch>;
    let mut challengers =
        std::collections::BTreeMap::<(ChallengerSignature, Option<String>), Branch>::new();
    for branch in frontier.drain(..) {
        let Some(signature) = challenger_signature_for_branch(&branch) else {
            let replace = match baseline.as_ref() {
                None => true,
                Some(existing) => stronger_frontier_branch(&branch, existing),
            };
            if replace {
                baseline = Some(branch);
            }
            continue;
        };
        let retention_identity = (signature, challenger_divergence_key(&branch));
        let replace = match challengers.get(&retention_identity) {
            None => true,
            Some(existing) => stronger_frontier_branch(&branch, existing),
        };
        if replace {
            challengers.insert(retention_identity, branch);
        }
    }
    let mut retained = Vec::new();
    if let Some(baseline) = baseline {
        retained.push(baseline);
    }
    let challenger_slots = limit.saturating_sub(retained.len()).min(2);
    let mut distinct_challengers = challengers.into_values().collect::<Vec<_>>();
    distinct_challengers.sort_by(|left, right| {
        frontier_retention_key(right)
            .cmp(&frontier_retention_key(left))
            .then_with(|| left.id.cmp(&right.id))
    });
    retained.extend(distinct_challengers.into_iter().take(challenger_slots));
    *frontier = retained.into();
}

fn challenger_divergence_key(branch: &Branch) -> Option<String> {
    let lane = branch.policy_lane.label();
    branch
        .path
        .iter()
        .find(|step| step.policy_lane == lane)
        .and_then(|step| step.key.as_ref())
        .and_then(|key| serde_json::to_string(key).ok())
}

fn challenger_signature_for_branch(branch: &Branch) -> Option<ChallengerSignature> {
    let policy = branch.policy_lane.challenger_policy()?.clone();
    let plan = DeckPlanSnapshot::from_run_state(&branch.session.run_state);
    let burden = match plan.strategic_deficit.deck_burden {
        StrategicBurdenLevel::Clean => DeckBurdenBand::Clean,
        StrategicBurdenLevel::Watch => DeckBurdenBand::Watch,
        StrategicBurdenLevel::Heavy => DeckBurdenBand::Heavy,
    };
    let deployability = if matches!(
        plan.strategic_deficit.deck_access,
        StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
    ) || matches!(
        plan.strategic_deficit.energy_or_playability,
        StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
    ) {
        DeployabilityBand::Thin
    } else {
        DeployabilityBand::Adequate
    };
    Some(
        ChallengerLaneSnapshot {
            policy,
            burden,
            deployability,
            evidence_rank: 0,
        }
        .signature(),
    )
}

fn stronger_frontier_branch(candidate: &Branch, existing: &Branch) -> bool {
    frontier_retention_key(candidate) > frontier_retention_key(existing)
        || (frontier_retention_key(candidate) == frontier_retention_key(existing)
            && candidate.id < existing.id)
}

fn frontier_retention_key(branch: &Branch) -> (u8, u8, i32, u32, i32) {
    let status = match branch.status {
        BranchStatus::Terminal(TerminalOutcome::Victory) => 4,
        BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. } => 3,
        BranchStatus::CombatGap { .. }
        | BranchStatus::OperationBudgetExhausted { .. }
        | BranchStatus::BudgetGap { .. } => 1,
        BranchStatus::Terminal(TerminalOutcome::Defeat)
        | BranchStatus::AutomationGap { .. }
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => 0,
    };
    let hp = branch.session.run_state.current_hp;
    let max_hp = branch.session.run_state.max_hp.max(1);
    let hp_ratio = (hp.max(0) as u32).saturating_mul(1000) / max_hp as u32;
    (
        status,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        hp_ratio,
        hp,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;
    use sts_simulator::ai::strategy::pressure_assessment::{
        EvidenceConfidence, PressureAxis, PressureCoverage, PressureHypothesis,
    };
    use sts_simulator::content::relics::RelicId;
    use sts_simulator::eval::run_control::DecisionCandidateKey;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

    use super::super::branch_path::{BranchPathStep, ChoiceAnnotationSnapshot};
    use super::super::branch_policy_lane::BranchPolicyLane;
    use super::super::{BranchStatus, Owner};

    fn branch_with_lane(id: usize, policy_lane: BranchPolicyLane, hp: i32) -> Branch {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.current_hp = hp;
        session.run_state.max_hp = 80;
        Branch {
            id,
            parent_id: None,
            path: Vec::new(),
            session,
            status: BranchStatus::Running {
                owner: Owner::CardReward,
                boundary: "test".to_string(),
            },
            policy_lane,
            combat_portfolio: None,
            recent_progress_journal: Default::default(),
            recent_planner_capture: Default::default(),
            trajectory: Default::default(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            comparison_search_start: None,
            accepted_high_loss_diagnostics: Vec::new(),
        }
    }

    fn baseline_branch(hp: i32) -> Branch {
        branch_with_lane(0, BranchPolicyLane::default(), hp)
    }

    fn challenger_branch(lane_id: u8, axis: PressureAxis, hp: i32) -> Branch {
        let mut policy = ChallengerPolicyState::new(lane_id);
        policy.active_pressure.push(PressureHypothesis {
            axis,
            coverage: PressureCoverage::Open,
            confidence: EvidenceConfidence::Low,
            supporting_evidence: Vec::new(),
            contradicting_evidence: Vec::new(),
        });
        branch_with_lane(lane_id as usize, BranchPolicyLane::challenger(policy), hp)
    }

    fn boss_relic_challenger_branch(lane_id: u8, relic: RelicId) -> Branch {
        let mut branch = challenger_branch(lane_id, PressureAxis::ResolutionTempo, 50);
        branch.path.push(BranchPathStep {
            policy_lane: branch.policy_lane.label(),
            policy_selection: None,
            candidate_id: None,
            key: Some(DecisionCandidateKey::BossRelicPick {
                option_index: lane_id as usize,
                relic,
            }),
            action_debug: format!("take {relic:?}"),
            label: format!("Take {relic:?}"),
            annotation: ChoiceAnnotationSnapshot::none(),
            state_before: None,
            decision_delta: None,
            candidate_pool: Vec::new(),
            shop_boss_preview_candidates: Vec::new(),
        });
        branch
    }

    #[test]
    fn lower_hp_baseline_is_not_dropped_for_healthier_challenger() {
        let mut frontier = VecDeque::from([
            challenger_branch(1, PressureAxis::DelayCapacity, 70),
            baseline_branch(20),
        ]);

        retain_frontier(&mut frontier, 1);

        assert_eq!(frontier.len(), 1);
        assert_eq!(frontier[0].policy_lane.label(), "baseline");
    }

    #[test]
    fn equivalent_challengers_merge_but_distinct_axes_survive() {
        let mut frontier = VecDeque::from([
            baseline_branch(50),
            challenger_branch(1, PressureAxis::ResolutionTempo, 30),
            challenger_branch(2, PressureAxis::ResolutionTempo, 45),
            challenger_branch(3, PressureAxis::DelayCapacity, 25),
        ]);

        retain_frontier(&mut frontier, 3);

        assert_eq!(frontier.len(), 3);
        assert_eq!(frontier[0].policy_lane.label(), "baseline");
        assert!(frontier
            .iter()
            .any(|branch| branch.session.run_state.current_hp == 45));
        assert!(frontier.iter().any(|branch| {
            branch
                .policy_lane
                .challenger_policy()
                .is_some_and(|policy| {
                    policy
                        .active_pressure
                        .iter()
                        .any(|item| item.axis == PressureAxis::DelayCapacity)
                })
        }));
    }

    #[test]
    fn distinct_boss_relic_experiment_arms_survive_semantic_deduplication() {
        let mut frontier = VecDeque::from([
            baseline_branch(50),
            boss_relic_challenger_branch(1, RelicId::CoffeeDripper),
            boss_relic_challenger_branch(2, RelicId::PhilosopherStone),
        ]);

        retain_frontier(&mut frontier, 3);

        assert_eq!(frontier.len(), 3);
        assert!(frontier.iter().any(|branch| {
            branch.path.last().is_some_and(|step| {
                matches!(
                    step.key,
                    Some(DecisionCandidateKey::BossRelicPick {
                        relic: RelicId::CoffeeDripper,
                        ..
                    })
                )
            })
        }));
        assert!(frontier.iter().any(|branch| {
            branch.path.last().is_some_and(|step| {
                matches!(
                    step.key,
                    Some(DecisionCandidateKey::BossRelicPick {
                        relic: RelicId::PhilosopherStone,
                        ..
                    })
                )
            })
        }));
    }
}
