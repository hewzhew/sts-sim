use std::time::Instant;

use crate::ai::combat_state_key::combat_exact_state_hash_v1;
use crate::engine::core::is_smoke_escape_stable_boundary;
use crate::sim::combat::{CombatPosition, CombatTerminal};
use crate::state::core::{ClientInput, EngineState};

#[derive(Clone, Debug)]
pub struct CombatDecisionRoot {
    position: CombatPosition,
    exact_state_hash: String,
    turn_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatDecisionRootError {
    NotStablePlayerTurn,
    AlreadyTerminal,
}

impl CombatDecisionRoot {
    pub fn new(position: CombatPosition) -> Result<Self, CombatDecisionRootError> {
        if !matches!(position.engine, EngineState::CombatPlayerTurn) {
            return Err(CombatDecisionRootError::NotStablePlayerTurn);
        }
        if crate::sim::combat::combat_terminal(&position.engine, &position.combat)
            != CombatTerminal::Unresolved
        {
            return Err(CombatDecisionRootError::AlreadyTerminal);
        }
        Ok(Self {
            exact_state_hash: exact_hash(&position),
            turn_count: position.combat.turn.turn_count,
            position,
        })
    }

    pub fn position(&self) -> &CombatPosition {
        &self.position
    }

    pub fn exact_state_hash(&self) -> &str {
        &self.exact_state_hash
    }

    pub fn turn_count(&self) -> u32 {
        self.turn_count
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CombatPlanningCounters {
    pub generation_work: usize,
    pub engine_steps: usize,
}

impl CombatPlanningCounters {
    pub(crate) fn saturating_add(self, other: Self) -> Self {
        Self {
            generation_work: self.generation_work.saturating_add(other.generation_work),
            engine_steps: self.engine_steps.saturating_add(other.engine_steps),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CombatPlanningQuantum {
    pub additional_generation_work: usize,
    pub additional_engine_steps: usize,
    pub deadline: Option<Instant>,
}

impl CombatPlanningQuantum {
    pub fn deterministic(generation_work: usize, engine_steps: usize) -> Self {
        Self {
            additional_generation_work: generation_work,
            additional_engine_steps: engine_steps,
            deadline: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TurnOptionGeneratorConfig {
    /// A transition starts only after this whole allowance is reserved. That
    /// makes splitting a deterministic budget between quanta replay-free.
    pub max_engine_steps_per_transition: usize,
}

impl Default for TurnOptionGeneratorConfig {
    fn default() -> Self {
        Self {
            max_engine_steps_per_transition: 512,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TurnOptionAction {
    pub input: ClientInput,
    pub expected_successor_hash: String,
    pub engine_steps: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompleteTurnOptionBoundary {
    NextPlayerTurn,
    TerminalWin,
    TerminalLoss,
    Escape,
}

#[derive(Clone, Debug)]
pub struct CompleteTurnOption {
    root_exact_state_hash: String,
    actions: Vec<TurnOptionAction>,
    boundary: CompleteTurnOptionBoundary,
    exact_successor_hash: String,
    exact_successor: CombatPosition,
    engine_steps: usize,
}

impl CompleteTurnOption {
    pub(crate) fn new(
        root_exact_state_hash: String,
        actions: Vec<TurnOptionAction>,
        boundary: CompleteTurnOptionBoundary,
        exact_successor: CombatPosition,
    ) -> Self {
        let engine_steps = actions.iter().map(|action| action.engine_steps).sum();
        Self {
            root_exact_state_hash,
            exact_successor_hash: exact_hash(&exact_successor),
            actions,
            boundary,
            exact_successor,
            engine_steps,
        }
    }

    pub fn root_exact_state_hash(&self) -> &str {
        &self.root_exact_state_hash
    }

    pub fn actions(&self) -> &[TurnOptionAction] {
        &self.actions
    }

    pub fn boundary(&self) -> CompleteTurnOptionBoundary {
        self.boundary
    }

    pub fn exact_successor_hash(&self) -> &str {
        &self.exact_successor_hash
    }

    pub fn exact_successor(&self) -> &CombatPosition {
        &self.exact_successor
    }

    pub fn engine_steps(&self) -> usize {
        self.engine_steps
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationInterruption {
    GenerationWorkBudget,
    EngineStepBudget,
    Deadline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnOptionGenerationGapKind {
    UnsupportedStableBoundary,
    UnsupportedStructuredChoice,
    DisabledStructuredChoice,
    EmptyLegalActionSurface,
    GeneratedInputRejected,
    TransitionStepLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnOptionGenerationGap {
    pub kind: TurnOptionGenerationGapKind,
    pub exact_state_hash: String,
    pub action_depth: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnOptionGenerationStatus {
    Complete,
    Partial(GenerationInterruption),
    PartialWithMechanicsGaps,
}

#[derive(Clone, Debug)]
pub struct TurnOptionGenerationReport {
    pub before: CombatPlanningCounters,
    pub after: CombatPlanningCounters,
    pub granted: CombatPlanningCounters,
    pub retained_work_items: usize,
    pub newly_completed_options: usize,
    pub total_completed_options: usize,
    pub gaps: Vec<TurnOptionGenerationGap>,
    pub status: TurnOptionGenerationStatus,
}

pub(crate) fn exact_hash(position: &CombatPosition) -> String {
    combat_exact_state_hash_v1(&position.engine, &position.combat)
}

pub(crate) fn supported_boundary(
    root: &CombatDecisionRoot,
    position: &CombatPosition,
    terminal: CombatTerminal,
) -> Option<CompleteTurnOptionBoundary> {
    if is_smoke_escape_stable_boundary(&position.engine, &position.combat) {
        return Some(CompleteTurnOptionBoundary::Escape);
    }
    match terminal {
        CombatTerminal::Win => Some(CompleteTurnOptionBoundary::TerminalWin),
        CombatTerminal::Loss => Some(CompleteTurnOptionBoundary::TerminalLoss),
        CombatTerminal::Unresolved
            if matches!(position.engine, EngineState::CombatPlayerTurn)
                && position.combat.turn.turn_count > root.turn_count() =>
        {
            Some(CompleteTurnOptionBoundary::NextPlayerTurn)
        }
        CombatTerminal::Unresolved => None,
    }
}
