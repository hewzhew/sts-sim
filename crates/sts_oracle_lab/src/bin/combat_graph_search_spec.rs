//! Effective planner settings and one bounded allowance for local graph search.
//!
//! Keeping this typed value beside every report prevents a command line from
//! becoming an irrecoverable experiment description. The value may construct
//! planner inputs, but it never owns a session or advances search.

use std::time::{Duration, Instant};

use serde::Serialize;
use sts_combat_planner::{
    CombatGuideLaneId, LocalTurnGraphGuideServiceBias, LocalTurnGraphWitnessConfig,
    LocalTurnGraphWitnessQuantum, OracleCombatWitnessSatisfaction, TurnOptionGeneratorConfig,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct LocalGraphGuideServiceBiasSpec {
    lane: u32,
    extra_services_per_cycle: usize,
}

impl LocalGraphGuideServiceBiasSpec {
    pub(super) fn new(lane: u32, extra_services_per_cycle: usize) -> Self {
        Self {
            lane,
            extra_services_per_cycle,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct LocalGraphPlannerSettings {
    max_engine_steps_per_transition: usize,
    uniform_exploration_ppm: u32,
    allow_potion_expenditure: bool,
    allow_potion_discard: bool,
    generation_quantum_work: usize,
    backed_generation_quantum_work: usize,
    guide_service_bias: Option<LocalGraphGuideServiceBiasSpec>,
    initial_expansion_work: usize,
    root_initial_expansion_work: usize,
    lookahead_max_evaluations: usize,
    lookahead_work_per_evaluation: usize,
    max_turn_depth: usize,
    max_potions_used: Option<u32>,
    allowed_potion_slots: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct LocalGraphSearchAllowance {
    max_selections: usize,
    max_generation_work: usize,
    max_engine_steps: usize,
    wall_ms: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct LocalGraphSearchSpec {
    planner: LocalGraphPlannerSettings,
    allowance: LocalGraphSearchAllowance,
}

impl LocalGraphSearchSpec {
    pub(super) fn from_controls(
        max_generation_work: usize,
        max_selections: usize,
        wall_ms: u64,
        max_engine_steps_per_transition: usize,
        uniform_exploration_ppm: u32,
        generation_quantum_work: usize,
        max_turn_depth: usize,
        max_potions_used: Option<u32>,
        allow_potion_discard: bool,
        allowed_potion_slots: Option<u64>,
        initial_expansion_work: Option<usize>,
        guide_service_bias: Option<LocalGraphGuideServiceBiasSpec>,
    ) -> Self {
        let defaults = LocalTurnGraphWitnessConfig::default();
        let lookahead_work_per_evaluation = defaults.lookahead_work_per_evaluation;
        Self {
            planner: LocalGraphPlannerSettings {
                max_engine_steps_per_transition,
                uniform_exploration_ppm,
                allow_potion_expenditure: max_potions_used != Some(0),
                allow_potion_discard,
                generation_quantum_work,
                backed_generation_quantum_work: defaults.backed_generation_quantum_work,
                guide_service_bias,
                initial_expansion_work: initial_expansion_work
                    .unwrap_or(defaults.initial_expansion_work),
                root_initial_expansion_work: defaults.root_initial_expansion_work,
                lookahead_max_evaluations: max_generation_work
                    .saturating_div(lookahead_work_per_evaluation)
                    .max(1),
                lookahead_work_per_evaluation,
                max_turn_depth,
                max_potions_used,
                allowed_potion_slots,
            },
            allowance: LocalGraphSearchAllowance {
                max_selections,
                max_generation_work,
                max_engine_steps: max_generation_work
                    .saturating_mul(max_engine_steps_per_transition),
                wall_ms,
            },
        }
    }

    pub(super) fn planner_config(
        self,
        satisfaction: OracleCombatWitnessSatisfaction,
    ) -> LocalTurnGraphWitnessConfig {
        LocalTurnGraphWitnessConfig {
            generator: TurnOptionGeneratorConfig {
                max_engine_steps_per_transition: self.planner.max_engine_steps_per_transition,
                uniform_exploration_ppm: self.planner.uniform_exploration_ppm,
                allow_potion_expenditure: self.planner.allow_potion_expenditure,
                allow_potion_discard: self.planner.allow_potion_discard,
                allowed_potion_slots: self.planner.allowed_potion_slots,
            },
            generation_quantum_work: self.planner.generation_quantum_work,
            backed_generation_quantum_work: self.planner.backed_generation_quantum_work,
            guide_service_bias: self.planner.guide_service_bias.map(|bias| {
                LocalTurnGraphGuideServiceBias {
                    lane: CombatGuideLaneId::new(bias.lane),
                    extra_services_per_cycle: bias.extra_services_per_cycle,
                }
            }),
            initial_expansion_work: self.planner.initial_expansion_work,
            root_initial_expansion_work: self.planner.root_initial_expansion_work,
            lookahead_max_evaluations: self.planner.lookahead_max_evaluations,
            lookahead_work_per_evaluation: self.planner.lookahead_work_per_evaluation,
            max_turn_depth: self.planner.max_turn_depth,
            satisfaction,
            require_no_unrecovered_stolen_gold: false,
            max_potions_used: self.planner.max_potions_used,
        }
    }

    pub(super) fn quantum(self) -> LocalTurnGraphWitnessQuantum {
        LocalTurnGraphWitnessQuantum {
            additional_selections: self.allowance.max_selections,
            additional_generation_work: self.allowance.max_generation_work,
            additional_engine_steps: self.allowance.max_engine_steps,
            deadline: Some(Instant::now() + Duration::from_millis(self.allowance.wall_ms)),
        }
    }
}

#[cfg(test)]
#[path = "combat_graph_search_spec_tests.rs"]
mod tests;
