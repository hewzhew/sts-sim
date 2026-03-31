use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Pen Nib: Every 10th Attack you play deals double damage.
pub fn on_use_card(counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    
    let current = if counter < 0 { 0 } else { counter };
    let next_counter = current + 1;
    
    if next_counter == 10 {
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::PenNib,
                counter: 0,
            },
            insertion_mode: AddTo::Bottom,
        });
    } else if next_counter == 9 {
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::PenNib,
                counter: 9,
            },
            insertion_mode: AddTo::Bottom,
        });
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::PenNibPower,
                amount: 1,
            },
            insertion_mode: AddTo::Bottom,
        });
    } else {
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::PenNib,
                counter: next_counter,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}

pub fn at_battle_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if counter == 9 {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::PenNibPower,
                amount: 1,
            },
            insertion_mode: AddTo::Bottom,
        });
    }
    actions
}
