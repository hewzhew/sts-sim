use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::content::cards::{get_card_definition, CardId, CardTag, CardType};
use crate::state::run::RunState;
use crate::state::shop::ShopState;

use super::certificates::certified_action;
use super::types::{
    purge_candidate_id, ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopDecisionV1,
    ShopPolicyActionV1, ShopPolicyClassV1, ShopPolicyConfigV1, ShopPurchaseTargetV1,
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
        candidates.extend(
            run_state
                .master_deck
                .iter()
                .enumerate()
                .filter(|(_, card)| purge_eligible(run_state, card))
                .map(|(deck_index, card)| {
                    purge_candidate_evidence(deck_index, card.id, shop.purge_cost, &strategy)
                }),
        );
    }

    candidates.extend(shop.cards.iter().enumerate().map(|(index, card)| {
        purchase_candidate_evidence(
            format!(
                "buy card {} for {} gold",
                get_card_definition(card.card_id).name,
                card.price
            ),
            card.can_buy && card.price <= run_state.gold,
            ShopPurchaseTargetV1::Card {
                index,
                card: card.card_id,
            },
            purchase_priority_with_strategy(
                ShopPurchaseTargetV1::Card {
                    index,
                    card: card.card_id,
                },
                crate::ai::shop_policy_v1::shop_card_conversion_priority_v1(
                    card.card_id,
                    run_state,
                ),
                &strategy,
            ),
        )
    }));
    candidates.extend(shop.relics.iter().enumerate().map(|(index, relic)| {
        purchase_candidate_evidence(
            format!("buy relic {:?} for {} gold", relic.relic_id, relic.price),
            relic.can_buy && relic.price <= run_state.gold,
            ShopPurchaseTargetV1::Relic {
                index,
                relic: relic.relic_id,
            },
            crate::ai::shop_policy_v1::shop_relic_conversion_priority_v1(relic.relic_id),
        )
    }));
    candidates.extend(shop.potions.iter().enumerate().map(|(index, potion)| {
        purchase_candidate_evidence(
            format!(
                "buy potion {:?} for {} gold",
                potion.potion_id, potion.price
            ),
            potion.can_buy && potion.price <= run_state.gold,
            ShopPurchaseTargetV1::Potion {
                index,
                potion: potion.potion_id,
            },
            purchase_priority_with_strategy(
                ShopPurchaseTargetV1::Potion {
                    index,
                    potion: potion.potion_id,
                },
                crate::ai::shop_policy_v1::shop_potion_conversion_priority_for_v1(
                    potion.potion_id,
                    run_state,
                ),
                &strategy,
            ),
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

pub fn plan_shop_decision_v1(
    context: &ShopDecisionContextV1,
    config: &ShopPolicyConfigV1,
) -> ShopDecisionV1 {
    let action = certified_action(context, config).unwrap_or_else(|| ShopPolicyActionV1::Stop {
        reason: stop_reason(context),
    });

    ShopDecisionV1 {
        action,
        label_role: "behavior_policy_not_teacher",
        context: context.clone(),
    }
}

fn purge_candidate_evidence(
    deck_index: usize,
    card: CardId,
    purge_cost: i32,
    strategy: &crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2,
) -> ShopCandidateEvidenceV1 {
    let class = purge_class(card);
    let support_gate = purge_support_gate(class, strategy);
    let card_name = get_card_definition(card).name;
    let mut evidence = vec![
        format!("deck index {deck_index} is purge eligible"),
        format!("purge cost={purge_cost}"),
    ];
    let mut risks = Vec::new();
    match class {
        ShopPolicyClassV1::CursePurge => {
            evidence.push("card is a curse".to_string());
        }
        ShopPolicyClassV1::StarterStrikePurge => {
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
            risks.push("shop policy has no purge certificate for this card".to_string());
        }
    }

    ShopCandidateEvidenceV1 {
        candidate_id: purge_candidate_id(deck_index),
        label: format!("purge {card_name}"),
        class,
        deck_index: Some(deck_index),
        card: Some(card),
        purchase_target: None,
        purchase_priority: None,
        support_gate,
        evidence,
        risks,
    }
}

fn purchase_candidate_evidence(
    label: String,
    can_buy: bool,
    target: ShopPurchaseTargetV1,
    priority: i32,
) -> ShopCandidateEvidenceV1 {
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
        support_gate: if can_buy {
            StrategyPlanSupportV1::Strong
        } else {
            StrategyPlanSupportV1::Blocked
        },
        evidence: vec![format!("can_buy={can_buy}"), format!("priority={priority}")],
        risks: if can_buy {
            vec!["purchase must clear high-impact priority gate".to_string()]
        } else {
            Vec::new()
        },
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

fn purge_class(card: CardId) -> ShopPolicyClassV1 {
    let definition = get_card_definition(card);
    if definition.card_type == CardType::Curse {
        ShopPolicyClassV1::CursePurge
    } else if definition.tags.contains(&CardTag::StarterStrike) {
        ShopPolicyClassV1::StarterStrikePurge
    } else {
        ShopPolicyClassV1::Unknown
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

fn purge_eligible(run_state: &RunState, card: &crate::runtime::combat::CombatCard) -> bool {
    crate::state::core::master_deck_card_is_purgeable(card)
        && !crate::state::core::master_deck_card_is_bottled(card, &run_state.relics)
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

fn stop_reason(context: &ShopDecisionContextV1) -> String {
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
    format!("shop policy stopped because no conservative purge certificate matched ({classes})")
}
