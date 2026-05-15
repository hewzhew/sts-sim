use crate::content::powers::PowerId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn crippling_poison_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let mut actions = SmallVec::new();

    if state.are_monsters_basically_dead_java() {
        return actions;
    }

    for monster in state.entities.monsters.iter().filter(|monster| {
        !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
    }) {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: monster.id,
                power_id: PowerId::Poison,
                amount: evaluated.base_magic_num_mut,
            },
            insertion_mode: AddTo::Bottom,
        });
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: monster.id,
                power_id: PowerId::Weak,
                amount: 2,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
