use std::collections::{BTreeMap, BTreeSet};

use crate::state::core::{ClientInput, EngineState};
use crate::state::rewards::{RewardCard, RewardItem};

use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::RunControlTraceAnnotationV1;

pub(super) fn apply_card_reward_policy_pick(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    if let Some(cards) = active_pending_reward_cards(session) {
        return apply_policy_to_pending_cards(session, cards);
    }

    let Some((reward_index, cards)) = visible_card_reward_item(session) else {
        return Ok(None);
    };
    let decision = card_reward_decision(session, &cards);
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let trace_annotation = card_reward_policy_trace_annotation(&decision, noncombat_record)?;
    let crate::ai::card_reward_policy_v1::CardRewardPolicyActionV1::Pick {
        index,
        card,
        confidence,
        reason,
    } = decision.action
    else {
        return Ok(None);
    };

    session.apply_input(ClientInput::ClaimReward(reward_index))?;
    let Some(opened_cards) = active_pending_reward_cards(session) else {
        return Err(
            "card reward policy opened a reward item but no pending card choice appeared"
                .to_string(),
        );
    };
    if opened_cards.len() <= index || opened_cards[index].id != card {
        return Err(
            "card reward policy opened a reward item but the pending card choices drifted"
                .to_string(),
        );
    }
    let outcome = session
        .apply_input_without_manual_card_reward_trace(ClientInput::SelectCard(index))?
        .with_trace_annotations(vec![trace_annotation]);
    Ok(Some((
        outcome,
        card_reward_summary(card, confidence, &reason, decision.label_role),
    )))
}

pub(super) fn apply_card_reward_item_open(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let Some((reward_index, _cards)) = visible_card_reward_item(session) else {
        return Ok(None);
    };
    let decision = card_reward_decision(session, &_cards);
    let crate::ai::card_reward_policy_v1::CardRewardPolicyActionV1::Stop { disposition, .. } =
        decision.action
    else {
        return Ok(None);
    };
    if disposition
        == crate::ai::card_reward_policy_v1::CardRewardStopDispositionV1::KeepRewardItemClosed
    {
        return Ok(None);
    }
    let outcome = session.apply_input(ClientInput::ClaimReward(reward_index))?;
    Ok(Some((
        outcome,
        "card reward: opened card reward item".to_string(),
    )))
}

pub(super) fn apply_recorded_card_reward_pick(
    session: &mut RunControlSession,
    index: usize,
) -> Result<RunControlCommandOutcome, String> {
    if let Some(cards) = active_pending_reward_cards(session) {
        return apply_recorded_pick_to_pending_cards(session, cards, index);
    }

    let Some((reward_index, cards)) = visible_card_reward_item(session) else {
        return Err(
            "rp <idx> is only valid on a card reward item or card reward screen".to_string(),
        );
    };
    let decision = recorded_card_reward_decision(session, &cards, index)?;
    let selected_card = decision.candidates[index].card;
    let record = selected_card_reward_record(
        &decision,
        "run_control_recorded_card_reward_pick_v1",
        "human_recorded_pick",
    );
    let trace_annotation = card_reward_policy_trace_annotation(&decision, record)?;

    session.apply_input(ClientInput::ClaimReward(reward_index))?;
    let Some(opened_cards) = active_pending_reward_cards(session) else {
        return Err(
            "recorded card reward pick opened a reward item but no pending card choice appeared"
                .to_string(),
        );
    };
    if opened_cards.len() <= index || opened_cards[index].id != selected_card {
        return Err(
            "recorded card reward pick opened a reward item but choices drifted".to_string(),
        );
    }

    Ok(session
        .apply_input_without_manual_card_reward_trace(ClientInput::SelectCard(index))?
        .with_trace_annotations(vec![trace_annotation]))
}

pub(super) fn card_reward_policy_stop_annotation(
    session: &RunControlSession,
) -> Result<Option<(RunControlTraceAnnotationV1, String)>, String> {
    let cards = active_pending_reward_cards(session)
        .or_else(|| visible_card_reward_item(session).map(|(_, cards)| cards));
    let Some(cards) = cards else {
        return Ok(None);
    };
    let decision = card_reward_decision(session, &cards);
    let crate::ai::card_reward_policy_v1::CardRewardPolicyActionV1::Stop { reason, .. } =
        &decision.action
    else {
        return Ok(None);
    };
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let detail = card_reward_policy_stop_detail(&decision, reason, &noncombat_record);
    Ok(Some((
        card_reward_policy_trace_annotation(&decision, noncombat_record)?,
        detail,
    )))
}

fn apply_policy_to_pending_cards(
    session: &mut RunControlSession,
    cards: Vec<RewardCard>,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let decision = card_reward_decision(session, &cards);
    let noncombat_record = decision.to_noncombat_decision_record_v1();
    let trace_annotation = card_reward_policy_trace_annotation(&decision, noncombat_record)?;
    let crate::ai::card_reward_policy_v1::CardRewardPolicyActionV1::Pick {
        index,
        card,
        confidence,
        reason,
    } = decision.action
    else {
        return Ok(None);
    };
    let outcome = session
        .apply_input_without_manual_card_reward_trace(ClientInput::SelectCard(index))?
        .with_trace_annotations(vec![trace_annotation]);
    Ok(Some((
        outcome,
        card_reward_summary(card, confidence, &reason, decision.label_role),
    )))
}

