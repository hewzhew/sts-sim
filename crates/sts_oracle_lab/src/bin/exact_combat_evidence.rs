//! Shared exact evidence classification for bounded combat-successor audits.
//!
//! Callers may establish an immediate terminal result or a verified witness
//! directly. Non-terminal positions are searched with the same bounded exact
//! graph search. A bounded miss remains `BudgetUnknown`; only gap-free,
//! depth-complete frontier exhaustion is an exact refutation.

use serde::Serialize;
use sts_combat_planner::{
    CombatDecisionRoot, LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessQuantum,
    LocalTurnGraphWitnessReport, LocalTurnGraphWitnessSession, LocalTurnGraphWitnessStatus,
    OracleCombatWitnessSatisfaction, TurnOptionGeneratorConfig,
};
use sts_simulator::eval::run_control::{
    existing_combat_knowledge_policy_v1, existing_combat_rollout_lookahead_v1,
};
use sts_simulator::sim::combat::{CombatPosition, EngineCombatStepper};
use sts_simulator::state::core::ClientInput;

pub(crate) struct ExactCombatEvaluation {
    pub(crate) evidence: ExactCombatEvidence,
    pub(crate) witness_actions: Option<Vec<ClientInput>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ExactCombatEvidence {
    ExactWin {
        source: &'static str,
        final_hp: i32,
        suffix_action_count: usize,
        search_cost: Option<ExactCombatSearchCost>,
    },
    ExactRefutation {
        source: &'static str,
        search_cost: ExactCombatSearchCost,
    },
    ExactTerminalNonWin {
        boundary: String,
    },
    BudgetUnknown {
        status: String,
        search_cost: ExactCombatSearchCost,
        deepest_player_turn: u32,
        gap_count: usize,
        depth_limited_successors: usize,
    },
}

impl ExactCombatEvidence {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::ExactWin { .. } => "exact_win",
            Self::ExactRefutation { .. } => "exact_refutation",
            Self::ExactTerminalNonWin { .. } => "exact_terminal_non_win",
            Self::BudgetUnknown { .. } => "budget_unknown",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ExactCombatSearchCost {
    generation_work: usize,
    lookahead_work: usize,
    applied_action_transitions: usize,
    engine_steps: usize,
    exact_nodes: usize,
    exact_edges: usize,
}

pub(crate) fn known_exact_win(
    source: &'static str,
    final_hp: i32,
    suffix_action_count: usize,
) -> ExactCombatEvidence {
    ExactCombatEvidence::ExactWin {
        source,
        final_hp,
        suffix_action_count,
        search_cost: None,
    }
}

pub(crate) fn exact_terminal_non_win(boundary: impl Into<String>) -> ExactCombatEvidence {
    ExactCombatEvidence::ExactTerminalNonWin {
        boundary: boundary.into(),
    }
}

pub(crate) fn evaluate_nonterminal_position(
    position: &CombatPosition,
    solve_work: usize,
    max_engine_steps_per_transition: usize,
) -> Result<ExactCombatEvaluation, String> {
    let root = CombatDecisionRoot::new(position.clone())
        .map_err(|error| format!("invalid successor root: {error:?}"))?;
    let search_config = LocalTurnGraphWitnessConfig {
        generator: TurnOptionGeneratorConfig {
            max_engine_steps_per_transition,
            ..TurnOptionGeneratorConfig::default()
        },
        generation_quantum_work: 4,
        backed_generation_quantum_work: 256,
        initial_expansion_work: 64,
        root_initial_expansion_work: 2_048,
        lookahead_max_evaluations: solve_work.saturating_div(24).max(1),
        lookahead_work_per_evaluation: 24,
        max_turn_depth: 32,
        satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
    };
    let mut session = LocalTurnGraphWitnessSession::with_policy_and_lookahead(
        root,
        search_config,
        existing_combat_knowledge_policy_v1(),
        existing_combat_rollout_lookahead_v1(),
    );
    let report = session.advance(
        LocalTurnGraphWitnessQuantum {
            additional_selections: solve_work.saturating_mul(8),
            additional_generation_work: solve_work,
            additional_engine_steps: solve_work.saturating_mul(max_engine_steps_per_transition),
            deadline: None,
        },
        &EngineCombatStepper,
    );
    if let Some(witness) = report.witness.as_ref() {
        return Ok(ExactCombatEvaluation {
            evidence: ExactCombatEvidence::ExactWin {
                source: "bounded_exact_search",
                final_hp: witness.final_position.combat.entities.player.current_hp,
                suffix_action_count: witness.actions.len(),
                search_cost: Some(search_cost(&report)),
            },
            witness_actions: Some(
                witness
                    .actions
                    .iter()
                    .map(|action| action.input.clone())
                    .collect(),
            ),
        });
    }
    if matches!(
        report.status,
        LocalTurnGraphWitnessStatus::FrontierExhausted
    ) && report.generation_gaps.is_empty()
        && report.counters.depth_limited_successors == 0
    {
        return Ok(ExactCombatEvaluation {
            evidence: ExactCombatEvidence::ExactRefutation {
                source: "gap_free_frontier_exhaustion",
                search_cost: search_cost(&report),
            },
            witness_actions: None,
        });
    }
    let progress = session.progress_snapshot();
    Ok(ExactCombatEvaluation {
        evidence: ExactCombatEvidence::BudgetUnknown {
            status: format!("{:?}", report.status),
            search_cost: search_cost(&report),
            deepest_player_turn: progress.max_player_turn,
            gap_count: report.generation_gaps.len(),
            depth_limited_successors: report.counters.depth_limited_successors,
        },
        witness_actions: None,
    })
}

fn search_cost(report: &LocalTurnGraphWitnessReport) -> ExactCombatSearchCost {
    ExactCombatSearchCost {
        generation_work: report.counters.generation_work,
        lookahead_work: report.counters.lookahead_work,
        applied_action_transitions: report.counters.applied_action_transitions,
        engine_steps: report.counters.engine_steps,
        exact_nodes: report.counters.exact_nodes,
        exact_edges: report.counters.exact_edges,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        exact_terminal_non_win, known_exact_win, ExactCombatEvidence, ExactCombatSearchCost,
    };

    #[test]
    fn exact_evidence_kinds_do_not_collapse_unknown_with_terminal_results() {
        let cost = ExactCombatSearchCost {
            generation_work: 1,
            lookahead_work: 2,
            applied_action_transitions: 3,
            engine_steps: 4,
            exact_nodes: 5,
            exact_edges: 6,
        };
        assert_eq!(known_exact_win("verified", 17, 3).kind(), "exact_win");
        assert_eq!(
            ExactCombatEvidence::ExactRefutation {
                source: "exhausted",
                search_cost: cost.clone(),
            }
            .kind(),
            "exact_refutation"
        );
        assert_eq!(
            exact_terminal_non_win("Loss").kind(),
            "exact_terminal_non_win"
        );
        assert_eq!(
            ExactCombatEvidence::BudgetUnknown {
                status: "Partial".to_string(),
                search_cost: cost,
                deepest_player_turn: 2,
                gap_count: 0,
                depth_limited_successors: 0,
            }
            .kind(),
            "budget_unknown"
        );
    }
}
