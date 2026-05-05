use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn reckless_charge_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Reckless Charge requires a valid target!");
    smallvec::smallvec![
        ActionInfo {
            action: Action::Damage(crate::runtime::action::DamageInfo {
                source: 0,
                target,
                base: card.base_damage_mut,
                output: card.base_damage_mut,
                damage_type: crate::runtime::action::DamageType::Normal,
                is_modified: false,
            }),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::MakeTempCardInDrawPile {
                card_id: crate::content::cards::CardId::Dazed,
                amount: 1,
                random_spot: true,
                upgraded: false
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
