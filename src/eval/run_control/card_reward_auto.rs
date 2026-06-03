use crate::state::core::{ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_card_reward_policy_pick(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    if session
        .run_state
        .relics
        .iter()
        .any(|relic| relic.id == crate::content::relics::RelicId::SingingBowl)
    {
        return Ok(None);
    }
    let Some(cards) = active_pending_reward_cards(session) else {
        return Ok(None);
    };
    let decision = crate::ai::card_reward_policy_v1::plan_card_reward_decision_v1(
        &session.run_state,
        &cards,
        &crate::ai::card_reward_policy_v1::CardRewardPolicyConfigV1::default(),
    );
    let crate::ai::card_reward_policy_v1::CardRewardPolicyActionV1::Pick {
        index,
        card,
        confidence,
        reason,
    } = decision.action
    else {
        return Ok(None);
    };
    let name = crate::content::cards::get_card_definition(card).name;
    let outcome = session.apply_input(ClientInput::SelectCard(index))?;
    Ok(Some((
        outcome,
        format!(
            "card reward policy: {name} confidence={confidence:.2} reason={reason} label_role={}",
            decision.label_role
        ),
    )))
}

fn active_pending_reward_cards(
    session: &RunControlSession,
) -> Option<Vec<crate::state::rewards::RewardCard>> {
    let cards = match &session.engine_state {
        EngineState::RewardScreen(reward) => reward.pending_card_choice.as_ref()?,
        EngineState::RewardOverlay { reward_state, .. } => {
            reward_state.pending_card_choice.as_ref()?
        }
        _ => return None,
    };
    Some(cards.clone())
}
