use crate::ai::noncombat_decision_v1::{
    render_noncombat_decision_record_validation_errors, validate_noncombat_decision_record_v1,
    NonCombatDecisionRecordV1,
};
use crate::state::core::{CampfireChoice, ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::RunControlTraceAnnotationV1;

pub(super) fn apply_campfire_policy_rest(
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
    let crate::ai::campfire_policy_v1::CampfirePolicyActionV1::Rest { confidence, reason } =
        decision.action
    else {
        return Ok(None);
    };

    let outcome = session
        .apply_input(ClientInput::CampfireOption(CampfireChoice::Rest))?
        .with_trace_annotations(vec![noncombat_policy_annotation(noncombat_record)?]);
    Ok(Some((
        outcome,
        format!(
            "campfire policy: rest confidence={confidence:.2} reason={reason} label_role={}",
            decision.label_role
        ),
    )))
}

fn noncombat_policy_annotation(
    record: NonCombatDecisionRecordV1,
) -> Result<RunControlTraceAnnotationV1, String> {
    validate_noncombat_decision_record_v1(&record).map_err(|errors| {
        format!(
            "campfire policy produced invalid NonCombatDecisionRecordV1: {}",
            render_noncombat_decision_record_validation_errors(&errors)
        )
    })?;
    Ok(RunControlTraceAnnotationV1::NonCombatPolicyDecision { record })
}
