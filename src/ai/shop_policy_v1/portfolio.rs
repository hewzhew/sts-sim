use crate::ai::noncombat_strategy_v1::StrategyPlanSupportV1;

use super::compiler::single_candidate_plan_v1;
use super::types::{
    ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopPlanKindV1, ShopPlanSourceV1, ShopPlanV1,
    ShopPolicyClassV1, ShopPurchaseTargetV1,
};

pub(super) fn legacy_shop_portfolio_plans_v1(
    context: &ShopDecisionContextV1,
    max_plans: usize,
) -> Vec<ShopPlanV1> {
    if max_plans == 0 {
        return Vec::new();
    }

    let mut options = context
        .candidates
        .iter()
        .filter_map(scored_shop_plan_candidate_v1)
        .collect::<Vec<_>>();

    if options.is_empty() {
        return context
            .candidates
            .iter()
            .find(|candidate| candidate.class == ShopPolicyClassV1::Leave)
            .and_then(|candidate| {
                single_candidate_plan_v1(candidate, ShopPlanSourceV1::PortfolioCandidate)
            })
            .into_iter()
            .collect();
    }

    options.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.plan.plan_id.cmp(&right.plan.plan_id))
    });

    let combo_pressure = options.len() >= 3
        && (options.len() > max_plans || (context.need.gold >= 300 && context.conversion_pressure));
    let combo = combo_pressure
        .then(|| best_shop_combo_plan_v1(&options, context.need.gold))
        .flatten();
    if options.len() <= max_plans && combo.is_none() {
        return options.into_iter().map(|entry| entry.plan).collect();
    }

    let mut selected = Vec::<ShopPlanV1>::new();
    let mut represented = std::collections::BTreeSet::<String>::new();
    if let Some(combo) = combo {
        represented.insert(combo.plan.plan_id.clone());
        selected.push(combo.plan);
    }

    for effect_kind in [
        "shop_buy_relic",
        "shop_buy_card",
        "shop_buy_potion",
        "shop_purge",
    ] {
        if selected.len() >= max_plans {
            break;
        }
        if let Some(entry) = options.iter().find(|entry| {
            entry.effect_kind == effect_kind && !represented.contains(&entry.plan.plan_id)
        }) {
            represented.insert(entry.plan.plan_id.clone());
            selected.push(entry.plan.clone());
        }
    }
    for entry in &options {
        if selected.len() >= max_plans {
            break;
        }
        if represented.insert(entry.plan.plan_id.clone()) {
            selected.push(entry.plan.clone());
        }
    }

    let suppressed_count = options.len().saturating_sub(represented.len());
    if suppressed_count > 0 {
        if let Some(plan) = selected.first_mut() {
            plan.suppressed_count = suppressed_count;
            plan.reason = format!(
                "{} | shop portfolio cap suppressed {suppressed_count} affordable plan(s)",
                plan.reason
            );
        }
    }
    selected
}

#[derive(Clone, Debug)]
struct ScoredShopPlanCandidateV1 {
    score: i32,
    cost: i32,
    can_start_combo: bool,
    can_follow_combo: bool,
    effect_kind: &'static str,
    plan: ShopPlanV1,
}

fn scored_shop_plan_candidate_v1(
    candidate: &ShopCandidateEvidenceV1,
) -> Option<ScoredShopPlanCandidateV1> {
    if candidate.support_gate != StrategyPlanSupportV1::Strong {
        return None;
    }
    let mut plan = single_candidate_plan_v1(candidate, ShopPlanSourceV1::PortfolioCandidate)?;
    plan.plan_id = format!("legacy_portfolio:{}", candidate.candidate_id);
    let (score, effect_kind, can_start_combo, can_follow_combo) = match candidate.class {
        ShopPolicyClassV1::CursePurge => (1000, "shop_purge", false, false),
        ShopPolicyClassV1::StarterStrikePurge => (700, "shop_purge", false, false),
        ShopPolicyClassV1::PurchaseOpportunity => {
            let priority = candidate.purchase_priority.unwrap_or_default();
            if priority <= 0 {
                return None;
            }
            match candidate.purchase_target? {
                ShopPurchaseTargetV1::Card { .. } => (priority, "shop_buy_card", true, true),
                ShopPurchaseTargetV1::Relic { relic, .. } => (
                    priority,
                    "shop_buy_relic",
                    shop_relic_purchase_keeps_shop_open(relic),
                    false,
                ),
                ShopPurchaseTargetV1::Potion { .. } => (priority, "shop_buy_potion", true, true),
            }
        }
        ShopPolicyClassV1::Leave => return None,
        ShopPolicyClassV1::Unknown => return None,
    };
    plan.legacy_priority = Some(score);
    Some(ScoredShopPlanCandidateV1 {
        score,
        cost: candidate.gold_cost.unwrap_or_default(),
        can_start_combo,
        can_follow_combo,
        effect_kind,
        plan,
    })
}

