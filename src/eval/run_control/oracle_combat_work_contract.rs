use serde::{Deserialize, Serialize};
use sts_combat_planner::OracleCombatWitness;

/// Stable diagnostic vocabulary for how the live combat portfolio handled
/// the local-search candidate visible in a progress report.
///
/// This is evidence only. It does not authorize candidate selection or carry
/// any live planner state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleCombatLocalCandidateDispositionV1 {
    SelectedIncumbent,
    RejectedPotionSpendMissesSatisfaction,
    RejectedPotionSpendLeavesUnrecoveredTheft,
    RejectedOutsidePotionContract,
    RejectedByPortfolioComparison,
}

/// Durable resume contract for one run-owned exact-combat search job.
///
/// The tactical frontier is deliberately absent: process restore rebuilds it
/// from the branch's exact combat root. The retained incumbent is data-only,
/// replay-exact evidence and is fully replayed and verified by the live work
/// owner before it can re-enter the portfolio.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunCombatWorkCheckpointV1 {
    pub consumed_nodes: u64,
    pub remaining_nodes: usize,
    pub remaining_engine_steps: usize,
    pub remaining_wall_ms: Option<u64>,
    pub quantum_count: usize,
    pub restart_count: usize,
    #[serde(default)]
    pub incumbent_revision: u64,
    #[serde(default)]
    pub quanta_since_incumbent_improvement: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_potions_used: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_potion_slots: Option<u64>,
    /// When true, a verified potion-free incumbent is protected from a
    /// higher-HP spending line that still misses the configured satisfaction.
    #[serde(default)]
    pub potion_spend_requires_satisfaction: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incumbent: Option<OracleCombatWitness>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_checkpoint_shape_retains_defaults_without_live_search_state() {
        let checkpoint: OracleRunCombatWorkCheckpointV1 =
            serde_json::from_value(serde_json::json!({
                "consumed_nodes": 13,
                "remaining_nodes": 21,
                "remaining_engine_steps": 34,
                "remaining_wall_ms": 55,
                "quantum_count": 2,
                "restart_count": 1
            }))
            .expect("legacy combat-work checkpoint");

        assert_eq!(checkpoint.incumbent_revision, 0);
        assert_eq!(checkpoint.quanta_since_incumbent_improvement, 0);
        assert_eq!(checkpoint.max_potions_used, None);
        assert_eq!(checkpoint.allowed_potion_slots, None);
        assert!(!checkpoint.potion_spend_requires_satisfaction);
        assert!(checkpoint.incumbent.is_none());

        let encoded = serde_json::to_value(checkpoint).expect("combat-work checkpoint JSON");
        assert_eq!(encoded["consumed_nodes"], 13);
        assert_eq!(encoded["remaining_nodes"], 21);
        assert!(encoded.get("max_potions_used").is_none());
        assert!(encoded.get("allowed_potion_slots").is_none());
        assert!(encoded.get("incumbent").is_none());
    }

    #[test]
    fn checkpoint_rejects_live_search_fields() {
        let error = serde_json::from_value::<OracleRunCombatWorkCheckpointV1>(serde_json::json!({
            "consumed_nodes": 0,
            "remaining_nodes": 0,
            "remaining_engine_steps": 0,
            "remaining_wall_ms": null,
            "quantum_count": 0,
            "restart_count": 0,
            "planner_frontier": []
        }))
        .expect_err("live frontier must not enter the durable contract");

        assert!(error
            .to_string()
            .contains("unknown field `planner_frontier`"));
    }
}
