use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::sim::combat::CombatStepLimits;

use super::super::turn_option_effect::{
    combat_turn_option_observable_effect_v1, CombatTurnOptionObservableEffectEvidenceV1,
};
use super::super::turn_option_schedule::{
    CombatTurnOptionCandidateExpansionStateV1, CombatTurnOptionWideningCandidateViewV1,
    CombatTurnOptionWideningChoiceV1, CombatTurnOptionWideningContextV1,
    CombatTurnOptionWideningScheduleV1,
};

use super::group::CombatScenarioGroupV1;
use super::step::{
    step_combat_scenario_group_v1, CombatScenarioStepFailureV1, CombatScenarioStepResultV1,
};
use super::types::{CombatPolicyInformationSetKeyV1, CombatPublicActionV1};

pub const COMBAT_TURN_OPTION_PREFIX_EXPANSION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatTurnOptionExpansionBudgetLimitsV1 {
    pub max_candidate_evaluations: usize,
    pub max_engine_steps: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_time_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatTurnOptionExpansionBudgetGrantV1 {
    pub additional_candidate_evaluations: usize,
    pub additional_engine_steps: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_time_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatTurnOptionExpansionBudgetSnapshotV1 {
    pub limits: CombatTurnOptionExpansionBudgetLimitsV1,
    pub candidate_evaluations: usize,
    pub remaining_candidate_evaluations: usize,
    pub engine_steps: usize,
    pub remaining_engine_steps: usize,
    pub deadline_reached: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CombatTurnOptionPrefixExpansionStopV1 {
    RequestedWidth,
    CandidateEvaluationBudget,
    EngineStepBudget,
    Deadline,
    ActionEvaluationFailed { action: CombatPublicActionV1 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CombatTurnOptionPrefixExpansionStatusV1 {
    Exhausted,
    PartiallyExpanded {
        cause: CombatTurnOptionPrefixExpansionStopV1,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatTurnOptionPrefixSuccessorV1 {
    pub information_set: CombatPolicyInformationSetKeyV1,
    pub engine_state: String,
    pub turn_count: u32,
    pub scenario_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatTurnOptionPrefixCandidateV1 {
    pub action: CombatPublicActionV1,
    pub scenario_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub escapes: usize,
    pub continuing: usize,
    pub successors: Vec<CombatTurnOptionPrefixSuccessorV1>,
    pub engine_steps: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatTurnOptionPrefixExpansionV1 {
    pub schema_version: u32,
    pub information_set: CombatPolicyInformationSetKeyV1,
    pub previous_opened_action_count: usize,
    pub newly_opened: Vec<CombatTurnOptionPrefixCandidateV1>,
    pub expansion_order: Vec<CombatPublicActionV1>,
    pub total_opened_action_count: usize,
    pub remaining_action_count: usize,
    pub new_candidate_evaluations: usize,
    pub new_engine_steps: usize,
    pub cumulative_engine_steps: usize,
    pub budget: CombatTurnOptionExpansionBudgetSnapshotV1,
    pub status: CombatTurnOptionPrefixExpansionStatusV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatTurnOptionExpansionErrorV1 {
    InvalidLimit { field: &'static str },
    SelectedUnknownCandidate { action: CombatPublicActionV1 },
    SelectedAlreadyExpandedCandidate { action: CombatPublicActionV1 },
    ReportedExhaustedWithUnopenedCandidates { remaining_action_count: usize },
}

impl fmt::Display for CombatTurnOptionExpansionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit { field } => {
                write!(
                    formatter,
                    "combat turn-option expansion limit '{field}' is invalid"
                )
            }
            Self::SelectedUnknownCandidate { action } => write!(
                formatter,
                "combat turn-option schedule selected unknown candidate {action:?}"
            ),
            Self::SelectedAlreadyExpandedCandidate { action } => write!(
                formatter,
                "combat turn-option schedule selected already-expanded candidate {action:?}"
            ),
            Self::ReportedExhaustedWithUnopenedCandidates {
                remaining_action_count,
            } => write!(
                formatter,
                "combat turn-option schedule reported exhaustion with {remaining_action_count} unopened candidates"
            ),
        }
    }
}

impl Error for CombatTurnOptionExpansionErrorV1 {}

pub struct CombatTurnOptionExpansionBudgetV1 {
    limits: CombatTurnOptionExpansionBudgetLimitsV1,
    deadline: Option<Instant>,
    candidate_evaluations: usize,
    engine_steps: usize,
}

impl CombatTurnOptionExpansionBudgetV1 {
    pub fn new(
        limits: CombatTurnOptionExpansionBudgetLimitsV1,
    ) -> Result<Self, CombatTurnOptionExpansionErrorV1> {
        for (field, value) in [
            (
                "max_candidate_evaluations",
                limits.max_candidate_evaluations,
            ),
            ("max_engine_steps", limits.max_engine_steps),
        ] {
            if value == 0 {
                return Err(CombatTurnOptionExpansionErrorV1::InvalidLimit { field });
            }
        }
        let deadline = budget_deadline(limits.wall_time_ms)?;

        Ok(Self {
            limits,
            deadline,
            candidate_evaluations: 0,
            engine_steps: 0,
        })
    }

    pub fn grant(
        &mut self,
        grant: CombatTurnOptionExpansionBudgetGrantV1,
    ) -> Result<CombatTurnOptionExpansionBudgetSnapshotV1, CombatTurnOptionExpansionErrorV1> {
        if grant.additional_candidate_evaluations == 0 && grant.additional_engine_steps == 0 {
            return Err(CombatTurnOptionExpansionErrorV1::InvalidLimit {
                field: "budget_grant",
            });
        }
        let max_candidate_evaluations = self
            .limits
            .max_candidate_evaluations
            .checked_add(grant.additional_candidate_evaluations)
            .ok_or(CombatTurnOptionExpansionErrorV1::InvalidLimit {
                field: "additional_candidate_evaluations",
            })?;
        let max_engine_steps = self
            .limits
            .max_engine_steps
            .checked_add(grant.additional_engine_steps)
            .ok_or(CombatTurnOptionExpansionErrorV1::InvalidLimit {
                field: "additional_engine_steps",
            })?;
        let deadline = budget_deadline(grant.wall_time_ms)?;

        self.limits = CombatTurnOptionExpansionBudgetLimitsV1 {
            max_candidate_evaluations,
            max_engine_steps,
            wall_time_ms: grant.wall_time_ms,
        };
        self.deadline = deadline;
        Ok(self.snapshot())
    }

    pub fn snapshot(&self) -> CombatTurnOptionExpansionBudgetSnapshotV1 {
        CombatTurnOptionExpansionBudgetSnapshotV1 {
            limits: self.limits,
            candidate_evaluations: self.candidate_evaluations,
            remaining_candidate_evaluations: self
                .limits
                .max_candidate_evaluations
                .saturating_sub(self.candidate_evaluations),
            engine_steps: self.engine_steps,
            remaining_engine_steps: self
                .limits
                .max_engine_steps
                .saturating_sub(self.engine_steps),
            deadline_reached: self
                .deadline
                .is_some_and(|deadline| Instant::now() >= deadline),
        }
    }

    fn preflight(&self) -> Result<CombatStepLimits, CombatTurnOptionPrefixExpansionStopV1> {
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(CombatTurnOptionPrefixExpansionStopV1::Deadline);
        }
        if self.candidate_evaluations >= self.limits.max_candidate_evaluations {
            return Err(CombatTurnOptionPrefixExpansionStopV1::CandidateEvaluationBudget);
        }
        let remaining_engine_steps = self
            .limits
            .max_engine_steps
            .saturating_sub(self.engine_steps);
        if remaining_engine_steps == 0 {
            return Err(CombatTurnOptionPrefixExpansionStopV1::EngineStepBudget);
        }
        Ok(CombatStepLimits {
            max_engine_steps: remaining_engine_steps,
            deadline: self.deadline,
        })
    }

    fn record_candidate_evaluation(&mut self, engine_steps: usize) {
        self.candidate_evaluations = self.candidate_evaluations.saturating_add(1);
        self.engine_steps = self.engine_steps.saturating_add(engine_steps);
        debug_assert!(self.candidate_evaluations <= self.limits.max_candidate_evaluations);
        debug_assert!(self.engine_steps <= self.limits.max_engine_steps);
    }
}

fn budget_deadline(
    wall_time_ms: Option<u64>,
) -> Result<Option<Instant>, CombatTurnOptionExpansionErrorV1> {
    wall_time_ms
        .map(Duration::from_millis)
        .map(|duration| {
            Instant::now().checked_add(duration).ok_or(
                CombatTurnOptionExpansionErrorV1::InvalidLimit {
                    field: "wall_time_ms",
                },
            )
        })
        .transpose()
}

struct ExpandedCombatTurnOptionPrefixCandidateV1 {
    public: CombatTurnOptionPrefixCandidateV1,
    observable_effect: CombatTurnOptionObservableEffectEvidenceV1,
    retained_step: Option<CombatScenarioStepResultV1>,
}

pub struct CombatTurnOptionPrefixExpansionSessionV1 {
    root: CombatScenarioGroupV1,
    canonical_candidates: Vec<CombatPublicActionV1>,
    expanded: BTreeMap<CombatPublicActionV1, ExpandedCombatTurnOptionPrefixCandidateV1>,
    expansion_order: Vec<CombatPublicActionV1>,
    cumulative_engine_steps: usize,
}

impl CombatTurnOptionPrefixExpansionSessionV1 {
    pub fn new(root: CombatScenarioGroupV1) -> Self {
        let canonical_candidates = root.view().candidates.clone();
        Self {
            root,
            canonical_candidates,
            expanded: BTreeMap::new(),
            expansion_order: Vec::new(),
            cumulative_engine_steps: 0,
        }
    }

    pub fn information_set(&self) -> &CombatPolicyInformationSetKeyV1 {
        &self.root.view().key
    }

    pub fn opened_action_count(&self) -> usize {
        self.expanded.len()
    }

    pub fn remaining_action_count(&self) -> usize {
        self.canonical_candidates
            .len()
            .saturating_sub(self.expanded.len())
    }

    pub fn cumulative_engine_steps(&self) -> usize {
        self.cumulative_engine_steps
    }

    pub fn opened_candidates(&self) -> impl Iterator<Item = &CombatTurnOptionPrefixCandidateV1> {
        self.expansion_order
            .iter()
            .filter_map(|action| self.expanded.get(action).map(|candidate| &candidate.public))
    }

    pub fn successor_groups(
        &self,
        action: &CombatPublicActionV1,
    ) -> Option<&[CombatScenarioGroupV1]> {
        self.expanded
            .get(action)
            .and_then(|candidate| candidate.retained_step.as_ref())
            .map(|stepped| stepped.next_groups.as_slice())
    }

    pub fn widening_context<'a>(
        &'a self,
        budget: &CombatTurnOptionExpansionBudgetV1,
    ) -> CombatTurnOptionWideningContextV1<'a> {
        let candidates = self
            .canonical_candidates
            .iter()
            .map(|action| {
                let expanded = self.expanded.get(action);
                let state = match expanded {
                    None => CombatTurnOptionCandidateExpansionStateV1::Unopened,
                    Some(candidate) if candidate.retained_step.is_some() => {
                        CombatTurnOptionCandidateExpansionStateV1::Expanded
                    }
                    Some(_) => CombatTurnOptionCandidateExpansionStateV1::TransitionConsumed,
                };
                CombatTurnOptionWideningCandidateViewV1 {
                    action,
                    state,
                    result: expanded.map(|candidate| &candidate.public),
                    observable_effect: expanded.map(|candidate| &candidate.observable_effect),
                }
            })
            .collect();
        CombatTurnOptionWideningContextV1 {
            information_set: &self.root.view().key,
            observation: &self.root.view().observation,
            scenario_count: self.root.view().scenario_count,
            candidates,
            expansion_order: &self.expansion_order,
            budget: budget.snapshot(),
        }
    }

    pub(super) fn root_group(&self) -> &CombatScenarioGroupV1 {
        &self.root
    }

    pub(super) fn opened_step(
        &self,
        action: &CombatPublicActionV1,
    ) -> Option<&CombatScenarioStepResultV1> {
        self.expanded
            .get(action)
            .and_then(|candidate| candidate.retained_step.as_ref())
    }

    pub(super) fn into_opened_step(
        mut self,
        action: &CombatPublicActionV1,
    ) -> Option<CombatScenarioStepResultV1> {
        self.expanded
            .get_mut(action)
            .and_then(|candidate| candidate.retained_step.take())
    }

    pub fn widen(
        &mut self,
        budget: &mut CombatTurnOptionExpansionBudgetV1,
        max_new_actions: usize,
    ) -> Result<CombatTurnOptionPrefixExpansionV1, CombatTurnOptionExpansionErrorV1> {
        if max_new_actions == 0 {
            return Err(CombatTurnOptionExpansionErrorV1::InvalidLimit {
                field: "max_new_actions",
            });
        }

        let actions = self
            .canonical_candidates
            .iter()
            .filter(|action| !self.expanded.contains_key(*action))
            .take(max_new_actions)
            .cloned()
            .collect::<Vec<_>>();
        Ok(self.widen_selected_actions(budget, actions))
    }

    pub fn widen_next_with_schedule(
        &mut self,
        budget: &mut CombatTurnOptionExpansionBudgetV1,
        schedule: &dyn CombatTurnOptionWideningScheduleV1,
    ) -> Result<CombatTurnOptionPrefixExpansionV1, CombatTurnOptionExpansionErrorV1> {
        let previous_opened_action_count = self.expanded.len();
        if self.remaining_action_count() == 0 {
            return Ok(self.expansion_report(
                budget,
                previous_opened_action_count,
                Vec::new(),
                0,
                0,
                None,
            ));
        }
        if let Err(cause) = budget.preflight() {
            return Ok(self.expansion_report(
                budget,
                previous_opened_action_count,
                Vec::new(),
                0,
                0,
                Some(cause),
            ));
        }

        let choice = schedule.select_next(&self.widening_context(budget));
        let action = match choice {
            CombatTurnOptionWideningChoiceV1::Expand { action } => action,
            CombatTurnOptionWideningChoiceV1::Exhausted => {
                return Err(
                    CombatTurnOptionExpansionErrorV1::ReportedExhaustedWithUnopenedCandidates {
                        remaining_action_count: self.remaining_action_count(),
                    },
                );
            }
        };
        if !self.canonical_candidates.contains(&action) {
            return Err(CombatTurnOptionExpansionErrorV1::SelectedUnknownCandidate { action });
        }
        if self.expanded.contains_key(&action) {
            return Err(
                CombatTurnOptionExpansionErrorV1::SelectedAlreadyExpandedCandidate { action },
            );
        }

        Ok(self.widen_selected_actions(budget, vec![action]))
    }

    fn widen_selected_actions(
        &mut self,
        budget: &mut CombatTurnOptionExpansionBudgetV1,
        actions: Vec<CombatPublicActionV1>,
    ) -> CombatTurnOptionPrefixExpansionV1 {
        let previous_opened_action_count = self.expanded.len();
        let mut newly_opened = Vec::with_capacity(actions.len());
        let mut new_candidate_evaluations = 0usize;
        let mut new_engine_steps = 0usize;
        let mut stop = None;

        for action in actions {
            let step_limits = match budget.preflight() {
                Ok(limits) => limits,
                Err(cause) => {
                    stop = Some(cause);
                    break;
                }
            };
            let stepped = match step_combat_scenario_group_v1(&self.root, &action, step_limits) {
                Ok(stepped) => stepped,
                Err(error) => {
                    let engine_steps = error.failure.engine_steps();
                    budget.record_candidate_evaluation(engine_steps);
                    self.cumulative_engine_steps =
                        self.cumulative_engine_steps.saturating_add(engine_steps);
                    new_candidate_evaluations = new_candidate_evaluations.saturating_add(1);
                    new_engine_steps = new_engine_steps.saturating_add(engine_steps);
                    stop = Some(match error.failure {
                        CombatScenarioStepFailureV1::Truncated {
                            timed_out: true, ..
                        } => CombatTurnOptionPrefixExpansionStopV1::Deadline,
                        CombatScenarioStepFailureV1::Truncated { .. } => {
                            CombatTurnOptionPrefixExpansionStopV1::EngineStepBudget
                        }
                        CombatScenarioStepFailureV1::PublicBoundary { .. } => {
                            CombatTurnOptionPrefixExpansionStopV1::ActionEvaluationFailed {
                                action: action.clone(),
                            }
                        }
                    });
                    break;
                }
            };
            budget.record_candidate_evaluation(stepped.view.engine_steps);
            self.cumulative_engine_steps = self
                .cumulative_engine_steps
                .saturating_add(stepped.view.engine_steps);
            new_candidate_evaluations = new_candidate_evaluations.saturating_add(1);
            let public = public_candidate(&action, &stepped);
            let observable_effect = combat_turn_option_observable_effect_v1(&public);
            new_engine_steps = new_engine_steps.saturating_add(public.engine_steps);
            newly_opened.push(public.clone());
            self.expansion_order.push(action.clone());
            let previous = self.expanded.insert(
                action,
                ExpandedCombatTurnOptionPrefixCandidateV1 {
                    public,
                    observable_effect,
                    retained_step: Some(stepped),
                },
            );
            debug_assert!(previous.is_none());
        }

        self.expansion_report(
            budget,
            previous_opened_action_count,
            newly_opened,
            new_candidate_evaluations,
            new_engine_steps,
            stop,
        )
    }

    fn expansion_report(
        &self,
        budget: &CombatTurnOptionExpansionBudgetV1,
        previous_opened_action_count: usize,
        newly_opened: Vec<CombatTurnOptionPrefixCandidateV1>,
        new_candidate_evaluations: usize,
        new_engine_steps: usize,
        stop: Option<CombatTurnOptionPrefixExpansionStopV1>,
    ) -> CombatTurnOptionPrefixExpansionV1 {
        let remaining_action_count = self.remaining_action_count();
        let status = if remaining_action_count == 0 {
            CombatTurnOptionPrefixExpansionStatusV1::Exhausted
        } else {
            CombatTurnOptionPrefixExpansionStatusV1::PartiallyExpanded {
                cause: stop.unwrap_or(CombatTurnOptionPrefixExpansionStopV1::RequestedWidth),
            }
        };

        CombatTurnOptionPrefixExpansionV1 {
            schema_version: COMBAT_TURN_OPTION_PREFIX_EXPANSION_SCHEMA_VERSION,
            information_set: self.root.view().key.clone(),
            previous_opened_action_count,
            newly_opened,
            expansion_order: self.expansion_order.clone(),
            total_opened_action_count: self.expanded.len(),
            remaining_action_count,
            new_candidate_evaluations,
            new_engine_steps,
            cumulative_engine_steps: self.cumulative_engine_steps,
            budget: budget.snapshot(),
            status,
        }
    }
}

fn public_candidate(
    action: &CombatPublicActionV1,
    stepped: &CombatScenarioStepResultV1,
) -> CombatTurnOptionPrefixCandidateV1 {
    CombatTurnOptionPrefixCandidateV1 {
        action: action.clone(),
        scenario_count: stepped.view.scenario_count,
        wins: stepped.view.win_count,
        losses: stepped.view.loss_count,
        escapes: stepped.view.escape_count,
        continuing: stepped.view.continuing_scenario_count,
        successors: stepped
            .next_groups
            .iter()
            .map(|group| CombatTurnOptionPrefixSuccessorV1 {
                information_set: group.view().key.clone(),
                engine_state: group.view().observation.engine_state.clone(),
                turn_count: group.view().observation.turn_count,
                scenario_count: group.view().scenario_count,
            })
            .collect(),
        engine_steps: stepped.view.engine_steps,
    }
}
