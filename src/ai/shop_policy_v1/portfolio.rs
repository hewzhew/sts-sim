use super::types::{
    ShopDecisionContextV1, ShopPlanCandidateRoleV1, ShopPlanCandidateV1, ShopPlanKindV1,
    ShopPlanSourceV1, ShopPlanStepV1, ShopPlanV1,
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

    best_shop_combo_plans_v1(&options, context.need.gold, max_plans)
        .into_iter()
        .map(|combo| combo.plan)
        .collect()
}

#[derive(Clone, Debug)]
struct EvaluatedShopComboOptionV1 {
    rank: i32,
    cost: i32,
    execution_phase: ShopPlanExecutionPhaseV1,
    can_start_combo: bool,
    can_follow_combo: bool,
    can_continue_combo: bool,
    effect_kind: ShopPlanEffectKindV1,
    plan: ShopPlanV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShopPlanEffectKindV1 {
    BuyCard,
    BuyPotion,
    BuyRelic,
    Purge,
    Combo,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ShopPlanExecutionPhaseV1 {
    PurchaseBenefit,
    DeckCleanup,
    ShopClosingInteraction,
}

fn evaluated_combo_option_v1(
    candidate: &ShopPlanCandidateV1,
) -> Option<EvaluatedShopComboOptionV1> {
    if candidate.role != ShopPlanCandidateRoleV1::SingleAction
        || !candidate.evaluation.branch_admission.is_admitted()
        || candidate.plan.steps.len() != 1
    {
        return None;
    }

    let step = candidate.plan.steps.first()?;
    let (effect_kind, execution_phase, can_start_combo, can_follow_combo, can_continue_combo) =
        match *step {
            ShopPlanStepV1::BuyCard { .. } => (
                ShopPlanEffectKindV1::BuyCard,
                ShopPlanExecutionPhaseV1::PurchaseBenefit,
                true,
                true,
                true,
            ),
            ShopPlanStepV1::BuyPotion { .. } => (
                ShopPlanEffectKindV1::BuyPotion,
                ShopPlanExecutionPhaseV1::PurchaseBenefit,
                true,
                true,
                true,
            ),
            ShopPlanStepV1::BuyRelic { relic, .. } => (
                ShopPlanEffectKindV1::BuyRelic,
                if shop_relic_purchase_keeps_shop_open(relic) {
                    ShopPlanExecutionPhaseV1::PurchaseBenefit
                } else {
                    ShopPlanExecutionPhaseV1::ShopClosingInteraction
                },
                shop_relic_purchase_keeps_shop_open(relic),
                true,
                shop_relic_purchase_keeps_shop_open(relic),
            ),
            ShopPlanStepV1::RemoveCard { .. } => (
                ShopPlanEffectKindV1::Purge,
                ShopPlanExecutionPhaseV1::DeckCleanup,
                true,
                true,
                true,
            ),
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
        execution_phase,
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

fn best_shop_combo_plans_v1(
    options: &[EvaluatedShopComboOptionV1],
    gold: i32,
    max_plans: usize,
) -> Vec<EvaluatedShopComboOptionV1> {
    let mut candidates = std::collections::BTreeMap::<String, EvaluatedShopComboOptionV1>::new();
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
            insert_shop_combo_candidate_v1(&mut candidates, candidate);
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
                    insert_shop_combo_candidate_v1(&mut candidates, candidate);
                }
            }
        }
    }
    let mut candidates = candidates.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .rank
            .cmp(&left.rank)
            .then_with(|| left.plan.plan_id.cmp(&right.plan.plan_id))
    });
    candidates.truncate(max_plans);
    candidates
}

fn insert_shop_combo_candidate_v1(
    candidates: &mut std::collections::BTreeMap<String, EvaluatedShopComboOptionV1>,
    candidate: EvaluatedShopComboOptionV1,
) {
    let mut candidate_ids = candidate.plan.candidate_ids.clone();
    candidate_ids.sort();
    let key = candidate_ids.join("+");
    if candidates
        .get(&key)
        .is_none_or(|existing| candidate.rank > existing.rank)
    {
        candidates.insert(key, candidate);
    }
}

fn shop_combo_plan_v1(entries: &[&EvaluatedShopComboOptionV1]) -> EvaluatedShopComboOptionV1 {
    let rank = entries
        .iter()
        .map(|entry| entry.rank)
        .sum::<i32>()
        .saturating_add((entries.len() as i32).saturating_sub(1) * 100);
    let cost = entries.iter().map(|entry| entry.cost).sum::<i32>();
    let mut entries = entries.to_vec();
    entries.sort_by(|left, right| {
        left.execution_phase
            .cmp(&right.execution_phase)
            .then_with(|| right.rank.cmp(&left.rank))
            .then_with(|| left.plan.plan_id.cmp(&right.plan.plan_id))
    });
    let label = entries
        .iter()
        .map(|entry| entry.plan.label.as_str())
        .collect::<Vec<_>>()
        .join(" + ");
    let mut steps = Vec::new();
    let mut candidate_ids = Vec::new();
    for entry in &entries {
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
        execution_phase: ShopPlanExecutionPhaseV1::ShopClosingInteraction,
        can_start_combo: false,
        can_follow_combo: false,
        can_continue_combo: false,
        effect_kind: ShopPlanEffectKindV1::Combo,
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
