use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::content::cards::get_card_definition;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;
use crate::state::shop::ShopState;

use super::compiler::{compile_shop_decision_v1, shop_policy_action_from_plan_v1};
use super::strategy_tags::shop_purchase_strategy_analysis_v1;
use super::types::{
    purge_candidate_id, ShopCandidateEvidenceV1, ShopCompileModeV1, ShopDecisionContextV1,
    ShopDecisionV1, ShopPolicyActionV1, ShopPolicyClassV1, ShopPolicyConfigV1,
    ShopPurchaseTargetV1,
};
use crate::ai::decision_tags_v1::TAG_DECK_CLEANING;
use crate::ai::deck_mutation_compiler_v1::{
    compile_deck_mutation_decision_v1, DeckMutationCompilerModeV1, DeckMutationKindV1,
    DeckMutationPlanCandidateV1, DeckMutationPlanRoleV1, DeckMutationTargetClassV1,
};

pub fn build_shop_decision_context_v1(
    run_state: &RunState,
    shop: &ShopState,
) -> ShopDecisionContextV1 {
    let strategy = build_run_strategy_snapshot_from_run_state_v2(run_state);
    let need = crate::ai::shop_policy_v1::build_shop_need_profile_v1(run_state);
    let affordable_purchase_exists = affordable_purchase_exists(shop, run_state.gold);
    let conversion_pressure =
        crate::ai::shop_policy_v1::shop_conversion_pressure_v1(run_state, shop);
    let mut candidates = Vec::new();

    if shop.purge_available && run_state.gold >= shop.purge_cost {
        candidates.extend(shop_purge_candidates_from_deck_mutation_compiler_v1(
            run_state, shop, &strategy,
        ));
    }

    candidates.extend(shop.cards.iter().enumerate().map(|(index, card)| {
        let target = ShopPurchaseTargetV1::Card {
            index,
            card: card.card_id,
        };
        let base_priority =
            crate::ai::shop_policy_v1::shop_card_conversion_priority_v1(card.card_id, run_state);
        let analysis = shop_purchase_strategy_analysis_v1(target, run_state, &strategy);
        let priority = purchase_priority_with_strategy(target, base_priority, &strategy);
        purchase_candidate_evidence(
            format!(
                "buy card {} for {} gold",
                get_card_definition(card.card_id).name,
                card.price
            ),
            card.can_buy && card.price <= run_state.gold,
            target,
            priority,
            card.price,
            analysis.evidence,
            analysis.risks,
        )
    }));
    candidates.extend(shop.relics.iter().enumerate().map(|(index, relic)| {
        let target = ShopPurchaseTargetV1::Relic {
            index,
            relic: relic.relic_id,
        };
        let analysis = shop_purchase_strategy_analysis_v1(target, run_state, &strategy);
        purchase_candidate_evidence(
            format!("buy relic {:?} for {} gold", relic.relic_id, relic.price),
            relic.can_buy && relic.price <= run_state.gold,
            target,
            crate::ai::shop_policy_v1::shop_relic_conversion_priority_v1(relic.relic_id),
            relic.price,
            analysis.evidence,
            analysis.risks,
        )
    }));
    candidates.extend(shop.potions.iter().enumerate().map(|(index, potion)| {
        let target = ShopPurchaseTargetV1::Potion {
            index,
            potion: potion.potion_id,
        };
        let analysis = shop_purchase_strategy_analysis_v1(target, run_state, &strategy);
        let potion_can_buy = shop_potion_purchase_block_reason_v1(run_state, potion).is_none();
        purchase_candidate_evidence(
            format!(
                "buy potion {:?} for {} gold",
                potion.potion_id, potion.price
            ),
            potion_can_buy,
            target,
            purchase_priority_with_strategy(
                target,
                crate::ai::shop_policy_v1::shop_potion_conversion_priority_for_v1(
                    potion.potion_id,
                    run_state,
                ),
                &strategy,
            ),
            potion.price,
            analysis.evidence,
            analysis.risks,
        )
    }));
    candidates.push(ShopCandidateEvidenceV1 {
        candidate_id: "shop:leave".to_string(),
        label: "leave shop".to_string(),
        class: ShopPolicyClassV1::Leave,
        deck_index: None,
        card: None,
        purchase_target: None,
        purchase_priority: None,
        gold_cost: None,
        support_gate: StrategyPlanSupportV1::Strong,
        evidence: leave_shop_evidence(&need, conversion_pressure),
        risks: leave_shop_risks(&need, conversion_pressure, affordable_purchase_exists),
    });

    ShopDecisionContextV1 {
        strategy,
        need,
        candidates,
        affordable_purchase_exists,
        conversion_pressure,
    }
}

