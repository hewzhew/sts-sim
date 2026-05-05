use crate::content::cards::{get_card_definition, CardId};
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn dagger_throw_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if let Some(target) = target {
        let def = get_card_definition(CardId::DaggerThrow);
        actions.push(ActionInfo {
            action: Action::Damage(DamageInfo {
                source: 0,
                target,
                base: def.base_damage,
                output: card.base_damage_mut,
                damage_type: DamageType::Normal,
                is_modified: card.base_damage_mut != def.base_damage,
            }),
            insertion_mode: AddTo::Bottom,
        });
    }
    actions.push(ActionInfo {
        action: Action::DrawCards(1),
        insertion_mode: AddTo::Bottom,
    });
    actions.push(ActionInfo {
        action: Action::SuspendForHandSelect {
            min: 1,
            max: 1,
            can_cancel: false,
            filter: crate::state::HandSelectFilter::Any,
            reason: crate::state::HandSelectReason::Discard,
        },
        insertion_mode: AddTo::Bottom,
    });
    actions
}
