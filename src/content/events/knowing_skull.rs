use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

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
    let source = DomainEventSource::Event(EventId::KnowingSkull);

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
                    run_state.change_hp_with_source(-cost, source);
                    inc_potion(&mut event_state.internal_state);
                    if !run_state
                        .relics
                        .iter()
                        .any(|relic| relic.id == crate::content::relics::RelicId::Sozu)
                    {
                        let pid = run_state.random_potion();
                        let potion = crate::content::potions::Potion::new(
                            pid,
                            20000 + potion_n(event_state.internal_state) as u32,
                        );
                        let _ = run_state.obtain_potion_with_source(potion, source);
                    }
                    // Stay on ASK screen (repeatable)
                }
                1 => {
                    // Gold: take goldCost damage, gain 90g, ++goldCost
                    let cost = gold_cost(event_state.internal_state);
                    run_state.change_hp_with_source(-cost, source);
                    inc_gold(&mut event_state.internal_state);
                    run_state.change_gold_with_source(GOLD_REWARD, source);
                    // Stay on ASK screen
                }
                2 => {
                    // Card: take cardCost damage, get colorless card, ++cardCost
                    let cost = card_cost(event_state.internal_state);
                    run_state.change_hp_with_source(-cost, source);
                    inc_card(&mut event_state.internal_state);
                    let card_id = run_state
                        .random_colorless_card(crate::content::cards::CardRarity::Uncommon);
                    run_state.add_card_to_deck_with_upgrades_from(card_id, 0, source);
                    // Stay on ASK screen
                }
                _ => {
                    // Leave: take fixed 6 damage, transition to COMPLETE
                    run_state.change_hp_with_source(-BASE_COST, source);
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

#[cfg(test)]
mod tests {
    use super::{card_cost, get_choices, gold_cost, handle_choice, potion_cost};
    use crate::content::relics::RelicId;
    use crate::state::core::{EngineState, RunResult};
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    #[test]
    fn java_cost_counters_are_independent() {
        let mut event = EventState::new(EventId::KnowingSkull);
        event.current_screen = 1;
        event.internal_state = 0;
        assert_eq!(potion_cost(event.internal_state), 6);
        assert_eq!(gold_cost(event.internal_state), 6);
        assert_eq!(card_cost(event.internal_state), 6);
    }

    #[test]
    fn potion_option_respects_sozu_like_java() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        run.event_state = Some({
            let mut event = EventState::new(EventId::KnowingSkull);
            event.current_screen = 1;
            event
        });
        run.obtain_relic(RelicId::Sozu, EngineState::EventRoom);
        let mut engine = EngineState::EventRoom;

        handle_choice(&mut engine, &mut run, 0);

        assert!(run.potions.iter().all(|slot| slot.is_none()));
        assert_eq!(run.current_hp, 74);
    }

    #[test]
    fn choices_match_java_screen_structure() {
        let run = RunState::new(1, 0, false, "Ironclad");
        let intro = EventState::new(EventId::KnowingSkull);
        let ask = {
            let mut event = EventState::new(EventId::KnowingSkull);
            event.current_screen = 1;
            event
        };
        assert_eq!(get_choices(&run, &intro).len(), 1);
        assert_eq!(get_choices(&run, &ask).len(), 4);
    }

    #[test]
    fn lethal_knowing_skull_click_emits_hp_loss_event() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        run.current_hp = 5;
        run.event_state = Some({
            let mut event = EventState::new(EventId::KnowingSkull);
            event.current_screen = 1;
            event
        });
        let mut engine = EngineState::EventRoom;

        handle_choice(&mut engine, &mut run, 3);

        assert_eq!(run.current_hp, 0);
        assert!(run.emitted_events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::HpChanged {
                    delta: -5,
                    current_hp: 0,
                    max_hp: _,
                    source: DomainEventSource::Event(EventId::KnowingSkull),
                }
            )
        }));
        assert!(!matches!(engine, EngineState::GameOver(RunResult::Defeat)));
    }
}
