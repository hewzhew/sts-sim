use crate::runtime::action::{Action, ActionInfo, AddTo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn whirlwind_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);

    actions.push(ActionInfo {
        action: Action::Whirlwind {
            damages: evaluated.multi_damage.clone(),
            damage_type: DamageType::Normal,
            free_to_play_once: card.free_to_play_once,
            energy_on_use: card.energy_on_use,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
