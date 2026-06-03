use super::commands::{RunControlAutoStepOptions, RunControlRouteAutomationMode};
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::view_model::build_run_control_view_model;
use crate::state::core::EngineState;

const DEFAULT_AUTO_RUN_MAX_OPERATIONS: usize = 128;

pub(in crate::eval::run_control) fn apply_auto_run(
    session: &mut RunControlSession,
    mut options: RunControlAutoStepOptions,
) -> Result<RunControlCommandOutcome, String> {
    options.route = RunControlRouteAutomationMode::Planner;
    let max_operations = options
        .max_operations
        .unwrap_or(DEFAULT_AUTO_RUN_MAX_OPERATIONS);
    options.max_operations = Some(max_operations);

    let mut outcome = super::auto_step::apply_guarded_auto_step(session, options)?;
    let title = build_run_control_view_model(session).header.title;
    let reason = extract_reason(&outcome.message);
    let applied_operations = count_applied_operations(&outcome.message);
    let next_hint = auto_run_next_hint(session);
    outcome.message = format!(
        "Auto-run stopped: {title}\nroute=planner max_operations={max_operations} applied_operations={applied_operations}\n{reason}\n{next_hint}\n{}",
        outcome.message
    );
    Ok(outcome)
}

fn extract_reason(message: &str) -> String {
    message
        .lines()
        .find(|line| line.starts_with("Reason: "))
        .unwrap_or("Reason: unknown")
        .to_string()
}

fn count_applied_operations(message: &str) -> usize {
    let mut in_applied = false;
    let mut count = 0usize;
    for line in message.lines() {
        if line == "Applied:" {
            in_applied = true;
            continue;
        }
        if line.starts_with("Reason: ") {
            break;
        }
        if in_applied && line.starts_with("  - ") {
            count = count.saturating_add(1);
        }
    }
    count
}

fn auto_run_next_hint(session: &RunControlSession) -> &'static str {
    match &session.engine_state {
        EngineState::EventRoom => {
            let is_neow_bonus = session.run_state.event_state.as_ref().is_some_and(|event| {
                event.id == crate::state::events::EventId::Neow && event.current_screen > 0
            });
            if is_neow_bonus {
                "Next: choose a Neow bonus id, or inspect deck/map/relics first."
            } else {
                "Next: choose a visible event option id; use inspect/details/raw if the semantics look wrong."
            }
        }
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => {
            "Next: use rs to inspect route evidence, rg to accept the route planner, or type a visible path id."
        }
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            "Next: choose a card id or skip; use deck/map/relics before choosing if needed."
        }
        EngineState::RewardOverlay { reward_state, .. } if reward_state.pending_card_choice.is_some() => {
            "Next: choose a card id or skip; use deck/map/relics before choosing if needed."
        }
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => {
            "Next: choose a visible reward id, or skip to preview the map while unclaimed rewards remain."
        }
        EngineState::CombatPlayerTurn | EngineState::CombatProcessing | EngineState::PendingChoice(_) => {
            "Next: play manually, cap the combat if useful, or try sc max_nodes=N wall_ms=N."
        }
        EngineState::BossRelicSelect(_) => {
            "Next: choose a visible boss relic id; inspect deck/relics first if needed."
        }
        EngineState::Shop(_) => {
            "Next: buy card/relic/potion, purge a card, or leave the shop."
        }
        EngineState::Campfire => {
            "Next: rest, smith a deck index, or use another visible campfire option."
        }
        EngineState::TreasureRoom(_) => "Next: open the chest.",
        EngineState::RunPendingChoice(_) => "Next: choose a visible run-choice id.",
        EngineState::CombatStart(_) => "Next: advance once; combat setup should settle into a player turn.",
        EngineState::GameOver(_) => "Next: q to exit, or start a new run from the shell.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_applied_operations_ignores_none() {
        assert_eq!(
            count_applied_operations("Applied:\n  none\nReason: map route requires human choice"),
            0
        );
    }

    #[test]
    fn count_applied_operations_counts_bullets_before_reason() {
        assert_eq!(
            count_applied_operations(
                "Applied:\n  - route planner\n  - combat search\nReason: done\n  - detail"
            ),
            2
        );
    }
}
