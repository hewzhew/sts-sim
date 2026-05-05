use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Ink Bottle: Whenever you play 10 cards, draw 1 card.
/// Counter persists across combats (Java: no reset in atBattleStart).
/// Java: onUseCard() → ++counter, if counter==10 → counter=0, addToBot(DrawCardAction(1))

pub fn on_use_card(counter: i32) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    let next_counter = if counter + 1 >= 10 { 0 } else { counter + 1 };

    actions.push(ActionInfo {
        action: Action::UpdateRelicCounter {
            relic_id: crate::content::relics::RelicId::InkBottle,
            counter: next_counter,
        },
        insertion_mode: AddTo::Bottom, // Java: implicit (counter update is inline, not an action)
    });

    if next_counter == 0 {
        actions.push(ActionInfo {
            action: Action::DrawCards(1),
            insertion_mode: AddTo::Bottom, // Java: addToBot(DrawCardAction(1))
        });
    }

    actions
}
