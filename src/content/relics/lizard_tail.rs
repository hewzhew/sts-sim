use crate::combat::CombatState;
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Lizard Tail: When you would die, heal to 50% of your Max HP instead (Works once).
/// Java: onTrigger() → addToTop(RelicAboveCreatureAction) → player.heal(maxHealth/2, true) → setCounter(-2)
/// Note: heal amount minimum 1, heal is direct (not an action), uses addToTop for the relic display.

pub fn on_lose_hp(state: &CombatState, used: bool) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    if state.player.current_hp <= 0 && !used {
        // Java: int healAmt = maxHealth / 2; if (healAmt < 1) healAmt = 1;
        let heal_amount = std::cmp::max(1, state.player.max_hp / 2);

        actions.push(ActionInfo {
            action: Action::Heal { target: 0, amount: heal_amount },
            insertion_mode: AddTo::Top, // Java: addToTop (heal happens before death check resolves)
        });

        actions.push(ActionInfo {
            action: Action::UpdateRelicUsedUp {
                relic_id: crate::content::relics::RelicId::LizardTail,
                used_up: true,
            },
            insertion_mode: AddTo::Top, // Java: setCounter(-2) is inline
        });
    }

    actions
}
