use crate::action::{Action, ActionInfo, AddTo, DamageType};
use crate::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn thunderclap_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::smallvec![ActionInfo {
        action: Action::DamageAllEnemies {
            source: 0,
            damages: card.multi_damage.clone(),
            damage_type: DamageType::Normal,
            is_modified: false,
        },
        insertion_mode: AddTo::Bottom,
    }];
    for monster in &state.monsters {
        if !monster.is_dying {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: 0,
                    target: monster.id,
                    power_id: crate::content::powers::PowerId::Vulnerable,
                    amount: card.base_magic_num_mut,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
    }
    actions
}
