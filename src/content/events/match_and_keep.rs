// Java: GremlinMatchGame (shrines) — "Match and Keep!"
// Memory card game: 12 cards face-down (6 pairs), player has 5 attempts to match pairs.
//
// Card pool (Java initializeCards):
//   A15+: Rare + Uncommon + Common + Curse + Curse + StartCard (Bash for Ironclad)
//   Non-A15: Rare + Uncommon + Common + ColorlessUncommon + Curse + StartCard
//   Each card duplicated (pair), then shuffled with miscRng.randomLong() seed.
//
// Game state stored in extra_data:
//   [0..12]  = card type indices for each board position (0-5)
//   [12]     = matched bitmask (bit i = position i matched/removed)
//   [13]     = attempts remaining (starts at 5)
//   [14]     = first flipped position (-1 if none)
//   [15..21] = CardId discriminant for each card type (lookup table)
//
// Screen flow:
//   0 = Intro: [Play]
//   1 = First flip: show face-down positions, player picks one
//   2 = Second flip: first card revealed, player picks second
//   3 = Result: match/mismatch shown, [Continue]
//   4 = Game over: [Leave]

use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

/// Curse pool matching Java CardLibrary.getCurse():
/// Excludes AscendersBane, Necronomicurse, CurseOfTheBell, Pride
const CURSE_POOL: &[CardId] = &[
    CardId::Clumsy, CardId::Decay, CardId::Doubt, CardId::Injury,
    CardId::Normality, CardId::Pain, CardId::Parasite, CardId::Regret,
    CardId::Shame, CardId::Writhe,
];

/// CardId lookup table stored in extra_data[15..21]
const CARD_LOOKUP_OFFSET: usize = 15;

/// Initialize match game board directly into event_state.extra_data.
/// Consumes RNG to match Java: getCard(×3) + returnRandomCurse/returnColorlessCard +
/// miscRng.randomLong() for shuffle seed.
pub fn init_match_game_board(run_state: &mut RunState, extra_data: &mut Vec<i32>) {
    let mut card_types: [CardId; 6] = [CardId::Strike; 6];

    // Java: getCard(RARE) — uses cardRng
    card_types[0] = run_state.random_card_by_rarity(crate::content::cards::CardRarity::Rare);
    // Java: getCard(UNCOMMON) — uses cardRng
    card_types[1] = run_state.random_card_by_rarity(crate::content::cards::CardRarity::Uncommon);
    // Java: getCard(COMMON) — uses cardRng
    card_types[2] = run_state.random_card_by_rarity(crate::content::cards::CardRarity::Common);

    if run_state.ascension_level >= 15 {
        // A15+: 2 curses, no colorless
        let idx1 = run_state.rng_pool.card_rng.random_range(0, CURSE_POOL.len() as i32 - 1) as usize;
        card_types[3] = CURSE_POOL[idx1];
        let idx2 = run_state.rng_pool.card_rng.random_range(0, CURSE_POOL.len() as i32 - 1) as usize;
        card_types[4] = CURSE_POOL[idx2];
    } else {
        // Non-A15: 1 colorless uncommon + 1 curse
        card_types[3] = run_state.random_colorless_card(crate::content::cards::CardRarity::Uncommon);
        let idx = run_state.rng_pool.card_rng.random_range(0, CURSE_POOL.len() as i32 - 1) as usize;
        card_types[4] = CURSE_POOL[idx];
    }

    // Java: player.getStartCardForEvent() — Bash for Ironclad (no RNG)
    card_types[5] = CardId::Bash;

    extra_data.clear();
    // Positions 0-11: card type index (0-5), 6 pairs
    for i in 0..6i32 {
        extra_data.push(i);
    }
    for i in 0..6i32 {
        extra_data.push(i);
    }

    // Java: Collections.shuffle(cards, new Random(miscRng.randomLong()))
    let seed = run_state.rng_pool.misc_rng.random_long();
    {
        let mut jur = crate::rng::JavaUtilRandom::new(seed as u64);
        let slice = &mut extra_data[0..12];
        for i in (1..slice.len()).rev() {
            let j = jur.next_int((i + 1) as i32) as usize;
            slice.swap(i, j);
        }
    }

    // [12] = matched bitmask (0 = none matched)
    extra_data.push(0);
    // [13] = attempts remaining
    extra_data.push(5);
    // [14] = first flipped position (-1 = none)
    extra_data.push(-1);
    // [15..21] = CardId values for each type index (stored as i32 discriminant)
    for i in 0..6 {
        extra_data.push(card_types[i] as i32);
    }
}