fn apply_recorded_pick_to_pending_cards(
    session: &mut RunControlSession,
    cards: Vec<RewardCard>,
    index: usize,
) -> Result<RunControlCommandOutcome, String> {
    let decision = recorded_card_reward_decision(session, &cards, index)?;
    let record = selected_card_reward_record(
        &decision,
        "run_control_recorded_card_reward_pick_v1",
        "human_recorded_pick",
    );
    let trace_annotation = card_reward_policy_trace_annotation(&decision, record)?;
    Ok(session
        .apply_input_without_manual_card_reward_trace(ClientInput::SelectCard(index))?
        .with_trace_annotations(vec![trace_annotation]))
}

pub(super) fn manual_card_reward_selection_annotation(
    session: &RunControlSession,
    index: usize,
) -> Result<Option<RunControlTraceAnnotationV1>, String> {
    let Some(cards) = active_pending_reward_cards(session) else {
        return Ok(None);
    };
    let decision = recorded_card_reward_decision(session, &cards, index)?;
    let record = selected_card_reward_record(
        &decision,
        "run_control_manual_card_reward_pick_v1",
        "human_visible_card_reward_pick",
    );
    card_reward_policy_trace_annotation(&decision, record).map(Some)
}

fn card_reward_decision(
    session: &RunControlSession,
    cards: &[RewardCard],
) -> crate::ai::card_reward_policy_v1::CardRewardDecisionV1 {
    let route_trace = crate::ai::route_planner_v1::plan_route_decision_v1(
        &session.run_state,
        &session.engine_state,
        Default::default(),
    );
    let route_trace = (!route_trace.candidates.is_empty()).then_some(route_trace);
    let context = crate::ai::card_reward_policy_v1::build_card_reward_decision_context_v1(
        &session.run_state,
        cards.to_vec(),
        route_trace.as_ref(),
    );
    let inputs = card_reward_estimator_inputs(session, &context);
    crate::ai::card_reward_policy_v1::plan_card_reward_decision_with_estimator_inputs_v1(
        &context,
        &crate::ai::card_reward_policy_v1::CardRewardPolicyConfigV1::default(),
        &inputs,
    )
}

fn card_reward_estimator_inputs(
    session: &RunControlSession,
    context: &crate::ai::card_reward_policy_v1::CardRewardDecisionContextV1,
) -> crate::ai::card_reward_policy_v1::CardRewardEstimatorInputsV1 {
    let mut external_value_estimates = session
        .card_reward_outcome_calibration
        .as_ref()
        .map(|calibration| {
            crate::eval::card_reward_value_loop::estimate_card_reward_values_from_calibration_v1(
                context,
                calibration,
            )
        })
        .unwrap_or_default();
    if let Some(calibration) = session.card_reward_route_risk_calibration.as_ref() {
        external_value_estimates.extend(
            crate::eval::card_reward_value_loop::estimate_card_reward_values_from_route_risk_calibration_v1(
                context,
                calibration,
            ),
        );
    }
    crate::ai::card_reward_policy_v1::CardRewardEstimatorInputsV1 {
        external_value_estimates,
    }
}

