use std::collections::BTreeMap;

use serde::Serialize;

use crate::sim::combat::CombatPosition;

use super::frontier::SearchNode;
use super::turn_planner::{
    enumerate_turn_plans, TurnPlanBucket, TurnPlanFirstActionSummaryV1, TurnPlanV1,
    TurnPlannerConfigV1,
};
use super::*;

const TURN_PLAN_PROBE_MAX_INNER_NODES: usize = 256;
const TURN_PLAN_PROBE_MAX_END_STATES: usize = 24;
const TURN_PLAN_PROBE_PER_BUCKET_LIMIT: usize = TURN_PLAN_PROBE_MAX_END_STATES;

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TurnPlanProbeRootReport {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub question: &'static str,
    pub behavioral_scope: &'static str,
    pub input_label: Option<String>,
    pub config: CombatSearchV2TurnPlanProbeConfigReport,
    pub initial_context: CombatSearchV2DecisionContext,
    pub root_action_mask: CombatSearchV2TurnPlanProbeActionMaskReport,
    pub enumeration: CombatSearchV2TurnPlanProbeEnumerationReport,
    pub candidates: Vec<CombatSearchV2TurnPlanProbeCandidateReport>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TurnPlanProbeConfigReport {
    pub max_inner_nodes: usize,
    pub max_end_states: usize,
    pub per_bucket_limit: usize,
    pub potion_policy: &'static str,
    pub max_engine_steps_per_action: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TurnPlanProbeEnumerationReport {
    pub planning_policy: &'static str,
    pub plans: usize,
    pub preselection_plans: usize,
    pub preselection_first_action_count: usize,
    pub preselection_bucket_counts: BTreeMap<&'static str, usize>,
    pub selected_bucket_counts: BTreeMap<&'static str, usize>,
    pub nodes_expanded: usize,
    pub nodes_generated: usize,
    pub exact_state_skips: usize,
    pub truncated_children: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TurnPlanProbeActionMaskReport {
    pub data_role: &'static str,
    pub availability: &'static str,
    pub complete_legal_mask: bool,
    pub legal_action_count: usize,
    pub candidate_eligible_action_count: usize,
    pub equivalence_representative_action_count: usize,
    pub preselection_first_action_count: usize,
    pub potion_policy: &'static str,
    pub legal_actions: Vec<CombatSearchV2TurnPlanProbeActionReport>,
    pub candidate_eligible_actions: Vec<CombatSearchV2TurnPlanProbeActionReport>,
    pub equivalence_representative_actions: Vec<CombatSearchV2TurnPlanProbeActionReport>,
    pub preselection_first_actions: Vec<CombatSearchV2TurnPlanProbeActionReport>,
    pub preselection_first_action_summaries:
        Vec<CombatSearchV2TurnPlanProbeFirstActionSummaryReport>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TurnPlanProbeFirstActionSummaryReport {
    pub action: CombatSearchV2TurnPlanProbeActionReport,
    pub plan_count: usize,
    pub bucket_counts: BTreeMap<&'static str, usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TurnPlanProbeActionReport {
    pub action_id: usize,
    pub action_key: String,
    pub action_debug: String,
    pub input: ClientInput,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TurnPlanProbeCandidateReport {
    pub plan_index: usize,
    pub bucket: &'static str,
    pub stop_reason: &'static str,
    pub outcome_class: &'static str,
    pub survival_bucket: &'static str,
    pub progress_bucket: &'static str,
    pub action_count: usize,
    pub first_action_key: Option<String>,
    pub action_keys: Vec<String>,
    pub actions: Vec<CombatSearchV2ActionTrace>,
    pub action_facts: Vec<CombatSearchV2ActionFacts>,
    pub steps: Vec<CombatSearchV2TurnPlanProbeStepReport>,
    pub eval_final_hp: i32,
    pub eval_risk_margin: i32,
    pub eval_enemy_progress: i32,
    pub end_state: CombatSearchV2StateSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2TurnPlanProbeStepReport {
    pub step_index: usize,
    pub action: CombatSearchV2ActionTrace,
    pub action_facts: CombatSearchV2ActionFacts,
    pub exact_state_hash_kind: &'static str,
    pub state_before_exact_state_hash: String,
    pub state_after_exact_state_hash: String,
    pub state_before: CombatSearchV2StateSummary,
    pub state_after: CombatSearchV2StateSummary,
}

#[derive(Clone)]
pub(crate) struct CombatSearchV2TurnPlanProbeCandidate {
    pub(crate) report: CombatSearchV2TurnPlanProbeCandidateReport,
    pub(crate) position: CombatPosition,
}

#[derive(Clone)]
pub(crate) struct CombatSearchV2TurnPlanProbeEnumeration {
    pub(crate) report: CombatSearchV2TurnPlanProbeRootReport,
    pub(crate) candidates: Vec<CombatSearchV2TurnPlanProbeCandidate>,
}

pub(crate) fn enumerate_combat_search_v2_turn_plan_probe_candidates(
    engine: &EngineState,
    combat: &CombatState,
    config: &CombatSearchV2Config,
) -> CombatSearchV2TurnPlanProbeEnumeration {
    let root = SearchNode {
        engine: engine.clone(),
        combat: combat.clone(),
        actions: Vec::new(),
        turn_prefix: TurnPrefixState::default(),
        initial_hp: combat.entities.player.current_hp,
        potions_used: 0,
        potions_discarded: 0,
        cards_played: 0,
        potion_tactical_priority: 0,
        last_turn_branch_priority: 0,
        action_prior_score: None,
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
    };
    let turn_config = TurnPlannerConfigV1 {
        max_inner_nodes: config
            .turn_plan_probe_max_inner_nodes
            .unwrap_or(TURN_PLAN_PROBE_MAX_INNER_NODES),
        max_end_states: config
            .turn_plan_probe_max_end_states
            .unwrap_or(TURN_PLAN_PROBE_MAX_END_STATES),
        per_bucket_limit: config
            .turn_plan_probe_per_bucket_limit
            .unwrap_or(TURN_PLAN_PROBE_PER_BUCKET_LIMIT),
        potion_policy: config.potion_policy,
        max_engine_steps_per_action: config.max_engine_steps_per_action,
        turn_plan_prior: config.turn_plan_prior.clone(),
    };
    let enumeration = enumerate_turn_plans(&root, &EngineCombatStepper, &turn_config, None);
    let position = CombatPosition::new(engine.clone(), combat.clone());
    let legal_action_choices = EngineCombatStepper.legal_action_choices(&position);
    let root_action_mask = root_action_mask_report(
        engine,
        combat,
        turn_config.potion_policy,
        legal_action_choices,
        &enumeration.preselection_first_actions,
        &enumeration.preselection_first_action_summaries,
    );
    let candidates = enumeration
        .plans
        .iter()
        .enumerate()
        .map(|(index, plan)| CombatSearchV2TurnPlanProbeCandidate {
            report: CombatSearchV2TurnPlanProbeCandidateReport {
                plan_index: index,
                bucket: plan.bucket.label(),
                stop_reason: plan.stop_reason.label(),
                outcome_class: plan.eval.outcome_class().label(),
                survival_bucket: plan.eval.survival_bucket().label(),
                progress_bucket: plan.eval.progress_bucket().label(),
                action_count: plan.actions.len(),
                first_action_key: plan.actions.first().map(|action| action.action_key.clone()),
                action_keys: plan
                    .actions
                    .iter()
                    .map(|action| action.action_key.clone())
                    .collect(),
                actions: plan.actions.clone(),
                action_facts: plan.action_facts.clone(),
                steps: turn_plan_step_reports(plan),
                eval_final_hp: plan.eval.final_hp(),
                eval_risk_margin: plan.eval.risk_margin(),
                eval_enemy_progress: plan.eval.enemy_progress(),
                end_state: summarize_state(&plan.end_node.engine, &plan.end_node.combat),
            },
            position: CombatPosition::new(
                plan.end_node.engine.clone(),
                plan.end_node.combat.clone(),
            ),
        })
        .collect::<Vec<_>>();
    let root_report = CombatSearchV2TurnPlanProbeRootReport {
        schema_name: "CombatSearchV2TurnPlanProbeRootReport",
        schema_version: 1,
        question: "which_exact_same_turn_plan_should_receive_followup_search_budget",
        behavioral_scope: "diagnostic_only_no_prune_no_policy_change_no_artifact_promotion",
        input_label: config.input_label.clone(),
        config: CombatSearchV2TurnPlanProbeConfigReport {
            max_inner_nodes: turn_config.max_inner_nodes,
            max_end_states: turn_config.max_end_states,
            per_bucket_limit: turn_config.per_bucket_limit,
            potion_policy: turn_config.potion_policy.label(),
            max_engine_steps_per_action: turn_config.max_engine_steps_per_action,
        },
        initial_context: CombatSearchV2DecisionContext {
            state: summarize_state(engine, combat),
            phase_profile: combat_search_phase_profile_report(combat_search_phase_profile(
                engine, combat,
            )),
            frontier_value: combat_search_frontier_value_report(&root),
        },
        root_action_mask,
        enumeration: CombatSearchV2TurnPlanProbeEnumerationReport {
            planning_policy: "turn_plan_v1_root_only_bounded_exact_step_enumeration",
            plans: enumeration.plans.len(),
            preselection_plans: enumeration.preselection_plan_count,
            preselection_first_action_count: enumeration.preselection_first_actions.len(),
            preselection_bucket_counts: bucket_count_report(
                &enumeration.preselection_bucket_counts,
            ),
            selected_bucket_counts: selected_bucket_count_report(&enumeration.plans),
            nodes_expanded: enumeration.nodes_expanded,
            nodes_generated: enumeration.nodes_generated,
            exact_state_skips: enumeration.exact_state_skips,
            truncated_children: enumeration.truncated_children,
        },
        candidates: candidates
            .iter()
            .map(|candidate| candidate.report.clone())
            .collect(),
        notes: vec![
            "turn-plan probes are exact same-turn candidates ending at a stable boundary",
            "plan enumeration preserves bucket diversity; plan_index is not an optimality claim",
            "followup child search is owned by eval labs, not by this root probe helper",
        ],
    };
    CombatSearchV2TurnPlanProbeEnumeration {
        report: root_report,
        candidates,
    }
}

fn bucket_count_report(counts: &BTreeMap<TurnPlanBucket, usize>) -> BTreeMap<&'static str, usize> {
    counts
        .iter()
        .map(|(bucket, count)| (bucket.label(), *count))
        .collect()
}

fn selected_bucket_count_report(plans: &[TurnPlanV1]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::<TurnPlanBucket, usize>::new();
    for plan in plans {
        *counts.entry(plan.bucket).or_default() += 1;
    }
    bucket_count_report(&counts)
}

fn root_action_mask_report(
    engine: &EngineState,
    combat: &CombatState,
    potion_policy: CombatSearchV2PotionPolicy,
    legal_actions: Vec<CombatActionChoice>,
    preselection_first_actions: &[CombatSearchV2ActionTrace],
    preselection_first_action_summaries: &[TurnPlanFirstActionSummaryV1],
) -> CombatSearchV2TurnPlanProbeActionMaskReport {
    let candidate_eligible = filtered_legal_actions(legal_actions.clone(), potion_policy, combat);
    let equivalence = compress_equivalent_actions(engine, combat, candidate_eligible.clone());
    CombatSearchV2TurnPlanProbeActionMaskReport {
        data_role: "ObservedExact",
        availability: "RootOnly",
        complete_legal_mask: true,
        legal_action_count: legal_actions.len(),
        candidate_eligible_action_count: candidate_eligible.len(),
        equivalence_representative_action_count: equivalence.choices.len(),
        preselection_first_action_count: preselection_first_actions.len(),
        potion_policy: potion_policy.label(),
        legal_actions: action_mask_entries(legal_actions),
        candidate_eligible_actions: action_mask_entries(candidate_eligible),
        equivalence_representative_actions: indexed_action_mask_entries(equivalence.choices),
        preselection_first_actions: action_trace_mask_entries(preselection_first_actions),
        preselection_first_action_summaries: first_action_summary_entries(
            preselection_first_action_summaries,
        ),
        notes: vec![
            "legal_actions is the complete root legal action list from the combat stepper",
            "candidate_eligible_actions applies the current combat search potion policy before turn-plan enumeration",
            "equivalence_representative_actions applies root action equivalence compression before turn-plan enumeration",
            "preselection_first_actions are first actions present before bucket selection truncates turn-plan candidates",
        ],
    }
}

fn first_action_summary_entries(
    summaries: &[TurnPlanFirstActionSummaryV1],
) -> Vec<CombatSearchV2TurnPlanProbeFirstActionSummaryReport> {
    summaries
        .iter()
        .map(
            |summary| CombatSearchV2TurnPlanProbeFirstActionSummaryReport {
                action: action_trace_mask_entry(&summary.action),
                plan_count: summary.plan_count,
                bucket_counts: bucket_count_report(&summary.bucket_counts),
            },
        )
        .collect()
}

fn action_mask_entries(
    actions: Vec<CombatActionChoice>,
) -> Vec<CombatSearchV2TurnPlanProbeActionReport> {
    actions
        .into_iter()
        .enumerate()
        .map(
            |(action_id, action)| CombatSearchV2TurnPlanProbeActionReport {
                action_id,
                action_key: action.action_key,
                action_debug: action.action_debug,
                input: action.input,
            },
        )
        .collect()
}

fn indexed_action_mask_entries(
    actions: Vec<IndexedActionChoice>,
) -> Vec<CombatSearchV2TurnPlanProbeActionReport> {
    actions
        .into_iter()
        .map(|action| CombatSearchV2TurnPlanProbeActionReport {
            action_id: action.original_action_id,
            action_key: action.choice.action_key,
            action_debug: action.choice.action_debug,
            input: action.choice.input,
        })
        .collect()
}

fn action_trace_mask_entries(
    actions: &[CombatSearchV2ActionTrace],
) -> Vec<CombatSearchV2TurnPlanProbeActionReport> {
    actions.iter().map(action_trace_mask_entry).collect()
}

fn action_trace_mask_entry(
    action: &CombatSearchV2ActionTrace,
) -> CombatSearchV2TurnPlanProbeActionReport {
    CombatSearchV2TurnPlanProbeActionReport {
        action_id: action.action_id,
        action_key: action.action_key.clone(),
        action_debug: action.action_debug.clone(),
        input: action.input.clone(),
    }
}

fn turn_plan_step_reports(
    plan: &super::turn_planner::TurnPlanV1,
) -> Vec<CombatSearchV2TurnPlanProbeStepReport> {
    plan.actions
        .iter()
        .zip(plan.action_facts.iter())
        .zip(plan.step_states.iter())
        .enumerate()
        .map(|(step_index, ((action, action_facts), state))| {
            CombatSearchV2TurnPlanProbeStepReport {
                step_index,
                action: action.clone(),
                action_facts: action_facts.clone(),
                exact_state_hash_kind: "combat_exact_state_hash_v1",
                state_before_exact_state_hash: state.before_exact_state_hash.clone(),
                state_after_exact_state_hash: state.after_exact_state_hash.clone(),
                state_before: state.before.clone(),
                state_after: state.after.clone(),
            }
        })
        .collect()
}