fn card_at(extra_data: &[i32], pos: usize) -> CardId {
    let type_idx = extra_data[pos] as usize;
    let card_disc = extra_data[CARD_LOOKUP_OFFSET + type_idx];
    card_id_from_i32(card_disc)
}

/// Convert i32 discriminant back to CardId.
/// Safe because CardId is #[repr(i32)] and we only store valid discriminants.
fn card_id_from_i32(v: i32) -> CardId {
    unsafe { std::mem::transmute::<i32, CardId>(v) }
}

fn is_matched(extra_data: &[i32], pos: usize) -> bool {
    (extra_data[12] & (1 << pos)) != 0
}

fn set_matched(extra_data: &mut [i32], pos: usize) {
    extra_data[12] |= 1 << pos;
}

fn attempts_remaining(extra_data: &[i32]) -> i32 {
    extra_data[13]
}

fn first_flipped(extra_data: &[i32]) -> i32 {
    extra_data[14]
}

pub fn get_choices(_run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            vec![EventChoiceMeta::new("[Play Match and Keep!]")]
        },
        1 | 2 => {
            // Show face-down card positions as choices
            let ed = &event_state.extra_data;
            let first = first_flipped(ed);
            let mut choices = Vec::new();
            for pos in 0..12usize {
                if is_matched(ed, pos) {
                    continue;
                }
                if event_state.current_screen == 2 && first == pos as i32 {
                    continue; // can't pick the already-flipped card
                }
                choices.push(EventChoiceMeta::new(format!("[Flip card {}]", pos)));
            }
            if choices.is_empty() {
                vec![EventChoiceMeta::new("[Leave]")]
            } else {
                choices
            }
        },
        3 => {
            let ed = &event_state.extra_data;
            let remaining = attempts_remaining(ed);
            let all_matched = ed[12] == 0xFFF;
            if remaining <= 0 || all_matched {
                vec![EventChoiceMeta::new("[Leave] Game over.")]
            } else {
                vec![EventChoiceMeta::new("[Continue] Flip more cards.")]
            }
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        },
        1 => {
            // First flip: map choice_idx to actual board position
            let mut available = Vec::new();
            for pos in 0..12usize {
                if !is_matched(&event_state.extra_data, pos) {
                    available.push(pos);
                }
            }
            if let Some(&pos) = available.get(choice_idx) {
                event_state.extra_data[14] = pos as i32;
                event_state.current_screen = 2;
            }
        },
        2 => {
            // Second flip
            let first_pos = first_flipped(&event_state.extra_data) as usize;
            let mut available = Vec::new();
            for pos in 0..12usize {
                if !is_matched(&event_state.extra_data, pos) && pos != first_pos {
                    available.push(pos);
                }
            }
            if let Some(&second_pos) = available.get(choice_idx) {
                let type1 = event_state.extra_data[first_pos];
                let type2 = event_state.extra_data[second_pos];

                if type1 == type2 {
                    // Match! Add card to deck via ShowCardAndObtainEffect (direct add)
                    let card_id = card_at(&event_state.extra_data, first_pos);
                    run_state.add_card_to_deck(card_id);
                    set_matched(&mut event_state.extra_data, first_pos);
                    set_matched(&mut event_state.extra_data, second_pos);
                }

                event_state.extra_data[13] -= 1; // consume attempt
                event_state.extra_data[14] = -1;  // reset first flipped

                let remaining = attempts_remaining(&event_state.extra_data);
                let all_matched = event_state.extra_data[12] == 0xFFF;
                if remaining <= 0 || all_matched {
                    event_state.current_screen = 4; // game over
                } else {
                    event_state.current_screen = 3; // continue
                }
            }
        },
        3 => {
            event_state.current_screen = 1; // back to first flip
        },
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}
