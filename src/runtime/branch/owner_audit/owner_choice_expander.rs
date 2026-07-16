use sts_simulator::eval::run_control::{
    capture_planner_boundary_ticket_v1, PlannerBoundaryCaptureSegmentV1, RunProgressJournalV1,
    RunProgressOutcome, RunProgressStepV1,
};

use super::accepted_high_loss_diagnostic::extend_unique_diagnostics;
use super::branch_path::{
    BranchPathCandidateSnapshot, BranchPathPolicySelectionSnapshot, BranchPathState,
    BranchPathStep, ChoiceAnnotationSnapshot,
};
use super::owner_model::OwnerChoice;
use super::policy_expansion_plan::PolicyExpansion;
use super::run_deadline::RunDeadline;
use super::{decision_delta, runner, Args, Branch, BranchStatus};

pub(super) fn expand_registered_owner(
    branch: &Branch,
    args: Args,
    deadline: RunDeadline,
    choices: &[OwnerChoice],
    policy_expansions: &[PolicyExpansion],
    next_branch_id: &mut usize,
) -> Vec<Branch> {
    let mut children = Vec::new();
    let comparison_search_start = branch
        .comparison_search_start
        .or_else(|| (policy_expansions.len() > 1).then_some(branch.combat_search_history.len()));
    for expansion in policy_expansions.iter().cloned() {
        let choice_index = expansion.choice_index;
        let Some(choice) = choices.get(choice_index).cloned() else {
            continue;
        };
        let policy_selection =
            BranchPathPolicySelectionSnapshot::from_evidence(&expansion.selection_evidence);
        let policy_lane_label = expansion.child_lane.label();
        let mut session = branch.session.clone();
        let capture_ticket = capture_planner_boundary_ticket_v1(&session);
        let (advance, decision_delta) = match capture_ticket {
            Ok(capture_ticket) => {
                match session.apply_owner_candidate(&choice.candidate_id, choice.action.clone()) {
                    Ok(outcome) => {
                        let planner_prefix = capture_ticket
                            .map(|ticket| ticket.finish_for_progress(&outcome.progress_steps))
                            .unwrap_or_default();
                        let delta = decision_delta::decision_delta(
                            &branch.session.run_state,
                            &session.run_state,
                        );
                        let advance = match owner_candidate_journal(&outcome) {
                            Ok(prefix) => prepend_progress_evidence(
                                prefix,
                                planner_prefix,
                                runner::advance_to_owner_or_gap(&mut session, args, deadline),
                            ),
                            Err(error) => failed_advance_with_capture(error, planner_prefix),
                        };
                        (advance, delta)
                    }
                    Err(err) => (
                        failed_advance_with_capture(
                            err,
                            capture_ticket
                                .map(|ticket| ticket.finish_failed())
                                .unwrap_or_default(),
                        ),
                        None,
                    ),
                }
            }
            Err(error) => (failed_advance(error), None),
        };
        let combat_search = advance.combat_search;
        let mut combat_search_history = branch.combat_search_history.clone();
        combat_search_history.extend(combat_search.clone());
        let mut accepted_high_loss_diagnostics = branch.accepted_high_loss_diagnostics.clone();
        extend_unique_diagnostics(
            &mut accepted_high_loss_diagnostics,
            advance.accepted_high_loss_diagnostics,
        );
        let mut path = branch.path.clone();
        path.push(BranchPathStep {
            policy_lane: policy_lane_label,
            policy_selection: Some(policy_selection),
            candidate_id: Some(choice.candidate_id.clone()),
            key: choice.key,
            action_debug: format!("{:?}", choice.action),
            label: choice.label,
            annotation: ChoiceAnnotationSnapshot::from_annotation(&choice.annotation),
            state_before: Some(BranchPathState::from_branch(branch)),
            decision_delta,
            candidate_pool: BranchPathCandidateSnapshot::from_choices(choices, choice_index),
            shop_boss_preview_candidates:
                super::branch_path::BranchPathShopBossPreviewSnapshot::from_choices(choices),
        });
        let id = *next_branch_id;
        *next_branch_id += 1;
        children.push(Branch {
            id,
            parent_id: Some(branch.id),
            path,
            session,
            status: advance.status,
            policy_lane: expansion.child_lane,
            combat_portfolio: advance.combat_portfolio,
            recent_progress_journal: advance.progress_journal,
            recent_planner_capture: advance.planner_capture,
            combat_search,
            combat_search_history,
            comparison_search_start,
            accepted_high_loss_diagnostics,
        });
    }
    children
}

