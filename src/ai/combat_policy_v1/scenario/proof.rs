//! Bounded public-policy win proofs over exact scenario banks.
//!
//! Actions at one public information set form an OR choice. Every public
//! successor information set produced by a chosen action forms an AND
//! obligation. Depth or work-budget exhaustion is inconclusive and must never
//! be treated as a refutation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::sim::combat::{CombatStepLimits, CombatTerminal};

use super::group::CombatScenarioGroupV1;
use super::portfolio::{
    metric_summary, CombatScenarioActionPortfolioEvaluatorV1, CombatScenarioActionPortfolioMetricV1,
};
use super::step::{step_combat_scenario_group_v1, CombatScenarioStepResultV1};
use super::types::{CombatPolicyInformationSetKeyV1, CombatPublicActionV1};

pub const COMBAT_SCENARIO_BOUNDED_WIN_PROOF_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatScenarioBoundedWinProofLimitsV1 {
    pub max_depth: usize,
    pub max_candidates_per_information_set: usize,
    pub max_information_sets: usize,
    pub max_candidate_evaluations: usize,
    pub max_engine_steps_per_action: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatScenarioBoundedWinProofGapV1 {
    InvalidLimit,
    NoCandidates,
    NoProvenWin,
    NoStrictDominance,
    DepthLimit,
    CandidateCountExceeds,
    InformationSetBudget,
    CandidateEvaluationBudget,
    ActionEvaluationFailed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatScenarioBoundedWinProofSelectionV1 {
    pub schema_version: u32,
    pub information_set: CombatPolicyInformationSetKeyV1,
    pub action: CombatPublicActionV1,
    pub scenario_count: usize,
    pub proof_information_sets: usize,
    pub explored_information_sets: usize,
    pub evaluated_actions: usize,
    pub observed_hp_loss: CombatScenarioActionPortfolioMetricV1,
    pub actions_to_win: CombatScenarioActionPortfolioMetricV1,
    pub potions_used: CombatScenarioActionPortfolioMetricV1,
}

impl CombatScenarioActionPortfolioEvaluatorV1<'_> {
    pub fn prove_bounded_public_win(
        &self,
        limits: CombatScenarioBoundedWinProofLimitsV1,
    ) -> Result<CombatScenarioBoundedWinProofSelectionV1, CombatScenarioBoundedWinProofGapV1> {
        validate_limits(limits)?;
        let mut search = BoundedWinSearch {
            limits,
            explored_information_sets: 0,
            evaluated_actions: 0,
            session: self.session,
        };
        let result = search.prove_group(self.group, limits.max_depth, true);
        match result {
            NodeProof::Proven(plan) => Ok(selection_from_plan(
                self.group,
                plan,
                search.explored_information_sets,
                search.evaluated_actions,
            )),
            NodeProof::Refuted => Err(CombatScenarioBoundedWinProofGapV1::NoProvenWin),
            NodeProof::Inconclusive(gap) => Err(gap),
        }
    }
}

struct BoundedWinSearch<'a> {
    limits: CombatScenarioBoundedWinProofLimitsV1,
    explored_information_sets: usize,
    evaluated_actions: usize,
    session: &'a super::portfolio::CombatScenarioActionPortfolioSessionV1,
}

