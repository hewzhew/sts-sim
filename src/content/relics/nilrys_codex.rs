use crate::runtime::action::ActionInfo;
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

pub fn at_end_of_turn(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    if !state.are_monsters_basically_dead_java() {
        actions.push(ActionInfo {
            action: crate::runtime::action::Action::SuspendForCardReward {
                pool: crate::runtime::action::CardRewardPool::ClassAll,
                destination: crate::runtime::action::CardDestination::DrawPileRandom,
                can_skip: true,
            },
            insertion_mode: crate::runtime::action::AddTo::Bottom,
        });
    }
    actions
}
