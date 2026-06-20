use super::evaluator::evaluate_shop_plan_candidate_v1;
use super::policy::stop_reason;
use super::portfolio::evaluated_shop_portfolio_combo_plans_v1;
use super::types::{
    CompiledShopDecisionV1, ShopCandidateEvidenceV1, ShopCompileModeV1, ShopDecisionContextV1,
    ShopDecisionSourceV1, ShopPlanCandidateRoleV1, ShopPlanCandidateV1, ShopPlanEvaluationV1,
    ShopPlanFrontierV1, ShopPlanKindV1, ShopPlanLaneGroupV1, ShopPlanLaneV1,
    ShopPlanProjectionRoleV1, ShopPlanProjectionV1, ShopPlanSourceV1, ShopPlanStepV1, ShopPlanV1,
    ShopPlanVerdictV1, ShopPolicyClassV1, ShopPolicyConfigV1, ShopPurchaseTargetV1,
};
use crate::ai::strategic::{
    AcquisitionExplorationAxisV1, CandidateDelta, CandidateRole, StrategicDecisionTrace,
};

pub fn compile_shop_decision_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
    mode: ShopCompileModeV1,
) -> CompiledShopDecisionV1 {
    let strategic_trace = crate::ai::strategic::strategic_trace_for_shop(context);
    let mut candidate_plans = enumerate_single_action_plan_candidates_v1(context);
    candidate_plans.push(stop_candidate_plan_v1(stop_reason(context)));
    let mut candidate_plans = candidate_plans
        .into_iter()
        .map(|mut candidate| {
            candidate.evaluation =
                evaluate_shop_plan_candidate_v1(context, config, &strategic_trace, &candidate);
            candidate
        })
        .collect::<Vec<_>>();
    if let ShopCompileModeV1::BranchTopK { max_plans } = mode {
        let portfolio_candidates =
            evaluated_shop_portfolio_combo_plans_v1(context, &candidate_plans, max_plans)
                .into_iter()
                .map(|plan| {
                    let mut candidate = ShopPlanCandidateV1 {
                        plan,
                        role: ShopPlanCandidateRoleV1::PortfolioAlternative,
                        evaluation: ShopPlanEvaluationV1::pending(),
                    };
                    candidate.evaluation = evaluate_shop_plan_candidate_v1(
                        context,
                        config,
                        &strategic_trace,
                        &candidate,
                    );
                    candidate
                });
        candidate_plans.extend(portfolio_candidates);
    }
    let frontier = shop_plan_frontier_v1(&strategic_trace, &candidate_plans);
    let execution_projection =
        select_execution_projection_v1(&strategic_trace, &candidate_plans, mode);
    let selected_plan = execution_projection
        .as_ref()
        .and_then(|projection| {
            plan_with_evaluation_by_id_v1(&candidate_plans, projection.plan_id.as_str())
        })
        .unwrap_or_else(|| {
            stop_candidate_plan_v1("shop compiler produced no candidates".to_string()).plan
        });
    let branch_projection = match mode {
        ShopCompileModeV1::ExecuteOne => Vec::new(),
        ShopCompileModeV1::BranchTopK { max_plans } => {
            branch_exploration_projection_v1(context, &strategic_trace, &candidate_plans, max_plans)
        }
    };
    let alternatives = branch_projection
        .iter()
        .filter(|projection| projection.plan_id != selected_plan.plan_id)
        .filter_map(|projection| {
            plan_with_evaluation_by_id_v1(&candidate_plans, projection.plan_id.as_str())
        })
        .collect();

    CompiledShopDecisionV1 {
        frontier,
        execution_projection,
        branch_projection,
        selected_plan,
        alternatives,
        candidate_plans,
        strategic_trace,
        source: ShopDecisionSourceV1::PlanEvaluationCompiler,
    }
}

