use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

const MAX_HP_AMT: i32 = 5;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let heal_amt = run_state.max_hp / 3;
    vec![
        EventChoiceMeta::new(format!("[Banana] Heal {} HP.", heal_amt)),
        EventChoiceMeta::new(format!("[Donut] Gain {} Max HP.", MAX_HP_AMT)),
        EventChoiceMeta::new("[Box] Obtain a random Relic. Become Cursed - Regret."),
    ]
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Banana: Heal maxHP/3
                    let heal_amt = run_state.max_hp / 3;
                    run_state.current_hp = (run_state.current_hp + heal_amt).min(run_state.max_hp);
                }
                1 => {
                    // Donut: +5 Max HP
                    run_state.max_hp += MAX_HP_AMT;
                    run_state.current_hp += MAX_HP_AMT;
                }
                _ => {
                    // Box: Random relic + Regret curse
                    let relic_id = run_state.random_screenless_relic_reward();
                    if let Some(next_state) = run_state.obtain_relic_with_source(
                        relic_id,
                        EngineState::EventRoom,
                        DomainEventSource::Event(EventId::BigFish),
                    ) {
                        *engine_state = next_state;
                    }
                    run_state.add_card_to_deck(CardId::Regret);
                }
            }
            event_state.current_screen = 1;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
