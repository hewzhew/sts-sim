use crate::content::relics::RelicId;
use crate::state::core::{ClientInput, EngineState};
use crate::state::rewards::{RewardCard, RewardItem};

use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::RunControlTraceAnnotationV1;

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

pub(super) fn apply_singing_bowl_to_visible_card_reward_item(
    session: &mut RunControlSession,
    reward_index: usize,
) -> Result<RunControlCommandOutcome, String> {
    if !session
        .run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::SingingBowl)
    {
        return Err("Singing Bowl card reward requires Singing Bowl relic".to_string());
    }

    ensure_visible_card_reward_item_at(session, reward_index)?;

    session.apply_input(ClientInput::ClaimReward(reward_index))?;
    let Some(opened_cards) = active_pending_reward_cards(session) else {
        return Err(
            "Singing Bowl opened a reward item but no pending card choice appeared".to_string(),
        );
    };
    let bowl_index = opened_cards.len();
    session.apply_input_without_manual_card_reward_trace(ClientInput::SelectCard(bowl_index))
}

fn ensure_visible_card_reward_item_at(
    session: &RunControlSession,
    reward_index: usize,
) -> Result<(), String> {
    let reward = match &session.engine_state {
        EngineState::RewardScreen(reward) => reward,
        EngineState::RewardOverlay { reward_state, .. } => reward_state,
        _ => return Err("Singing Bowl card reward requires a reward screen".to_string()),
    };
    if reward.pending_card_choice.is_some() {
        return Err(
            "Singing Bowl visible card reward requires an unopened card reward item".to_string(),
        );
    }
    if !matches!(
        reward.items.get(reward_index),
        Some(RewardItem::Card { .. })
    ) {
        return Err(format!(
            "reward item {reward_index} is not a visible card reward item"
        ));
    }
    Ok(())
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
    let context =
        crate::ai::card_reward_policy_v1::build_card_reward_decision_context_with_current_route_v1(
            &session.run_state,
            &session.engine_state,
            cards.to_vec(),
        );
    let inputs = card_reward_estimator_inputs(session, &context);
    crate::ai::card_reward_policy_v1::plan_card_reward_decision_with_estimator_inputs_v1(
        &context,
        &crate::ai::card_reward_policy_v1::CardRewardPolicyConfigV1::behavior_autopick(),
        &inputs,
    )
}

fn card_reward_estimator_inputs(
    session: &RunControlSession,
    context: &crate::ai::card_reward_policy_v1::CardRewardDecisionContextV1,
) -> crate::ai::card_reward_policy_v1::CardRewardEstimatorInputsV1 {
    crate::eval::card_reward_value_loop::build_card_reward_runtime_estimator_inputs_v1(
        context,
        crate::eval::card_reward_value_loop::CardRewardRuntimeEstimatorCalibrationsV1 {
            outcome: session.card_reward_outcome_calibration.as_ref(),
            route_risk: session.card_reward_route_risk_calibration.as_ref(),
            strategy_package: session.card_reward_strategy_package_calibration.as_ref(),
        },
    )
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
    decision.decision_approval = None;
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
