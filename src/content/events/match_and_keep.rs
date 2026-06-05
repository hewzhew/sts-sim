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
use crate::state::events::{
    EventActionKind, EventChoiceMeta, EventEffect, EventId, EventOption, EventOptionSemantics,
    EventOptionTransition, EventRandomOutcomeKind, EventState,
};
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

fn build_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
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

                choices.push(EventChoiceMeta::new(format!("[Flip card {}]", pos)));
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

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    let choices = build_choices(run_state, event_state);

    match event_state.current_screen {
        0 => choices
            .into_iter()
            .map(|ui| {
                EventOption::new(
                    ui,
                    EventOptionSemantics {
                        action: EventActionKind::Continue,
                        transition: EventOptionTransition::AdvanceScreen,
                        ..Default::default()
                    },
                )
            })
            .collect(),
        1 | 2 => {
            if choices.len() == 1 && choices[0].text.contains("[Leave]") {
                return vec![EventOption::new(
                    choices.into_iter().next().unwrap(),
                    EventOptionSemantics {
                        action: EventActionKind::Leave,
                        transition: EventOptionTransition::Complete,
                        terminal: true,
                        ..Default::default()
                    },
                )];
            }
            choices
                .into_iter()
                .map(|ui| {
                    EventOption::new(
                        ui,
                        EventOptionSemantics {
                            action: EventActionKind::Special,
                            effects: vec![EventEffect::RandomOutcome {
                                kind: EventRandomOutcomeKind::MatchAndKeepFlip,
                            }],
                            ..Default::default()
                        },
                    )
                })
                .collect()
        }
        3 => {
            let leaves = choices
                .first()
                .is_some_and(|choice| choice.text.contains("[Leave]"));
            choices
                .into_iter()
                .map(|ui| {
                    EventOption::new(
                        ui,
                        EventOptionSemantics {
                            action: if leaves {
                                EventActionKind::Leave
                            } else {
                                EventActionKind::Continue
                            },
                            transition: if leaves {
                                EventOptionTransition::Complete
                            } else {
                                EventOptionTransition::AdvanceScreen
                            },
                            terminal: leaves,
                            ..Default::default()
                        },
                    )
                })
                .collect()
        }
        _ => choices
            .into_iter()
            .map(|ui| {
                EventOption::new(
                    ui,
                    EventOptionSemantics {
                        action: EventActionKind::Leave,
                        transition: EventOptionTransition::Complete,
                        terminal: true,
                        ..Default::default()
                    },
                )
            })
            .collect(),
    }
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    get_options(run_state, event_state)
        .into_iter()
        .map(|option| option.ui)
        .collect()
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
            let first_raw = first_flipped(&event_state.extra_data);
            if first_raw < 0 {
                run_state.event_state = Some(event_state);
                return;
            }
            let first_pos = first_raw as usize;

            let mut available = Vec::new();
            for pos in 0..12usize {
                if !is_matched(&event_state.extra_data, pos) && pos != first_pos {
                    available.push(pos);
                }
            }

            if let Some(&second_pos) = available.get(choice_idx) {
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
    use crate::state::events::{
        EventActionKind, EventEffect, EventOptionTransition, EventRandomOutcomeKind,
    };
    use crate::state::selection::DomainEvent;

    #[test]
    fn structured_options_expose_hidden_flip_boundaries_without_revealing_cards() {
        let run_state = RunState::new(1, 0, false, "Ironclad");
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

        let options = crate::engine::event_handler::try_get_structured_event_options_for_state(
            &run_state,
            &event_state,
        )
        .expect("Match and Keep should expose structured event options");

        assert_eq!(options.len(), 12);
        assert_eq!(options[0].ui.text, "[Flip card 0]");
        assert_eq!(options[0].semantics.action, EventActionKind::Special);
        assert_eq!(
            options[0].semantics.effects,
            vec![EventEffect::RandomOutcome {
                kind: EventRandomOutcomeKind::MatchAndKeepFlip,
            }]
        );
        assert_eq!(options[0].semantics.transition, EventOptionTransition::None);

        let mut second_flip = event_state;
        second_flip.current_screen = 2;
        second_flip.extra_data[14] = 0;
        let second_options =
            crate::engine::event_handler::try_get_structured_event_options_for_state(
                &run_state,
                &second_flip,
            )
            .expect("Match and Keep second flip should expose structured event options");

        assert_eq!(second_options.len(), 11);
        assert!(second_options
            .iter()
            .all(|option| option.ui.text != "[Flip card 0]"));
        assert!(second_options.iter().all(|option| {
            option.semantics.effects
                == vec![EventEffect::RandomOutcome {
                    kind: EventRandomOutcomeKind::MatchAndKeepFlip,
                }]
        }));
    }

    #[test]
    fn structured_intro_result_and_complete_boundaries() {
        let run_state = RunState::new(1, 0, false, "Ironclad");
        let intro = EventState::new(EventId::MatchAndKeep);
        let intro_options =
            crate::engine::event_handler::try_get_structured_event_options_for_state(
                &run_state, &intro,
            )
            .expect("Match and Keep intro should expose structured event options");

        assert_eq!(intro_options[0].semantics.action, EventActionKind::Continue);
        assert_eq!(
            intro_options[0].semantics.transition,
            EventOptionTransition::AdvanceScreen
        );

        let mut result = EventState::new(EventId::MatchAndKeep);
        result.current_screen = 3;
        result.extra_data = board_with_entries(&[
            (CardId::Bash, 1),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Clumsy, 0),
            (CardId::IronWave, 0),
            (CardId::Cleave, 0),
        ]);
        let result_options =
            crate::engine::event_handler::try_get_structured_event_options_for_state(
                &run_state, &result,
            )
            .expect("Match and Keep result screen should expose structured event options");

        assert_eq!(
            result_options[0].semantics.action,
            EventActionKind::Continue
        );
        assert_eq!(
            result_options[0].semantics.transition,
            EventOptionTransition::AdvanceScreen
        );

        let mut complete = result;
        complete.current_screen = 4;
        let complete_options =
            crate::engine::event_handler::try_get_structured_event_options_for_state(
                &run_state, &complete,
            )
            .expect("Match and Keep complete screen should expose structured event options");

        assert_eq!(complete_options[0].semantics.action, EventActionKind::Leave);
        assert_eq!(
            complete_options[0].semantics.transition,
            EventOptionTransition::Complete
        );
        assert!(complete_options[0].semantics.terminal);
    }

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
        handle_choice(&mut engine_state, &mut run_state, 0);

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
    fn matching_cards_run_obtain_hooks_before_card_obtained_event() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.push(RelicState::new(RelicId::CeramicFish));
        let mut event_state = EventState::new(EventId::MatchAndKeep);
        event_state.current_screen = 1;
        event_state.extra_data = board_with_entries(&[
            (CardId::Bash, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Clumsy, 0),
            (CardId::IronWave, 0),
            (CardId::Cleave, 0),
        ]);
        run_state.event_state = Some(event_state);

        let mut engine_state = EngineState::EventRoom;
        handle_choice(&mut engine_state, &mut run_state, 0);
        handle_choice(&mut engine_state, &mut run_state, 0);

        let events = run_state.take_emitted_events();
        let fish_gold_pos = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    DomainEvent::GoldChanged {
                        delta: 9,
                        source: DomainEventSource::Event(EventId::MatchAndKeep),
                        ..
                    }
                )
            })
            .expect(
                "Ceramic Fish should run from the matched card ShowCardAndObtainEffect obtain hook",
            );
        let obtained_pos = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    DomainEvent::CardObtained {
                        card,
                        source: DomainEventSource::Event(EventId::MatchAndKeep),
                    } if card.id == CardId::Bash
                )
            })
            .expect("Match and Keep should obtain a copy of the matched card");

        assert!(
            fish_gold_pos < obtained_pos,
            "Java GremlinMatchGame queues ShowCardAndObtainEffect for the matched card; that effect runs onObtainCard before Soul.obtain adds the card"
        );
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
        handle_choice(&mut engine_state, &mut run_state, 0);

        let state = run_state.event_state.as_ref().unwrap();
        assert!(is_matched(&state.extra_data, 0));
        assert!(is_matched(&state.extra_data, 1));
        assert_eq!(run_state.master_deck.last().unwrap().id, CardId::Bash);
    }

    #[test]
    fn second_flip_choices_do_not_include_synthetic_disabled_info_row() {
        let run_state = RunState::new(1, 0, false, "Ironclad");
        let mut event_state = EventState::new(EventId::MatchAndKeep);
        event_state.current_screen = 2;
        event_state.extra_data = board_with_entries(&[
            (CardId::Bash, 0),
            (CardId::Bash, 0),
            (CardId::Defend, 0),
            (CardId::Clumsy, 0),
            (CardId::IronWave, 0),
            (CardId::Cleave, 0),
        ]);
        event_state.extra_data[14] = 0;

        let choices = get_choices(&run_state, &event_state);

        assert_eq!(choices[0].text, "[Flip card 1]");
        assert!(
            choices.iter().all(|choice| !choice.disabled),
            "Java GremlinMatchGame exposes board card hitboxes, not a disabled dialog choice for the first flipped card"
        );
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
