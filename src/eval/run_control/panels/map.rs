use super::{format_first_floor, push_line};
use crate::eval::run_control::route_policy::{route_targets, summarize_route_from};
use crate::eval::run_control::session::RunControlSession;
use crate::eval::run_control::view_model::{boss_label, room_type_label};
use crate::state::core::EngineState;
use crate::state::events::EventId;

pub fn render_map_panel(session: &RunControlSession) -> String {
    let mut out = String::new();
    let navigable = matches!(session.engine_state, EngineState::MapNavigation);
    push_line(
        &mut out,
        format!(
            "{} to Act {} boss: {}",
            if navigable {
                "Map route summary"
            } else {
                "Map route preview"
            },
            session.run_state.act_num,
            boss_label(&session.run_state)
        ),
    );
    if !navigable {
        push_line(&mut out, map_locked_note(session));
    }
    push_line(
        &mut out,
        "Warning: path counts are visible graph paths, not policy probabilities.",
    );
    push_line(&mut out, "");

    let starts = route_targets(session);
    if starts.is_empty() {
        push_line(&mut out, "No visible legal map targets.");
    } else {
        push_line(
            &mut out,
            if navigable {
                "Start choices:"
            } else {
                "Visible routes:"
            },
        );
        for node in starts {
            let summary = summarize_route_from(session, node.x, node.y);
            push_line(
                &mut out,
                format!("  x={} {}", node.x, room_type_label(node.class)),
            );
            push_line(
                &mut out,
                format!(
                    "    early pressure: {} monster/elite rooms before floor 4",
                    summary.early_pressure_label()
                ),
            );
            push_line(&mut out, format!("    elites: {}", summary.elite_label()));
            push_line(
                &mut out,
                format!(
                    "    shops: {}, first shop {}",
                    summary.shop_label(),
                    format_first_floor(summary.first_shop_floor)
                ),
            );
            push_line(
                &mut out,
                format!(
                    "    fires: {}, first fire {}",
                    summary.fire_label(),
                    format_first_floor(summary.first_fire_floor)
                ),
            );
            push_line(
                &mut out,
                format!("    recovery: {}", summary.recovery_label()),
            );
            push_line(
                &mut out,
                format!("    flexibility: {} visible paths", summary.path_count),
            );
        }
    }
    push_line(&mut out, "");
    if navigable {
        push_line(&mut out, "Commands: main | rs | go <x> | details | raw | q");
    } else {
        push_line(&mut out, "Commands: main | rs | details | raw | q");
    }
    out
}

fn map_locked_note(session: &RunControlSession) -> &'static str {
    if session.run_state.event_state.as_ref().is_some_and(|event| {
        event.id == EventId::Neow && matches!(session.engine_state, EngineState::EventRoom)
    }) {
        "Read-only: first room selection is locked until Neow is complete."
    } else {
        "Read-only: route selection is locked until the current screen returns to map navigation."
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::session::RunControlConfig;
    use crate::state::core::EngineState;

    #[test]
    fn map_panel_shows_route_summary_on_demand() {
        let session = test_session_after_neow_at_map();
        let rendered = render_map_panel(&session);

        assert!(rendered.contains("Map route summary"));
        assert!(rendered.contains("Start choices:"));
        assert!(rendered.contains("early pressure:"));
        assert!(rendered.contains("Warning: path counts"));
    }

    #[test]
    fn neow_map_panel_is_read_only_and_does_not_offer_go() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session
            .apply_command(crate::eval::run_control::commands::RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");

        let rendered = render_map_panel(&session);

        assert!(rendered.contains("Map route preview"));
        assert!(rendered.contains("first room selection is locked until Neow is complete"));
        assert!(rendered.contains("Visible routes:"));
        assert!(!rendered.contains("go <x>"));
    }

    fn test_session_after_neow_at_map() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        session
    }
}