fn best_shop_combo_plan_v1(
    options: &[ScoredShopPlanCandidateV1],
    gold: i32,
) -> Option<ScoredShopPlanCandidateV1> {
    let mut best = None::<ScoredShopPlanCandidateV1>;
    for first in options.iter().filter(|entry| entry.can_start_combo) {
        for second in options.iter().filter(|entry| {
            entry.can_follow_combo
                && entry.plan.plan_id != first.plan.plan_id
                && entry.effect_kind != first.effect_kind
        }) {
            if first.cost.saturating_add(second.cost) > gold {
                continue;
            }
            let candidate = shop_combo_plan_v1(&[first, second]);
            if best
                .as_ref()
                .is_none_or(|current| candidate.score > current.score)
            {
                best = Some(candidate);
            }
        }
    }
    if gold >= 300 {
        for first in options.iter().filter(|entry| entry.can_start_combo) {
            for second in options.iter().filter(|entry| {
                entry.can_follow_combo
                    && entry.plan.plan_id != first.plan.plan_id
                    && entry.effect_kind != first.effect_kind
            }) {
                for third in options.iter().filter(|entry| {
                    entry.can_follow_combo
                        && entry.plan.plan_id != first.plan.plan_id
                        && entry.plan.plan_id != second.plan.plan_id
                        && entry.effect_kind != first.effect_kind
                        && entry.effect_kind != second.effect_kind
                }) {
                    if first
                        .cost
                        .saturating_add(second.cost)
                        .saturating_add(third.cost)
                        > gold
                    {
                        continue;
                    }
                    let candidate = shop_combo_plan_v1(&[first, second, third]);
                    if best
                        .as_ref()
                        .is_none_or(|current| candidate.score > current.score)
                    {
                        best = Some(candidate);
                    }
                }
            }
        }
    }
    best
}

fn shop_combo_plan_v1(entries: &[&ScoredShopPlanCandidateV1]) -> ScoredShopPlanCandidateV1 {
    let score = entries
        .iter()
        .map(|entry| entry.score)
        .sum::<i32>()
        .saturating_add((entries.len() as i32).saturating_sub(1) * 100);
    let cost = entries.iter().map(|entry| entry.cost).sum::<i32>();
    let label = entries
        .iter()
        .map(|entry| entry.plan.label.as_str())
        .collect::<Vec<_>>()
        .join(" + ");
    let mut steps = Vec::new();
    let mut candidate_ids = Vec::new();
    for entry in entries {
        steps.extend(entry.plan.steps.clone());
        candidate_ids.extend(entry.plan.candidate_ids.clone());
    }
    let plan = ShopPlanV1 {
        plan_id: format!("legacy:combo:{}", candidate_ids.join("+")),
        label: label.clone(),
        kind: ShopPlanKindV1::Execute,
        steps,
        total_gold_spent: cost,
        candidate_ids,
        source: ShopPlanSourceV1::PortfolioCandidate,
        legacy_priority: Some(score),
        legacy_confidence: None,
        suppressed_count: 0,
        reason: format!(
            "legacy shop portfolio combo: {}",
            label.replace(" + ", " then ")
        ),
    };
    ScoredShopPlanCandidateV1 {
        score,
        cost,
        can_start_combo: false,
        can_follow_combo: false,
        effect_kind: "shop_buy_combo",
        plan,
    }
}

fn shop_relic_purchase_keeps_shop_open(relic: crate::content::relics::RelicId) -> bool {
    !matches!(
        relic,
        crate::content::relics::RelicId::Orrery
            | crate::content::relics::RelicId::DollysMirror
            | crate::content::relics::RelicId::BottledFlame
            | crate::content::relics::RelicId::BottledLightning
            | crate::content::relics::RelicId::BottledTornado
            | crate::content::relics::RelicId::Cauldron
    )
}
