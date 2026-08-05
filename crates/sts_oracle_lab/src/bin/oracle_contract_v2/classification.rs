use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sts_combat_planner::{
    LocalTurnGraphTerminalOutcomeSnapshotV1, LocalTurnGraphWitnessInterruption,
    LocalTurnGraphWitnessReport, LocalTurnGraphWitnessStatus, OracleCombatWitness,
    OracleCombatWitnessDiscoverySource,
};

use super::CombatContractRequestV2;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CombatContractClassificationV2 {
    Passed,
    WitnessBelowFinalHp,
    WitnessSpentTooManyPotions,
    WitnessLeftStolenGold,
    SuffixNotProposed,
    SuffixProposalsRejected,
    SuffixProposalsNotVerified,
    FrontierExhausted,
    MechanicsGap,
    ReplayMismatch,
    BudgetUnknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CombatContractSearchStatusV2 {
    WitnessFound,
    SelectionBudget,
    GenerationWorkBudget,
    EngineStepBudget,
    Deadline,
    FrontierExhausted,
    MechanicsGap,
    ReplayMismatch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct CombatContractResultV2 {
    pub(super) schema_name: String,
    pub(super) schema_version: u32,
    pub(super) artifact: PathBuf,
    pub(super) classification: CombatContractClassificationV2,
    pub(super) contract_passed: bool,
    pub(super) search_status: CombatContractSearchStatusV2,
    pub(super) candidate_witness: bool,
    pub(super) contract_witness: bool,
    pub(super) discovery_source: Option<OracleCombatWitnessDiscoverySource>,
    pub(super) final_hp: Option<i32>,
    pub(super) potions_used: Option<u32>,
    pub(super) unrecovered_stolen_gold: Option<i32>,
    pub(super) action_count: Option<usize>,
    pub(super) witness_actions: Option<PathBuf>,
    pub(super) suffix_proposals: usize,
    pub(super) suffix_rejections: usize,
    pub(super) suffix_witnesses: usize,
    pub(super) generation_work: usize,
    pub(super) elapsed_ms: u128,
}

pub(super) struct CombatContractAssessmentV2 {
    pub(super) result: CombatContractResultV2,
    pub(super) selected_witness_index: Option<usize>,
}

pub(super) fn classify_contract(
    request: &CombatContractRequestV2,
    report: &LocalTurnGraphWitnessReport,
    witness_frontier: &[OracleCombatWitness],
    artifact: &Path,
    elapsed: Duration,
) -> CombatContractAssessmentV2 {
    let (contract_index, candidate_index) = evidence_indexes(request, report, witness_frontier);
    let outcome = candidate_index.and_then(|index| report.witness_frontier.get(index));
    let witness = candidate_index.and_then(|index| witness_frontier.get(index));
    let final_hp = outcome.map(|outcome| outcome.final_hp);
    let potions_used = outcome.map(|outcome| outcome.potion_expenditures);
    let unrecovered_stolen_gold = outcome.map(|outcome| outcome.unrecovered_stolen_gold);
    let classification = if contract_index.is_some() {
        CombatContractClassificationV2::Passed
    } else if let Some(outcome) = outcome {
        if request
            .min_final_hp
            .is_some_and(|minimum| outcome.final_hp < minimum)
        {
            CombatContractClassificationV2::WitnessBelowFinalHp
        } else if outcome.potion_expenditures > request.max_potions_used {
            CombatContractClassificationV2::WitnessSpentTooManyPotions
        } else if request.require_recovered_stolen_gold && outcome.unrecovered_stolen_gold > 0 {
            CombatContractClassificationV2::WitnessLeftStolenGold
        } else {
            CombatContractClassificationV2::BudgetUnknown
        }
    } else {
        match report.status {
            LocalTurnGraphWitnessStatus::MechanicsGap => {
                CombatContractClassificationV2::MechanicsGap
            }
            LocalTurnGraphWitnessStatus::ReplayMismatch(_) => {
                CombatContractClassificationV2::ReplayMismatch
            }
            _ if report.counters.lookahead_suffix_proposals == 0 => {
                CombatContractClassificationV2::SuffixNotProposed
            }
            _ if report.counters.lookahead_suffix_proposal_rejections
                == report.counters.lookahead_suffix_proposals =>
            {
                CombatContractClassificationV2::SuffixProposalsRejected
            }
            _ if report.counters.lookahead_suffix_witnesses == 0 => {
                CombatContractClassificationV2::SuffixProposalsNotVerified
            }
            LocalTurnGraphWitnessStatus::FrontierExhausted => {
                CombatContractClassificationV2::FrontierExhausted
            }
            _ => CombatContractClassificationV2::BudgetUnknown,
        }
    };
    CombatContractAssessmentV2 {
        result: CombatContractResultV2 {
            schema_name: "OracleCombatContractResultV2".to_owned(),
            schema_version: 2,
            artifact: artifact.to_path_buf(),
            classification,
            contract_passed: classification == CombatContractClassificationV2::Passed,
            search_status: search_status(&report.status),
            candidate_witness: candidate_index.is_some(),
            contract_witness: contract_index.is_some(),
            discovery_source: witness.map(|witness| witness.discovery_source),
            final_hp,
            potions_used,
            unrecovered_stolen_gold,
            action_count: outcome.map(|outcome| outcome.action_count),
            witness_actions: None,
            suffix_proposals: report.counters.lookahead_suffix_proposals,
            suffix_rejections: report.counters.lookahead_suffix_proposal_rejections,
            suffix_witnesses: report.counters.lookahead_suffix_witnesses,
            generation_work: report
                .counters
                .generation_work
                .saturating_add(report.counters.lookahead_work),
            elapsed_ms: elapsed.as_millis(),
        },
        selected_witness_index: candidate_index,
    }
}

fn evidence_indexes(
    request: &CombatContractRequestV2,
    report: &LocalTurnGraphWitnessReport,
    witness_frontier: &[OracleCombatWitness],
) -> (Option<usize>, Option<usize>) {
    let paired_len = report.witness_frontier.len().min(witness_frontier.len());
    let contract = report
        .witness_frontier
        .iter()
        .take(paired_len)
        .position(|outcome| outcome_satisfies_contract(request, outcome));
    let candidate = contract.or_else(|| {
        report
            .witness_frontier
            .iter()
            .take(paired_len)
            .position(|outcome| outcome.selected_by_local_hp_view)
            .or((paired_len > 0).then_some(0))
    });
    (contract, candidate)
}

fn outcome_satisfies_contract(
    request: &CombatContractRequestV2,
    outcome: &LocalTurnGraphTerminalOutcomeSnapshotV1,
) -> bool {
    request
        .min_final_hp
        .is_none_or(|minimum| outcome.final_hp >= minimum)
        && outcome.potion_expenditures <= request.max_potions_used
        && (!request.require_recovered_stolen_gold || outcome.unrecovered_stolen_gold == 0)
}

fn search_status(status: &LocalTurnGraphWitnessStatus) -> CombatContractSearchStatusV2 {
    match status {
        LocalTurnGraphWitnessStatus::WitnessFound => CombatContractSearchStatusV2::WitnessFound,
        LocalTurnGraphWitnessStatus::Partial(interruption) => match interruption {
            LocalTurnGraphWitnessInterruption::SelectionBudget => {
                CombatContractSearchStatusV2::SelectionBudget
            }
            LocalTurnGraphWitnessInterruption::GenerationWorkBudget => {
                CombatContractSearchStatusV2::GenerationWorkBudget
            }
            LocalTurnGraphWitnessInterruption::EngineStepBudget => {
                CombatContractSearchStatusV2::EngineStepBudget
            }
            LocalTurnGraphWitnessInterruption::Deadline => CombatContractSearchStatusV2::Deadline,
        },
        LocalTurnGraphWitnessStatus::FrontierExhausted => {
            CombatContractSearchStatusV2::FrontierExhausted
        }
        LocalTurnGraphWitnessStatus::MechanicsGap => CombatContractSearchStatusV2::MechanicsGap,
        LocalTurnGraphWitnessStatus::ReplayMismatch(_) => {
            CombatContractSearchStatusV2::ReplayMismatch
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_combat_planner::LocalTurnGraphWitnessCounters;
    use sts_oracle_runtime::sim::combat::CombatPosition;
    use sts_oracle_runtime::state::core::{EngineState, RunResult};
    use sts_oracle_runtime::test_support::blank_test_combat;

    fn request() -> CombatContractRequestV2 {
        CombatContractRequestV2 {
            case_id: "root".to_owned(),
            case: PathBuf::from("case.json"),
            min_final_hp: Some(66),
            max_potions_used: 0,
            require_recovered_stolen_gold: true,
            generation_work: 4_096,
            wall_ms: 2_000,
        }
    }

    fn report(counters: LocalTurnGraphWitnessCounters) -> LocalTurnGraphWitnessReport {
        LocalTurnGraphWitnessReport {
            status: LocalTurnGraphWitnessStatus::Partial(
                LocalTurnGraphWitnessInterruption::GenerationWorkBudget,
            ),
            counters,
            performance_timing: Default::default(),
            root_visits: 0,
            root_generated_options: 0,
            root_children: 0,
            generation_gaps: Vec::new(),
            witness: None,
            witness_frontier: Vec::new(),
        }
    }

    fn terminal_outcome(
        final_hp: i32,
        unrecovered_stolen_gold: i32,
        selected: bool,
    ) -> LocalTurnGraphTerminalOutcomeSnapshotV1 {
        LocalTurnGraphTerminalOutcomeSnapshotV1 {
            selected_by_local_hp_view: selected,
            final_hp,
            final_max_hp: 80,
            recoverable_gold_delta: 0,
            recoverable_stolen_gold: 0,
            unrecovered_stolen_gold,
            ritual_dagger_value: 0,
            genetic_algorithm_value: 0,
            external_burden_count: 0,
            potion_expenditures: 0,
            action_count: 1,
            negative_log_policy: 1.0,
        }
    }

    fn witness(final_hp: i32) -> OracleCombatWitness {
        let mut combat = blank_test_combat();
        combat.entities.player.current_hp = final_hp;
        OracleCombatWitness {
            actions: Vec::new(),
            final_position: CombatPosition::new(EngineState::GameOver(RunResult::Victory), combat),
            negative_log_policy: 1.0,
            replay_engine_steps: 1,
            discovery_source: OracleCombatWitnessDiscoverySource::LookaheadProposal,
        }
    }

    #[test]
    fn compact_classification_distinguishes_missing_suffix_service() {
        let result = classify_contract(
            &request(),
            &report(LocalTurnGraphWitnessCounters::default()),
            &[],
            Path::new("manifest.json"),
            Duration::ZERO,
        )
        .result;
        assert_eq!(
            result.classification,
            CombatContractClassificationV2::SuffixNotProposed
        );
        assert!(!result.contract_passed);
    }

    #[test]
    fn compact_classification_distinguishes_rejected_suffixes() {
        let result = classify_contract(
            &request(),
            &report(LocalTurnGraphWitnessCounters {
                lookahead_suffix_proposals: 2,
                lookahead_suffix_proposal_rejections: 2,
                ..LocalTurnGraphWitnessCounters::default()
            }),
            &[],
            Path::new("manifest.json"),
            Duration::ZERO,
        )
        .result;
        assert_eq!(
            result.classification,
            CombatContractClassificationV2::SuffixProposalsRejected
        );
    }

    #[test]
    fn compact_result_stays_below_one_kibibyte() {
        let result = classify_contract(
            &request(),
            &report(LocalTurnGraphWitnessCounters::default()),
            &[],
            Path::new("manifest.json"),
            Duration::ZERO,
        )
        .result;
        assert!(serde_json::to_vec(&result).unwrap().len() < 1_024);
    }

    #[test]
    fn contract_selects_a_satisfying_frontier_witness_not_the_hp_first_candidate() {
        let hp_first = witness(71);
        let contract = witness(66);
        let mut report = report(LocalTurnGraphWitnessCounters::default());
        report.status = LocalTurnGraphWitnessStatus::WitnessFound;
        report.witness = Some(hp_first.clone());
        report.witness_frontier = vec![
            terminal_outcome(71, 30, true),
            terminal_outcome(66, 0, false),
        ];

        let assessment = classify_contract(
            &request(),
            &report,
            &[hp_first, contract],
            Path::new("manifest.json"),
            Duration::ZERO,
        );

        assert_eq!(assessment.selected_witness_index, Some(1));
        let result = assessment.result;
        assert_eq!(
            result.classification,
            CombatContractClassificationV2::Passed
        );
        assert!(result.contract_witness);
        assert_eq!(result.final_hp, Some(66));
        assert_eq!(result.unrecovered_stolen_gold, Some(0));
    }
}
