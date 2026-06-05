use crate::state::core::{ClientInput, EngineState};

use super::session::RunControlSession;
use super::view_model::{build_run_control_view_model, DecisionCandidate, RunControlViewModel};

#[derive(Clone, Debug, PartialEq)]
pub struct DecisionSurface {
    pub view: RunControlViewModel,
    pub candidate_section_title: &'static str,
    pub inspectable_panels: &'static str,
    pub command_hint: String,
    pub visible_executable_inputs: Vec<ClientInput>,
}

pub fn build_decision_surface(session: &RunControlSession) -> DecisionSurface {
    let view = build_run_control_view_model(session);
    let visible_executable_inputs = view
        .candidates
        .iter()
        .filter_map(|candidate| candidate.action.executable_input())
        .collect::<Vec<_>>();
    DecisionSurface {
        candidate_section_title: candidate_section_title(session),
        inspectable_panels: inspectable_panels(session),
        command_hint: main_command_hint(session, &view),
        view,
        visible_executable_inputs,
    }
}

pub fn resolve_surface_candidate<'a>(
    surface: &'a DecisionSurface,
    engine_state: &EngineState,
    raw_id: &str,
) -> Option<&'a DecisionCandidate> {
    resolve_candidate_alias(&surface.view.candidates, engine_state, raw_id)
}

pub(super) fn resolve_candidate_alias<'a>(
    candidates: &'a [DecisionCandidate],
    engine_state: &EngineState,
    raw_id: &str,
) -> Option<&'a DecisionCandidate> {
    if let Some(candidate) = candidates.iter().find(|candidate| candidate.id == raw_id) {
        return Some(candidate);
    }

    let id = raw_id.trim().to_ascii_lowercase();
    if let Some(candidate) = candidates.iter().find(|candidate| candidate.id == id) {
        return Some(candidate);
    }
    if id.chars().all(|ch| ch.is_ascii_digit()) && !id.is_empty() {
        let structured = match engine_state {
            EngineState::Shop(_) => Some(format!("card-{id}")),
            EngineState::Campfire => Some(format!("smith-{id}")),
            _ => None,
        };
        if let Some(structured) = structured {
            if let Some(candidate) = candidates
                .iter()
                .find(|candidate| candidate.id == structured)
            {
                return Some(candidate);
            }
        }
    }

    match id.as_str() {
        "leave" | "skip" => candidates.iter().find(|candidate| {
            let label = candidate
                .label
                .trim_start()
                .to_ascii_lowercase()
                .trim_end_matches(['.', '!', '?'])
                .to_string();
            label.starts_with(&id)
        }),
        _ => None,
    }
}

pub fn surface_allows_visible_input(surface: &DecisionSurface, input: &ClientInput) -> bool {
    surface
        .visible_executable_inputs
        .iter()
        .any(|candidate_input| candidate_input == input)
}

#[cfg(test)]
pub(super) fn surface_legal_visibility_violations(session: &RunControlSession) -> Vec<String> {
    let surface = build_decision_surface(session);
    let mut violations = Vec::new();
    for candidate in &surface.view.candidates {
        if candidate.action.executable_input().is_none() {
            continue;
        }
        if candidate.id.trim().is_empty() {
            violations.push(format!(
                "visible candidate '{}' has empty id",
                candidate.label
            ));
        }
        if candidate.label.trim().is_empty() {
            violations.push(format!(
                "visible candidate '{}' has empty label",
                candidate.id
            ));
        }
    }

    if let Ok(position) = session.current_combat_position_for_actions() {
        if matches!(position.engine, EngineState::PendingChoice(_)) {
            if super::selection_surface::active_selection_surface(session).is_some() {
                if !surface
                    .view
                    .candidates
                    .iter()
                    .any(|candidate| candidate.id == "select")
                {
                    violations.push(
                        "compact selection surface is missing select command candidate".to_string(),
                    );
                }
                return violations;
            }
            let legal_moves = crate::sim::combat_legal_actions::get_legal_moves(
                &position.engine,
                &position.combat,
            );
            for legal in legal_moves {
                if !surface_allows_visible_input(&surface, &legal) {
                    violations.push(format!(
                        "pending choice legal input '{}' is not visible",
                        super::view_model::client_input_hint(&legal)
                    ));
                }
            }
        }
    }
    violations
}

