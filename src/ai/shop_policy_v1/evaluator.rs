use crate::ai::noncombat_strategy_v1::StrategyPlanSupportV1;
use crate::ai::strategic::{
    AcquisitionVerdict, CandidateAction, CompiledDecision, PressureKind, StrategicDecisionTrace,
};

use super::component_scorer::score_shop_plan_components_v1;
use super::types::{
    ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopFutureShopV1, ShopMawBankStateV1,
    ShopPlanCandidateRoleV1, ShopPlanCandidateV1, ShopPlanComponentKindV1, ShopPlanComponentV1,
    ShopPlanEvaluationV1, ShopPlanKindV1, ShopPlanSourceV1, ShopPlanStepV1, ShopPolicyClassV1,
    ShopPolicyConfigV1, ShopPurchaseRiskV1, ShopPurchaseTargetV1, ShopThreatWindowV1,
};

pub(crate) fn evaluate_shop_plan_candidate_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
    strategic_trace: &StrategicDecisionTrace,
    candidate_plan: &ShopPlanCandidateV1,
) -> ShopPlanEvaluationV1 {
    if candidate_plan.plan.kind == ShopPlanKindV1::Stop
        || candidate_plan.role == ShopPlanCandidateRoleV1::StopFallback
        || candidate_plan.plan.steps.is_empty()
    {
        return attach_components_and_score_v1(
            context,
            strategic_trace,
            ShopPlanEvaluationV1::stop(candidate_plan.plan.reason.clone()),
            candidate_plan,
        );
    }

    if candidate_plan.role == ShopPlanCandidateRoleV1::PortfolioAlternative
        || candidate_plan.plan.source == ShopPlanSourceV1::PortfolioCandidate
    {
        return attach_components_and_score_v1(
            context,
            strategic_trace,
            evaluate_portfolio_plan_v1(context, config, strategic_trace, candidate_plan),
            candidate_plan,
        );
    }

    let Some(candidate_id) = candidate_plan.plan.candidate_ids.first() else {
        return attach_components_and_score_v1(
            context,
            strategic_trace,
            ShopPlanEvaluationV1::block(
                candidate_plan.plan.legacy_priority,
                "shop plan has no candidate id",
            ),
            candidate_plan,
        );
    };
    let Some(candidate) = context
        .candidates
        .iter()
        .find(|candidate| &candidate.candidate_id == candidate_id)
    else {
        return attach_components_and_score_v1(
            context,
            strategic_trace,
            ShopPlanEvaluationV1::block(
                candidate_plan.plan.legacy_priority,
                format!("shop plan candidate id {candidate_id} is no longer visible"),
            ),
            candidate_plan,
        );
    };

    attach_components_and_score_v1(
        context,
        strategic_trace,
        evaluate_single_candidate_v1(context, config, strategic_trace, candidate),
        candidate_plan,
    )
}

fn attach_components_and_score_v1(
    context: &ShopDecisionContextV1,
    strategic_trace: &StrategicDecisionTrace,
    mut evaluation: ShopPlanEvaluationV1,
    candidate_plan: &ShopPlanCandidateV1,
) -> ShopPlanEvaluationV1 {
    evaluation.components = plan_components_v1(context, strategic_trace, candidate_plan);
    evaluation.component_score = score_shop_plan_components_v1(&evaluation.components);
    evaluation
}

fn evaluate_single_candidate_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
    strategic_trace: &StrategicDecisionTrace,
    candidate: &ShopCandidateEvidenceV1,
) -> ShopPlanEvaluationV1 {
    match candidate.class {
        ShopPolicyClassV1::CursePurge => evaluate_curse_purge_v1(candidate, config),
        ShopPolicyClassV1::StarterStrikePurge | ShopPolicyClassV1::StarterDefendPurge => {
            evaluate_starter_purge_v1(candidate, config, strategic_trace)
        }
        ShopPolicyClassV1::FunctionalRepairPurge => {
            evaluate_functional_repair_purge_v1(candidate, config)
        }
        ShopPolicyClassV1::PurchaseOpportunity => {
            evaluate_purchase_v1(context, candidate, config, strategic_trace)
        }
        ShopPolicyClassV1::Leave => ShopPlanEvaluationV1::stop("legacy shop leave candidate"),
        ShopPolicyClassV1::Unknown => ShopPlanEvaluationV1::block(
            candidate.legacy_estimate,
            "shop evaluator does not mark unknown shop candidate rollout-eligible",
        ),
    }
}

