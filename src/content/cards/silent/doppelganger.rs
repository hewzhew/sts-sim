use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn doppelganger_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::Doppelganger {
            upgraded: card.upgrades > 0,
            free_to_play_once: card.free_to_play_once,
            energy_on_use: card.energy_on_use,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
