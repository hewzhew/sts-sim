use crate::eval::run_control::{build_decision_surface, RunControlSession};
use crate::state::core::{ClientInput, EngineState};

const MAX_EVENT_OPTIONS_PER_BRANCH: usize = 4;

#[derive(Clone, Debug)]
pub(crate) struct EventBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
}

pub(crate) fn event_branch_options(session: &RunControlSession) -> Option<Vec<EventBranchOption>> {
    if !matches!(session.engine_state, EngineState::EventRoom) {
        return None;
    }
    let event_options = crate::engine::event_handler::get_event_options(&session.run_state);
    let surface = build_decision_surface(session);
    let mut branch_options = Vec::new();

    for candidate in &surface.view.candidates {
        let Some(ClientInput::EventChoice(index)) = candidate.action.executable_input() else {
            continue;
        };
        let Some(event_option) = event_options.get(index) else {
            continue;
        };
        if event_option.ui.disabled {
            return None;
        }
        branch_options.push(EventBranchOption {
            label: candidate.label.clone(),
            command: candidate.action.command_hint(),
        });
    }

    if branch_options.is_empty() || branch_options.len() > MAX_EVENT_OPTIONS_PER_BRANCH {
        return None;
    }
    Some(branch_options)
}
