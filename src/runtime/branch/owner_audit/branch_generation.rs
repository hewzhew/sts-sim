use std::collections::VecDeque;
use std::path::PathBuf;

use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;

use super::owner_model::OwnerChoice;
use super::policy_expansion_plan::{plan_policy_expansions, PolicyExpansion};
use super::run_capsule::RunCapsule;
use super::run_deadline::RunDeadline;
use super::run_slice_result::ArtifactWriteSummary;
use super::{branch_generation_step, branch_scheduler, trace, Args, Branch};

type BranchWork = (Branch, bool, Vec<OwnerChoice>);

pub(super) struct PreparedGeneration {
    work: Vec<BranchWork>,
    policy_expansions: Vec<Vec<PolicyExpansion>>,
    expanded_masks: Vec<Vec<bool>>,
    pub(super) total_expanded: usize,
}

pub(super) enum GenerationAdvance {
    ObjectiveCompleted {
        branch: Branch,
        artifacts: ArtifactWriteSummary,
    },
    Advanced {
        next: VecDeque<Branch>,
        generation_result: Option<(usize, Branch)>,
        artifacts: ArtifactWriteSummary,
    },
}

impl PreparedGeneration {
    pub(super) fn into_frontier(self) -> VecDeque<Branch> {
        self.work.into_iter().map(|(branch, _, _)| branch).collect()
    }
}

pub(super) fn prepare_generation(
    frontier: &mut VecDeque<Branch>,
    args: Args,
    generation: usize,
    deadline: RunDeadline,
) -> PreparedGeneration {
    let mut work = Vec::new();
    while let Some(branch) = frontier.pop_front() {
        work.push(branch_scheduler::prepare_branch_work(
            branch, args, generation, deadline,
        ));
    }
    let (policy_expansions, expanded_masks) = policy_expansion_plans(&work, args.max_branches);
    let total_expanded = expanded_masks
        .iter()
        .flatten()
        .filter(|expanded| **expanded)
        .count();
    PreparedGeneration {
        work,
        policy_expansions,
        expanded_masks,
        total_expanded,
    }
}

fn policy_expansion_plans(
    work: &[BranchWork],
    max_branches: usize,
) -> (Vec<Vec<PolicyExpansion>>, Vec<Vec<bool>>) {
    let mut remaining = max_branches;
    let mut plans = Vec::with_capacity(work.len());
    let mut masks = Vec::with_capacity(work.len());
    for (branch_index, (branch, expandable, choices)) in work.iter().enumerate() {
        let reserved_after = work[branch_index + 1..]
            .iter()
            .filter(|(_, expandable, _)| *expandable)
            .count();
        let branch_budget = if *expandable {
            remaining.saturating_sub(reserved_after)
        } else {
            0
        };
        let branch_plans = if branch_budget == 0 {
            Vec::new()
        } else {
            let facts =
                DeckPlanSnapshot::from_run_state(&branch.session.run_state).strategic_deficit;
            let checkpoint_ref = format!("branch-{}/step-{}", branch.id, branch.path.len());
            plan_policy_expansions(
                &branch.policy_lane,
                facts,
                choices,
                branch_budget,
                &checkpoint_ref,
            )
        };
        remaining = remaining.saturating_sub(branch_plans.len());
        let mut mask = vec![false; choices.len()];
        for expansion in &branch_plans {
            if let Some(expanded) = mask.get_mut(expansion.choice_index) {
                *expanded = true;
            }
        }
        plans.push(branch_plans);
        masks.push(mask);
    }
    (plans, masks)
}

