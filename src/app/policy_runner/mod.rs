use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::verification::decision_env::{ActionId, PolicyInput};
use crate::verification::neutral_engine_query::{
    BranchEffectGroup, NeutralEngineQueryResult, NeutralEngineQueryService, SearchExecutionContext,
};
use crate::verification::search_policy::{
    CandidateScore, DecisionMode, DeliberationTrace, PolicyDecision, PolicyProposal, SearchBudget,
    SearchEvidence, SearchHint, SearchKind, SearchPlan, SEARCH_AWARE_POLICY_SCHEMA_VERSION,
};

pub const NEUTRAL_POLICY_RUNNER_ID: &str = "neutral_compressed_policy_runner_v0";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NeutralPolicyRunnerConfig {
    pub max_branch_depth: u8,
    pub max_candidates: usize,
    pub require_strict_dominance: bool,
    pub allow_resource_action_selection: bool,
}

impl Default for NeutralPolicyRunnerConfig {
    fn default() -> Self {
        Self {
            max_branch_depth: 1,
            max_candidates: 64,
            require_strict_dominance: true,
            allow_resource_action_selection: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EvaluationTrace {
    pub schema_version: String,
    pub runner_id: String,
    pub expanded_branch_groups: Vec<BranchEffectGroup>,
    pub unexpanded_branch_groups: Vec<BranchEffectGroup>,
    pub candidate_evaluations: Vec<CandidateEvaluation>,
    pub selected_group_id: Option<usize>,
    pub selected_action_id: Option<ActionId>,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CandidateEvaluation {
    pub action_id: ActionId,
    pub group_id: usize,
    pub player_dead: bool,
    pub combat_cleared: bool,
    pub hp_lost: i32,
    pub enemy_hp_removed: i32,
    pub enemies_killed: i32,
    pub energy_left: u8,
    pub truncated: bool,
    pub resource_action: bool,
    pub dominance_eligible: bool,
    pub dominance_score: i32,
}

pub struct NeutralCompressedPolicyRunner {
    pub config: NeutralPolicyRunnerConfig,
    pub query: NeutralEngineQueryService,
}

impl Default for NeutralCompressedPolicyRunner {
    fn default() -> Self {
        Self {
            config: NeutralPolicyRunnerConfig::default(),
            query: NeutralEngineQueryService::default(),
        }
    }
}

impl NeutralCompressedPolicyRunner {
    pub fn deliberate(
        &self,
        input: &PolicyInput,
        context: &SearchExecutionContext,
    ) -> DeliberationTrace {
        let proposal = self.propose(input);
        let search_plan = self.plan_search(input, &proposal);
        let candidate_ids = input
            .candidates
            .iter()
            .take(self.config.max_candidates)
            .map(|candidate| candidate.id)
            .collect::<Vec<_>>();
        let results = self.query.branch_effect_evidence(context, &candidate_ids);
        let groups = self.query.compress_branch_effects(&results);
        let evidence = results
            .iter()
            .enumerate()
            .map(|(idx, result)| result.to_search_evidence(format!("neutral_branch_effect_{idx}")))
            .collect::<Vec<_>>();
        let evaluation = self.evaluate(input, &results, &groups);
        let decision = self.decide(input, &evidence, &evaluation);
        DeliberationTrace::new(
            input,
            proposal,
            with_evaluation_payload(search_plan, &evaluation),
            evidence,
            decision,
        )
    }

    fn propose(&self, input: &PolicyInput) -> PolicyProposal {
        let prior_scores = input
            .candidates
            .iter()
            .enumerate()
            .map(|(rank, candidate)| CandidateScore {
                action_id: candidate.id,
                score: 0.0,
                rank,
                source: "neutral_no_model_uniform_prior".to_string(),
                payload: Value::Null,
            })
            .collect::<Vec<_>>();
        let search_hints = input
            .candidates
            .iter()
            .take(self.config.max_candidates)
            .map(|candidate| SearchHint {
                candidate_id: Some(candidate.id),
                search_kind: SearchKind::NeutralBranchCompression {
                    max_engine_steps: self.query.step_limit.max_engine_steps,
                },
                priority: 1.0,
                reason: "evaluate_candidate_engine_effect".to_string(),
                payload: Value::Null,
            })
            .collect::<Vec<_>>();
        PolicyProposal {
            schema_version: SEARCH_AWARE_POLICY_SCHEMA_VERSION.to_string(),
            decision_id: input.decision_id.clone(),
            policy_id: NEUTRAL_POLICY_RUNNER_ID.to_string(),
            prior_scores,
            uncertainty: Vec::new(),
            risk_flags: Vec::new(),
            search_hints,
            fast_path_allowed: false,
            payload: serde_json::json!({
                "model": "none",
                "role": "neutral_engine_query_search_allocator",
            }),
        }
    }

    fn plan_search(&self, input: &PolicyInput, proposal: &PolicyProposal) -> SearchPlan {
        SearchPlan::from_hints(
            input,
            &proposal.search_hints,
            SearchBudget {
                time_budget_ms: input.time_budget_ms,
                max_requests: proposal.search_hints.len(),
                payload: serde_json::json!({
                    "runner": NEUTRAL_POLICY_RUNNER_ID,
                    "budget_kind": "candidate_neutral_branch_effects",
                }),
            },
            serde_json::json!({
                "runner": NEUTRAL_POLICY_RUNNER_ID,
                "search_service": "NeutralEngineQueryService",
            }),
        )
    }

    fn evaluate(
        &self,
        input: &PolicyInput,
        results: &[NeutralEngineQueryResult],
        groups: &[BranchEffectGroup],
    ) -> EvaluationTrace {
        let candidate_evaluations = results
            .iter()
            .map(|result| {
                let resource_action = input
                    .candidates
                    .iter()
                    .find(|candidate| candidate.id == result.action_id)
                    .is_some_and(is_resource_action);
                let dominance_eligible =
                    !resource_action || self.config.allow_resource_action_selection;
                let group_id = groups
                    .iter()
                    .find(|group| group.action_ids.contains(&result.action_id))
                    .map(|group| group.group_id)
                    .unwrap_or(usize::MAX);
                CandidateEvaluation {
                    action_id: result.action_id,
                    group_id,
                    player_dead: result.branch_effect.player_dead,
                    combat_cleared: result.branch_effect.combat_cleared,
                    hp_lost: result.branch_effect.hp_lost,
                    enemy_hp_removed: result.branch_effect.enemy_hp_removed,
                    enemies_killed: result.branch_effect.enemies_killed,
                    energy_left: result.branch_effect.energy_left,
                    truncated: result.truncated,
                    resource_action,
                    dominance_eligible,
                    dominance_score: dominance_score(result),
                }
            })
            .collect::<Vec<_>>();
        let group_representatives = groups
            .iter()
            .filter_map(|group| {
                candidate_evaluations
                    .iter()
                    .find(|eval| eval.action_id == group.representative_action_id)
            })
            .cloned()
            .collect::<Vec<_>>();
        let selected = select_by_strict_generic_dominance(&group_representatives);
        let selected_group_id = selected.and_then(|action_id| {
            candidate_evaluations
                .iter()
                .find(|eval| eval.action_id == action_id)
                .map(|eval| eval.group_id)
        });
        let (expanded_branch_groups, unexpanded_branch_groups) =
            split_expanded_groups(groups, self.config.max_branch_depth);
        EvaluationTrace {
            schema_version: "neutral_recursive_evaluation_trace_v0".to_string(),
            runner_id: NEUTRAL_POLICY_RUNNER_ID.to_string(),
            expanded_branch_groups,
            unexpanded_branch_groups,
            candidate_evaluations,
            selected_group_id,
            selected_action_id: selected,
            reason: if selected.is_some() {
                "strict_generic_engine_effect_dominance".to_string()
            } else {
                "no_strict_generic_dominance".to_string()
            },
        }
    }

    fn decide(
        &self,
        input: &PolicyInput,
        evidence: &[SearchEvidence],
        evaluation: &EvaluationTrace,
    ) -> PolicyDecision {
        if let Some(action_id) = evaluation.selected_action_id {
            PolicyDecision {
                schema_version: SEARCH_AWARE_POLICY_SCHEMA_VERSION.to_string(),
                decision_id: input.decision_id.clone(),
                policy_id: NEUTRAL_POLICY_RUNNER_ID.to_string(),
                selected_action_id: Some(action_id),
                mode: DecisionMode::NeutralEvidenceResolved,
                confidence: "generic_dominance".to_string(),
                fallback_reason: None,
                evidence_used: evidence
                    .iter()
                    .map(|item| item.evidence_id.clone())
                    .collect(),
                payload: serde_json::to_value(evaluation).unwrap_or_else(|_| Value::Null),
            }
        } else {
            PolicyDecision {
                schema_version: SEARCH_AWARE_POLICY_SCHEMA_VERSION.to_string(),
                decision_id: input.decision_id.clone(),
                policy_id: NEUTRAL_POLICY_RUNNER_ID.to_string(),
                selected_action_id: None,
                mode: DecisionMode::EvidenceInsufficient,
                confidence: "none".to_string(),
                fallback_reason: Some("no_strict_generic_dominance".to_string()),
                evidence_used: evidence
                    .iter()
                    .map(|item| item.evidence_id.clone())
                    .collect(),
                payload: serde_json::to_value(evaluation).unwrap_or_else(|_| Value::Null),
            }
        }
    }
}

fn with_evaluation_payload(mut plan: SearchPlan, evaluation: &EvaluationTrace) -> SearchPlan {
    plan.payload = serde_json::json!({
        "runner": NEUTRAL_POLICY_RUNNER_ID,
        "evaluation_trace": evaluation,
    });
    plan
}

fn split_expanded_groups(
    groups: &[BranchEffectGroup],
    max_branch_depth: u8,
) -> (Vec<BranchEffectGroup>, Vec<BranchEffectGroup>) {
    if max_branch_depth == 0 {
        return (Vec::new(), groups.to_vec());
    }
    (groups.to_vec(), Vec::new())
}

fn dominance_score(result: &NeutralEngineQueryResult) -> i32 {
    let effect = &result.branch_effect;
    if effect.player_dead {
        return -1_000_000;
    }
    let mut score = 0;
    if effect.combat_cleared {
        score += 100_000;
    }
    score += effect.enemies_killed * 10_000;
    score += effect.enemy_hp_removed * 100;
    score -= effect.hp_lost * 500;
    score += i32::from(effect.energy_left);
    if effect.pending_choice_created {
        score -= 10;
    }
    if result.truncated {
        score -= 1_000;
    }
    score
}

fn select_by_strict_generic_dominance(evaluations: &[CandidateEvaluation]) -> Option<ActionId> {
    let mut viable = evaluations
        .iter()
        .filter(|eval| !eval.player_dead && !eval.truncated && eval.dominance_eligible)
        .collect::<Vec<_>>();
    if viable.is_empty() {
        return None;
    }
    viable.sort_by_key(|eval| std::cmp::Reverse(eval.dominance_score));
    let best = viable[0];
    let second = viable.get(1).copied();
    if best.combat_cleared && second.is_none_or(|other| !other.combat_cleared) {
        return Some(best.action_id);
    }
    if best.enemies_killed > 0
        && second.is_none_or(|other| best.enemies_killed > other.enemies_killed)
    {
        return Some(best.action_id);
    }
    if let Some(other) = second {
        let strict_progress = best.enemy_hp_removed >= other.enemy_hp_removed + 5;
        let no_extra_hp_cost = best.hp_lost <= other.hp_lost;
        if strict_progress && no_extra_hp_cost {
            return Some(best.action_id);
        }
        return None;
    }
    Some(best.action_id)
}

fn is_resource_action(
    candidate: &crate::verification::decision_env::PublicActionCandidateView,
) -> bool {
    matches!(
        candidate.action_kind.as_str(),
        "use_potion" | "discard_potion"
    ) || candidate.action_key.contains("/use_potion/")
        || candidate.action_key.contains("/discard_potion/")
}
