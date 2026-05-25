use crate::state::core::EngineState;
use crate::state::map::node::RoomType;

use super::session::RunControlSession;
use super::view_model::{boss_label, room_type_label};

mod assessment;
mod summary;

use assessment::{assess_route, tier_label, RoutePreference};
pub(in crate::eval::run_control) use summary::{route_targets, summarize_route_from};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::eval::run_control) struct MapRouteTarget {
    pub x: i32,
    pub y: i32,
    pub class: Option<RoomType>,
    pub has_emerald_key: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::eval::run_control) struct RouteSummary {
    pub path_count: usize,
    pub min_early_pressure: usize,
    pub max_early_pressure: usize,
    pub min_elites: usize,
    pub max_elites: usize,
    pub min_shops: usize,
    pub max_shops: usize,
    pub min_fires: usize,
    pub max_fires: usize,
    pub first_shop_floor: Option<i32>,
    pub first_fire_floor: Option<i32>,
}

impl RouteSummary {
    pub(in crate::eval::run_control) fn early_pressure_label(&self) -> String {
        format_range(self.min_early_pressure, self.max_early_pressure)
    }

    pub(in crate::eval::run_control) fn elite_label(&self) -> String {
        format_range(self.min_elites, self.max_elites)
    }

    pub(in crate::eval::run_control) fn shop_label(&self) -> String {
        format_range(self.min_shops, self.max_shops)
    }

    pub(in crate::eval::run_control) fn fire_label(&self) -> String {
        format_range(self.min_fires, self.max_fires)
    }

    pub(in crate::eval::run_control) fn recovery_label(&self) -> &'static str {
        if self.min_fires > 0 {
            "rest site exists on every visible path"
        } else if self.max_fires > 0 {
            "rest site exists on some visible paths"
        } else {
            "not visible on this route"
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::eval::run_control) struct RouteCandidate {
    pub target: MapRouteTarget,
    pub summary: RouteSummary,
    pub assessment: RouteAssessment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::eval::run_control) struct RouteAssessment {
    pub tier: RouteTier,
    pub reasons: Vec<String>,
    pub cautions: Vec<String>,
    preference: RoutePreference,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::eval::run_control) enum RouteTier {
    Avoid,
    Conservative,
    Flexible,
    Preferred,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::eval::run_control) struct RouteSuggestion {
    pub navigable: bool,
    pub candidates: Vec<RouteCandidate>,
    pub best_indices: Vec<usize>,
}

pub(in crate::eval::run_control) fn render_route_suggestion(session: &RunControlSession) -> String {
    let suggestion = build_route_suggestion(session);
    let mut out = String::new();
    push_line(
        &mut out,
        format!(
            "Route suggestion to Act {} boss: {}",
            session.run_state.act_num,
            boss_label(&session.run_state)
        ),
    );
    push_line(
        &mut out,
        "Policy: balanced_act1_v0 (read-only; no route is selected here)",
    );
    if !suggestion.navigable {
        push_line(
            &mut out,
            "Status: route selection is locked until the current screen returns to map navigation.",
        );
    }
    push_line(
        &mut out,
        "Warning: path counts are visible graph paths, not policy probabilities.",
    );
    push_line(&mut out, "");

    if suggestion.candidates.is_empty() {
        push_line(&mut out, "No visible legal map targets.");
        return out;
    }

    push_line(&mut out, "Candidates:");
    for (idx, candidate) in suggestion.candidates.iter().enumerate() {
        let marker = if suggestion.best_indices.contains(&idx) {
            "*"
        } else {
            " "
        };
        push_line(
            &mut out,
            format!(
                "{marker} x={} {} [{}]",
                candidate.target.x,
                room_type_label(candidate.target.class),
                tier_label(candidate.assessment.tier)
            ),
        );
        for reason in &candidate.assessment.reasons {
            push_line(&mut out, format!("    + {reason}"));
        }
        for caution in &candidate.assessment.cautions {
            push_line(&mut out, format!("    ! {caution}"));
        }
        push_line(
            &mut out,
            format!(
                "    paths={} elites={} fires={} shops={} early_pressure={}",
                candidate.summary.path_count,
                candidate.summary.elite_label(),
                candidate.summary.fire_label(),
                candidate.summary.shop_label(),
                candidate.summary.early_pressure_label(),
            ),
        );
    }
    push_line(&mut out, "");
    match suggestion.best_indices.as_slice() {
        [idx] if suggestion.navigable => {
            let target = &suggestion.candidates[*idx].target;
            push_line(
                &mut out,
                format!("Suggested command: go {}  (not executed)", target.x),
            );
        }
        [_] => push_line(
            &mut out,
            "Suggested command: none while map selection is locked.",
        ),
        [] => push_line(&mut out, "Suggested command: none."),
        _ => push_line(
            &mut out,
            "Suggested command: no unique route; compare the starred candidates.",
        ),
    }
    out
}

fn build_route_suggestion(session: &RunControlSession) -> RouteSuggestion {
    let navigable = matches!(session.engine_state, EngineState::MapNavigation);
    let mut candidates = route_targets(session)
        .into_iter()
        .map(|target| {
            let summary = summarize_route_from(session, target.x, target.y);
            let assessment = assess_route(&target, &summary);
            RouteCandidate {
                target,
                summary,
                assessment,
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| {
        b.assessment
            .preference
            .cmp(&a.assessment.preference)
            .then_with(|| a.target.x.cmp(&b.target.x))
    });
    let best_indices = candidates
        .first()
        .map(|best| {
            candidates
                .iter()
                .enumerate()
                .filter_map(|(idx, candidate)| {
                    (candidate.assessment.preference == best.assessment.preference).then_some(idx)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    RouteSuggestion {
        navigable,
        candidates,
        best_indices,
    }
}

pub(super) fn format_range(min: usize, max: usize) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}-{max}")
    }
}

pub(super) fn format_first_floor(floor: Option<i32>) -> String {
    floor
        .map(|floor| format!("floor {floor}"))
        .unwrap_or_else(|| "none".to_string())
}

fn push_line(out: &mut String, line: impl AsRef<str>) {
    out.push_str(line.as_ref());
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::session::RunControlConfig;

    #[test]
    fn route_suggestion_is_read_only_before_map_navigation() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session
            .apply_command(crate::eval::run_control::commands::RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");

        let rendered = render_route_suggestion(&session);

        assert!(rendered.contains("read-only"));
        assert!(rendered.contains("route selection is locked"));
        assert!(!rendered.contains("Suggested command: go"));
    }

    #[test]
    fn route_suggestion_recommends_without_mutating_map_position() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        let before = (
            session.run_state.map.current_x,
            session.run_state.map.current_y,
        );

        let rendered = render_route_suggestion(&session);

        assert!(rendered.contains("Route suggestion"));
        assert!(rendered.contains("Suggested command: go"));
        assert_eq!(
            before,
            (
                session.run_state.map.current_x,
                session.run_state.map.current_y
            )
        );
    }

    #[test]
    fn route_suggest_command_is_read_only() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        let before = (
            session.run_state.map.current_x,
            session.run_state.map.current_y,
            session.decision_step,
        );

        let outcome = session
            .apply_command(crate::eval::run_control::commands::RunControlCommand::RouteSuggest)
            .expect("route-suggest should render");

        assert!(outcome.message.contains("Route suggestion"));
        assert!(outcome.action_result.is_none());
        assert_eq!(
            before,
            (
                session.run_state.map.current_x,
                session.run_state.map.current_y,
                session.decision_step
            )
        );
    }
}
