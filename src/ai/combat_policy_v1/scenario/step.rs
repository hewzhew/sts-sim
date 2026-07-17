use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::Serialize;

use crate::engine::core::is_smoke_escape_stable_boundary;
use crate::sim::combat::{apply_combat_input_to_stable, CombatStepLimits, CombatTerminal};

use super::boundary::policy_observation_envelope;
use super::group::{group_combat_scenarios_v1, CombatScenarioGroupV1};
use super::hash::stable_hash;
use super::types::{
    CombatPolicyObservationEnvelopeV1, CombatPublicActionV1, CombatScenarioParticleV1,
    CombatScenarioPolicyErrorV1,
};

const COMBAT_POLICY_HISTORY_TRANSITION_SCHEMA_NAME: &str = "CombatPolicyPublicHistoryTransitionV1";
const COMBAT_POLICY_HISTORY_TRANSITION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatScenarioStepViewV1 {
    pub scenario_count: usize,
    pub continuing_scenario_count: usize,
    pub next_information_set_count: usize,
    pub win_count: usize,
    pub loss_count: usize,
    pub engine_steps: usize,
    pub escape_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatScenarioTerminalV1 {
    Win,
    Loss,
    Escape,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatScenarioStepFailureV1 {
    Truncated {
        engine_steps: usize,
        timed_out: bool,
    },
    PublicBoundary {
        engine_steps: usize,
    },
}

impl CombatScenarioStepFailureV1 {
    pub fn engine_steps(self) -> usize {
        match self {
            Self::Truncated { engine_steps, .. } | Self::PublicBoundary { engine_steps } => {
                engine_steps
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatScenarioStepErrorV1 {
    pub failure: CombatScenarioStepFailureV1,
    exact: CombatScenarioPolicyErrorV1,
}

impl CombatScenarioStepErrorV1 {
    #[cfg(test)]
    pub(crate) fn exact_error(&self) -> &CombatScenarioPolicyErrorV1 {
        &self.exact
    }
}

impl fmt::Display for CombatScenarioStepErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.failure {
            CombatScenarioStepFailureV1::Truncated {
                engine_steps,
                timed_out,
            } => write!(formatter, "combat scenario step stopped after {engine_steps} engine steps (timed_out={timed_out})"),
            CombatScenarioStepFailureV1::PublicBoundary { engine_steps } => write!(formatter, "combat scenario step failed to form a public boundary after {engine_steps} engine steps"),
        }
    }
}

impl Error for CombatScenarioStepErrorV1 {}

pub struct CombatScenarioStepResultV1 {
    pub view: CombatScenarioStepViewV1,
    pub next_groups: Vec<CombatScenarioGroupV1>,
    pub terminal_outcomes: Vec<CombatScenarioTerminalOutcomeV1>,
}

pub struct CombatScenarioTerminalOutcomeV1 {
    pub scenario_id: String,
    pub terminal: CombatScenarioTerminalV1,
    pub final_hp: i32,
    pub player_block: i32,
    pub enemy_effective_hp: i32,
    pub turn_count: u32,
    pub cards_played: u32,
}

pub fn step_combat_scenario_group_v1(
    group: &CombatScenarioGroupV1,
    action: &CombatPublicActionV1,
    limits: CombatStepLimits,
) -> Result<CombatScenarioStepResultV1, CombatScenarioStepErrorV1> {
    let binding = group
        .bind_action(action)
        .map_err(|exact| scenario_step_error(exact, 0))?;
    let exact_inputs = binding.exact_inputs.into_iter().collect::<BTreeMap<_, _>>();
    let mut next_particles = Vec::new();
    let mut win_count = 0usize;
    let mut loss_count = 0usize;
    let mut escape_count = 0usize;
    let mut engine_steps = 0usize;
    let mut terminal_outcomes = Vec::new();

    for world in &group.worlds {
        let remaining_engine_steps = limits.max_engine_steps.saturating_sub(engine_steps);
        if remaining_engine_steps == 0 {
            return Err(scenario_step_error(
                CombatScenarioPolicyErrorV1::StepTruncated {
                    scenario_id: world.scenario_id().to_string(),
                    engine_steps,
                    timed_out: false,
                },
                engine_steps,
            ));
        }
        let exact_input = exact_inputs
            .get(world.scenario_id())
            .cloned()
            .ok_or_else(|| {
                scenario_step_error(
                    CombatScenarioPolicyErrorV1::MissingExactBinding {
                        action: format!("{action:?}"),
                    },
                    engine_steps,
                )
            })?;
        let stepped = apply_combat_input_to_stable(
            &world.position,
            exact_input,
            CombatStepLimits {
                max_engine_steps: remaining_engine_steps,
                deadline: limits.deadline,
            },
        );
        engine_steps = engine_steps.saturating_add(stepped.engine_steps);

        if stepped.truncated {
            return Err(scenario_step_error(
                CombatScenarioPolicyErrorV1::StepTruncated {
                    scenario_id: world.scenario_id().to_string(),
                    engine_steps,
                    timed_out: stepped.timed_out,
                },
                engine_steps,
            ));
        }

        match stepped.terminal {
            CombatTerminal::Win => {
                win_count = win_count.saturating_add(1);
                terminal_outcomes.push(terminal_outcome(
                    world.scenario_id(),
                    CombatScenarioTerminalV1::Win,
                    &stepped,
                ));
            }
            CombatTerminal::Loss => {
                loss_count = loss_count.saturating_add(1);
                terminal_outcomes.push(terminal_outcome(
                    world.scenario_id(),
                    CombatScenarioTerminalV1::Loss,
                    &stepped,
                ));
            }
            CombatTerminal::Unresolved
                if is_smoke_escape_stable_boundary(
                    &stepped.position.engine,
                    &stepped.position.combat,
                ) =>
            {
                escape_count = escape_count.saturating_add(1);
                terminal_outcomes.push(terminal_outcome(
                    world.scenario_id(),
                    CombatScenarioTerminalV1::Escape,
                    &stepped,
                ));
            }
            CombatTerminal::Unresolved => {
                let boundary = policy_observation_envelope(world.scenario_id(), &stepped.position)
                    .map_err(|exact| scenario_step_error(exact, engine_steps))?;
                let history_id = stable_hash(&PublicHistoryTransitionV1 {
                    schema_name: COMBAT_POLICY_HISTORY_TRANSITION_SCHEMA_NAME,
                    schema_version: COMBAT_POLICY_HISTORY_TRANSITION_SCHEMA_VERSION,
                    previous_history_id: world.public_history_id(),
                    action,
                    boundary: &boundary,
                });
                next_particles.push(CombatScenarioParticleV1::from_public_history(
                    world.scenario_id().to_string(),
                    history_id,
                    stepped.position,
                ));
            }
        }
    }

    let continuing_scenario_count = next_particles.len();
    let next_groups = if next_particles.is_empty() {
        Vec::new()
    } else {
        group_combat_scenarios_v1(next_particles)
            .map_err(|exact| scenario_step_error(exact, engine_steps))?
    };

    Ok(CombatScenarioStepResultV1 {
        view: CombatScenarioStepViewV1 {
            scenario_count: group.worlds.len(),
            continuing_scenario_count,
            next_information_set_count: next_groups.len(),
            win_count,
            loss_count,
            escape_count,
            engine_steps,
        },
        next_groups,
        terminal_outcomes,
    })
}

fn scenario_step_error(
    exact: CombatScenarioPolicyErrorV1,
    engine_steps: usize,
) -> CombatScenarioStepErrorV1 {
    let failure = match &exact {
        CombatScenarioPolicyErrorV1::StepTruncated {
            engine_steps,
            timed_out,
            ..
        } => CombatScenarioStepFailureV1::Truncated {
            engine_steps: *engine_steps,
            timed_out: *timed_out,
        },
        _ => CombatScenarioStepFailureV1::PublicBoundary { engine_steps },
    };
    CombatScenarioStepErrorV1 { failure, exact }
}

fn terminal_outcome(
    scenario_id: &str,
    terminal: CombatScenarioTerminalV1,
    stepped: &crate::sim::combat::CombatStepResult,
) -> CombatScenarioTerminalOutcomeV1 {
    CombatScenarioTerminalOutcomeV1 {
        scenario_id: scenario_id.to_string(),
        terminal,
        final_hp: stepped.position.combat.entities.player.current_hp,
        player_block: stepped.position.combat.entities.player.block,
        enemy_effective_hp: stepped
            .position
            .combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| {
                monster
                    .current_hp
                    .max(0)
                    .saturating_add(monster.block.max(0))
            })
            .fold(0, i32::saturating_add),
        turn_count: stepped.position.combat.turn.turn_count,
        cards_played: stepped
            .position
            .combat
            .turn
            .counters
            .card_ids_played_this_combat
            .len()
            .try_into()
            .unwrap_or(u32::MAX),
    }
}

#[derive(Serialize)]
struct PublicHistoryTransitionV1<'a> {
    schema_name: &'static str,
    schema_version: u32,
    previous_history_id: &'a str,
    action: &'a CombatPublicActionV1,
    boundary: &'a CombatPolicyObservationEnvelopeV1,
}