fn evaluate_curse_purge_v1(
    candidate: &ShopCandidateEvidenceV1,
    config: &ShopPolicyConfigV1,
) -> ShopPlanEvaluationV1 {
    if !config.allow_curse_purge {
        return ShopPlanEvaluationV1::block(None, "curse purge disabled by shop policy config");
    }
    if candidate.deck_index.is_none() || candidate.card.is_none() {
        return ShopPlanEvaluationV1::block(None, "curse purge candidate lacks deck/card identity");
    }
    ShopPlanEvaluationV1::allow(400, 1000, 0.92, None, "shop evaluator: curse cleanup")
}

fn evaluate_functional_repair_purge_v1(
    candidate: &ShopCandidateEvidenceV1,
    config: &ShopPolicyConfigV1,
) -> ShopPlanEvaluationV1 {
    if !config.allow_functional_repair_purge {
        return ShopPlanEvaluationV1::block(
            None,
            "functional repair purge disabled by shop policy config",
        );
    }
    if candidate.support_gate != StrategyPlanSupportV1::Strong
        || candidate.deck_index.is_none()
        || candidate.card.is_none()
        || !candidate
            .evidence
            .iter()
            .any(|item| item == "deck_repair_profile=low_loss_redundant_functional")
    {
        return ShopPlanEvaluationV1::block(
            None,
            "functional purge lacks exact low-loss deck-repair evidence",
        );
    }
    ShopPlanEvaluationV1::allow(
        305,
        450,
        0.74,
        None,
        "shop evaluator: evidence-backed functional deck repair",
    )
}

fn evaluate_purchase_v1(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
    config: &ShopPolicyConfigV1,
    strategic_trace: &StrategicDecisionTrace,
) -> ShopPlanEvaluationV1 {
    if candidate.support_gate != StrategyPlanSupportV1::Strong {
        return ShopPlanEvaluationV1::block(
            candidate.legacy_estimate,
            format!(
                "purchase support gate {:?} is not Strong",
                candidate.support_gate
            ),
        );
    }
    let Some(target) = candidate.purchase_target else {
        return ShopPlanEvaluationV1::block(candidate.legacy_estimate, "purchase target missing");
    };
    if let Some(reason) = blocking_purchase_risk_reason_v1(candidate) {
        return ShopPlanEvaluationV1::block(candidate.legacy_estimate, reason);
    }
    if let ShopPurchaseTargetV1::Card { .. } = target {
        let Some(strategic_decision) = purchase_strategic_decision(target, strategic_trace) else {
            return ShopPlanEvaluationV1::block(
                candidate.legacy_estimate,
                "strategic trace has no shop card purchase decision",
            );
        };
        if !strategic_decision.verdict.allows_behavior_acquisition() {
            return ShopPlanEvaluationV1::block(
                candidate.legacy_estimate,
                format!(
                    "strategic trace rejects shop purchase as rollout head verdict={:?} score={:.2}",
                    strategic_decision.verdict, strategic_decision.score
                ),
            );
        }
        return strategic_purchase_evaluation_v1(
            candidate.legacy_estimate,
            target,
            strategic_decision,
        );
    }

    if let ShopPurchaseTargetV1::Potion { .. } = target {
        return evaluate_temporary_potion_purchase_v1(context, candidate, target, strategic_trace);
    }

    let Some(priority) = candidate.legacy_estimate else {
        return ShopPlanEvaluationV1::block(None, "purchase legacy estimate missing");
    };
    let threshold = config.high_impact_relic_legacy_estimate_threshold;
    if config.allow_high_impact_purchase && priority >= threshold {
        return ShopPlanEvaluationV1::allow(
            300,
            priority,
            0.76,
            Some(priority),
            format!(
                "shop evaluator: high-impact legacy estimate {priority} clears threshold {threshold}; strategic verdict allows purchase"
            ),
        );
    }

    ShopPlanEvaluationV1::block(
        Some(priority),
        format!("purchase legacy estimate {priority} does not clear legacy shop evaluator gates"),
    )
}

