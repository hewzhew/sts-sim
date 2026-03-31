use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Necronomicon: The first Attack you play each turn that costs 2 or more is played twice.
/// Java: onUseCard() checks if card is Attack, cost >= 2, and counter == 0 (not yet used this turn).
/// If conditions match, sets counter to 1, and queues a second play of the card.
pub fn on_use_card(
    card_id: crate::content::cards::CardId,
    card_cost: i32,
    counter: i32,
    card: &crate::combat::CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let def = crate::content::cards::get_card_definition(card_id);

    // Only triggers once per turn (counter=0 means not yet triggered)
    if counter == 0 && def.card_type == crate::content::cards::CardType::Attack && card_cost >= 2 {
        // Mark as used this turn
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::Necronomicon,
                counter: 1,
            },
            insertion_mode: AddTo::Top,
        });
        // Play the card again (without consuming it from hand — it's a free replay)
        actions.push(ActionInfo {
            action: Action::PlayCardDirect {
                card: Box::new(card.clone()),
                target,
                purge: true, // Don't add to discard again
            },
            insertion_mode: AddTo::Top,
        });
    }

    actions
}
