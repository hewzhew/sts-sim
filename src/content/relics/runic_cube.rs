use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Runic Cube (Ironclad Boss Relic): Whenever you lose HP, draw 1 card.
/// Hook: wasHPLost (Java: addToTop)
pub fn was_hp_lost(damage_amount: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if damage_amount > 0 {
        actions.push(ActionInfo {
            action: Action::DrawCards(1),
            insertion_mode: AddTo::Top,
        });
    }
    actions
}
