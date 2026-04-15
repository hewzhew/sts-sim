use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

pub fn at_pre_battle(
    relic_state: &mut crate::content::relics::RelicState,
) -> SmallVec<[ActionInfo; 4]> {
    // Java: firstTurn = true; gainEnergyNext = true;
    // In our implementation, counter = -1 simulates firstTurn.
    relic_state.counter = -1;
    SmallVec::new()
}

/// Art of War: If you do not play any Attacks during your turn, gain an extra Energy next turn.
pub fn at_turn_start(
    _state: &CombatState,
    relic_state: &mut crate::content::relics::RelicState,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // In Java, at PreBattle gainEnergyNext = true, firstTurn = true.
    // counter = 1 means gainEnergyNext. counter = 0 means don't.
    // We treat initially empty counter (-1) as first turn.
    if relic_state.counter == 1 {
        actions.push(ActionInfo {
            action: Action::GainEnergy { amount: 1 },
            insertion_mode: AddTo::Bottom, // Java addToBot
        });
    }

    // Always reset to true at start of turn (gainEnergyNext = true).
    // Note: Java handles firstTurn by skipping energy gain but setting gainEnergyNext=true.
    // If counter is -1 (init), it won't trigger == 1, but we set it to 1 here for the next turn.
    actions.push(ActionInfo {
        action: Action::UpdateRelicCounter {
            relic_id: crate::content::relics::RelicId::ArtOfWar,
            counter: 1,
        },
        insertion_mode: AddTo::Bottom,
    });

    actions
}

pub fn on_use_card(
    _state: &CombatState,
    _relic_state: &mut crate::content::relics::RelicState,
    card_id: crate::content::cards::CardId,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let def = crate::content::cards::get_card_definition(card_id);

    if def.card_type == crate::content::cards::CardType::Attack {
        // Attack played -> disable energy gain next turn
        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::ArtOfWar,
                counter: 0,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
