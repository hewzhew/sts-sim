use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn wild_strike_play(_state: &CombatState, card: &CombatCard, target: Option<crate::core::EntityId>) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Wild Strike requires a valid target!");
    smallvec::smallvec![
        ActionInfo {
            action: Action::Damage(crate::action::DamageInfo {
                source: 0,
                target,
                base: card.base_damage_mut,
                output: card.base_damage_mut,
                damage_type: crate::action::DamageType::Normal,
                is_modified: false,
            }),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::MakeTempCardInDiscard { card_id: crate::content::cards::CardId::Wound, amount: 1, upgraded: false },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
