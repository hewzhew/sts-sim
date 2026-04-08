// Java: TheJoust (city) — "The Joust"
// Screen 0: [Approach] → explanation
// Screen 1: [Bet Against (Murderer)] pay 50g, win 100g if murderer wins (70%)
//           [Bet For (Owner)] pay 50g, win 250g if owner wins (30%)
// Screen 2: [Continue] resolve fight
// Screen 3: Result screen → gold payout
// Screen 4: [Leave]
//
// Java uses miscRng.randomBoolean(0.3f) — owner wins 30%, murderer wins 70%

use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new(
            "[Approach] Investigate the commotion.",
        )],
        1 => vec![
            EventChoiceMeta::new("[Bet Against Owner] Pay 50 Gold. Win 100 Gold if Murderer wins."),
            EventChoiceMeta::new("[Bet For Owner] Pay 50 Gold. Win 250 Gold if Owner wins."),
        ],
        2 => vec![EventChoiceMeta::new("[Continue] Watch the joust!")],
        3 => {
            // internal_state encodes: bit0 = betFor, bit1 = ownerWins
            let bet_for = (event_state.internal_state & 1) != 0;
            let owner_wins = (event_state.internal_state & 2) != 0;
            let msg = if owner_wins {
                if bet_for {
                    "Owner wins! You won 250 Gold!"
                } else {
                    "Owner wins! You lost your bet."
                }
            } else {
                if bet_for {
                    "Murderer wins! You lost your bet."
                } else {
                    "Murderer wins! You won 100 Gold!"
                }
            };
            vec![EventChoiceMeta::new(format!("{} [Leave]", msg))]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            // Approach → explanation
            event_state.current_screen = 1;
        }
        1 => {
            // Place bet
            let bet_for = choice_idx == 1;
            run_state.gold = (run_state.gold - 50).max(0);
            event_state.internal_state = if bet_for { 1 } else { 0 };
            event_state.current_screen = 2;
        }
        2 => {
            // Resolve joust
            // Java: miscRng.randomBoolean(0.3f) — owner wins 30%
            let owner_wins = run_state.rng_pool.misc_rng.random_boolean_chance(0.3);

            let bet_for = (event_state.internal_state & 1) != 0;

            // Calculate gold payout
            if owner_wins && bet_for {
                run_state.gold += 250;
            } else if !owner_wins && !bet_for {
                run_state.gold += 100;
            }
            // else: bet lost, gold already deducted

            event_state.internal_state =
                (if bet_for { 1 } else { 0 }) | (if owner_wins { 2 } else { 0 });
            event_state.current_screen = 3;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
