use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::{CombatCard, CombatState};
use crate::core::EntityId;
use smallvec::SmallVec;

pub fn pommel_strike_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Pommel Strike requires a valid target!");
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, Some(target));
    let mut actions = SmallVec::new();

    actions.push(ActionInfo {
        action: Action::Damage(DamageInfo {
            source: 0,
            target,
            base: evaluated.base_damage_mut,
            output: evaluated.base_damage_mut,
            damage_type: DamageType::Normal,
            is_modified: true,
        }),
        insertion_mode: AddTo::Bottom,
    });

    actions.push(ActionInfo {
        action: Action::DrawCards(evaluated.base_magic_num_mut as u32),
        insertion_mode: AddTo::Bottom,
    });

    actions
}
