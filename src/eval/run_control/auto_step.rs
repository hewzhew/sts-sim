use std::collections::BTreeSet;

use crate::state::core::{ClientInput, EngineState, RunResult};

use super::commands::{
    RunControlAutoStepOptions, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
};
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::RunControlTraceAnnotationV1;
use super::transition_report::{
    action_result_from_transition, render_action_result, RunApplyStatus, RunVisibleSnapshot,
    TransitionAction,
};
use super::view_model::{build_run_control_view_model, DecisionCandidate, RunControlViewModel};

const DEFAULT_MAX_OPERATIONS: usize = 16;
const DEFAULT_AUTO_SEARCH_WALL_MS: u64 = 5_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AutoAdvanceClass {
    Routine,
    Forced,
    Strategic,
    Unsafe,
}

struct AutoAdvanceCandidate<'a> {
    candidate: &'a DecisionCandidate,
    class: AutoAdvanceClass,
    reason: &'static str,
}

pub(super) fn apply_guarded_auto_step(
    session: &mut RunControlSession,
    options: RunControlAutoStepOptions,
) -> Result<RunControlCommandOutcome, String> {
    let before = RunVisibleSnapshot::capture(session);
    let mut applied = Vec::new();
    let mut trace_annotations = Vec::new();
    let mut seen_boundaries = BTreeSet::new();
    let max_operations = options.max_operations.unwrap_or(DEFAULT_MAX_OPERATIONS);

    for _ in 0..max_operations {
        let boundary_key = auto_boundary_key(session);
        if !seen_boundaries.insert(boundary_key.clone()) {
            return finish_auto_step(
                session,
                &before,
                applied,
                trace_annotations,
                "repeated boundary detected while advancing automatically",
                Some(format!(
                    "boundary={boundary_key}\nThis usually means an event or transition kept presenting the same screen after an automatic action."
                )),
            );
        }

        let reward_report = super::reward_auto::apply_reward_automation(session)?;
        if !reward_report.is_empty() {
            applied.push(format!(
                "routine reward: {}",
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
                auto_search_options(session, options.search.clone()),
            )?;
            if let Some(result) = outcome.action_result.as_ref() {
                applied.push(format!("combat search: {}", result.chosen_label));
                let auto_capture_summaries = auto_capture_summaries(&outcome.trace_annotations);
                trace_annotations.extend(outcome.trace_annotations);
                applied.extend(auto_capture_summaries);
                continue;
            }
            return finish_auto_step(
                session,
                &before,
                applied,
                trace_annotations,
                "combat search did not find an executable complete win",
                Some(trim_search_rejection(&outcome.message)),
            );
        }

        if session.engine_state.is_map_surface()
            && options.route == RunControlRouteAutomationMode::Planner
        {
            match super::route_policy::apply_route_go_with_summary(session) {
                Ok(applied_route) => {
                    if applied_route.outcome.action_result.is_some() {
                        let auto_capture_summaries =
                            auto_capture_summaries(&applied_route.outcome.trace_annotations);
                        trace_annotations.extend(applied_route.outcome.trace_annotations);
                        applied.push(applied_route.auto_step_summary);
                        applied.extend(auto_capture_summaries);
                        continue;
                    }
                    trace_annotations.extend(applied_route.outcome.trace_annotations);
                    return finish_auto_step(
                        session,
                        &before,
                        applied,
                        trace_annotations,
                        "route planner did not modify state",
                        Some(applied_route.outcome.message),
                    );
                }
                Err(err) => {
                    return finish_auto_step(
                        session,
                        &before,
                        applied,
                        trace_annotations,
                        "route planner declined automatic map selection",
                        Some(err),
                    );
                }
            }
        }

        let view = build_run_control_view_model(session);
        if let Some(auto_candidate) = auto_advance_candidate(session, &view) {
            let Some(input) = auto_candidate.candidate.action.executable_input() else {
                return finish_auto_step(
                    session,
                    &before,
                    applied,
                    trace_annotations,
                    "auto-selected candidate is not executable",
                    None,
                );
            };
            let outcome = session.apply_input(input)?;
            let label = outcome
                .action_result
                .as_ref()
                .map(|result| result.chosen_label.clone())
                .unwrap_or_else(|| auto_candidate.candidate.label.clone());
            let auto_capture_summaries = auto_capture_summaries(&outcome.trace_annotations);
            trace_annotations.extend(outcome.trace_annotations);
            applied.push(format!(
                "{}: {label} ({})",
                auto_class_label(auto_candidate.class),
                auto_candidate.reason
            ));
            applied.extend(auto_capture_summaries);
            continue;
        }

        return finish_auto_step(
            session,
            &before,
            applied,
            trace_annotations,
            human_stop_reason(session),
            None,
        );
    }

    finish_auto_step(
        session,
        &before,
        applied,
        trace_annotations,
        format!("operation budget exhausted at {max_operations} automatic operations"),
        None,
    )
}

fn auto_capture_summaries(annotations: &[RunControlTraceAnnotationV1]) -> Vec<String> {
    annotations
        .iter()
        .filter_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::AutoCombatCapture {
                case_id,
                capture_path,
                ..
            } => Some(format!("auto capture: {case_id} -> {capture_path}")),
            RunControlTraceAnnotationV1::RoutePlannerSelection { .. } => None,
        })
        .collect()
}

