use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::CombatState;
use crate::content::relics::RelicState;

/// Du-Vu Doll: At the start of each combat, gain 1 Strength for each Curse in your deck.
/// Java: atBattleStart() → count curses in masterDeck → applyPower(Strength, count)
/// We count curses from the draw_pile + discard_pile + hand (all combat cards) since
/// master_deck is not accessible from CombatState. The draw_pile at combat start IS the master_deck.
pub fn at_battle_start(state: &CombatState, _relic: &mut RelicState) -> smallvec::SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    
    // Count curses in draw pile (which at combat start contains the full deck)
    let curse_count = state.draw_pile.iter()
        .filter(|c| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.card_type == crate::content::cards::CardType::Curse
        })
        .count() as i32;

    if curse_count > 0 {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                target: 0,
                source: 0,
                power_id: crate::content::powers::PowerId::Strength,
                amount: curse_count,
            },
            insertion_mode: AddTo::Top,
        });
    }
    actions
}
