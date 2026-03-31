use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use crate::content::cards::CardId;

pub fn on_end_turn_in_hand(_state: &CombatState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    // Note: To truly match Spire, this should make a copy and insert it.
    // We will generate an action for MakeTempCard in DrawPile later, but for now we will stub it to just insert it directly into the state if possible, or build a new Action variant.
    
    // As we lack Action::MakeTempCard right now, we will add it to the CombatState directly via a helper Action or mutate state.
    // For now we will create an Action::MakeTempCardInDrawPile
    smallvec::smallvec![
        ActionInfo {
            action: Action::MakeTempCardInDrawPile { card_id: CardId::Pride, amount: 1, random_spot: false, upgraded: false }, // Top of draw pile
            insertion_mode: AddTo::Bottom,
        }
    ]
}
