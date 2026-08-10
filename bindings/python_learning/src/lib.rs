use std::collections::BTreeSet;
use std::io::{self, Cursor, Write};

use numpy::ndarray::Array2;
use numpy::{IntoPyArray, PyArray1};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use serde::{Deserialize, Serialize};
use sts_oracle_eval::eval::run_control::{
    capture_planner_boundary_yield_v1, CombatLearningPotionPolicyV1,
    CombatLearningRootBatchArtifactV1, CombatLearningRootV1, LearningActionV1,
    LearningBoundaryKindV1, LearningBoundaryV1, LearningEnvPoolV1, LearningEnvV1,
    LearningModelDecisionV1, LearningPublicRunContextV1, LearningSelectionDraftV1,
    PlannerBoundaryYieldKindV1, RunControlConfig, RunControlSessionCheckpointV1,
};

mod bridge_decision;
mod combat_batch;
mod semantic;

use bridge_decision::{
    bridge_states_ready, choose_bridge_ordinals, collect_ready_actions,
    combat_decision_audit_json_from_source, decision_snapshot_from_source, replay_bridge_state,
    semantic_snapshot_from_source, states_from_source, strategic_decision_audit_json_from_source,
    LearningBatchDecisionSource,
};
use combat_batch::{
    potion_id_names, CombatLearningBatchEnv, CombatLearningDecisionProgressV1,
    CombatLearningRecoveryRoot, PyCombatLearningRootContextV1,
    COMBAT_TERMINAL_LOSS, COMBAT_TERMINAL_UNRESOLVED, COMBAT_TERMINAL_WIN,
};

use semantic::{
    ActionKind, CardZoneKind, CategoricalField, CombatActionKind, ContextKind, CounterItemKind,
    EnemyIdentityKind, IndexedChoiceCandidateKind, IndexedChoiceReasonKind, IntentKind,
    PublicCounterKind, RelationKind, RewardKind, ScalarField, SelectionCandidateKind,
    SelectionDomainKind, SelectionReasonKind, SemanticBatch, SemanticCompleteness, TokenKind,
    CARD_ID_VOCABULARY_SIZE, CATEGORICAL_VOCABULARY_SIZES, ENCOUNTER_ID_VOCABULARY_SIZE,
    ENEMY_ID_VOCABULARY_SIZE, EVENT_ID_VOCABULARY_SIZE, NO_CANDIDATE_TOKEN,
    POTION_ID_VOCABULARY_SIZE, POWER_ID_VOCABULARY_SIZE, RELIC_ID_VOCABULARY_SIZE,
    SEMANTIC_SCHEMA_VERSION,
};

const PHASE_STRATEGIC_ROOT: u8 = 0;
const PHASE_COMBAT_ROOT: u8 = 1;
const PHASE_SELECTION: u8 = 2;
const RUN_BOUNDARY_STRATEGIC: u8 = 0;
const RUN_BOUNDARY_COMBAT: u8 = 1;
const RUN_BOUNDARY_TERMINAL: u8 = 2;
const RUN_BOUNDARY_UNSUPPORTED: u8 = 3;
const BATCH_CHECKPOINT_MAGIC: &[u8] = b"STS-LEARNING-BATCH\0";
const BATCH_CHECKPOINT_VERSION: u32 = 2;
const CHECKPOINT_BANK_MAGIC: &[u8] = b"STS-LEARNING-BANK\0";
const CHECKPOINT_BANK_VERSION: u32 = 2;
const MAX_EXPORTED_COMBAT_ROOTS: usize = 64;

#[derive(Clone, Debug)]
enum BridgeSlotState {
    Terminal,
    Root,
    Selection {
        draft: LearningSelectionDraftV1,
        decision_ordinals: Vec<usize>,
    },
    Ready {
        action: LearningActionV1,
        decision_ordinals: Vec<usize>,
    },
}

