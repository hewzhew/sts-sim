use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use crate::ai::combat_state_key::{combat_exact_state_key, CombatExactStateKey};
use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};
use crate::state::core::{ClientInput, EngineState};

use super::selection_transaction::SelectionTransactionCursor;
use super::types::{
    exact_hash, supported_boundary, CombatDecisionRoot, CombatPlanningCounters,
    CombatPlanningQuantum, CompleteTurnOption, GenerationInterruption, TurnOptionAction,
    TurnOptionGenerationGap, TurnOptionGenerationGapKind, TurnOptionGenerationReport,
    TurnOptionGenerationStatus, TurnOptionGeneratorConfig,
};

#[derive(Clone, Debug)]
struct PartialTurnOption {
    position: CombatPosition,
    actions: Vec<TurnOptionAction>,
}

#[derive(Clone, Debug)]
struct ActionTransitionWork {
    parent: PartialTurnOption,
    input: ClientInput,
}

#[derive(Clone, Debug)]
struct StructuredSelectionWork {
    parent: PartialTurnOption,
    cursor: SelectionTransactionCursor,
}

#[derive(Clone, Debug)]
enum GeneratorWork {
    Expand(PartialTurnOption),
    ApplyAction(ActionTransitionWork),
    StructuredSelection(StructuredSelectionWork),
}

pub struct TurnOptionGeneratorSession {
    root: CombatDecisionRoot,
    config: TurnOptionGeneratorConfig,
    frontier: VecDeque<GeneratorWork>,
    seen: HashSet<CombatExactStateKey>,
    completed: Vec<CompleteTurnOption>,
    gaps: Vec<TurnOptionGenerationGap>,
    used: CombatPlanningCounters,
    granted: CombatPlanningCounters,
}

impl TurnOptionGeneratorSession {
    pub fn new(root: CombatDecisionRoot, config: TurnOptionGeneratorConfig) -> Self {
        let mut seen = HashSet::new();
        seen.insert(combat_exact_state_key(
            &root.position().engine,
            &root.position().combat,
        ));
        let frontier = VecDeque::from([GeneratorWork::Expand(PartialTurnOption {
            position: root.position().clone(),
            actions: Vec::new(),
        })]);
        Self {
            root,
            config,
            frontier,
            seen,
            completed: Vec::new(),
            gaps: Vec::new(),
            used: CombatPlanningCounters::default(),
            granted: CombatPlanningCounters::default(),
        }
    }

    pub fn root(&self) -> &CombatDecisionRoot {
        &self.root
    }

    pub fn completed_options(&self) -> &[CompleteTurnOption] {
        &self.completed
    }

    pub fn gaps(&self) -> &[TurnOptionGenerationGap] {
        &self.gaps
    }

    pub fn counters(&self) -> CombatPlanningCounters {
        self.used
    }

    pub fn granted_budget(&self) -> CombatPlanningCounters {
        self.granted
    }

