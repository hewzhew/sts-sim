use crate::content::relics::RelicState;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            // Potion option: stored potion slot in byte 2
            let potion_slot = ((event_state.internal_state >> 16) & 0xFF) as usize;
            let has_potion = potion_slot != 0xFF;
            // Gold amount in byte 0
            let gold_amt = event_state.internal_state & 0xFF;
            let has_gold = gold_amt > 0;
            // Card idx in byte 1
            let card_idx = ((event_state.internal_state >> 8) & 0xFF) as usize;
            let has_card = card_idx != 0xFF;

            vec![
                if has_potion {
                    EventChoiceMeta::new("[Give Potion] Obtain a Relic.")
                } else {
                    EventChoiceMeta::disabled("[Give Potion]", "No Potions")
                },
                if has_gold {
                    EventChoiceMeta::new(format!("[Give Gold] Lose {} Gold. Obtain a Relic.", gold_amt))
                } else {
                    EventChoiceMeta::disabled("[Give Gold]", "Not enough Gold")
                },
                if has_card {
                    EventChoiceMeta::new("[Give Card] Remove a card. Obtain a Relic.")
                } else {
                    EventChoiceMeta::disabled("[Give Card]", "No eligible cards")
                },
                EventChoiceMeta::new("[Attack]"),
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
                0 => {
                    // Give potion → relic
                    let potion_slot = ((event_state.internal_state >> 16) & 0xFF) as usize;
                    if potion_slot < run_state.potions.len() {
                        run_state.potions[potion_slot] = None;
                    }
                    let relic_id = run_state.random_relic();
                    event_state.current_screen = 1;
                    if let Some(next_state) = run_state.obtain_relic(relic_id, EngineState::EventRoom) {
                        *_engine_state = next_state;
                    }
                },
                1 => {
                    // Give gold → relic
                    let amt = event_state.internal_state & 0xFF;
                    run_state.gold = (run_state.gold - amt).max(0);
                    let relic_id = run_state.random_relic();
                    event_state.current_screen = 1;
                    if let Some(next_state) = run_state.obtain_relic(relic_id, EngineState::EventRoom) {
                        *_engine_state = next_state;
                    }
                },
                2 => {
                    // Give card → relic
                    let card_idx = ((event_state.internal_state >> 8) & 0xFF) as usize;
                    if card_idx < run_state.master_deck.len() {
                        let uuid = run_state.master_deck[card_idx].uuid;
                        run_state.remove_card_from_deck(uuid);
                    }
                    let relic_id = run_state.random_relic();
                    event_state.current_screen = 1;
                    if let Some(next_state) = run_state.obtain_relic(relic_id, EngineState::EventRoom) {
                        *_engine_state = next_state;
                    }
                },
                _ => {
                    // Attack (leave)
                    event_state.current_screen = 1;
                },
            }
        },
        _ => { event_state.completed = true; }
    }

    run_state.event_state = Some(event_state);
}

/// Initialize WeMeetAgain state.
/// Java constructor RNG call order:
///   1. getRandomPotion() → Collections.shuffle(list, new Random(miscRng.randomLong()))
///   2. getGoldAmount() → miscRng.random(50, min(gold, 150)) if gold >= 50
///   3. getRandomNonBasicCard() → Collections.shuffle(list, new Random(miscRng.randomLong()))
///
/// internal_state packing:
///   byte 0 (bits 0-7):   goldAmount (0-150, or 0 = none)
///   byte 1 (bits 8-15):  cardIdx (or 0xFF = none)
///   byte 2 (bits 16-23): potion slot index (or 0xFF = none)
pub fn init_we_meet_again_state(run_state: &mut RunState) -> i32 {
    // 1. Random potion: Java getRandomPotion() shuffles via miscRng.randomLong()
    let potion_slot: u8 = {
        let potion_indices: Vec<usize> = run_state.potions.iter()
            .enumerate()
            .filter(|(_, p)| p.is_some())
            .map(|(i, _)| i)
            .collect();
        if potion_indices.is_empty() {
            0xFF // no potion — Java also skips randomLong when no potions
        } else {
            // Consume miscRng.randomLong() for shuffle seed, pick first after shuffle
            let mut shuffled = potion_indices;
            crate::rng::shuffle_with_random_long(&mut shuffled, &mut run_state.rng_pool.misc_rng);
            shuffled[0] as u8
        }
    };

    // 2. Gold amount: Java miscRng.random(50, min(gold, 150))
    let gold_amount: u8 = if run_state.gold < 50 {
        0
    } else {
        let cap = if run_state.gold > 150 { 150 } else { run_state.gold };
        run_state.rng_pool.misc_rng.random_range(50, cap) as u8
    };

    // 3. Random non-basic card: shuffle with randomLong then pick [0]
    let mut eligible_indices: Vec<usize> = run_state.master_deck.iter().enumerate()
        .filter(|(_, c)| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.rarity != crate::content::cards::CardRarity::Basic
                && def.card_type != crate::content::cards::CardType::Curse
        })
        .map(|(i, _)| i)
        .collect();

    let card_idx: u8 = if eligible_indices.is_empty() {
        // Still consume randomLong? No — Java returns null if list is empty, no shuffle
        0xFF
    } else {
        crate::rng::shuffle_with_random_long(&mut eligible_indices, &mut run_state.rng_pool.misc_rng);
        eligible_indices[0] as u8
    };

    (gold_amount as i32) | ((card_idx as i32) << 8) | ((potion_slot as i32) << 16)
}
