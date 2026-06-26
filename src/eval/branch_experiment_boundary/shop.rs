use crate::ai::shop_policy_v1::{
    build_shop_decision_context_v1, compile_shop_decision_v1, ShopCompileModeV1,
    ShopPlanEvaluationV1, ShopPlanStepV1, ShopPlanV1, ShopPolicyConfigV1,
};
use crate::content::cards::{get_card_definition, CardId};
use crate::content::potions::get_potion_definition;
use crate::eval::branch_experiment::{
    BranchExperimentChoiceDecisionSignalV1, BranchExperimentShopPlanCandidateEntryV1,
    BranchExperimentShopPlanCandidatePoolV1,
    BRANCH_EXPERIMENT_SHOP_BRANCH_FRONTIER_SIGNAL_SOURCE_V1,
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
    pub(crate) candidate_axis: Option<String>,
    pub(crate) representative_count: usize,
    pub(crate) suppressed_count: usize,
    pub(crate) decision_signal: Option<BranchExperimentChoiceDecisionSignalV1>,
}

#[derive(Clone, Debug)]
pub(crate) struct ShopBranchOptionSelection {
    pub(crate) options: Vec<ShopBranchOption>,
    pub(crate) candidate_pool: BranchExperimentShopPlanCandidatePoolV1,
}

pub(crate) fn shop_branch_options(
    session: &RunControlSession,
) -> Option<ShopBranchOptionSelection> {
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
            .find(|candidate| candidate.plan.plan_id == compiled.compat_selected_plan.plan_id)
            .map(|candidate| &candidate.evaluation);
        if let Some(option) =
            shop_branch_option_from_plan(&compiled.compat_selected_plan, evaluation)
        {
            options.push(option);
        }
    }
    debug_assert!(
        !options.is_empty(),
        "shop compiler should expose an executable branch option, usually LeaveShop"
    );
    Some(ShopBranchOptionSelection {
        options,
        candidate_pool: shop_candidate_pool_from_compiled_v1(&compiled),
    })
}

fn shop_candidate_pool_from_compiled_v1(
    compiled: &crate::ai::shop_policy_v1::CompiledShopDecisionV1,
) -> BranchExperimentShopPlanCandidatePoolV1 {
    let candidates = compiled
        .candidate_plans
        .iter()
        .map(|candidate| shop_candidate_entry_from_plan_v1(compiled, candidate))
        .collect::<Vec<_>>();
    BranchExperimentShopPlanCandidatePoolV1 {
        branch_id: String::new(),
        branch_choices: Vec::new(),
        branch_commands: Vec::new(),
        depth: 0,
        frontier_key: String::new(),
        boundary_title: "Shop".to_string(),
        candidate_count: candidates.len(),
        branch_frontier_count: compiled.branch_frontier.len(),
        rollout_head_plan_id: compiled
            .rollout_head
            .as_ref()
            .map(|projection| projection.plan_id.clone()),
        candidates,
    }
}

fn shop_candidate_entry_from_plan_v1(
    compiled: &crate::ai::shop_policy_v1::CompiledShopDecisionV1,
    candidate: &crate::ai::shop_policy_v1::ShopPlanCandidateV1,
) -> BranchExperimentShopPlanCandidateEntryV1 {
    let evaluation = &candidate.evaluation;
    BranchExperimentShopPlanCandidateEntryV1 {
        plan_id: candidate.plan.plan_id.clone(),
        command: shop_plan_command(&candidate.plan),
        label: candidate.plan.label.clone(),
        role: format!("{:?}", candidate.role),
        source: format!("{:?}", candidate.plan.source),
        kind: format!("{:?}", candidate.plan.kind),
        lane: shop_candidate_lane_v1(compiled, candidate.plan.plan_id.as_str()),
        projection_roles: shop_candidate_projection_roles_v1(
            compiled,
            candidate.plan.plan_id.as_str(),
        ),
        total_gold_spent: candidate.plan.total_gold_spent,
        legacy_priority: evaluation
            .legacy_priority
            .or(candidate.plan.legacy_priority),
        suppressed_count: candidate.plan.suppressed_count,
        verdict: format!("{:?}", evaluation.verdict),
        rollout_admission: format!("{:?}", evaluation.rollout_admission.status),
        branch_admission: format!("{:?}", evaluation.branch_admission.status),
        tier: evaluation.tier,
        score: evaluation.score,
        confidence_milli: (evaluation.confidence * 1000.0).round() as i32,
        component_net_rank: evaluation.component_score.net.round() as i32,
        reasons: evaluation.reasons.clone(),
    }
}