    pub fn advance(
        &mut self,
        stepper: &dyn CombatStepper,
        quantum: CombatPlanningQuantum,
    ) -> TurnOptionGenerationReport {
        let before = self.used;
        let completed_before = self.completed.len();
        self.granted = self.granted.saturating_add(CombatPlanningCounters {
            generation_work: quantum.additional_generation_work,
            engine_steps: quantum.additional_engine_steps,
        });

        let interruption = loop {
            if self.frontier.is_empty() {
                break None;
            }
            if deadline_reached(quantum.deadline) {
                break Some(GenerationInterruption::Deadline);
            }
            if self.used.generation_work >= self.granted.generation_work {
                break Some(GenerationInterruption::GenerationWorkBudget);
            }
            let transition_reservation = self.config.max_engine_steps_per_transition.max(1);
            if matches!(self.frontier.front(), Some(GeneratorWork::ApplyAction(_)))
                && self
                    .granted
                    .engine_steps
                    .saturating_sub(self.used.engine_steps)
                    < transition_reservation
            {
                break Some(GenerationInterruption::EngineStepBudget);
            }

            let work = self
                .frontier
                .pop_front()
                .expect("non-empty frontier has front work");
            self.used.generation_work = self.used.generation_work.saturating_add(1);
            match work {
                GeneratorWork::Expand(partial) => self.expand(stepper, partial),
                GeneratorWork::StructuredSelection(mut selection) => {
                    if let Some(input) = selection.cursor.next_input() {
                        if !selection.cursor.is_exhausted() {
                            self.frontier
                                .push_back(GeneratorWork::StructuredSelection(selection.clone()));
                        }
                        self.frontier
                            .push_back(GeneratorWork::ApplyAction(ActionTransitionWork {
                                parent: selection.parent,
                                input,
                            }));
                    }
                }
                GeneratorWork::ApplyAction(action) => {
                    if stepper
                        .choice_for_legal_input(&action.parent.position, &action.input)
                        .is_none()
                    {
                        self.record_gap(
                            TurnOptionGenerationGapKind::GeneratedInputRejected,
                            &action.parent,
                        );
                        continue;
                    }
                    let result = stepper.apply_to_stable(
                        &action.parent.position,
                        action.input.clone(),
                        CombatStepLimits {
                            max_engine_steps: transition_reservation,
                            deadline: quantum.deadline,
                        },
                    );
                    self.used.engine_steps =
                        self.used.engine_steps.saturating_add(result.engine_steps);
                    if result.timed_out {
                        self.frontier.push_front(GeneratorWork::ApplyAction(action));
                        break Some(GenerationInterruption::Deadline);
                    }
                    if result.truncated {
                        self.record_gap(
                            TurnOptionGenerationGapKind::TransitionStepLimit,
                            &action.parent,
                        );
                        continue;
                    }

                    let mut actions = action.parent.actions;
                    actions.push(TurnOptionAction {
                        input: action.input,
                        expected_successor_hash: exact_hash(&result.position),
                        engine_steps: result.engine_steps,
                    });
                    let key =
                        combat_exact_state_key(&result.position.engine, &result.position.combat);
                    if self.seen.insert(key) {
                        self.frontier
                            .push_back(GeneratorWork::Expand(PartialTurnOption {
                                position: result.position,
                                actions,
                            }));
                    }
                }
            }
        };

        let status = if let Some(cause) = interruption {
            TurnOptionGenerationStatus::Partial(cause)
        } else if self.gaps.is_empty() {
            TurnOptionGenerationStatus::Complete
        } else {
            TurnOptionGenerationStatus::PartialWithMechanicsGaps
        };
        TurnOptionGenerationReport {
            before,
            after: self.used,
            granted: self.granted,
            retained_work_items: self.frontier.len(),
            newly_completed_options: self.completed.len().saturating_sub(completed_before),
            total_completed_options: self.completed.len(),
            gaps: self.gaps.clone(),
            status,
        }
    }

    fn expand(&mut self, stepper: &dyn CombatStepper, partial: PartialTurnOption) {
        let terminal = stepper.terminal(&partial.position);
        if let Some(boundary) = supported_boundary(&self.root, &partial.position, terminal) {
            self.completed.push(CompleteTurnOption::new(
                self.root.exact_state_hash().to_owned(),
                partial.actions,
                boundary,
                partial.position,
            ));
            return;
        }

        if terminal != CombatTerminal::Unresolved
            || !matches!(
                partial.position.engine,
                EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
            )
            || (matches!(partial.position.engine, EngineState::CombatPlayerTurn)
                && partial.position.combat.turn.turn_count != self.root.turn_count())
        {
            self.record_gap(
                TurnOptionGenerationGapKind::UnsupportedStableBoundary,
                &partial,
            );
            return;
        }

        let surface = stepper.legal_action_surface(&partial.position);
        let surface_is_empty =
            surface.atomic_actions.is_empty() && surface.selection_families.is_empty();
        for input in surface.atomic_actions {
            self.frontier
                .push_back(GeneratorWork::ApplyAction(ActionTransitionWork {
                    parent: partial.clone(),
                    input,
                }));
        }
        if !surface.selection_families.is_empty()
            && !stepper.supports_canonical_pending_choice_actions()
        {
            self.record_gap(
                TurnOptionGenerationGapKind::UnsupportedStructuredChoice,
                &partial,
            );
        } else {
            for family in surface.selection_families {
                match SelectionTransactionCursor::new(&family) {
                    Ok(cursor) if !cursor.is_exhausted() => {
                        self.frontier.push_back(GeneratorWork::StructuredSelection(
                            StructuredSelectionWork {
                                parent: partial.clone(),
                                cursor,
                            },
                        ));
                    }
                    Ok(_) => {}
                    Err(kind) => self.record_gap(kind, &partial),
                }
            }
        }
        if surface_is_empty {
            self.record_gap(
                TurnOptionGenerationGapKind::EmptyLegalActionSurface,
                &partial,
            );
        }
    }

    fn record_gap(&mut self, kind: TurnOptionGenerationGapKind, partial: &PartialTurnOption) {
        self.gaps.push(TurnOptionGenerationGap {
            kind,
            exact_state_hash: exact_hash(&partial.position),
            action_depth: partial.actions.len(),
        });
    }
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}