fn evaluate_temporary_potion_purchase_v1(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
    target: ShopPurchaseTargetV1,
    strategic_trace: &StrategicDecisionTrace,
) -> ShopPlanEvaluationV1 {
    // This is an admission contract over observable semantics and the typed
    // pressure ledger. A single seed result (or combat-search failure) must
    // not promote or demote a potion here; outcome calibration belongs in a
    // separate evidence loop with comparable samples.
    if !matches!(
        context.visit.next_threat,
        ShopThreatWindowV1::EliteIn(0..=2) | ShopThreatWindowV1::BossIn(0..=4)
    ) {
        return ShopPlanEvaluationV1::block(
            candidate.legacy_estimate,
            "temporary potion has no typed near-hard-fight window",
        );
    }

    let Some(strategic_decision) = purchase_strategic_decision(target, strategic_trace) else {
        return ShopPlanEvaluationV1::block(
            candidate.legacy_estimate,
            "strategic trace has no typed potion purchase decision",
        );
    };
    let matched_temporary_pressures = strategic_decision
        .matched_pressure_kinds
        .iter()
        .copied()
        .filter(|kind| immediate_combat_pressure_kind_v1(*kind))
        .collect::<Vec<_>>();
    if matched_temporary_pressures.is_empty() {
        return ShopPlanEvaluationV1::block(
            candidate.legacy_estimate,
            "temporary potion matches no current strategic pressure",
        );
    }
    if !strategic_decision.verdict.allows_behavior_acquisition() {
        return ShopPlanEvaluationV1::block(
            candidate.legacy_estimate,
            format!(
                "typed potion pressure match is not strong enough for rollout: verdict={:?} score={:.2}",
                strategic_decision.verdict, strategic_decision.score
            ),
        );
    }

    strategic_temporary_potion_evaluation_v1(
        candidate.legacy_estimate,
        strategic_decision,
        matched_temporary_pressures.len(),
    )
}

fn immediate_combat_pressure_kind_v1(kind: PressureKind) -> bool {
    matches!(
        kind,
        PressureKind::MissingJob(
            crate::ai::strategic::StrategicJob::Frontload
                | crate::ai::strategic::StrategicJob::Block
                | crate::ai::strategic::StrategicJob::Scaling
                | crate::ai::strategic::StrategicJob::DrawEnergy
                | crate::ai::strategic::StrategicJob::Consistency
                | crate::ai::strategic::StrategicJob::EnemyStrengthDown
        ) | PressureKind::BossTax(_)
    )
}

fn strategic_temporary_potion_evaluation_v1(
    legacy_priority: Option<i32>,
    strategic_decision: &CompiledDecision,
    matched_pressure_count: usize,
) -> ShopPlanEvaluationV1 {
    let mut tier = match strategic_decision.verdict {
        AcquisitionVerdict::MustTake => 315,
        AcquisitionVerdict::StrongTake => 305,
        AcquisitionVerdict::ContextTake => 290,
        _ => 0,
    };
    if strategic_decision_matches_boss_tax_v1(strategic_decision) {
        tier += 5;
    }
    let strategic_score = (strategic_decision.score.max(0.0) * 1000.0).round() as i32;
    let score = strategic_score.saturating_add((matched_pressure_count as i32) * 25);
    let confidence = match strategic_decision.verdict {
        AcquisitionVerdict::MustTake => 0.72,
        AcquisitionVerdict::StrongTake => 0.66,
        AcquisitionVerdict::ContextTake => 0.60,
        _ => 0.0,
    };

    ShopPlanEvaluationV1::allow(
        tier,
        score,
        confidence,
        legacy_priority,
        format!(
            "typed temporary-resource evaluation: verdict={:?} score={:.2} matched_pressures={matched_pressure_count}; legacy estimate is audit-only",
            strategic_decision.verdict, strategic_decision.score
        ),
    )
}

