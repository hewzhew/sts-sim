use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;

pub fn on_attacked(
    state: &CombatState,
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
        let has_artifact = state.entities.power_db.get(&owner).is_some_and(|powers| {
            powers
                .iter()
                .any(|p| p.power_type == PowerId::Artifact && p.amount > 0)
        });
        if !has_artifact {
            actions.push(Action::ApplyPower {
                source: owner,
                target: owner,
                power_id: PowerId::Shackled,
                amount: damage,
            });
        }
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
