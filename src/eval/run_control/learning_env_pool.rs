//! Batched in-process execution for online-learning environments.
//!
//! The pool owns no policy, tensor encoder, reset curriculum, or automatic
//! recovery rule. It only keeps environment slots aligned with one ragged
//! model batch and applies one already-selected action per active slot.

use std::fmt;

use super::{
    LearningActionV1, LearningBoundaryV1, LearningEnvV1, LearningModelBatchV1,
    LearningModelInputError, RunControlConfig, RunControlSessionCheckpointV1,
};

#[derive(Clone, Debug)]
struct LearningEnvPoolSlotV1 {
    env: LearningEnvV1,
    boundary: LearningBoundaryV1,
}

#[derive(Clone, Debug)]
pub struct LearningEnvPoolV1 {
    slots: Vec<LearningEnvPoolSlotV1>,
    poisoned: bool,
}

impl LearningEnvPoolV1 {
    pub fn from_configs(
        configs: impl IntoIterator<Item = RunControlConfig>,
    ) -> Result<Self, LearningEnvPoolError> {
        Self::from_envs(configs.into_iter().map(LearningEnvV1::new))
    }

    pub fn from_envs(
        envs: impl IntoIterator<Item = LearningEnvV1>,
    ) -> Result<Self, LearningEnvPoolError> {
        let envs = envs.into_iter();
        let (lower_bound, _) = envs.size_hint();
        let mut slots = Vec::with_capacity(lower_bound);
        for (slot_index, env) in envs.enumerate() {
            let boundary =
                env.observe()
                    .map_err(|message| LearningEnvPoolError::InitialObservation {
                        slot_index,
                        message,
                    })?;
            slots.push(LearningEnvPoolSlotV1 { env, boundary });
        }
        Ok(Self {
            slots,
            poisoned: false,
        })
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn active_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| !slot.boundary.is_terminal())
            .count()
    }

    pub fn terminal_count(&self) -> usize {
        self.slot_count().saturating_sub(self.active_count())
    }

    pub fn all_terminal(&self) -> bool {
        self.active_count() == 0
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn boundary(&self, slot_index: usize) -> Option<&LearningBoundaryV1> {
        self.slots.get(slot_index).map(|slot| &slot.boundary)
    }

    /// Captures one exact in-memory slot for a caller-owned recovery or
    /// curriculum policy. Nothing is serialized and the pool does not retain
    /// an automatic checkpoint history.
    pub fn checkpoint_slot(
        &self,
        slot_index: usize,
    ) -> Result<RunControlSessionCheckpointV1, LearningEnvPoolError> {
        if self.poisoned {
            return Err(LearningEnvPoolError::PoolPoisoned);
        }
        let slot = self
            .slots
            .get(slot_index)
            .ok_or(LearningEnvPoolError::SlotIndexOutOfRange {
                slot_index,
                slot_count: self.slots.len(),
            })?;
        Ok(slot.env.checkpoint())
    }

    /// Explicitly replaces one slot after a caller has chosen its reset or
    /// curriculum policy. The replacement is observed before the old slot is
    /// changed, so an invalid environment leaves the pool untouched.
    pub fn replace_slot(
        &mut self,
        slot_index: usize,
        env: LearningEnvV1,
    ) -> Result<(), LearningEnvPoolError> {
        if self.poisoned {
            return Err(LearningEnvPoolError::PoolPoisoned);
        }
        if slot_index >= self.slots.len() {
            return Err(LearningEnvPoolError::SlotIndexOutOfRange {
                slot_index,
                slot_count: self.slots.len(),
            });
        }
        let boundary =
            env.observe()
                .map_err(|message| LearningEnvPoolError::ReplacementObservation {
                    slot_index,
                    message,
                })?;
        self.slots[slot_index] = LearningEnvPoolSlotV1 { env, boundary };
        Ok(())
    }

    pub fn reset_slot(
        &mut self,
        slot_index: usize,
        config: RunControlConfig,
    ) -> Result<(), LearningEnvPoolError> {
        self.replace_slot(slot_index, LearningEnvV1::new(config))
    }

    pub fn active_model_batch(
        &self,
    ) -> Result<LearningEnvPoolModelBatchV1<'_>, LearningEnvPoolError> {
        if self.poisoned {
            return Err(LearningEnvPoolError::PoolPoisoned);
        }
        let active_slot_indices = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot_index, slot)| (!slot.boundary.is_terminal()).then_some(slot_index))
            .collect::<Vec<_>>();
        let model_batch = LearningModelBatchV1::from_boundary_refs(
            active_slot_indices
                .iter()
                .map(|slot_index| &self.slots[*slot_index].boundary),
        )
        .map_err(LearningEnvPoolError::ModelInput)?;
        Ok(LearningEnvPoolModelBatchV1 {
            active_slot_indices,
            model_batch,
        })
    }

    /// Applies one action per currently active slot.
    ///
    /// Every action is prepared against its unchanged slot before any slot is
    /// mutated, so a bad policy output cannot partially advance the pool. An
    /// unexpected engine failure after preparation poisons the pool; callers
    /// must discard it instead of continuing from mixed advancement.
    pub fn step_active(
        &mut self,
        actions: Vec<LearningActionV1>,
    ) -> Result<LearningEnvPoolStepV1, LearningEnvPoolError> {
        if self.poisoned {
            return Err(LearningEnvPoolError::PoolPoisoned);
        }
        let active_slot_indices = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot_index, slot)| (!slot.boundary.is_terminal()).then_some(slot_index))
            .collect::<Vec<_>>();
        if actions.len() != active_slot_indices.len() {
            return Err(LearningEnvPoolError::ActionCountMismatch {
                active_slot_count: active_slot_indices.len(),
                action_count: actions.len(),
            });
        }

        let mut prepared = Vec::with_capacity(actions.len());
        for (slot_index, action) in active_slot_indices.iter().copied().zip(actions) {
            let action = self.slots[slot_index]
                .env
                .prepare_action(action)
                .map_err(|message| LearningEnvPoolError::InvalidAction {
                    slot_index,
                    message,
                })?;
            prepared.push(action);
        }

        let mut slots = Vec::with_capacity(prepared.len());
        for (slot_index, action) in active_slot_indices.into_iter().zip(prepared) {
            let step = match self.slots[slot_index].env.step_prepared(action) {
                Ok(step) => step,
                Err(message) => {
                    self.poisoned = true;
                    return Err(LearningEnvPoolError::EngineStep {
                        slot_index,
                        message,
                    });
                }
            };
            slots.push(LearningEnvPoolSlotStepV1 {
                slot_index,
                reward: step.reward,
                terminated: step.terminated,
            });
            self.slots[slot_index].boundary = step.boundary;
        }
        Ok(LearningEnvPoolStepV1 { slots })
    }
}

