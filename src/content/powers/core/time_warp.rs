use crate::action::Action;
use crate::combat::{CombatCard, CombatState};
use crate::content::powers::PowerId;
use crate::core::EntityId;

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
        // Phase 2: 12-Card Trigger
        // 2.1 Reset counter to 0 (engine preservation handled in action_handlers.rs)
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::TimeWarp,
            amount: -amount,
        });

        // 2.2 Time Eater gains +2 Strength
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Strength,
            amount: 2,
        });

        // 2.3 Forcefully end the player's turn
        actions.push(Action::EndTurnTrigger);
    }

    actions
}
