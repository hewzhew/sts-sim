use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct GremlinHorn;

impl GremlinHorn {
    pub fn on_monster_death() -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        actions.push(ActionInfo {
            action: Action::GainEnergy { amount: 1 },
            insertion_mode: AddTo::Bottom, // AddTo::Top if it needs to resolve immediately, but Bottom matches the native cadence
        });
        actions.push(ActionInfo {
            action: Action::DrawCards(1),
            insertion_mode: AddTo::Bottom, // Bottom resolves draw after energy cleanly
        });
        actions
    }
}
