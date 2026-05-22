use crate::state::core::{ClientInput, EngineState, RunResult};

use super::commands::{RunControlAutoStepOptions, RunControlSearchCombatOptions};
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::transition_report::{
    action_result_from_transition, render_action_result, RunApplyStatus, RunVisibleSnapshot,
    TransitionAction,
};
use super::view_model::{build_run_control_view_model, DecisionCandidate, RunControlViewModel};

const DEFAULT_MAX_OPERATIONS: usize = 16;
const DEFAULT_AUTO_SEARCH_WALL_MS: u64 = 5_000;

pub(super) fn apply_guarded_auto_step(
    session: &mut RunControlSession,
    options: RunControlAutoStepOptions,
) -> Result<RunControlCommandOutcome, String> {
    let before = RunVisibleSnapshot::capture(session);
    let mut applied = Vec::new();
    let max_operations = options.max_operations.unwrap_or(DEFAULT_MAX_OPERATIONS);

    for _ in 0..max_operations {
        let reward_report = super::reward_auto::apply_reward_automation(session)?;
        if !reward_report.is_empty() {
            applied.push(format!(
                "auto reward: {}",
                reward_report
                    .claims
                    .iter()
                    .map(|claim| claim.label.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            continue;
        }

        if session.current_active_combat_position().is_ok() {
            let outcome = super::combat_search::apply_search_combat(
                session,
                auto_search_options(options.search.clone()),
            )?;
            if let Some(result) = outcome.action_result.as_ref() {
                applied.push(format!("combat search: {}", result.chosen_label));
                continue;
            }
            return finish_auto_step(
                session,
                &before,
                applied,
                "combat search did not find an executable complete win",
                Some(trim_search_rejection(&outcome.message)),
            );
        }

        let view = build_run_control_view_model(session);
        if let Some(candidate) = routine_candidate(session, &view) {
            let Some(input) = candidate.action.executable_input() else {
                return finish_auto_step(
                    session,
                    &before,
                    applied,
                    "routine candidate is not executable",
                    None,
                );
            };
            let outcome = session.apply_input(input)?;
            let label = outcome
                .action_result
                .as_ref()
                .map(|result| result.chosen_label.clone())
                .unwrap_or_else(|| candidate.label.clone());
            applied.push(format!("routine: {label}"));
            continue;
        }

        return finish_auto_step(session, &before, applied, human_stop_reason(session), None);
    }

    finish_auto_step(
        session,
        &before,
        applied,
        format!("operation budget exhausted at {max_operations} guarded operations"),
        None,
    )
}

fn auto_search_options(
    mut options: RunControlSearchCombatOptions,
) -> RunControlSearchCombatOptions {
    if options.wall_ms.is_none() {
        options.wall_ms = Some(DEFAULT_AUTO_SEARCH_WALL_MS);
    }
    options
}

fn routine_candidate<'a>(
    session: &RunControlSession,
    view: &'a RunControlViewModel,
) -> Option<&'a DecisionCandidate> {
    if let EngineState::RewardScreen(reward) = &session.engine_state {
        if reward.pending_card_choice.is_none() && reward.items.is_empty() && reward.skippable {
            return view.candidates.iter().find(|candidate| {
                candidate.action.executable_input() == Some(ClientInput::Proceed)
            });
        }
    }

    if view.candidates.len() == 1
        && view.candidates[0].note.as_deref() == Some("routine")
        && view.candidates[0].action.executable_input().is_some()
    {
        return Some(&view.candidates[0]);
    }

    None
}

fn human_stop_reason(session: &RunControlSession) -> String {
    match &session.engine_state {
        EngineState::EventRoom => {
            let is_neow_bonus = session.run_state.event_state.as_ref().is_some_and(|event| {
                event.id == crate::state::events::EventId::Neow && event.current_screen > 0
            });
            if is_neow_bonus {
                "Neow bonus requires human choice".to_string()
            } else {
                "event option requires human choice".to_string()
            }
        }
        EngineState::MapNavigation => "map route requires human choice".to_string(),
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardScreen(reward) if reward_has_card_item(reward) => {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardScreen(reward) if reward_has_relic_item(reward) => {
            "relic reward requires human choice".to_string()
        }
        EngineState::RewardScreen(reward) if !reward.items.is_empty() => {
            "remaining reward requires human choice".to_string()
        }
        EngineState::RewardScreen(_) => {
            "reward screen cannot be advanced automatically".to_string()
        }
        EngineState::TreasureRoom(_) => {
            "treasure room is not at an executable routine boundary".to_string()
        }
        EngineState::Campfire => "campfire action requires human choice".to_string(),
        EngineState::Shop(_) => "shop action requires human choice".to_string(),
        EngineState::RunPendingChoice(_) => "card selection requires human choice".to_string(),
        EngineState::BossRelicSelect(_) => "boss relic choice requires human choice".to_string(),
        EngineState::CombatStart(_) => {
            "combat start is not yet a stable player boundary".to_string()
        }
        EngineState::CombatProcessing => "combat is still processing".to_string(),
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => {
            "combat boundary requires search or human action".to_string()
        }
        EngineState::GameOver(_) => "run is over".to_string(),
    }
}

fn reward_has_card_item(reward: &crate::state::rewards::RewardState) -> bool {
    reward
        .items
        .iter()
        .any(|item| matches!(item, crate::state::rewards::RewardItem::Card { .. }))
}

fn reward_has_relic_item(reward: &crate::state::rewards::RewardState) -> bool {
    reward
        .items
        .iter()
        .any(|item| matches!(item, crate::state::rewards::RewardItem::Relic { .. }))
}

fn finish_auto_step(
    session: &RunControlSession,
    before: &RunVisibleSnapshot,
    applied: Vec<String>,
    reason: impl Into<String>,
    detail: Option<String>,
) -> Result<RunControlCommandOutcome, String> {
    let reason = reason.into();
    let view = build_run_control_view_model(session);
    let mut lines = vec![
        format!("Guarded auto step stopped: {}", view.header.title),
        "Applied:".to_string(),
    ];
    if applied.is_empty() {
        lines.push("  none".to_string());
    } else {
        for item in &applied {
            lines.push(format!("  - {item}"));
        }
    }
    lines.push(format!("Reason: {reason}"));
    if let Some(detail) = detail.filter(|detail| !detail.trim().is_empty()) {
        lines.push("Detail:".to_string());
        lines.extend(detail.lines().map(|line| format!("  {line}")));
    }

    if applied.is_empty() {
        lines.push(super::render::render_run_control_state(session));
        return Ok(RunControlCommandOutcome::message(lines.join("\n")));
    }

    let after = RunVisibleSnapshot::capture(session);
    let action_result = action_result_from_transition(
        TransitionAction {
            label: format!("guarded auto-step applied {} operation(s)", applied.len()),
        },
        before,
        &after,
        current_run_apply_status(session),
    );
    lines.push(render_action_result(&action_result));
    lines.push(super::render::render_run_control_state(session));
    Ok(RunControlCommandOutcome::action(
        lines.join("\n"),
        action_result,
    ))
}

fn current_run_apply_status(session: &RunControlSession) -> RunApplyStatus {
    match session.engine_state {
        EngineState::GameOver(RunResult::Victory) => RunApplyStatus::Victory,
        EngineState::GameOver(RunResult::Defeat) => RunApplyStatus::Defeat,
        _ => RunApplyStatus::Running,
    }
}

fn trim_search_rejection(message: &str) -> String {
    message
        .lines()
        .take_while(|line| !line.starts_with("===="))
        .take(8)
        .collect::<Vec<_>>()
        .join("\n")
}
