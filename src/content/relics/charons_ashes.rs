use crate::action::DamageType;
use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use smallvec::SmallVec;

pub struct CharonsAshes;

impl CharonsAshes {
    pub fn on_exhaust(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        let mut damages = SmallVec::new();
        for _ in 0..state.monsters.len() {
            damages.push(3);
        }

        actions.push(ActionInfo {
            action: Action::DamageAllEnemies {
                source: 0,
                damages,
                damage_type: DamageType::Normal,
                is_modified: false,
            },
            insertion_mode: AddTo::Top,
        });
        actions
    }
}
