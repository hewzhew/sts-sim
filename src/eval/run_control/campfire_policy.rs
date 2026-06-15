use crate::state::core::{CampfireChoice, ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_campfire_policy_action(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return Ok(None);
    }
    let choices = crate::engine::campfire_handler::get_available_options(&session.run_state);
    let context = crate::ai::campfire_policy_v1::build_campfire_decision_context_v1(
        &session.run_state,
        choices,
    );
    let decision = crate::ai::campfire_policy_v1::plan_campfire_decision_v1(
        &context,
        &crate::ai::campfire_policy_v1::CampfirePolicyConfigV1::default(),
    );
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let (choice, verb, confidence, reason) = match decision.selected_plan.action.clone() {
        crate::ai::campfire_policy_v1::CampfirePolicyActionV1::Rest { confidence, reason } => {
            (CampfireChoice::Rest, "rest", confidence, reason)
        }
        crate::ai::campfire_policy_v1::CampfirePolicyActionV1::Smith {
            deck_index,
            confidence,
            reason,
        } => (
            CampfireChoice::Smith(deck_index),
            "smith",
            confidence,
            reason,
        ),
        crate::ai::campfire_policy_v1::CampfirePolicyActionV1::Stop { .. } => return Ok(None),
    };

    let outcome = session
        .apply_input(ClientInput::CampfireOption(choice))?
        .with_trace_annotations(vec![
            super::noncombat_policy_annotation::noncombat_policy_annotation(
                "campfire policy",
                noncombat_record,
            )?,
        ]);
    Ok(Some((
        outcome,
        format!(
            "campfire policy: {verb} confidence={confidence:.2} reason={reason} label_role={}",
            decision.label_role
        ),
    )))
}