fn evaluate_starter_purge_v1(
    candidate: &ShopCandidateEvidenceV1,
    config: &ShopPolicyConfigV1,
    strategic_trace: &StrategicDecisionTrace,
) -> ShopPlanEvaluationV1 {
    if !config.allow_starter_strike_purge_when_core_plan_protected {
        return ShopPlanEvaluationV1::block(
            None,
            "starter strike purge disabled by shop policy config",
        );
    }
    if candidate.support_gate != StrategyPlanSupportV1::Strong {
        return ShopPlanEvaluationV1::block(
            None,
            format!(
                "starter strike purge support gate {:?} is not Strong",
                candidate.support_gate
            ),
        );
    }
    let Some(strategic_decision) = starter_purge_strategic_decision(candidate, strategic_trace)
    else {
        return ShopPlanEvaluationV1::block(None, "strategic trace has no starter purge decision");
    };
    if !strategic_decision.verdict.allows_behavior_acquisition() {
        return ShopPlanEvaluationV1::block(
            None,
            format!(
                "strategic trace rejects starter purge as rollout head verdict={:?} score={:.2}",
                strategic_decision.verdict, strategic_decision.score
            ),
        );
    }

    strategic_starter_purge_evaluation_v1(candidate, strategic_decision)
}

fn evaluate_portfolio_plan_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
    strategic_trace: &StrategicDecisionTrace,
    candidate_plan: &ShopPlanCandidateV1,
) -> ShopPlanEvaluationV1 {
    if candidate_plan.plan.steps.is_empty() {
        return ShopPlanEvaluationV1::stop(candidate_plan.plan.reason.clone());
    }
    if !candidate_plan
        .plan
        .steps
        .iter()
        .all(|step| !matches!(step, ShopPlanStepV1::LeaveShop))
    {
        return ShopPlanEvaluationV1::stop(candidate_plan.plan.reason.clone());
    }
    if candidate_plan.plan.candidate_ids.is_empty() {
        return ShopPlanEvaluationV1::block(
            candidate_plan.plan.legacy_priority,
            "portfolio plan has no candidate ids for unified shop evaluation",
        );
    }

    let mut step_evaluations = Vec::new();
    let mut branch_only_step_count = 0usize;
    for candidate_id in &candidate_plan.plan.candidate_ids {
        let Some(candidate) = context
            .candidates
            .iter()
            .find(|candidate| &candidate.candidate_id == candidate_id)
        else {
            return ShopPlanEvaluationV1::block(
                candidate_plan.plan.legacy_priority,
                format!("portfolio plan candidate id {candidate_id} is no longer visible"),
            );
        };
        let evaluation = evaluate_single_candidate_v1(context, config, strategic_trace, candidate);
        if !evaluation.branch_admission.is_admitted() {
            let reason = evaluation
                .reasons
                .first()
                .cloned()
                .unwrap_or_else(|| "candidate blocked by unified shop gate".to_string());
            return ShopPlanEvaluationV1::block(
                candidate
                    .legacy_estimate
                    .or(candidate_plan.plan.legacy_priority),
                format!("portfolio step {candidate_id} failed shop branch admission: {reason}"),
            );
        }
        if !evaluation.rollout_admission.is_admitted() {
            branch_only_step_count += 1;
        }
        if candidate_plan.plan.steps.len() > 1
            && candidate
                .purchase_target
                .is_some_and(|target| matches!(target, ShopPurchaseTargetV1::Card { .. }))
            && evaluation.rollout_admission.is_admitted()
            && evaluation.tier < 320
        {
            return ShopPlanEvaluationV1::block(
                candidate
                    .legacy_estimate
                    .or(candidate_plan.plan.legacy_priority),
                format!(
                    "portfolio step {candidate_id} is a context card purchase; keep it as a single-step branch probe instead of a multi-buy combo"
                ),
            );
        }
        step_evaluations.push(evaluation);
    }

    let confidence = step_evaluations
        .iter()
        .map(|evaluation| evaluation.confidence)
        .fold(0.50_f32, f32::min);
    let tier = step_evaluations
        .iter()
        .map(|evaluation| evaluation.tier)
        .max()
        .unwrap_or(150);
    let legacy_priority = candidate_plan.plan.legacy_priority.unwrap_or_default();
    let score = step_evaluations
        .iter()
        .map(|evaluation| evaluation.score)
        .sum::<i32>()
        .max(legacy_priority);
    if branch_only_step_count > 0 {
        return ShopPlanEvaluationV1::block(
            Some(score),
            format!(
                "multi-step shop plan contains {branch_only_step_count} branch-frontier-only step(s); keep as branch frontier, not rollout head"
            ),
        )
        .with_branch_admission(
            "multi-step shop plan admitted to branch frontier because every step passed branch admission",
        );
    }
    ShopPlanEvaluationV1::allow(
        tier,
        score,
        confidence,
        candidate_plan.plan.legacy_priority,
        "multi-step shop plan passed unified shop gates; strongest step tier retained for plan comparison",
    )
}

