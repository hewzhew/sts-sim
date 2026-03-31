use crate::content::relics::{RelicId, RelicState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // N'loth offers to trade 2 random relics for N'loth's Gift
            // internal_state encodes: bits[0..7]=choice1_idx, bits[8..15]=choice2_idx
            let c1 = (event_state.internal_state & 0xFF) as usize;
            let c2 = ((event_state.internal_state >> 8) & 0xFF) as usize;
            let r1_name = run_state.relics.get(c1).map(|r| format!("{:?}", r.id)).unwrap_or("???".into());
            let r2_name = run_state.relics.get(c2).map(|r| format!("{:?}", r.id)).unwrap_or("???".into());
            vec![
                EventChoiceMeta::new(format!("[Trade {}] Obtain N'loth's Gift.", r1_name)),
                EventChoiceMeta::new(format!("[Trade {}] Obtain N'loth's Gift.", r2_name)),
                EventChoiceMeta::new("[Leave]"),
            ]
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 | 1 => {
                    let relic_idx = if choice_idx == 0 {
                        (event_state.internal_state & 0xFF) as usize
                    } else {
                        ((event_state.internal_state >> 8) & 0xFF) as usize
                    };
                    // Remove the traded relic
                    if relic_idx < run_state.relics.len() {
                        run_state.relics.remove(relic_idx);
                    }
                    // Give N'loth's Gift (or Circlet if already owned, matching Java)
                    let gift_id = if run_state.relics.iter().any(|r| r.id == RelicId::NlothsGift) {
                        RelicId::Circlet
                    } else {
                        RelicId::NlothsGift
                    };
                    run_state.relics.push(RelicState::new(gift_id));
                    event_state.current_screen = 1;
                },
                _ => {
                    event_state.current_screen = 1;
                },
            }
        },
        _ => { event_state.completed = true; }
    }

    run_state.event_state = Some(event_state);
}

/// Initialize N'loth event state: pick 2 random relics to offer.
/// Java: Collections.shuffle(relics, new Random(miscRng.randomLong()))
/// then choice1 = relics[0], choice2 = relics[1]
pub fn init_nloth_state(run_state: &mut RunState) -> i32 {
    if run_state.relics.len() < 2 { return 0; }
    // Build index list and shuffle with randomLong seed (matching Java exactly)
    let mut indices: Vec<usize> = (0..run_state.relics.len()).collect();
    crate::rng::shuffle_with_random_long(&mut indices, &mut run_state.rng_pool.misc_rng);
    let idx1 = indices[0];
    let idx2 = indices[1];
    (idx1 as i32) | ((idx2 as i32) << 8)
}
