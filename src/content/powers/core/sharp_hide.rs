use crate::content::cards::CardType;
use crate::core::EntityId;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::{smallvec, SmallVec};

/// Java SharpHidePower.onUseCard: fires once when an Attack card is played.
/// Deals THORNS damage to the player equal to Sharp Hide amount.
pub fn on_card_played(
    _state: &CombatState,
    owner: EntityId,
    card: &CombatCard,
    power_amount: i32,
) -> SmallVec<[Action; 2]> {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type == CardType::Attack {
        smallvec![Action::Damage(DamageInfo {
            source: owner,
            target: 0, // player
            base: power_amount,
            output: power_amount,
            damage_type: DamageType::Thorns,
            is_modified: false,
        })]
    } else {
        smallvec![]
    }
}