pub fn compiled_shop_decision_has_executable_conversion_branch_v1(
    decision: &CompiledShopDecisionV1,
) -> bool {
    decision
        .branch_projection
        .iter()
        .filter_map(|projection| {
            decision
                .candidate_plans
                .iter()
                .find(|candidate| candidate.plan.plan_id == projection.plan_id)
                .map(|candidate| &candidate.plan)
        })
        .any(shop_plan_has_conversion_step_v1)
}

fn select_execution_projection_v1(
    strategic_trace: &StrategicDecisionTrace,
    candidates: &[ShopPlanCandidateV1],
    mode: ShopCompileModeV1,
) -> Option<ShopPlanProjectionV1> {
    candidates
        .iter()
        .filter(|candidate| shop_plan_is_selectable_in_mode_v1(candidate, mode))
        .max_by(|left, right| compare_evaluated_shop_candidates_v1(left, right, mode))
        .map(|candidate| ShopPlanProjectionV1 {
            plan_id: candidate.plan.plan_id.clone(),
            lane: shop_plan_lane_v1(strategic_trace, candidate),
            role: ShopPlanProjectionRoleV1::ExecutionHead,
            reason: "execution projection from evaluated shop frontier".to_string(),
        })
}

fn branch_exploration_projection_v1(
    context: &ShopDecisionContextV1,
    strategic_trace: &StrategicDecisionTrace,
    candidates: &[ShopPlanCandidateV1],
    max_plans: usize,
) -> Vec<ShopPlanProjectionV1> {
    let mut allow_candidates = candidates
        .iter()
        .filter(|candidate| {
            !candidate.plan.steps.is_empty()
                && candidate.evaluation.branch_admission.is_admitted()
                && !plan_has_leave_shop_step_v1(candidate)
        })
        .collect::<Vec<_>>();
    if branch_should_include_leave_with_allowed_candidates_v1(context)
        && allow_candidates
            .iter()
            .any(|candidate| shop_plan_is_context_card_purchase_v1(candidate))
    {
        allow_candidates.extend(candidates.iter().filter(|candidate| {
            candidate.evaluation.verdict == ShopPlanVerdictV1::Stop
                && plan_has_leave_shop_step_v1(candidate)
        }));
    }
    let mut alternatives = if allow_candidates.is_empty() {
        candidates
            .iter()
            .filter(|candidate| {
                !candidate.plan.steps.is_empty()
                    && candidate.evaluation.verdict == ShopPlanVerdictV1::Stop
                    && plan_has_leave_shop_step_v1(candidate)
            })
            .collect::<Vec<_>>()
    } else {
        allow_candidates
    };
    alternatives.sort_by(|left, right| compare_branch_alternative_candidates_v1(left, right));
    let alternatives = select_branch_alternatives_with_effect_coverage_v1(
        strategic_trace,
        &alternatives,
        max_plans,
    );
    alternatives
        .into_iter()
        .map(|candidate| ShopPlanProjectionV1 {
            plan_id: candidate.plan.plan_id.clone(),
            lane: shop_plan_lane_v1(strategic_trace, candidate),
            role: ShopPlanProjectionRoleV1::BranchExplore,
            reason: "branch projection from evaluated shop frontier lane coverage".to_string(),
        })
        .collect()
}

fn shop_plan_frontier_v1(
    strategic_trace: &StrategicDecisionTrace,
    candidates: &[ShopPlanCandidateV1],
) -> ShopPlanFrontierV1 {
    let mut grouped = std::collections::BTreeMap::<ShopPlanLaneV1, Vec<String>>::new();
    for candidate in candidates {
        grouped
            .entry(shop_plan_lane_v1(strategic_trace, candidate))
            .or_default()
            .push(candidate.plan.plan_id.clone());
    }
    ShopPlanFrontierV1 {
        plans: candidates.to_vec(),
        lanes: grouped
            .into_iter()
            .map(|(lane, plan_ids)| ShopPlanLaneGroupV1 { lane, plan_ids })
            .collect(),
    }
}

