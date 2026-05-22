use crate::state::map::node::{MapRoomNode, RoomType};

use super::{format_first_floor, format_range, push_line};
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

    let starts = legal_next_map_nodes(session);
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
                    summary.early_pressure
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
            push_line(&mut out, format!("    recovery: {}", summary.recovery));
            push_line(
                &mut out,
                format!("    flexibility: {} visible paths", summary.path_count),
            );
        }
    }
    push_line(&mut out, "");
    if navigable {
        push_line(&mut out, "Commands: main | go <x> | details | raw | q");
    } else {
        push_line(&mut out, "Commands: main | details | raw | q");
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

#[derive(Clone, Debug)]
struct RouteSummary {
    path_count: usize,
    early_pressure: String,
    min_elites: usize,
    max_elites: usize,
    min_shops: usize,
    max_shops: usize,
    min_fires: usize,
    max_fires: usize,
    first_shop_floor: Option<i32>,
    first_fire_floor: Option<i32>,
    recovery: String,
}

#[derive(Clone, Copy, Debug, Default)]
struct PathStats {
    early_pressure: usize,
    elites: usize,
    shops: usize,
    fires: usize,
    first_shop_floor: Option<i32>,
    first_fire_floor: Option<i32>,
}

fn summarize_route_from(session: &RunControlSession, x: i32, y: i32) -> RouteSummary {
    let mut paths = Vec::new();
    collect_path_stats(session, x, y, PathStats::default(), &mut paths, 2_000);
    if paths.is_empty() {
        return RouteSummary {
            path_count: 0,
            early_pressure: "unknown".to_string(),
            min_elites: 0,
            max_elites: 0,
            min_shops: 0,
            max_shops: 0,
            min_fires: 0,
            max_fires: 0,
            first_shop_floor: None,
            first_fire_floor: None,
            recovery: "unknown".to_string(),
        };
    }
    let min = |f: fn(&PathStats) -> usize| paths.iter().map(f).min().unwrap_or(0);
    let max = |f: fn(&PathStats) -> usize| paths.iter().map(f).max().unwrap_or(0);
    let min_early = min(|stats| stats.early_pressure);
    let max_early = max(|stats| stats.early_pressure);
    let min_fires = min(|stats| stats.fires);
    RouteSummary {
        path_count: paths.len(),
        early_pressure: format_range(min_early, max_early),
        min_elites: min(|stats| stats.elites),
        max_elites: max(|stats| stats.elites),
        min_shops: min(|stats| stats.shops),
        max_shops: max(|stats| stats.shops),
        min_fires,
        max_fires: max(|stats| stats.fires),
        first_shop_floor: paths
            .iter()
            .filter_map(|stats| stats.first_shop_floor)
            .min(),
        first_fire_floor: paths
            .iter()
            .filter_map(|stats| stats.first_fire_floor)
            .min(),
        recovery: if min_fires > 0 {
            "rest site exists on every visible path".to_string()
        } else {
            "not guaranteed on every visible path".to_string()
        },
    }
}

fn collect_path_stats(
    session: &RunControlSession,
    x: i32,
    y: i32,
    current: PathStats,
    paths: &mut Vec<PathStats>,
    budget: usize,
) {
    if paths.len() >= budget {
        return;
    }
    let Some(node) = session
        .run_state
        .map
        .graph
        .get(y.max(0) as usize)
        .and_then(|row| row.get(x.max(0) as usize))
    else {
        return;
    };
    let current = update_path_stats(current, node);
    if node.edges.is_empty() || y >= 14 {
        paths.push(current);
        return;
    }
    for edge in &node.edges {
        collect_path_stats(session, edge.dst_x, edge.dst_y, current, paths, budget);
    }
}

fn update_path_stats(mut stats: PathStats, node: &MapRoomNode) -> PathStats {
    match node.class {
        Some(RoomType::MonsterRoom) => {
            if node.y <= 3 {
                stats.early_pressure += 1;
            }
        }
        Some(RoomType::MonsterRoomElite) => {
            stats.elites += 1;
            if node.y <= 3 {
                stats.early_pressure += 1;
            }
        }
        Some(RoomType::ShopRoom) => {
            stats.shops += 1;
            stats.first_shop_floor.get_or_insert(node.y + 1);
        }
        Some(RoomType::RestRoom) => {
            stats.fires += 1;
            stats.first_fire_floor.get_or_insert(node.y + 1);
        }
        _ => {}
    }
    stats
}

fn legal_next_map_nodes(session: &RunControlSession) -> Vec<&MapRoomNode> {
    let target_y = if session.run_state.map.current_y == -1 {
        0
    } else {
        session.run_state.map.current_y + 1
    };
    session
        .run_state
        .map
        .graph
        .get(target_y.max(0) as usize)
        .into_iter()
        .flat_map(|row| row.iter())
        .filter(|node| session.run_state.map.can_travel_to(node.x, node.y, false))
        .collect()
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
