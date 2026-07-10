use crate::state::events::EventActionKind;
use crate::state::run::RunState;

use super::{action, event_screen, option_index, EventOwnerOptionSelector};

pub(super) fn gremlin_wheel_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Special),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn lab_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Gain),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn colosseum_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 => action(EventActionKind::Continue),
        1 => action(EventActionKind::Fight),
        _ => action(EventActionKind::Leave),
    }
}

pub(super) fn the_joust_choice(run_state: &RunState) -> EventOwnerOptionSelector {
    match event_screen(run_state) {
        0 | 2 | 3 => action(EventActionKind::Continue),
        1 => option_index(0),
        _ => action(EventActionKind::Leave),
    }
}