fn branch_should_include_leave_with_allowed_candidates_v1(context: &ShopDecisionContextV1) -> bool {
    !(context.conversion_pressure && context.affordable_purchase_exists)
}

fn shop_plan_is_selectable_in_mode_v1(
    candidate: &ShopPlanCandidateV1,
    mode: ShopCompileModeV1,
) -> bool {
    if !candidate.evaluation.execution_approval.is_approved() {
        return false;
    }
    match mode {
        ShopCompileModeV1::BranchTopK { .. } => true,
        ShopCompileModeV1::ExecuteOne => !shop_plan_is_context_card_purchase_v1(candidate),
    }
}

fn shop_plan_is_context_card_purchase_v1(candidate: &ShopPlanCandidateV1) -> bool {
    candidate.evaluation.execution_approval.is_approved()
        && candidate.evaluation.tier < 320
        && candidate.plan.steps.len() == 1
        && matches!(
            candidate.plan.steps.first(),
            Some(ShopPlanStepV1::BuyCard { .. })
        )
}

fn select_branch_alternatives_with_effect_coverage_v1<'a>(
    strategic_trace: &StrategicDecisionTrace,
    sorted_candidates: &[&'a ShopPlanCandidateV1],
    max_plans: usize,
) -> Vec<&'a ShopPlanCandidateV1> {
    if max_plans == 0 {
        return Vec::new();
    }

    let mut selected = Vec::new();
    let mut represented = std::collections::BTreeSet::<String>::new();
    for effect_kind in [
        // Keep primitive actions ahead of portfolio probes. Multi-step shop
        // combos are useful coverage probes, but they should not consume the
        // fanout budget before a cleanup or single-purchase branch can appear.
        ShopPlanLaneV1::Purge,
        ShopPlanLaneV1::BuyRelic,
        ShopPlanLaneV1::BuyPotion,
        ShopPlanLaneV1::BuyCardBossAnswer,
        ShopPlanLaneV1::BuyCardMissingCeiling,
        ShopPlanLaneV1::BuyCardFutureSustain,
        ShopPlanLaneV1::BuyCardScalingEngine,
        ShopPlanLaneV1::BuyCombo,
        ShopPlanLaneV1::BuyCardDrawAccess,
        ShopPlanLaneV1::BuyCardExhaustAccess,
        ShopPlanLaneV1::BuyCardDefense,
        ShopPlanLaneV1::BuyCardFrontload,
        ShopPlanLaneV1::BuyCardGeneric,
        ShopPlanLaneV1::Leave,
    ] {
        if selected.len() >= max_plans {
            break;
        }
        let Some(candidate) = sorted_candidates.iter().copied().find(|candidate| {
            shop_plan_lane_v1(strategic_trace, candidate) == effect_kind
                && !represented.contains(&candidate.plan.plan_id)
        }) else {
            continue;
        };
        represented.insert(candidate.plan.plan_id.clone());
        selected.push(candidate);
    }

    for candidate in sorted_candidates.iter().copied() {
        if selected.len() >= max_plans {
            break;
        }
        if represented.insert(candidate.plan.plan_id.clone()) {
            selected.push(candidate);
        }
    }
    selected
}

fn shop_plan_lane_v1(
    strategic_trace: &StrategicDecisionTrace,
    candidate: &ShopPlanCandidateV1,
) -> ShopPlanLaneV1 {
    let plan = &candidate.plan;
    if plan.steps.len() > 1 {
        return ShopPlanLaneV1::BuyCombo;
    }
    match plan.steps.first() {
        Some(ShopPlanStepV1::BuyCard { .. }) => shop_card_buy_coverage_lane_v1(
            plan_delta_from_strategic_trace_v1(strategic_trace, plan),
        ),
        Some(ShopPlanStepV1::BuyRelic { .. }) => ShopPlanLaneV1::BuyRelic,
        Some(ShopPlanStepV1::BuyPotion { .. }) => ShopPlanLaneV1::BuyPotion,
        Some(ShopPlanStepV1::RemoveCard { .. }) => ShopPlanLaneV1::Purge,
        Some(ShopPlanStepV1::LeaveShop) => ShopPlanLaneV1::Leave,
        None => ShopPlanLaneV1::Stop,
    }
}

