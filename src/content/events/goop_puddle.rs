use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

const GOLD_GAIN: i32 = 75;
const DAMAGE: i32 = 11;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    if event_state.current_screen == 1 {
        return vec![EventChoiceMeta::new("[Leave]")];
    }

    // Gold loss stored in internal_state (rolled at init time, matching Java)
    let gold_loss = event_state.internal_state;
    vec![
        EventChoiceMeta::new(format!("[Gather Gold] Gain {} Gold. Take {} damage.", GOLD_GAIN, DAMAGE)),
        EventChoiceMeta::new(format!("[Leave] Lose {} Gold.", gold_loss)),
    ]
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Gather Gold: +75g, take 11 damage
                    run_state.gold += GOLD_GAIN;
                    run_state.current_hp = (run_state.current_hp - DAMAGE).max(0);
                },
                _ => {
                    // Leave: lose pre-rolled gold amount
                    let gold_loss = event_state.internal_state;
                    let actual_loss = gold_loss.min(run_state.gold);
                    run_state.gold -= actual_loss;
                },
            }
            event_state.current_screen = 1;
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

/// Initialize GoopPuddle state.
/// Java: goldLoss is a constructor field — miscRng.random(35,75) or random(20,50) at init time.
/// internal_state = goldLoss amount
pub fn init_goop_puddle_state(run_state: &mut RunState) -> i32 {
    if run_state.ascension_level >= 15 {
        run_state.rng_pool.misc_rng.random_range(35, 75)
    } else {
        run_state.rng_pool.misc_rng.random_range(20, 50)
    }
}
