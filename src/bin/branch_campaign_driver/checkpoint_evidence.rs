use sts_simulator::eval::run_control::RunControlSession;
use sts_simulator::state::core::EngineState;

pub(super) fn render_checkpoint_campfire_evidence_v1(
    session: &RunControlSession,
) -> Result<String, String> {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return Err(format!(
            "--inspect-campfire-evidence requires Campfire engine state, got {:?}",
            session.engine_state
        ));
    }
    let context = sts_simulator::ai::campfire_policy_v1::build_campfire_decision_context_v1(
        &session.run_state,
        sts_simulator::engine::campfire_handler::get_available_options(&session.run_state),
    );
    let decision = sts_simulator::ai::campfire_policy_v1::plan_campfire_decision_v1(
        &context,
        &sts_simulator::ai::campfire_policy_v1::CampfirePolicyConfigV1::default(),
    );
    let mut lines = Vec::new();
    let formation = context.strategy.formation_summary();
    lines.push(format!(
        "Campfire compiled decision: act={} floor={} hp={}/{} gold={} boss={:?}",
        session.run_state.act_num,
        session.run_state.floor_num,
        session.run_state.current_hp,
        session.run_state.max_hp,
        session.run_state.gold,
        session.run_state.boss_key
    ));
    lines.push(format!(
        "selected: plan_id={} role={:?} score={} execute={} confidence={:.2} action={:?}",
        decision.selected_plan.plan_id,
        decision.selected_plan.role,
        decision.selected_plan.score_hint,
        decision.selected_plan.execute_autopilot,
        decision.selected_plan.confidence,
        decision.selected_plan.action
    ));
    lines.push(format!(
        "context: candidates={} formation={:?} needs={:?}",
        context.candidates.len(),
        formation.stage,
        formation.needs
    ));
    lines.push("candidate plans:".to_string());
    for plan in &decision.candidate_plans {
        lines.push(format!(
            "  - {} role={:?} score={} execute={} branch_active={} confidence={:.2} action={:?}",
            plan.plan_id,
            plan.role,
            plan.score_hint,
            plan.execute_autopilot,
            plan.branch_active,
            plan.confidence,
            plan.action
        ));
        if let Some(tag) = &plan.strategy_tag {
            lines.push(format!("      strategy_tag={tag}"));
        }
        for reason in plan.reasons.iter().take(4) {
            lines.push(format!("      reason: {reason}"));
        }
        if let Some(candidate) = context
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == plan.plan_id)
        {
            lines.push(format!(
                "      class={:?} support_gate={:?} upgrade_score={:?} deck_mutation_execute={:?}",
                candidate.class,
                candidate.support_gate,
                candidate.upgrade_plan_score_hint,
                candidate.deck_mutation_execute_allowed
            ));
            for evidence in candidate.evidence.iter().take(6) {
                lines.push(format!("      evidence: {evidence}"));
            }
            for risk in candidate.risks.iter().take(4) {
                lines.push(format!("      risk: {risk}"));
            }
        }
    }
    Ok(lines.join("\n"))
}

pub(super) fn render_checkpoint_route_evidence_v1(
    session: &RunControlSession,
) -> Result<String, String> {
    if !session.engine_state.is_map_surface() {
        return Err(format!(
            "--inspect-route-evidence requires MapNavigation/MapOverlay engine state, got {:?}",
            session.engine_state
        ));
    }
    let trace = sts_simulator::ai::route_planner_v1::plan_route_decision_v1(
        &session.run_state,
        &session.engine_state,
        sts_simulator::ai::route_planner_v1::RoutePlannerConfigV1::default(),
    );
    Ok(sts_simulator::ai::route_planner_v1::render_route_decision_trace_v1(&trace))
}

pub(super) fn render_checkpoint_deck_mutation_v1(
    session: &RunControlSession,
) -> Result<String, String> {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return Err(format!(
            "--inspect-deck-mutation requires RunPendingChoice engine state, got {:?}",
            session.engine_state
        ));
    };
    let decision = sts_simulator::ai::deck_mutation_compiler_v1::compile_deck_mutation_decision_v1(
        &session.run_state,
        choice,
        sts_simulator::ai::deck_mutation_compiler_v1::DeckMutationCompilerModeV1::Inspect,
    );
    Ok(
        sts_simulator::ai::deck_mutation_compiler_v1::render_compiled_deck_mutation_decision_v1(
            &decision,
        ),
    )
}

