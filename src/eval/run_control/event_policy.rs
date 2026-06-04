use crate::state::core::{ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_event_policy_choice(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let EngineState::EventRoom = session.engine_state else {
        return Ok(None);
    };
    let Some(event_state) = session.run_state.event_state.as_ref() else {
        return Ok(None);
    };

    let event_id = event_state.id;
    let options = crate::engine::event_handler::get_event_options(&session.run_state);
    let context = crate::ai::event_policy_v1::build_event_decision_context_v1(
        &session.run_state,
        event_id,
        options,
    );
    let decision = crate::ai::event_policy_v1::plan_event_decision_v1(
        &context,
        &crate::ai::event_policy_v1::EventPolicyConfigV1::default(),
    );
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let crate::ai::event_policy_v1::EventPolicyActionV1::Pick {
        index,
        label,
        confidence,
        reason,
    } = decision.action
    else {
        return Ok(None);
    };

    let outcome = session
        .apply_input(ClientInput::EventChoice(index))?
        .with_trace_annotations(vec![
            super::noncombat_policy_annotation::noncombat_policy_annotation(
                "event policy",
                noncombat_record,
            )?,
        ]);
    Ok(Some((
        outcome,
        format!(
            "event policy: {label} confidence={confidence:.2} reason={reason} label_role={}",
            decision.label_role
        ),
    )))
}
