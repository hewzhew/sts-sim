use numpy::ndarray::Array2;
use numpy::{IntoPyArray, PyArray1};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sts_oracle_eval::eval::run_control::{
    LearningActionV1, LearningBoundaryV1, LearningEnvPoolV1, LearningModelChoiceV1,
    LearningModelDecisionV1, LearningModelObservationV1, LearningSelectionDraftV1,
    LearningSelectionStepV1, RunControlConfig,
};

const PHASE_STRATEGIC_ROOT: u8 = 0;
const PHASE_COMBAT_ROOT: u8 = 1;
const PHASE_SELECTION: u8 = 2;

#[derive(Clone, Debug)]
enum BridgeSlotState {
    Terminal,
    Root,
    Selection(LearningSelectionDraftV1),
    Ready(LearningActionV1),
}

#[derive(Debug)]
struct DecisionSnapshot {
    slot_indices: Vec<usize>,
    phases: Vec<u8>,
    candidate_counts: Vec<usize>,
    candidate_row_splits: Vec<usize>,
}

#[pyclass]
struct LearningBatchEnv {
    pool: LearningEnvPoolV1,
    states: Vec<BridgeSlotState>,
}

#[pymethods]
impl LearningBatchEnv {
    #[new]
    fn new(seeds: Vec<u64>) -> PyResult<Self> {
        let pool =
            LearningEnvPoolV1::from_configs(seeds.into_iter().map(|seed| RunControlConfig {
                seed,
                ..RunControlConfig::default()
            }))
            .map_err(runtime_error)?;
        let states = states_from_pool(&pool);
        Ok(Self { pool, states })
    }

    #[getter]
    fn slot_count(&self) -> usize {
        self.pool.slot_count()
    }

    #[getter]
    fn terminal_count(&self) -> usize {
        self.pool.terminal_count()
    }

    #[getter]
    fn ready(&self) -> bool {
        self.states
            .iter()
            .all(|state| matches!(state, BridgeSlotState::Terminal | BridgeSlotState::Ready(_)))
    }

    #[pyo3(signature = (dense_mask=false))]
    fn decision_batch<'py>(
        &self,
        py: Python<'py>,
        dense_mask: bool,
    ) -> PyResult<Bound<'py, PyDict>> {
        let snapshot = self.decision_snapshot()?;
        let result = PyDict::new(py);
        result.set_item(
            "slot_indices",
            usize_array(py, snapshot.slot_indices.clone()),
        )?;
        result.set_item("phase", PyArray1::from_vec(py, snapshot.phases))?;
        result.set_item(
            "candidate_counts",
            usize_array(py, snapshot.candidate_counts.clone()),
        )?;
        result.set_item(
            "candidate_row_splits",
            usize_array(py, snapshot.candidate_row_splits),
        )?;
        if dense_mask {
            let width = snapshot.candidate_counts.iter().copied().max().unwrap_or(0);
            let mut values = vec![false; snapshot.candidate_counts.len().saturating_mul(width)];
            for (row, count) in snapshot.candidate_counts.iter().copied().enumerate() {
                values[row * width..row * width + count].fill(true);
            }
            let mask = Array2::from_shape_vec((snapshot.candidate_counts.len(), width), values)
                .map_err(|error| PyRuntimeError::new_err(error.to_string()))?;
            result.set_item("dense_action_mask", mask.into_pyarray(py))?;
        }
        Ok(result)
    }

    fn choose(&mut self, ordinals: Vec<usize>) -> PyResult<()> {
        let snapshot = self.decision_snapshot()?;
        if ordinals.len() != snapshot.slot_indices.len() {
            return Err(PyValueError::new_err(format!(
                "expected {} candidate ordinals, received {}",
                snapshot.slot_indices.len(),
                ordinals.len()
            )));
        }

        let mut next_states = self.states.clone();
        for ((slot_index, candidate_count), ordinal) in snapshot
            .slot_indices
            .into_iter()
            .zip(snapshot.candidate_counts)
            .zip(ordinals)
        {
            if ordinal >= candidate_count {
                return Err(PyValueError::new_err(format!(
                    "slot {slot_index} candidate ordinal {ordinal} is outside 0..{candidate_count}"
                )));
            }
            match &mut next_states[slot_index] {
                BridgeSlotState::Root => {
                    let boundary = self.pool.boundary(slot_index).ok_or_else(|| {
                        PyRuntimeError::new_err(format!("missing pool slot {slot_index}"))
                    })?;
                    let decision =
                        LearningModelDecisionV1::from_boundary(boundary).map_err(value_error)?;
                    next_states[slot_index] = match decision.choose(ordinal).map_err(value_error)? {
                        LearningModelChoiceV1::Apply(action) => BridgeSlotState::Ready(action),
                        LearningModelChoiceV1::DecodeSelection(draft) => {
                            BridgeSlotState::Selection(draft)
                        }
                    };
                }
                BridgeSlotState::Selection(draft) => {
                    if let LearningSelectionStepV1::Apply(action) =
                        draft.choose(ordinal).map_err(value_error)?
                    {
                        next_states[slot_index] = BridgeSlotState::Ready(action);
                    }
                }
                BridgeSlotState::Terminal | BridgeSlotState::Ready(_) => {
                    return Err(PyRuntimeError::new_err(format!(
                        "slot {slot_index} appeared in a decision batch without a pending decision"
                    )));
                }
            }
        }
        self.states = next_states;
        Ok(())
    }

    fn step<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        if !self.ready() {
            return Err(PyValueError::new_err(
                "all active slots must finish root and selection decisions before step",
            ));
        }
        let mut actions = Vec::with_capacity(self.pool.active_count());
        for state in &self.states {
            match state {
                BridgeSlotState::Ready(action) => actions.push(action.clone()),
                BridgeSlotState::Terminal => {}
                BridgeSlotState::Root | BridgeSlotState::Selection(_) => {
                    return Err(PyRuntimeError::new_err(
                        "driver readiness changed while collecting actions",
                    ));
                }
            }
        }
        let step = self.pool.step_active(actions).map_err(runtime_error)?;
        self.states = states_from_pool(&self.pool);

        let result = PyDict::new(py);
        result.set_item(
            "slot_indices",
            usize_array(py, step.slots.iter().map(|slot| slot.slot_index).collect()),
        )?;
        result.set_item(
            "reward",
            PyArray1::from_vec(py, step.slots.iter().map(|slot| slot.reward).collect()),
        )?;
        result.set_item(
            "terminated",
            PyArray1::from_vec(py, step.slots.iter().map(|slot| slot.terminated).collect()),
        )?;
        Ok(result)
    }

    fn reset_slot(&mut self, slot_index: usize, seed: u64) -> PyResult<()> {
        if !matches!(self.states.get(slot_index), Some(BridgeSlotState::Terminal)) {
            return Err(PyValueError::new_err(format!(
                "slot {slot_index} is not terminal"
            )));
        }
        self.pool
            .reset_slot(
                slot_index,
                RunControlConfig {
                    seed,
                    ..RunControlConfig::default()
                },
            )
            .map_err(runtime_error)?;
        self.states[slot_index] = BridgeSlotState::Root;
        Ok(())
    }
}

