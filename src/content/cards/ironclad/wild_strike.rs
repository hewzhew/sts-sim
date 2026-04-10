use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn wild_strike_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Wild Strike requires a valid target!");
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, Some(target));
    smallvec::smallvec![
        ActionInfo {
            action: Action::Damage(crate::action::DamageInfo {
                source: 0,
                target,
                base: evaluated.base_damage_mut,
                output: evaluated.base_damage_mut,
                damage_type: crate::action::DamageType::Normal,
                is_modified: false,
            }),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::MakeTempCardInDrawPile {
                card_id: crate::content::cards::CardId::Wound,
                amount: 1,
                random_spot: true,
                upgraded: false
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
