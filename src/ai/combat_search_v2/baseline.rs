use super::*;

pub const WHOLE_COMBAT_OUTCOME_CRITERIA: [&str; 5] = [
    "win_over_loss",
    "higher_final_hp",
    "fewer_potions_used",
    "fewer_turns",
    "fewer_cards_played",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CombatSearchV2OutcomeMetrics {
    pub terminal: SearchTerminalLabel,
    pub final_hp: i32,
    pub potions_used: u32,
    pub turns: u32,
    pub cards_played: u32,
}

impl CombatSearchV2OutcomeMetrics {
    pub fn from_trajectory(trajectory: &CombatSearchV2TrajectoryReport) -> Self {
        Self {
            terminal: trajectory.terminal,
            final_hp: trajectory.final_hp,
            potions_used: trajectory.potions_used,
            turns: trajectory.turns,
            cards_played: trajectory.cards_played,
        }
    }
}

pub fn compare_trajectory_reports(
    search: Option<&CombatSearchV2TrajectoryReport>,
    search_exhaustive: bool,
    baseline: &CombatSearchV2TrajectoryReport,
) -> serde_json::Value {
    let Some(search) = search else {
        return serde_json::json!({
            "verdict": "inconclusive_no_search_complete_trajectory",
            "basis": "whole_combat_outcome",
        });
    };
    if !search_exhaustive || search.terminal == SearchTerminalLabel::Unresolved {
        return serde_json::json!({
            "verdict": "inconclusive_unresolved_search",
            "basis": "whole_combat_outcome",
            "reason": "search has unresolved frontier and cannot claim not-weaker-than-baseline",
            "baseline_terminal": baseline.terminal,
            "search_complete_candidate_terminal": search.terminal,
        });
    }

    let ordering = compare_outcome_metrics(
        CombatSearchV2OutcomeMetrics::from_trajectory(search),
        CombatSearchV2OutcomeMetrics::from_trajectory(baseline),
    );
    serde_json::json!({
        "verdict": match ordering {
            Ordering::Greater => "search_better",
            Ordering::Equal => "search_tied",
            Ordering::Less => "baseline_better",
        },
        "basis": "whole_combat_outcome",
        "criteria_order": WHOLE_COMBAT_OUTCOME_CRITERIA,
        "search_terminal": search.terminal,
        "baseline_terminal": baseline.terminal,
        "search_final_hp": search.final_hp,
        "baseline_final_hp": baseline.final_hp,
        "search_potions_used": search.potions_used,
        "baseline_potions_used": baseline.potions_used,
        "search_turns": search.turns,
        "baseline_turns": baseline.turns,
    })
}

pub fn compare_outcome_metrics(
    left: CombatSearchV2OutcomeMetrics,
    right: CombatSearchV2OutcomeMetrics,
) -> Ordering {
    terminal_rank(left.terminal)
        .cmp(&terminal_rank(right.terminal))
        .then_with(|| left.final_hp.cmp(&right.final_hp))
        .then_with(|| right.potions_used.cmp(&left.potions_used))
        .then_with(|| right.turns.cmp(&left.turns))
        .then_with(|| right.cards_played.cmp(&left.cards_played))
}
