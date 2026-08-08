//! Python projection of one exact same-root combat replicate group.
//!
//! Ordinal decoding and semantic encoding stay in the shared bridge driver. This module owns
//! only combat-group construction and typed terminal columns; it has no policy or objective.

use numpy::PyArray1;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sts_oracle_eval::eval::run_control::{CombatLearningEnvPoolV1, CombatLearningRootV1};
use sts_oracle_eval::sim::combat::CombatTerminal;

use super::{
    bridge_states_ready, choose_bridge_ordinals, collect_ready_actions, decision_batch_dict,
    decision_snapshot_from_source, runtime_error, semantic_snapshot_from_source,
    states_from_source, usize_array, value_error, BridgeSlotState,
};

pub(super) const COMBAT_TERMINAL_WIN: u8 = 0;
pub(super) const COMBAT_TERMINAL_LOSS: u8 = 1;
pub(super) const COMBAT_TERMINAL_UNRESOLVED: u8 = 2;

/// Same-root combat replicates created from one exact live run-control slot.
///
/// This class deliberately has no public constructor: a combat group must be derived from a
/// typed `LearningBatchEnv` combat boundary so Python never reconstructs simulator state.
#[pyclass]
pub(super) struct CombatLearningBatchEnv {
    pool: CombatLearningEnvPoolV1,
    states: Vec<BridgeSlotState>,
}

#[pymethods]
impl CombatLearningBatchEnv {
    #[getter]
    fn root_id(&self) -> String {
        self.pool.root_identity().root_id.clone()
    }

    #[getter]
    fn exact_combat_state_hash(&self) -> String {
        self.pool.root_identity().exact_combat_state_hash.clone()
    }

    #[getter]
    fn replicate_count(&self) -> usize {
        self.pool.replicate_count()
    }

    #[getter]
    fn terminal_count(&self) -> usize {
        self.pool.terminal_count()
    }

    #[getter]
    fn ready(&self) -> bool {
        bridge_states_ready(&self.states)
    }

    #[pyo3(signature = (dense_mask=false, semantic=false))]
    fn decision_batch<'py>(
        &self,
        py: Python<'py>,
        dense_mask: bool,
        semantic: bool,
    ) -> PyResult<Bound<'py, PyDict>> {
        let snapshot =
            decision_snapshot_from_source(&self.pool, &self.states).map_err(runtime_error)?;
        let semantic = semantic
            .then(|| semantic_snapshot_from_source(&self.pool, &self.states))
            .transpose()
            .map_err(runtime_error)?;
        decision_batch_dict(py, snapshot, dense_mask, semantic)
    }

    fn choose(&mut self, ordinals: Vec<usize>) -> PyResult<()> {
        choose_bridge_ordinals(&self.pool, &mut self.states, ordinals).map_err(value_error)
    }

    fn step<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        if !self.ready() {
            return Err(PyValueError::new_err(
                "all active combat replicates must finish root and selection decisions before step",
            ));
        }
        let actions = collect_ready_actions(&self.states).map_err(runtime_error)?;
        let step = self.pool.step_active(actions).map_err(runtime_error)?;
        self.states = states_from_source(&self.pool).map_err(runtime_error)?;

        let result = PyDict::new(py);
        result.set_item(
            "slot_indices",
            usize_array(
                py,
                step.slots
                    .iter()
                    .map(|slot| slot.replicate_index as usize)
                    .collect(),
            ),
        )?;
        result.set_item(
            "terminated",
            PyArray1::from_vec(py, step.slots.iter().map(|slot| slot.terminated).collect()),
        )?;
        let terminal_slots = step
            .slots
            .iter()
            .filter_map(|slot| {
                slot.terminal_outcome
                    .as_ref()
                    .map(|outcome| (slot, outcome))
            })
            .collect::<Vec<_>>();
        result.set_item(
            "terminal_slot_indices",
            usize_array(
                py,
                terminal_slots
                    .iter()
                    .map(|(slot, _)| slot.replicate_index as usize)
                    .collect(),
            ),
        )?;
        result.set_item(
            "terminal_kind",
            PyArray1::from_vec(
                py,
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| combat_terminal_code(outcome.combat.terminal))
                    .collect(),
            ),
        )?;
        for (key, values) in [
            (
                "terminal_start_hp",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.combat.start_hp)
                    .collect(),
            ),
            (
                "terminal_final_hp",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.combat.final_hp)
                    .collect(),
            ),
            (
                "terminal_hp_loss",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.combat.hp_loss)
                    .collect(),
            ),
        ] {
            result.set_item(key, PyArray1::from_vec(py, values))?;
        }
        for (key, values) in [
            (
                "terminal_turns",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.combat.turns)
                    .collect(),
            ),
            (
                "terminal_potions_used",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.combat.potions_used)
                    .collect(),
            ),
            (
                "terminal_potions_discarded",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.combat.potions_discarded)
                    .collect(),
            ),
            (
                "terminal_cards_played",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.combat.cards_played)
                    .collect(),
            ),
        ] {
            result.set_item(key, PyArray1::from_vec(py, values))?;
        }
        Ok(result)
    }
}

impl CombatLearningBatchEnv {
    pub(super) fn from_root(
        root: &CombatLearningRootV1,
        replicate_count: usize,
    ) -> Result<Self, String> {
        let pool = CombatLearningEnvPoolV1::from_root(root, replicate_count)
            .map_err(|error| error.to_string())?;
        let states = states_from_source(&pool)?;
        Ok(Self { pool, states })
    }
}

fn combat_terminal_code(terminal: CombatTerminal) -> u8 {
    match terminal {
        CombatTerminal::Win => COMBAT_TERMINAL_WIN,
        CombatTerminal::Loss => COMBAT_TERMINAL_LOSS,
        CombatTerminal::Unresolved => COMBAT_TERMINAL_UNRESOLVED,
    }
}
