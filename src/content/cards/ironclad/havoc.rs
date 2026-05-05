use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn havoc_play(
    _state: &CombatState,
    _card: &CombatCard,
    _target: Option<crate::core::EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::PlayTopCard {
            target: None,
            exhaust: true
        },
        insertion_mode: AddTo::Bottom,
    }]
}
