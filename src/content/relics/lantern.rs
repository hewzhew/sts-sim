use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Lantern: Gain 1 Energy on the first turn of each combat.

pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    
    // We prime the used_up flag to track whether it's Turn 1
    actions.push(ActionInfo {
        action: Action::UpdateRelicUsedUp { 
            relic_id: crate::content::relics::RelicId::Lantern, 
            used_up: false,
        },
        insertion_mode: AddTo::Bottom,
    });
    
    actions
}

pub fn at_turn_start(used_up: bool) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    
    if !used_up {
        actions.push(ActionInfo {
            action: Action::GainEnergy { amount: 1 },
            insertion_mode: AddTo::Bottom,
        });
        actions.push(ActionInfo {
            action: Action::UpdateRelicUsedUp {
                relic_id: crate::content::relics::RelicId::Lantern,
                used_up: true,
            },
            insertion_mode: AddTo::Top, // Java: addToTop
        });
    }

    actions
}
