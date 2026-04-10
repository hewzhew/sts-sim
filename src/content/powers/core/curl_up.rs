use crate::action::Action;
use crate::combat::{CombatState, PowerId};
use crate::core::EntityId;

pub fn on_attacked(
    state: &CombatState,
    target: EntityId,
    amount: i32,
    _source: EntityId,
    power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    // Java CurlUp.onAttacked conditions:
    //   !this.triggered && damageAmount < this.owner.currentHealth
    //   && damageAmount > 0 && info.owner != null && info.type == NORMAL
    //
    // power_amount > 0 serves as "not triggered" guard — action_handlers.rs
    // zeroes the CurlUp amount before dispatching on_attacked hooks, so
    // multi-hit cards (Twin Strike, Pummel) won't re-trigger.
    if let Some(m) = state.entities.monsters.iter().find(|m| m.id == target) {
        if power_amount > 0 && amount > 0 && m.current_hp > 0 {
            actions.push(Action::GainBlock {
                target,
                amount: power_amount,
            });
            actions.push(Action::RemovePower {
                target,
                power_id: PowerId::CurlUp,
            });
        }
    }
    actions
}
