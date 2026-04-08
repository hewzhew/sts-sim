use crate::state::core::{ClientInput, EngineState};

pub fn handle(
    run_state: &mut crate::state::run::RunState,
    shop: &mut crate::shop::ShopState,
    input: Option<crate::state::core::ClientInput>,
) -> Option<crate::state::core::EngineState> {
    if let Some(in_val) = input {
        match in_val {
            ClientInput::BuyCard(idx) => {
                if idx < shop.cards.len() && run_state.gold >= shop.cards[idx].price {
                    run_state.gold -= shop.cards[idx].price;
                    let c = shop.cards.remove(idx);
                    run_state.add_card_to_deck(c.card_id);
                }
            }
            ClientInput::BuyRelic(idx) => {
                if idx < shop.relics.len() && run_state.gold >= shop.relics[idx].price {
                    run_state.gold -= shop.relics[idx].price;
                    let r = shop.relics.remove(idx);
                    if let Some(next_state) =
                        run_state.obtain_relic(r.relic_id, EngineState::Shop(shop.clone()))
                    {
                        return Some(next_state);
                    }
                }
            }
            ClientInput::BuyPotion(idx) => {
                if idx < shop.potions.len() && run_state.gold >= shop.potions[idx].price {
                    // Sozu blocks obtaining potions
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == crate::content::relics::RelicId::Sozu)
                    {
                        // Sozu prevents potion acquisition — do nothing
                    } else if let Some(empty_slot) =
                        run_state.potions.iter().position(|p| p.is_none())
                    {
                        run_state.gold -= shop.potions[idx].price;
                        let bought = shop.potions.remove(idx);
                        run_state.potions[empty_slot] =
                            Some(crate::content::potions::Potion::new(bought.potion_id, 0));
                    }
                    // If no empty slot, purchase fails silently (matches Java behavior)
                }
            }
            ClientInput::PurgeCard(idx) => {
                if shop.purge_available && run_state.gold >= shop.purge_cost {
                    if idx < run_state.master_deck.len() {
                        let uuid = run_state.master_deck[idx].uuid;
                        run_state.gold -= shop.purge_cost;
                        shop.purge_available = false;
                        run_state.remove_card_from_deck(uuid);
                        run_state.shop_purge_count += 1;
                    }
                }
            }
            ClientInput::Proceed => {
                return Some(crate::state::core::EngineState::MapNavigation);
            }
            ClientInput::Cancel => {
                return Some(crate::state::core::EngineState::MapNavigation);
            }
            _ => {}
        }
    }
    None
}