pub fn shop_potion_purchase_is_allowed_v1(
    run_state: &RunState,
    potion: &crate::state::shop::ShopPotion,
) -> bool {
    shop_potion_purchase_block_reason_v1(run_state, potion).is_none()
}

pub fn shop_potion_purchase_block_reason_v1(
    run_state: &RunState,
    potion: &crate::state::shop::ShopPotion,
) -> Option<String> {
    if !potion.can_buy {
        return Some(
            potion
                .blocked_reason
                .clone()
                .unwrap_or_else(|| "cannot buy".to_string()),
        );
    }
    if run_state.gold < potion.price {
        return Some("not enough gold".to_string());
    }
    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == crate::content::relics::RelicId::Sozu)
    {
        return Some("blocked by Sozu".to_string());
    }
    if run_state.find_empty_potion_slot().is_none() {
        return Some("no empty potion slot".to_string());
    }
    None
}

pub fn plan_shop_decision_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
) -> ShopDecisionV1 {
    let compiled = compile_shop_decision_v1(context, config, ShopCompileModeV1::ExecuteOne);
    let strategic_trace = compiled.strategic_trace.clone();
    let action = shop_policy_action_from_plan_v1(&compiled.selected_plan).unwrap_or_else(|| {
        ShopPolicyActionV1::Stop {
            reason: compiled.selected_plan.reason.clone(),
        }
    });

    ShopDecisionV1 {
        action,
        label_role: "behavior_policy_not_teacher",
        strategic_trace,
        context: context.clone(),
    }
}

