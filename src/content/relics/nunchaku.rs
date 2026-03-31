use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Nunchaku: Every time you play 10 Attacks, gain 1 Energy.
pub fn on_use_card(counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    
    // The dispatcher only triggers `on_use_card` here if the card is an Attack.
    let current = if counter < 0 { 0 } else { counter };
    let next_counter = current + 1;
    
    if next_counter >= 10 {
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::Nunchaku,
                counter: 0,
            },
            insertion_mode: AddTo::Bottom,
        });
        actions.push(ActionInfo {
            action: Action::GainEnergy { amount: 1 },
            insertion_mode: AddTo::Bottom,
        });
    } else {
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::Nunchaku,
                counter: next_counter,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
