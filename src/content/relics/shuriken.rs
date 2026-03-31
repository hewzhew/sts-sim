use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Shuriken: Every time you play 3 Attacks in a single turn, gain 1 Strength.
/// Java: counter-based, resets each turn.
pub fn on_use_card(card_id: crate::content::cards::CardId, counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let def = crate::content::cards::get_card_definition(card_id);

    if def.card_type == crate::content::cards::CardType::Attack {
        let next_counter = if counter + 1 >= 3 { 0 } else { counter + 1 };

        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::Shuriken,
                counter: next_counter,
            },
            insertion_mode: AddTo::Bottom,
        });

        if next_counter == 0 {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: 0,
                    target: 0,
                    power_id: crate::content::powers::PowerId::Strength,
                    amount: 1,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
    }

    actions
}
