use crate::sim::combat::CombatPosition;

use super::super::turn_plan_probe_report::{
    CombatSearchV2TurnPlanProbeCandidateReport, CombatSearchV2TurnPlanProbeRootReport,
};

#[derive(Clone)]
pub(crate) struct CombatSearchV2TurnPlanProbeCandidate {
    pub(crate) report: CombatSearchV2TurnPlanProbeCandidateReport,
    pub(crate) position: CombatPosition,
}

#[derive(Clone)]
pub(crate) struct CombatSearchV2TurnPlanProbeEnumeration {
    pub(crate) report: CombatSearchV2TurnPlanProbeRootReport,
    pub(crate) candidates: Vec<CombatSearchV2TurnPlanProbeCandidate>,
}
