#![allow(dead_code)]
// TurnPlan V1 is intentionally staged: this module is tested independently
// before it is allowed to steer the main combat search frontier.

mod diagnostics;
mod enumerate;
mod frontier_seed;
mod types;

pub(in crate::ai::combat_search_v2) use diagnostics::TurnPlanDiagnosticsCollector;
pub(in crate::ai::combat_search_v2) use enumerate::enumerate_turn_plans;
pub(in crate::ai::combat_search_v2) use frontier_seed::turn_plan_frontier_seed;
pub(in crate::ai::combat_search_v2) use types::{
    TurnPlanBucket, TurnPlanEnumeration, TurnPlanFirstActionSummaryV1, TurnPlanStopReason,
    TurnPlanV1, TurnPlannerConfigV1,
};

#[cfg(test)]
mod tests;
