use super::*;
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