fn blocking_purchase_risk_reason_v1(candidate: &ShopCandidateEvidenceV1) -> Option<String> {
    candidate
        .risk_kinds
        .iter()
        .find(|risk| **risk == ShopPurchaseRiskV1::BossEnemyStrengthMultiHit)
        .map(|risk| format!("shop purchase blocked by typed risk {risk:?}"))
}

fn purchase_strategic_decision(
    target: ShopPurchaseTargetV1,
    strategic_trace: &StrategicDecisionTrace,
) -> Option<&CompiledDecision> {
    let action = match target {
        ShopPurchaseTargetV1::Card { index, card } => CandidateAction::BuyCard {
            shop_index: index,
            card,
            gold: 0,
        },
        ShopPurchaseTargetV1::Relic { index, relic } => CandidateAction::BuyRelic {
            shop_index: index,
            relic,
            gold: 0,
        },
        ShopPurchaseTargetV1::Potion { index, potion } => CandidateAction::BuyPotion {
            shop_index: index,
            potion,
            gold: 0,
        },
    };
    strategic_trace.compiled_for_action(&action)
}

fn starter_purge_strategic_decision<'a>(
    candidate: &ShopCandidateEvidenceV1,
    strategic_trace: &'a StrategicDecisionTrace,
) -> Option<&'a CompiledDecision> {
    let action = CandidateAction::RemoveCard {
        deck_index: candidate.deck_index?,
        card: candidate.card?,
        gold: None,
    };
    strategic_trace.compiled_for_action(&action)
}

fn strategic_starter_purge_evaluation_v1(
    candidate: &ShopCandidateEvidenceV1,
    strategic_decision: &CompiledDecision,
) -> ShopPlanEvaluationV1 {
    let tier = match strategic_decision.verdict {
        AcquisitionVerdict::MustTake => 330,
        AcquisitionVerdict::StrongTake => 320,
        AcquisitionVerdict::ContextTake => 300,
        _ => 0,
    };
    let base_score = match candidate.class {
        ShopPolicyClassV1::StarterStrikePurge => 40,
        ShopPolicyClassV1::StarterDefendPurge => 0,
        _ => 0,
    };
    let strategic_score = (strategic_decision.score.max(0.0) * 1000.0).round() as i32;
    let score = strategic_score.saturating_add(base_score);
    let confidence = match strategic_decision.verdict {
        AcquisitionVerdict::MustTake => 0.82,
        AcquisitionVerdict::StrongTake => 0.76,
        AcquisitionVerdict::ContextTake => 0.68,
        _ => 0.0,
    };

    ShopPlanEvaluationV1::allow(
        tier,
        score,
        confidence,
        None,
        format!(
            "strategic deck-cleaning evaluation: verdict={:?} score={:.2}",
            strategic_decision.verdict, strategic_decision.score
        ),
    )
}

