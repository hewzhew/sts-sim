use crate::content::cards::get_card_definition;
use crate::content::potions::get_potion_definition;
use crate::state::core::ClientInput;

pub fn shop_plan_step_input_and_label_v1(
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
