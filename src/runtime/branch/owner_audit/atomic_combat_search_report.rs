use sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy;

use super::branch_status_view;
use super::{BranchStatus, TerminalOutcome};

#[derive(Clone)]
pub(super) struct AtomicCombatSearchSessionReportV2 {
    pub(super) status: AtomicCombatSearchSessionStatusV2,
    pub(super) profile_id: &'static str,
    pub(super) max_nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) potion_policy: &'static str,
    pub(super) max_potions_used: Option<u32>,
    pub(super) allowed_potion_slots: Option<u64>,
    pub(super) work_quanta: Vec<AtomicCombatSearchQuantumReportV2>,
    pub(super) action_keys: Vec<String>,
    pub(super) semantics_fingerprint: String,
    pub(super) candidate_tier: Option<String>,
    pub(super) applied: bool,
    pub(super) decision: String,
    pub(super) combat_final_hp: Option<i32>,
    pub(super) run_hp: Option<i32>,
    pub(super) potions_used: Option<u32>,
    pub(super) turns: Option<u32>,
}

#[derive(Clone)]
pub(super) struct AtomicCombatSearchQuantumReportV2 {
    pub(super) label: &'static str,
    pub(super) additional_nodes: usize,
    pub(super) soft_wall_ms: Option<u64>,
}

#[derive(Clone)]
pub(super) enum AtomicCombatSearchSessionStatusV2 {
    Failed(String),
    Advanced(String),
    Terminal(TerminalOutcome),
}

pub(super) struct AtomicCombatSearchSessionReportInputV2 {
    pub(super) status: BranchStatus,
    pub(super) profile_id: &'static str,
    pub(super) max_nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) potion_policy: CombatSearchV2PotionPolicy,
    pub(super) max_potions_used: Option<u32>,
    pub(super) allowed_potion_slots: Option<u64>,
    pub(super) work_quanta: Vec<AtomicCombatSearchQuantumReportV2>,
    pub(super) action_keys: Vec<String>,
    pub(super) semantics_fingerprint: String,
    pub(super) candidate_tier: Option<String>,
    pub(super) applied: bool,
    pub(super) decision: String,
    pub(super) combat_final_hp: Option<i32>,
    pub(super) run_hp: Option<i32>,
    pub(super) potions_used: Option<u32>,
    pub(super) turns: Option<u32>,
}

pub(super) fn atomic_combat_search_session_report(
    input: AtomicCombatSearchSessionReportInputV2,
) -> AtomicCombatSearchSessionReportV2 {
    AtomicCombatSearchSessionReportV2 {
        status: atomic_combat_search_session_status(&input.status),
        profile_id: input.profile_id,
        max_nodes: input.max_nodes,
        wall_ms: input.wall_ms,
        potion_policy: potion_policy_label(input.potion_policy),
        max_potions_used: input.max_potions_used,
        allowed_potion_slots: input.allowed_potion_slots,
        work_quanta: input.work_quanta,
        action_keys: input.action_keys,
        semantics_fingerprint: input.semantics_fingerprint,
        candidate_tier: input.candidate_tier,
        applied: input.applied,
        decision: input.decision,
        combat_final_hp: input.combat_final_hp,
        run_hp: input.run_hp,
        potions_used: input.potions_used,
        turns: input.turns,
    }
}

fn atomic_combat_search_session_status(status: &BranchStatus) -> AtomicCombatSearchSessionStatusV2 {
    match status {
        BranchStatus::CombatGap { reason, .. } => {
            AtomicCombatSearchSessionStatusV2::Failed(reason.clone())
        }
        BranchStatus::ApplyFailed(err)
        | BranchStatus::AdvanceFailed(err)
        | BranchStatus::OperationBudgetExhausted { reason: err, .. }
        | BranchStatus::BudgetGap { reason: err, .. } => {
            AtomicCombatSearchSessionStatusV2::Failed(err.clone())
        }
        BranchStatus::Terminal(TerminalOutcome::Defeat) => {
            AtomicCombatSearchSessionStatusV2::Failed("combat search ended in defeat".to_string())
        }
        BranchStatus::Terminal(result) => AtomicCombatSearchSessionStatusV2::Terminal(*result),
        _ => AtomicCombatSearchSessionStatusV2::Advanced(
            branch_status_view::status_boundary(status).to_string(),
        ),
    }
}

fn potion_policy_label(policy: CombatSearchV2PotionPolicy) -> &'static str {
    match policy {
        CombatSearchV2PotionPolicy::Never => "never",
        CombatSearchV2PotionPolicy::All => "all",
        CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_report_has_one_search_identity_and_incremental_work() {
        let report = atomic_combat_search_session_report(AtomicCombatSearchSessionReportInputV2 {
            status: BranchStatus::AwaitingAuto {
                boundary: "PostCombat".to_string(),
                reason: "accepted".to_string(),
            },
            profile_id: "canonical_combat_session",
            max_nodes: 30,
            wall_ms: 300,
            potion_policy: CombatSearchV2PotionPolicy::SemanticBudgeted,
            max_potions_used: Some(2),
            allowed_potion_slots: Some(3),
            work_quanta: vec![
                AtomicCombatSearchQuantumReportV2 {
                    label: "initial",
                    additional_nodes: 10,
                    soft_wall_ms: Some(100),
                },
                AtomicCombatSearchQuantumReportV2 {
                    label: "refine",
                    additional_nodes: 20,
                    soft_wall_ms: Some(200),
                },
            ],
            action_keys: vec!["selected-action".to_string()],
            semantics_fingerprint: "engine".to_string(),
            candidate_tier: Some("reserve_compliant_complete_win".to_string()),
            applied: true,
            decision: "accepted_clean_candidate".to_string(),
            combat_final_hp: Some(48),
            run_hp: Some(48),
            potions_used: Some(1),
            turns: Some(5),
        });

        assert_eq!(report.profile_id, "canonical_combat_session");
        assert_eq!(report.work_quanta.len(), 2);
        assert_eq!(report.action_keys, vec!["selected-action"]);
        assert_eq!(report.allowed_potion_slots, Some(3));
        assert!(report.applied);
    }
}
