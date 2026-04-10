use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use crate::content::powers::PowerId;
use smallvec::SmallVec;

pub fn shockwave_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    let amount = card.base_magic_num_mut; // 3, upgraded 5

    for m in &state.entities.monsters {
        if !m.is_dying && !m.is_escaped {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: 0,
                    target: m.id,
                    power_id: PowerId::Weak,
                    amount,
                },
                insertion_mode: AddTo::Bottom,
            });
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: 0,
                    target: m.id,
                    power_id: PowerId::Vulnerable,
                    amount,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
    }

    actions
}
