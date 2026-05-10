use crate::runtime::action::{Action, ActionInfo, AddTo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn thunderclap_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let mut actions = smallvec::smallvec![ActionInfo {
        action: Action::DamageAllEnemies {
            source: 0,
            damages: evaluated.multi_damage.clone(),
            damage_type: DamageType::Normal,
            is_modified: false,
        },
        insertion_mode: AddTo::Bottom,
    }];
    for monster in &state.entities.monsters {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: monster.id,
                power_id: crate::content::powers::PowerId::Vulnerable,
                amount: 1,
            },
            insertion_mode: AddTo::Bottom,
        });
    }
    actions
}