fn strategic_purchase_evaluation_v1(
    legacy_priority: Option<i32>,
    target: ShopPurchaseTargetV1,
    strategic_decision: &CompiledDecision,
) -> ShopPlanEvaluationV1 {
    let mut tier = match strategic_decision.verdict {
        AcquisitionVerdict::MustTake => 330,
        AcquisitionVerdict::StrongTake => 320,
        AcquisitionVerdict::ContextTake => 300,
        _ => 0,
    };
    let matches_boss_tax = strategic_decision_matches_boss_tax_v1(strategic_decision);
    if matches_boss_tax && tier > 0 {
        // Boss-pressure alignment should win ties inside the same strategic
        // verdict class, but it must not let a ContextTake purchase outrank a
        // StrongTake/MustTake plan from the same unified compiler.
        tier = tier.max(match strategic_decision.verdict {
            AcquisitionVerdict::MustTake => 335,
            AcquisitionVerdict::StrongTake => 325,
            AcquisitionVerdict::ContextTake => 310,
            _ => tier,
        });
    }
    let strategic_score = (strategic_decision.score.max(0.0) * 1000.0).round() as i32;
    let score = strategic_score
        .saturating_add(legacy_priority.unwrap_or_default().max(0))
        .saturating_add(purchase_tiebreaker(target));
    let confidence = match strategic_decision.verdict {
        AcquisitionVerdict::MustTake => 0.82,
        AcquisitionVerdict::StrongTake => 0.76,
        AcquisitionVerdict::ContextTake => 0.68,
        _ => 0.0,
    };

    ShopPlanEvaluationV1::allow(
        tier,
        score,
        confidence,
        legacy_priority,
        if matches_boss_tax {
            format!(
                "strategic evaluation: verdict={:?} score={:.2}; matched boss tax; legacy estimate {:?} retained as tie-breaker",
                strategic_decision.verdict, strategic_decision.score, legacy_priority
            )
        } else {
            format!(
            "strategic evaluation: verdict={:?} score={:.2}; legacy estimate {:?} retained as tie-breaker",
            strategic_decision.verdict, strategic_decision.score, legacy_priority
            )
        },
    )
}

fn strategic_decision_matches_boss_tax_v1(strategic_decision: &CompiledDecision) -> bool {
    strategic_decision
        .matched_pressure_kinds
        .iter()
        .any(|kind| matches!(kind, PressureKind::BossTax(_)))
}

fn purchase_tiebreaker(target: ShopPurchaseTargetV1) -> i32 {
    match target {
        ShopPurchaseTargetV1::Relic { .. } => 3,
        ShopPurchaseTargetV1::Potion { .. } => 2,
        ShopPurchaseTargetV1::Card { .. } => 1,
    }
}

