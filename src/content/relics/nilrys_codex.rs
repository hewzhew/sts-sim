use crate::action::ActionInfo;
use crate::combat::CombatState;
use smallvec::SmallVec;

pub fn at_end_of_turn(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let all_dead = state
        .monsters
        .iter()
        .all(|m| m.current_hp <= 0 || m.is_dying || m.is_escaped);
    if !all_dead {
        actions.push(ActionInfo {
            action: crate::action::Action::SuspendForCardReward {
                pool: crate::action::CardRewardPool::ClassAll,
                destination: crate::action::CardDestination::DrawPileRandom,
                can_skip: true,
            },
            insertion_mode: crate::action::AddTo::Bottom,
        });
    }
    actions
}