fn auto_search_options(
    session: &RunControlSession,
    mut options: RunControlSearchCombatOptions,
) -> RunControlSearchCombatOptions {
    if options.wall_ms.is_none() && session.search_wall_ms.is_none() {
        options.wall_ms = Some(DEFAULT_AUTO_SEARCH_WALL_MS);
    }
    options
}

fn auto_advance_candidate<'a>(
    session: &RunControlSession,
    view: &'a RunControlViewModel,
) -> Option<AutoAdvanceCandidate<'a>> {
    if let EngineState::RewardScreen(reward) = &session.engine_state {
        if reward.pending_card_choice.is_none() && reward.items.is_empty() && reward.skippable {
            return view
                .candidates
                .iter()
                .find(|candidate| candidate.action.executable_input() == Some(ClientInput::Proceed))
                .map(|candidate| AutoAdvanceCandidate {
                    candidate,
                    class: AutoAdvanceClass::Routine,
                    reason: "empty reward screen",
                });
        }
    }
    if let EngineState::RewardOverlay { reward_state, .. } = &session.engine_state {
        if reward_state.pending_card_choice.is_none()
            && reward_state.items.is_empty()
            && reward_state.skippable
        {
            return view
                .candidates
                .iter()
                .find(|candidate| candidate.action.executable_input() == Some(ClientInput::Cancel))
                .map(|candidate| AutoAdvanceCandidate {
                    candidate,
                    class: AutoAdvanceClass::Routine,
                    reason: "empty overlay reward screen",
                });
        }
    }

    if view.candidates.len() == 1
        && view.candidates[0].note.as_deref() == Some("routine")
        && view.candidates[0].action.executable_input().is_some()
    {
        return Some(AutoAdvanceCandidate {
            candidate: &view.candidates[0],
            class: AutoAdvanceClass::Routine,
            reason: "single routine action",
        });
    }

    let executable = view
        .candidates
        .iter()
        .filter(|candidate| candidate.action.executable_input().is_some())
        .collect::<Vec<_>>();
    if executable.len() == 1 {
        let candidate = executable[0];
        let class = classify_single_executable_candidate(session, candidate);
        if matches!(class, AutoAdvanceClass::Routine | AutoAdvanceClass::Forced) {
            return Some(AutoAdvanceCandidate {
                candidate,
                class,
                reason: single_candidate_reason(session, candidate, class),
            });
        }
    }

    None
}

fn classify_single_executable_candidate(
    session: &RunControlSession,
    candidate: &DecisionCandidate,
) -> AutoAdvanceClass {
    if candidate.action.executable_input().is_none() {
        return AutoAdvanceClass::Unsafe;
    }
    match &session.engine_state {
        EngineState::TreasureRoom(_)
            if candidate.action.executable_input() == Some(ClientInput::OpenChest) =>
        {
            AutoAdvanceClass::Routine
        }
        EngineState::Shop(_) if candidate.id == "leave" => AutoAdvanceClass::Routine,
        EngineState::RewardScreen(reward)
            if reward.pending_card_choice.is_none()
                && reward.items.is_empty()
                && candidate.action.executable_input() == Some(ClientInput::Proceed) =>
        {
            AutoAdvanceClass::Routine
        }
        EngineState::RewardOverlay { reward_state, .. }
            if reward_state.pending_card_choice.is_none()
                && reward_state.items.is_empty()
                && candidate.action.executable_input() == Some(ClientInput::Cancel) =>
        {
            AutoAdvanceClass::Routine
        }
        EngineState::EventRoom if event_single_candidate_is_safe(session, candidate) => {
            AutoAdvanceClass::Forced
        }
        EngineState::GameOver(_) => AutoAdvanceClass::Unsafe,
        _ => AutoAdvanceClass::Strategic,
    }
}

fn event_single_candidate_is_safe(
    session: &RunControlSession,
    candidate: &DecisionCandidate,
) -> bool {
    if session.run_state.event_state.as_ref().is_some_and(|event| {
        event.id == crate::state::events::EventId::Neow && event.current_screen > 0
    }) {
        return false;
    }
    let Some(resolution) = candidate.resolution.as_ref() else {
        return candidate.note.as_deref() == Some("routine");
    };
    resolution.known_effects.is_empty()
        && resolution.unresolved_effects.is_empty()
        && matches!(
            resolution.followup,
            Some(
                super::view_model::FollowupBoundary::EventScreenAdvance
                    | super::view_model::FollowupBoundary::EventComplete
            )
        )
}

