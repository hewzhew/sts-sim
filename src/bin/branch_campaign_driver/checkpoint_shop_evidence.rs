use sts_simulator::eval::run_control::RunControlSession;
use sts_simulator::state::core::EngineState;

use super::command_inputs::InspectEvidenceDetailV1;

pub(super) fn render_checkpoint_shop_evidence_v1(
    session: &RunControlSession,
    detail: InspectEvidenceDetailV1,
) -> Result<String, String> {
    let EngineState::Shop(shop) = &session.engine_state else {
        return Err(format!(
            "--inspect-shop-evidence requires Shop engine state, got {:?}",
            session.engine_state
        ));
    };
    let context =
        sts_simulator::ai::shop_policy_v1::build_shop_decision_context_v1(&session.run_state, shop);
    let compiled = sts_simulator::ai::shop_policy_v1::compile_shop_decision_v1(
        &context,
        &sts_simulator::ai::shop_policy_v1::ShopPolicyConfigV1::default(),
        sts_simulator::ai::shop_policy_v1::ShopCompileModeV1::BranchTopK { max_plans: 6 },
    );
    let trace = &compiled.strategic_trace;
    let mut lines = Vec::new();
    lines.push(format!(
        "Shop compiled decision: act={} floor={} hp={}/{} gold={} boss={:?}",
        session.run_state.act_num,
        session.run_state.floor_num,
        session.run_state.current_hp,
        session.run_state.max_hp,
        session.run_state.gold,
        session.run_state.boss_key
    ));
    lines.push(format!(
        "context: conversion_pressure={} affordable_purchase_exists={} candidates={}",
        context.conversion_pressure,
        context.affordable_purchase_exists,
        context.candidates.len()
    ));
    if let Some(projection) = &compiled.rollout_head {
        let rendered = compiled
            .candidate_plans
            .iter()
            .find(|candidate| candidate.plan.plan_id == projection.plan_id)
            .map(|candidate| {
                render_shop_plan_with_evaluation_for_detail_v1(
                    &candidate.plan,
                    &compiled.candidate_plans,
                    detail,
                )
            })
            .unwrap_or_else(|| format!("missing plan {}", projection.plan_id));
        lines.push(format!(
            "rollout_head: lane={:?} {}",
            projection.lane, rendered
        ));
    } else {
        lines.push("rollout_head: -".to_string());
    }
    lines.push(render_shop_plan_candidate_summary_v1(
        &compiled.candidate_plans,
    ));
    if compiled.branch_frontier.is_empty() {
        lines.push("branch_frontier: -".to_string());
    } else {
        lines.push(format!(
            "branch_frontier: {}",
            compiled.branch_frontier.len()
        ));
        for (idx, projection) in compiled.branch_frontier.iter().enumerate() {
            let rendered = compiled
                .candidate_plans
                .iter()
                .find(|candidate| candidate.plan.plan_id == projection.plan_id)
                .map(|candidate| {
                    render_shop_plan_with_evaluation_for_detail_v1(
                        &candidate.plan,
                        &compiled.candidate_plans,
                        detail,
                    )
                })
                .unwrap_or_else(|| format!("missing plan {}", projection.plan_id));
            lines.push(format!("  {idx}. lane={:?} {}", projection.lane, rendered));
        }
    }
    match detail {
        InspectEvidenceDetailV1::Compact => {
            lines.extend(render_shop_candidate_evidence_compact_v1(
                &context.candidates,
                trace,
            ));
        }
        InspectEvidenceDetailV1::Full => {
            lines.extend(render_shop_candidate_evidence_full_v1(
                &context.candidates,
                trace,
            ));
        }
    }
    if let Some(action) = trace.would_choose.as_ref() {
        lines.push(format!(
            "strategic_trace_would_choose: {}",
            action.candidate_id()
        ));
    } else {
        lines.push("strategic_trace_would_choose: -".to_string());
    }
    Ok(lines.join("\n"))
}