#[derive(Clone, Debug)]
pub struct LearningEnvPoolModelBatchV1<'a> {
    /// Row `n` in `model_batch` belongs to this environment slot.
    pub active_slot_indices: Vec<usize>,
    pub model_batch: LearningModelBatchV1<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LearningEnvPoolSlotStepV1 {
    pub slot_index: usize,
    pub reward: i8,
    pub terminated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningEnvPoolStepV1 {
    pub slots: Vec<LearningEnvPoolSlotStepV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LearningEnvPoolError {
    InitialObservation {
        slot_index: usize,
        message: String,
    },
    ReplacementObservation {
        slot_index: usize,
        message: String,
    },
    ModelInput(LearningModelInputError),
    SlotIndexOutOfRange {
        slot_index: usize,
        slot_count: usize,
    },
    ActionCountMismatch {
        active_slot_count: usize,
        action_count: usize,
    },
    InvalidAction {
        slot_index: usize,
        message: String,
    },
    EngineStep {
        slot_index: usize,
        message: String,
    },
    PoolPoisoned,
}

impl fmt::Display for LearningEnvPoolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LearningEnvPoolError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::RunControlSession;
    use crate::state::core::RunResult;

    #[test]
    fn empty_pool_is_a_valid_empty_batch() {
        let pool = LearningEnvPoolV1::from_configs([]).expect("create empty pool");
        let batch = pool.active_model_batch().expect("build empty model batch");

        assert_eq!(pool.slot_count(), 0);
        assert!(pool.all_terminal());
        assert!(batch.active_slot_indices.is_empty());
        assert!(batch.model_batch.decisions.is_empty());
        assert_eq!(batch.model_batch.candidate_row_splits, vec![0]);
    }

    #[test]
    fn slot_checkpoint_is_explicit_exact_and_bounds_checked() {
        let pool = LearningEnvPoolV1::from_configs([RunControlConfig {
            seed: 17,
            ..RunControlConfig::default()
        }])
        .expect("create pool");

        assert_eq!(
            pool.checkpoint_slot(0).expect("checkpoint first slot"),
            pool.slots[0].env.checkpoint()
        );
        assert_eq!(
            pool.checkpoint_slot(1)
                .expect_err("missing slot must not produce a checkpoint"),
            LearningEnvPoolError::SlotIndexOutOfRange {
                slot_index: 1,
                slot_count: 1,
            }
        );
    }

    #[test]
    fn invalid_action_is_rejected_before_any_slot_advances() {
        let mut pool = LearningEnvPoolV1::from_configs([
            RunControlConfig {
                seed: 1,
                ..RunControlConfig::default()
            },
            RunControlConfig {
                seed: 2,
                ..RunControlConfig::default()
            },
        ])
        .expect("create pool");
        let before = pool
            .slots
            .iter()
            .map(|slot| slot.env.checkpoint())
            .collect::<Vec<_>>();
        let first_candidate = match &pool.slots[0].boundary {
            LearningBoundaryV1::Strategic { boundary } => {
                boundary.legal_candidates.candidates[0].candidate_id.clone()
            }
            _ => panic!("new run should start at a strategic boundary"),
        };

        assert!(matches!(
            pool.step_active(vec![
                LearningActionV1::StrategicCandidate {
                    candidate_id: first_candidate,
                },
                LearningActionV1::StrategicCandidate {
                    candidate_id: "not-a-legal-candidate".to_string(),
                },
            ]),
            Err(LearningEnvPoolError::InvalidAction { slot_index: 1, .. })
        ));
        assert!(!pool.is_poisoned());
        assert_eq!(
            pool.slots
                .iter()
                .map(|slot| slot.env.checkpoint())
                .collect::<Vec<_>>(),
            before
        );
    }

    #[test]
    fn terminal_slots_drop_out_of_the_next_model_batch() {
        let mut terminal_session = RunControlSession::new(RunControlConfig::default());
        terminal_session.engine_state =
            crate::state::core::EngineState::GameOver(RunResult::Defeat);
        let terminal = LearningEnvV1::from_session(terminal_session);
        let active = LearningEnvV1::new(RunControlConfig {
            seed: 2,
            ..RunControlConfig::default()
        });
        let pool = LearningEnvPoolV1::from_envs([terminal, active]).expect("create mixed pool");
        let batch = pool.active_model_batch().expect("build active batch");

        assert_eq!(pool.slot_count(), 2);
        assert_eq!(pool.terminal_count(), 1);
        assert_eq!(batch.active_slot_indices, vec![1]);
        assert_eq!(batch.model_batch.decisions.len(), 1);
    }

    #[test]
    fn terminal_slot_returns_only_after_an_explicit_reset() {
        let mut terminal_session = RunControlSession::new(RunControlConfig::default());
        terminal_session.engine_state =
            crate::state::core::EngineState::GameOver(RunResult::Defeat);
        let terminal = LearningEnvV1::from_session(terminal_session);
        let mut pool = LearningEnvPoolV1::from_envs([terminal]).expect("create terminal pool");

        assert!(pool.all_terminal());
        pool.reset_slot(
            0,
            RunControlConfig {
                seed: 99,
                ..RunControlConfig::default()
            },
        )
        .expect("explicitly reset terminal slot");
        let batch = pool.active_model_batch().expect("build reset batch");

        assert_eq!(pool.active_count(), 1);
        assert_eq!(batch.active_slot_indices, vec![0]);
        assert_eq!(batch.model_batch.decisions.len(), 1);
    }
}
