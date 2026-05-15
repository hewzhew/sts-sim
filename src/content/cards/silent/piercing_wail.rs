use crate::content::powers::{store, PowerId};
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn piercing_wail_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let amount = evaluated.base_magic_num_mut;
    let mut actions = SmallVec::new();

    for monster in &state.entities.monsters {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: monster.id,
                power_id: PowerId::Strength,
                amount: -amount,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    for monster in &state.entities.monsters {
        if store::has_power(state, monster.id, PowerId::Artifact) {
            continue;
        }
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: monster.id,
                power_id: PowerId::Shackled,
                amount,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
