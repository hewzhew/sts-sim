use crate::content::cards::{get_card_definition, ironclad_pool_for_rarity, CardId, CardRarity};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

/// TheLibrary event.
/// Java: 2 options:
///   [Read] Generate 20 unique class cards (rollRarity + getCard, deduped), player picks 1
///   [Sleep] Heal 33% HP (A15: 20% HP)
///
/// Screen 0: initial choice (Read / Sleep)
/// Screen 1: 20 cards to choose from (only when Read was picked)
/// Screen 2: Leave

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let heal_pct = if run_state.ascension_level >= 15 {
                0.20
            } else {
                0.33
            };
            let heal_amt = (run_state.max_hp as f32 * heal_pct).round() as i32;
            vec![
                EventChoiceMeta::new("[Read] Choose a card from 20 offerings."),
                EventChoiceMeta::new(format!("[Sleep] Heal {} HP.", heal_amt)),
            ]
        }
        1 => {
            // Show 20 card offerings from extra_data
            let mut choices = Vec::with_capacity(20);
            for &card_disc in &event_state.extra_data {
                let card_id: CardId = unsafe { std::mem::transmute::<i32, CardId>(card_disc) };
                let def = get_card_definition(card_id);
                choices.push(EventChoiceMeta::new(format!(
                    "{} ({:?} {:?})",
                    def.name, def.rarity, def.card_type
                )));
            }
            choices
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Read: generate 20 cards and show them
                    generate_library_cards(run_state, &mut event_state.extra_data);
                    event_state.current_screen = 1;
                }
                _ => {
                    // Sleep: heal
                    let heal_pct = if run_state.ascension_level >= 15 {
                        0.20
                    } else {
                        0.33
                    };
                    let heal_amt = (run_state.max_hp as f32 * heal_pct).round() as i32;
                    run_state.current_hp = (run_state.current_hp + heal_amt).min(run_state.max_hp);
                    event_state.current_screen = 2;
                }
            }
        }
        1 => {
            // Pick one of the 20 cards
            if choice_idx < event_state.extra_data.len() {
                let card_disc = event_state.extra_data[choice_idx];
                let card_id: CardId = unsafe { std::mem::transmute::<i32, CardId>(card_disc) };
                run_state.add_card_to_deck(card_id);
            }
            event_state.current_screen = 2;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

/// Generate 20 unique class cards via Java's rollRarity + getCard with dedup.
///
/// Java logic (TheLibrary.buttonEffect case 0):
///   for (int i = 0; i < 20; ++i) {
///       card = getCard(rollRarity());
///       while (group contains card) {
///           card = getCard(rollRarity());  // re-roll both rarity and card
///       }
///       group.add(card);
///   }
///
/// rollRarity() uses cardRng.random(0,99) + cardBlizzRandomizer
/// getCard(rarity) uses cardRng.random(pool.size()-1) via pool.getRandomCard(true)
fn generate_library_cards(run_state: &mut RunState, extra_data: &mut Vec<i32>) {
    extra_data.clear();

    let mut selected: Vec<CardId> = Vec::with_capacity(20);

    for _ in 0..20 {
        let mut card_id = roll_and_get_card(run_state);

        // Dedup: re-roll if we already have this card
        // Java does while(containsDupe) { re-roll both rarity and card }
        // Safety limit to prevent infinite loop (shouldn't happen with 70+ card pool)
        let mut attempts = 0;
        while selected.contains(&card_id) && attempts < 100 {
            card_id = roll_and_get_card(run_state);
            attempts += 1;
        }

        selected.push(card_id);
        extra_data.push(card_id as i32);
    }
}

/// Roll a rarity and get a random card from that rarity pool.
/// Mirrors Java: rollRarity() + getCard(rarity)
///
/// rollRarity: cardRng.random(0,99) + cardBlizzRandomizer
///   - roll < 3 → RARE (fallback rates, since we're in event room)
///   - roll < 40 → UNCOMMON
///   - else → COMMON
///
/// getCard(rarity): pool.getRandomCard(true) → cardRng.random(pool.size()-1)
fn roll_and_get_card(run_state: &mut RunState) -> CardId {
    // Step 1: rollRarity — uses cardRng
    let roll = run_state.rng_pool.card_rng.random_range(0, 99) + run_state.card_blizz_randomizer;

    // Event room uses fallback rarity rates (no combat room rarity adjustments)
    let rarity = if roll < 3 {
        CardRarity::Rare
    } else if roll < 40 {
        CardRarity::Uncommon
    } else {
        CardRarity::Common
    };

    // Step 2: getCard(rarity) — uses cardRng via pool.getRandomCard(true)
    let pool = ironclad_pool_for_rarity(rarity);
    let idx = run_state
        .rng_pool
        .card_rng
        .random_range(0, pool.len() as i32 - 1) as usize;
    pool[idx]
}