fn render_shop_candidate_evidence_full_v1(
    candidates: &[sts_simulator::ai::shop_policy_v1::ShopCandidateEvidenceV1],
    trace: &sts_simulator::ai::strategic::StrategicDecisionTrace,
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("candidate evidence:".to_string());
    for candidate in candidates {
        let action_id = inspect_shop_candidate_action_id(candidate);
        let compiled = trace
            .compiled
            .iter()
            .find(|decision| decision.action.candidate_id() == action_id);
        let delta = trace
            .candidate_deltas
            .iter()
            .find(|delta| delta.action.candidate_id() == action_id);
        lines.push(format!(
            "- {} | id={} | class={:?} gate={:?} legacy_estimate={} verdict={} score={}",
            candidate.label,
            action_id,
            candidate.class,
            candidate.support_gate,
            candidate
                .legacy_estimate
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            compiled
                .map(|decision| format!("{:?}", decision.verdict))
                .unwrap_or_else(|| "-".to_string()),
            compiled
                .map(|decision| format!("{:.2}", decision.score))
                .unwrap_or_else(|| "-".to_string()),
        ));
        lines.push(format!(
            "    evidence: {}",
            render_short_list(&candidate.evidence)
        ));
        lines.push(format!(
            "    risks: {}",
            render_short_list(&candidate.risks)
        ));
        if let Some(delta) = delta {
            lines.push(format!(
                "    delta: role={:?} hint={:?} theses=[{}] positive=[{}] negative=[{}]",
                delta.role,
                delta.verdict_hint,
                render_acquisition_theses(&delta.acquisition_theses),
                render_ledger_deltas(&delta.positive),
                render_ledger_deltas(&delta.negative)
            ));
        }
    }
    lines
}

fn render_shop_candidate_evidence_compact_v1(
    candidates: &[sts_simulator::ai::shop_policy_v1::ShopCandidateEvidenceV1],
    trace: &sts_simulator::ai::strategic::StrategicDecisionTrace,
) -> Vec<String> {
    let mut by_class = std::collections::BTreeMap::<String, usize>::new();
    let mut by_gate = std::collections::BTreeMap::<String, usize>::new();
    let mut by_verdict = std::collections::BTreeMap::<String, usize>::new();
    for candidate in candidates {
        *by_class
            .entry(format!("{:?}", candidate.class))
            .or_insert(0) += 1;
        *by_gate
            .entry(format!("{:?}", candidate.support_gate))
            .or_insert(0) += 1;
        let action_id = inspect_shop_candidate_action_id(candidate);
        let verdict = trace
            .compiled
            .iter()
            .find(|decision| decision.action.candidate_id() == action_id)
            .map(|decision| format!("{:?}", decision.verdict))
            .unwrap_or_else(|| "-".to_string());
        *by_verdict.entry(verdict).or_insert(0) += 1;
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "candidate_evidence: compact total={} by_class=[{}] by_gate=[{}] by_verdict=[{}]",
        candidates.len(),
        render_count_map_v1(&by_class),
        render_count_map_v1(&by_gate),
        render_count_map_v1(&by_verdict)
    ));

    let notable = compact_shop_notable_candidates_v1(candidates, trace);
    if notable.is_empty() {
        lines.push("candidate_highlights: -".to_string());
    } else {
        lines.push(format!("candidate_highlights: {}", notable.len()));
        for (idx, line) in notable.into_iter().enumerate() {
            lines.push(format!("  {idx}. {line}"));
        }
    }
    lines.push(
        "candidate_evidence_detail: hidden; rerun with --inspect-evidence-detail full for full candidate evidence"
            .to_string(),
    );
    lines
}

