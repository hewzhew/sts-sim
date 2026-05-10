use crate::runtime::action::{Action, ActionInfo, AddTo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn whirlwind_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);

    let effect = card.energy_on_use.max(0);

    for _ in 0..effect {
        actions.push(ActionInfo {
            action: Action::DamageAllEnemies {
                source: 0,
                damages: evaluated.multi_damage.clone(),
                damage_type: DamageType::Normal,
                is_modified: false,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
