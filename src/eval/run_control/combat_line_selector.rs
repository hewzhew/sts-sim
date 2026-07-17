use crate::ai::combat_search_v2::{
    CombatSearchV2Config, CombatSearchV2Report, CombatSearchV2TrajectoryReport,
};
use crate::sim::combat::CombatPosition;

use super::combat_candidate_line::CombatCandidateLine;
use super::combat_line_adjudication::{
    CombatLineAcceptancePolicy, CombatLineAdjudicationV1, CombatLineCleanlinessV1,
};
use super::combat_line_outcome::{
    evaluate_combat_candidate_line_outcome, find_accepted_alternative_in_report,
    render_combat_line_outcome_detail,
};
use super::session::RunControlSession;

pub(super) struct SelectedCombatLine {
    pub(super) line: CombatCandidateLine,
    pub(super) summary: Option<String>,
    pub(super) adjudication: CombatLineAdjudicationV1,
}

pub(super) enum CombatLineSelection {
    Selected(SelectedCombatLine),
    Rejected {
        adjudication: CombatLineAdjudicationV1,
        detail: String,
    },
    ReplayFailed {
        adjudication: CombatLineAdjudicationV1,
    },
}

pub(super) fn select_accepted_search_combat_line(
    session: &RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    report: &CombatSearchV2Report,
    trajectory: &CombatSearchV2TrajectoryReport,
    policy: CombatLineAcceptancePolicy,
) -> CombatLineSelection {
    let selected_line = CombatCandidateLine::from_search_trajectory(trajectory);
    let mut summary = None;

    let selected_eval =
        match evaluate_combat_candidate_line_outcome(session, start, config, selected_line.clone())
        {
            Ok(evaluation) => evaluation,
            Err(error) => return replay_failed(policy, error),
        };
    let selected_adjudication = policy.adjudicate(selected_eval.outcome.clone());
    if matches!(
        &selected_adjudication,
        CombatLineAdjudicationV1::Accepted { .. }
    ) {
        return CombatLineSelection::Selected(SelectedCombatLine {
            line: selected_eval.line,
            summary,
            adjudication: selected_adjudication,
        });
    }
    if !policy.requires_clean_line() {
        return CombatLineSelection::Rejected {
            detail: render_combat_line_outcome_detail(&selected_eval.outcome),
            adjudication: selected_adjudication,
        };
    }

    let same_report_alternative =
        match find_accepted_alternative_in_report(session, start, config, report, policy) {
            Ok(alternative) => alternative,
            Err(error) => return replay_failed(policy, error),
        };
    if let Some(alternative) = same_report_alternative {
        let adjudication = policy.adjudicate(alternative.outcome.clone());
        debug_assert!(matches!(
            &adjudication,
            CombatLineAdjudicationV1::Accepted {
                cleanliness: CombatLineCleanlinessV1::Clean,
                ..
            }
        ));
        append_selection_summary(
            &mut summary,
            format!(
                "same_report_clean_alternative replaced dirty_win gained_curses={} original_final_hp={} clean_final_hp={}",
                selected_eval.outcome.gained_curse_count(),
                selected_eval.outcome.final_hp,
                alternative.outcome.final_hp
            ),
        );
        return CombatLineSelection::Selected(SelectedCombatLine {
            line: alternative.line,
            summary,
            adjudication,
        });
    }

    CombatLineSelection::Rejected {
        detail: render_combat_line_outcome_detail(&selected_eval.outcome),
        adjudication: selected_adjudication,
    }
}

fn replay_failed(policy: CombatLineAcceptancePolicy, error: String) -> CombatLineSelection {
    CombatLineSelection::ReplayFailed {
        adjudication: CombatLineAdjudicationV1::ReplayFailed {
            policy: policy.plugin(),
            error,
        },
    }
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
