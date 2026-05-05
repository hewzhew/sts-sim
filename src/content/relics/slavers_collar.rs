use crate::runtime::action::ActionInfo;
use crate::runtime::combat::CombatState;

pub fn at_battle_start(
    state: &mut CombatState,
    relic: &mut crate::content::relics::RelicState,
) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let actions = smallvec::SmallVec::new();

    let is_elite_or_boss = state.meta.is_elite_fight || state.meta.is_boss_fight;

    if is_elite_or_boss {
        state.entities.player.energy_master += 1;
        relic.counter = 1;
    } else {
        relic.counter = 0;
    }

    actions
}

pub fn on_victory(
    state: &mut CombatState,
    relic: &mut crate::content::relics::RelicState,
) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let actions = smallvec::SmallVec::new();
    if relic.counter == 1 {
        state.entities.player.energy_master -= 1;
        relic.counter = 0;
    }
    actions
}
