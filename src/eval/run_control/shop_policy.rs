use crate::ai::noncombat_decision_v1::{
    render_noncombat_decision_record_validation_errors, validate_noncombat_decision_record_v1,
    NonCombatDecisionRecordV1,
};
use crate::content::cards::get_card_definition;
use crate::state::core::{ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::RunControlTraceAnnotationV1;

pub(super) fn apply_shop_policy_purge(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let EngineState::Shop(shop) = &session.engine_state else {
        return Ok(None);
    };
    let context =
        crate::ai::shop_policy_v1::build_shop_decision_context_v1(&session.run_state, shop);
    let decision = crate::ai::shop_policy_v1::plan_shop_decision_v1(
        &context,
        &crate::ai::shop_policy_v1::ShopPolicyConfigV1::default(),
    );
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let crate::ai::shop_policy_v1::ShopPolicyActionV1::Purge {
        deck_index,
        card,
        confidence,
        reason,
    } = decision.action
    else {
        return Ok(None);
    };

    let card_name = get_card_definition(card).name;
    let outcome = session
        .apply_input(ClientInput::PurgeCard(deck_index))?
        .with_trace_annotations(vec![noncombat_policy_annotation(noncombat_record)?]);
    Ok(Some((
        outcome,
        format!(
            "shop policy: purge {card_name} confidence={confidence:.2} reason={reason} label_role={}",
            decision.label_role
        ),
    )))
}

fn noncombat_policy_annotation(
    record: NonCombatDecisionRecordV1,
) -> Result<RunControlTraceAnnotationV1, String> {
    validate_noncombat_decision_record_v1(&record).map_err(|errors| {
        format!(
            "shop policy produced invalid NonCombatDecisionRecordV1: {}",
            render_noncombat_decision_record_validation_errors(&errors)
        )
    })?;
    Ok(RunControlTraceAnnotationV1::NonCombatPolicyDecision { record })
}
