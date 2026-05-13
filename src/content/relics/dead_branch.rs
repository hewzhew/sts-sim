use crate::content::relics::RelicState;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;

/// Dead Branch: Whenever you Exhaust a card, add a random card to your hand.
/// Java: MakeTempCardInHandAction(returnTrulyRandomCardInCombat().makeCopy())
pub fn on_exhaust(
    state: &CombatState,
    _relic: &mut RelicState,
) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    if state.are_monsters_basically_dead_java() {
        return actions;
    }

    // Java: returnTrulyRandomCardInCombat() — no type filter, no cost override
    actions.push(ActionInfo {
        action: Action::MakeRandomCardInHand {
            card_type: None, // No type filter — any card type
            cost_for_turn: None,
        },
        insertion_mode: AddTo::Bottom,
    });
    actions
}
