use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use crate::core::EntityId;
use smallvec::SmallVec;

pub fn feed_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Feed requires a valid target!");
    smallvec::smallvec![ActionInfo {
        action: Action::Feed {
            target,
            damage_info: DamageInfo {
                source: 0,
                target,
                base: card.base_damage_mut,
                output: card.base_damage_mut,
                damage_type: DamageType::Normal,
                is_modified: false,
            },
            max_hp_amount: card.base_magic_num_mut,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
