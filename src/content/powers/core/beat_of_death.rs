use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::CombatCard;
use crate::core::EntityId;

pub fn on_player_card_played(
    owner: EntityId,
    amount: i32,
    _card: &CombatCard,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();

    // Beat of Death deals direct damage to the player
    actions.push(Action::Damage(DamageInfo {
        source: owner,
        target: 0, // Player
        base: amount,
        output: amount,
        damage_type: DamageType::Thorns,
        is_modified: false,
    }));

    actions
}
