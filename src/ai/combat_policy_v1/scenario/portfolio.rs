use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::sim::combat::CombatStepLimits;

use super::group::CombatScenarioGroupV1;
use super::step::{step_combat_scenario_group_v1, CombatScenarioStepResultV1};
use super::types::{CombatPolicyInformationSetKeyV1, CombatPublicActionV1};

pub const COMBAT_SCENARIO_ACTION_PORTFOLIO_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatScenarioActionPortfolioLimitsV1 {
    pub max_candidates: usize,
    pub max_engine_steps_per_action: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatScenarioActionPortfolioMetricV1 {
    pub count: usize,
    pub mean: f64,
    pub min: i32,
    pub p10_nearest_rank: i32,
    pub median_nearest_rank: i32,
    pub p90_nearest_rank: i32,
    pub max: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatScenarioActionPortfolioEvaluationV1 {
    pub action: CombatPublicActionV1,
    pub scenario_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub continuing: usize,
    pub next_information_set_count: usize,
    pub observed_hp_loss: CombatScenarioActionPortfolioMetricV1,
    pub player_block: CombatScenarioActionPortfolioMetricV1,
    pub enemy_effective_hp: CombatScenarioActionPortfolioMetricV1,
    pub consumes_potion: bool,
    pub engine_steps: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatScenarioActionPortfolioV1 {
    pub schema_version: u32,
    pub information_set: CombatPolicyInformationSetKeyV1,
    pub scenario_count: usize,
    pub evaluations: Vec<CombatScenarioActionPortfolioEvaluationV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatScenarioActionPortfolioErrorV1 {
    InvalidLimit { field: &'static str },
    CandidateCountExceeds { candidate_count: usize, cap: usize },
    ActionEvaluationFailed { action: CombatPublicActionV1 },
}

impl fmt::Display for CombatScenarioActionPortfolioErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit { field } => {
                write!(
                    formatter,
                    "combat action portfolio limit '{field}' must be nonzero"
                )
            }
            Self::CandidateCountExceeds {
                candidate_count,
                cap,
            } => write!(
                formatter,
                "combat action portfolio exposes {candidate_count} candidates above cap {cap}"
            ),
            Self::ActionEvaluationFailed { action } => {
                write!(
                    formatter,
                    "combat action portfolio could not evaluate {action:?}"
                )
            }
        }
    }
}

impl Error for CombatScenarioActionPortfolioErrorV1 {}

#[derive(Clone, Copy)]
pub struct CombatScenarioActionPortfolioEvaluatorV1<'a> {
    group: &'a CombatScenarioGroupV1,
    session: &'a CombatScenarioActionPortfolioSessionV1,
}

pub(crate) struct CombatScenarioActionPortfolioSessionV1 {
    steps: RefCell<BTreeMap<(CombatPublicActionV1, usize), CombatScenarioStepResultV1>>,
    engine_steps: Cell<usize>,
}

impl CombatScenarioActionPortfolioSessionV1 {
    pub(crate) fn new() -> Self {
        Self {
            steps: RefCell::new(BTreeMap::new()),
            engine_steps: Cell::new(0),
        }
    }

    pub(crate) fn evaluator<'a>(
        &'a self,
        group: &'a CombatScenarioGroupV1,
    ) -> CombatScenarioActionPortfolioEvaluatorV1<'a> {
        CombatScenarioActionPortfolioEvaluatorV1 {
            group,
            session: self,
        }
    }

    pub(crate) fn engine_steps(&self) -> usize {
        self.engine_steps.get()
    }

    pub(crate) fn take_step(
        &self,
        action: &CombatPublicActionV1,
        max_engine_steps: usize,
    ) -> Option<CombatScenarioStepResultV1> {
        self.steps
            .borrow_mut()
            .remove(&(action.clone(), max_engine_steps))
    }
}