fn owner_candidate_journal(outcome: &RunProgressOutcome) -> Result<RunProgressJournalV1, String> {
    match outcome.progress_steps.as_slice() {
        [step @ RunProgressStepV1::Decision(_)] => {
            RunProgressJournalV1::from_committed_steps(vec![step.clone()])
        }
        _ => Err("owner candidate did not produce exactly one decision transaction".to_string()),
    }
}

fn prepend_progress_evidence(
    mut prefix: RunProgressJournalV1,
    mut planner_prefix: PlannerBoundaryCaptureSegmentV1,
    mut advance: runner::AdvanceResult,
) -> runner::AdvanceResult {
    if let Err(error) = prefix.append(advance.progress_journal) {
        advance.status = BranchStatus::ApplyFailed(error);
    }
    advance.progress_journal = prefix;
    planner_prefix.append(advance.planner_capture);
    advance.planner_capture = planner_prefix;
    advance
}

fn failed_advance(error: String) -> runner::AdvanceResult {
    runner::AdvanceResult {
        status: BranchStatus::ApplyFailed(error),
        combat_portfolio: None,
        progress_journal: RunProgressJournalV1::default(),
        planner_capture: PlannerBoundaryCaptureSegmentV1::default(),
        combat_search: Vec::new(),
        accepted_high_loss_diagnostics: Vec::new(),
    }
}