fn plan_delta_from_strategic_trace_v1<'a>(
    strategic_trace: &'a StrategicDecisionTrace,
    plan: &ShopPlanV1,
) -> Option<&'a CandidateDelta> {
    plan.candidate_ids
        .iter()
        .find_map(|candidate_id| {
            strategic_trace
                .candidate_deltas
                .iter()
                .find(|delta| delta.action.candidate_id() == *candidate_id)
        })
        .or_else(|| {
            plan.steps.iter().find_map(|step| {
                let step_id = shop_plan_step_action_candidate_id_v1(step)?;
                strategic_trace
                    .candidate_deltas
                    .iter()
                    .find(|delta| delta.action.candidate_id() == step_id)
            })
        })
}

fn shop_plan_step_action_candidate_id_v1(step: &ShopPlanStepV1) -> Option<String> {
    match *step {
        ShopPlanStepV1::BuyCard { index, card, .. } => {
            Some(format!("shop:buy_card:{index}:{card:?}"))
        }
        ShopPlanStepV1::BuyRelic { index, relic, .. } => {
            Some(format!("shop:buy_relic:{index}:{relic:?}"))
        }
        ShopPlanStepV1::BuyPotion { index, potion, .. } => {
            Some(format!("shop:buy_potion:{index}:{potion:?}"))
        }
        ShopPlanStepV1::RemoveCard {
            deck_index, card, ..
        } => Some(format!("shop:remove:{deck_index}:{card:?}")),
        ShopPlanStepV1::LeaveShop => Some("shop:leave".to_string()),
    }
}

fn shop_card_buy_coverage_lane_v1(delta: Option<&CandidateDelta>) -> ShopPlanLaneV1 {
    let Some(delta) = delta else {
        return ShopPlanLaneV1::BuyCardGeneric;
    };
    let thesis = delta.acquisition_thesis_profile_v1();
    if thesis.has_axis(AcquisitionExplorationAxisV1::BossAnswer) {
        return ShopPlanLaneV1::BuyCardBossAnswer;
    }
    if thesis.has_axis(AcquisitionExplorationAxisV1::FutureCeiling) {
        return ShopPlanLaneV1::BuyCardMissingCeiling;
    }
    if thesis.has_axis(AcquisitionExplorationAxisV1::SustainOrRecovery) {
        return ShopPlanLaneV1::BuyCardFutureSustain;
    }
    if thesis.has_axis(AcquisitionExplorationAxisV1::ScalingEngine) {
        return ShopPlanLaneV1::BuyCardScalingEngine;
    }
    if thesis.has_axis(AcquisitionExplorationAxisV1::DrawAccess)
        || delta.role == CandidateRole::Lubricant
    {
        return ShopPlanLaneV1::BuyCardDrawAccess;
    }
    if thesis.has_axis(AcquisitionExplorationAxisV1::ExhaustAccess) {
        return ShopPlanLaneV1::BuyCardExhaustAccess;
    }
    if thesis.has_axis(AcquisitionExplorationAxisV1::DefenseCoverage)
        || matches!(delta.role, CandidateRole::DefensivePatch)
    {
        return ShopPlanLaneV1::BuyCardDefense;
    }
    if thesis.has_axis(AcquisitionExplorationAxisV1::TransitionFrontload)
        || matches!(delta.role, CandidateRole::Transition)
    {
        return ShopPlanLaneV1::BuyCardFrontload;
    }
    ShopPlanLaneV1::BuyCardGeneric
}