impl BoundedWinSearch<'_> {
    fn prove_group(
        &mut self,
        group: &CombatScenarioGroupV1,
        depth_remaining: usize,
        retain_steps: bool,
    ) -> NodeProof {
        if depth_remaining == 0 {
            return NodeProof::Inconclusive(CombatScenarioBoundedWinProofGapV1::DepthLimit);
        }
        if self.explored_information_sets >= self.limits.max_information_sets {
            return NodeProof::Inconclusive(
                CombatScenarioBoundedWinProofGapV1::InformationSetBudget,
            );
        }
        self.explored_information_sets = self.explored_information_sets.saturating_add(1);
        self.session.record_proof_information_set();

        let candidates = &group.view().candidates;
        if candidates.is_empty() {
            return NodeProof::Inconclusive(CombatScenarioBoundedWinProofGapV1::NoCandidates);
        }
        if candidates.len() > self.limits.max_candidates_per_information_set {
            return NodeProof::Inconclusive(
                CombatScenarioBoundedWinProofGapV1::CandidateCountExceeds,
            );
        }

        let mut proven = Vec::new();
        let mut first_inconclusive = None;
        for action in candidates {
            if self.evaluated_actions >= self.limits.max_candidate_evaluations {
                return NodeProof::Inconclusive(
                    CombatScenarioBoundedWinProofGapV1::CandidateEvaluationBudget,
                );
            }
            self.evaluated_actions = self.evaluated_actions.saturating_add(1);
            self.session.record_proof_candidate_evaluation();
            match self.prove_action(group, action, depth_remaining, retain_steps) {
                ActionProof::Proven(plan) => proven.push(plan),
                ActionProof::Refuted => {}
                ActionProof::Inconclusive(gap) => {
                    first_inconclusive.get_or_insert(gap);
                }
            }
        }

        if let Some(gap) = first_inconclusive {
            return NodeProof::Inconclusive(gap);
        }
        match proven.len() {
            0 => NodeProof::Refuted,
            1 => NodeProof::Proven(proven.remove(0)),
            _ => select_strictly_dominant_plan(group, proven)
                .map(NodeProof::Proven)
                .unwrap_or(NodeProof::Inconclusive(
                    CombatScenarioBoundedWinProofGapV1::NoStrictDominance,
                )),
        }
    }

    fn prove_action(
        &mut self,
        group: &CombatScenarioGroupV1,
        action: &CombatPublicActionV1,
        depth_remaining: usize,
        retain_step: bool,
    ) -> ActionProof {
        let max_engine_steps = self.limits.max_engine_steps_per_action;
        let stepped = match self.session.take_step(group, action, max_engine_steps) {
            Some(stepped) => stepped,
            None => {
                let stepped = match step_combat_scenario_group_v1(
                    group,
                    action,
                    CombatStepLimits {
                        max_engine_steps,
                        deadline: None,
                    },
                ) {
                    Ok(stepped) => stepped,
                    Err(_) => {
                        return ActionProof::Inconclusive(
                            CombatScenarioBoundedWinProofGapV1::ActionEvaluationFailed,
                        );
                    }
                };
                self.session.record_engine_steps(stepped.view.engine_steps);
                stepped
            }
        };

        let proof = self.prove_stepped_action(action, &stepped, depth_remaining);
        if retain_step {
            self.session
                .put_step(group, action, max_engine_steps, stepped);
        }
        proof
    }

    fn prove_stepped_action(
        &mut self,
        action: &CombatPublicActionV1,
        stepped: &CombatScenarioStepResultV1,
        depth_remaining: usize,
    ) -> ActionProof {
        if stepped.view.loss_count > 0 {
            return ActionProof::Refuted;
        }
        let uses_potion = usize::from(matches!(action, CombatPublicActionV1::UsePotion { .. }));
        let mut outcomes = BTreeMap::new();
        for terminal in &stepped.terminal_outcomes {
            if terminal.terminal != CombatTerminal::Win {
                return ActionProof::Refuted;
            }
            outcomes.insert(
                terminal.scenario_id.clone(),
                ProofOutcome {
                    final_hp: terminal.final_hp,
                    actions: 1,
                    potions_used: uses_potion,
                },
            );
        }

        let mut proof_information_sets = 1usize;
        for next_group in &stepped.next_groups {
            match self.prove_group(next_group, depth_remaining.saturating_sub(1), false) {
                NodeProof::Proven(child) => {
                    proof_information_sets =
                        proof_information_sets.saturating_add(child.proof_information_sets);
                    for (scenario_id, child_outcome) in child.outcomes {
                        if outcomes
                            .insert(
                                scenario_id,
                                ProofOutcome {
                                    final_hp: child_outcome.final_hp,
                                    actions: child_outcome.actions.saturating_add(1),
                                    potions_used: child_outcome
                                        .potions_used
                                        .saturating_add(uses_potion),
                                },
                            )
                            .is_some()
                        {
                            return ActionProof::Inconclusive(
                                CombatScenarioBoundedWinProofGapV1::ActionEvaluationFailed,
                            );
                        }
                    }
                }
                NodeProof::Refuted => return ActionProof::Refuted,
                NodeProof::Inconclusive(gap) => return ActionProof::Inconclusive(gap),
            }
        }

        if outcomes.len() != stepped.view.scenario_count {
            return ActionProof::Inconclusive(
                CombatScenarioBoundedWinProofGapV1::ActionEvaluationFailed,
            );
        }
        ActionProof::Proven(ProofPlan {
            action: action.clone(),
            proof_information_sets,
            outcomes,
        })
    }
}

enum NodeProof {
    Proven(ProofPlan),
    Refuted,
    Inconclusive(CombatScenarioBoundedWinProofGapV1),
}

enum ActionProof {
    Proven(ProofPlan),
    Refuted,
    Inconclusive(CombatScenarioBoundedWinProofGapV1),
}

