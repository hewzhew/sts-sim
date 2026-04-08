use crate::action::Action;
use crate::combat::{CombatCard, CombatState, PowerId};
use crate::core::EntityId;
use smallvec::{smallvec, SmallVec};

pub fn on_card_played(
    _state: &CombatState,
    owner: EntityId,
    card: &CombatCard,
    power_amount: i32,
) -> SmallVec<[Action; 2]> {
    let mut actions = smallvec![];

    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type == crate::content::cards::CardType::Skill {
        actions.push(Action::ApplyPower {
            target: owner,
            source: owner,
            power_id: PowerId::Strength,
            amount: power_amount,
        });
    }

    actions
}
