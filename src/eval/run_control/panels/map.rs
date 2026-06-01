use super::{format_first_floor, push_line};
use crate::eval::run_control::route_policy::{
    format_range, recovery_label, route_targets, summarize_route_from,
};
use crate::eval::run_control::session::RunControlSession;
use crate::eval::run_control::view_model::{boss_label, room_type_label};
use crate::state::core::EngineState;
use crate::state::events::EventId;
use crate::state::map::node::{Map, MapRoomNode, RoomType};
use std::cmp::Ordering;

pub fn render_map_panel(session: &RunControlSession) -> String {
    let mut out = String::new();
    let navigable = session.engine_state.is_map_surface();
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
                format!("  x={} {}", node.x, room_type_label(node.room_type)),
            );
            push_line(
                &mut out,
                format!(
                    "    early pressure: {} monster/elite rooms before floor 4",
                    format_range(summary.min_early_pressure, summary.max_early_pressure)
                ),
            );
            push_line(
                &mut out,
                format!(
                    "    elites: {}",
                    format_range(summary.min_elites, summary.max_elites)
                ),
            );
            push_line(
                &mut out,
                format!(
                    "    shops: {}, first shop {}",
                    format_range(summary.min_shops, summary.max_shops),
                    format_first_floor(summary.first_shop_floor)
                ),
            );
            push_line(
                &mut out,
                format!(
                    "    fires: {}, first fire {}",
                    format_range(summary.min_fires, summary.max_fires),
                    format_first_floor(summary.first_fire_floor)
                ),
            );
            push_line(
                &mut out,
                format!("    recovery: {}", recovery_label(&summary)),
            );
            push_line(
                &mut out,
                format!("    flexibility: {} visible paths", summary.path_count),
            );
        }
    }
    push_line(&mut out, "");
    if navigable {
        push_line(
            &mut out,
            "Commands: main | mf | rs | rg | go <x> | details | raw | q",
        );
    } else {
        push_line(&mut out, "Commands: main | mf | rs | details | raw | q");
    }
    out
}

pub fn render_full_map_panel(session: &RunControlSession) -> String {
    let mut out = String::new();
    push_line(
        &mut out,
        format!(
            "Full map to Act {} boss: {}",
            session.run_state.act_num,
            boss_label(&session.run_state)
        ),
    );
    push_line(
        &mut out,
        "Legend: M=Monster, E=Elite, ?=Unknown, $=Shop, R=Rest, T=Treasure, B=Boss, V=TrueVictory, *=unassigned",
    );
    push_line(&mut out, current_position_line(session));
    push_line(&mut out, legal_next_line(session));
    push_line(&mut out, "");
    out.push_str(&format_map_grid(&session.run_state.map.graph));
    if !out.ends_with('\n') {
        out.push('\n');
    }
    push_line(&mut out, "");
    if session.engine_state.is_map_surface() {
        push_line(
            &mut out,
            if matches!(session.engine_state, EngineState::MapOverlay { .. }) {
                "Commands: map | rs | rg | go <x> | back | details | raw | q"
            } else {
                "Commands: map | rs | rg | go <x> | details | raw | q"
            },
        );
    } else {
        push_line(&mut out, "Commands: map | rs | details | raw | q");
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

fn current_position_line(session: &RunControlSession) -> String {
    let map = &session.run_state.map;
    if map.current_y < 0 {
        "Current: before first room".to_string()
    } else if map.current_y == 15 {
        "Current: boss room".to_string()
    } else {
        format!("Current: x={} y={}", map.current_x, map.current_y)
    }
}

fn legal_next_line(session: &RunControlSession) -> String {
    let targets = route_targets(session);
    if targets.is_empty() {
        return "Legal next: none visible".to_string();
    }
    let parts = targets
        .iter()
        .map(|node| {
            format!(
                "x={} y={} {}",
                node.x,
                node.y,
                room_type_label(node.room_type)
            )
        })
        .collect::<Vec<_>>();
    format!("Legal next: {}", parts.join(" | "))
}

fn format_map_grid(graph: &Map) -> String {
    let mut out = String::new();
    if graph.is_empty() {
        push_line(&mut out, "(empty map)");
        return out;
    }
    for row_num in (0..graph.len()).rev() {
        out.push_str("     ");
        for node in &graph[row_num] {
            let (mut left, mut mid, mut right) = (" ", " ", " ");
            for edge in &node.edges {
                match edge.dst_x.cmp(&node.x) {
                    Ordering::Less => left = r"\",
                    Ordering::Equal => mid = "|",
                    Ordering::Greater => right = "/",
                }
            }
            out.push_str(&format!("{left}{mid}{right}"));
        }
        out.push('\n');
        out.push_str(&format!("{row_num:>2}   "));
        for node in &graph[row_num] {
            let symbol = if is_visible_map_node(graph, node) {
                room_symbol(node.class)
            } else {
                " "
            };
            out.push_str(&format!(" {symbol} "));
        }
        out.push('\n');
    }
    out
}

fn is_visible_map_node(graph: &Map, node: &MapRoomNode) -> bool {
    node.class.is_some() && (!node.edges.is_empty() || has_incoming_edge(graph, node.x, node.y))
}

fn has_incoming_edge(graph: &Map, x: i32, y: i32) -> bool {
    graph.iter().flatten().any(|node| {
        node.edges
            .iter()
            .any(|edge| edge.dst_x == x && edge.dst_y == y)
    })
}

fn room_symbol(room_type: Option<RoomType>) -> &'static str {
    match room_type {
        Some(RoomType::EventRoom) => "?",
        Some(RoomType::MonsterRoom) => "M",
        Some(RoomType::MonsterRoomElite) => "E",
        Some(RoomType::MonsterRoomBoss) => "B",
        Some(RoomType::RestRoom) => "R",
        Some(RoomType::ShopRoom) => "$",
        Some(RoomType::TreasureRoom) => "T",
        Some(RoomType::TrueVictoryRoom) => "V",
        None => "*",
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
    fn full_map_panel_shows_complete_map_grid_on_demand() {
        let session = test_session_after_neow_at_map();
        let rendered = render_full_map_panel(&session);

        assert!(rendered.contains("Full map to Act"));
        assert!(rendered.contains("Legend: M=Monster"));
        assert!(rendered.contains("Current: before first room"));
        assert!(rendered.contains("Legal next:"));
        assert!(rendered.contains("\n14"));
        assert!(rendered.contains("\n 0"));
        assert!(rendered.contains(" M "));
    }

    #[test]
    fn full_map_panel_keeps_neow_preview_read_only() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session
            .apply_command(crate::eval::run_control::commands::RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");

        let rendered = render_full_map_panel(&session);

        assert!(rendered.contains("Full map to Act"));
        assert!(rendered.contains("Legal next:"));
        assert!(!rendered.contains("go <x>"));
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
