//! Shared root/selection ordinal driver for run batches and same-root combat groups.
//!
//! This module converts typed boundaries into model decisions, preserves bridge-local symbolic
//! prefixes, and prepares complete action batches. It owns no environment mutation or policy.

use sts_oracle_eval::eval::run_control::{
    CombatLearningBoundaryV1, CombatLearningEnvPoolV1, LearningActionV1, LearningBoundaryV1,
    LearningEnvPoolV1, LearningModelChoiceV1, LearningModelDecisionV1, LearningModelObservationV1,
    LearningSelectionStepV1,
};

use super::semantic::{SemanticBatch, SemanticBatchBuilder};
use super::{
    BridgeSlotState, DecisionSnapshot, PHASE_COMBAT_ROOT, PHASE_SELECTION, PHASE_STRATEGIC_ROOT,
};

pub(super) trait BridgeDecisionSource {
    fn bridge_slot_count(&self) -> usize;
    fn bridge_is_terminal(&self, slot_index: usize) -> Result<bool, String>;
    fn bridge_root_decision(
        &self,
        slot_index: usize,
    ) -> Result<LearningModelDecisionV1<'_>, String>;
}

impl BridgeDecisionSource for LearningEnvPoolV1 {
    fn bridge_slot_count(&self) -> usize {
        self.slot_count()
    }

    fn bridge_is_terminal(&self, slot_index: usize) -> Result<bool, String> {
        self.boundary(slot_index)
            .map(LearningBoundaryV1::is_terminal)
            .ok_or_else(|| format!("missing pool slot {slot_index}"))
    }

    fn bridge_root_decision(
        &self,
        slot_index: usize,
    ) -> Result<LearningModelDecisionV1<'_>, String> {
        let boundary = self
            .boundary(slot_index)
            .ok_or_else(|| format!("missing pool slot {slot_index}"))?;
        LearningModelDecisionV1::from_boundary(boundary).map_err(|error| error.to_string())
    }
}

impl BridgeDecisionSource for CombatLearningEnvPoolV1 {
    fn bridge_slot_count(&self) -> usize {
        self.replicate_count()
    }

    fn bridge_is_terminal(&self, slot_index: usize) -> Result<bool, String> {
        let replicate_index = u32::try_from(slot_index)
            .map_err(|_| format!("combat replicate index {slot_index} exceeds u32"))?;
        self.boundary(replicate_index)
            .map(CombatLearningBoundaryV1::is_terminal)
            .ok_or_else(|| format!("missing combat replicate {replicate_index}"))
    }

    fn bridge_root_decision(
        &self,
        slot_index: usize,
    ) -> Result<LearningModelDecisionV1<'_>, String> {
        let replicate_index = u32::try_from(slot_index)
            .map_err(|_| format!("combat replicate index {slot_index} exceeds u32"))?;
        let boundary = self
            .boundary(replicate_index)
            .ok_or_else(|| format!("missing combat replicate {replicate_index}"))?;
        let CombatLearningBoundaryV1::Decision { boundary, .. } = boundary else {
            return Err(format!("combat replicate {replicate_index} is terminal"));
        };
        LearningModelDecisionV1::from_combat_boundary_with_potion_policy(
            boundary,
            self.potion_policy(),
        )
        .map_err(|error| error.to_string())
    }
}

pub(super) fn decision_snapshot_from_source(
    source: &impl BridgeDecisionSource,
    states: &[BridgeSlotState],
) -> Result<DecisionSnapshot, String> {
    if states.len() != source.bridge_slot_count() {
        return Err("bridge state count does not match its decision source".to_owned());
    }
    let mut slot_indices = Vec::new();
    let mut phases = Vec::new();
    let mut candidate_counts = Vec::new();
    let mut candidate_row_splits = vec![0];

    for (slot_index, state) in states.iter().enumerate() {
        let (phase, candidate_count) = match state {
            BridgeSlotState::Root => {
                let decision = source.bridge_root_decision(slot_index)?;
                let phase = match decision.observation {
                    LearningModelObservationV1::Strategic(_) => PHASE_STRATEGIC_ROOT,
                    LearningModelObservationV1::Combat(_) => PHASE_COMBAT_ROOT,
                };
                (phase, decision.candidates.len())
            }
            BridgeSlotState::Selection { draft, .. } => {
                (PHASE_SELECTION, draft.decision().candidates.len())
            }
            BridgeSlotState::Terminal | BridgeSlotState::Ready { .. } => continue,
        };
        if candidate_count == 0 {
            return Err(format!("slot {slot_index} exposed an empty decision row"));
        }
        let next = candidate_row_splits
            .last()
            .copied()
            .unwrap_or(0usize)
            .checked_add(candidate_count)
            .ok_or_else(|| "candidate count overflow".to_owned())?;
        slot_indices.push(slot_index);
        phases.push(phase);
        candidate_counts.push(candidate_count);
        candidate_row_splits.push(next);
    }

    Ok(DecisionSnapshot {
        slot_indices,
        phases,
        candidate_counts,
        candidate_row_splits,
    })
}

