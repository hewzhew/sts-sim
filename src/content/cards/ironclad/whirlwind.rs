use crate::runtime::action::{Action, ActionInfo, AddTo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn whirlwind_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // Check for Chemical X relic here in the future:
    let effect = card.energy_on_use; // + 2 if Chemical X

    for _ in 0..effect {
        actions.push(ActionInfo {
            action: Action::DamageAllEnemies {
                source: 0,
                damages: card.multi_damage.clone(),
                damage_type: DamageType::Normal,
                is_modified: false,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
