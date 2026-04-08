use crate::action::{Action, ActionInfo, AddTo};
use crate::content::relics::RelicId;
use smallvec::SmallVec;

pub struct AncientTeaSet;

impl AncientTeaSet {
    pub fn at_pre_battle(
        _relic_state: &mut crate::content::relics::RelicState,
    ) -> SmallVec<[ActionInfo; 4]> {
        // Java: firstTurn = true;
        // Rust's counter system (-2 / -1) implicitly prevents duplicate triggers,
        // so no explicit state reset is strictly needed for parity given the engine constraints,
        // but included here structurally to mirror Java's architecture.
        SmallVec::new()
    }
    pub fn at_turn_start(counter: i32) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        if counter == -2 {
            actions.push(ActionInfo {
                action: Action::GainEnergy { amount: 2 },
                insertion_mode: AddTo::Bottom,
            });
            actions.push(ActionInfo {
                action: Action::UpdateRelicCounter {
                    relic_id: RelicId::AncientTeaSet,
                    counter: -1,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
        actions
    }

    pub fn on_enter_rest_room(relic_state: &mut crate::content::relics::RelicState) {
        relic_state.counter = -2;
    }
}
