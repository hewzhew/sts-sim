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
//   [15..27] = CardId discriminant + previewed upgrades for each card type
//              (6 entries, 2 ints each)
//   [27]     = last flipped 1 pos (-1 if none)
//   [28]     = last flipped 2 pos (-1 if none)
//
// Screen flow:
//   0 = Intro: [Play]
//   1 = First flip: show face-down positions, player picks one
//   2 = Second flip: first card revealed, player picks second
//   3 = Result: match/mismatch shown, [Continue]
//   4 = Game over: [Leave]

use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

/// Curse pool matching Java CardLibrary.getCurse():
/// Excludes AscendersBane, Necronomicurse, CurseOfTheBell, Pride
const CURSE_POOL: &[CardId] = &[
    CardId::Clumsy,
    CardId::Decay,
    CardId::Doubt,
    CardId::Injury,
    CardId::Normality,
    CardId::Pain,
    CardId::Parasite,
    CardId::Regret,
    CardId::Shame,
    CardId::Writhe,
];

/// CardId lookup table stored in extra_data[15..21]
const CARD_LOOKUP_OFFSET: usize = 15;
const CARD_TYPE_COUNT: usize = 6;
const CARD_ENTRY_WIDTH: usize = 2;
const LAST_FLIPPED_1_OFFSET: usize = CARD_LOOKUP_OFFSET + CARD_TYPE_COUNT * CARD_ENTRY_WIDTH;
const LAST_FLIPPED_2_OFFSET: usize = LAST_FLIPPED_1_OFFSET + 1;

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
        let idx1 = run_state
            .rng_pool
            .card_rng
            .random_range(0, CURSE_POOL.len() as i32 - 1) as usize;
        card_types[3] = CURSE_POOL[idx1];
        let idx2 = run_state
            .rng_pool
            .card_rng
            .random_range(0, CURSE_POOL.len() as i32 - 1) as usize;
        card_types[4] = CURSE_POOL[idx2];
    } else {
        // Non-A15: 1 colorless uncommon + 1 curse
        card_types[3] =
            run_state.random_colorless_card(crate::content::cards::CardRarity::Uncommon);
        let idx = run_state
            .rng_pool
            .card_rng
            .random_range(0, CURSE_POOL.len() as i32 - 1) as usize;
        card_types[4] = CURSE_POOL[idx];
    }

    // Java: player.getStartCardForEvent()
    card_types[5] = match run_state.player_class {
        "Ironclad" => CardId::Bash,
        "Silent" => CardId::Neutralize,
        "Defect" => CardId::Zap,
        "Watcher" => CardId::Eruption,
        _ => CardId::Strike,
    };

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
        let mut jur = crate::runtime::rng::JavaUtilRandom::new(seed as u64);
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
    // [15..27] = CardId values plus previewed upgrades for each type index.
    // Java calls onPreviewObtainCard before duplicating the board pair.
    for &card_id in card_types.iter().take(CARD_TYPE_COUNT) {
        push_card_entry(
            extra_data,
            card_id,
            run_state.preview_obtain_card_upgrades(card_id, 0),
        );
    }
    // [27] = last flipped 1 pos (-1 if none)
    extra_data.push(-1);
    // [28] = last flipped 2 pos (-1 if none)
    extra_data.push(-1);
}

fn push_card_entry(extra_data: &mut Vec<i32>, card_id: CardId, upgrades: u8) {
    extra_data.push(card_id as i32);
    extra_data.push(upgrades as i32);
}

fn card_entry_at(run_state: &RunState, extra_data: &[i32], pos: usize) -> Option<(CardId, u8)> {
    let type_idx = extra_data[pos] as usize;
    let offset = CARD_LOOKUP_OFFSET + type_idx * CARD_ENTRY_WIDTH;
    let card_disc = *extra_data.get(offset)?;
    let upgrades = *extra_data.get(offset + 1)?;
    if upgrades < 0 {
        return None;
    }
    Some((card_id_from_i32(run_state, card_disc)?, upgrades as u8))
}

