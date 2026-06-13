use crate::state::run::RunState;
use crate::state::shop::ShopPotion;

pub(crate) fn shop_potion_purchase_block_reason_v1(
    run_state: &RunState,
    potion: &ShopPotion,
) -> Option<String> {
    crate::ai::shop_policy_v1::shop_potion_purchase_block_reason_v1(run_state, potion)
}
