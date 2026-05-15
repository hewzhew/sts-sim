use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn corpse_explosion_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Corpse Explosion requires a valid target");
    smallvec::smallvec![
        ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target,
                power_id: PowerId::Poison,
                amount: card.base_magic_num_mut,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target,
                power_id: PowerId::CorpseExplosion,
                amount: 1,
            },
            insertion_mode: AddTo::Bottom,
        },
    ]
}
