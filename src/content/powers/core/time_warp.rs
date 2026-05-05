use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState};

pub fn on_player_card_played(
    owner: EntityId,
    amount: i32,
    _card: &CombatCard,
    _state: &CombatState,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();

    // Phase 1: Increment Time Warp counter
    if amount < 11 {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::TimeWarp,
            amount: 1,
        });
    } else {
        actions.push(Action::TriggerTimeWarpEndTurn { owner });
    }

    actions
}
