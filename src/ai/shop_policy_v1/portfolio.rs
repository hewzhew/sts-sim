use super::types::{
    ShopDecisionContextV1, ShopPlanCandidateRoleV1, ShopPlanCandidateV1, ShopPlanKindV1,
    ShopPlanSourceV1, ShopPlanStepV1, ShopPlanV1, ShopPlanVerdictV1,
};

pub(super) fn evaluated_shop_portfolio_combo_plans_v1(
    context: &ShopDecisionContextV1,
    evaluated_candidates: &[ShopPlanCandidateV1],
    max_plans: usize,
) -> Vec<ShopPlanV1> {
    if max_plans == 0 {
        return Vec::new();
    }

    let options = evaluated_candidates
        .iter()
        .filter_map(evaluated_combo_option_v1)
        .collect::<Vec<_>>();
    if options.len() < 2 {
        return Vec::new();
    }

    best_shop_combo_plan_v1(&options, context.need.gold)
        .map(|combo| vec![combo.plan])
        .unwrap_or_default()
}

#[derive(Clone, Debug)]
struct EvaluatedShopComboOptionV1 {
    rank: i32,
    cost: i32,
    can_start_combo: bool,
    can_follow_combo: bool,
    can_continue_combo: bool,
    effect_kind: &'static str,
    plan: ShopPlanV1,
}

fn evaluated_combo_option_v1(
    candidate: &ShopPlanCandidateV1,
) -> Option<EvaluatedShopComboOptionV1> {
    if candidate.role != ShopPlanCandidateRoleV1::SingleAction
        || candidate.evaluation.verdict != ShopPlanVerdictV1::Allow
        || candidate.plan.steps.len() != 1
    {
        return None;
    }

    let step = candidate.plan.steps.first()?;
    let (effect_kind, can_start_combo, can_follow_combo, can_continue_combo) = match *step {
        ShopPlanStepV1::BuyCard { .. } => ("shop_buy_card", true, true, true),
        ShopPlanStepV1::BuyPotion { .. } => ("shop_buy_potion", true, true, true),
        ShopPlanStepV1::BuyRelic { relic, .. } => (
            "shop_buy_relic",
            shop_relic_purchase_keeps_shop_open(relic),
            true,
            shop_relic_purchase_keeps_shop_open(relic),
        ),
        ShopPlanStepV1::RemoveCard { .. } => ("shop_purge", true, true, true),
        ShopPlanStepV1::LeaveShop => return None,
    };

    let mut plan = candidate.plan.clone();
    plan.plan_id = format!("portfolio:{}", candidate.plan.plan_id);
    plan.source = ShopPlanSourceV1::PortfolioCandidate;
    plan.legacy_priority = None;
    plan.reason = format!(
        "evaluated shop portfolio option from {}",
        candidate.plan.plan_id
    );

    Some(EvaluatedShopComboOptionV1 {
        rank: evaluated_candidate_rank_v1(candidate),
        cost: candidate.plan.total_gold_spent,
        can_start_combo,
        can_follow_combo,
        can_continue_combo,
        effect_kind,
        plan,
    })
}

fn evaluated_candidate_rank_v1(candidate: &ShopPlanCandidateV1) -> i32 {
    let component_rank = (candidate.evaluation.component_score.net * 100.0).round() as i32;
    candidate
        .evaluation
        .tier
        .saturating_mul(1_000)
        .saturating_add(candidate.evaluation.score.max(0))
        .saturating_add(component_rank)
}

fn best_shop_combo_plan_v1(
    options: &[EvaluatedShopComboOptionV1],
    gold: i32,
) -> Option<EvaluatedShopComboOptionV1> {
    let mut best = None::<EvaluatedShopComboOptionV1>;
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
                .is_none_or(|current| candidate.rank > current.rank)
            {
                best = Some(candidate);
            }
        }
    }
    if gold >= 300 {
        for first in options.iter().filter(|entry| entry.can_start_combo) {
            for second in options.iter().filter(|entry| {
                entry.can_follow_combo
                    && entry.can_continue_combo
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
                        .is_none_or(|current| candidate.rank > current.rank)
                    {
                        best = Some(candidate);
                    }
                }
            }
        }
    }
    best
}

fn shop_combo_plan_v1(entries: &[&EvaluatedShopComboOptionV1]) -> EvaluatedShopComboOptionV1 {
    let rank = entries
        .iter()
        .map(|entry| entry.rank)
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
        plan_id: format!("portfolio:combo:{}", candidate_ids.join("+")),
        label: label.clone(),
        kind: ShopPlanKindV1::Execute,
        steps,
        total_gold_spent: cost,
        candidate_ids,
        source: ShopPlanSourceV1::PortfolioCandidate,
        legacy_priority: None,
        legacy_confidence: None,
        suppressed_count: 0,
        reason: format!(
            "evaluated shop portfolio combo: {}",
            label.replace(" + ", " then ")
        ),
    };
    EvaluatedShopComboOptionV1 {
        rank,
        cost,
        can_start_combo: false,
        can_follow_combo: false,
        can_continue_combo: false,
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
