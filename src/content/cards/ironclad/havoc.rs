use crate::combat::{CombatState, CombatCard};
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub fn havoc_play(_state: &CombatState, _card: &CombatCard, target: Option<crate::core::EntityId>) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Havoc requires a valid target!");
    smallvec::smallvec![
        ActionInfo {
            action: Action::PlayTopCard { target: Some(target), exhaust: true },
            insertion_mode: AddTo::Bottom,
        }
    ]
}
