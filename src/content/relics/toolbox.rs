use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Toolbox (Shop Relic): At the start of each combat, choose 1 of 3 random Colorless cards and add it to your hand.
pub fn at_battle_start() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    
    // Java: Toolbox.atBattleStartPreDraw()
    // -> this.addToBot(new ChooseOneColorless())
    // Which opens a Discovery choice with cardRandomRng using Colorless cards.
    
    actions.push(ActionInfo {
        action: Action::SuspendForDiscovery {
            card_type: None, // Signal for colorless discovery
            cost_for_turn: None, // Normal cost, unlike Discovery which makes it cost 0
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}