fn candidate_section_title(session: &RunControlSession) -> &'static str {
    match &session.engine_state {
        _ if super::selection_surface::active_selection_surface(session).is_some() => {
            "Selection commands:"
        }
        EngineState::EventRoom => {
            if session.run_state.event_state.as_ref().is_some_and(|event| {
                event.id == crate::state::events::EventId::Neow && event.current_screen > 0
            }) {
                "Options:"
            } else {
                "Available action:"
            }
        }
        EngineState::PendingChoice(_) => "Selections:",
        EngineState::CombatPlayerTurn | EngineState::CombatProcessing => "Actions:",
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => "Choices:",
        EngineState::RewardOverlay { reward_state, .. }
            if reward_state.pending_card_choice.is_some() =>
        {
            "Choices:"
        }
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => "Paths:",
        _ => "Available actions:",
    }
}

fn inspectable_panels(session: &RunControlSession) -> &'static str {
    match &session.engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => {
            "deck | draw | discard | exhaust | relics | potions | inspect <id> | details | raw"
        }
        _ => "deck | map | relics | potions | inspect <id> | details | raw",
    }
}

fn main_command_hint(session: &RunControlSession, view: &RunControlViewModel) -> String {
    let first = view.candidates.first();
    let primary = match first {
        Some(candidate)
            if view.candidates.len() == 1 && candidate.action.executable_input().is_some() =>
        {
            format!("Enter/{}: {}", candidate.id, candidate.label)
        }
        Some(_) => state_command_hint(session),
        None => "type a command".to_string(),
    };
    let views = match session.engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => {
            "draw | discard | exhaust | potions | relics | case | raw | help | q"
        }
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => {
            "deck | map | ms | rs | rg | relics | potions | case | raw | help | q"
        }
        _ => "deck | map | relics | potions | case | raw | help | q",
    };
    let baseline = if session.last_completed_manual_combat_matches_capture_case() {
        " | baseline"
    } else {
        ""
    };
    format!("{primary} | {views}{baseline}")
}

fn state_command_hint(session: &RunControlSession) -> String {
    match &session.engine_state {
        EngineState::Shop(_) => {
            "card-2 or card 2 | relic-1 or relic 1 | potion-0 or potion 0 | leave".to_string()
        }
        EngineState::Campfire => "rest | smith-<deck_idx> or smith <deck_idx> | recall".to_string(),
        EngineState::MapNavigation => {
            "type a path id, e.g. 0 or 5 | map=full map | rg=route-go".to_string()
        }
        EngineState::MapOverlay { .. } => {
            "type a path id to commit, or back/cancel to return".to_string()
        }
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            "type visible id to take card; rp <id> records pick; back".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. }
            if reward_state.pending_card_choice.is_some() =>
        {
            "type visible id to take card; rp <id> records pick; bowl; back".to_string()
        }
        EngineState::RewardOverlay { .. } => "type visible id, bowl, or back".to_string(),
        EngineState::RewardScreen(reward) if reward.has_card_reward_item() => {
            "type visible id to open reward; rp <card_idx> records first card reward pick; skip"
                .to_string()
        }
        EngineState::RewardScreen(_) => "type visible id, pick <idx>, or skip".to_string(),
        EngineState::PendingChoice(_)
            if super::selection_surface::active_selection_surface(session).is_some() =>
        {
            "select <idx...> | select = choose nothing | cancel".to_string()
        }
        EngineState::PendingChoice(_) => "type visible selection id".to_string(),
        EngineState::CombatPlayerTurn | EngineState::CombatProcessing => {
            "cap <case_id> | n | visible action id | end".to_string()
        }
        _ => "type visible id".to_string(),
    }
}