fn plan_components_v1(
    context: &ShopDecisionContextV1,
    strategic_trace: &StrategicDecisionTrace,
    candidate_plan: &ShopPlanCandidateV1,
) -> Vec<ShopPlanComponentV1> {
    let mut components = Vec::new();
    for step in &candidate_plan.plan.steps {
        match *step {
            ShopPlanStepV1::RemoveCard { cost, .. } => {
                if cost > 0 {
                    components.push(component_v1(
                        ShopPlanComponentKindV1::GoldSpend,
                        cost as f32,
                        "shop purge spends gold",
                    ));
                }
                components.push(component_v1(
                    ShopPlanComponentKindV1::DeckCleanup,
                    1.0,
                    "shop purge removes a deck card",
                ));
            }
            ShopPlanStepV1::BuyCard { cost, .. } => {
                if cost > 0 {
                    components.push(component_v1(
                        ShopPlanComponentKindV1::GoldSpend,
                        cost as f32,
                        "card purchase spends gold",
                    ));
                }
                components.push(component_v1(
                    ShopPlanComponentKindV1::DeckBloatCost,
                    1.0,
                    "card purchase adds one deck card",
                ));
            }
            ShopPlanStepV1::BuyRelic { cost, .. } => {
                if cost > 0 {
                    components.push(component_v1(
                        ShopPlanComponentKindV1::GoldSpend,
                        cost as f32,
                        "relic purchase spends gold",
                    ));
                }
                components.push(component_v1(
                    ShopPlanComponentKindV1::RelicValue,
                    1.0,
                    "shop relic adds persistent power",
                ));
            }
            ShopPlanStepV1::BuyPotion { cost, .. } => {
                if cost > 0 {
                    components.push(component_v1(
                        ShopPlanComponentKindV1::GoldSpend,
                        cost as f32,
                        "potion purchase spends gold",
                    ));
                }
                components.push(component_v1(
                    ShopPlanComponentKindV1::PotionFill,
                    1.0,
                    "shop potion fills a potion slot",
                ));
            }
            ShopPlanStepV1::LeaveShop => components.push(component_v1(
                ShopPlanComponentKindV1::StopReason,
                1.0,
                "leave shop is a non-purchase plan",
            )),
        }
    }

    if let Some(priority) = candidate_plan.plan.legacy_priority {
        components.push(component_v1(
            ShopPlanComponentKindV1::LegacyEstimate,
            priority as f32,
            "legacy purchase estimate retained as an audit component",
        ));
    }
    if candidate_plan.role == ShopPlanCandidateRoleV1::PortfolioAlternative {
        components.push(component_v1(
            ShopPlanComponentKindV1::BranchExploration,
            1.0,
            "portfolio plan is retained for branch exploration",
        ));
    }
    if shop_plan_matches_pressure_v1(candidate_plan, strategic_trace, |kind| {
        matches!(kind, PressureKind::BossTax(_))
    }) {
        components.push(component_v1(
            ShopPlanComponentKindV1::BossAnswer,
            1.0,
            "typed strategic delta matches a current boss tax",
        ));
    }
    let hard_threat_is_near = matches!(
        context.visit.next_threat,
        ShopThreatWindowV1::EliteIn(0..=2) | ShopThreatWindowV1::BossIn(0..=4)
    );
    let plan_has_temporary_potion = candidate_plan
        .plan
        .steps
        .iter()
        .any(|step| matches!(step, ShopPlanStepV1::BuyPotion { .. }));
    let plan_covers_near_threat = plan_has_temporary_potion
        && shop_plan_matches_pressure_v1(
            candidate_plan,
            strategic_trace,
            immediate_combat_pressure_kind_v1,
        );
    if hard_threat_is_near && plan_covers_near_threat {
        components.push(component_v1(
            ShopPlanComponentKindV1::ImmediateThreatCoverage,
            1.0,
            "shop plan carries typed coverage for a near hard fight",
        ));
    }
    if candidate_plan.plan.total_gold_spent > 0
        && context.visit.maw_bank == ShopMawBankStateV1::LiveUnspent
        && !context.visit.spent_gold_in_visit
    {
        if let ShopFutureShopV1::VisibleIn(floors) = context.visit.future_shop {
            components.push(component_v1(
                ShopPlanComponentKindV1::MawBankOpportunityCost,
                floors as f32,
                "spending now ends typed Maw Bank income before a visible future shop",
            ));
        }
    }
    if components.is_empty() {
        components.push(component_v1(
            ShopPlanComponentKindV1::StopReason,
            1.0,
            "shop plan has no rollout/frontier purchase component",
        ));
    }
    components
}

fn shop_plan_matches_pressure_v1(
    candidate_plan: &ShopPlanCandidateV1,
    strategic_trace: &StrategicDecisionTrace,
    predicate: impl Fn(PressureKind) -> bool,
) -> bool {
    candidate_plan.plan.steps.iter().any(|step| {
        let candidate_id = step.strategic_candidate_id_v1();
        strategic_trace.compiled.iter().any(|decision| {
            decision.action.candidate_id() == candidate_id
                && decision
                    .matched_pressure_kinds
                    .iter()
                    .copied()
                    .any(|kind| predicate(kind))
        })
    })
}

fn component_v1(
    kind: ShopPlanComponentKindV1,
    amount: f32,
    reason: &'static str,
) -> ShopPlanComponentV1 {
    ShopPlanComponentV1 {
        kind,
        amount,
        reason: reason.to_string(),
    }
}
