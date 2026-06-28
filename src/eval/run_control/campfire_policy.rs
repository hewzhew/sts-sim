use crate::ai::strategy::campfire_upgrade_quality::{
    rank_campfire_upgrades, should_rest_before_smith, CampfireUpgradeTier,
};
use crate::content::cards::get_card_definition;
use crate::state::core::{CampfireChoice, ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_campfire_policy_action(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return Ok(None);
    }
    let choices = crate::engine::campfire_handler::get_available_options(&session.run_state);
    let rest_available = choices.contains(&CampfireChoice::Rest);
    let smith_available = choices
        .iter()
        .any(|choice| matches!(choice, CampfireChoice::Smith(_)));
    if rest_available
        && (!smith_available
            || should_rest_before_smith(session.run_state.current_hp, session.run_state.max_hp))
    {
        let outcome = session.apply_input(ClientInput::CampfireOption(CampfireChoice::Rest))?;
        return Ok(Some((
            outcome,
            format!(
                "campfire policy: rest hp={}/{}",
                session.run_state.current_hp, session.run_state.max_hp
            ),
        )));
    }
    if !smith_available {
        return Ok(None);
    }

    let ranked = rank_campfire_upgrades(&session.run_state.master_deck);
    let Some(best) = ranked
        .iter()
        .find(|target| target.tier >= CampfireUpgradeTier::Low)
    else {
        return Ok(None);
    };
    let top = ranked
        .iter()
        .take(3)
        .map(|target| target.compact_label())
        .collect::<Vec<_>>()
        .join(" > ");

    let outcome = session.apply_input(ClientInput::CampfireOption(CampfireChoice::Smith(
        best.deck_index,
    )))?;
    let card_name = get_card_definition(best.card).name;
    Ok(Some((
        outcome,
        format!(
            "campfire policy: smith {card_name} tier={:?} hints={} cautions={} hp={}/{} top={top}",
            best.tier,
            best.hints_label(),
            best.cautions_label(),
            session.run_state.current_hp,
            session.run_state.max_hp,
        ),
    )))
}
