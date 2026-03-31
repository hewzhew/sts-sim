use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;
use crate::state::HandSelectReason;

pub fn armaments_play(state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    // Armaments provides Block explicitly.
    actions.push(ActionInfo {
        action: Action::GainBlock {
            target: 0,
            amount: _card.base_block_mut,
        },
        insertion_mode: AddTo::Bottom,
    });
    
    // Armaments Upgrade logic
    if _card.upgrades > 0 {
        // Upgrade all
        for c in &state.hand {
            actions.push(ActionInfo {
                action: Action::UpgradeCard { card_uuid: c.uuid },
                insertion_mode: AddTo::Bottom,
            });
        }
    } else {
        // Upgrade one via hand select
        actions.push(ActionInfo {
            action: Action::SuspendForHandSelect {
                min: 1,
                max: 1,
                reason: HandSelectReason::Upgrade,
            },
            insertion_mode: AddTo::Bottom,
        });
    }
    
    actions
}