pub(super) fn render_checkpoint_shop_evidence_v1(
    session: &RunControlSession,
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
                render_shop_plan_with_evaluation_v1(&candidate.plan, &compiled.candidate_plans)
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
                    render_shop_plan_with_evaluation_v1(&candidate.plan, &compiled.candidate_plans)
                })
                .unwrap_or_else(|| format!("missing plan {}", projection.plan_id));
            lines.push(format!("  {idx}. lane={:?} {}", projection.lane, rendered));
        }
    }
    lines.push("candidate evidence:".to_string());
    for candidate in &context.candidates {
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
                .purchase_priority
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

pub(super) fn render_checkpoint_card_reward_evidence_v1(
    session: &RunControlSession,
) -> Result<String, String> {
    let cards = active_or_visible_reward_cards_for_inspect_v1(session).ok_or_else(|| {
        format!(
            "--inspect-card-reward-evidence requires an open or visible card reward, got {:?}",
            session.engine_state
        )
    })?;
    let context =
        sts_simulator::ai::card_reward_policy_v1::build_card_reward_decision_context_with_current_route_v1(
            &session.run_state,
            &session.engine_state,
            cards,
        );
    let trace = sts_simulator::ai::strategic::strategic_trace_for_card_reward(&context);
    let mut lines = Vec::new();
    lines.push(format!(
        "Card reward compiled decision: act={} floor={} hp={}/{} gold={} boss={:?}",
        session.run_state.act_num,
        session.run_state.floor_num,
        session.run_state.current_hp,
        session.run_state.max_hp,
        session.run_state.gold,
        session.run_state.boss_key
    ));
    lines.push(format!(
        "context: candidates={} deck_size={} startup_strong_draw={}->{} has_singing_bowl={}",
        context.candidates.len(),
        context.deck.deck_size,
        context.startup.strong_draw_count,
        context.startup.effective_strong_draw_count,
        context.has_singing_bowl
    ));
    lines.push("candidate evidence:".to_string());
    for candidate in &context.candidates {
        let action = sts_simulator::ai::strategic::CandidateAction::TakeCard {
            index: candidate.index,
            card: candidate.card,
        };
        let compiled = trace.compiled_for_action(&action);
        let delta = trace
            .candidate_deltas
            .iter()
            .find(|delta| delta.action == action);
        lines.push(format!(
            "- {} | index={} card={:?} same_card_count={} verdict={} score={}",
            candidate.name,
            candidate.index,
            candidate.card,
            candidate.same_card_count,
            compiled
                .map(|decision| format!("{:?}", decision.verdict))
                .unwrap_or_else(|| "-".to_string()),
            compiled
                .map(|decision| format!("{:.2}", decision.score))
                .unwrap_or_else(|| "-".to_string()),
        ));
        lines.push(format!(
            "    facts: type={:?} cost={} damage={} block={} draw={} energy={} roles=[{}]",
            candidate.card_type,
            candidate.facts.cost,
            candidate.facts.damage.total_damage,
            candidate.facts.block,
            candidate.facts.draw_cards,
            candidate.facts.energy_gain,
            render_short_list(
                &sts_simulator::ai::card_reward_policy_v1::card_reward_semantic_profile_v1(
                    &sts_simulator::state::rewards::RewardCard::new(
                        candidate.card,
                        candidate.facts.upgrades,
                    ),
                )
                .roles
                .iter()
                .map(|role| format!("{role:?}"))
                .collect::<Vec<_>>(),
            )
        ));
        if let Some(delta) = delta {
            lines.push(format!(
                "    delta: role={:?} hint={:?} theses=[{}] positive=[{}] negative=[{}] notes=[{}]",
                delta.role,
                delta.verdict_hint,
                render_acquisition_theses(&delta.acquisition_theses),
                render_ledger_deltas(&delta.positive),
                render_ledger_deltas(&delta.negative),
                render_short_list(&delta.notes)
            ));
        }
    }
    let skip_action = if context.has_singing_bowl {
        sts_simulator::ai::strategic::CandidateAction::TakeSingingBowl { max_hp_gain: 2 }
    } else {
        sts_simulator::ai::strategic::CandidateAction::SkipCardReward
    };
    if let Some(compiled) = trace.compiled_for_action(&skip_action) {
        lines.push(format!(
            "decline_candidate: action={} verdict={:?} score={:.2}",
            skip_action.candidate_id(),
            compiled.verdict,
            compiled.score
        ));
    }
    if let Some(action) = trace.would_choose.as_ref() {
        lines.push(format!("trace_would_choose: {}", action.candidate_id()));
    } else {
        lines.push("trace_would_choose: -".to_string());
    }
    Ok(lines.join("\n"))
}

fn active_or_visible_reward_cards_for_inspect_v1(
    session: &RunControlSession,
) -> Option<Vec<sts_simulator::state::rewards::RewardCard>> {
    match &session.engine_state {
        EngineState::RewardScreen(reward) => reward
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward_for_inspect_v1(reward)),
        EngineState::RewardOverlay { reward_state, .. } => reward_state
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward_for_inspect_v1(reward_state)),
        _ => None,
    }
}

fn first_visible_card_reward_for_inspect_v1(
    reward: &sts_simulator::state::rewards::RewardState,
) -> Option<Vec<sts_simulator::state::rewards::RewardCard>> {
    reward.items.iter().find_map(|item| match item {
        sts_simulator::state::rewards::RewardItem::Card { cards } => Some(cards.clone()),
        _ => None,
    })
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

fn render_shop_plan_with_evaluation_v1(
    plan: &sts_simulator::ai::shop_policy_v1::ShopPlanV1,
    candidates: &[sts_simulator::ai::shop_policy_v1::ShopPlanCandidateV1],
) -> String {
    let evaluation = candidates
        .iter()
        .find(|candidate| candidate.plan.plan_id == plan.plan_id)
        .map(|candidate| render_shop_plan_evaluation_v1(&candidate.evaluation))
        .unwrap_or_else(|| "evaluation=-".to_string());
    format!("{} | {}", render_shop_plan_v1(plan), evaluation)
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