pub(super) fn advance_generation(
    prepared: PreparedGeneration,
    args: Args,
    child_args: Args,
    generation: usize,
    deadline: RunDeadline,
    next_branch_id: &mut usize,
    trace: &mut Option<trace::TraceWriter>,
    combat_gap_case_dir: Option<&PathBuf>,
    capsule: Option<&RunCapsule>,
    human_output: bool,
) -> Result<GenerationAdvance, String> {
    let mut next = VecDeque::new();
    let mut deferred = VecDeque::new();
    let mut generation_result = None;
    let mut artifacts = ArtifactWriteSummary::default();
    for (((branch, expandable, choices), expanded_mask), policy_expansions) in prepared
        .work
        .into_iter()
        .zip(prepared.expanded_masks)
        .zip(prepared.policy_expansions)
    {
        match branch_generation_step::advance_branch_work(
            branch,
            expandable,
            choices,
            expanded_mask,
            policy_expansions,
            args,
            child_args,
            generation,
            deadline,
            next_branch_id,
            trace,
            combat_gap_case_dir,
            capsule,
            human_output,
        )? {
            branch_generation_step::BranchWorkAdvance::ObjectiveCompleted {
                branch,
                artifacts: step_artifacts,
            } => {
                artifacts.merge(step_artifacts);
                return Ok(GenerationAdvance::ObjectiveCompleted { branch, artifacts });
            }
            branch_generation_step::BranchWorkAdvance::Deferred {
                branch,
                artifacts: step_artifacts,
            } => {
                artifacts.merge(step_artifacts);
                deferred.push_back(branch);
            }
            branch_generation_step::BranchWorkAdvance::GenerationResult {
                branch,
                artifacts: step_artifacts,
            } => {
                artifacts.merge(step_artifacts);
                generation_result = Some((generation, branch));
            }
            branch_generation_step::BranchWorkAdvance::Children {
                children,
                artifacts: step_artifacts,
            } => {
                artifacts.merge(step_artifacts);
                next.extend(children);
            }
        }
    }
    next.append(&mut deferred);
    Ok(GenerationAdvance::Advanced {
        next,
        generation_result,
        artifacts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::strategy::decision_pipeline::{
        CandidateEvaluation, CandidateLane, CandidateLaneAdjudication, DecisionCandidateIr,
        DecisionCandidateKind, ExpansionPlan,
    };
    use sts_simulator::ai::strategy::reward_admission::{
        RewardAdmission, RewardAdmissionClass, RewardAdmissionReason,
    };
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::{
        RunControlCommand, RunControlConfig, RunControlSession,
    };

    use super::super::branch_policy_lane::BranchPolicyLane;
    use super::super::owner_model::{
        ChoiceAnnotation, OwnerCandidateDecision, OwnerChoice, OwnerChoiceExpansion,
    };
    use super::super::{BranchStatus, Owner};

    fn choice(kind: DecisionCandidateKind, lane: CandidateLane) -> OwnerChoice {
        let auto = lane != CandidateLane::Probe;
        let card = match kind {
            DecisionCandidateKind::CardRewardPick { card, .. } => Some(card),
            _ => None,
        };
        OwnerChoice {
            key: None,
            action: RunControlCommand::Noop,
            label: format!("{kind:?}"),
            annotation: ChoiceAnnotation::Candidate(OwnerCandidateDecision {
                evaluation: CandidateEvaluation {
                    candidate: DecisionCandidateIr { kind },
                    lane,
                    adjudication: CandidateLaneAdjudication::uncapped(lane),
                    expansion: if auto {
                        ExpansionPlan::Auto
                    } else {
                        ExpansionPlan::InspectOnly("probe fixture")
                    },
                    scores: Vec::new(),
                },
                admission: Some(RewardAdmission {
                    card,
                    class: RewardAdmissionClass::EngineSeed,
                    reasons: card
                        .map(|_| vec![RewardAdmissionReason::FrontloadDamage])
                        .unwrap_or_default(),
                }),
            }),
            expansion: if auto {
                OwnerChoiceExpansion::AutoAllowed
            } else {
                OwnerChoiceExpansion::InspectOnly("probe fixture")
            },
        }
    }

    fn branch() -> Branch {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.master_deck.clear();
        Branch {
            id: 0,
            parent_id: None,
            path: Vec::new(),
            session,
            status: BranchStatus::Running {
                owner: Owner::CardReward,
                boundary: "test".to_string(),
            },
            policy_lane: BranchPolicyLane::default(),
            combat_portfolio: None,
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            accepted_high_loss_diagnostics: Vec::new(),
        }
    }

    #[test]
    fn one_root_lane_can_plan_baseline_and_one_distinct_challenger() {
        let work = vec![(
            branch(),
            true,
            vec![
                choice(DecisionCandidateKind::CardRewardSkip, CandidateLane::Skip),
                choice(
                    DecisionCandidateKind::CardRewardPick {
                        card: CardId::PommelStrike,
                        upgrades: 0,
                    },
                    CandidateLane::Probe,
                ),
            ],
        )];

        let (plans, masks) = policy_expansion_plans(&work, 3);

        assert_eq!(plans[0].len(), 2);
        assert_eq!(plans[0][0].child_lane.label(), "baseline");
        assert_eq!(plans[0][1].child_lane.label(), "challenger-1");
        assert_eq!(masks[0], vec![true, true]);
    }
}