fn shop_candidate_lane_v1(
    compiled: &crate::ai::shop_policy_v1::CompiledShopDecisionV1,
    plan_id: &str,
) -> String {
    compiled
        .frontier
        .lanes
        .iter()
        .find(|lane| lane.plan_ids.iter().any(|candidate| candidate == plan_id))
        .map(|lane| format!("{:?}", lane.lane))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn shop_candidate_projection_roles_v1(
    compiled: &crate::ai::shop_policy_v1::CompiledShopDecisionV1,
    plan_id: &str,
) -> Vec<String> {
    let mut roles = Vec::new();
    if compiled
        .rollout_head
        .as_ref()
        .is_some_and(|projection| projection.plan_id == plan_id)
    {
        roles.push("rollout_head".to_string());
    }
    if compiled
        .branch_frontier
        .iter()
        .any(|projection| projection.plan_id == plan_id)
    {
        roles.push("branch_frontier".to_string());
    }
    roles
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
        candidate_axis: shop_plan_candidate_axis_v1(plan),
        representative_count: plan.steps.len(),
        suppressed_count: plan.suppressed_count,
        decision_signal: evaluation.map(shop_decision_signal_v1),
    })
}

fn shop_plan_candidate_axis_v1(plan: &ShopPlanV1) -> Option<String> {
    let family = shop_plan_axis_family_v1(plan);
    crate::eval::decision_candidate_axis_v1::shop_decision_candidate_axis_v1(&family)
}

fn shop_plan_axis_family_v1(plan: &ShopPlanV1) -> String {
    let mut parts = Vec::<&'static str>::new();
    if plan
        .steps
        .iter()
        .any(|step| matches!(step, ShopPlanStepV1::RemoveCard { .. }))
    {
        parts.push("purge");
    }
    if plan
        .steps
        .iter()
        .any(|step| matches!(step, ShopPlanStepV1::BuyCard { .. }))
    {
        parts.push("buy_card");
    }
    if plan
        .steps
        .iter()
        .any(|step| matches!(step, ShopPlanStepV1::BuyRelic { .. }))
    {
        parts.push("buy_relic");
    }
    if plan
        .steps
        .iter()
        .any(|step| matches!(step, ShopPlanStepV1::BuyPotion { .. }))
    {
        parts.push("buy_potion");
    }
    if plan
        .steps
        .iter()
        .any(|step| matches!(step, ShopPlanStepV1::LeaveShop))
    {
        parts.push("leave");
    }

    match parts.as_slice() {
        [] => "stop".to_string(),
        ["purge"] => "purge_only".to_string(),
        ["buy_card"] => "buy_card_only".to_string(),
        ["buy_relic"] => "buy_relic_only".to_string(),
        ["buy_potion"] => "buy_potion_only".to_string(),
        ["leave"] => "leave_or_save_gold".to_string(),
        _ => parts.join("_plus_"),
    }
}

fn shop_decision_signal_v1(
    evaluation: &ShopPlanEvaluationV1,
) -> BranchExperimentChoiceDecisionSignalV1 {
    BranchExperimentChoiceDecisionSignalV1 {
        source: BRANCH_EXPERIMENT_SHOP_BRANCH_FRONTIER_SIGNAL_SOURCE_V1.to_string(),
        verdict: format!("{:?}", evaluation.verdict),
        tier: evaluation.tier,
        score: evaluation.score,
        confidence_milli: (evaluation.confidence * 1000.0).round() as i32,
        component_net_rank: evaluation.component_score.net.round() as i32,
        preferred: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::shop_policy_v1::{ShopPlanKindV1, ShopPlanSourceV1};

    fn test_shop_plan(steps: Vec<ShopPlanStepV1>) -> ShopPlanV1 {
        ShopPlanV1 {
            plan_id: "test".to_string(),
            label: "test".to_string(),
            kind: ShopPlanKindV1::Execute,
            steps,
            total_gold_spent: 0,
            candidate_ids: Vec::new(),
            source: ShopPlanSourceV1::CandidateEvidence,
            legacy_priority: None,
            legacy_confidence: None,
            suppressed_count: 0,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn shop_candidate_axis_describes_plan_shape_without_card_value() {
        let plan = test_shop_plan(vec![
            ShopPlanStepV1::RemoveCard {
                deck_index: 0,
                card: CardId::Strike,
                cost: 75,
            },
            ShopPlanStepV1::BuyCard {
                index: 2,
                card: CardId::Reaper,
                cost: 80,
            },
        ]);

        assert_eq!(
            shop_plan_candidate_axis_v1(&plan).as_deref(),
            Some("shop:shop:purge_plus_buy_card")
        );
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
