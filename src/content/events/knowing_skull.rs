use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

// Java KnowingSkull has 4 independent cost counters:
//   potionCost, goldCost, cardCost — each starts at 6, incremented independently per purchase
//   leaveCost — fixed at 6
// We pack 3 counters into internal_state: bits [0..7]=potionN, [8..15]=goldN, [16..23]=cardN

const BASE_COST: i32 = 6;
const GOLD_REWARD: i32 = 90;

fn potion_n(state: i32) -> i32 {
    state & 0xFF
}
fn gold_n(state: i32) -> i32 {
    (state >> 8) & 0xFF
}
fn card_n(state: i32) -> i32 {
    (state >> 16) & 0xFF
}

fn inc_potion(state: &mut i32) {
    *state += 1;
}
fn inc_gold(state: &mut i32) {
    *state += 1 << 8;
}
fn inc_card(state: &mut i32) {
    *state += 1 << 16;
}

fn potion_cost(state: i32) -> i32 {
    BASE_COST + potion_n(state)
}
fn gold_cost(state: i32) -> i32 {
    BASE_COST + gold_n(state)
}
fn card_cost(state: i32) -> i32 {
    BASE_COST + card_n(state)
}

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // Intro screen
            vec![EventChoiceMeta::new("[Proceed]")]
        }
        1 => {
            // ASK screen: repeatable options with independent escalating costs
            let s = event_state.internal_state;
            vec![
                EventChoiceMeta::new(format!(
                    "[Potion] Lose {} HP. Obtain a random Potion.",
                    potion_cost(s)
                )),
                EventChoiceMeta::new(format!(
                    "[Gold] Gain {} Gold. Lose {} HP.",
                    GOLD_REWARD,
                    gold_cost(s)
                )),
                EventChoiceMeta::new(format!(
                    "[Card] Lose {} HP. Obtain a colorless card.",
                    card_cost(s)
                )),
                EventChoiceMeta::new(format!("[Leave] Lose {} HP.", BASE_COST)),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            // Intro → ASK
            event_state.current_screen = 1;
        }
        1 => {
            match choice_idx {
                0 => {
                    // Potion: take potionCost damage, get potion, ++potionCost
                    let cost = potion_cost(event_state.internal_state);
                    run_state.current_hp = (run_state.current_hp - cost).max(0);
                    inc_potion(&mut event_state.internal_state);
                    let pid = run_state.random_potion();
                    let potion = crate::content::potions::Potion::new(
                        pid,
                        20000 + potion_n(event_state.internal_state) as u32,
                    );
                    run_state.obtain_potion(potion);
                    // Stay on ASK screen (repeatable)
                }
                1 => {
                    // Gold: take goldCost damage, gain 90g, ++goldCost
                    let cost = gold_cost(event_state.internal_state);
                    run_state.current_hp = (run_state.current_hp - cost).max(0);
                    inc_gold(&mut event_state.internal_state);
                    run_state.gold += GOLD_REWARD;
                    // Stay on ASK screen
                }
                2 => {
                    // Card: take cardCost damage, get colorless card, ++cardCost
                    let cost = card_cost(event_state.internal_state);
                    run_state.current_hp = (run_state.current_hp - cost).max(0);
                    inc_card(&mut event_state.internal_state);
                    let card_id = run_state
                        .random_colorless_card(crate::content::cards::CardRarity::Uncommon);
                    run_state.add_card_to_deck(card_id);
                    // Stay on ASK screen
                }
                _ => {
                    // Leave: take fixed 6 damage, transition to COMPLETE
                    run_state.current_hp = (run_state.current_hp - BASE_COST).max(0);
                    event_state.current_screen = 2;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