fn card_reward_policy_stop_detail(
    decision: &crate::ai::card_reward_policy_v1::CardRewardDecisionV1,
    reason: &str,
    record: &crate::ai::noncombat_decision_v1::NonCombatDecisionRecordV1,
) -> String {
    let mut details = Vec::new();
    let value_tags = record
        .values
        .iter()
        .flat_map(|value| value.components.iter())
        .filter_map(|component| {
            (component.name.starts_with("value_source_")
                || component.name.starts_with("value_status_"))
            .then_some(component.name.clone())
        })
        .collect::<BTreeSet<_>>();
    if !value_tags.is_empty() {
        details.push(format!(
            "value inputs: {}",
            value_tags.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    let gate_estimate_counts = gate_estimate_source_counts(decision);
    if !gate_estimate_counts.is_empty() {
        details.push(format!(
            "gate estimates: {}",
            gate_estimate_counts
                .into_iter()
                .map(|(source, count)| format!("{source}={count}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    let uncalibrated_candidates = decision
        .value_arbitration
        .candidate_reports
        .iter()
        .filter(|report| {
            report.selected_source
                == Some(
                    crate::ai::card_reward_policy_v1::CardRewardValueSourceV1::UncalibratedImpactPrior,
                )
        })
        .map(|report| {
            decision
                .candidates
                .iter()
                .find(|candidate| candidate.index == report.index && candidate.card == report.card)
                .map(|candidate| candidate.name.clone())
                .unwrap_or_else(|| format!("{:?}", report.card))
        })
        .collect::<Vec<_>>();
    if !uncalibrated_candidates.is_empty() {
        details.push(format!(
            "uncalibrated gate candidates: {}",
            uncalibrated_candidates.join(", ")
        ));
    }

    let non_gate_candidates = decision
        .value_arbitration
        .candidate_reports
        .iter()
        .filter(|report| {
            report.selected_source.is_some() && !report.selected_estimate_gate_eligible
        })
        .map(|report| {
            let name = decision
                .candidates
                .iter()
                .find(|candidate| candidate.index == report.index && candidate.card == report.card)
                .map(|candidate| candidate.name.clone())
                .unwrap_or_else(|| format!("{:?}", report.card));
            let source = report
                .selected_source
                .map(|source| format!("{source:?}"))
                .unwrap_or_else(|| "MissingValueEstimate".to_string());
            format!("{name} ({source})")
        })
        .collect::<Vec<_>>();
    if !non_gate_candidates.is_empty() {
        details.push(format!(
            "non-gate value candidates: {}",
            non_gate_candidates.join(", ")
        ));
    }

    let base = format!("card reward policy stopped: {reason}");
    if details.is_empty() {
        base
    } else {
        format!("{base}; {}", details.join("; "))
    }
}

fn gate_estimate_source_counts(
    decision: &crate::ai::card_reward_policy_v1::CardRewardDecisionV1,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::<String, usize>::new();
    for estimate in &decision.value_arbitration.gate_value_estimates {
        *counts.entry(format!("{:?}", estimate.source)).or_default() += 1;
    }
    counts
}

fn recorded_card_reward_decision(
    session: &RunControlSession,
    cards: &[RewardCard],
    index: usize,
) -> Result<crate::ai::card_reward_policy_v1::CardRewardDecisionV1, String> {
    if index >= cards.len() {
        return Err(format!(
            "card reward index {index} is out of range; visible choices are 0..{}",
            cards.len().saturating_sub(1)
        ));
    }
    let mut decision = card_reward_decision(session, cards);
    let Some(candidate) = decision.candidates.get(index) else {
        return Err(format!(
            "card reward index {index} is out of range for policy candidates"
        ));
    };
    decision.action = crate::ai::card_reward_policy_v1::CardRewardPolicyActionV1::Pick {
        index,
        card: candidate.card,
        confidence: 0.25,
        reason: "human recorded card reward pick; diagnostic behavior data, not a teacher label"
            .to_string(),
    };
    decision.pick_certificate = None;
    decision.label_role = "behavior_policy_not_teacher";
    Ok(decision)
}

fn selected_card_reward_record(
    decision: &crate::ai::card_reward_policy_v1::CardRewardDecisionV1,
    source_policy: &'static str,
    selection_mode: &'static str,
) -> crate::ai::noncombat_decision_v1::NonCombatDecisionRecordV1 {
    let mut record = decision.to_noncombat_decision_record_v1();
    record.provenance.source_policy = source_policy.to_string();
    record.selection.selection_mode = selection_mode.to_string();
    record
}

fn card_reward_policy_trace_annotation(
    decision: &crate::ai::card_reward_policy_v1::CardRewardDecisionV1,
    record: crate::ai::noncombat_decision_v1::NonCombatDecisionRecordV1,
) -> Result<RunControlTraceAnnotationV1, String> {
    super::noncombat_policy_annotation::validate_noncombat_policy_record(
        "card reward policy",
        &record,
    )?;
    Ok(RunControlTraceAnnotationV1::NonCombatPolicyDecision {
        record,
        card_reward_packet: Some(
            crate::ai::card_reward_policy_v1::PublicRewardDecisionPacketV1::from_context(
                &decision.context,
            ),
        ),
    })
}

fn card_reward_summary(
    card: crate::content::cards::CardId,
    confidence: f32,
    reason: &str,
    label_role: &'static str,
) -> String {
    let name = crate::content::cards::get_card_definition(card).name;
    format!(
        "card reward policy: {name} confidence={confidence:.2} reason={reason} label_role={label_role}",
    )
}

fn active_pending_reward_cards(session: &RunControlSession) -> Option<Vec<RewardCard>> {
    let cards = match &session.engine_state {
        EngineState::RewardScreen(reward) => reward.pending_card_choice.as_ref()?,
        EngineState::RewardOverlay { reward_state, .. } => {
            reward_state.pending_card_choice.as_ref()?
        }
        _ => return None,
    };
    Some(cards.clone())
}

fn visible_card_reward_item(session: &RunControlSession) -> Option<(usize, Vec<RewardCard>)> {
    let reward = match &session.engine_state {
        EngineState::RewardScreen(reward) => reward,
        EngineState::RewardOverlay { reward_state, .. } => reward_state,
        _ => return None,
    };
    if reward.pending_card_choice.is_some() {
        return None;
    }
    reward
        .items
        .iter()
        .enumerate()
        .find_map(|(idx, item)| match item {
            RewardItem::Card { cards } => Some((idx, cards.clone())),
            _ => None,
        })
}
