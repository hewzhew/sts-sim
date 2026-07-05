use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlCommand, RunControlCommandOutcome, RunControlSession,
};
use sts_simulator::state::core::{ClientInput, EngineState};

use super::owner_model::OwnerRoutine;
use super::render;

pub(super) fn apply_owner_routine(
    session: &mut RunControlSession,
    routine: OwnerRoutine,
) -> Result<RunControlCommandOutcome, String> {
    match routine {
        OwnerRoutine::Command(command) => session.apply_command(command),
        OwnerRoutine::RewardTinyAutomation => apply_reward_tiny_routine(session),
        OwnerRoutine::AdvanceEmptyCampfire => {
            sts_simulator::engine::run_loop::tick_run_active_with_observer(
                &mut session.engine_state,
                &mut session.run_state,
                &mut session.active_combat,
                None,
            );
            session.apply_command(RunControlCommand::Noop)
        }
    }
}

fn apply_reward_tiny_routine(
    session: &mut RunControlSession,
) -> Result<RunControlCommandOutcome, String> {
    if let Some(outcome) = sts_simulator::eval::run_control::apply_reward_tiny_automation(session)?
    {
        return Ok(outcome);
    }
    session.apply_command(RunControlCommand::Input(reward_tiny_exit_input(session)?))
}

fn reward_tiny_exit_input(session: &RunControlSession) -> Result<ClientInput, String> {
    let (reward, exit) = match &session.engine_state {
        EngineState::RewardScreen(reward) => (reward, ClientInput::Proceed),
        EngineState::RewardOverlay { reward_state, .. } => (reward_state, ClientInput::Cancel),
        _ => return Err("RewardTiny owner requires reward surface".to_string()),
    };
    if reward.pending_card_choice.is_some() || reward.has_card_reward_item() {
        return Err("RewardTiny owner received card reward surface".to_string());
    }
    let only_unclaimable_potions = !reward.items.is_empty()
        && reward.items.iter().all(|item| {
            matches!(
                item,
                sts_simulator::state::rewards::RewardItem::Potion { .. }
            )
        })
        && session.run_state.find_empty_potion_slot().is_none();
    if reward.items.is_empty() || only_unclaimable_potions {
        return require_visible_input(session, exit);
    }
    Err(format!(
        "RewardTiny owner has strategic residual reward items: {:?}",
        reward.items
    ))
}

fn require_visible_input(
    session: &RunControlSession,
    input: ClientInput,
) -> Result<ClientInput, String> {
    let surface = build_decision_surface(session);
    if surface
        .visible_executable_inputs
        .iter()
        .any(|visible_input| visible_input == &input)
    {
        return Ok(input);
    }
    Err(format!(
        "input {:?} is not visible at {} among [{}]",
        input,
        surface.view.header.title,
        super::owner_commands::executable_choices_including_cancel(&surface)
            .iter()
            .map(render::render_timeline_choice)
            .collect::<Vec<_>>()
            .join(" | ")
    ))
}
