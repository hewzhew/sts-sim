use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::super::turn_option_schedule::CombatTurnOptionWideningScheduleV1;
use super::group::CombatScenarioGroupV1;
use super::turn_option::{
    CombatTurnOptionExpansionBudgetGrantV1, CombatTurnOptionExpansionBudgetSnapshotV1,
    CombatTurnOptionExpansionBudgetV1, CombatTurnOptionExpansionErrorV1,
    CombatTurnOptionPrefixExpansionSessionV1, CombatTurnOptionPrefixExpansionV1,
};
use super::types::{CombatPolicyInformationSetKeyV1, CombatPublicActionV1};

pub const COMBAT_PUBLIC_TURN_OPTION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPublicTurnOptionCompletionV1 {
    Open,
    Complete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPublicTurnOptionTerminalV1 {
    Win,
    Loss,
    Escape,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatPublicTurnOptionSuccessorV1 {
    OpenContinuation {
        information_set: CombatPolicyInformationSetKeyV1,
        engine_state: String,
        turn_count: u32,
        scenario_count: usize,
    },
    NextPlayerTurn {
        information_set: CombatPolicyInformationSetKeyV1,
        turn_count: u32,
        scenario_count: usize,
    },
    Terminal {
        terminal: CombatPublicTurnOptionTerminalV1,
        scenario_count: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicTurnOptionDecisionV1 {
    pub information_set: CombatPolicyInformationSetKeyV1,
    pub action: CombatPublicActionV1,
    pub successors: Vec<CombatPublicTurnOptionSuccessorV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicTurnOptionOpenLeafV1 {
    pub information_set: CombatPolicyInformationSetKeyV1,
    pub engine_state: String,
    pub turn_count: u32,
    pub scenario_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicTurnOptionV1 {
    pub schema_version: u32,
    pub root_information_set: CombatPolicyInformationSetKeyV1,
    pub root_turn_count: u32,
    pub decisions: Vec<CombatPublicTurnOptionDecisionV1>,
    pub open_leaves: Vec<CombatPublicTurnOptionOpenLeafV1>,
    pub completion: CombatPublicTurnOptionCompletionV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatPublicTurnOptionCompositionErrorV1 {
    InvalidRootBoundary {
        engine_state: String,
    },
    UnknownOpenInformationSet {
        information_set: CombatPolicyInformationSetKeyV1,
    },
    ActionNotOpened {
        information_set: CombatPolicyInformationSetKeyV1,
        action: CombatPublicActionV1,
    },
    DuplicateOpenInformationSet {
        information_set: CombatPolicyInformationSetKeyV1,
    },
    PrefixExpansionFailed {
        information_set: CombatPolicyInformationSetKeyV1,
        source: CombatTurnOptionExpansionErrorV1,
    },
}

impl fmt::Display for CombatPublicTurnOptionCompositionErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRootBoundary { engine_state } => write!(
                formatter,
                "combat turn option must start at a player-turn boundary, got {engine_state}"
            ),
            Self::UnknownOpenInformationSet { information_set } => write!(
                formatter,
                "combat turn option has no open leaf for information set '{}'",
                information_set.public_observation_hash
            ),
            Self::ActionNotOpened {
                information_set,
                action,
            } => write!(
                formatter,
                "combat turn-option action {action:?} was not opened for information set '{}'",
                information_set.public_observation_hash
            ),
            Self::DuplicateOpenInformationSet { information_set } => write!(
                formatter,
                "combat turn option produced duplicate open information set '{}'",
                information_set.public_observation_hash
            ),
            Self::PrefixExpansionFailed {
                information_set,
                source,
            } => write!(
                formatter,
                "combat turn option could not expand open information set '{}': {source}",
                information_set.public_observation_hash
            ),
        }
    }
}

impl Error for CombatPublicTurnOptionCompositionErrorV1 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PrefixExpansionFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub struct CombatPublicTurnOptionCompositionSessionV1 {
    root_information_set: CombatPolicyInformationSetKeyV1,
    root_turn_count: u32,
    open_leaves:
        BTreeMap<CombatPolicyInformationSetKeyV1, CombatTurnOptionPrefixExpansionSessionV1>,
    decisions: BTreeMap<CombatPolicyInformationSetKeyV1, CombatPublicTurnOptionDecisionV1>,
    budget: CombatTurnOptionExpansionBudgetV1,
}

impl CombatPublicTurnOptionCompositionSessionV1 {
    pub fn new(
        root: CombatScenarioGroupV1,
        budget: CombatTurnOptionExpansionBudgetV1,
    ) -> Result<Self, CombatPublicTurnOptionCompositionErrorV1> {
        if root.view().observation.engine_state != "combat_player_turn" {
            return Err(
                CombatPublicTurnOptionCompositionErrorV1::InvalidRootBoundary {
                    engine_state: root.view().observation.engine_state.clone(),
                },
            );
        }
        let root_information_set = root.view().key.clone();
        let root_turn_count = root.view().observation.turn_count;
        Ok(Self {
            root_information_set: root_information_set.clone(),
            root_turn_count,
            open_leaves: BTreeMap::from([(
                root_information_set,
                CombatTurnOptionPrefixExpansionSessionV1::new(root),
            )]),
            decisions: BTreeMap::new(),
            budget,
        })
    }

    pub fn widen_open_leaf(
        &mut self,
        information_set: &CombatPolicyInformationSetKeyV1,
        max_new_actions: usize,
    ) -> Result<CombatTurnOptionPrefixExpansionV1, CombatPublicTurnOptionCompositionErrorV1> {
        let expansion = self.open_leaves.get_mut(information_set).ok_or_else(|| {
            CombatPublicTurnOptionCompositionErrorV1::UnknownOpenInformationSet {
                information_set: information_set.clone(),
            }
        })?;
        expansion
            .widen(&mut self.budget, max_new_actions)
            .map_err(
                |source| CombatPublicTurnOptionCompositionErrorV1::PrefixExpansionFailed {
                    information_set: information_set.clone(),
                    source,
                },
            )
    }

    pub fn widen_open_leaf_with_schedule(
        &mut self,
        information_set: &CombatPolicyInformationSetKeyV1,
        schedule: &dyn CombatTurnOptionWideningScheduleV1,
    ) -> Result<CombatTurnOptionPrefixExpansionV1, CombatPublicTurnOptionCompositionErrorV1> {
        let expansion = self.open_leaves.get_mut(information_set).ok_or_else(|| {
            CombatPublicTurnOptionCompositionErrorV1::UnknownOpenInformationSet {
                information_set: information_set.clone(),
            }
        })?;
        expansion
            .widen_next_with_schedule(&mut self.budget, schedule)
            .map_err(
                |source| CombatPublicTurnOptionCompositionErrorV1::PrefixExpansionFailed {
                    information_set: information_set.clone(),
                    source,
                },
            )
    }

    pub fn budget_snapshot(&self) -> CombatTurnOptionExpansionBudgetSnapshotV1 {
        self.budget.snapshot()
    }

    pub fn grant_budget(
        &mut self,
        grant: CombatTurnOptionExpansionBudgetGrantV1,
    ) -> Result<CombatTurnOptionExpansionBudgetSnapshotV1, CombatTurnOptionExpansionErrorV1> {
        self.budget.grant(grant)
    }

    #[cfg(test)]
    pub(super) fn insert_open_leaf_for_test(&mut self, group: CombatScenarioGroupV1) {
        let information_set = group.view().key.clone();
        let previous = self.open_leaves.insert(
            information_set,
            CombatTurnOptionPrefixExpansionSessionV1::new(group),
        );
        assert!(previous.is_none(), "test leaf must be new");
    }

    #[cfg(test)]
    pub(super) fn remove_open_leaf_for_test(
        &mut self,
        information_set: &CombatPolicyInformationSetKeyV1,
    ) {
        assert!(
            self.open_leaves.remove(information_set).is_some(),
            "test leaf must exist"
        );
    }
    pub fn commit_opened_action(
        &mut self,
        information_set: &CombatPolicyInformationSetKeyV1,
        action: &CombatPublicActionV1,
    ) -> Result<CombatPublicTurnOptionV1, CombatPublicTurnOptionCompositionErrorV1> {
        let duplicate_successor = {
            let expansion = self.open_leaves.get(information_set).ok_or_else(|| {
                CombatPublicTurnOptionCompositionErrorV1::UnknownOpenInformationSet {
                    information_set: information_set.clone(),
                }
            })?;
            let stepped = expansion.opened_step(action).ok_or_else(|| {
                CombatPublicTurnOptionCompositionErrorV1::ActionNotOpened {
                    information_set: information_set.clone(),
                    action: action.clone(),
                }
            })?;
            let mut successor_keys = BTreeSet::new();
            stepped.next_groups.iter().find_map(|group| {
                let key = group.view().key.clone();
                if !successor_keys.insert(key.clone()) || self.open_leaves.contains_key(&key) {
                    Some(key)
                } else {
                    None
                }
            })
        };
        if let Some(information_set) = duplicate_successor {
            return Err(
                CombatPublicTurnOptionCompositionErrorV1::DuplicateOpenInformationSet {
                    information_set,
                },
            );
        }

        let expansion = self
            .open_leaves
            .remove(information_set)
            .expect("validated open turn-option leaf exists");
        let stepped = expansion
            .into_opened_step(action)
            .expect("validated opened transition remains retained");

        let mut successors = Vec::new();
        if stepped.view.win_count > 0 {
            successors.push(CombatPublicTurnOptionSuccessorV1::Terminal {
                terminal: CombatPublicTurnOptionTerminalV1::Win,
                scenario_count: stepped.view.win_count,
            });
        }
        if stepped.view.loss_count > 0 {
            successors.push(CombatPublicTurnOptionSuccessorV1::Terminal {
                terminal: CombatPublicTurnOptionTerminalV1::Loss,
                scenario_count: stepped.view.loss_count,
            });
        }
        if stepped.view.escape_count > 0 {
            successors.push(CombatPublicTurnOptionSuccessorV1::Terminal {
                terminal: CombatPublicTurnOptionTerminalV1::Escape,
                scenario_count: stepped.view.escape_count,
            });
        }
        for group in stepped.next_groups {
            let view = group.view();
            let successor = if view.observation.engine_state == "combat_player_turn"
                && view.observation.turn_count > self.root_turn_count
            {
                CombatPublicTurnOptionSuccessorV1::NextPlayerTurn {
                    information_set: view.key.clone(),
                    turn_count: view.observation.turn_count,
                    scenario_count: view.scenario_count,
                }
            } else {
                let information_set = view.key.clone();
                let successor = CombatPublicTurnOptionSuccessorV1::OpenContinuation {
                    information_set: information_set.clone(),
                    engine_state: view.observation.engine_state.clone(),
                    turn_count: view.observation.turn_count,
                    scenario_count: view.scenario_count,
                };
                let previous = self.open_leaves.insert(
                    information_set.clone(),
                    CombatTurnOptionPrefixExpansionSessionV1::new(group),
                );
                debug_assert!(previous.is_none());
                successor
            };
            successors.push(successor);
        }
        debug_assert_eq!(
            successors
                .iter()
                .map(successor_scenario_count)
                .sum::<usize>(),
            stepped.view.scenario_count
        );
        let previous = self.decisions.insert(
            information_set.clone(),
            CombatPublicTurnOptionDecisionV1 {
                information_set: information_set.clone(),
                action: action.clone(),
                successors,
            },
        );
        debug_assert!(previous.is_none());

        Ok(self.snapshot())
    }

    pub fn snapshot(&self) -> CombatPublicTurnOptionV1 {
        let open_leaves = self
            .open_leaves
            .values()
            .map(|expansion| {
                let group = expansion.root_group();
                CombatPublicTurnOptionOpenLeafV1 {
                    information_set: group.view().key.clone(),
                    engine_state: group.view().observation.engine_state.clone(),
                    turn_count: group.view().observation.turn_count,
                    scenario_count: group.view().scenario_count,
                }
            })
            .collect::<Vec<_>>();
        CombatPublicTurnOptionV1 {
            schema_version: COMBAT_PUBLIC_TURN_OPTION_SCHEMA_VERSION,
            root_information_set: self.root_information_set.clone(),
            root_turn_count: self.root_turn_count,
            decisions: self.decisions.values().cloned().collect(),
            completion: if open_leaves.is_empty() {
                CombatPublicTurnOptionCompletionV1::Complete
            } else {
                CombatPublicTurnOptionCompletionV1::Open
            },
            open_leaves,
        }
    }
}

fn successor_scenario_count(successor: &CombatPublicTurnOptionSuccessorV1) -> usize {
    match successor {
        CombatPublicTurnOptionSuccessorV1::OpenContinuation { scenario_count, .. }
        | CombatPublicTurnOptionSuccessorV1::NextPlayerTurn { scenario_count, .. }
        | CombatPublicTurnOptionSuccessorV1::Terminal { scenario_count, .. } => *scenario_count,
    }
}
