use crate::sim::combat::CombatPosition;

use super::super::turn_plan_probe_report::{
    CombatSearchV2TurnPlanProbeCandidateReport, CombatSearchV2TurnPlanProbeRootReport,
};

#[derive(Clone)]
pub struct CombatSearchV2TurnPlanProbeCandidate {
    pub report: CombatSearchV2TurnPlanProbeCandidateReport,
    pub position: CombatPosition,
}

#[derive(Clone)]
pub struct CombatSearchV2TurnPlanProbeEnumeration {
    pub report: CombatSearchV2TurnPlanProbeRootReport,
    pub candidates: Vec<CombatSearchV2TurnPlanProbeCandidate>,
}
