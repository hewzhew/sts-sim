use crate::runtime::action::ActionInfo;
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

pub fn at_end_of_turn(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let all_dead = state
        .entities
        .monsters
        .iter()
        .all(|m| m.current_hp <= 0 || m.is_dying || m.is_escaped);
    if !all_dead {
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
