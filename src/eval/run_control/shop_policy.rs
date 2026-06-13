use crate::content::cards::get_card_definition;
use crate::content::potions::get_potion_definition;
use crate::state::core::{ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_shop_policy_action(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let EngineState::Shop(shop) = &session.engine_state else {
        return Ok(None);
    };
    let context =
        crate::ai::shop_policy_v1::build_shop_decision_context_v1(&session.run_state, shop);
    let compiled = crate::ai::shop_policy_v1::compile_shop_decision_v1(
        &context,
        &crate::ai::shop_policy_v1::ShopPolicyConfigV1::default(),
        crate::ai::shop_policy_v1::ShopCompileModeV1::ExecuteOne,
    );
    let Some(step) = compiled.selected_plan.steps.first() else {
        return Ok(None);
    };
    let action =
        crate::ai::shop_policy_v1::shop_policy_action_from_plan_v1(&compiled.selected_plan)
            .unwrap_or_else(|| crate::ai::shop_policy_v1::ShopPolicyActionV1::Stop {
                reason: compiled.selected_plan.reason.clone(),
            });
    let decision = crate::ai::shop_policy_v1::ShopDecisionV1 {
        action,
        label_role: "behavior_policy_not_teacher",
        context: context.clone(),
        strategic_trace: compiled.strategic_trace.clone(),
    };
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let (input, label) = shop_plan_step_input_and_label(step);
    let confidence = compiled.selected_plan.legacy_confidence.unwrap_or(0.0);
    let summary = format!(
        "shop policy: {} confidence={confidence:.2} reason={} plan={} source={:?} label_role={}",
        label,
        compiled.selected_plan.reason,
        compiled.selected_plan.plan_id,
        compiled.selected_plan.source,
        decision.label_role
    );

    let outcome = session.apply_input(input)?.with_trace_annotations(vec![
        super::noncombat_policy_annotation::noncombat_policy_annotation(
            "shop policy",
            noncombat_record,
        )?,
    ]);
    Ok(Some((outcome, summary)))
}

fn shop_plan_step_input_and_label(
    step: &crate::ai::shop_policy_v1::ShopPlanStepV1,
) -> (ClientInput, String) {
    match *step {
        crate::ai::shop_policy_v1::ShopPlanStepV1::BuyCard { index, card, .. } => (
            ClientInput::BuyCard(index),
            format!("buy card {}", get_card_definition(card).name),
        ),
        crate::ai::shop_policy_v1::ShopPlanStepV1::BuyRelic { index, relic, .. } => {
            (ClientInput::BuyRelic(index), format!("buy relic {relic:?}"))
        }
        crate::ai::shop_policy_v1::ShopPlanStepV1::BuyPotion { index, potion, .. } => (
            ClientInput::BuyPotion(index),
            format!("buy potion {}", get_potion_definition(potion).name),
        ),
        crate::ai::shop_policy_v1::ShopPlanStepV1::RemoveCard {
            deck_index, card, ..
        } => (
            ClientInput::PurgeCard(deck_index),
            format!("purge {}", get_card_definition(card).name),
        ),
        crate::ai::shop_policy_v1::ShopPlanStepV1::LeaveShop => {
            (ClientInput::Proceed, "leave shop".to_string())
        }
    }
}