fn failed_advance_with_capture(
    error: String,
    planner_capture: PlannerBoundaryCaptureSegmentV1,
) -> runner::AdvanceResult {
    let mut advance = failed_advance(error);
    advance.planner_capture = planner_capture;
    advance
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;
    use sts_simulator::ai::strategy::decision_pipeline::{
        CandidateEvaluation, CandidateLane, CandidateLaneAdjudication, DecisionCandidateIr,
        DecisionCandidateKind, ExpansionPlan,
    };
    use sts_simulator::eval::run_control::{
        CombatSearchTraceSummary, DecisionCandidateKey, RunDecisionAction,
    };

    use super::super::branch_policy_lane::BranchPolicyLane;
    use super::super::policy_expansion_plan::{PolicyExpansion, PolicyExpansionEvidence};
    use super::super::run_contract::RunObjective;
    use super::super::{BranchStatus, Owner};

    fn candidate_choice(
        kind: DecisionCandidateKind,
        key: DecisionCandidateKey,
        expansion: super::super::owner_model::OwnerChoiceExpansion,
    ) -> OwnerChoice {
        OwnerChoice {
            candidate_id: "test".to_string(),
            key: Some(key),
            action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
            label: format!("{kind:?}"),
            annotation: super::super::owner_model::ChoiceAnnotation::Candidate(
                super::super::owner_model::OwnerCandidateDecision {
                    evaluation: CandidateEvaluation {
                        candidate: DecisionCandidateIr { kind },
                        lane: CandidateLane::Mainline,
                        adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
                        expansion: ExpansionPlan::Auto,
                        scores: Vec::new(),
                    },
                    admission: None,
                },
            ),
            expansion,
        }
    }

    fn sample_args() -> Args {
        Args {
            seed: 1,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 2,
            max_branches: 3,
            auto_ops: 1,
            search_nodes: 1,
            search_ms: 1,
            rescue_search_nodes: 1,
            rescue_search_ms: 1,
            boss_search_nodes: 1,
            boss_search_ms: 1,
            wall_ms: None,
            checkpoint_before_combat_portfolio: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    #[test]
    fn planned_children_keep_distinct_policy_lane_identity() {
        let parent = Branch {
            id: 0,
            parent_id: None,
            path: Vec::new(),
            session: sts_simulator::eval::run_control::RunControlSession::new(
                sts_simulator::eval::run_control::RunControlConfig::default(),
            ),
            status: BranchStatus::Running {
                owner: Owner::CardReward,
                boundary: "test".to_string(),
            },
            policy_lane: BranchPolicyLane::default(),
            combat_portfolio: None,
            recent_progress_journal: Default::default(),
            recent_planner_capture: Default::default(),
            combat_search: Vec::new(),
            combat_search_history: vec![CombatSearchTraceSummary {
                source: "shared-prefix".to_string(),
                coverage_status: "TimeBudgetLimited".to_string(),
                deadline_hit: true,
                ..CombatSearchTraceSummary::default()
            }],
            comparison_search_start: None,
            accepted_high_loss_diagnostics: Vec::new(),
        };
        let choices = vec![
            candidate_choice(
                DecisionCandidateKind::CardRewardSkip,
                DecisionCandidateKey::CardRewardSkip {
                    reward_item_index: 0,
                },
                super::super::owner_model::OwnerChoiceExpansion::AutoAllowed,
            ),
            candidate_choice(
                DecisionCandidateKind::CardRewardSkip,
                DecisionCandidateKey::CardRewardSkip {
                    reward_item_index: 1,
                },
                super::super::owner_model::OwnerChoiceExpansion::AutoAllowed,
            ),
        ];
        let plans = vec![
            PolicyExpansion {
                choice_index: 0,
                child_lane: BranchPolicyLane::default(),
                selection_evidence: PolicyExpansionEvidence::production("branch-0/step-0"),
            },
            PolicyExpansion {
                choice_index: 1,
                child_lane: BranchPolicyLane::challenger(ChallengerPolicyState::new(1)),
                selection_evidence: PolicyExpansionEvidence::production("branch-0/step-0"),
            },
        ];
        let mut next_branch_id = 1;

        let children = expand_registered_owner(
            &parent,
            sample_args(),
            RunDeadline::new(Instant::now(), None),
            &choices,
            &plans,
            &mut next_branch_id,
        );

        assert_eq!(children.len(), 2);
        assert_eq!(children[0].policy_lane.label(), "baseline");
        assert_eq!(children[1].policy_lane.label(), "challenger-1");
        assert_eq!(children[0].comparison_search_start, Some(1));
        assert_eq!(children[1].comparison_search_start, Some(1));
        assert_eq!(children[1].path[0].policy_lane, "challenger-1");
        let selection = serde_json::to_value(
            children[1].path[0]
                .policy_selection
                .as_ref()
                .expect("planned child should retain policy selection evidence"),
        )
        .unwrap();
        assert_eq!(selection["class"], "production");
        assert_eq!(selection["checkpoint_ref"], "branch-0/step-0");
    }

    #[test]
    fn committed_owner_candidate_starts_a_typed_progress_journal() {
        let mut session = sts_simulator::eval::run_control::RunControlSession::new(
            sts_simulator::eval::run_control::RunControlConfig::default(),
        );
        let surface = sts_simulator::eval::run_control::build_decision_surface(&session);
        let candidate = surface
            .view
            .candidates
            .iter()
            .find_map(|candidate| {
                candidate
                    .action
                    .executable_action()
                    .map(|action| (candidate.id.clone(), action))
            })
            .expect("initial boundary should expose one executable candidate");

        let outcome = session
            .apply_owner_candidate(&candidate.0, candidate.1)
            .expect("owner candidate should commit");
        let journal = owner_candidate_journal(&outcome).expect("decision should form a journal");

        assert!(matches!(
            journal.entries(),
            [RunProgressStepV1::Decision(_)]
        ));
    }
}
