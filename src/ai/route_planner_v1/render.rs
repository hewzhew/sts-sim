use crate::state::map::node::RoomType;

use super::types::{RouteDecisionTraceV1, RouteMoveKindV1, RoutePathSummaryV1, RouteSafetyFlagV1};

pub fn render_route_decision_trace_v1(trace: &RouteDecisionTraceV1) -> String {
    let mut out = String::new();
    push_line(
        &mut out,
        format!(
            "Route suggestion to Act {} boss: {}",
            trace.context.act,
            trace.context.boss.as_deref().unwrap_or("unknown")
        ),
    );
    push_line(
        &mut out,
        "Policy: route_planner_v1/data_collection_survival (read-only; no route is selected here)",
    );
    push_line(&mut out, format!("Label role: {}", trace.label_role));
    for warning in &trace.warnings {
        push_line(&mut out, format!("Warning: {warning}."));
    }
    push_line(
        &mut out,
        "Warning: path counts are visible graph paths, not policy probabilities.",
    );
    push_line(&mut out, "");

    if trace.candidates.is_empty() {
        push_line(&mut out, "No visible legal map targets.");
        return out;
    }

    push_line(&mut out, "Candidates:");
    for (idx, candidate) in trace.candidates.iter().enumerate() {
        let marker = if trace.selected_index == Some(idx) {
            "*"
        } else {
            " "
        };
        push_line(
            &mut out,
            format!(
                "{marker} x={} {} [{} score={:.2}]",
                candidate.target.x,
                room_label(candidate.target.room_type),
                safety_label(candidate.safety),
                candidate.total_score
            ),
        );
        if candidate.target.move_kind == RouteMoveKindV1::WingBootsJump {
            push_line(&mut out, "    ! uses Wing Boots charge");
        }
        for reason in &candidate.reasons {
            push_line(&mut out, format!("    + {reason}"));
        }
        for caution in &candidate.cautions {
            push_line(&mut out, format!("    ! {caution}"));
        }
        push_line(
            &mut out,
            format!(
                "    terms: card={:.2} relic={:.2} shop={:.2} heal={:.2} hp={:.2} risk={:.2} flex={:.2} elite_prep={:.2}",
                candidate.score_terms.card_reward,
                candidate.score_terms.relic,
                candidate.score_terms.shop,
                candidate.score_terms.heal,
                candidate.score_terms.hp_loss,
                candidate.score_terms.death_risk,
                candidate.score_terms.flexibility,
                candidate.score_terms.elite_prep,
            ),
        );
        push_line(
            &mut out,
            format!("    path: {}", path_line(&candidate.path_summary)),
        );
    }
    push_line(&mut out, "");
    match trace.selected_index {
        Some(idx) => {
            let candidate = &trace.candidates[idx];
            if let Some(command) = candidate.suggested_command.as_ref() {
                push_line(
                    &mut out,
                    format!("Suggested command: {command}  (not executed)"),
                );
            } else {
                push_line(
                    &mut out,
                    "Suggested command: none while map selection is locked.",
                );
            }
        }
        None => push_line(&mut out, "Suggested command: none."),
    }
    out
}

fn path_line(path: &RoutePathSummaryV1) -> String {
    format!(
        "paths={} elites={} fires={} shops={} unknowns={} treasures={} early_pressure={} first_elite={}",
        path.path_count,
        format_range(path.min_elites, path.max_elites),
        format_range(path.min_fires, path.max_fires),
        format_range(path.min_shops, path.max_shops),
        format_range(path.min_unknowns, path.max_unknowns),
        format_range(path.min_treasures, path.max_treasures),
        format_range(path.min_early_pressure, path.max_early_pressure),
        first_elite_line(path),
    )
}

fn first_elite_line(path: &RoutePathSummaryV1) -> String {
    let segment = &path.first_elite;
    if segment.paths_with_first_elite == 0 {
        return "none".to_string();
    }
    let mode = if segment.forced {
        "forced"
    } else if segment.optional {
        "optional"
    } else {
        "seen"
    };
    let mut bailouts = Vec::new();
    if segment.can_bail_to_rest_before {
        bailouts.push("rest");
    }
    if segment.can_bail_to_shop_before {
        bailouts.push("shop");
    }
    let bailout = if bailouts.is_empty() {
        "-".to_string()
    } else {
        bailouts.join("+")
    };
    format!(
        "{} prep_hallways={} prep_?={} bailout={}",
        mode,
        format_range(
            segment.min_hallway_fights_before,
            segment.max_hallway_fights_before
        ),
        format_range(segment.min_unknowns_before, segment.max_unknowns_before),
        bailout
    )
}

fn room_label(room_type: Option<RoomType>) -> &'static str {
    match room_type {
        Some(RoomType::EventRoom) => "?",
        Some(RoomType::MonsterRoom) => "Monster",
        Some(RoomType::MonsterRoomElite) => "Elite",
        Some(RoomType::MonsterRoomBoss) => "Boss",
        Some(RoomType::RestRoom) => "Rest",
        Some(RoomType::ShopRoom) => "Shop",
        Some(RoomType::TreasureRoom) => "Treasure",
        Some(RoomType::TrueVictoryRoom) => "TrueVictory",
        None => "Unknown",
    }
}

fn safety_label(safety: RouteSafetyFlagV1) -> &'static str {
    match safety {
        RouteSafetyFlagV1::Ok => "ok",
        RouteSafetyFlagV1::RiskyButAllowed => "risky",
        RouteSafetyFlagV1::RejectUnlessNoAlternative => "reject_unless_forced",
    }
}

fn format_range(min: usize, max: usize) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}-{max}")
    }
}

fn push_line(out: &mut String, line: impl AsRef<str>) {
    out.push_str(line.as_ref());
    out.push('\n');
}
