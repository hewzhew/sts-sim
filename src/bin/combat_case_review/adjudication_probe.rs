use sts_simulator::ai::combat_search_v2::{CombatSearchV2Config, CombatSearchV2TrajectoryReport};
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::eval::run_control::{
    adjudicate_combat_case_line_v1, CombatCaseAdjudicationProbeV1,
};

pub(super) struct ReviewAdjudicationCandidate {
    pub(super) source_review: &'static str,
    pub(super) config: CombatSearchV2Config,
    pub(super) trajectory: CombatSearchV2TrajectoryReport,
}

pub(super) fn run_adjudication_probe(
    enabled: bool,
    candidates: &[ReviewAdjudicationCandidate],
    focus_label: Option<&str>,
    case: Option<&CombatCase>,
) -> Option<CombatCaseAdjudicationProbeV1> {
    if !enabled {
        return None;
    }
    let candidate = focus_label
        .and_then(|label| {
            candidates
                .iter()
                .find(|candidate| candidate.source_review == label)
        })
        .or_else(|| candidates.first());
    let Some(candidate) = candidate else {
        return Some(CombatCaseAdjudicationProbeV1::NoCompleteLine);
    };
    let Some(case) = case else {
        return Some(CombatCaseAdjudicationProbeV1::ProjectionFailed {
            source_review: candidate.source_review.to_string(),
            error: "combat case unavailable".to_string(),
        });
    };
    Some(adjudicate_combat_case_line_v1(
        candidate.source_review,
        case,
        &candidate.config,
        &candidate.trajectory,
    ))
}

#[cfg(test)]
mod tests {
    use sts_simulator::eval::run_control::CombatCaseAdjudicationProbeV1;

    #[test]
    fn enabled_probe_without_complete_line_is_typed() {
        assert_eq!(
            super::run_adjudication_probe(true, &[], None, None),
            Some(CombatCaseAdjudicationProbeV1::NoCompleteLine)
        );
    }

    #[test]
    fn disabled_probe_is_absent_from_review_artifacts() {
        assert_eq!(super::run_adjudication_probe(false, &[], None, None), None);
    }
}
