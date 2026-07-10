use super::*;
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::engine::event_handler::get_event_options;
use crate::state::events::{EventId, EventState};

fn event_run(event_id: EventId, screen: usize) -> RunState {
    let mut run_state = RunState::new(1, 0, false, "Ironclad");
    let mut event_state = EventState::new(event_id);
    event_state.current_screen = screen;
    run_state.event_state = Some(event_state);
    run_state
}

fn assert_unique_selector(run_state: &RunState, expected: EventOwnerOptionSelector) {
    let action = event_owner_policy_action(&EngineState::EventRoom, run_state).unwrap();
    let EventOwnerAction::ChooseOption(selector) = action else {
        panic!("event-room owner must choose an event option");
    };
    assert_eq!(selector, expected);

    let options = get_event_options(run_state);
    let matches = options
        .iter()
        .enumerate()
        .filter(|(index, option)| {
            !option.ui.disabled && selector.matches(*index, &option.semantics)
        })
        .count();
    assert_eq!(matches, 1, "selector must resolve to one enabled real option");
}

#[test]
fn forced_flow_events_select_one_real_option_on_every_screen() {
    let cases = [
        (EventId::GremlinWheelGame, 0, action(EventActionKind::Special)),
        (EventId::GremlinWheelGame, 1, action(EventActionKind::Leave)),
        (EventId::Lab, 0, action(EventActionKind::Gain)),
        (EventId::Lab, 1, action(EventActionKind::Leave)),
        (EventId::Colosseum, 0, action(EventActionKind::Continue)),
        (EventId::Colosseum, 1, action(EventActionKind::Fight)),
        (EventId::Colosseum, 2, action(EventActionKind::Leave)),
        (EventId::Colosseum, 3, action(EventActionKind::Leave)),
        (EventId::TheJoust, 0, action(EventActionKind::Continue)),
        (EventId::TheJoust, 1, option_index(0)),
        (EventId::TheJoust, 2, action(EventActionKind::Continue)),
        (EventId::TheJoust, 3, action(EventActionKind::Continue)),
        (EventId::TheJoust, 4, action(EventActionKind::Leave)),
        (EventId::TheJoust, 5, action(EventActionKind::Leave)),
    ];

    for (event_id, screen, expected) in cases {
        assert_unique_selector(&event_run(event_id, screen), expected);
    }
}

#[test]
fn golden_shrine_uses_omamori_for_desecrate_and_otherwise_prays() {
    let run_state = event_run(EventId::GoldenShrine, 0);
    assert_unique_selector(&run_state, effect(EventEffect::GainGold(100)));

    let mut protected = event_run(EventId::GoldenShrine, 0);
    protected.relics.push(RelicState::new(RelicId::Omamori));
    assert_unique_selector(
        &protected,
        effect(EventEffect::ObtainCurse {
            count: 1,
            kind: EventCardKind::Specific(CardId::Regret),
        }),
    );
}

#[test]
fn fountain_upgrade_blacksmith_duplicate_and_note_use_real_deck_facts() {
    let empty_fountain = event_run(EventId::FountainOfCurseCleansing, 0);
    assert_unique_selector(&empty_fountain, action(EventActionKind::Leave));

    let mut cursed = event_run(EventId::FountainOfCurseCleansing, 0);
    cursed.add_card_to_deck(CardId::Injury);
    assert_unique_selector(&cursed, action(EventActionKind::DeckOperation));

    let mut empty_upgrade = event_run(EventId::UpgradeShrine, 0);
    empty_upgrade.master_deck.clear();
    assert_unique_selector(&empty_upgrade, action(EventActionKind::Leave));
    let mut upgrade = event_run(EventId::UpgradeShrine, 0);
    upgrade.add_card_to_deck(CardId::Bash);
    assert_unique_selector(&upgrade, action(EventActionKind::DeckOperation));

    let mut forge = event_run(EventId::AccursedBlacksmith, 0);
    forge.add_card_to_deck(CardId::Bash);
    assert_unique_selector(&forge, action(EventActionKind::DeckOperation));
    let mut plain_blacksmith = event_run(EventId::AccursedBlacksmith, 0);
    plain_blacksmith.master_deck.clear();
    assert_unique_selector(&plain_blacksmith, action(EventActionKind::Leave));

    let mut empty_duplicate = event_run(EventId::Duplicator, 0);
    empty_duplicate.master_deck.clear();
    assert_unique_selector(&empty_duplicate, action(EventActionKind::Leave));
    let mut premium_duplicate = event_run(EventId::Duplicator, 0);
    premium_duplicate.add_card_to_deck(CardId::Offering);
    assert_unique_selector(
        &premium_duplicate,
        action(EventActionKind::DeckOperation),
    );

    let default_note = event_run(EventId::NoteForYourself, 1);
    assert_unique_selector(&default_note, action(EventActionKind::Decline));
    let mut useful_note = event_run(EventId::NoteForYourself, 1);
    useful_note.note_for_yourself_card = CardId::Offering;
    useful_note.add_card_to_deck(CardId::Strike);
    assert_unique_selector(&useful_note, action(EventActionKind::DeckOperation));
}

#[test]
fn deck_positive_events_cover_intro_and_completion_screens() {
    let cases = [
        (EventId::GoldenShrine, 1, action(EventActionKind::Leave)),
        (
            EventId::FountainOfCurseCleansing,
            1,
            action(EventActionKind::Leave),
        ),
        (EventId::UpgradeShrine, 1, action(EventActionKind::Leave)),
        (
            EventId::AccursedBlacksmith,
            1,
            action(EventActionKind::Leave),
        ),
        (EventId::Duplicator, 1, action(EventActionKind::Leave)),
        (EventId::NoteForYourself, 0, action(EventActionKind::Continue)),
        (EventId::NoteForYourself, 2, action(EventActionKind::Leave)),
    ];
    for (event_id, screen, expected) in cases {
        assert_unique_selector(&event_run(event_id, screen), expected);
    }
}