impl LearningBatchEnv {
    fn decision_snapshot(&self) -> PyResult<DecisionSnapshot> {
        let mut slot_indices = Vec::new();
        let mut phases = Vec::new();
        let mut candidate_counts = Vec::new();
        let mut candidate_row_splits = vec![0];

        for (slot_index, state) in self.states.iter().enumerate() {
            let (phase, candidate_count) = match state {
                BridgeSlotState::Root => {
                    let boundary = self.pool.boundary(slot_index).ok_or_else(|| {
                        PyRuntimeError::new_err(format!("missing pool slot {slot_index}"))
                    })?;
                    let decision =
                        LearningModelDecisionV1::from_boundary(boundary).map_err(value_error)?;
                    let phase = match decision.observation {
                        LearningModelObservationV1::Strategic(_) => PHASE_STRATEGIC_ROOT,
                        LearningModelObservationV1::Combat(_) => PHASE_COMBAT_ROOT,
                    };
                    (phase, decision.candidates.len())
                }
                BridgeSlotState::Selection(draft) => {
                    (PHASE_SELECTION, draft.decision().candidates.len())
                }
                BridgeSlotState::Terminal | BridgeSlotState::Ready(_) => continue,
            };
            if candidate_count == 0 {
                return Err(PyRuntimeError::new_err(format!(
                    "slot {slot_index} exposed an empty decision row"
                )));
            }
            let next = candidate_row_splits
                .last()
                .copied()
                .unwrap_or(0usize)
                .checked_add(candidate_count)
                .ok_or_else(|| PyRuntimeError::new_err("candidate count overflow"))?;
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
}

fn states_from_pool(pool: &LearningEnvPoolV1) -> Vec<BridgeSlotState> {
    (0..pool.slot_count())
        .map(|slot_index| match pool.boundary(slot_index) {
            Some(LearningBoundaryV1::Terminal { .. }) => BridgeSlotState::Terminal,
            Some(_) => BridgeSlotState::Root,
            None => unreachable!("pool slot count and boundary lookup diverged"),
        })
        .collect()
}

fn usize_array(py: Python<'_>, values: Vec<usize>) -> Bound<'_, PyArray1<u64>> {
    PyArray1::from_vec(
        py,
        values
            .into_iter()
            .map(|value| u64::try_from(value).unwrap_or(u64::MAX))
            .collect(),
    )
}

fn value_error(error: impl ToString) -> PyErr {
    PyValueError::new_err(error.to_string())
}

fn runtime_error(error: impl ToString) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<LearningBatchEnv>()?;
    module.add("PHASE_STRATEGIC_ROOT", PHASE_STRATEGIC_ROOT)?;
    module.add("PHASE_COMBAT_ROOT", PHASE_COMBAT_ROOT)?;
    module.add("PHASE_SELECTION", PHASE_SELECTION)?;
    Ok(())
}
