use crate::action::Action;
use crate::combat::{CombatState, PowerId};
use crate::core::EntityId;
use smallvec::{smallvec, SmallVec};

pub fn on_hp_lost(state: &CombatState, owner: EntityId, amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = smallvec![];
    
    // Mode Shift amount reduces by amount of HP lost
    // If it reaches 0 or less, it goes into Defensive Mode (SetMove 1)
    if let Some(pow) = state.power_db.get(&owner).and_then(|v| v.iter().find(|p| p.power_type == PowerId::ModeShift)) {
        if pow.amount - amount <= 0 {
            // Trigger Defensive mode!
            actions.push(Action::RemovePower { target: owner, power_id: PowerId::ModeShift });
            // Guardian clears its queue and forces Defensive Mode intent.
            actions.push(Action::SetMonsterMove {
                monster_id: owner,
                next_move_byte: 1, // CLOSE UP
                intent: crate::combat::Intent::Buff,
            });
            // Gain 20 block immediately
            actions.push(Action::GainBlock {
                target: owner,
                amount: 20,
            });
            // Increase GuardianThreshold by 10
            actions.push(Action::ApplyPower {
                target: owner,
                source: owner,
                power_id: PowerId::GuardianThreshold,
                amount: 10,
            });
        } else {
            actions.push(Action::ApplyPower {
                target: owner,
                source: owner,
                power_id: PowerId::ModeShift,
                amount: -amount,
            });
        }
    }

    actions
}
