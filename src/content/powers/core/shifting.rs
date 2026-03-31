use crate::action::Action;
use crate::core::EntityId;
use crate::combat::CombatState;
use crate::content::powers::PowerId;

pub fn on_attacked(
    _state: &CombatState,
    owner: EntityId,
    damage: i32,
    _source: EntityId,
    _power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    if damage > 0 {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Strength,
            amount: -damage,
        });
    }

    actions
}

pub fn at_end_of_turn(_owner: EntityId) -> smallvec::SmallVec<[Action; 2]> {
    let actions = smallvec::smallvec![];
    // In actual game shifting restores stripped strength back to its starting state each turn.
    // For now we assume a hard reset or clean state handling per turn. 
    // Actual implementation requires an internal track `amount_lost_this_turn`, simplified for MVP.
    // actions.push(Action::ApplyPower { ... amount: amount_lost_this_turn ... })
    actions
}
