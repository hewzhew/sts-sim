use std::collections::BTreeMap;

use serde::Serialize;

use super::CombatTurnOptionPrefixCandidateV1;

pub const COMBAT_TURN_OPTION_OBSERVABLE_EFFECT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CombatTurnOptionObservableBoundaryKindV1 {
    CombatPlayerTurn,
    CombatPendingChoice,
    OtherPublicBoundary { engine_state: String },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatTurnOptionObservableSuccessorV1 {
    pub boundary: CombatTurnOptionObservableBoundaryKindV1,
    pub turn_count: u32,
    pub public_observation_hash: String,
    pub public_candidate_set_hash: String,
    pub scenario_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatTurnOptionTerminalMultiplicityV1 {
    pub wins: usize,
    pub losses: usize,
    pub escapes: usize,
}

impl CombatTurnOptionTerminalMultiplicityV1 {
    pub fn has_terminal_outcome(self) -> bool {
        self.wins > 0 || self.losses > 0 || self.escapes > 0
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatTurnOptionObservableEffectV1 {
    schema_version: u32,
    input_scenario_count: usize,
    terminal: CombatTurnOptionTerminalMultiplicityV1,
    continuing_scenario_count: usize,
    successors: Vec<CombatTurnOptionObservableSuccessorV1>,
}

impl CombatTurnOptionObservableEffectV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn input_scenario_count(&self) -> usize {
        self.input_scenario_count
    }

    pub fn terminal(&self) -> CombatTurnOptionTerminalMultiplicityV1 {
        self.terminal
    }

    pub fn continuing_scenario_count(&self) -> usize {
        self.continuing_scenario_count
    }

    pub fn successors(&self) -> &[CombatTurnOptionObservableSuccessorV1] {
        &self.successors
    }

    fn has_same_public_shape(&self, other: &Self) -> bool {
        self.schema_version == other.schema_version
            && self.input_scenario_count == other.input_scenario_count
            && self.terminal == other.terminal
            && self.continuing_scenario_count == other.continuing_scenario_count
            && self.successors == other.successors
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CombatTurnOptionObservableEffectEvidenceGapV1 {
    OutcomeCountOverflow,
    OutcomeCountMismatch {
        input_scenario_count: usize,
        accounted_scenario_count: usize,
    },
    SuccessorCountOverflow,
    SuccessorCountMismatch {
        continuing_scenario_count: usize,
        accounted_scenario_count: usize,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "availability", rename_all = "snake_case", deny_unknown_fields)]
pub enum CombatTurnOptionObservableEffectEvidenceV1 {
    Available {
        effect: CombatTurnOptionObservableEffectV1,
    },
    Inconclusive {
        gap: CombatTurnOptionObservableEffectEvidenceGapV1,
    },
}

impl CombatTurnOptionObservableEffectEvidenceV1 {
    pub fn available(&self) -> Option<&CombatTurnOptionObservableEffectV1> {
        match self {
            Self::Available { effect } => Some(effect),
            Self::Inconclusive { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatTurnOptionObservableEffectComparisonGapV1 {
    PublicEffectEvidenceUnavailable,
    TerminalPublicStateUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "relation", rename_all = "snake_case", deny_unknown_fields)]
pub enum CombatTurnOptionObservableEffectRelationV1 {
    ObservablySame,
    ObservablyDifferent,
    Inconclusive {
        reason: CombatTurnOptionObservableEffectComparisonGapV1,
    },
}

pub fn combat_turn_option_observable_effect_v1(
    candidate: &CombatTurnOptionPrefixCandidateV1,
) -> CombatTurnOptionObservableEffectEvidenceV1 {
    let Some(accounted_scenario_count) = candidate
        .wins
        .checked_add(candidate.losses)
        .and_then(|count| count.checked_add(candidate.escapes))
        .and_then(|count| count.checked_add(candidate.continuing))
    else {
        return CombatTurnOptionObservableEffectEvidenceV1::Inconclusive {
            gap: CombatTurnOptionObservableEffectEvidenceGapV1::OutcomeCountOverflow,
        };
    };
    if accounted_scenario_count != candidate.scenario_count {
        return CombatTurnOptionObservableEffectEvidenceV1::Inconclusive {
            gap: CombatTurnOptionObservableEffectEvidenceGapV1::OutcomeCountMismatch {
                input_scenario_count: candidate.scenario_count,
                accounted_scenario_count,
            },
        };
    }

    let mut canonical = BTreeMap::<CanonicalObservableSuccessorKeyV1, usize>::new();
    let mut successor_scenario_count = 0usize;
    for successor in &candidate.successors {
        let Some(next_successor_count) =
            successor_scenario_count.checked_add(successor.scenario_count)
        else {
            return CombatTurnOptionObservableEffectEvidenceV1::Inconclusive {
                gap: CombatTurnOptionObservableEffectEvidenceGapV1::SuccessorCountOverflow,
            };
        };
        successor_scenario_count = next_successor_count;

        let key = CanonicalObservableSuccessorKeyV1 {
            boundary: observable_boundary_kind(&successor.engine_state),
            turn_count: successor.turn_count,
            public_observation_hash: successor.information_set.public_observation_hash.clone(),
            public_candidate_set_hash: successor.information_set.public_candidate_set_hash.clone(),
        };
        let current = canonical.get(&key).copied().unwrap_or(0);
        let Some(merged) = current.checked_add(successor.scenario_count) else {
            return CombatTurnOptionObservableEffectEvidenceV1::Inconclusive {
                gap: CombatTurnOptionObservableEffectEvidenceGapV1::SuccessorCountOverflow,
            };
        };
        canonical.insert(key, merged);
    }
    if successor_scenario_count != candidate.continuing {
        return CombatTurnOptionObservableEffectEvidenceV1::Inconclusive {
            gap: CombatTurnOptionObservableEffectEvidenceGapV1::SuccessorCountMismatch {
                continuing_scenario_count: candidate.continuing,
                accounted_scenario_count: successor_scenario_count,
            },
        };
    }

    let successors = canonical
        .into_iter()
        .map(
            |(key, scenario_count)| CombatTurnOptionObservableSuccessorV1 {
                boundary: key.boundary,
                turn_count: key.turn_count,
                public_observation_hash: key.public_observation_hash,
                public_candidate_set_hash: key.public_candidate_set_hash,
                scenario_count,
            },
        )
        .collect();
    CombatTurnOptionObservableEffectEvidenceV1::Available {
        effect: CombatTurnOptionObservableEffectV1 {
            schema_version: COMBAT_TURN_OPTION_OBSERVABLE_EFFECT_SCHEMA_VERSION,
            input_scenario_count: candidate.scenario_count,
            terminal: CombatTurnOptionTerminalMultiplicityV1 {
                wins: candidate.wins,
                losses: candidate.losses,
                escapes: candidate.escapes,
            },
            continuing_scenario_count: candidate.continuing,
            successors,
        },
    }
}

pub fn compare_combat_turn_option_observable_effects_v1(
    left: &CombatTurnOptionObservableEffectEvidenceV1,
    right: &CombatTurnOptionObservableEffectEvidenceV1,
) -> CombatTurnOptionObservableEffectRelationV1 {
    let (
        CombatTurnOptionObservableEffectEvidenceV1::Available { effect: left },
        CombatTurnOptionObservableEffectEvidenceV1::Available { effect: right },
    ) = (left, right)
    else {
        return CombatTurnOptionObservableEffectRelationV1::Inconclusive {
            reason:
                CombatTurnOptionObservableEffectComparisonGapV1::PublicEffectEvidenceUnavailable,
        };
    };

    if !left.has_same_public_shape(right) {
        return CombatTurnOptionObservableEffectRelationV1::ObservablyDifferent;
    }
    if left.terminal.has_terminal_outcome() {
        return CombatTurnOptionObservableEffectRelationV1::Inconclusive {
            reason: CombatTurnOptionObservableEffectComparisonGapV1::TerminalPublicStateUnavailable,
        };
    }
    CombatTurnOptionObservableEffectRelationV1::ObservablySame
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CanonicalObservableSuccessorKeyV1 {
    boundary: CombatTurnOptionObservableBoundaryKindV1,
    turn_count: u32,
    public_observation_hash: String,
    public_candidate_set_hash: String,
}

fn observable_boundary_kind(engine_state: &str) -> CombatTurnOptionObservableBoundaryKindV1 {
    match engine_state {
        "combat_player_turn" => CombatTurnOptionObservableBoundaryKindV1::CombatPlayerTurn,
        "combat_pending_choice" => CombatTurnOptionObservableBoundaryKindV1::CombatPendingChoice,
        other => CombatTurnOptionObservableBoundaryKindV1::OtherPublicBoundary {
            engine_state: other.to_string(),
        },
    }
}
