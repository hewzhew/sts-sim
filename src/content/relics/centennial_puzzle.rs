use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

pub struct CentennialPuzzle;

impl CentennialPuzzle {
    pub fn on_lose_hp(used_up: bool) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        if !used_up {
            actions.push(ActionInfo {
                action: Action::DrawCards(3),
                insertion_mode: AddTo::Top,
            });
            actions.push(ActionInfo {
                action: Action::UpdateRelicUsedUp {
                    relic_id: crate::content::relics::RelicId::CentennialPuzzle,
                    used_up: true,
                },
                insertion_mode: AddTo::Top,
            });
        }
        actions
    }

    pub fn at_pre_battle(
        relic_state: &mut crate::content::relics::RelicState,
    ) -> SmallVec<[ActionInfo; 4]> {
        // Java: usedThisCombat = false;
        relic_state.used_up = false;
        SmallVec::new()
    }
}