struct ProofPlan {
    action: CombatPublicActionV1,
    proof_information_sets: usize,
    outcomes: BTreeMap<String, ProofOutcome>,
}

struct ProofOutcome {
    final_hp: i32,
    actions: usize,
    potions_used: usize,
}

struct PlanSummary {
    plan: ProofPlan,
    observed_hp_loss: CombatScenarioActionPortfolioMetricV1,
    actions_to_win: CombatScenarioActionPortfolioMetricV1,
    potions_used: CombatScenarioActionPortfolioMetricV1,
}

fn select_strictly_dominant_plan(
    group: &CombatScenarioGroupV1,
    plans: Vec<ProofPlan>,
) -> Option<ProofPlan> {
    let summaries = plans
        .into_iter()
        .map(|plan| summarize_plan(group, plan))
        .collect::<Vec<_>>();
    let mut selected = None;
    for (index, candidate) in summaries.iter().enumerate() {
        if summaries
            .iter()
            .enumerate()
            .filter(|(other_index, _)| *other_index != index)
            .all(|(_, other)| plan_strictly_dominates(candidate, other))
        {
            if selected.is_some() {
                return None;
            }
            selected = Some(index);
        }
    }
    selected.map(|index| {
        summaries
            .into_iter()
            .nth(index)
            .expect("selected proof plan index exists")
            .plan
    })
}

fn plan_strictly_dominates(candidate: &PlanSummary, other: &PlanSummary) -> bool {
    if candidate.plan.outcomes.len() != other.plan.outcomes.len() {
        return false;
    }
    let mut strictly_better = false;
    for (scenario_id, candidate_outcome) in &candidate.plan.outcomes {
        let Some(other_outcome) = other.plan.outcomes.get(scenario_id) else {
            return false;
        };
        if candidate_outcome.final_hp < other_outcome.final_hp
            || candidate_outcome.actions > other_outcome.actions
            || candidate_outcome.potions_used > other_outcome.potions_used
        {
            return false;
        }
        strictly_better |= candidate_outcome.final_hp > other_outcome.final_hp
            || candidate_outcome.actions < other_outcome.actions
            || candidate_outcome.potions_used < other_outcome.potions_used;
    }
    strictly_better
}

fn selection_from_plan(
    group: &CombatScenarioGroupV1,
    plan: ProofPlan,
    explored_information_sets: usize,
    evaluated_actions: usize,
) -> CombatScenarioBoundedWinProofSelectionV1 {
    let summary = summarize_plan(group, plan);
    CombatScenarioBoundedWinProofSelectionV1 {
        schema_version: COMBAT_SCENARIO_BOUNDED_WIN_PROOF_SCHEMA_VERSION,
        information_set: group.view().key.clone(),
        action: summary.plan.action,
        scenario_count: group.view().scenario_count,
        proof_information_sets: summary.plan.proof_information_sets,
        explored_information_sets,
        evaluated_actions,
        observed_hp_loss: summary.observed_hp_loss,
        actions_to_win: summary.actions_to_win,
        potions_used: summary.potions_used,
    }
}

fn summarize_plan(group: &CombatScenarioGroupV1, plan: ProofPlan) -> PlanSummary {
    let root_hp = group
        .view()
        .observation
        .observation
        .compatibility_public
        .player
        .hp;
    let hp_losses = plan
        .outcomes
        .values()
        .map(|outcome| root_hp.saturating_sub(outcome.final_hp))
        .collect();
    let actions = plan
        .outcomes
        .values()
        .map(|outcome| saturating_i32(outcome.actions))
        .collect();
    let potions = plan
        .outcomes
        .values()
        .map(|outcome| saturating_i32(outcome.potions_used))
        .collect();
    PlanSummary {
        observed_hp_loss: metric_summary(hp_losses),
        actions_to_win: metric_summary(actions),
        potions_used: metric_summary(potions),
        plan,
    }
}

fn validate_limits(
    limits: CombatScenarioBoundedWinProofLimitsV1,
) -> Result<(), CombatScenarioBoundedWinProofGapV1> {
    if [
        limits.max_depth,
        limits.max_candidates_per_information_set,
        limits.max_information_sets,
        limits.max_candidate_evaluations,
        limits.max_engine_steps_per_action,
    ]
    .contains(&0)
    {
        return Err(CombatScenarioBoundedWinProofGapV1::InvalidLimit);
    }
    Ok(())
}

fn saturating_i32(value: usize) -> i32 {
    value.try_into().unwrap_or(i32::MAX)
}