pub(super) fn semantic_snapshot_from_source(
    source: &impl BridgeDecisionSource,
    states: &[BridgeSlotState],
) -> Result<SemanticBatch, String> {
    if states.len() != source.bridge_slot_count() {
        return Err("bridge state count does not match its decision source".to_owned());
    }
    let mut builder = SemanticBatchBuilder::new();
    for (slot_index, state) in states.iter().enumerate() {
        match state {
            BridgeSlotState::Root => {
                let decision = source.bridge_root_decision(slot_index)?;
                builder
                    .push_decision(&decision)
                    .map_err(|error| error.to_string())?;
            }
            BridgeSlotState::Selection { draft, .. } => {
                let decision = source.bridge_root_decision(slot_index)?;
                builder
                    .push_selection(decision.observation, draft)
                    .map_err(|error| error.to_string())?;
            }
            BridgeSlotState::Terminal | BridgeSlotState::Ready { .. } => {}
        }
    }
    Ok(builder.finish())
}

pub(super) fn bridge_states_ready(states: &[BridgeSlotState]) -> bool {
    states.iter().all(|state| {
        matches!(
            state,
            BridgeSlotState::Terminal | BridgeSlotState::Ready { .. }
        )
    })
}

pub(super) fn choose_bridge_ordinals(
    source: &impl BridgeDecisionSource,
    states: &mut Vec<BridgeSlotState>,
    ordinals: Vec<usize>,
) -> Result<(), String> {
    let snapshot = decision_snapshot_from_source(source, states)?;
    if ordinals.len() != snapshot.slot_indices.len() {
        return Err(format!(
            "expected {} candidate ordinals, received {}",
            snapshot.slot_indices.len(),
            ordinals.len()
        ));
    }

    let mut next_states = states.clone();
    for ((slot_index, candidate_count), ordinal) in snapshot
        .slot_indices
        .into_iter()
        .zip(snapshot.candidate_counts)
        .zip(ordinals)
    {
        if ordinal >= candidate_count {
            return Err(format!(
                "slot {slot_index} candidate ordinal {ordinal} is outside 0..{candidate_count}"
            ));
        }
        apply_bridge_ordinal(source, slot_index, &mut next_states[slot_index], ordinal)?;
    }
    *states = next_states;
    Ok(())
}

pub(super) fn collect_ready_actions(
    states: &[BridgeSlotState],
) -> Result<Vec<LearningActionV1>, String> {
    let mut actions = Vec::new();
    for state in states {
        match state {
            BridgeSlotState::Ready { action, .. } => actions.push(action.clone()),
            BridgeSlotState::Terminal => {}
            BridgeSlotState::Root | BridgeSlotState::Selection { .. } => {
                return Err("driver readiness changed while collecting actions".to_owned());
            }
        }
    }
    Ok(actions)
}

fn apply_bridge_ordinal(
    source: &impl BridgeDecisionSource,
    slot_index: usize,
    state: &mut BridgeSlotState,
    ordinal: usize,
) -> Result<(), String> {
    let next = match state.clone() {
        BridgeSlotState::Root => {
            let decision = source.bridge_root_decision(slot_index)?;
            match decision
                .choose(ordinal)
                .map_err(|error| error.to_string())?
            {
                LearningModelChoiceV1::Apply(action) => BridgeSlotState::Ready {
                    action,
                    decision_ordinals: vec![ordinal],
                },
                LearningModelChoiceV1::DecodeSelection(draft) => BridgeSlotState::Selection {
                    draft,
                    decision_ordinals: vec![ordinal],
                },
            }
        }
        BridgeSlotState::Selection {
            mut draft,
            mut decision_ordinals,
        } => {
            decision_ordinals.push(ordinal);
            match draft.choose(ordinal).map_err(|error| error.to_string())? {
                LearningSelectionStepV1::Continue => BridgeSlotState::Selection {
                    draft,
                    decision_ordinals,
                },
                LearningSelectionStepV1::Apply(action) => BridgeSlotState::Ready {
                    action,
                    decision_ordinals,
                },
            }
        }
        BridgeSlotState::Terminal | BridgeSlotState::Ready { .. } => {
            return Err(format!(
                "slot {slot_index} decision prefix continues after readiness"
            ));
        }
    };
    *state = next;
    Ok(())
}

pub(super) fn replay_bridge_state(
    source: &impl BridgeDecisionSource,
    slot_index: usize,
    decision_ordinals: &[usize],
) -> Result<BridgeSlotState, String> {
    let mut state = if source.bridge_is_terminal(slot_index)? {
        BridgeSlotState::Terminal
    } else {
        BridgeSlotState::Root
    };
    for ordinal in decision_ordinals {
        apply_bridge_ordinal(source, slot_index, &mut state, *ordinal)?;
    }
    Ok(state)
}

pub(super) fn states_from_source(
    source: &impl BridgeDecisionSource,
) -> Result<Vec<BridgeSlotState>, String> {
    (0..source.bridge_slot_count())
        .map(|slot_index| {
            Ok(if source.bridge_is_terminal(slot_index)? {
                BridgeSlotState::Terminal
            } else {
                BridgeSlotState::Root
            })
        })
        .collect()
}
