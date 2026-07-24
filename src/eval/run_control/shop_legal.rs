use crate::state::run::RunState;
use crate::state::shop::ShopPotion;

pub(crate) fn shop_merchandise_purchase_block_reason_v1(
    run_state: &RunState,
    can_buy: bool,
    blocked_reason: Option<&str>,
    price: i32,
) -> Option<String> {
    if !can_buy {
        return Some(blocked_reason.unwrap_or("cannot buy").to_string());
    }
    if run_state.gold < price {
        return Some("not enough gold".to_string());
    }
    None
}

pub(crate) fn shop_potion_purchase_block_reason_v1(
    run_state: &RunState,
    potion: &ShopPotion,
) -> Option<String> {
    if let Some(reason) = shop_merchandise_purchase_block_reason_v1(
        run_state,
        potion.can_buy,
        potion.blocked_reason.as_deref(),
        potion.price,
    ) {
        return Some(reason);
    }
    if run_state
        .relics
        .iter()
        .any(|relic| relic.id == crate::content::relics::RelicId::Sozu)
    {
        return Some("blocked by Sozu".to_string());
    }
    if run_state.find_empty_potion_slot().is_none() {
        return Some("no empty potion slot".to_string());
    }
    None
}
