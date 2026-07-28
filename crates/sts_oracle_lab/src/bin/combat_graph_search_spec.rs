//! Effective planner settings and one bounded allowance for local graph search.
//!
//! Keeping this typed value beside every report prevents a command line from
//! becoming an irrecoverable experiment description. The value may construct
//! planner inputs, but it never owns a session or advances search.

use std::time::{Duration, Instant};

use serde::Serialize;
use sts_combat_planner::{
    LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessQuantum, OracleCombatWitnessSatisfaction,
    TurnOptionGeneratorConfig,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct LocalGraphPlannerSettings {
    max_engine_steps_per_transition: usize,
    allow_potion_expenditure: bool,
    generation_quantum_work: usize,
    backed_generation_quantum_work: usize,
    initial_expansion_work: usize,
    root_initial_expansion_work: usize,
    lookahead_max_evaluations: usize,
    lookahead_work_per_evaluation: usize,
    max_turn_depth: usize,
    max_potions_used: Option<u32>,
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
        generation_quantum_work: usize,
        max_turn_depth: usize,
        max_potions_used: Option<u32>,
    ) -> Self {
        let defaults = LocalTurnGraphWitnessConfig::default();
        let lookahead_work_per_evaluation = defaults.lookahead_work_per_evaluation;
        Self {
            planner: LocalGraphPlannerSettings {
                max_engine_steps_per_transition,
                allow_potion_expenditure: max_potions_used != Some(0),
                generation_quantum_work,
                backed_generation_quantum_work: defaults.backed_generation_quantum_work,
                initial_expansion_work: defaults.initial_expansion_work,
                root_initial_expansion_work: defaults.root_initial_expansion_work,
                lookahead_max_evaluations: max_generation_work
                    .saturating_div(lookahead_work_per_evaluation)
                    .max(1),
                lookahead_work_per_evaluation,
                max_turn_depth,
                max_potions_used,
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
                allow_potion_expenditure: self.planner.allow_potion_expenditure,
                ..TurnOptionGeneratorConfig::default()
            },
            generation_quantum_work: self.planner.generation_quantum_work,
            backed_generation_quantum_work: self.planner.backed_generation_quantum_work,
            initial_expansion_work: self.planner.initial_expansion_work,
            root_initial_expansion_work: self.planner.root_initial_expansion_work,
            lookahead_max_evaluations: self.planner.lookahead_max_evaluations,
            lookahead_work_per_evaluation: self.planner.lookahead_work_per_evaluation,
            max_turn_depth: self.planner.max_turn_depth,
            satisfaction,
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
mod tests {
    use super::*;

    #[test]
    fn spec_serializes_every_effective_setting_and_allowance() {
        let spec = LocalGraphSearchSpec::from_controls(240, 80, 15, 7, 3, 9, Some(0));
        let value = serde_json::to_value(spec).expect("serialize search spec");

        assert_eq!(value["planner"]["max_engine_steps_per_transition"], 7);
        assert_eq!(value["planner"]["allow_potion_expenditure"], false);
        assert_eq!(value["planner"]["generation_quantum_work"], 3);
        assert_eq!(value["planner"]["lookahead_max_evaluations"], 10);
        assert_eq!(value["planner"]["max_turn_depth"], 9);
        assert_eq!(value["planner"]["max_potions_used"], 0);
        assert_eq!(value["allowance"]["max_selections"], 80);
        assert_eq!(value["allowance"]["max_generation_work"], 240);
        assert_eq!(value["allowance"]["max_engine_steps"], 1_680);
        assert_eq!(value["allowance"]["wall_ms"], 15);
    }

    #[test]
    fn planner_config_and_quantum_are_built_from_the_reported_spec() {
        let spec = LocalGraphSearchSpec::from_controls(240, 80, 15, 7, 3, 9, Some(2));
        let config = spec.planner_config(OracleCombatWitnessSatisfaction::HpLossAtMost(4));
        let quantum = spec.quantum();

        assert_eq!(config.generator.max_engine_steps_per_transition, 7);
        assert!(config.generator.allow_potion_expenditure);
        assert_eq!(config.generation_quantum_work, 3);
        assert_eq!(config.lookahead_max_evaluations, 10);
        assert_eq!(config.max_turn_depth, 9);
        assert_eq!(config.max_potions_used, Some(2));
        assert_eq!(quantum.additional_selections, 80);
        assert_eq!(quantum.additional_generation_work, 240);
        assert_eq!(quantum.additional_engine_steps, 1_680);
        assert!(quantum.deadline.is_some());
    }
}