impl BridgeSlotState {
    fn decision_ordinals(&self) -> &[usize] {
        match self {
            Self::Terminal | Self::Root => &[],
            Self::Selection {
                decision_ordinals, ..
            }
            | Self::Ready {
                decision_ordinals, ..
            } => decision_ordinals,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SerializedLearningBatchCheckpointV1 {
    slots: Vec<SerializedLearningSlotCheckpointV1>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SerializedLearningSlotCheckpointV1 {
    source_slot_index: usize,
    session: RunControlSessionCheckpointV1,
    decision_ordinals: Vec<usize>,
}

struct BoundedCheckpointWriter {
    bytes: Vec<u8>,
    max_bytes: usize,
}

impl BoundedCheckpointWriter {
    fn new(magic: &[u8], version: u32, max_bytes: usize) -> Result<Self, String> {
        let header_bytes = magic.len() + std::mem::size_of::<u32>();
        if max_bytes < header_bytes {
            return Err("checkpoint byte limit is smaller than its header".to_owned());
        }
        let mut bytes = Vec::with_capacity(max_bytes.min(64 * 1024));
        bytes.extend_from_slice(magic);
        bytes.extend_from_slice(&version.to_be_bytes());
        Ok(Self { bytes, max_bytes })
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedCheckpointWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let next = self
            .bytes
            .len()
            .checked_add(buffer.len())
            .ok_or_else(|| io::Error::other("batch checkpoint byte count overflow"))?;
        if next > self.max_bytes {
            return Err(io::Error::other(
                "batch checkpoint exceeds its caller-provided byte limit",
            ));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
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

    #[pyo3(signature = (*, max_bytes))]
    fn checkpoint_bytes<'py>(
        &self,
        py: Python<'py>,
        max_bytes: usize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let payload = self
            .encode_cross_process_checkpoint(max_bytes)
            .map_err(value_error)?;
        Ok(PyBytes::new(py, &payload))
    }

    #[staticmethod]
    #[pyo3(signature = (payload, *, expected_slot_indices, max_bytes))]
    fn from_checkpoint_bytes(
        payload: &[u8],
        expected_slot_indices: Vec<usize>,
        max_bytes: usize,
    ) -> PyResult<Self> {
        Self::decode_cross_process_checkpoint(payload, &expected_slot_indices, max_bytes)
            .map_err(value_error)
    }
}

impl LearningCheckpointBatch {
    fn encode_cross_process_checkpoint(&self, max_bytes: usize) -> Result<Vec<u8>, String> {
        let slots = self
            .checkpoints
            .iter()
            .map(|checkpoint| SerializedLearningSlotCheckpointV1 {
                source_slot_index: checkpoint.source_slot_index,
                session: checkpoint.session.clone(),
                decision_ordinals: checkpoint.bridge_state.decision_ordinals().to_vec(),
            })
            .collect();
        encode_serialized_checkpoint(
            &SerializedLearningBatchCheckpointV1 { slots },
            CHECKPOINT_BANK_MAGIC,
            CHECKPOINT_BANK_VERSION,
            max_bytes,
        )
    }

    fn decode_cross_process_checkpoint(
        payload: &[u8],
        expected_slot_indices: &[usize],
        max_bytes: usize,
    ) -> Result<Self, String> {
        let mut seen = BTreeSet::new();
        for slot_index in expected_slot_indices {
            if !seen.insert(*slot_index) {
                return Err(format!(
                    "slot {slot_index} appears more than once in expected checkpoint bank"
                ));
            }
        }
        let serialized = decode_serialized_checkpoint(
            payload,
            CHECKPOINT_BANK_MAGIC,
            CHECKPOINT_BANK_VERSION,
            max_bytes,
        )?;
        if serialized.slots.len() != expected_slot_indices.len() {
            return Err(format!(
                "checkpoint bank contains {} slots, expected {}",
                serialized.slots.len(),
                expected_slot_indices.len()
            ));
        }
        for (expected_slot, slot) in expected_slot_indices.iter().zip(&serialized.slots) {
            if slot.source_slot_index != *expected_slot {
                return Err(format!(
                    "checkpoint bank contains slot {} where slot {expected_slot} was expected",
                    slot.source_slot_index
                ));
            }
        }

        let checkpoints = serialized
            .slots
            .into_iter()
            .map(|slot| {
                let env = LearningEnvV1::from_checkpoint(slot.session.clone())?;
                let pool =
                    LearningEnvPoolV1::from_envs([env]).map_err(|error| error.to_string())?;
                let bridge_state = replay_bridge_state(&pool, 0, &slot.decision_ordinals)?;
                Ok(LearningSlotCheckpoint {
                    source_slot_index: slot.source_slot_index,
                    session: slot.session,
                    bridge_state,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self { checkpoints })
    }
}

#[derive(Debug)]
struct DecisionSnapshot {
    slot_indices: Vec<usize>,
    phases: Vec<u8>,
    candidate_counts: Vec<usize>,
    candidate_row_splits: Vec<usize>,
}

#[derive(Debug)]
struct ProductionBehaviorSnapshot {
    available: Vec<bool>,
    ordinals: Vec<usize>,
}

#[pyclass(frozen, name = "LearningPublicRunContextV1")]
struct PyLearningPublicRunContextV1 {
    inner: LearningPublicRunContextV1,
}

/// Compact, on-demand audit facts for one exact combat root.
///
/// This view is deliberately separate from the per-decision semantic batch:
/// collection and training journals can inspect curriculum composition without
/// copying the deck and relic inventory through every model inference call.
#[pyclass(frozen, name = "CombatLearningRootAuditV1")]
struct PyCombatLearningRootAuditV1 {
    seed: u64,
    act: u8,
    floor: i32,
    ascension_level: u8,
    hp: i32,
    max_hp: i32,
    potion_ids: Vec<Option<String>>,
    encounter_id: String,
    monster_ids: Vec<String>,
    is_elite_fight: bool,
    is_boss_fight: bool,
    master_deck_cards: Vec<(String, u8)>,
    relic_ids: Vec<String>,
}

#[pymethods]
impl PyCombatLearningRootAuditV1 {
    #[getter]
    fn seed(&self) -> u64 {
        self.seed
    }

    #[getter]
    fn act(&self) -> u8 {
        self.act
    }

    #[getter]
    fn floor(&self) -> i32 {
        self.floor
    }

    #[getter]
    fn ascension_level(&self) -> u8 {
        self.ascension_level
    }

    #[getter]
    fn hp(&self) -> i32 {
        self.hp
    }

    #[getter]
    fn max_hp(&self) -> i32 {
        self.max_hp
    }

    #[getter]
    fn potion_ids(&self) -> Vec<Option<String>> {
        self.potion_ids.clone()
    }

    #[getter]
    fn encounter_id(&self) -> String {
        self.encounter_id.clone()
    }

    #[getter]
    fn monster_ids(&self) -> Vec<String> {
        self.monster_ids.clone()
    }

    #[getter]
    fn is_elite_fight(&self) -> bool {
        self.is_elite_fight
    }

    #[getter]
    fn is_boss_fight(&self) -> bool {
        self.is_boss_fight
    }

    #[getter]
    fn master_deck_cards(&self) -> Vec<(String, u8)> {
        self.master_deck_cards.clone()
    }

    #[getter]
    fn relic_ids(&self) -> Vec<String> {
        self.relic_ids.clone()
    }
}

#[pymethods]
impl PyLearningPublicRunContextV1 {
    #[getter]
    fn boundary_kind(&self) -> u8 {
        match self.inner.boundary_kind {
            LearningBoundaryKindV1::Strategic => RUN_BOUNDARY_STRATEGIC,
            LearningBoundaryKindV1::Combat => RUN_BOUNDARY_COMBAT,
            LearningBoundaryKindV1::Terminal => RUN_BOUNDARY_TERMINAL,
            LearningBoundaryKindV1::Unsupported => RUN_BOUNDARY_UNSUPPORTED,
        }
    }

    #[getter]
    fn strategic_context_kind(&self) -> Option<u8> {
        self.inner.strategic_context_kind.map(|kind| kind as u8)
    }

    #[getter]
    fn is_combat(&self) -> bool {
        self.inner.boundary_kind == LearningBoundaryKindV1::Combat
    }

    #[getter]
    fn is_terminal(&self) -> bool {
        self.inner.boundary_kind == LearningBoundaryKindV1::Terminal
    }

    #[getter]
    fn seed(&self) -> u64 {
        self.inner.seed
    }

    #[getter]
    fn ascension_level(&self) -> u8 {
        self.inner.ascension_level
    }

    #[getter]
    fn act(&self) -> u8 {
        self.inner.act
    }

    #[getter]
    fn floor(&self) -> i32 {
        self.inner.floor
    }

    #[getter]
    fn hp(&self) -> i32 {
        self.inner.hp
    }

    #[getter]
    fn max_hp(&self) -> i32 {
        self.inner.max_hp
    }

    #[getter]
    fn gold(&self) -> i32 {
        self.inner.gold
    }

    #[getter]
    fn potion_ids(&self) -> Vec<Option<String>> {
        potion_id_names(&self.inner.potion_ids)
    }

    #[getter]
    fn encounter_id(&self) -> Option<String> {
        self.inner
            .encounter_id
            .map(|encounter| format!("{encounter:?}"))
    }

    #[getter]
    fn monster_ids(&self) -> Vec<String> {
        self.inner
            .monster_ids
            .iter()
            .map(|monster| format!("{monster:?}"))
            .collect()
    }
}

#[pyclass]
struct LearningBatchEnv {
    pool: LearningEnvPoolV1,
    states: Vec<BridgeSlotState>,
    potion_policy: CombatLearningPotionPolicyV1,
}

#[pymethods]
impl LearningBatchEnv {
    #[new]
    #[pyo3(signature = (seeds, ascension_level))]
    fn new(seeds: Vec<u64>, ascension_level: u8) -> PyResult<Self> {
        Self::from_seeds_with_potion_policy(
            seeds,
            ascension_level,
            CombatLearningPotionPolicyV1::All,
        )
    }

    #[staticmethod]
    #[pyo3(signature = (seeds, ascension_level))]
    fn without_combat_potions(seeds: Vec<u64>, ascension_level: u8) -> PyResult<Self> {
        Self::from_seeds_with_potion_policy(
            seeds,
            ascension_level,
            CombatLearningPotionPolicyV1::never(),
        )
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
        bridge_states_ready(&self.states)
    }

    #[pyo3(signature = (*, max_bytes))]
    fn checkpoint_bytes<'py>(
        &self,
        py: Python<'py>,
        max_bytes: usize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let payload = self
            .encode_cross_process_checkpoint(max_bytes)
            .map_err(value_error)?;
        Ok(PyBytes::new(py, &payload))
    }

    #[staticmethod]
    #[pyo3(signature = (payload, *, expected_slots, max_bytes))]
    fn from_checkpoint_bytes(
        payload: &[u8],
        expected_slots: usize,
        max_bytes: usize,
    ) -> PyResult<Self> {
        Self::decode_cross_process_checkpoint(payload, expected_slots, max_bytes)
            .map_err(value_error)
    }

    /// Construct a fresh batch from exact production combat-root artifacts.
    ///
    /// Rust decodes and validates every opaque run-control checkpoint before
    /// exposing the first slot. Python never receives simulator session data.
    #[staticmethod]
    #[pyo3(signature = (payload, *, expected_roots, max_bytes))]
    fn from_combat_root_artifact_bytes(
        payload: &[u8],
        expected_roots: usize,
        max_bytes: usize,
    ) -> PyResult<Self> {
        Self::decode_combat_root_artifact(payload, expected_roots, max_bytes).map_err(value_error)
    }

    /// Merge canonical single-root payloads while keeping checkpoints opaque.
    #[staticmethod]
    #[pyo3(signature = (payloads, *, max_bytes))]
    fn merge_combat_root_artifact_bytes<'py>(
        py: Python<'py>,
        payloads: Vec<Vec<u8>>,
        max_bytes: usize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        if payloads.is_empty() || payloads.len() > MAX_EXPORTED_COMBAT_ROOTS {
            return Err(PyValueError::new_err(format!(
                "combat root merge requires 1..={MAX_EXPORTED_COMBAT_ROOTS} payloads"
            )));
        }
        let artifact = CombatLearningRootBatchArtifactV1::merge_single_root_payloads(
            payloads.iter().map(Vec::as_slice),
            max_bytes,
        )
        .map_err(value_error)?;
        let payload = artifact.encode(max_bytes).map_err(value_error)?;
        Ok(PyBytes::new(py, &payload))
    }

    /// Return canonical potion identities accepted by typed root selectors.
    #[staticmethod]
    fn supported_potion_ids() -> Vec<String> {
        sts_oracle_eval::content::potions::ALL_POTIONS
            .iter()
            .map(|potion| format!("{potion:?}"))
            .collect()
    }

    /// Normalize one typed encounter identity accepted by root selectors.
    #[staticmethod]
    fn canonical_encounter_id(raw: &str) -> PyResult<String> {
        sts_oracle_eval::sim::combat_start::encounter_id_from_input(raw)
            .map(|encounter| format!("{encounter:?}"))
            .map_err(value_error)
    }

    /// Export selected current undecoded combat slots as one opaque root artifact.
    ///
    /// Python may persist or forward the bytes but never receives simulator
    /// checkpoint fields.
    #[pyo3(signature = (slot_indices, *, max_bytes))]
    fn combat_root_artifact_bytes<'py>(
        &self,
        py: Python<'py>,
        slot_indices: Vec<usize>,
        max_bytes: usize,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let payload = self
            .encode_combat_root_artifact(&slot_indices, max_bytes)
            .map_err(value_error)?;
        Ok(PyBytes::new(py, &payload))
    }

    #[pyo3(signature = (dense_mask=false, semantic=false, production_behavior=false))]
    fn decision_batch<'py>(
        &self,
        py: Python<'py>,
        dense_mask: bool,
        semantic: bool,
        production_behavior: bool,
    ) -> PyResult<Bound<'py, PyDict>> {
        let snapshot = self.decision_snapshot()?;
        let semantic = semantic.then(|| self.semantic_snapshot()).transpose()?;
        let production_behavior = production_behavior
            .then(|| self.production_behavior_snapshot(&snapshot))
            .transpose()?;
        decision_batch_dict(
            py,
            snapshot,
            dense_mask,
            semantic,
            production_behavior,
        )
    }

    /// Return the exact typed strategic candidates for one current root decision.
    ///
    /// Combat, terminal, ready, and symbolic-selection slots return `None`.
    fn strategic_decision_audit_json(&self, slot_index: usize) -> PyResult<Option<String>> {
        let source = LearningBatchDecisionSource::new(&self.pool, &self.potion_policy);
        strategic_decision_audit_json_from_source(&source, &self.states, slot_index)
            .map_err(value_error)
    }

    fn choose(&mut self, ordinals: Vec<usize>) -> PyResult<()> {
        let source = LearningBatchDecisionSource::new(&self.pool, &self.potion_policy);
        choose_bridge_ordinals(&source, &mut self.states, ordinals).map_err(value_error)
    }

    fn step<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        if !self.ready() {
            return Err(PyValueError::new_err(
                "all active slots must finish root and selection decisions before step",
            ));
        }
        let actions = collect_ready_actions(&self.states).map_err(runtime_error)?;
        let step = self.pool.step_active(actions).map_err(runtime_error)?;
        self.states = states_from_source(&self.pool).map_err(runtime_error)?;

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

    #[pyo3(signature = (slot_index, replicate_count, potion_slots=None))]
    fn combat_group(
        &self,
        slot_index: usize,
        replicate_count: usize,
        potion_slots: Option<Vec<usize>>,
    ) -> PyResult<CombatLearningBatchEnv> {
        if !matches!(self.states.get(slot_index), Some(BridgeSlotState::Root)) {
            return Err(PyValueError::new_err(format!(
                "slot {slot_index} must be at an undecoded root decision"
            )));
        }
        let checkpoint = self
            .pool
            .checkpoint_slot(slot_index)
            .map_err(runtime_error)?;
        let root = CombatLearningRootV1::from_checkpoint(checkpoint).map_err(value_error)?;
        CombatLearningBatchEnv::from_root_with_potion_slots(
            &root,
            replicate_count,
            potion_slots,
        )
        .map_err(value_error)
    }

    /// Return every current undecoded combat root without creating replicate groups.
    fn combat_root_contexts<'py>(
        &self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyList>> {
        let contexts = PyList::empty(py);
        for (slot_index, state) in self.states.iter().enumerate() {
            if !matches!(state, BridgeSlotState::Root)
                || !matches!(
                    self.pool.boundary(slot_index),
                    Some(LearningBoundaryV1::Combat { .. })
                )
            {
                continue;
            }
            let context = self
                .pool
                .combat_root_context(slot_index)
                .map_err(runtime_error)?;
            let view = Py::new(py, PyCombatLearningRootContextV1::from_context(context))?;
            contexts.append((slot_index, view))?;
        }
        Ok(contexts)
    }

    /// Return auditable deck, upgrade, relic, and ascension facts for one root.
    ///
    /// The exact session stays opaque.  Python receives only stable domain
    /// identities needed to review curriculum composition.
    fn combat_root_audit(&self, slot_index: usize) -> PyResult<PyCombatLearningRootAuditV1> {
        if !matches!(self.states.get(slot_index), Some(BridgeSlotState::Root))
            || !matches!(
                self.pool.boundary(slot_index),
                Some(LearningBoundaryV1::Combat { .. })
            )
        {
            return Err(PyValueError::new_err(format!(
                "slot {slot_index} must be at an undecoded combat root"
            )));
        }
        let context = self
            .pool
            .combat_root_context(slot_index)
            .map_err(runtime_error)?;
        let public = self
            .pool
            .public_run_context(slot_index)
            .map_err(runtime_error)?;
        let encounter_id = public.encounter_id.ok_or_else(|| {
            PyRuntimeError::new_err("combat root audit has no encounter identity")
        })?;
        let session = self
            .pool
            .checkpoint_slot(slot_index)
            .map_err(runtime_error)?
            .into_session()
            .map_err(runtime_error)?;
        let active = session.active_combat.as_ref().ok_or_else(|| {
            PyRuntimeError::new_err("combat root audit has no active combat")
        })?;
        let deck = &active.combat_state.meta.master_deck_snapshot;
        let relics = &active.combat_state.entities.player.relics;
        if deck.len() != context.master_deck_card_count as usize
            || relics.len() != context.relic_count as usize
            || active.combat_state.meta.ascension_level != context.ascension_level
            || public.ascension_level != context.ascension_level
            || public.act != context.act
            || public.floor != context.floor
            || public.hp != context.hp
            || public.max_hp != context.max_hp
        {
            return Err(PyRuntimeError::new_err(
                "combat root audit disagrees with its compact context",
            ));
        }
        Ok(PyCombatLearningRootAuditV1 {
            seed: public.seed,
            act: public.act,
            floor: public.floor,
            ascension_level: context.ascension_level,
            hp: public.hp,
            max_hp: public.max_hp,
            potion_ids: potion_id_names(&public.potion_ids),
            encounter_id: format!("{encounter_id:?}"),
            monster_ids: public
                .monster_ids
                .iter()
                .map(|monster| format!("{monster:?}"))
                .collect(),
            is_elite_fight: context.is_elite_fight,
            is_boss_fight: context.is_boss_fight,
            master_deck_cards: deck
                .iter()
                .map(|card| (format!("{:?}", card.id), card.upgrades))
                .collect(),
            relic_ids: relics
                .iter()
                .map(|relic| format!("{:?}", relic.id))
                .collect(),
        })
    }

    /// Return compact public run facts for every slot without cloning sessions.
    fn public_run_contexts<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let contexts = PyList::empty(py);
        for slot_index in 0..self.pool.slot_count() {
            let context = self
                .pool
                .public_run_context(slot_index)
                .map_err(runtime_error)?;
            let view = Py::new(py, PyLearningPublicRunContextV1 { inner: context })?;
            contexts.append((slot_index, view))?;
        }
        Ok(contexts)
    }

    fn reset_slot(&mut self, slot_index: usize, seed: u64) -> PyResult<()> {
        self.reset_slots(vec![slot_index], vec![seed])
    }

    fn reset_slots(&mut self, slot_indices: Vec<usize>, seeds: Vec<u64>) -> PyResult<()> {
        self.reset_slots_checkpointed(slot_indices, seeds)
            .map(|_| ())
    }

    fn reset_slots_checkpointed(
        &mut self,
        slot_indices: Vec<usize>,
        seeds: Vec<u64>,
    ) -> PyResult<LearningCheckpointBatch> {
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
        let mut checkpoints = Vec::with_capacity(slot_indices.len());
        let mut replacements = Vec::with_capacity(slot_indices.len());
        for (slot_index, seed) in slot_indices.iter().copied().zip(seeds) {
            let config = self
                .pool
                .fresh_run_config(slot_index, seed)
                .map_err(runtime_error)?;
            let env = LearningEnvV1::new(config);
            checkpoints.push(LearningSlotCheckpoint {
                source_slot_index: slot_index,
                session: env.checkpoint(),
                bridge_state: BridgeSlotState::Root,
            });
            replacements.push((slot_index, env));
        }
        self.pool
            .replace_slots(replacements)
            .map_err(runtime_error)?;
        for slot_index in slot_indices {
            self.states[slot_index] = BridgeSlotState::Root;
        }
        Ok(LearningCheckpointBatch { checkpoints })
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
    fn from_seeds_with_potion_policy(
        seeds: Vec<u64>,
        ascension_level: u8,
        potion_policy: CombatLearningPotionPolicyV1,
    ) -> PyResult<Self> {
        if ascension_level > 20 {
            return Err(PyValueError::new_err(
                "ascension_level must be between 0 and 20",
            ));
        }
        let pool =
            LearningEnvPoolV1::from_configs(seeds.into_iter().map(|seed| RunControlConfig {
                seed,
                ascension_level,
                ..RunControlConfig::default()
            }))
            .map_err(runtime_error)?;
        let states = states_from_source(&pool).map_err(runtime_error)?;
        Ok(Self {
            pool,
            states,
            potion_policy,
        })
    }

    fn encode_combat_root_artifact(
        &self,
        slot_indices: &[usize],
        max_bytes: usize,
    ) -> Result<Vec<u8>, String> {
        if slot_indices.is_empty() || slot_indices.len() > MAX_EXPORTED_COMBAT_ROOTS {
            return Err(format!(
                "combat root export count must be in 1..={MAX_EXPORTED_COMBAT_ROOTS}"
            ));
        }
        let mut seen = BTreeSet::new();
        let mut checkpoints = Vec::with_capacity(slot_indices.len());
        for &slot_index in slot_indices {
            if !seen.insert(slot_index) {
                return Err(format!(
                    "combat root export slot {slot_index} appears more than once"
                ));
            }
            if !matches!(self.states.get(slot_index), Some(BridgeSlotState::Root))
                || !matches!(
                    self.pool.boundary(slot_index),
                    Some(LearningBoundaryV1::Combat { .. })
                )
            {
                return Err(format!(
                    "combat root export slot {slot_index} must be at an undecoded combat root"
                ));
            }
            checkpoints.push(
                self.pool
                    .checkpoint_slot(slot_index)
                    .map_err(|error| error.to_string())?,
            );
        }
        CombatLearningRootBatchArtifactV1::from_checkpoints(checkpoints)?.encode(max_bytes)
    }

    fn decode_combat_root_artifact(
        payload: &[u8],
        expected_roots: usize,
        max_bytes: usize,
    ) -> Result<Self, String> {
        let artifact =
            CombatLearningRootBatchArtifactV1::decode(payload, expected_roots, max_bytes)?;
        let envs = artifact
            .into_checkpoints()?
            .into_iter()
            .map(LearningEnvV1::from_checkpoint)
            .collect::<Result<Vec<_>, _>>()?;
        let pool = LearningEnvPoolV1::from_envs(envs).map_err(|error| error.to_string())?;
        for slot_index in 0..pool.slot_count() {
            if !matches!(
                pool.boundary(slot_index),
                Some(LearningBoundaryV1::Combat { .. })
            ) {
                return Err(format!(
                    "combat learning root artifact slot {slot_index} is not at a combat boundary"
                ));
            }
        }
        let states = states_from_source(&pool)?;
        if let Some(slot_index) = states
            .iter()
            .position(|state| !matches!(state, BridgeSlotState::Root))
        {
            return Err(format!(
                "combat learning root artifact slot {slot_index} is not at an undecoded root"
            ));
        }
        Ok(Self {
            pool,
            states,
            potion_policy: CombatLearningPotionPolicyV1::All,
        })
    }

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
    fn encode_cross_process_checkpoint(&self, max_bytes: usize) -> Result<Vec<u8>, String> {
        if self.potion_policy.root_slots().is_some() {
            return Err(
                "cross-process batch checkpoints require the all-potions surface".to_owned(),
            );
        }
        let slots = self
            .states
            .iter()
            .enumerate()
            .map(|(source_slot_index, state)| {
                let session = self
                    .pool
                    .checkpoint_slot(source_slot_index)
                    .map_err(|error| error.to_string())?;
                Ok(SerializedLearningSlotCheckpointV1 {
                    source_slot_index,
                    session,
                    decision_ordinals: state.decision_ordinals().to_vec(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let checkpoint = SerializedLearningBatchCheckpointV1 { slots };
        encode_serialized_checkpoint(
            &checkpoint,
            BATCH_CHECKPOINT_MAGIC,
            BATCH_CHECKPOINT_VERSION,
            max_bytes,
        )
    }

    fn decode_cross_process_checkpoint(
        payload: &[u8],
        expected_slots: usize,
        max_bytes: usize,
    ) -> Result<Self, String> {
        if expected_slots == 0 {
            return Err("batch checkpoint expected slot count must be positive".to_owned());
        }
        let checkpoint = decode_serialized_checkpoint(
            payload,
            BATCH_CHECKPOINT_MAGIC,
            BATCH_CHECKPOINT_VERSION,
            max_bytes,
        )?;
        if checkpoint.slots.len() != expected_slots {
            return Err(format!(
                "batch checkpoint contains {} slots, expected {expected_slots}",
                checkpoint.slots.len()
            ));
        }
        for (expected_slot, slot) in checkpoint.slots.iter().enumerate() {
            if slot.source_slot_index != expected_slot {
                return Err(format!(
                    "batch checkpoint slot {} is stored at position {expected_slot}",
                    slot.source_slot_index
                ));
            }
        }

        let envs = checkpoint
            .slots
            .iter()
            .map(|slot| LearningEnvV1::from_checkpoint(slot.session.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let pool = LearningEnvPoolV1::from_envs(envs).map_err(|error| error.to_string())?;
        let states = checkpoint
            .slots
            .iter()
            .map(|slot| replay_bridge_state(&pool, slot.source_slot_index, &slot.decision_ordinals))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            pool,
            states,
            potion_policy: CombatLearningPotionPolicyV1::All,
        })
    }

    fn decision_snapshot(&self) -> PyResult<DecisionSnapshot> {
        let source = LearningBatchDecisionSource::new(&self.pool, &self.potion_policy);
        decision_snapshot_from_source(&source, &self.states).map_err(runtime_error)
    }

    fn semantic_snapshot(&self) -> PyResult<SemanticBatch> {
        let source = LearningBatchDecisionSource::new(&self.pool, &self.potion_policy);
        semantic_snapshot_from_source(&source, &self.states).map_err(runtime_error)
    }

    fn production_behavior_snapshot(
        &self,
        snapshot: &DecisionSnapshot,
    ) -> PyResult<ProductionBehaviorSnapshot> {
        let mut available = Vec::with_capacity(snapshot.slot_indices.len());
        let mut ordinals = Vec::with_capacity(snapshot.slot_indices.len());
        for &slot_index in &snapshot.slot_indices {
            let ordinal = production_behavior_ordinal(
                &self.pool,
                &self.states[slot_index],
                slot_index,
            )
            .map_err(runtime_error)?;
            available.push(ordinal.is_some());
            ordinals.push(ordinal.unwrap_or(0));
        }
        Ok(ProductionBehaviorSnapshot {
            available,
            ordinals,
        })
    }
}

fn encode_serialized_checkpoint(
    checkpoint: &SerializedLearningBatchCheckpointV1,
    magic: &[u8],
    version: u32,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let mut writer = BoundedCheckpointWriter::new(magic, version, max_bytes)?;
    rmp_serde::encode::write_named(&mut writer, checkpoint)
        .map_err(|error| format!("cannot encode checkpoint: {error}"))?;
    Ok(writer.finish())
}

fn decode_serialized_checkpoint(
    payload: &[u8],
    magic: &[u8],
    version: u32,
    max_bytes: usize,
) -> Result<SerializedLearningBatchCheckpointV1, String> {
    if payload.len() > max_bytes {
        return Err("checkpoint exceeds its caller-provided byte limit".to_owned());
    }
    let header_bytes = magic.len() + std::mem::size_of::<u32>();
    if payload.len() < header_bytes {
        return Err("checkpoint ended before its header".to_owned());
    }
    if &payload[..magic.len()] != magic {
        return Err("checkpoint magic is invalid".to_owned());
    }
    let version_start = magic.len();
    let encoded_version = u32::from_be_bytes(
        payload[version_start..header_bytes]
            .try_into()
            .map_err(|_| "checkpoint version is truncated")?,
    );
    if encoded_version != version {
        return Err("checkpoint format version is unsupported".to_owned());
    }
    let mut decoder = rmp_serde::Deserializer::new(Cursor::new(&payload[header_bytes..]));
    let checkpoint = SerializedLearningBatchCheckpointV1::deserialize(&mut decoder)
        .map_err(|error| format!("cannot decode checkpoint: {error}"))?;
    if usize::try_from(decoder.position()).ok() != Some(payload.len() - header_bytes) {
        return Err("checkpoint contains trailing bytes".to_owned());
    }
    if encode_serialized_checkpoint(&checkpoint, magic, version, max_bytes)? != payload {
        return Err("checkpoint encoding is not canonical".to_owned());
    }
    Ok(checkpoint)
}

fn decision_batch_dict<'py>(
    py: Python<'py>,
    snapshot: DecisionSnapshot,
    dense_mask: bool,
    semantic: Option<SemanticBatch>,
    production_behavior: Option<ProductionBehaviorSnapshot>,
) -> PyResult<Bound<'py, PyDict>> {
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
    if let Some(semantic) = semantic {
        result.set_item("semantic", semantic_dict(py, semantic)?)?;
    }
    if let Some(production_behavior) = production_behavior {
        result.set_item(
            "production_behavior_available",
            PyArray1::from_vec(py, production_behavior.available),
        )?;
        result.set_item(
            "production_behavior_ordinals",
            usize_array(py, production_behavior.ordinals),
        )?;
    }
    Ok(result)
}

fn production_behavior_ordinal(
    pool: &LearningEnvPoolV1,
    state: &BridgeSlotState,
    slot_index: usize,
) -> Result<Option<usize>, String> {
    if !matches!(state, BridgeSlotState::Root) {
        return Ok(None);
    }
    let Some(LearningBoundaryV1::Strategic { boundary }) = pool.boundary(slot_index) else {
        return Ok(None);
    };
    let session = pool
        .checkpoint_slot(slot_index)
        .map_err(|error| error.to_string())?
        .into_session()?;
    let Some(run_candidate_id) =
        sts_oracle_runtime::runtime::branch::current_oracle_candidate_order_v1(&session)
            .into_iter()
            .next()
    else {
        return Ok(None);
    };
    let segment = capture_planner_boundary_yield_v1(
        &session,
        PlannerBoundaryYieldKindV1::CallbackStop,
    )?;
    let [visit] = segment.visits.as_slice() else {
        return Ok(None);
    };
    let Some(link) = visit
        .candidate_links
        .iter()
        .find(|link| link.run_candidate_id == run_candidate_id)
    else {
        return Ok(None);
    };
    let decision = LearningModelDecisionV1::from_strategic_boundary(boundary)
        .map_err(|error| error.to_string())?;
    Ok(decision.strategic_ordinal_for_planner_candidate_id(&link.planner_candidate_id))
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
    module.add_class::<CombatLearningBatchEnv>()?;
    module.add_class::<CombatLearningDecisionProgressV1>()?;
    module.add_class::<CombatLearningRecoveryRoot>()?;
    module.add_class::<PyCombatLearningRootContextV1>()?;
    module.add_class::<PyCombatLearningRootAuditV1>()?;
    module.add_class::<PyLearningPublicRunContextV1>()?;
    module.add_class::<LearningBatchEnv>()?;
    module.add_class::<LearningCheckpointBatch>()?;
    module.add_class::<LearningSlotCheckpoint>()?;
    module.add_function(wrap_pyfunction!(semantic_schema, module)?)?;
    module.add("PHASE_STRATEGIC_ROOT", PHASE_STRATEGIC_ROOT)?;
    module.add("PHASE_COMBAT_ROOT", PHASE_COMBAT_ROOT)?;
    module.add("PHASE_SELECTION", PHASE_SELECTION)?;
    module.add("RUN_BOUNDARY_STRATEGIC", RUN_BOUNDARY_STRATEGIC)?;
    module.add("RUN_BOUNDARY_COMBAT", RUN_BOUNDARY_COMBAT)?;
    module.add("RUN_BOUNDARY_TERMINAL", RUN_BOUNDARY_TERMINAL)?;
    module.add("RUN_BOUNDARY_UNSUPPORTED", RUN_BOUNDARY_UNSUPPORTED)?;
    module.add("COMBAT_TERMINAL_WIN", COMBAT_TERMINAL_WIN)?;
    module.add("COMBAT_TERMINAL_LOSS", COMBAT_TERMINAL_LOSS)?;
    module.add("COMBAT_TERMINAL_UNRESOLVED", COMBAT_TERMINAL_UNRESOLVED)?;
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
    use sts_oracle_eval::content::monsters::EnemyId;
    use sts_oracle_eval::content::potions::{Potion, PotionId};
    use sts_oracle_eval::eval::run_control::{
        CombatLearningRootBatchArtifactV1, LearningEnvV1, LearningModelChoiceV1,
        LearningModelDecisionV1, LearningSelectionStepV1, RunControlSession,
    };
    use sts_oracle_eval::runtime::combat::CombatCard;
    use sts_oracle_eval::state::core::{
        ActiveCombat, CombatContext, EngineState, PendingChoice, RoomCombatContext,
    };
    use sts_oracle_eval::state::map::node::RoomType;

    use super::*;

    #[test]
    fn production_behavior_labels_only_represented_strategic_roots() {
        let env = LearningBatchEnv::from_seeds_with_potion_policy(
            vec![0],
            20,
            CombatLearningPotionPolicyV1::All,
        )
        .expect("create strategic learning batch");
        let snapshot = env.decision_snapshot().expect("strategic decision");
        let behavior = env
            .production_behavior_snapshot(&snapshot)
            .expect("production behavior");

        assert_eq!(behavior.available, vec![true]);
        assert!(behavior.ordinals[0] < snapshot.candidate_counts[0]);
    }

    #[test]
    fn no_potion_batch_changes_the_native_combat_candidate_surface() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = sts_oracle_eval::test_support::blank_test_combat();
        combat
            .entities
            .monsters
            .push(sts_oracle_eval::test_support::test_monster(EnemyId::JawWorm));
        combat.entities.potions = vec![Some(Potion::new(PotionId::BlockPotion, 17))];
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let pool = LearningEnvPoolV1::from_envs([LearningEnvV1::from_session(session)])
            .expect("create combat pool");
        let states = states_from_source(&pool).expect("derive bridge states");
        let mut env = LearningBatchEnv {
            pool,
            states,
            potion_policy: CombatLearningPotionPolicyV1::All,
        };

        let all_candidates = env
            .decision_snapshot()
            .expect("all-potion decision")
            .candidate_counts[0];
        let snapshot = env.decision_snapshot().expect("combat decision");
        let behavior = env
            .production_behavior_snapshot(&snapshot)
            .expect("combat behavior availability");
        assert_eq!(behavior.available, vec![false]);
        env.potion_policy = CombatLearningPotionPolicyV1::never();
        let never_candidates = env
            .decision_snapshot()
            .expect("no-potion decision")
            .candidate_counts[0];

        assert!(never_candidates < all_candidates);
        assert!(env.encode_cross_process_checkpoint(1024 * 1024).is_err());
    }

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
            states: states_from_source(&pool).expect("derive bridge states"),
            pool,
            potion_policy: CombatLearningPotionPolicyV1::All,
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
            bridge_state: BridgeSlotState::Selection {
                draft: draft.clone(),
                decision_ordinals: vec![0, 1],
            },
        };
        env.states[0] = checkpoint.bridge_state.clone();
        let payload = env
            .encode_cross_process_checkpoint(1024 * 1024)
            .expect("encode selection prefix");
        let restored = LearningBatchEnv::decode_cross_process_checkpoint(&payload, 1, 1024 * 1024)
            .expect("restore selection prefix in a fresh owner");
        assert_eq!(
            restored
                .encode_cross_process_checkpoint(1024 * 1024)
                .expect("re-encode restored prefix"),
            payload
        );
        let BridgeSlotState::Selection {
            draft: restored_draft,
            decision_ordinals,
        } = &restored.states[0]
        else {
            panic!("cross-process checkpoint must resume symbolic selection");
        };
        assert_eq!(decision_ordinals, &[0, 1]);
        assert_eq!(restored_draft.selected_domain_indices(), &[0]);
        assert_eq!(restored_draft.decision().candidates.len(), 2);

        assert!(env.encode_cross_process_checkpoint(16).is_err());
        assert!(
            LearningBatchEnv::decode_cross_process_checkpoint(&payload, 2, 1024 * 1024,).is_err()
        );
        let mut bad_magic = payload.clone();
        bad_magic[0] ^= 0xff;
        assert!(
            LearningBatchEnv::decode_cross_process_checkpoint(&bad_magic, 1, 1024 * 1024,).is_err()
        );
        let header_bytes = BATCH_CHECKPOINT_MAGIC.len() + std::mem::size_of::<u32>();

        let mut trailing = payload.clone();
        trailing.push(0xc0);
        let trailing_error =
            LearningBatchEnv::decode_cross_process_checkpoint(&trailing, 1, 1024 * 1024)
                .err()
                .expect("trailing MessagePack value must be rejected");
        assert!(trailing_error.contains("trailing bytes"));

        assert_eq!(
            payload[header_bytes], 0x81,
            "root must use a one-entry fixmap"
        );
        let mut noncanonical = Vec::with_capacity(payload.len() + 2);
        noncanonical.extend_from_slice(&payload[..header_bytes]);
        noncanonical.extend_from_slice(&[0xde, 0x00, 0x01]);
        noncanonical.extend_from_slice(&payload[header_bytes + 1..]);
        let noncanonical_error =
            LearningBatchEnv::decode_cross_process_checkpoint(&noncanonical, 1, 1024 * 1024)
                .err()
                .expect("non-canonical MessagePack map width must be rejected");
        assert!(noncanonical_error.contains("not canonical"));

        let mut malformed: SerializedLearningBatchCheckpointV1 =
            rmp_serde::from_slice(&payload[header_bytes..]).expect("decode owned payload");
        malformed.slots[0].decision_ordinals.push(usize::MAX);
        let malformed = encode_serialized_checkpoint(
            &malformed,
            BATCH_CHECKPOINT_MAGIC,
            BATCH_CHECKPOINT_VERSION,
            1024 * 1024,
        )
        .expect("encode malformed prefix fixture");
        assert!(
            LearningBatchEnv::decode_cross_process_checkpoint(&malformed, 1, 1024 * 1024,).is_err()
        );

        let LearningSelectionStepV1::Apply(action) = draft.choose(0).expect("submit prefix") else {
            panic!("submit must produce an action");
        };
        env.states[0] = BridgeSlotState::Ready {
            action,
            decision_ordinals: vec![0, 1, 0],
        };
        assert!(matches!(env.states[0], BridgeSlotState::Ready { .. }));

        env.restore_slot_checkpoint(0, &checkpoint)
            .expect("restore prefix");
        let BridgeSlotState::Selection { draft, .. } = &env.states[0] else {
            panic!("restored slot must resume symbolic selection");
        };
        assert_eq!(draft.selected_domain_indices(), &[0]);
        assert_eq!(draft.decision().candidates.len(), 2);
    }

    #[test]
    fn checkpoint_bank_round_trips_episode_root_by_exact_slot_identity() {
        let env = LearningEnvV1::new(RunControlConfig {
            seed: 37,
            ..RunControlConfig::default()
        });
        let bank = LearningCheckpointBatch {
            checkpoints: vec![LearningSlotCheckpoint {
                source_slot_index: 3,
                session: env.checkpoint(),
                bridge_state: BridgeSlotState::Root,
            }],
        };
        let payload = bank
            .encode_cross_process_checkpoint(1024 * 1024)
            .expect("encode episode-root bank");
        let restored =
            LearningCheckpointBatch::decode_cross_process_checkpoint(&payload, &[3], 1024 * 1024)
                .expect("restore episode-root bank in a fresh owner");
        assert_eq!(restored.checkpoints.len(), 1);
        assert_eq!(restored.checkpoints[0].source_slot_index, 3);
        assert!(matches!(
            restored.checkpoints[0].bridge_state,
            BridgeSlotState::Root
        ));
        assert_eq!(
            restored
                .encode_cross_process_checkpoint(1024 * 1024)
                .expect("re-encode episode-root bank"),
            payload
        );
        assert!(LearningCheckpointBatch::decode_cross_process_checkpoint(
            &payload,
            &[2],
            1024 * 1024,
        )
        .is_err());
        assert!(LearningCheckpointBatch::decode_cross_process_checkpoint(
            &payload,
            &[3, 3],
            1024 * 1024,
        )
        .is_err());
    }

    #[test]
    fn production_combat_root_artifact_constructs_fresh_bridge_batch() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let choice =
            PendingChoice::DiscoverySelect(sts_oracle_eval::state::core::DiscoveryChoiceState {
                cards: vec![CardId::Bash, CardId::FiendFire],
                colorless: false,
                card_type: None,
                amount: 1,
                can_skip: true,
            });
        let mut combat = sts_oracle_eval::test_support::blank_test_combat();
        combat
            .entities
            .monsters
            .push(sts_oracle_eval::test_support::test_monster(EnemyId::JawWorm));
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let artifact = CombatLearningRootBatchArtifactV1::from_checkpoints([
            RunControlSessionCheckpointV1::from_session(&session),
        ])
        .expect("capture production combat root");
        let payload = artifact.encode(1024 * 1024).expect("encode artifact");
        let restored = LearningBatchEnv::decode_combat_root_artifact(&payload, 1, 1024 * 1024)
            .expect("construct bridge batch");

        assert_eq!(restored.pool.slot_count(), 1);
        assert!(matches!(restored.states[0], BridgeSlotState::Root));
        assert!(matches!(
            restored.pool.boundary(0),
            Some(LearningBoundaryV1::Combat { .. })
        ));
        assert_eq!(
            restored
                .encode_combat_root_artifact(&[0], 1024 * 1024)
                .expect("re-export selected exact combat root"),
            payload
        );
        assert!(restored
            .encode_combat_root_artifact(&[], 1024 * 1024)
            .is_err());
        assert!(restored
            .encode_combat_root_artifact(&[0, 0], 1024 * 1024)
            .is_err());
        assert!(LearningBatchEnv::decode_combat_root_artifact(&payload, 2, 1024 * 1024).is_err());
    }
}
