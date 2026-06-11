use crate::content::relics::RelicId;
use crate::state::run::RunState;
use crate::state::shop::ShopPotion;

pub(crate) fn shop_potion_purchase_is_allowed_v1(
    run_state: &RunState,
    potion: &ShopPotion,
) -> bool {
    shop_potion_purchase_block_reason_v1(run_state, potion).is_none()
}

pub(crate) fn shop_potion_purchase_block_reason_v1(
    run_state: &RunState,
    potion: &ShopPotion,
) -> Option<String> {
    if !potion.can_buy {
        return Some(
            potion
                .blocked_reason
                .clone()
                .unwrap_or_else(|| "cannot buy".to_string()),
        );
    }
    if run_state.gold < potion.price {
        return Some("not enough gold".to_string());
    }
    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::Sozu)
    {
        return Some("blocked by Sozu".to_string());
    }
    if run_state.find_empty_potion_slot().is_none() {
        return Some("no empty potion slot".to_string());
    }
    None
}
