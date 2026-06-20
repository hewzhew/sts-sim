use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, compile_shop_decision_v1, ShopCompileModeV1,
    ShopPlanEvaluationV1, ShopPlanStepV1, ShopPlanV1, ShopPolicyConfigV1,
};
use crate::content::cards::{get_card_definition, CardId};
use crate::content::potions::get_potion_definition;
use crate::eval::branch_experiment::{
    BranchExperimentChoiceDecisionSignalV1,
    BRANCH_EXPERIMENT_SHOP_BRANCH_PROJECTION_SIGNAL_SOURCE_V1,
};
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;

const MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH: usize = 4;
const SHOP_COMMAND_SEQUENCE_SEPARATOR: &str = " && ";

#[derive(Clone, Debug)]
pub(crate) struct ShopBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) card: Option<CardId>,
    pub(crate) effect_kind: String,
    pub(crate) effect_label: String,
    pub(crate) representative_count: usize,
    pub(crate) suppressed_count: usize,
    pub(crate) decision_signal: Option<BranchExperimentChoiceDecisionSignalV1>,
}

pub(crate) fn shop_branch_options(session: &RunControlSession) -> Option<Vec<ShopBranchOption>> {
    let EngineState::Shop(shop) = &session.engine_state else {
        return None;
    };
    if shop.pending_reward_overlay.is_some() {
        return None;
    }

    let context = build_shop_decision_context_v1(&session.run_state, shop);
    let compiled = compile_shop_decision_v1(
        &context,
        &ShopPolicyConfigV1::default(),
        ShopCompileModeV1::BranchTopK {
            max_plans: MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH,
        },
    );
    let mut options = Vec::new();
    let mut seen_commands = std::collections::BTreeSet::<String>::new();

    for projection in &compiled.branch_frontier {
        if options.len() >= MAX_SHOP_PURCHASE_OPTIONS_PER_BRANCH {
            break;
        }
        let Some(candidate) = compiled
            .candidate_plans
            .iter()
            .find(|candidate| candidate.plan.plan_id == projection.plan_id)
        else {
            continue;
        };
        if let Some(option) =
            shop_branch_option_from_plan(&candidate.plan, Some(&candidate.evaluation))
        {
            if seen_commands.insert(option.command.clone()) {
                options.push(option);
            }
        }
    }
    if options.is_empty() {
        let evaluation = compiled
            .candidate_plans
            .iter()
            .find(|candidate| candidate.plan.plan_id == compiled.selected_plan.plan_id)
            .map(|candidate| &candidate.evaluation);
        if let Some(option) = shop_branch_option_from_plan(&compiled.selected_plan, evaluation) {
            options.push(option);
        }
    }
    debug_assert!(
        !options.is_empty(),
        "shop compiler should expose an executable branch option, usually LeaveShop"
    );
    Some(options)
}

fn shop_branch_option_from_plan(
    plan: &ShopPlanV1,
    evaluation: Option<&ShopPlanEvaluationV1>,
) -> Option<ShopBranchOption> {
    if plan.steps.is_empty() {
        return None;
    }
    Some(ShopBranchOption {
        label: plan.label.clone(),
        command: shop_plan_command(plan),
        card: shop_plan_card(plan),
        effect_kind: shop_plan_effect_kind(plan).to_string(),
        effect_label: shop_plan_effect_label(plan),
        representative_count: plan.steps.len(),
        suppressed_count: plan.suppressed_count,
        decision_signal: evaluation.map(shop_decision_signal_v1),
    })
}

fn shop_decision_signal_v1(
    evaluation: &ShopPlanEvaluationV1,
) -> BranchExperimentChoiceDecisionSignalV1 {
    BranchExperimentChoiceDecisionSignalV1 {
        source: BRANCH_EXPERIMENT_SHOP_BRANCH_PROJECTION_SIGNAL_SOURCE_V1.to_string(),
        verdict: format!("{:?}", evaluation.verdict),
        tier: evaluation.tier,
        score: evaluation.score,
        confidence_milli: (evaluation.confidence * 1000.0).round() as i32,
        component_net_rank: evaluation.component_score.net.round() as i32,
        preferred: false,
        acquisition_thesis_rank_adjustment: 0,
        acquisition_thesis_summary: Vec::new(),
    }
}

fn shop_plan_command(plan: &ShopPlanV1) -> String {
    plan.steps
        .iter()
        .map(shop_plan_step_command)
        .collect::<Vec<_>>()
        .join(SHOP_COMMAND_SEQUENCE_SEPARATOR)
}

fn shop_plan_step_command(step: &ShopPlanStepV1) -> String {
    match *step {
        ShopPlanStepV1::BuyCard { index, .. } => format!("buy card {index}"),
        ShopPlanStepV1::BuyRelic { index, .. } => format!("buy relic {index}"),
        ShopPlanStepV1::BuyPotion { index, .. } => format!("buy potion {index}"),
        ShopPlanStepV1::RemoveCard { deck_index, .. } => format!("purge {deck_index}"),
        ShopPlanStepV1::LeaveShop => "leave".to_string(),
    }
}

fn shop_plan_card(plan: &ShopPlanV1) -> Option<CardId> {
    plan.steps.iter().find_map(|step| match *step {
        ShopPlanStepV1::BuyCard { card, .. } | ShopPlanStepV1::RemoveCard { card, .. } => {
            Some(card)
        }
        _ => None,
    })
}

fn shop_plan_effect_kind(plan: &ShopPlanV1) -> &'static str {
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

fn shop_plan_effect_label(plan: &ShopPlanV1) -> String {
    let step_labels = plan
        .steps
        .iter()
        .map(shop_plan_step_label)
        .collect::<Vec<_>>()
        .join(" then ");
    let mut label = if step_labels.is_empty() {
        plan.label.clone()
    } else {
        format!("{step_labels} | total {} gold", plan.total_gold_spent)
    };
    if let Some(priority) = plan.legacy_priority {
        label.push_str(&format!(" | shop_legacy_estimate={priority}"));
    }
    label.push_str(&format!(" | source={:?}", plan.source));
    if plan.suppressed_count > 0 {
        label.push_str(&format!(
            " | shop portfolio cap suppressed {} plan(s)",
            plan.suppressed_count
        ));
    }
    label
}

fn shop_plan_step_label(step: &ShopPlanStepV1) -> String {
    match *step {
        ShopPlanStepV1::BuyCard { card, cost, .. } => {
            format!("Buy {} | {cost} gold", get_card_definition(card).name)
        }
        ShopPlanStepV1::BuyRelic { relic, cost, .. } => format!("Buy {relic:?} | {cost} gold"),
        ShopPlanStepV1::BuyPotion { potion, cost, .. } => {
            format!(
                "Buy {} potion | {cost} gold",
                get_potion_definition(potion).name
            )
        }
        ShopPlanStepV1::RemoveCard { card, cost, .. } => {
            format!("Purge {} | {cost} gold", get_card_definition(card).name)
        }
        ShopPlanStepV1::LeaveShop => "Leave shop".to_string(),
    }
}