fn single_candidate_reason(
    session: &RunControlSession,
    candidate: &DecisionCandidate,
    class: AutoAdvanceClass,
) -> &'static str {
    match (&session.engine_state, class, candidate.id.as_str()) {
        (EngineState::TreasureRoom(_), AutoAdvanceClass::Routine, _) => "single chest action",
        (EngineState::Shop(_), AutoAdvanceClass::Routine, "leave") => "only shop exit remains",
        (EngineState::EventRoom, AutoAdvanceClass::Forced, _) => "single safe event transition",
        _ => "single safe action",
    }
}

fn auto_class_label(class: AutoAdvanceClass) -> &'static str {
    match class {
        AutoAdvanceClass::Routine => "routine",
        AutoAdvanceClass::Forced => "forced",
        AutoAdvanceClass::Strategic => "strategic",
        AutoAdvanceClass::Unsafe => "unsafe",
    }
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
        EngineState::MapOverlay { .. } => "map preview requires route choice or cancel".to_string(),
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. }
            if reward_state.pending_card_choice.is_some() =>
        {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardScreen(reward) if reward_has_card_item(reward) => {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. } if reward_has_card_item(reward_state) => {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardScreen(reward) if reward_has_relic_item(reward) => {
            "relic reward requires human choice".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. } if reward_has_relic_item(reward_state) => {
            "relic reward requires human choice".to_string()
        }
        EngineState::RewardScreen(reward) if !reward.items.is_empty() => {
            "remaining reward requires human choice".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. } if !reward_state.items.is_empty() => {
            "remaining overlay reward requires human choice".to_string()
        }
        EngineState::RewardScreen(_) => {
            "reward screen cannot be advanced automatically".to_string()
        }
        EngineState::RewardOverlay { .. } => {
            "overlay reward screen cannot be advanced automatically".to_string()
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
    trace_annotations: Vec<RunControlTraceAnnotationV1>,
    reason: impl Into<String>,
    detail: Option<String>,
) -> Result<RunControlCommandOutcome, String> {
    let reason = reason.into();
    let view = build_run_control_view_model(session);
    let mut lines = vec![
        format!("Advanced to human boundary: {}", view.header.title),
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
        return Ok(RunControlCommandOutcome::message(lines.join("\n"))
            .with_trace_annotations(trace_annotations));
    }

    let after = RunVisibleSnapshot::capture(session);
    let action_result = action_result_from_transition(
        TransitionAction {
            label: format!(
                "advance-to-human-boundary applied {} operation(s)",
                applied.len()
            ),
        },
        before,
        &after,
        current_run_apply_status(session),
    );
    lines.push(render_action_result(&action_result));
    lines.push(super::render::render_run_control_state(session));
    Ok(
        RunControlCommandOutcome::action(lines.join("\n"), action_result)
            .with_trace_annotations(trace_annotations),
    )
}

fn auto_boundary_key(session: &RunControlSession) -> String {
    let view = build_run_control_view_model(session);
    let active_combat = session
        .active_combat
        .as_ref()
        .map(|active| {
            format!(
                "{:?}:turn{}:hp{}:hand{}",
                active.engine_state,
                active.combat_state.turn.turn_count,
                active.combat_state.entities.player.current_hp,
                active.combat_state.zones.hand.len()
            )
        })
        .unwrap_or_else(|| "no-combat".to_string());
    let event = session
        .run_state
        .event_state
        .as_ref()
        .map(|event| format!("{:?}:screen{}", event.id, event.current_screen))
        .unwrap_or_else(|| "no-event".to_string());
    let candidates = view
        .candidates
        .iter()
        .map(|candidate| format!("{}={}", candidate.id, candidate.action.command_hint()))
        .collect::<Vec<_>>()
        .join(",");
    let (player_hp, _) = session.visible_player_hp();
    format!(
        "{:?}|{}|{}|act{}|floor{}|hp{}|gold{}|{}|{}",
        session.engine_state,
        view.header.title,
        event,
        session.run_state.act_num,
        session.run_state.floor_num,
        player_hp,
        session.run_state.gold,
        active_combat,
        candidates
    )
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
        .take(12)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::auto_search_options;
    use crate::eval::run_control::{
        RunControlConfig, RunControlSearchCombatOptions, RunControlSession,
    };

    #[test]
    fn auto_search_wall_time_uses_session_default_before_auto_fallback() {
        let session = RunControlSession::new(RunControlConfig {
            search_wall_ms: Some(30_000),
            ..RunControlConfig::default()
        });

        let options = auto_search_options(&session, RunControlSearchCombatOptions::default());
        assert_eq!(options.wall_ms, None);

        let options = auto_search_options(
            &session,
            RunControlSearchCombatOptions {
                wall_ms: Some(500),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(options.wall_ms, Some(500));

        let session = RunControlSession::new(RunControlConfig::default());
        let options = auto_search_options(&session, RunControlSearchCombatOptions::default());
        assert_eq!(options.wall_ms, Some(5_000));
    }
}
