use crate::ai::noncombat_decision_v1::{
    render_noncombat_decision_record_validation_errors, validate_noncombat_decision_record_v1,
    NonCombatDecisionRecordV1,
};
use crate::state::core::{ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::RunControlTraceAnnotationV1;

pub(super) fn apply_run_choice_policy_purge_curse(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return Ok(None);
    };
    let context = crate::ai::run_choice_policy_v1::build_run_choice_decision_context_v1(
        &session.run_state,
        choice,
    );
    let decision = crate::ai::run_choice_policy_v1::plan_run_choice_decision_v1(
        &context,
        &crate::ai::run_choice_policy_v1::RunChoicePolicyConfigV1::default(),
    );
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let crate::ai::run_choice_policy_v1::RunChoicePolicyActionV1::SelectDeckIndices {
        indices,
        labels,
        confidence,
        reason,
    } = decision.action
    else {
        return Ok(None);
    };

    let outcome = session
        .apply_input(ClientInput::SubmitDeckSelect(indices))?
        .with_trace_annotations(vec![noncombat_policy_annotation(noncombat_record)?]);
    Ok(Some((
        outcome,
        format!(
            "run choice policy: purge {} confidence={confidence:.2} reason={reason} label_role={}",
            labels.join(", "),
            decision.label_role
        ),
    )))
}

fn noncombat_policy_annotation(
    record: NonCombatDecisionRecordV1,
) -> Result<RunControlTraceAnnotationV1, String> {
    validate_noncombat_decision_record_v1(&record).map_err(|errors| {
        format!(
            "run choice policy produced invalid NonCombatDecisionRecordV1: {}",
            render_noncombat_decision_record_validation_errors(&errors)
        )
    })?;
    Ok(RunControlTraceAnnotationV1::NonCombatPolicyDecision { record })
}
