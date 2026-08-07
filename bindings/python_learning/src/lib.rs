use std::collections::BTreeSet;

use numpy::ndarray::Array2;
use numpy::{IntoPyArray, PyArray1};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use sts_oracle_eval::eval::run_control::{
    LearningActionV1, LearningBoundaryV1, LearningEnvPoolV1, LearningEnvV1, LearningModelChoiceV1,
    LearningModelDecisionV1, LearningModelObservationV1, LearningSelectionDraftV1,
    LearningSelectionStepV1, RunControlConfig, RunControlSessionCheckpointV1,
};

mod semantic;

use semantic::{
    ActionKind, CardZoneKind, CategoricalField, CombatActionKind, ContextKind, CounterItemKind,
    EnemyIdentityKind, IndexedChoiceCandidateKind, IndexedChoiceReasonKind, IntentKind,
    PublicCounterKind, RelationKind, RewardKind, ScalarField, SelectionCandidateKind,
    SelectionDomainKind, SelectionReasonKind, SemanticBatch, SemanticBatchBuilder,
    SemanticCompleteness, TokenKind, CARD_ID_VOCABULARY_SIZE, CATEGORICAL_VOCABULARY_SIZES,
    ENCOUNTER_ID_VOCABULARY_SIZE, ENEMY_ID_VOCABULARY_SIZE, EVENT_ID_VOCABULARY_SIZE,
    NO_CANDIDATE_TOKEN, POTION_ID_VOCABULARY_SIZE, POWER_ID_VOCABULARY_SIZE,
    RELIC_ID_VOCABULARY_SIZE, SEMANTIC_SCHEMA_VERSION,
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

/// Opaque, in-process exact state owned by the caller.
///
/// The bridge creates no automatic history and exposes no serialized session
/// payload. Retaining or discarding checkpoints is a curriculum decision.
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
struct LearningSlotCheckpoint {
    source_slot_index: usize,
    session: RunControlSessionCheckpointV1,
    bridge_state: BridgeSlotState,
}

/// Opaque collection used for one foreign-language call per recovery batch.
#[pyclass(skip_from_py_object)]
struct LearningCheckpointBatch {
    checkpoints: Vec<LearningSlotCheckpoint>,
}

#[pymethods]
impl LearningCheckpointBatch {
    fn __len__(&self) -> usize {
        self.checkpoints.len()
    }

    fn select(&self, slot_indices: Vec<usize>) -> PyResult<Self> {
        let mut seen = BTreeSet::new();
        let mut selected = Vec::with_capacity(slot_indices.len());
        for slot_index in slot_indices {
            if !seen.insert(slot_index) {
                return Err(PyValueError::new_err(format!(
                    "slot {slot_index} appears more than once in checkpoint selection"
                )));
            }
            let checkpoint = self
                .checkpoints
                .iter()
                .find(|checkpoint| checkpoint.source_slot_index == slot_index)
                .ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "checkpoint batch does not contain slot {slot_index}"
                    ))
                })?;
            selected.push(checkpoint.clone());
        }
        Ok(Self {
            checkpoints: selected,
        })
    }

    fn updated(&self, replacements: PyRef<'_, LearningCheckpointBatch>) -> PyResult<Self> {
        let mut updated = self.checkpoints.clone();
        for replacement in &replacements.checkpoints {
            let checkpoint = updated
                .iter_mut()
                .find(|checkpoint| checkpoint.source_slot_index == replacement.source_slot_index)
                .ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "checkpoint batch does not contain slot {}",
                        replacement.source_slot_index
                    ))
                })?;
            *checkpoint = replacement.clone();
        }
        Ok(Self {
            checkpoints: updated,
        })
    }
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

    #[pyo3(signature = (dense_mask=false, semantic=false))]
    fn decision_batch<'py>(
        &self,
        py: Python<'py>,
        dense_mask: bool,
        semantic: bool,
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
        if semantic {
            result.set_item("semantic", semantic_dict(py, self.semantic_snapshot()?)?)?;
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
                    .map(|(slot, _)| slot.slot_index)
                    .collect(),
            ),
        )?;
        result.set_item(
            "terminal_reward",
            PyArray1::from_vec(
                py,
                terminal_slots.iter().map(|(slot, _)| slot.reward).collect(),
            ),
        )?;
        result.set_item(
            "terminal_act",
            PyArray1::from_vec(
                py,
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.terminal_act)
                    .collect(),
            ),
        )?;
        for (key, values) in [
            (
                "terminal_floor",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.terminal_floor)
                    .collect(),
            ),
            (
                "terminal_hp",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.terminal_hp)
                    .collect(),
            ),
            (
                "terminal_max_hp",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.terminal_max_hp)
                    .collect(),
            ),
            (
                "terminal_gold",
                terminal_slots
                    .iter()
                    .map(|(_, outcome)| outcome.terminal_gold)
                    .collect(),
            ),
        ] {
            result.set_item(key, PyArray1::from_vec(py, values))?;
        }
        Ok(result)
    }

    fn reset_slot(&mut self, slot_index: usize, seed: u64) -> PyResult<()> {
        self.reset_slots(vec![slot_index], vec![seed])
    }

    fn reset_slots(&mut self, slot_indices: Vec<usize>, seeds: Vec<u64>) -> PyResult<()> {
        if slot_indices.len() != seeds.len() {
            return Err(PyValueError::new_err(format!(
                "expected {} reset seeds, received {}",
                slot_indices.len(),
                seeds.len()
            )));
        }
        let mut seen = BTreeSet::new();
        for slot_index in &slot_indices {
            if !seen.insert(*slot_index) {
                return Err(PyValueError::new_err(format!(
                    "slot {slot_index} appears more than once in reset batch"
                )));
            }
            if !matches!(
                self.states.get(*slot_index),
                Some(BridgeSlotState::Terminal)
            ) {
                return Err(PyValueError::new_err(format!(
                    "slot {slot_index} is not terminal"
                )));
            }
        }
        self.pool
            .reset_slots(
                slot_indices
                    .iter()
                    .copied()
                    .zip(seeds)
                    .map(|(slot_index, seed)| {
                        (
                            slot_index,
                            RunControlConfig {
                                seed,
                                ..RunControlConfig::default()
                            },
                        )
                    }),
            )
            .map_err(runtime_error)?;
        for slot_index in slot_indices {
            self.states[slot_index] = BridgeSlotState::Root;
        }
        Ok(())
    }

    fn checkpoint_slot(&self, slot_index: usize) -> PyResult<LearningSlotCheckpoint> {
        let bridge_state = self.states.get(slot_index).cloned().ok_or_else(|| {
            PyValueError::new_err(format!(
                "slot {slot_index} is outside 0..{}",
                self.states.len()
            ))
        })?;
        let session = self
            .pool
            .checkpoint_slot(slot_index)
            .map_err(runtime_error)?;
        Ok(LearningSlotCheckpoint {
            source_slot_index: slot_index,
            session,
            bridge_state,
        })
    }

    fn checkpoint_slots(&self, slot_indices: Vec<usize>) -> PyResult<LearningCheckpointBatch> {
        let mut seen = BTreeSet::new();
        let mut checkpoints = Vec::with_capacity(slot_indices.len());
        for slot_index in slot_indices {
            if !seen.insert(slot_index) {
                return Err(PyValueError::new_err(format!(
                    "slot {slot_index} appears more than once in checkpoint batch"
                )));
            }
            checkpoints.push(self.checkpoint_slot(slot_index)?);
        }
        Ok(LearningCheckpointBatch { checkpoints })
    }

    fn restore_slot(
        &mut self,
        slot_index: usize,
        checkpoint: PyRef<'_, LearningSlotCheckpoint>,
    ) -> PyResult<()> {
        if slot_index >= self.states.len() {
            return Err(PyValueError::new_err(format!(
                "slot {slot_index} is outside 0..{}",
                self.states.len()
            )));
        }
        if checkpoint.source_slot_index != slot_index {
            return Err(PyValueError::new_err(format!(
                "checkpoint belongs to slot {}, not slot {slot_index}",
                checkpoint.source_slot_index
            )));
        }
        self.restore_slot_checkpoint(slot_index, &checkpoint)
            .map_err(runtime_error)
    }

    fn restore_slots(
        &mut self,
        slot_indices: Vec<usize>,
        checkpoints: PyRef<'_, LearningCheckpointBatch>,
    ) -> PyResult<()> {
        if slot_indices.len() != checkpoints.checkpoints.len() {
            return Err(PyValueError::new_err(format!(
                "expected {} target slots, received {}",
                checkpoints.checkpoints.len(),
                slot_indices.len()
            )));
        }
        let mut seen = BTreeSet::new();
        let mut replacements = Vec::with_capacity(slot_indices.len());
        for (slot_index, checkpoint) in slot_indices.iter().copied().zip(&checkpoints.checkpoints) {
            if slot_index >= self.states.len() {
                return Err(PyValueError::new_err(format!(
                    "slot {slot_index} is outside 0..{}",
                    self.states.len()
                )));
            }
            if !seen.insert(slot_index) {
                return Err(PyValueError::new_err(format!(
                    "slot {slot_index} appears more than once in restore batch"
                )));
            }
            if checkpoint.source_slot_index != slot_index {
                return Err(PyValueError::new_err(format!(
                    "checkpoint belongs to slot {}, not slot {slot_index}",
                    checkpoint.source_slot_index
                )));
            }
            let env =
                LearningEnvV1::from_checkpoint(checkpoint.session.clone()).map_err(value_error)?;
            replacements.push((slot_index, env));
        }
        self.pool
            .replace_slots(replacements)
            .map_err(runtime_error)?;
        for (slot_index, checkpoint) in slot_indices.into_iter().zip(&checkpoints.checkpoints) {
            self.states[slot_index] = checkpoint.bridge_state.clone();
        }
        Ok(())
    }
}

