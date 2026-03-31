use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::reward::{RewardItem, RewardState};
use crate::state::run::RunState;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            if run_state.ascension_level >= 15 {
                vec![EventChoiceMeta::new("[Focus 1] Obtain 1 colorless card.")]
            } else {
                vec![
                    EventChoiceMeta::new("[Focus 1] Obtain 1 colorless card."),
                    EventChoiceMeta::new("[Focus 2] Obtain 2 colorless cards. Take 5 damage."),
                    EventChoiceMeta::new("[Focus 3] Obtain 3 colorless cards. Take 10 damage."),
                ]
            }
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            let count = if run_state.ascension_level >= 15 {
                1
            } else {
                (choice_idx + 1).min(3)
            };

            // Java: Collections.shuffle(memories, new Random(miscRng.randomLong()))
            // Consume randomLong for seed parity
            let _seed = run_state.rng_pool.misc_rng.random_long();

            // HP_LOSS damage for Focus 2/3 (Java: DamageInfo HP_LOSS — Tungsten Rod does NOT reduce)
            match choice_idx {
                1 => { run_state.current_hp = (run_state.current_hp - 5).max(0); },
                2 => { run_state.current_hp = (run_state.current_hp - 10).max(0); },
                _ => {},
            }

            // Java: addCardReward(RewardItem(COLORLESS)) × count
            // Each card reward row offers 3 colorless cards (pick 1, skippable)
            let mut rewards = RewardState::new();
            for _ in 0..count {
                let cards = generate_colorless_card_row(run_state);
                rewards.items.push(RewardItem::Card { cards });
            }

            event_state.current_screen = 1;
            run_state.event_state = Some(event_state);
            *engine_state = EngineState::RewardScreen(rewards);
            return;
        },
        _ => { event_state.completed = true; }
    }

    run_state.event_state = Some(event_state);
}

/// Generate a row of 3 colorless uncommon cards for the card reward screen.
/// Mirrors Java: RewardItem(CardColor.COLORLESS) generates 3 colorless cards.
/// Uses card_rng for consistency with card reward generation.
fn generate_colorless_card_row(run_state: &mut RunState) -> Vec<crate::content::cards::CardId> {
    use crate::content::cards::*;
    let pool = COLORLESS_UNCOMMON_POOL;
    let mut cards = Vec::with_capacity(3);
    for _ in 0..3 {
        let idx = run_state.rng_pool.card_rng.random_range(0, pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        // Avoid duplicates within the same row
        if !cards.contains(&card_id) {
            cards.push(card_id);
        } else {
            // If duplicate, try again with next random
            let idx2 = run_state.rng_pool.card_rng.random_range(0, pool.len() as i32 - 1) as usize;
            cards.push(pool[idx2]);
        }
    }
    cards
}
