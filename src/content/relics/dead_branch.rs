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
    if are_monsters_basically_dead(state) {
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

fn are_monsters_basically_dead(state: &CombatState) -> bool {
    state
        .entities
        .monsters
        .iter()
        .all(|monster| monster.is_dying || monster.is_escaped)
}
