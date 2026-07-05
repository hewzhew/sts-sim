use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};
use sts_simulator::state::core::ClientInput;

use super::owner_commands::visible_input_decision;
use super::owner_model::{OwnerDecision, OwnerRoutine};

pub(super) fn event_owner_decision(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> OwnerDecision {
    match sts_simulator::content::events::owner_policy::event_owner_policy_action(
        &session.engine_state,
        &session.run_state,
    ) {
        Ok(sts_simulator::content::events::owner_policy::EventOwnerAction::ChooseOption(
            selector,
        )) => visible_event_option_decision(session, surface, &selector),
        Ok(sts_simulator::content::events::owner_policy::EventOwnerAction::SubmitSelection(
            resolution,
        )) => visible_input_decision(surface, ClientInput::SubmitSelection(resolution)),
        Err(err) => OwnerDecision::Gap(format!("{err:?}")),
    }
}

fn visible_event_option_decision(
    session: &RunControlSession,
    surface: &DecisionSurface,
    selector: &sts_simulator::content::events::owner_policy::EventOwnerOptionSelector,
) -> OwnerDecision {
    let Some(event) = session.run_state.event_state.as_ref() else {
        return OwnerDecision::Gap("event owner requires event_state".to_string());
    };
    let options = sts_simulator::engine::event_handler::get_event_options(&session.run_state);
    let matches = surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let Some(DecisionCandidateKey::EventOption {
                event_id,
                screen,
                option_index,
                ..
            }) = candidate.key
            else {
                return None;
            };
            if event_id != event.id || screen != event.current_screen {
                return None;
            }
            let option = options.get(option_index)?;
            if option.ui.disabled || !selector.matches(option_index, &option.semantics) {
                return None;
            }
            candidate.action.executable_command()
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [command] => OwnerDecision::Routine(OwnerRoutine::Command(command.clone())),
        [] => OwnerDecision::Gap(format!("event selector {selector:?} has no visible option")),
        _ => OwnerDecision::Gap(format!(
            "event selector {selector:?} matched {} visible options",
            matches.len()
        )),
    }
}
