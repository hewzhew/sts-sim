use crate::content::cards::{get_card_definition, CardId};
use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn poisoned_stab_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let Some(target) = target else {
        return smallvec::smallvec![];
    };
    let def = get_card_definition(CardId::PoisonedStab);
    smallvec::smallvec![
        ActionInfo {
            action: Action::Damage(DamageInfo {
                source: 0,
                target,
                base: def.base_damage,
                output: card.base_damage_mut,
                damage_type: DamageType::Normal,
                is_modified: card.base_damage_mut != def.base_damage,
            }),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target,
                power_id: PowerId::Poison,
                amount: card.base_magic_num_mut,
            },
            insertion_mode: AddTo::Bottom,
        },
    ]
}
