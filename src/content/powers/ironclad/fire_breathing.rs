use crate::action::{Action, DamageType};
use crate::content::cards::{get_card_definition, CardType};
use smallvec::SmallVec;

pub fn on_card_drawn(
    owner: crate::core::EntityId,
    card_id: crate::content::cards::CardId,
    amount: i32,
) -> SmallVec<[Action; 2]> {
    let mut actions = SmallVec::new();
    let def = get_card_definition(card_id);
    if def.card_type == CardType::Status || def.card_type == CardType::Curse {
        actions.push(Action::DamageAllEnemies {
            source: owner,
            damages: smallvec::smallvec![amount; 5],
            damage_type: DamageType::Normal,
            is_modified: false,
        });
    }
    actions
}
