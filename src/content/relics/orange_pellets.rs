use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// OrangePellets: Whenever you play an Attack, Skill, AND Power in the same turn,
/// remove all of your debuffs.
/// Tracks which card types have been played via bit flags in the counter:
///   bit 0 = Attack played, bit 1 = Skill played, bit 2 = Power played
///   When all 3 set (counter & 0b111 == 0b111), remove debuffs and reset.
pub fn on_use_card(
    card_id: crate::content::cards::CardId,
    counter: i32,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let def = crate::content::cards::get_card_definition(card_id);

    let bit = match def.card_type {
        crate::content::cards::CardType::Attack => 1,
        crate::content::cards::CardType::Skill => 2,
        crate::content::cards::CardType::Power => 4,
        _ => 0,
    };

    let new_counter = counter | bit;

    if new_counter != counter {
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::OrangePellets,
                counter: new_counter,
            },
            insertion_mode: AddTo::Top,
        });
    }

    // All 3 types played → remove debuffs
    if new_counter & 0b111 == 0b111 {
        actions.push(ActionInfo {
            action: Action::RemovePower {
                target: 0,
                power_id: crate::content::powers::PowerId::Weak,
            },
            insertion_mode: AddTo::Bottom,
        });
        actions.push(ActionInfo {
            action: Action::RemovePower {
                target: 0,
                power_id: crate::content::powers::PowerId::Vulnerable,
            },
            insertion_mode: AddTo::Bottom,
        });
        actions.push(ActionInfo {
            action: Action::RemovePower {
                target: 0,
                power_id: crate::content::powers::PowerId::Frail,
            },
            insertion_mode: AddTo::Bottom,
        });
        // Reset counter for next combo
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::OrangePellets,
                counter: 0,
            },
            insertion_mode: AddTo::Top,
        });
    }

    actions
}
