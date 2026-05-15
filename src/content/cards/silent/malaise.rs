use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn malaise_play(
    _state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Malaise requires a valid target");
    smallvec::smallvec![ActionInfo {
        action: Action::Malaise {
            target,
            upgraded: card.upgrades > 0,
            free_to_play_once: card.free_to_play_once,
            energy_on_use: card.energy_on_use,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
