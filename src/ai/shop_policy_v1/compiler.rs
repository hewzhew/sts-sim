use super::evaluator::evaluate_shop_plan_candidate_v1;
use super::policy::stop_reason;
use super::portfolio::legacy_shop_portfolio_plans_v1;
use super::types::{
    CompiledShopDecisionV1, ShopCandidateEvidenceV1, ShopCompileModeV1, ShopDecisionContextV1,
    ShopDecisionSourceV1, ShopPlanCandidateRoleV1, ShopPlanCandidateV1, ShopPlanEvaluationV1,
    ShopPlanKindV1, ShopPlanSourceV1, ShopPlanStepV1, ShopPlanV1, ShopPlanVerdictV1,
    ShopPolicyClassV1, ShopPolicyConfigV1, ShopPurchaseTargetV1,
};

pub fn compile_shop_decision_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
    mode: ShopCompileModeV1,
) -> CompiledShopDecisionV1 {
    let strategic_trace = crate::ai::strategic::strategic_trace_for_shop(context);
    let mut candidate_plans = enumerate_single_action_plan_candidates_v1(context);
    candidate_plans.push(stop_candidate_plan_v1(stop_reason(context)));
    if let ShopCompileModeV1::BranchTopK { max_plans } = mode {
        candidate_plans.extend(
            legacy_shop_portfolio_plans_v1(context, max_plans)
                .into_iter()
                .map(|plan| ShopPlanCandidateV1 {
                    plan,
                    role: ShopPlanCandidateRoleV1::PortfolioAlternative,
                    evaluation: ShopPlanEvaluationV1::pending(),
                }),
        );
    }
    let candidate_plans = candidate_plans
        .into_iter()
        .map(|mut candidate| {
            candidate.evaluation =
                evaluate_shop_plan_candidate_v1(context, config, &strategic_trace, &candidate);
            candidate
        })
        .collect::<Vec<_>>();
    let selected_plan = select_evaluated_shop_plan_v1(&candidate_plans, mode);
    let alternatives = match mode {
        ShopCompileModeV1::ExecuteOne => Vec::new(),
        ShopCompileModeV1::BranchTopK { max_plans } => {
            evaluated_branch_alternatives_v1(&candidate_plans, max_plans)
        }
    };

    CompiledShopDecisionV1 {
        selected_plan,
        alternatives,
        candidate_plans,
        strategic_trace,
        source: ShopDecisionSourceV1::PlanEvaluationCompiler,
    }
}

fn select_evaluated_shop_plan_v1(
    candidates: &[ShopPlanCandidateV1],
    mode: ShopCompileModeV1,
) -> ShopPlanV1 {
    candidates
        .iter()
        .max_by(|left, right| compare_evaluated_shop_candidates_v1(left, right, mode))
        .map(|candidate| plan_with_evaluation_v1(&candidate.plan, &candidate.evaluation))
        .unwrap_or_else(|| {
            stop_candidate_plan_v1("shop compiler produced no candidates".to_string()).plan
        })
}

fn evaluated_branch_alternatives_v1(
    candidates: &[ShopPlanCandidateV1],
    max_plans: usize,
) -> Vec<ShopPlanV1> {
    let allow_candidates = candidates
        .iter()
        .filter(|candidate| {
            !candidate.plan.steps.is_empty()
                && candidate.evaluation.verdict == ShopPlanVerdictV1::Allow
        })
        .collect::<Vec<_>>();
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
    let alternatives = select_branch_alternatives_with_effect_coverage_v1(&alternatives, max_plans);
    alternatives
        .into_iter()
        .map(|candidate| plan_with_evaluation_v1(&candidate.plan, &candidate.evaluation))
        .collect()
}

fn select_branch_alternatives_with_effect_coverage_v1<'a>(
    sorted_candidates: &[&'a ShopPlanCandidateV1],
    max_plans: usize,
) -> Vec<&'a ShopPlanCandidateV1> {
    if max_plans == 0 {
        return Vec::new();
    }

    let mut selected = Vec::new();
    let mut represented = std::collections::BTreeSet::<String>::new();
    for effect_kind in [
        "shop_buy_combo",
        "shop_buy_relic",
        "shop_buy_card",
        "shop_buy_potion",
        "shop_purge",
        "shop_leave",
    ] {
        if selected.len() >= max_plans {
            break;
        }
        let Some(candidate) = sorted_candidates.iter().copied().find(|candidate| {
            shop_plan_effect_kind_for_coverage_v1(&candidate.plan) == effect_kind
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

fn shop_plan_effect_kind_for_coverage_v1(plan: &ShopPlanV1) -> &'static str {
    if plan.steps.len() > 1 {
        return "shop_buy_combo";
    }
    match plan.steps.first() {
        Some(ShopPlanStepV1::BuyCard { .. }) => "shop_buy_card",
        Some(ShopPlanStepV1::BuyRelic { .. }) => "shop_buy_relic",
        Some(ShopPlanStepV1::BuyPotion { .. }) => "shop_buy_potion",
        Some(ShopPlanStepV1::RemoveCard { .. }) => "shop_purge",
        Some(ShopPlanStepV1::LeaveShop) => "shop_leave",
        None => "shop_stop",
    }
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
        verdict_rank_v1(candidate.evaluation.verdict),
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
