use crate::ai::noncombat_decision_v1::{
    render_noncombat_decision_record_validation_errors, validate_noncombat_decision_record_v1,
    NonCombatDecisionRecordV1,
};
use crate::state::core::{ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::RunControlTraceAnnotationV1;

pub(super) fn apply_boss_relic_policy_pick(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let EngineState::BossRelicSelect(choice) = &session.engine_state else {
        return Ok(None);
    };

    let context = crate::ai::boss_relic_policy_v1::build_boss_relic_decision_context_v1(
        &session.run_state,
        choice.relics.clone(),
    );
    let decision = crate::ai::boss_relic_policy_v1::plan_boss_relic_decision_v1(
        &context,
        &crate::ai::boss_relic_policy_v1::BossRelicPolicyConfigV1::default(),
    );
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let crate::ai::boss_relic_policy_v1::BossRelicPolicyActionV1::Pick {
        index,
        relic,
        confidence,
        reason,
    } = decision.action
    else {
        return Ok(None);
    };

    let outcome = session
        .apply_input(ClientInput::SubmitRelicChoice(index))?
        .with_trace_annotations(vec![noncombat_policy_annotation(noncombat_record)?]);
    Ok(Some((
        outcome,
        format!(
            "boss relic policy: {:?} confidence={confidence:.2} reason={reason} label_role={}",
            relic, decision.label_role
        ),
    )))
}

fn noncombat_policy_annotation(
    record: NonCombatDecisionRecordV1,
) -> Result<RunControlTraceAnnotationV1, String> {
    validate_noncombat_decision_record_v1(&record).map_err(|errors| {
        format!(
            "boss relic policy produced invalid NonCombatDecisionRecordV1: {}",
            render_noncombat_decision_record_validation_errors(&errors)
        )
    })?;
    Ok(RunControlTraceAnnotationV1::NonCombatPolicyDecision { record })
}
