use crate::ai::combat_search_v2::{
    CombatSearchV2Config, CombatSearchV2Report, CombatSearchV2TrajectoryReport,
};
use crate::sim::combat::CombatPosition;

use super::combat_candidate_line::CombatCandidateLine;
use super::combat_line_outcome::{
    evaluate_combat_candidate_line_outcome, find_accepted_alternative_in_report,
    find_clean_no_potion_alternative, render_combat_line_outcome_detail,
    CombatLineAcceptancePolicy,
};
use super::session::RunControlSession;

pub(super) struct SelectedCombatLine {
    pub(super) line: CombatCandidateLine,
    pub(super) report: Option<CombatSearchV2Report>,
    pub(super) summary: Option<String>,
}

pub(super) enum CombatLineSelection {
    Selected(SelectedCombatLine),
    DirtyRejected { detail: String },
}

pub(super) fn select_accepted_search_combat_line(
    session: &RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    report: &CombatSearchV2Report,
    trajectory: &CombatSearchV2TrajectoryReport,
) -> Result<CombatLineSelection, String> {
    let original_line = CombatCandidateLine::from_search_trajectory(trajectory);
    let mut selected_line = original_line.clone();
    let mut selected_report = None;
    let mut summary = None;
    if let Some(repair) =
        super::combat_line_repair::try_repair_winning_trajectory(start, trajectory, config)
    {
        selected_line = repair.line;
        summary = Some(format!(
            "line_repair attempts={} wins={} improvements={} elapsed_ms={} original_hp_loss={} repaired_hp_loss={}",
            repair.attempts,
            repair.wins,
            repair.improvements,
            repair.elapsed_ms,
            trajectory.hp_loss,
            selected_line.hp_loss
        ));
    }
    let policy = CombatLineAcceptancePolicy::default();
    let selected_eval =
        evaluate_combat_candidate_line_outcome(session, start, config, selected_line.clone())?;
    if policy.classify(&selected_eval.outcome).is_rejected() {
        if let Some(alternative) =
            find_accepted_alternative_in_report(session, start, config, report, policy)?
        {
            selected_line = alternative.line;
            append_selection_summary(
                &mut summary,
                format!(
                    "same_report_clean_alternative replaced dirty_win gained_curses={} original_final_hp={} clean_final_hp={}",
                    selected_eval.outcome.gained_curse_count(),
                    selected_eval.outcome.final_hp,
                    alternative.outcome.final_hp
                ),
            );
        } else if let Some(alternative) =
            find_clean_no_potion_alternative(session, start, config, policy)?
        {
            selected_line = alternative.line;
            selected_report = Some(alternative.report);
            append_selection_summary(
                &mut summary,
                format!(
                    "clean_no_potion_alternative replaced dirty_win gained_curses={} original_final_hp={} clean_final_hp={}",
                    selected_eval.outcome.gained_curse_count(),
                    selected_eval.outcome.final_hp,
                    alternative.outcome.final_hp
                ),
            );
        } else {
            return Ok(CombatLineSelection::DirtyRejected {
                detail: render_combat_line_outcome_detail(&selected_eval.outcome),
            });
        }
    }
    Ok(CombatLineSelection::Selected(SelectedCombatLine {
        line: selected_line,
        report: selected_report,
        summary,
    }))
}

fn append_selection_summary(summary: &mut Option<String>, item: String) {
    match summary {
        Some(summary) => {
            summary.push(' ');
            summary.push_str(&item);
        }
        None => *summary = Some(item),
    }
}
