#![allow(dead_code)]
// TurnPlan V1 is intentionally staged: this module is tested independently
// before it is allowed to steer the main combat search frontier.

mod diagnostics;
mod enumerate;
mod types;

pub(in crate::ai::combat_search_v2) use diagnostics::TurnPlanDiagnosticsCollector;

#[cfg(test)]
mod tests;
