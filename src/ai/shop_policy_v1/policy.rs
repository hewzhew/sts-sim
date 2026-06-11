use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPlanSupportV1,
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
    let affordable_purchase_exists = affordable_purchase_exists(shop, run_state.gold);
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
            crate::ai::shop_policy_v1::shop_card_conversion_priority_v1(card.card_id, run_state),
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
            crate::ai::shop_policy_v1::shop_potion_conversion_priority_for_v1(
                potion.potion_id,
                run_state,
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
        evidence: vec!["leaving shop is always available".to_string()],
        risks: Vec::new(),
    });

    ShopDecisionContextV1 {
        strategy,
        candidates,
        affordable_purchase_exists,
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

fn stop_reason(context: &ShopDecisionContextV1) -> String {
    let classes = context
        .candidates
        .iter()
        .map(|candidate| format!("{}:{:?}", candidate.label, candidate.class))
        .collect::<Vec<_>>()
        .join(", ");
    format!("shop policy stopped because no conservative purge certificate matched ({classes})")
}