fn compare_branch_alternative_candidates_v1(
    left: &&ShopPlanCandidateV1,
    right: &&ShopPlanCandidateV1,
) -> std::cmp::Ordering {
    right
        .evaluation
        .tier
        .cmp(&left.evaluation.tier)
        .then_with(|| {
            right
                .evaluation
                .component_score
                .net
                .partial_cmp(&left.evaluation.component_score.net)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| right.evaluation.score.cmp(&left.evaluation.score))
        .then_with(|| left.plan.plan_id.cmp(&right.plan.plan_id))
}

fn candidate_rank_v1(
    candidate: &ShopPlanCandidateV1,
    mode: ShopCompileModeV1,
) -> (i32, i32, i32, i32, i32, i32, std::cmp::Reverse<String>) {
    (
        execution_rank_v1(candidate),
        candidate.evaluation.tier,
        component_net_rank_v1(candidate),
        (candidate.evaluation.confidence * 1000.0).round() as i32,
        candidate.evaluation.score,
        role_rank_v1(candidate, mode),
        std::cmp::Reverse(candidate.plan.plan_id.clone()),
    )
}

fn component_net_rank_v1(candidate: &ShopPlanCandidateV1) -> i32 {
    (candidate.evaluation.component_score.net * 1000.0).round() as i32
}

fn compare_evaluated_shop_candidates_v1(
    left: &ShopPlanCandidateV1,
    right: &ShopPlanCandidateV1,
    mode: ShopCompileModeV1,
) -> std::cmp::Ordering {
    candidate_rank_v1(left, mode).cmp(&candidate_rank_v1(right, mode))
}

fn verdict_rank_v1(verdict: ShopPlanVerdictV1) -> i32 {
    match verdict {
        ShopPlanVerdictV1::Allow => 2,
        ShopPlanVerdictV1::Stop => 1,
        ShopPlanVerdictV1::Block => 0,
    }
}

fn execution_rank_v1(candidate: &ShopPlanCandidateV1) -> i32 {
    if candidate.evaluation.execution_approval.is_approved() {
        return verdict_rank_v1(candidate.evaluation.verdict).max(1);
    }
    0
}

fn role_rank_v1(candidate: &ShopPlanCandidateV1, mode: ShopCompileModeV1) -> i32 {
    if candidate.evaluation.verdict == ShopPlanVerdictV1::Stop
        && matches!(mode, ShopCompileModeV1::BranchTopK { .. })
        && plan_has_leave_shop_step_v1(candidate)
    {
        return 5;
    }
    match (candidate.evaluation.verdict, candidate.role) {
        (ShopPlanVerdictV1::Stop, ShopPlanCandidateRoleV1::StopFallback) => 4,
        (ShopPlanVerdictV1::Stop, _) => 1,
        (_, ShopPlanCandidateRoleV1::SingleAction) => 3,
        (_, ShopPlanCandidateRoleV1::StopFallback) => 2,
        (_, ShopPlanCandidateRoleV1::PortfolioAlternative) => 1,
    }
}

fn plan_has_leave_shop_step_v1(candidate: &ShopPlanCandidateV1) -> bool {
    candidate
        .plan
        .steps
        .iter()
        .any(|step| matches!(step, ShopPlanStepV1::LeaveShop))
}

fn shop_plan_has_conversion_step_v1(plan: &ShopPlanV1) -> bool {
    plan.steps.iter().any(|step| {
        matches!(
            step,
            ShopPlanStepV1::BuyCard { .. }
                | ShopPlanStepV1::BuyRelic { .. }
                | ShopPlanStepV1::BuyPotion { .. }
                | ShopPlanStepV1::RemoveCard { .. }
        )
    })
}

fn enumerate_single_action_plan_candidates_v1(
    context: &ShopDecisionContextV1,
) -> Vec<ShopPlanCandidateV1> {
    context
        .candidates
        .iter()
        .filter_map(|candidate| {
            single_candidate_plan_v1(candidate, ShopPlanSourceV1::CandidateEvidence).map(|plan| {
                ShopPlanCandidateV1 {
                    plan,
                    role: ShopPlanCandidateRoleV1::SingleAction,
                    evaluation: ShopPlanEvaluationV1::pending(),
                }
            })
        })
        .collect()
}

fn plan_with_evaluation_v1(plan: &ShopPlanV1, evaluation: &ShopPlanEvaluationV1) -> ShopPlanV1 {
    let mut plan = plan.clone();
    plan.legacy_priority = evaluation.legacy_priority.or(plan.legacy_priority);
    plan.legacy_confidence = Some(evaluation.confidence);
    if let Some(reason) = evaluation.reasons.first() {
        plan.reason = reason.clone();
    }
    plan
}

fn plan_with_evaluation_by_id_v1(
    candidates: &[ShopPlanCandidateV1],
    plan_id: &str,
) -> Option<ShopPlanV1> {
    candidates
        .iter()
        .find(|candidate| candidate.plan.plan_id == plan_id)
        .map(|candidate| plan_with_evaluation_v1(&candidate.plan, &candidate.evaluation))
}

pub(super) fn single_candidate_plan_v1(
    candidate: &ShopCandidateEvidenceV1,
    source: ShopPlanSourceV1,
) -> Option<ShopPlanV1> {
    let step = match (
        candidate.deck_index,
        candidate.card,
        candidate.purchase_target,
        candidate.class,
    ) {
        (Some(deck_index), Some(card), None, _) => ShopPlanStepV1::RemoveCard {
            deck_index,
            card,
            cost: candidate.gold_cost.unwrap_or_default(),
        },
        (_, _, Some(ShopPurchaseTargetV1::Card { index, card }), _) => ShopPlanStepV1::BuyCard {
            index,
            card,
            cost: candidate.gold_cost.unwrap_or_default(),
        },
        (_, _, Some(ShopPurchaseTargetV1::Relic { index, relic }), _) => ShopPlanStepV1::BuyRelic {
            index,
            relic,
            cost: candidate.gold_cost.unwrap_or_default(),
        },
        (_, _, Some(ShopPurchaseTargetV1::Potion { index, potion }), _) => {
            ShopPlanStepV1::BuyPotion {
                index,
                potion,
                cost: candidate.gold_cost.unwrap_or_default(),
            }
        }
        (_, _, None, ShopPolicyClassV1::Leave) => ShopPlanStepV1::LeaveShop,
        _ => return None,
    };
    let total_gold_spent = match step {
        ShopPlanStepV1::LeaveShop => 0,
        _ => candidate.gold_cost.unwrap_or_default(),
    };
    Some(ShopPlanV1 {
        plan_id: format!("legacy:{}", candidate.candidate_id),
        label: candidate.label.clone(),
        kind: ShopPlanKindV1::Execute,
        steps: vec![step],
        total_gold_spent,
        candidate_ids: vec![candidate.candidate_id.clone()],
        source,
        legacy_priority: candidate.purchase_priority,
        legacy_confidence: None,
        suppressed_count: 0,
        reason: format!("legacy shop plan from {}", candidate.candidate_id),
    })
}

fn stop_candidate_plan_v1(reason: String) -> ShopPlanCandidateV1 {
    ShopPlanCandidateV1 {
        plan: ShopPlanV1 {
            plan_id: "legacy:stop".to_string(),
            label: "stop shop automation".to_string(),
            kind: ShopPlanKindV1::Stop,
            steps: Vec::new(),
            total_gold_spent: 0,
            candidate_ids: Vec::new(),
            source: ShopPlanSourceV1::CandidateEvidence,
            legacy_priority: None,
            legacy_confidence: None,
            suppressed_count: 0,
            reason,
        },
        role: ShopPlanCandidateRoleV1::StopFallback,
        evaluation: ShopPlanEvaluationV1::pending(),
    }
}
