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
    let decision = crate::ai::shop_policy_v1::plan_shop_decision_v1(
        &context,
        &crate::ai::shop_policy_v1::ShopPolicyConfigV1::default(),
    );
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let (input, summary) = match decision.action {
        crate::ai::shop_policy_v1::ShopPolicyActionV1::Purge {
            deck_index,
            card,
            confidence,
            reason,
        } => {
            let card_name = get_card_definition(card).name;
            (
                ClientInput::PurgeCard(deck_index),
                format!(
                    "shop policy: purge {card_name} confidence={confidence:.2} reason={reason} label_role={}",
                    decision.label_role
                ),
            )
        }
        crate::ai::shop_policy_v1::ShopPolicyActionV1::Purchase {
            target,
            confidence,
            reason,
        } => {
            let (input, label) = purchase_input_and_label(target);
            (
                input,
                format!(
                    "shop policy: buy {label} confidence={confidence:.2} reason={reason} label_role={}",
                    decision.label_role
                ),
            )
        }
        crate::ai::shop_policy_v1::ShopPolicyActionV1::Stop { .. } => return Ok(None),
    };

    let outcome = session.apply_input(input)?.with_trace_annotations(vec![
        super::noncombat_policy_annotation::noncombat_policy_annotation(
            "shop policy",
            noncombat_record,
        )?,
    ]);
    Ok(Some((outcome, summary)))
}

fn purchase_input_and_label(
    target: crate::ai::shop_policy_v1::ShopPurchaseTargetV1,
) -> (ClientInput, String) {
    match target {
        crate::ai::shop_policy_v1::ShopPurchaseTargetV1::Card { index, card } => (
            ClientInput::BuyCard(index),
            format!("card {}", get_card_definition(card).name),
        ),
        crate::ai::shop_policy_v1::ShopPurchaseTargetV1::Relic { index, relic } => {
            (ClientInput::BuyRelic(index), format!("relic {relic:?}"))
        }
        crate::ai::shop_policy_v1::ShopPurchaseTargetV1::Potion { index, potion } => (
            ClientInput::BuyPotion(index),
            format!("potion {}", get_potion_definition(potion).name),
        ),
    }
}
