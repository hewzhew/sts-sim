use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn bane_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Bane requires a valid target");
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, Some(target));
    let info = DamageInfo {
        source: 0,
        target,
        base: evaluated.base_damage_mut,
        output: evaluated.base_damage_mut,
        damage_type: DamageType::Normal,
        is_modified: true,
    };
    smallvec::smallvec![
        ActionInfo {
            action: Action::Damage(info.clone()),
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::BaneDamage(info),
            insertion_mode: AddTo::Bottom,
        },
    ]
}