impl LearningBatchEnv {
    fn restore_slot_checkpoint(
        &mut self,
        slot_index: usize,
        checkpoint: &LearningSlotCheckpoint,
    ) -> Result<(), String> {
        if slot_index >= self.states.len() {
            return Err(format!(
                "slot {slot_index} is outside 0..{}",
                self.states.len()
            ));
        }
        if checkpoint.source_slot_index != slot_index {
            return Err(format!(
                "checkpoint belongs to slot {}, not slot {slot_index}",
                checkpoint.source_slot_index
            ));
        }
        let env = LearningEnvV1::from_checkpoint(checkpoint.session.clone())?;
        self.pool
            .replace_slot(slot_index, env)
            .map_err(|error| error.to_string())?;
        self.states[slot_index] = checkpoint.bridge_state.clone();
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

    fn semantic_snapshot(&self) -> PyResult<SemanticBatch> {
        let mut builder = SemanticBatchBuilder::new();
        for (slot_index, state) in self.states.iter().enumerate() {
            match state {
                BridgeSlotState::Root => {
                    let boundary = self.pool.boundary(slot_index).ok_or_else(|| {
                        PyRuntimeError::new_err(format!("missing pool slot {slot_index}"))
                    })?;
                    let decision =
                        LearningModelDecisionV1::from_boundary(boundary).map_err(value_error)?;
                    builder.push_decision(&decision).map_err(runtime_error)?;
                }
                BridgeSlotState::Selection(draft) => {
                    let boundary = self.pool.boundary(slot_index).ok_or_else(|| {
                        PyRuntimeError::new_err(format!("missing pool slot {slot_index}"))
                    })?;
                    let decision =
                        LearningModelDecisionV1::from_boundary(boundary).map_err(value_error)?;
                    builder
                        .push_selection(decision.observation, draft)
                        .map_err(runtime_error)?;
                }
                BridgeSlotState::Terminal | BridgeSlotState::Ready(_) => {}
            }
        }
        Ok(builder.finish())
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

fn semantic_dict(py: Python<'_>, batch: SemanticBatch) -> PyResult<Bound<'_, PyDict>> {
    let result = PyDict::new(py);
    result.set_item("schema_version", SEMANTIC_SCHEMA_VERSION)?;
    result.set_item("completeness", PyArray1::from_vec(py, batch.completeness))?;

    let token = PyDict::new(py);
    token.set_item("row_splits", PyArray1::from_vec(py, batch.token_row_splits))?;
    token.set_item("kind", PyArray1::from_vec(py, batch.token_kinds))?;
    result.set_item("token", token)?;

    let categorical = PyDict::new(py);
    categorical.set_item(
        "token_indices",
        PyArray1::from_vec(py, batch.categorical.token_indices),
    )?;
    categorical.set_item("field", PyArray1::from_vec(py, batch.categorical.fields))?;
    categorical.set_item("value", PyArray1::from_vec(py, batch.categorical.values))?;
    result.set_item("categorical", categorical)?;

    let scalar = PyDict::new(py);
    scalar.set_item(
        "token_indices",
        PyArray1::from_vec(py, batch.scalar.token_indices),
    )?;
    scalar.set_item("field", PyArray1::from_vec(py, batch.scalar.fields))?;
    scalar.set_item("value", PyArray1::from_vec(py, batch.scalar.values))?;
    result.set_item("scalar", scalar)?;

    let relation = PyDict::new(py);
    relation.set_item(
        "source_token_indices",
        PyArray1::from_vec(py, batch.relation.source_token_indices),
    )?;
    relation.set_item("relation", PyArray1::from_vec(py, batch.relation.relations))?;
    relation.set_item(
        "target_token_indices",
        PyArray1::from_vec(py, batch.relation.target_token_indices),
    )?;
    result.set_item("relation", relation)?;
    result.set_item(
        "candidate_token_indices",
        PyArray1::from_vec(py, batch.candidate_token_indices),
    )?;
    Ok(result)
}

#[pyfunction]
fn semantic_schema(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    let result = PyDict::new(py);
    result.set_item("version", SEMANTIC_SCHEMA_VERSION)?;
    result.set_item(
        "completeness",
        numeric_schema_dict(py, SemanticCompleteness::SCHEMA)?,
    )?;
    result.set_item("token_kind", numeric_schema_dict(py, TokenKind::SCHEMA)?)?;
    result.set_item(
        "categorical_field",
        numeric_schema_dict(py, CategoricalField::SCHEMA)?,
    )?;
    result.set_item(
        "scalar_field",
        numeric_schema_dict(py, ScalarField::SCHEMA)?,
    )?;
    result.set_item(
        "relation_kind",
        numeric_schema_dict(py, RelationKind::SCHEMA)?,
    )?;
    result.set_item(
        "context_kind",
        numeric_schema_dict(py, ContextKind::SCHEMA)?,
    )?;
    result.set_item("action_kind", numeric_schema_dict(py, ActionKind::SCHEMA)?)?;
    result.set_item("reward_kind", numeric_schema_dict(py, RewardKind::SCHEMA)?)?;
    result.set_item(
        "combat_action_kind",
        numeric_schema_dict(py, CombatActionKind::SCHEMA)?,
    )?;
    result.set_item("intent_kind", numeric_schema_dict(py, IntentKind::SCHEMA)?)?;
    result.set_item(
        "enemy_identity_kind",
        numeric_schema_dict(py, EnemyIdentityKind::SCHEMA)?,
    )?;
    result.set_item(
        "public_counter_kind",
        numeric_schema_dict(py, PublicCounterKind::SCHEMA)?,
    )?;
    result.set_item(
        "card_zone_kind",
        numeric_schema_dict(py, CardZoneKind::SCHEMA)?,
    )?;
    result.set_item(
        "indexed_choice_reason_kind",
        numeric_schema_dict(py, IndexedChoiceReasonKind::SCHEMA)?,
    )?;
    result.set_item(
        "indexed_choice_candidate_kind",
        numeric_schema_dict(py, IndexedChoiceCandidateKind::SCHEMA)?,
    )?;
    result.set_item(
        "selection_reason_kind",
        numeric_schema_dict(py, SelectionReasonKind::SCHEMA)?,
    )?;
    result.set_item(
        "selection_candidate_kind",
        numeric_schema_dict(py, SelectionCandidateKind::SCHEMA)?,
    )?;
    result.set_item(
        "selection_domain_kind",
        numeric_schema_dict(py, SelectionDomainKind::SCHEMA)?,
    )?;
    result.set_item(
        "counter_item_kind",
        numeric_schema_dict(py, CounterItemKind::SCHEMA)?,
    )?;

    let vocabulary_sizes = PyDict::new(py);
    for (field, size) in CATEGORICAL_VOCABULARY_SIZES {
        vocabulary_sizes.set_item(*field, *size)?;
    }
    result.set_item("categorical_vocabulary_size", vocabulary_sizes)?;

    let domains = PyDict::new(py);
    domains.set_item("card_id", CARD_ID_VOCABULARY_SIZE)?;
    domains.set_item("relic_id", RELIC_ID_VOCABULARY_SIZE)?;
    domains.set_item("potion_id", POTION_ID_VOCABULARY_SIZE)?;
    domains.set_item("encounter_id", ENCOUNTER_ID_VOCABULARY_SIZE)?;
    domains.set_item("event_id", EVENT_ID_VOCABULARY_SIZE)?;
    domains.set_item("enemy_id", ENEMY_ID_VOCABULARY_SIZE)?;
    domains.set_item("power_id", POWER_ID_VOCABULARY_SIZE)?;
    result.set_item("domain_vocabulary_size", domains)?;
    Ok(result)
}

fn numeric_schema_dict<'py>(
    py: Python<'py>,
    entries: &[(&str, i64)],
) -> PyResult<Bound<'py, PyDict>> {
    let result = PyDict::new(py);
    for (name, value) in entries {
        result.set_item(*name, *value)?;
    }
    Ok(result)
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
    module.add_class::<LearningCheckpointBatch>()?;
    module.add_class::<LearningSlotCheckpoint>()?;
    module.add_function(wrap_pyfunction!(semantic_schema, module)?)?;
    module.add("PHASE_STRATEGIC_ROOT", PHASE_STRATEGIC_ROOT)?;
    module.add("PHASE_COMBAT_ROOT", PHASE_COMBAT_ROOT)?;
    module.add("PHASE_SELECTION", PHASE_SELECTION)?;
    module.add("SEMANTIC_SCHEMA_VERSION", SEMANTIC_SCHEMA_VERSION)?;
    module.add(
        "SEMANTIC_NOT_ENCODED",
        SemanticCompleteness::NotEncoded as u8,
    )?;
    module.add("SEMANTIC_COMPLETE", SemanticCompleteness::Complete as u8)?;
    module.add("SEMANTIC_NO_CANDIDATE_TOKEN", NO_CANDIDATE_TOKEN)?;
    module.add("SEMANTIC_TOKEN_CANDIDATE", TokenKind::Candidate as u16)?;
    module.add(
        "SEMANTIC_RELATION_OBSERVATION_HAS_CANDIDATE",
        RelationKind::ObservationHasCandidate as u16,
    )?;
    module.add(
        "SEMANTIC_RELATION_CANDIDATE_TARGETS",
        RelationKind::CandidateTargets as u16,
    )?;
    Ok(())
}

#[cfg(test)]
mod checkpoint_tests {
    use sts_oracle_eval::content::cards::CardId;
    use sts_oracle_eval::eval::run_control::{LearningEnvV1, RunControlSession};
    use sts_oracle_eval::runtime::combat::CombatCard;
    use sts_oracle_eval::state::core::{
        ActiveCombat, CombatContext, EngineState, PendingChoice, RoomCombatContext,
    };
    use sts_oracle_eval::state::map::node::RoomType;

    use super::*;

    #[test]
    fn opaque_checkpoint_restores_unfinished_symbolic_prefix() {
        // A real Scry boundary is the smallest fixture whose exact state spans
        // both the simulator checkpoint and bridge-only decoder progress.
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = sts_oracle_eval::test_support::blank_test_combat();
        combat.zones.draw_pile = (vec![
            CombatCard::new(CardId::Strike, 11),
            CombatCard::new(CardId::Defend, 12),
        ])
        .into();
        let choice = PendingChoice::ScrySelect {
            cards: vec![CardId::Strike, CardId::Defend],
            card_uuids: vec![11, 12],
        };
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let pool = LearningEnvPoolV1::from_envs([LearningEnvV1::from_session(session)])
            .expect("create selection pool");
        let mut env = LearningBatchEnv {
            states: states_from_pool(&pool),
            pool,
        };

        let boundary = env.pool.boundary(0).expect("selection boundary");
        let decision =
            LearningModelDecisionV1::from_boundary(boundary).expect("build symbolic root decision");
        let LearningModelChoiceV1::DecodeSelection(mut draft) =
            decision.choose(0).expect("start symbolic selection")
        else {
            panic!("scry root must start a symbolic decoder");
        };
        assert!(matches!(
            draft.choose(1).expect("append first scry card"),
            LearningSelectionStepV1::Continue
        ));
        let checkpoint = LearningSlotCheckpoint {
            source_slot_index: 0,
            session: env.pool.checkpoint_slot(0).expect("checkpoint prefix"),
            bridge_state: BridgeSlotState::Selection(draft.clone()),
        };
        let LearningSelectionStepV1::Apply(action) = draft.choose(0).expect("submit prefix") else {
            panic!("submit must produce an action");
        };
        env.states[0] = BridgeSlotState::Ready(action);
        assert!(matches!(env.states[0], BridgeSlotState::Ready(_)));

        env.restore_slot_checkpoint(0, &checkpoint)
            .expect("restore prefix");
        let BridgeSlotState::Selection(draft) = &env.states[0] else {
            panic!("restored slot must resume symbolic selection");
        };
        assert_eq!(draft.selected_domain_indices(), &[0]);
        assert_eq!(draft.decision().candidates.len(), 2);
    }
}
