use crate::runtime::action::{Action, ActionInfo, AddTo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn reaper_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    smallvec::smallvec![ActionInfo {
        action: Action::VampireDamageAllEnemies {
            source: 0,
            damages: evaluated.multi_damage.clone(),
            damage_type: DamageType::Normal,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