fn compact_shop_notable_candidates_v1(
    candidates: &[sts_simulator::ai::shop_policy_v1::ShopCandidateEvidenceV1],
    trace: &sts_simulator::ai::strategic::StrategicDecisionTrace,
) -> Vec<String> {
    let mut ranked = candidates
        .iter()
        .filter_map(|candidate| {
            let action_id = inspect_shop_candidate_action_id(candidate);
            let compiled = trace
                .compiled
                .iter()
                .find(|decision| decision.action.candidate_id() == action_id);
            let legacy = candidate.legacy_estimate.unwrap_or(0);
            let score = compiled.map(|decision| decision.score).unwrap_or(0.0);
            let strong_gate = matches!(
                candidate.support_gate,
                sts_simulator::ai::noncombat_strategy_v1::StrategyPlanSupportV1::Strong
            );
            let notable =
                strong_gate || legacy >= 650 || score.abs() >= 0.30 || !candidate.risks.is_empty();
            if !notable {
                return None;
            }
            Some((
                compact_shop_notable_rank_v1(strong_gate, legacy, score),
                render_shop_notable_candidate_line_v1(candidate, compiled),
            ))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.0.cmp(&left.0));
    ranked.into_iter().take(6).map(|(_, line)| line).collect()
}

fn compact_shop_notable_rank_v1(strong_gate: bool, legacy: i32, score: f32) -> i32 {
    let gate_bonus = if strong_gate { 10_000 } else { 0 };
    let legacy_bonus = legacy.max(0);
    let score_bonus = (score.abs() * 1_000.0) as i32;
    gate_bonus + legacy_bonus + score_bonus
}

fn render_shop_notable_candidate_line_v1(
    candidate: &sts_simulator::ai::shop_policy_v1::ShopCandidateEvidenceV1,
    compiled: Option<&sts_simulator::ai::strategic::CompiledDecision>,
) -> String {
    let legacy = candidate
        .legacy_estimate
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    let verdict = compiled
        .map(|decision| format!("{:?}", decision.verdict))
        .unwrap_or_else(|| "-".to_string());
    let score = compiled
        .map(|decision| format!("{:.2}", decision.score))
        .unwrap_or_else(|| "-".to_string());
    format!(
        "{} | class={:?} gate={:?} legacy={} verdict={} score={}",
        candidate.label, candidate.class, candidate.support_gate, legacy, verdict, score
    )
}

fn render_count_map_v1(counts: &std::collections::BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    counts
        .iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_shop_plan_candidate_summary_v1(
    candidates: &[sts_simulator::ai::shop_policy_v1::ShopPlanCandidateV1],
) -> String {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for candidate in candidates {
        *counts.entry(format!("{:?}", candidate.role)).or_insert(0) += 1;
    }
    let counts = counts
        .into_iter()
        .map(|(role, count)| format!("{role}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    let examples = candidates
        .iter()
        .take(4)
        .map(|candidate| {
            format!(
                "{:?}:{:?}:rollout{:?}:branch{:?}:tier{}:score{}:{}",
                candidate.role,
                candidate.evaluation.verdict,
                candidate.evaluation.rollout_admission.status,
                candidate.evaluation.branch_admission.status,
                candidate.evaluation.tier,
                candidate.evaluation.score,
                candidate.plan.plan_id
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "candidate_plans: {} [{}] examples=[{}]",
        candidates.len(),
        counts,
        if examples.is_empty() { "-" } else { &examples }
    )
}

fn render_shop_plan_with_evaluation_for_detail_v1(
    plan: &sts_simulator::ai::shop_policy_v1::ShopPlanV1,
    candidates: &[sts_simulator::ai::shop_policy_v1::ShopPlanCandidateV1],
    detail: InspectEvidenceDetailV1,
) -> String {
    let evaluation = candidates
        .iter()
        .find(|candidate| candidate.plan.plan_id == plan.plan_id)
        .map(|candidate| match detail {
            InspectEvidenceDetailV1::Compact => {
                render_shop_plan_evaluation_compact_v1(&candidate.evaluation)
            }
            InspectEvidenceDetailV1::Full => render_shop_plan_evaluation_v1(&candidate.evaluation),
        })
        .unwrap_or_else(|| "evaluation=-".to_string());
    let plan = match detail {
        InspectEvidenceDetailV1::Compact => render_shop_plan_compact_v1(plan),
        InspectEvidenceDetailV1::Full => render_shop_plan_v1(plan),
    };
    format!("{plan} | {evaluation}")
}

fn render_shop_plan_evaluation_compact_v1(
    evaluation: &sts_simulator::ai::shop_policy_v1::ShopPlanEvaluationV1,
) -> String {
    let legacy_estimate = evaluation
        .legacy_priority
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    format!(
        "evaluation={:?} rollout={:?} branch={:?} tier={} score={} confidence={:.2} legacy_estimate={} reasons=[{}]",
        evaluation.verdict,
        evaluation.rollout_admission.status,
        evaluation.branch_admission.status,
        evaluation.tier,
        evaluation.score,
        evaluation.confidence,
        legacy_estimate,
        render_short_list_limited_v1(&evaluation.reasons, 2)
    )
}

fn render_shop_plan_evaluation_v1(
    evaluation: &sts_simulator::ai::shop_policy_v1::ShopPlanEvaluationV1,
) -> String {
    let legacy_estimate = evaluation
        .legacy_priority
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    format!(
        "evaluation={:?} rollout={:?} branch={} tier={} score={} confidence={:.2} legacy_estimate={} component_score=net:{:.1}/pos:{:.1}/neg:{:.1}/conf:{:.2} components=[{}] reasons=[{}]",
        evaluation.verdict,
        evaluation.rollout_admission.status,
        match evaluation.branch_admission.status {
            sts_simulator::ai::shop_policy_v1::ShopPlanBranchAdmissionStatusV1::Admit => {
                format!("Admit({})", evaluation.branch_admission.reason)
            }
            sts_simulator::ai::shop_policy_v1::ShopPlanBranchAdmissionStatusV1::Reject => {
                format!("Reject({})", evaluation.branch_admission.reason)
            }
        },
        evaluation.tier,
        evaluation.score,
        evaluation.confidence,
        legacy_estimate,
        evaluation.component_score.net,
        evaluation.component_score.positive,
        evaluation.component_score.negative,
        evaluation.component_score.confidence,
        render_shop_plan_components_v1(&evaluation.components),
        render_short_list(&evaluation.reasons)
    )
}

fn render_shop_plan_components_v1(
    components: &[sts_simulator::ai::shop_policy_v1::ShopPlanComponentV1],
) -> String {
    if components.is_empty() {
        return "-".to_string();
    }
    components
        .iter()
        .map(|component| {
            format!(
                "{:?}:{:.1}:{}",
                component.kind, component.amount, component.reason
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_shop_plan_v1(plan: &sts_simulator::ai::shop_policy_v1::ShopPlanV1) -> String {
    let steps = if plan.steps.is_empty() {
        "-".to_string()
    } else {
        plan.steps
            .iter()
            .map(render_shop_plan_step_v1)
            .collect::<Vec<_>>()
            .join(" then ")
    };
    let legacy_estimate = plan
        .legacy_priority
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    format!(
        "{} | kind={:?} source={:?} cost={} legacy_estimate={} candidates=[{}] steps=[{}] reason={}",
        plan.label,
        plan.kind,
        plan.source,
        plan.total_gold_spent,
        legacy_estimate,
        plan.candidate_ids.join(","),
        steps,
        plan.reason
    )
}

fn render_shop_plan_compact_v1(plan: &sts_simulator::ai::shop_policy_v1::ShopPlanV1) -> String {
    let steps = if plan.steps.is_empty() {
        "-".to_string()
    } else {
        plan.steps
            .iter()
            .map(render_shop_plan_step_v1)
            .collect::<Vec<_>>()
            .join(" then ")
    };
    format!(
        "{} | kind={:?} cost={} steps=[{}]",
        plan.label, plan.kind, plan.total_gold_spent, steps
    )
}

fn render_shop_plan_step_v1(step: &sts_simulator::ai::shop_policy_v1::ShopPlanStepV1) -> String {
    match *step {
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::BuyCard { index, card, cost } => {
            format!("buy card {index} {:?} {cost}g", card)
        }
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::BuyRelic { index, relic, cost } => {
            format!("buy relic {index} {relic:?} {cost}g")
        }
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::BuyPotion {
            index,
            potion,
            cost,
        } => format!("buy potion {index} {potion:?} {cost}g"),
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::RemoveCard {
            deck_index,
            card,
            cost,
        } => format!("purge deck {deck_index} {card:?} {cost}g"),
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::LeaveShop => "leave shop".to_string(),
    }
}

fn inspect_shop_candidate_action_id(
    candidate: &sts_simulator::ai::shop_policy_v1::ShopCandidateEvidenceV1,
) -> String {
    use sts_simulator::ai::shop_policy_v1::{ShopPolicyClassV1, ShopPurchaseTargetV1};
    use sts_simulator::ai::strategic::CandidateAction;

    match candidate.purchase_target {
        Some(ShopPurchaseTargetV1::Card { index, card }) => CandidateAction::BuyCard {
            shop_index: index,
            card,
            gold: 0,
        }
        .candidate_id(),
        Some(ShopPurchaseTargetV1::Relic { index, relic }) => CandidateAction::BuyRelic {
            shop_index: index,
            relic,
            gold: 0,
        }
        .candidate_id(),
        Some(ShopPurchaseTargetV1::Potion { index, potion }) => CandidateAction::BuyPotion {
            shop_index: index,
            potion,
            gold: 0,
        }
        .candidate_id(),
        None if candidate.class == ShopPolicyClassV1::Leave => {
            CandidateAction::LeaveShop.candidate_id()
        }
        None => candidate
            .deck_index
            .zip(candidate.card)
            .map(|(deck_index, card)| CandidateAction::RemoveCard {
                deck_index,
                card,
                gold: None,
            })
            .map(|action| action.candidate_id())
            .unwrap_or_else(|| candidate.candidate_id.clone()),
    }
}

fn render_short_list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(", ")
    }
}

fn render_short_list_limited_v1(items: &[String], limit: usize) -> String {
    if items.is_empty() {
        return "-".to_string();
    }
    let mut rendered = items.iter().take(limit).cloned().collect::<Vec<_>>();
    if items.len() > limit {
        rendered.push(format!("... {} more", items.len() - limit));
    }
    rendered.join(", ")
}

fn render_ledger_deltas(items: &[sts_simulator::ai::strategic::LedgerDelta]) -> String {
    if items.is_empty() {
        return "-".to_string();
    }
    items
        .iter()
        .map(|delta| format!("{:?}:{:.2}:{}", delta.kind, delta.amount, delta.reason))
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_acquisition_theses(
    items: &[sts_simulator::ai::strategic::AcquisitionThesisSignal],
) -> String {
    if items.is_empty() {
        return "-".to_string();
    }
    items
        .iter()
        .map(|thesis| {
            format!(
                "{:?}/{:?}:{:.2}:{}",
                thesis.role, thesis.status, thesis.amount, thesis.reason
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
