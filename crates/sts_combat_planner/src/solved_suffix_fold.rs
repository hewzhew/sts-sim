use std::sync::Arc;
use std::time::{Duration, Instant};

use sts_core::sim::combat::{CombatPosition, CombatStepper};
use sts_core::state::core::ClientInput;

use super::{
    CombatDecisionRoot, CombatDecisionRootError, LayeredCombatSolvedSuffixIndex,
    LayeredCombatWitnessConfig, LayeredCombatWitnessCounters, LayeredCombatWitnessQuantum,
    LayeredCombatWitnessSession, LayeredCombatWitnessStatus, OracleCombatWitness,
    OracleCombatWitnessReplayError, SharedCombatActionPolicy,
};

#[derive(Clone, Copy, Debug)]
pub struct SolvedSuffixFoldConfig {
    pub search: LayeredCombatWitnessConfig,
    pub max_generation_work_per_fold: usize,
    pub max_engine_steps_per_transition: usize,
    pub wall_time_per_fold: Option<Duration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SolvedSuffixFoldError {
    EmptyCorridor,
    InvalidBoundary {
        boundary_index: usize,
        error: CombatDecisionRootError,
    },
    InvalidVerifiedSuffix {
        boundary_index: usize,
        error: OracleCombatWitnessReplayError,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SolvedSuffixFoldStatus {
    WitnessFound,
    Partial {
        predecessor_index: usize,
        search_status: LayeredCombatWitnessStatus,
    },
}

#[derive(Clone, Debug)]
pub struct SolvedSuffixFoldStepReport {
    pub predecessor_index: usize,
    pub status: LayeredCombatWitnessStatus,
    pub elapsed: Duration,
    pub counters: LayeredCombatWitnessCounters,
    pub action_count: Option<usize>,
    pub final_hp: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct SolvedSuffixFoldReport {
    pub status: SolvedSuffixFoldStatus,
    pub steps: Vec<SolvedSuffixFoldStepReport>,
    pub solved_suffix_count: usize,
    pub witness: Option<OracleCombatWitness>,
}

/// Proves a terminal suffix backwards through exact player-turn boundaries.
///
/// `corridor` is ordered from the earliest predecessor to the root of
/// `verified_suffix_inputs`. Its positions select independent subproblems;
/// they never influence action ordering inside a fold. Every predecessor is
/// accepted only after search naturally reaches an already verified exact
/// successor and the composed witness replays to a terminal win.
pub fn fold_verified_suffix_through_turn_predecessors(
    corridor: &[CombatPosition],
    verified_suffix_inputs: Vec<ClientInput>,
    config: SolvedSuffixFoldConfig,
    policy: SharedCombatActionPolicy,
    stepper: &dyn CombatStepper,
) -> Result<SolvedSuffixFoldReport, SolvedSuffixFoldError> {
    let Some(seed_position) = corridor.last() else {
        return Err(SolvedSuffixFoldError::EmptyCorridor);
    };
    let seed_index = corridor.len() - 1;
    let seed_root = CombatDecisionRoot::new(seed_position.clone()).map_err(|error| {
        SolvedSuffixFoldError::InvalidBoundary {
            boundary_index: seed_index,
            error,
        }
    })?;
    let mut suffixes = LayeredCombatSolvedSuffixIndex::default();
    suffixes
        .insert_verified_inputs(
            seed_root,
            verified_suffix_inputs,
            config.max_engine_steps_per_transition,
            stepper,
        )
        .map_err(|error| SolvedSuffixFoldError::InvalidVerifiedSuffix {
            boundary_index: seed_index,
            error,
        })?;

    let mut steps = Vec::with_capacity(seed_index);
    let mut root_witness = None;
    for predecessor_index in (0..seed_index).rev() {
        let started = Instant::now();
        let predecessor_root = CombatDecisionRoot::new(corridor[predecessor_index].clone())
            .map_err(|error| SolvedSuffixFoldError::InvalidBoundary {
                boundary_index: predecessor_index,
                error,
            })?;
        let mut session = LayeredCombatWitnessSession::with_policy_and_solved_suffixes(
            predecessor_root,
            config.search,
            policy.clone(),
            Arc::new(suffixes.clone()),
        );
        let report = session.advance(
            LayeredCombatWitnessQuantum {
                additional_generation_work: config.max_generation_work_per_fold.max(1),
                additional_engine_steps: config
                    .max_generation_work_per_fold
                    .max(1)
                    .saturating_mul(config.max_engine_steps_per_transition.max(1)),
                deadline: config
                    .wall_time_per_fold
                    .map(|allowance| Instant::now() + allowance),
            },
            stepper,
        );
        let elapsed = started.elapsed();
        let Some(witness) = report.witness else {
            let status = report.status;
            steps.push(SolvedSuffixFoldStepReport {
                predecessor_index,
                status: status.clone(),
                elapsed,
                counters: report.counters,
                action_count: None,
                final_hp: None,
            });
            return Ok(SolvedSuffixFoldReport {
                status: SolvedSuffixFoldStatus::Partial {
                    predecessor_index,
                    search_status: status,
                },
                steps,
                solved_suffix_count: suffixes.len(),
                witness: None,
            });
        };
        let inputs = witness
            .actions
            .iter()
            .map(|action| action.input.clone())
            .collect::<Vec<_>>();
        let inserted_root =
            CombatDecisionRoot::new(corridor[predecessor_index].clone()).map_err(|error| {
                SolvedSuffixFoldError::InvalidBoundary {
                    boundary_index: predecessor_index,
                    error,
                }
            })?;
        suffixes
            .insert_verified_inputs(
                inserted_root,
                inputs,
                config.max_engine_steps_per_transition,
                stepper,
            )
            .map_err(|error| SolvedSuffixFoldError::InvalidVerifiedSuffix {
                boundary_index: predecessor_index,
                error,
            })?;
        steps.push(SolvedSuffixFoldStepReport {
            predecessor_index,
            status: report.status,
            elapsed,
            counters: report.counters,
            action_count: Some(witness.actions.len()),
            final_hp: Some(witness.final_position.combat.entities.player.current_hp),
        });
        root_witness = Some(witness);
    }

    if seed_index == 0 {
        let seed_root = CombatDecisionRoot::new(seed_position.clone()).map_err(|error| {
            SolvedSuffixFoldError::InvalidBoundary {
                boundary_index: seed_index,
                error,
            }
        })?;
        let mut session = LayeredCombatWitnessSession::with_policy_and_solved_suffixes(
            seed_root,
            config.search,
            policy,
            Arc::new(suffixes.clone()),
        );
        root_witness = session
            .advance(
                LayeredCombatWitnessQuantum {
                    additional_generation_work: 1,
                    additional_engine_steps: config.max_engine_steps_per_transition.max(1),
                    deadline: config
                        .wall_time_per_fold
                        .map(|allowance| Instant::now() + allowance),
                },
                stepper,
            )
            .witness;
    }

    Ok(SolvedSuffixFoldReport {
        status: SolvedSuffixFoldStatus::WitnessFound,
        steps,
        solved_suffix_count: suffixes.len(),
        witness: root_witness,
    })
}