fn card_id_from_i32(run_state: &RunState, raw: i32) -> Option<CardId> {
    for rarity in [
        crate::content::cards::CardRarity::Common,
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Rare,
    ] {
        for &card_id in
            crate::engine::campfire_handler::card_pool_for_class(run_state.player_class, rarity)
        {
            if card_id as i32 == raw {
                return Some(card_id);
            }
        }
    }
    for rarity in [
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Rare,
    ] {
        for &card_id in crate::content::cards::colorless_pool_for_rarity(rarity) {
            if card_id as i32 == raw {
                return Some(card_id);
            }
        }
    }
    for &card_id in CURSE_POOL {
        if card_id as i32 == raw {
            return Some(card_id);
        }
    }
    for card_id in [
        CardId::Bash,
        CardId::Neutralize,
        CardId::Zap,
        CardId::Eruption,
    ] {
        if card_id as i32 == raw {
            return Some(card_id);
        }
    }
    None
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

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            vec![EventChoiceMeta::new("[Play Match and Keep!]")]
        }
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

                // Show face-up identity if this is the already flipped card in screen 2?
                // The prompt asks us to flip a card. If we are on screen 2, we could show the first flipped card's name.
                // But the user clicks "[Flip card X]".
                choices.push(EventChoiceMeta::new(format!("[Flip card {}]", pos)));
            }
            if event_state.current_screen == 2 && first != -1 {
                if let Some((first_card, upgrades)) = card_entry_at(run_state, ed, first as usize) {
                    let def = crate::content::cards::get_card_definition(first_card);
                    let upgrade_suffix = if upgrades > 0 { "+" } else { "" };
                    choices.insert(
                        0,
                        EventChoiceMeta::disabled(
                            format!("(First card: {}{})", def.name, upgrade_suffix),
                            "",
                        ),
                    );
                }
            }

            if choices.is_empty() {
                vec![EventChoiceMeta::new("[Leave]")]
            } else {
                choices
            }
        }
        3 | 4 => {
            let ed = &event_state.extra_data;
            let remaining = attempts_remaining(ed);
            let all_matched = ed[12] == 0xFFF;

            let mut prefix = "".to_string();
            if ed.len() > LAST_FLIPPED_2_OFFSET
                && ed[LAST_FLIPPED_1_OFFSET] != -1
                && ed[LAST_FLIPPED_2_OFFSET] != -1
            {
                let Some((card1, _)) =
                    card_entry_at(run_state, ed, ed[LAST_FLIPPED_1_OFFSET] as usize)
                else {
                    return vec![EventChoiceMeta::new("[Leave]")];
                };
                let Some((card2, _)) =
                    card_entry_at(run_state, ed, ed[LAST_FLIPPED_2_OFFSET] as usize)
                else {
                    return vec![EventChoiceMeta::new("[Leave]")];
                };
                let def1 = crate::content::cards::get_card_definition(card1);
                let def2 = crate::content::cards::get_card_definition(card2);
                if card1 == card2 {
                    prefix = format!("MATCH: {}! ", def1.name);
                } else {
                    prefix = format!("Mismatch: {} vs {}. ", def1.name, def2.name);
                }
            }

            if remaining <= 0 || all_matched || event_state.current_screen == 4 {
                vec![EventChoiceMeta::new(format!(
                    "{}[Leave] Game over. ({} attempts left)",
                    prefix, remaining
                ))]
            } else {
                vec![EventChoiceMeta::new(format!(
                    "{}[Continue] Flip more cards. ({} attempts left)",
                    prefix, remaining
                ))]
            }
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(_engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
        }
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
        }
        2 => {
            let first_pos = first_flipped(&event_state.extra_data) as usize;

            // To be perfectly aligned with choices array:
            let mut choice_counter = 0;
            // The choices array inserted disabled at 0
            let mut found_pos = None;

            if first_pos != std::usize::MAX {
                choice_counter += 1; // skip the (First card: XXX) disabled choice
            }

            for pos in 0..12usize {
                if !is_matched(&event_state.extra_data, pos) && pos != first_pos {
                    if choice_counter == choice_idx {
                        found_pos = Some(pos);
                        break;
                    }
                    choice_counter += 1;
                }
            }

            // Fallback for non-adjusted choices (e.g. from tests)
            if found_pos.is_none() {
                let mut avail: Vec<usize> = Vec::new();
                for pos in 0..12usize {
                    if !is_matched(&event_state.extra_data, pos) && pos != first_pos {
                        avail.push(pos);
                    }
                }
                if let Some(&p) = avail.get(choice_idx) {
                    found_pos = Some(p);
                }
            }

            if let Some(second_pos) = found_pos {
                if event_state.extra_data.len() > LAST_FLIPPED_2_OFFSET {
                    event_state.extra_data[LAST_FLIPPED_1_OFFSET] = first_pos as i32;
                    event_state.extra_data[LAST_FLIPPED_2_OFFSET] = second_pos as i32;
                }

                let first_card = card_entry_at(run_state, &event_state.extra_data, first_pos);
                let second_card = card_entry_at(run_state, &event_state.extra_data, second_pos);

                if let (Some((card_id, upgrades)), Some((second_id, _))) = (first_card, second_card)
                {
                    if card_id == second_id {
                        run_state.add_card_to_deck_with_upgrades_from(
                            card_id,
                            upgrades,
                            DomainEventSource::Event(EventId::MatchAndKeep),
                        );
                        set_matched(&mut event_state.extra_data, first_pos);
                        set_matched(&mut event_state.extra_data, second_pos);
                    }
                }

                event_state.extra_data[13] -= 1; // consume attempt
                event_state.extra_data[14] = -1; // reset first flipped

                let remaining = attempts_remaining(&event_state.extra_data);
                let all_matched = event_state.extra_data[12] == 0xFFF;
                if remaining <= 0 || all_matched {
                    event_state.current_screen = 4; // game over
                } else {
                    event_state.current_screen = 3; // continue
                }
            }
        }
        3 => {
            event_state.current_screen = 1; // back to first flip
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardType;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::selection::DomainEvent;

    #[test]
    fn match_and_keep_start_card_matches_java_player_get_start_card_for_event() {
        let cases = [
            ("Ironclad", CardId::Bash),
            ("Silent", CardId::Neutralize),
            ("Defect", CardId::Zap),
            ("Watcher", CardId::Eruption),
        ];

        for (player_class, expected_start_card) in cases {
            let mut run_state = RunState::new(12345, 0, false, player_class);
            let mut extra_data = Vec::new();

            init_match_game_board(&mut run_state, &mut extra_data);

            assert_eq!(
                card_id_from_i32(
                    &run_state,
                    extra_data[CARD_LOOKUP_OFFSET + 5 * CARD_ENTRY_WIDTH],
                )
                .unwrap(),
                expected_start_card,
                "Java {}.getStartCardForEvent() must feed GremlinMatchGame.initializeCards",
                player_class
            );
        }
    }

    #[test]
    fn generated_board_stores_preview_obtain_upgrades_like_java() {
        let mut run_state = RunState::new(12345, 0, false, "Ironclad");
        run_state.relics.push(RelicState::new(RelicId::MoltenEgg));
        run_state.relics.push(RelicState::new(RelicId::ToxicEgg));
        run_state.relics.push(RelicState::new(RelicId::FrozenEgg));
        let mut extra_data = Vec::new();

        init_match_game_board(&mut run_state, &mut extra_data);

        for type_idx in 0..CARD_TYPE_COUNT {
            let offset = CARD_LOOKUP_OFFSET + type_idx * CARD_ENTRY_WIDTH;
            let card_id = card_id_from_i32(&run_state, extra_data[offset]).unwrap();
            let upgrades = extra_data[offset + 1];
            let def = crate::content::cards::get_card_definition(card_id);
            if def.card_type == CardType::Curse {
                assert_eq!(upgrades, 0);
            } else {
                assert_eq!(
                    upgrades, 1,
                    "GremlinMatchGame calls onPreviewObtainCard before duplicating board cards"
                );
            }
        }
    }

    #[test]
    fn matching_cards_obtain_previewed_copy_with_event_source() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut event_state = EventState::new(EventId::MatchAndKeep);
        event_state.current_screen = 1;
        event_state.extra_data = board_with_entries(&[
            (CardId::Bash, 1),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Clumsy, 0),
            (CardId::IronWave, 0),
            (CardId::Cleave, 0),
        ]);
        run_state.event_state = Some(event_state);

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut run_state, 0);
        handle_choice(&mut engine_state, &mut run_state, 1);

        let obtained = run_state.master_deck.last().unwrap();
        assert_eq!(obtained.id, CardId::Bash);
        assert_eq!(obtained.upgrades, 1);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                card,
                source: DomainEventSource::Event(EventId::MatchAndKeep),
            } if card.id == CardId::Bash && card.upgrades == 1
        )));
    }

    #[test]
    fn matching_uses_card_id_not_board_type_index_like_java() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut event_state = EventState::new(EventId::MatchAndKeep);
        event_state.current_screen = 1;
        event_state.extra_data = board_with_entries(&[
            (CardId::Bash, 0),
            (CardId::Bash, 0),
            (CardId::Defend, 0),
            (CardId::Clumsy, 0),
            (CardId::IronWave, 0),
            (CardId::Cleave, 0),
        ]);
        run_state.event_state = Some(event_state);

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut run_state, 0);
        handle_choice(&mut engine_state, &mut run_state, 1);

        let state = run_state.event_state.as_ref().unwrap();
        assert!(is_matched(&state.extra_data, 0));
        assert!(is_matched(&state.extra_data, 1));
        assert_eq!(run_state.master_deck.last().unwrap().id, CardId::Bash);
    }

    fn board_with_entries(entries: &[(CardId, u8); CARD_TYPE_COUNT]) -> Vec<i32> {
        let mut extra_data = vec![0, 0, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 0, 5, -1];
        for &(card_id, upgrades) in entries {
            push_card_entry(&mut extra_data, card_id, upgrades);
        }
        extra_data.push(-1);
        extra_data.push(-1);
        extra_data
    }
}
