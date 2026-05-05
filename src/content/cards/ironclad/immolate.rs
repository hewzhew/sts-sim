use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn immolate_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::DamageAllEnemies {
                source: 0,
                damages: card.multi_damage.clone(),
                damage_type: crate::runtime::action::DamageType::Normal,
                is_modified: false,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::MakeTempCardInDiscard {
                card_id: crate::content::cards::CardId::Burn,
                amount: 1,
                upgraded: false
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
