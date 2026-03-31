use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn immolate_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::DamageAllEnemies {
                source: 0,
                damages: smallvec::smallvec![card.base_damage_mut; 5],
                damage_type: crate::action::DamageType::Normal,
                is_modified: false,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::MakeTempCardInDiscard { card_id: crate::content::cards::CardId::Burn, amount: 1, upgraded: false },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
