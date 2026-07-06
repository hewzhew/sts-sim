use std::collections::BTreeMap;

use crate::sim::combat::CombatPosition;

use super::frontier::SearchNode;
use super::turn_plan_probe_report::*;
use super::turn_planner::{
    enumerate_turn_plans, TurnPlanBucket, TurnPlanCandidateSelectionAuditV1,
    TurnPlanCoverageGroupAuditV1, TurnPlanCoverageKeyV1, TurnPlanCoverageSignatureV1,
    TurnPlanFirstActionSummaryV1, TurnPlanSelectionAuditV1, TurnPlanV1, TurnPlannerConfigV1,
};
use super::*;

const TURN_PLAN_PROBE_MAX_INNER_NODES: usize = 256;
const TURN_PLAN_PROBE_MAX_END_STATES: usize = 24;
const TURN_PLAN_PROBE_PER_BUCKET_LIMIT: usize = TURN_PLAN_PROBE_MAX_END_STATES;

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
    let root = SearchNode::root(engine.clone(), combat.clone());
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
        schema_version: 2,
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
        selection_audit: selection_audit_report(&enumeration.selection_audit),
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

fn selection_audit_report(
    audit: &TurnPlanSelectionAuditV1,
) -> CombatSearchV2TurnPlanProbeSelectionAuditReport {
    CombatSearchV2TurnPlanProbeSelectionAuditReport {
        data_role: "turn_plan_candidate_selection_audit",
        behavioral_effect: "diagnostic_only_no_candidate_reordering_no_budget_change",
        candidates: audit
            .candidates
            .iter()
            .map(candidate_selection_audit_report)
            .collect(),
        coverage_groups: audit
            .coverage_groups
            .iter()
            .map(coverage_group_audit_report)
            .collect(),
    }
}

fn candidate_selection_audit_report(
    candidate: &TurnPlanCandidateSelectionAuditV1,
) -> CombatSearchV2TurnPlanProbeCandidateSelectionAuditReport {
    CombatSearchV2TurnPlanProbeCandidateSelectionAuditReport {
        preselection_rank: candidate.preselection_rank,
        selected_plan_index: candidate.selected_plan_index,
        outcome: candidate.outcome.label(),
        drop_reason: candidate.drop_reason.map(|reason| reason.label()),
        bucket: candidate.bucket.label(),
        action_keys: candidate.action_keys.clone(),
        coverage_key: coverage_key_report(candidate.coverage_key),
        coverage_signature: coverage_signature_report(candidate.coverage_signature),
    }
}

fn coverage_group_audit_report(
    group: &TurnPlanCoverageGroupAuditV1,
) -> CombatSearchV2TurnPlanProbeCoverageGroupAuditReport {
    CombatSearchV2TurnPlanProbeCoverageGroupAuditReport {
        bucket: group.key.bucket.label(),
        coverage_key: coverage_key_report(group.key.coverage),
        preselection_count: group.preselection_count,
        selected_count: group.selected_count,
        bucket_cap_dropped_count: group.bucket_cap_dropped_count,
        max_end_states_dropped_count: group.max_end_states_dropped_count,
    }
}

fn coverage_key_report(key: TurnPlanCoverageKeyV1) -> CombatSearchV2TurnPlanProbeCoverageKeyReport {
    CombatSearchV2TurnPlanProbeCoverageKeyReport {
        damage: key.damage.label(),
        block: key.block.label(),
        debuff: key.debuff.label(),
        setup: key.setup.label(),
        resource: key.resource.label(),
        risk: key.risk.label(),
    }
}

fn coverage_signature_report(
    signature: TurnPlanCoverageSignatureV1,
) -> CombatSearchV2TurnPlanProbeCoverageSignatureReport {
    CombatSearchV2TurnPlanProbeCoverageSignatureReport {
        action_count: signature.action_count,
        cards_played: signature.cards_played,
        attacks_played: signature.attacks_played,
        skills_played: signature.skills_played,
        powers_played: signature.powers_played,
        potions_used: signature.potions_used,
        damage_done: signature.damage_done,
        block_gained_proxy: signature.block_gained_proxy,
        enemy_vulnerable_added: signature.enemy_vulnerable_added,
        enemy_weak_added: signature.enemy_weak_added,
        enemy_strength_down_added: signature.enemy_strength_down_added,
        player_strength_gain: signature.player_strength_gain,
        player_temporary_strength_gain: signature.player_temporary_strength_gain,
        energy_spent_proxy: signature.energy_spent_proxy,
        hand_delta: signature.hand_delta,
        draw_delta: signature.draw_delta,
        discard_delta: signature.discard_delta,
        exhaust_delta: signature.exhaust_delta,
        queued_cards_delta: signature.queued_cards_delta,
        player_hp_lost: signature.player_hp_lost,
        reactive_player_hp_loss: signature.reactive_player_hp_loss,
        reactive_forced_turn_end_actions: signature.reactive_forced_turn_end_actions,
        pending_choice_steps: signature.pending_choice_steps,
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