fn purge_candidate_evidence(
    plan: &DeckMutationPlanCandidateV1,
    purge_cost: i32,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> Option<ShopCandidateEvidenceV1> {
    if plan.step.kind != DeckMutationKindV1::Remove || plan.step.cards.len() != 1 {
        return None;
    }
    let card_snapshot = plan.step.cards.first()?;
    let deck_index = card_snapshot.deck_index;
    let card = card_snapshot.card;
    let class = purge_class_from_deck_mutation_target(card_snapshot.target_class);
    let support_gate = purge_support_gate(class, strategy);
    let card_name = get_card_definition(card).name;
    let mut evidence = vec![
        format!("DeckMutationCompilerV1 plan_id={}", plan.plan_id),
        format!("deck mutation role={:?}", plan.role),
        format!(
            "deck mutation allowed execute={} branch={} inspect={}",
            plan.allowed_consumers.execute_autopilot,
            plan.allowed_consumers.branch_active,
            plan.allowed_consumers.inspect
        ),
        format!(
            "deck mutation target_class={:?}",
            card_snapshot.target_class
        ),
        format!("deck mutation effect={}", plan.step.effect_label),
        format!(
            "deck mutation representative_count={} suppressed_count={}",
            plan.representative_count, plan.suppressed_count
        ),
        format!("purge cost={purge_cost}"),
    ];
    evidence.extend(plan.reasons.iter().cloned());
    let mut risks = Vec::new();
    risks.extend(plan.risks.iter().cloned());
    if matches!(
        plan.role,
        DeckMutationPlanRoleV1::InspectOnly | DeckMutationPlanRoleV1::Blocked
    ) {
        risks.push(format!(
            "deck mutation compiler did not approve this target for automatic execution: {:?}",
            plan.role
        ));
    }
    match class {
        ShopPolicyClassV1::CursePurge => {
            evidence.push(TAG_DECK_CLEANING.to_string());
            evidence.push("card is a curse".to_string());
        }
        ShopPolicyClassV1::StarterStrikePurge => {
            evidence.push(TAG_DECK_CLEANING.to_string());
            evidence.push(format!(
                "CorePlanProtection support is {:?}",
                strategy.support(
                    crate::ai::noncombat_strategy_v1::StrategyPackageIdV2::CorePlanProtection
                )
            ));
            evidence.push(format!(
                "CombatPatchWindow support is {:?}",
                strategy.support(
                    crate::ai::noncombat_strategy_v1::StrategyPackageIdV2::CombatPatchWindow
                )
            ));
            if support_gate != StrategyPlanSupportV1::Strong {
                risks.push(
                    "starter strike purge is blocked by current strategy packages".to_string(),
                );
            }
        }
        _ => {
            risks.push("shop policy has no purge approval for this card".to_string());
        }
    }

    Some(ShopCandidateEvidenceV1 {
        candidate_id: purge_candidate_id(deck_index),
        label: format!("purge {card_name}"),
        class,
        deck_index: Some(deck_index),
        card: Some(card),
        purchase_target: None,
        purchase_priority: None,
        gold_cost: Some(purge_cost),
        support_gate,
        evidence,
        risks,
    })
}

fn purchase_candidate_evidence(
    label: String,
    can_buy: bool,
    target: ShopPurchaseTargetV1,
    priority: i32,
    price: i32,
    extra_evidence: Vec<String>,
    extra_risks: Vec<String>,
) -> ShopCandidateEvidenceV1 {
    let mut evidence = vec![
        format!("can_buy={can_buy}"),
        format!("legacy_priority={priority}"),
    ];
    evidence.extend(extra_evidence);
    let mut risks = if can_buy {
        vec!["purchase must clear compiled shop plan evaluation".to_string()]
    } else {
        Vec::new()
    };
    risks.extend(extra_risks);

    ShopCandidateEvidenceV1 {
        candidate_id: super::types::purchase_candidate_id(target),
        label,
        class: ShopPolicyClassV1::PurchaseOpportunity,
        deck_index: None,
        card: match target {
            ShopPurchaseTargetV1::Card { card, .. } => Some(card),
            _ => None,
        },
        purchase_target: Some(target),
        purchase_priority: Some(priority),
        gold_cost: Some(price),
        support_gate: if can_buy {
            StrategyPlanSupportV1::Strong
        } else {
            StrategyPlanSupportV1::Blocked
        },
        evidence,
        risks,
    }
}

fn purchase_priority_with_strategy(
    target: ShopPurchaseTargetV1,
    base_priority: i32,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> i32 {
    base_priority + combat_patch_purchase_bonus(target, strategy)
}

fn combat_patch_purchase_bonus(
    target: ShopPurchaseTargetV1,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> i32 {
    let support = strategy.support(StrategyPackageIdV2::CombatPatchWindow);
    let base_bonus = match support {
        StrategyPlanSupportV1::Strong => 320,
        StrategyPlanSupportV1::Plausible => 260,
        _ => return 0,
    };
    match target {
        ShopPurchaseTargetV1::Card { card, .. }
            if super::conversion::shop_card_is_combat_patch_v1(card) =>
        {
            base_bonus / 2
        }
        ShopPurchaseTargetV1::Potion { potion, .. }
            if super::conversion::shop_potion_is_combat_patch_v1(potion) =>
        {
            base_bonus
        }
        _ => 0,
    }
}

fn shop_purge_candidates_from_deck_mutation_compiler_v1(
    run_state: &RunState,
    shop: &ShopState,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> Vec<ShopCandidateEvidenceV1> {
    let choice = RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::PurgeNonBottled,
        return_state: Box::new(EngineState::Shop(shop.clone())),
    };
    let decision = compile_deck_mutation_decision_v1(
        run_state,
        &choice,
        DeckMutationCompilerModeV1::BranchTopK {
            max_active: usize::MAX,
        },
    );

    decision
        .candidate_plans
        .iter()
        .filter_map(|plan| purge_candidate_evidence(plan, shop.purge_cost, strategy))
        .collect()
}

fn purge_class_from_deck_mutation_target(
    target_class: DeckMutationTargetClassV1,
) -> ShopPolicyClassV1 {
    match target_class {
        DeckMutationTargetClassV1::Curse => ShopPolicyClassV1::CursePurge,
        DeckMutationTargetClassV1::StarterStrike => ShopPolicyClassV1::StarterStrikePurge,
        _ => ShopPolicyClassV1::Unknown,
    }
}

fn purge_support_gate(
    class: ShopPolicyClassV1,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> StrategyPlanSupportV1 {
    match class {
        ShopPolicyClassV1::CursePurge => StrategyPlanSupportV1::Strong,
        ShopPolicyClassV1::StarterStrikePurge => {
            let core_plan = strategy
                .support(crate::ai::noncombat_strategy_v1::StrategyPackageIdV2::CorePlanProtection);
            let patch_window = strategy
                .support(crate::ai::noncombat_strategy_v1::StrategyPackageIdV2::CombatPatchWindow);
            if core_plan == StrategyPlanSupportV1::Strong
                && !matches!(
                    patch_window,
                    StrategyPlanSupportV1::Strong | StrategyPlanSupportV1::Plausible
                )
            {
                StrategyPlanSupportV1::Strong
            } else {
                StrategyPlanSupportV1::Blocked
            }
        }
        _ => StrategyPlanSupportV1::Blocked,
    }
}

fn affordable_purchase_exists(shop: &ShopState, gold: i32) -> bool {
    shop.cards
        .iter()
        .any(|card| card.can_buy && card.price <= gold)
        || shop
            .relics
            .iter()
            .any(|relic| relic.can_buy && relic.price <= gold)
        || shop
            .potions
            .iter()
            .any(|potion| potion.can_buy && potion.price <= gold)
}

fn leave_shop_evidence(
    need: &crate::ai::shop_policy_v1::ShopNeedProfileV1,
    conversion_pressure: bool,
) -> Vec<String> {
    let mut evidence = vec![
        "leaving shop is always mechanically available".to_string(),
        format!("gold={}", need.gold),
        format!("floors_to_boss={}", need.floors_to_boss),
        format!("conversion_pressure={conversion_pressure}"),
    ];
    if need.near_boss {
        evidence.push("near act boss".to_string());
    }
    evidence
}

fn leave_shop_risks(
    need: &crate::ai::shop_policy_v1::ShopNeedProfileV1,
    conversion_pressure: bool,
    affordable_purchase_exists: bool,
) -> Vec<String> {
    let mut risks = Vec::new();
    if conversion_pressure && affordable_purchase_exists {
        risks.push("unconverted gold remains while affordable purchases exist".to_string());
    }
    if need.gold >= 300 {
        risks.push("gold >= 300 makes empty shop exit a severe policy risk".to_string());
    } else if need.gold >= 250 {
        risks.push("gold >= 250 requires an explicit leave reason".to_string());
    }
    if need.near_boss && need.gold >= 200 {
        risks.push("near boss with high gold should evaluate immediate conversion".to_string());
    }
    risks
}

pub(super) fn stop_reason(context: &ShopDecisionContextV1) -> String {
    let classes = context
        .candidates
        .iter()
        .map(|candidate| format!("{}:{:?}", candidate.label, candidate.class))
        .collect::<Vec<_>>()
        .join(", ");
    if context.conversion_pressure {
        return format!(
            "shop policy stopped despite conversion pressure gold={} floors_to_boss={} ({classes})",
            context.need.gold, context.need.floors_to_boss
        );
    }
    format!("shop policy stopped because no conservative purge approval matched ({classes})")
}
