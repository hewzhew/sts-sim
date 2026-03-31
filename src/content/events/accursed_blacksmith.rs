// Java: AccursedBlacksmith (shrines)
// Screen 0: [Forge] Upgrade a card | [Rummage] Gain Pain curse + WarpedTongs relic | [Leave]
// Screen 1: [Leave]
//
// Forge uses gridSelectScreen for player to choose which card to upgrade.

use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let has_upgradable = run_state.master_deck.iter().any(|c| {
        let def = crate::content::cards::get_card_definition(c.id);
        // Java: canUpgrade() — curses never, SearingBlow always, others only if upgrades == 0
        match def.rarity {
            crate::content::cards::CardRarity::Curse => false,
            _ => c.id == crate::content::cards::CardId::SearingBlow || c.upgrades == 0,
        }
    });

    let mut choices = vec![];
    if has_upgradable {
        choices.push(EventChoiceMeta::new("[Forge] Upgrade a card."));
    } else {
        choices.push(EventChoiceMeta::disabled("[Forge] Upgrade a card.", "No upgradable cards"));
    }
    choices.push(EventChoiceMeta::new("[Rummage] Obtain Pain and Warped Tongs."));
    choices.push(EventChoiceMeta::new("[Leave]"));
    choices
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    if event_state.completed {
        run_state.event_state = Some(event_state);
        return;
    }

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => { // Forge: upgrade a card via grid-select (Java: gridSelectScreen.open(getUpgradableCards(), 1, ...))
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        min_choices: 1,
                        max_choices: 1,
                        reason: RunPendingChoiceReason::Upgrade,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                    event_state.current_screen = 1;
                },
                1 => { // Rummage: obtain Pain curse + WarpedTongs relic
                    run_state.add_card_to_deck(crate::content::cards::CardId::Pain);
                    run_state.relics.push(crate::content::relics::RelicState::new(
                        crate::content::relics::RelicId::WarpedTongs,
                    ));
                    event_state.current_screen = 1;
                },
                _ => { // Leave
                    event_state.completed = true;
                }
            }
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
