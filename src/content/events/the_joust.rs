// Java: TheJoust (city) — "The Joust"
// Screen 0: [Approach] → explanation
// Screen 1: [Bet Against (Murderer)] pay 50g, win 100g if murderer wins (70%)
//           [Bet For (Owner)] pay 50g, win 250g if owner wins (30%)
// Screen 2: [Continue] roll fight result
// Screen 3: [Continue] reveal result and apply gold payout
// Screen 4: Result screen
// Screen 5: [Leave]
//
// Java uses miscRng.randomBoolean(0.3f) — owner wins 30%, murderer wins 70%

use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

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
        3 => vec![EventChoiceMeta::new("[Continue] Resolve the joust.")],
        4 => {
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
            run_state.change_gold_with_source(-50, DomainEventSource::Event(EventId::TheJoust));
            event_state.internal_state = if bet_for { 1 } else { 0 };
            event_state.current_screen = 2;
        }
        2 => {
            // Java PRE_JOUST click: roll result and enter animation/result-pending screen.
            // Java: miscRng.randomBoolean(0.3f) — owner wins 30%
            let owner_wins = run_state.rng_pool.misc_rng.random_boolean_chance(0.3);
            let bet_for = (event_state.internal_state & 1) != 0;
            event_state.internal_state =
                (if bet_for { 1 } else { 0 }) | (if owner_wins { 2 } else { 0 });
            event_state.current_screen = 3;
        }
        3 => {
            // Java JOUST click: reveal result and apply payout.
            let owner_wins = (event_state.internal_state & 2) != 0;
            let bet_for = (event_state.internal_state & 1) != 0;

            // Calculate gold payout
            if owner_wins && bet_for {
                run_state.change_gold_with_source(250, DomainEventSource::Event(EventId::TheJoust));
            } else if !owner_wins && !bet_for {
                run_state.change_gold_with_source(100, DomainEventSource::Event(EventId::TheJoust));
            }
            // else: bet lost, gold already deducted

            event_state.current_screen = 4;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

#[cfg(test)]
mod tests {
    use super::handle_choice;
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn joust_run(screen: usize, internal_state: i32, gold: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = gold;
        let mut event_state = EventState::new(EventId::TheJoust);
        event_state.current_screen = screen;
        event_state.internal_state = internal_state;
        run_state.event_state = Some(event_state);
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn pre_joust_continue_rolls_result_without_payout_like_java() {
        let mut run_state = joust_run(2, 0, 50);
        let start_counter = run_state.rng_pool.misc_rng.counter;
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        let event_state = run_state.event_state.as_ref().unwrap();
        assert_eq!(event_state.current_screen, 3);
        assert_eq!(run_state.rng_pool.misc_rng.counter, start_counter + 1);
        assert_eq!(run_state.gold, 50);
        assert!(run_state.take_emitted_events().is_empty());
    }

    #[test]
    fn result_continue_pays_murderer_bet_after_roll_screen() {
        let mut run_state = joust_run(3, 0, 50);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 4);
        assert_eq!(run_state.gold, 150);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::GoldChanged {
                delta: 100,
                new_total: 150,
                source: DomainEventSource::Event(EventId::TheJoust)
            }
        )));
    }

    #[test]
    fn result_continue_pays_owner_bet_after_roll_screen() {
        let mut run_state = joust_run(3, 0b11, 50);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.event_state.as_ref().unwrap().current_screen, 4);
        assert_eq!(run_state.gold, 300);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::GoldChanged {
                delta: 250,
                new_total: 300,
                source: DomainEventSource::Event(EventId::TheJoust)
            }
        )));
    }
}
