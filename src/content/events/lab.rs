use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    let potion_count = if run_state.ascension_level >= 15 {
        2
    } else {
        3
    };
    vec![EventChoiceMeta::new(format!(
        "[Take] Obtain {} random Potions.",
        potion_count
    ))]
}

pub fn handle_choice(
    _engine_state: &mut EngineState,
    run_state: &mut RunState,
    _choice_idx: usize,
) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            // Take potions
            let potion_count = if run_state.ascension_level >= 15 {
                2
            } else {
                3
            };
            // Add random potions to the potion inventory
            for i in 0..potion_count {
                let pid = run_state.random_potion();
                let potion = crate::content::potions::Potion::new(pid, 10000 + i as u32);
                run_state.obtain_potion(potion);
            }
            event_state.current_screen = 1;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
