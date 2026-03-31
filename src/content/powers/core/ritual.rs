use crate::action::Action;
use crate::combat::PowerId;
use crate::core::EntityId;

pub fn at_end_of_round(state: &crate::combat::CombatState, owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    
    // Check extra_data to simulate skipFirst
    let extra = state.power_db.get(&owner)
        .and_then(|ps| ps.iter().find(|p| p.power_type == PowerId::Ritual))
        .map(|p| p.extra_data)
        .unwrap_or(0);

    if extra == 1 {
        // Skip first turn, but set skipFirst to false (extra_data to 0)
        actions.push(Action::UpdatePowerExtraData {
            target: owner,
            power_id: PowerId::Ritual,
            value: 0,
        });
    } else {
        // Ritual adds <amount> strength to owner at end of its round
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Strength,
            amount,
        });
    }
    
    actions
}
