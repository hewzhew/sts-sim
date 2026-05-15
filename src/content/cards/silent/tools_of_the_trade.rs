use crate::content::powers::PowerId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn tools_of_the_trade_play(
    _state: &CombatState,
    _card: &CombatCard,
) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::ToolsOfTheTrade,
            amount: 1,
        },
        insertion_mode: AddTo::Bottom,
    }]
}
