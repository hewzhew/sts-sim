use crate::action::{Action, ActionInfo, AddTo, DamageType};
use crate::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn reaper_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::VampireDamageAllEnemies {
            source: 0,
            damages: card.multi_damage.clone(),
            damage_type: DamageType::Normal,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
