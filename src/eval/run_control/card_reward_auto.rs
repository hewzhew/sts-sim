use crate::ai::noncombat_decision_v1::{
    render_noncombat_decision_record_validation_errors, validate_noncombat_decision_record_v1,
    NonCombatDecisionRecordV1,
};
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
        .apply_input(ClientInput::SelectCard(index))?
        .with_trace_annotations(vec![noncombat_policy_annotation(noncombat_record)?]);
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
    Ok(Some((
        noncombat_policy_annotation(noncombat_record)?,
        format!("card reward policy stopped: {reason}"),
    )))
}

fn apply_policy_to_pending_cards(
    session: &mut RunControlSession,
    cards: Vec<RewardCard>,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let decision = card_reward_decision(session, &cards);
    let noncombat_record = decision.to_noncombat_decision_record_v1();
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
        .apply_input(ClientInput::SelectCard(index))?
        .with_trace_annotations(vec![noncombat_policy_annotation(noncombat_record)?]);
    Ok(Some((
        outcome,
        card_reward_summary(card, confidence, &reason, decision.label_role),
    )))
}

fn noncombat_policy_annotation(
    record: NonCombatDecisionRecordV1,
) -> Result<RunControlTraceAnnotationV1, String> {
    validate_noncombat_decision_record_v1(&record).map_err(|errors| {
        format!(
            "card reward policy produced invalid NonCombatDecisionRecordV1: {}",
            render_noncombat_decision_record_validation_errors(&errors)
        )
    })?;
    Ok(RunControlTraceAnnotationV1::NonCombatPolicyDecision { record })
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
    crate::ai::card_reward_policy_v1::plan_card_reward_decision_v1(
        &context,
        &crate::ai::card_reward_policy_v1::CardRewardPolicyConfigV1::default(),
    )
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