impl CombatScenarioActionPortfolioEvaluatorV1<'_> {
    pub fn evaluate(
        &self,
        limits: CombatScenarioActionPortfolioLimitsV1,
    ) -> Result<CombatScenarioActionPortfolioV1, CombatScenarioActionPortfolioErrorV1> {
        validate_limits(limits)?;
        let view = self.group.view();
        if view.candidates.len() > limits.max_candidates {
            return Err(
                CombatScenarioActionPortfolioErrorV1::CandidateCountExceeds {
                    candidate_count: view.candidates.len(),
                    cap: limits.max_candidates,
                },
            );
        }

        let root_hp = view.observation.observation.compatibility_public.player.hp;
        let mut evaluations = Vec::with_capacity(view.candidates.len());
        for action in &view.candidates {
            let cache_key = (action.clone(), limits.max_engine_steps_per_action);
            if !self.session.steps.borrow().contains_key(&cache_key) {
                let stepped = step_combat_scenario_group_v1(
                    self.group,
                    action,
                    CombatStepLimits {
                        max_engine_steps: limits.max_engine_steps_per_action,
                        deadline: None,
                    },
                )
                .map_err(|_| {
                    CombatScenarioActionPortfolioErrorV1::ActionEvaluationFailed {
                        action: action.clone(),
                    }
                })?;
                self.session.engine_steps.set(
                    self.session
                        .engine_steps
                        .get()
                        .saturating_add(stepped.view.engine_steps),
                );
                self.session
                    .steps
                    .borrow_mut()
                    .insert(cache_key.clone(), stepped);
            }
            let cached_steps = self.session.steps.borrow();
            let stepped = cached_steps
                .get(&cache_key)
                .expect("combat portfolio cache contains the evaluated action");
            let mut hp_losses = Vec::with_capacity(view.scenario_count);
            let mut player_blocks = Vec::with_capacity(view.scenario_count);
            let mut enemy_effective_hp = Vec::with_capacity(view.scenario_count);

            for terminal in &stepped.terminal_outcomes {
                hp_losses.push(root_hp.saturating_sub(terminal.final_hp));
                player_blocks.push(terminal.player_block);
                enemy_effective_hp.push(terminal.enemy_effective_hp);
            }
            for next_group in &stepped.next_groups {
                let next_public = &next_group
                    .view()
                    .observation
                    .observation
                    .compatibility_public;
                let next_enemy_effective_hp = next_public
                    .monsters
                    .iter()
                    .filter(|monster| monster.alive)
                    .map(|monster| monster.hp.max(0).saturating_add(monster.block.max(0)))
                    .fold(0, i32::saturating_add);
                for _ in 0..next_group.view().scenario_count {
                    hp_losses.push(root_hp.saturating_sub(next_public.player.hp));
                    player_blocks.push(next_public.player.block);
                    enemy_effective_hp.push(next_enemy_effective_hp);
                }
            }
            debug_assert_eq!(hp_losses.len(), view.scenario_count);
            debug_assert_eq!(player_blocks.len(), view.scenario_count);
            debug_assert_eq!(enemy_effective_hp.len(), view.scenario_count);

            evaluations.push(CombatScenarioActionPortfolioEvaluationV1 {
                action: action.clone(),
                scenario_count: stepped.view.scenario_count,
                wins: stepped.view.win_count,
                losses: stepped.view.loss_count,
                continuing: stepped.view.continuing_scenario_count,
                next_information_set_count: stepped.view.next_information_set_count,
                observed_hp_loss: metric_summary(hp_losses),
                player_block: metric_summary(player_blocks),
                enemy_effective_hp: metric_summary(enemy_effective_hp),
                consumes_potion: matches!(action, CombatPublicActionV1::UsePotion { .. }),
                engine_steps: stepped.view.engine_steps,
            });
        }

        Ok(CombatScenarioActionPortfolioV1 {
            schema_version: COMBAT_SCENARIO_ACTION_PORTFOLIO_SCHEMA_VERSION,
            information_set: view.key.clone(),
            scenario_count: view.scenario_count,
            evaluations,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatScenarioActionPortfolioSelectionBasisV1 {
    SoleCandidate,
    StrictParetoDominance,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatScenarioActionPortfolioSelectionV1 {
    pub action: CombatPublicActionV1,
    pub basis: CombatScenarioActionPortfolioSelectionBasisV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatScenarioActionPortfolioSelectionGapV1 {
    NoCandidates,
    NoStrictDominance,
}

pub fn select_forced_or_strictly_dominant_combat_action_v1(
    portfolio: &CombatScenarioActionPortfolioV1,
) -> Result<CombatScenarioActionPortfolioSelectionV1, CombatScenarioActionPortfolioSelectionGapV1> {
    match portfolio.evaluations.as_slice() {
        [] => return Err(CombatScenarioActionPortfolioSelectionGapV1::NoCandidates),
        [only] => {
            return Ok(CombatScenarioActionPortfolioSelectionV1 {
                action: only.action.clone(),
                basis: CombatScenarioActionPortfolioSelectionBasisV1::SoleCandidate,
            });
        }
        _ => {}
    }

    let mut selected = None;
    for candidate in &portfolio.evaluations {
        if portfolio
            .evaluations
            .iter()
            .filter(|other| other.action != candidate.action)
            .all(|other| strictly_dominates(candidate, other))
        {
            if selected.is_some() {
                return Err(CombatScenarioActionPortfolioSelectionGapV1::NoStrictDominance);
            }
            selected = Some(candidate.action.clone());
        }
    }

    selected
        .map(|action| CombatScenarioActionPortfolioSelectionV1 {
            action,
            basis: CombatScenarioActionPortfolioSelectionBasisV1::StrictParetoDominance,
        })
        .ok_or(CombatScenarioActionPortfolioSelectionGapV1::NoStrictDominance)
}

fn validate_limits(
    limits: CombatScenarioActionPortfolioLimitsV1,
) -> Result<(), CombatScenarioActionPortfolioErrorV1> {
    for (field, value) in [
        ("max_candidates", limits.max_candidates),
        (
            "max_engine_steps_per_action",
            limits.max_engine_steps_per_action,
        ),
    ] {
        if value == 0 {
            return Err(CombatScenarioActionPortfolioErrorV1::InvalidLimit { field });
        }
    }
    Ok(())
}

fn metric_summary(mut values: Vec<i32>) -> CombatScenarioActionPortfolioMetricV1 {
    values.sort_unstable();
    let count = values.len();
    debug_assert!(count > 0);
    let sum = values.iter().map(|value| i64::from(*value)).sum::<i64>();
    CombatScenarioActionPortfolioMetricV1 {
        count,
        mean: sum as f64 / count as f64,
        min: values[0],
        p10_nearest_rank: nearest_rank(&values, 10),
        median_nearest_rank: nearest_rank(&values, 50),
        p90_nearest_rank: nearest_rank(&values, 90),
        max: values[count - 1],
    }
}

fn nearest_rank(values: &[i32], percentile: usize) -> i32 {
    let rank = (percentile.saturating_mul(values.len()).saturating_add(99) / 100).max(1);
    values[rank.min(values.len()) - 1]
}

fn strictly_dominates(
    candidate: &CombatScenarioActionPortfolioEvaluationV1,
    other: &CombatScenarioActionPortfolioEvaluationV1,
) -> bool {
    let candidate_is_all_world_lethal = candidate.wins == candidate.scenario_count
        && candidate.losses == 0
        && candidate.continuing == 0;
    let other_is_all_world_lethal =
        other.wins == other.scenario_count && other.losses == 0 && other.continuing == 0;
    let meaningful_strict_improvement = !other_is_all_world_lethal
        || (!candidate.consumes_potion && other.consumes_potion)
        || candidate.observed_hp_loss.p90_nearest_rank < other.observed_hp_loss.p90_nearest_rank
        || candidate.observed_hp_loss.max < other.observed_hp_loss.max
        || candidate.player_block.p10_nearest_rank > other.player_block.p10_nearest_rank
        || candidate.player_block.min > other.player_block.min;
    candidate.scenario_count == other.scenario_count
        && candidate_is_all_world_lethal
        && meaningful_strict_improvement
        && candidate.wins >= other.wins
        && candidate.losses <= other.losses
        && candidate.continuing <= other.continuing
        && (!candidate.consumes_potion || other.consumes_potion)
        && candidate.observed_hp_loss.p90_nearest_rank <= other.observed_hp_loss.p90_nearest_rank
        && candidate.observed_hp_loss.max <= other.observed_hp_loss.max
        && candidate.player_block.p10_nearest_rank >= other.player_block.p10_nearest_rank
        && candidate.player_block.min >= other.player_block.min
        && candidate.enemy_effective_hp.p90_nearest_rank
            <= other.enemy_effective_hp.p90_nearest_rank
        && candidate.enemy_effective_hp.max <= other.enemy_effective_hp.max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_terminal_improvement_is_not_strict_terminal_dominance() {
        let portfolio = portfolio(vec![
            evaluation(CombatPublicActionV1::Proceed, 2, 1, 0, 1, false, 0, 0, 0),
            evaluation(CombatPublicActionV1::EndTurn, 2, 0, 0, 2, false, 0, 0, 20),
        ]);

        assert_eq!(
            select_forced_or_strictly_dominant_combat_action_v1(&portfolio),
            Err(CombatScenarioActionPortfolioSelectionGapV1::NoStrictDominance)
        );
    }

    #[test]
    fn all_world_lethal_without_potion_dominates_equivalent_potion_lethal() {
        let portfolio = portfolio(vec![
            evaluation(CombatPublicActionV1::EndTurn, 1, 1, 0, 0, false, 0, 0, 0),
            evaluation(
                CombatPublicActionV1::UsePotion {
                    potion_slot: 0,
                    potion_id: "FirePotion".to_string(),
                    target: None,
                },
                1,
                1,
                0,
                0,
                true,
                0,
                0,
                0,
            ),
        ]);

        assert_eq!(
            select_forced_or_strictly_dominant_combat_action_v1(&portfolio),
            Ok(CombatScenarioActionPortfolioSelectionV1 {
                action: CombatPublicActionV1::EndTurn,
                basis: CombatScenarioActionPortfolioSelectionBasisV1::StrictParetoDominance,
            })
        );
    }

    fn portfolio(
        evaluations: Vec<CombatScenarioActionPortfolioEvaluationV1>,
    ) -> CombatScenarioActionPortfolioV1 {
        CombatScenarioActionPortfolioV1 {
            schema_version: COMBAT_SCENARIO_ACTION_PORTFOLIO_SCHEMA_VERSION,
            information_set: CombatPolicyInformationSetKeyV1 {
                public_history_id: "history".to_string(),
                public_observation_hash: "observation".to_string(),
                public_candidate_set_hash: "candidates".to_string(),
            },
            scenario_count: evaluations[0].scenario_count,
            evaluations,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn evaluation(
        action: CombatPublicActionV1,
        scenario_count: usize,
        wins: usize,
        losses: usize,
        continuing: usize,
        consumes_potion: bool,
        hp_loss: i32,
        player_block: i32,
        enemy_effective_hp: i32,
    ) -> CombatScenarioActionPortfolioEvaluationV1 {
        CombatScenarioActionPortfolioEvaluationV1 {
            action,
            scenario_count,
            wins,
            losses,
            continuing,
            next_information_set_count: usize::from(continuing > 0),
            observed_hp_loss: metric_summary(vec![hp_loss; scenario_count]),
            player_block: metric_summary(vec![player_block; scenario_count]),
            enemy_effective_hp: metric_summary(vec![enemy_effective_hp; scenario_count]),
            consumes_potion,
            engine_steps: scenario_count,
        }
    }
}
