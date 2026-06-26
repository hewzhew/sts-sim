use crate::state::core::ClientInput;
use crate::state::events::{EventId, EventOwnerPolicyKind};
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventOwnerPolicyGap {
    MissingEventState,
    MissingMarkedPolicy(EventId),
    AmbiguousMarkedPolicy { event_id: EventId, found: usize },
}

pub fn conservative_owner_policy_input(
    run_state: &RunState,
) -> Result<ClientInput, EventOwnerPolicyGap> {
    let event_id = run_state
        .event_state
        .as_ref()
        .map(|event| event.id)
        .ok_or(EventOwnerPolicyGap::MissingEventState)?;
    let marked_indices = crate::engine::event_handler::get_event_options(run_state)
        .iter()
        .enumerate()
        .filter(|(_, option)| {
            !option.ui.disabled
                && option.semantics.owner_policy == EventOwnerPolicyKind::ConservativeAuto
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let [index] = marked_indices.as_slice() else {
        return if marked_indices.is_empty() {
            Err(EventOwnerPolicyGap::MissingMarkedPolicy(event_id))
        } else {
            Err(EventOwnerPolicyGap::AmbiguousMarkedPolicy {
                event_id,
                found: marked_indices.len(),
            })
        };
    };
    Ok(ClientInput::EventChoice(*index))
}
