use std::collections::VecDeque;
use std::time::Instant;

use sts_core::sim::combat::CombatStepper;

use super::evidence::{
    BoundaryWitnessEvidence, ContinuationEvidence, ContinuationInterruption, ExactHorizonEvidence,
    ExactHorizonGenerationGapEvidence, OptionProspect, OptionProspectId,
};
use super::replay::replay_turn_option_observed;
use super::{
    CombatDecisionRoot, CombatPlanningQuantum, CompleteTurnOptionBoundary,
    ExactImmediateOptionProspect, GenerationInterruption, ReplayError, ReplayLimits,
    TurnOptionGenerationGap, TurnOptionGenerationStatus, TurnOptionGeneratorConfig,
    TurnOptionGeneratorSession,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CombatPlannerAgendaConfig {
    pub generator: TurnOptionGeneratorConfig,
    pub generation_work_per_item: usize,
}

impl Default for CombatPlannerAgendaConfig {
    fn default() -> Self {
        Self {
            generator: TurnOptionGeneratorConfig::default(),
            generation_work_per_item: 8,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CombatPlannerAgendaQuantum {
    pub additional_agenda_items: usize,
    pub additional_generation_work: usize,
    pub additional_engine_steps: usize,
    pub deadline: Option<Instant>,
}

impl CombatPlannerAgendaQuantum {
    pub fn deterministic(agenda_items: usize, generation_work: usize, engine_steps: usize) -> Self {
        Self {
            additional_agenda_items: agenda_items,
            additional_generation_work: generation_work,
            additional_engine_steps: engine_steps,
            deadline: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CombatPlannerAgendaCounters {
    pub agenda_items: usize,
    pub root_generation_work: usize,
    pub continuation_generation_work: usize,
    pub boundary_witness_replays: usize,
    pub engine_steps: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CombatPlannerAgendaBudget {
    pub agenda_items: usize,
    pub generation_work: usize,
    pub engine_steps: usize,
}

impl CombatPlannerAgendaBudget {
    fn saturating_add_quantum(self, quantum: CombatPlannerAgendaQuantum) -> Self {
        Self {
            agenda_items: self
                .agenda_items
                .saturating_add(quantum.additional_agenda_items),
            generation_work: self
                .generation_work
                .saturating_add(quantum.additional_generation_work),
            engine_steps: self
                .engine_steps
                .saturating_add(quantum.additional_engine_steps),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatPlannerAgendaInterruption {
    AgendaItemBudget,
    GenerationWorkBudget,
    EngineStepBudget,
    Deadline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatPlannerAgendaStatus {
    EvidenceComplete,
    Partial(CombatPlannerAgendaInterruption),
    PartialWithGenerationGaps,
    PartialWithContinuationGaps,
    PartialWithVerificationGaps,
}

#[derive(Clone, Debug)]
pub struct CombatPlannerAgendaReport {
    pub before: CombatPlannerAgendaCounters,
    pub after: CombatPlannerAgendaCounters,
    pub granted: CombatPlannerAgendaBudget,
    pub newly_discovered_prospects: usize,
    pub total_prospects: usize,
    pub retained_agenda_items: usize,
    pub generation_gaps: Vec<TurnOptionGenerationGap>,
    pub status: CombatPlannerAgendaStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AgendaItem {
    DiscoverTurnOption,
    ExtendOneTurn(OptionProspectId),
    VerifyBoundaryWitness(OptionProspectId),
}

pub struct CombatPlannerAgendaSession {
    root: CombatDecisionRoot,
    generator: TurnOptionGeneratorSession,
    config: CombatPlannerAgendaConfig,
    agenda: VecDeque<AgendaItem>,
    prospects: Vec<OptionProspect>,
    continuations: Vec<Option<TurnOptionGeneratorSession>>,
    synced_options: usize,
    granted: CombatPlannerAgendaBudget,
    committed_generation_work: usize,
    committed_engine_steps: usize,
    agenda_items_used: usize,
    witness_replays: usize,
    witness_engine_steps: usize,
    continuation_generation_work: usize,
    continuation_engine_steps: usize,
}

impl CombatPlannerAgendaSession {
    pub fn new(root: CombatDecisionRoot, config: CombatPlannerAgendaConfig) -> Self {
        let generator = TurnOptionGeneratorSession::new(root.clone(), config.generator);
        Self {
            root,
            generator,
            config,
            agenda: VecDeque::from([AgendaItem::DiscoverTurnOption]),
            prospects: Vec::new(),
            continuations: Vec::new(),
            synced_options: 0,
            granted: CombatPlannerAgendaBudget::default(),
            committed_generation_work: 0,
            committed_engine_steps: 0,
            agenda_items_used: 0,
            witness_replays: 0,
            witness_engine_steps: 0,
            continuation_generation_work: 0,
            continuation_engine_steps: 0,
        }
    }

    pub fn root(&self) -> &CombatDecisionRoot {
        &self.root
    }

    pub fn prospects(&self) -> &[OptionProspect] {
        &self.prospects
    }

    pub(crate) fn retained_agenda_items(&self) -> usize {
        self.agenda.len()
    }

    pub(crate) fn root_generation_gaps(&self) -> &[TurnOptionGenerationGap] {
        self.generator.gaps()
    }

    #[cfg(test)]
    pub(crate) fn committed_budget_for_test(&self) -> CombatPlannerAgendaBudget {
        CombatPlannerAgendaBudget {
            agenda_items: self.agenda_items_used,
            generation_work: self.committed_generation_work,
            engine_steps: self.committed_engine_steps,
        }
    }

    pub fn counters(&self) -> CombatPlannerAgendaCounters {
        let generation = self.generator.counters();
        CombatPlannerAgendaCounters {
            agenda_items: self.agenda_items_used,
            root_generation_work: generation.generation_work,
            continuation_generation_work: self.continuation_generation_work,
            boundary_witness_replays: self.witness_replays,
            engine_steps: generation
                .engine_steps
                .saturating_add(self.continuation_engine_steps)
                .saturating_add(self.witness_engine_steps),
        }
    }

    pub fn advance(
        &mut self,
        stepper: &dyn CombatStepper,
        quantum: CombatPlannerAgendaQuantum,
    ) -> CombatPlannerAgendaReport {
        let before = self.counters();
        let prospects_before = self.prospects.len();
        self.granted = self.granted.saturating_add_quantum(quantum);
        let interruption = loop {
            if self.agenda.is_empty() {
                break None;
            }
            if deadline_reached(quantum.deadline) {
                break Some(CombatPlannerAgendaInterruption::Deadline);
            }
            if self.agenda_items_used >= self.granted.agenda_items {
                break Some(CombatPlannerAgendaInterruption::AgendaItemBudget);
            }
            let item = self.agenda.pop_front().expect("checked non-empty agenda");
            match item {
                AgendaItem::DiscoverTurnOption => {
                    let remaining_generation = self
                        .granted
                        .generation_work
                        .saturating_sub(self.committed_generation_work);
                    let generator_granted = self.generator.granted_budget();
                    let generator_used = self.generator.counters();
                    let available_generation = generator_granted
                        .generation_work
                        .saturating_sub(generator_used.generation_work);
                    let desired_generation = self.config.generation_work_per_item.max(1);
                    let generation_grant = desired_generation
                        .saturating_sub(available_generation)
                        .min(remaining_generation);
                    if available_generation.saturating_add(generation_grant) == 0 {
                        self.agenda.push_front(item);
                        break Some(CombatPlannerAgendaInterruption::GenerationWorkBudget);
                    }
                    let remaining_engine = self
                        .granted
                        .engine_steps
                        .saturating_sub(self.committed_engine_steps);
                    let transition_reservation =
                        self.config.generator.max_engine_steps_per_transition.max(1);
                    let available_engine = generator_granted
                        .engine_steps
                        .saturating_sub(generator_used.engine_steps);
                    let required_engine_grant =
                        transition_reservation.saturating_sub(available_engine);
                    let engine_grant = (remaining_engine >= required_engine_grant)
                        .then_some(required_engine_grant)
                        .unwrap_or(0);
                    self.committed_generation_work = self
                        .committed_generation_work
                        .saturating_add(generation_grant);
                    self.committed_engine_steps =
                        self.committed_engine_steps.saturating_add(engine_grant);
                    self.agenda_items_used = self.agenda_items_used.saturating_add(1);
                    let generation_report = self.generator.advance(
                        stepper,
                        CombatPlanningQuantum {
                            additional_generation_work: generation_grant,
                            additional_engine_steps: engine_grant,
                            deadline: quantum.deadline,
                        },
                    );
                    self.sync_new_options();
                    let root_finished = self.generator.is_finished();
                    let released = self.generator.release_unused_grant();
                    self.committed_generation_work = self
                        .committed_generation_work
                        .saturating_sub(released.generation_work);
                    self.committed_engine_steps = self
                        .committed_engine_steps
                        .saturating_sub(released.engine_steps);
                    if !root_finished {
                        // Comparable continuation evidence is meaningful only after every
                        // complete option at this decision root has had a chance to exist.
                        self.agenda.push_front(AgendaItem::DiscoverTurnOption);
                    }

                    match generation_report.status {
                        TurnOptionGenerationStatus::Partial(GenerationInterruption::Deadline) => {
                            break Some(CombatPlannerAgendaInterruption::Deadline);
                        }
                        TurnOptionGenerationStatus::Partial(
                            GenerationInterruption::EngineStepBudget,
                        ) if self
                            .granted
                            .engine_steps
                            .saturating_sub(self.committed_engine_steps)
                            < transition_reservation =>
                        {
                            break Some(CombatPlannerAgendaInterruption::EngineStepBudget);
                        }
                        TurnOptionGenerationStatus::Partial(
                            GenerationInterruption::GenerationWorkBudget,
                        ) if self
                            .granted
                            .generation_work
                            .saturating_sub(self.committed_generation_work)
                            == 0 =>
                        {
                            break Some(CombatPlannerAgendaInterruption::GenerationWorkBudget);
                        }
                        _ => {}
                    }
                }
                AgendaItem::ExtendOneTurn(id) => {
                    let index = id.0 as usize;
                    let mut generator = self.continuations[index]
                        .take()
                        .expect("scheduled continuation keeps its generator");
                    let remaining_generation = self
                        .granted
                        .generation_work
                        .saturating_sub(self.committed_generation_work);
                    let remaining_engine = self
                        .granted
                        .engine_steps
                        .saturating_sub(self.committed_engine_steps);
                    let Some(grant) = subgenerator_grant(
                        &generator,
                        self.config,
                        remaining_generation,
                        remaining_engine,
                        quantum.deadline,
                    ) else {
                        self.continuations[index] = Some(generator);
                        self.prospect_mut(id)
                            .set_continuation(ContinuationEvidence::Interrupted(
                                ContinuationInterruption::GenerationWorkBudget,
                            ));
                        self.agenda.push_front(item);
                        break Some(CombatPlannerAgendaInterruption::GenerationWorkBudget);
                    };
                    self.committed_generation_work = self
                        .committed_generation_work
                        .saturating_add(grant.additional_generation_work);
                    self.committed_engine_steps = self
                        .committed_engine_steps
                        .saturating_add(grant.additional_engine_steps);
                    self.agenda_items_used = self.agenda_items_used.saturating_add(1);
                    let before_continuation = generator.counters();
                    let continuation_report = generator.advance(stepper, grant);
                    let after_continuation = generator.counters();
                    self.continuation_generation_work =
                        self.continuation_generation_work.saturating_add(
                            after_continuation
                                .generation_work
                                .saturating_sub(before_continuation.generation_work),
                        );
                    self.continuation_engine_steps = self.continuation_engine_steps.saturating_add(
                        after_continuation
                            .engine_steps
                            .saturating_sub(before_continuation.engine_steps),
                    );

                    let continuation_finished = generator.is_finished();
                    let released = generator.release_unused_grant();
                    self.committed_generation_work = self
                        .committed_generation_work
                        .saturating_sub(released.generation_work);
                    self.committed_engine_steps = self
                        .committed_engine_steps
                        .saturating_sub(released.engine_steps);
                    if continuation_finished {
                        let work = generator.counters();
                        let complete_options = generator.completed_options().to_vec();
                        if generator.gaps().is_empty() {
                            self.prospect_mut(id).set_continuation(
                                ContinuationEvidence::ExactHorizon(ExactHorizonEvidence {
                                    turn_boundaries: 1,
                                    complete_options,
                                    work,
                                }),
                            );
                        } else {
                            self.prospect_mut(id).set_continuation(
                                ContinuationEvidence::ExactHorizonGenerationGap(
                                    ExactHorizonGenerationGapEvidence {
                                        requested_turn_boundaries: 1,
                                        complete_options,
                                        gaps: generator.gaps().to_vec(),
                                        work,
                                    },
                                ),
                            );
                        }
                    } else {
                        let interruption = match continuation_report.status {
                            TurnOptionGenerationStatus::Partial(
                                GenerationInterruption::EngineStepBudget,
                            ) => ContinuationInterruption::EngineStepBudget,
                            TurnOptionGenerationStatus::Partial(
                                GenerationInterruption::Deadline,
                            ) => ContinuationInterruption::Deadline,
                            _ => ContinuationInterruption::GenerationWorkBudget,
                        };
                        self.prospect_mut(id)
                            .set_continuation(ContinuationEvidence::Interrupted(interruption));
                        self.continuations[index] = Some(generator);
                        self.agenda.push_back(item);
                    }

                    match continuation_report.status {
                        TurnOptionGenerationStatus::Partial(GenerationInterruption::Deadline) => {
                            break Some(CombatPlannerAgendaInterruption::Deadline);
                        }
                        TurnOptionGenerationStatus::Partial(
                            GenerationInterruption::EngineStepBudget,
                        ) => {
                            break Some(CombatPlannerAgendaInterruption::EngineStepBudget);
                        }
                        TurnOptionGenerationStatus::Partial(
                            GenerationInterruption::GenerationWorkBudget,
                        ) if self
                            .granted
                            .generation_work
                            .saturating_sub(self.committed_generation_work)
                            == 0 =>
                        {
                            break Some(CombatPlannerAgendaInterruption::GenerationWorkBudget);
                        }
                        _ => {}
                    }
                }
                AgendaItem::VerifyBoundaryWitness(id) => {
                    let option = self.prospect(id).option().clone();
                    let required_engine_steps = option.engine_steps();
                    if self
                        .granted
                        .engine_steps
                        .saturating_sub(self.committed_engine_steps)
                        < required_engine_steps
                    {
                        self.prospect_mut(id)
                            .set_continuation(ContinuationEvidence::Interrupted(
                                ContinuationInterruption::EngineStepBudget,
                            ));
                        self.agenda.push_front(item);
                        break Some(CombatPlannerAgendaInterruption::EngineStepBudget);
                    }
                    self.committed_engine_steps = self
                        .committed_engine_steps
                        .saturating_add(required_engine_steps);
                    self.agenda_items_used = self.agenda_items_used.saturating_add(1);
                    match replay_turn_option_observed(
                        &self.root,
                        &option,
                        stepper,
                        ReplayLimits {
                            max_engine_steps: required_engine_steps,
                            deadline: quantum.deadline,
                        },
                    ) {
                        Ok(replay) => {
                            self.witness_replays = self.witness_replays.saturating_add(1);
                            self.witness_engine_steps = self
                                .witness_engine_steps
                                .saturating_add(replay.engine_steps);
                            self.prospect_mut(id).set_continuation(
                                ContinuationEvidence::VerifiedBoundary(BoundaryWitnessEvidence {
                                    boundary: replay.boundary,
                                    exact_successor_hash: option.exact_successor_hash().to_owned(),
                                    replay_engine_steps: replay.engine_steps,
                                }),
                            );
                        }
                        Err(failure) if failure.error == ReplayError::Deadline => {
                            self.witness_engine_steps = self
                                .witness_engine_steps
                                .saturating_add(failure.engine_steps);
                            self.committed_engine_steps =
                                self.committed_engine_steps.saturating_sub(
                                    required_engine_steps.saturating_sub(failure.engine_steps),
                                );
                            self.prospect_mut(id).set_continuation(
                                ContinuationEvidence::Interrupted(
                                    ContinuationInterruption::Deadline,
                                ),
                            );
                            self.agenda.push_front(item);
                            break Some(CombatPlannerAgendaInterruption::Deadline);
                        }
                        Err(failure) => {
                            self.witness_engine_steps = self
                                .witness_engine_steps
                                .saturating_add(failure.engine_steps);
                            self.committed_engine_steps =
                                self.committed_engine_steps.saturating_sub(
                                    required_engine_steps.saturating_sub(failure.engine_steps),
                                );
                            self.prospect_mut(id).set_continuation(
                                ContinuationEvidence::VerificationFailed(failure.error),
                            );
                        }
                    }
                }
            }
        };

        let status = if let Some(cause) = interruption {
            CombatPlannerAgendaStatus::Partial(cause)
        } else if !self.generator.gaps().is_empty() {
            CombatPlannerAgendaStatus::PartialWithGenerationGaps
        } else if self.prospects.iter().any(|prospect| {
            matches!(
                prospect.continuation(),
                ContinuationEvidence::ExactHorizonGenerationGap(_)
                    | ContinuationEvidence::ConstructionFailed(_)
            )
        }) {
            CombatPlannerAgendaStatus::PartialWithContinuationGaps
        } else if self.prospects.iter().any(|prospect| {
            matches!(
                prospect.continuation(),
                ContinuationEvidence::VerificationFailed(_)
            )
        }) {
            CombatPlannerAgendaStatus::PartialWithVerificationGaps
        } else {
            CombatPlannerAgendaStatus::EvidenceComplete
        };
        CombatPlannerAgendaReport {
            before,
            after: self.counters(),
            granted: self.granted,
            newly_discovered_prospects: self.prospects.len().saturating_sub(prospects_before),
            total_prospects: self.prospects.len(),
            retained_agenda_items: self.agenda.len(),
            generation_gaps: self.generator.gaps().to_vec(),
            status,
        }
    }

    fn sync_new_options(&mut self) {
        while self.synced_options < self.generator.completed_options().len() {
            let option = self.generator.completed_options()[self.synced_options].clone();
            let id = OptionProspectId(u64::try_from(self.prospects.len()).unwrap_or(u64::MAX));
            let immediate = ExactImmediateOptionProspect::from_option(&self.root, &option)
                .expect("generator option belongs to the agenda root");
            let (continuation, continuation_generator) = match option.boundary() {
                CompleteTurnOptionBoundary::TerminalWin
                | CompleteTurnOptionBoundary::TerminalLoss
                | CompleteTurnOptionBoundary::Escape => {
                    self.agenda.push_back(AgendaItem::VerifyBoundaryWitness(id));
                    (ContinuationEvidence::PendingBoundaryVerification, None)
                }
                CompleteTurnOptionBoundary::NextPlayerTurn => {
                    match CombatDecisionRoot::new(option.exact_successor().clone()) {
                        Ok(root) => {
                            self.agenda.push_back(AgendaItem::ExtendOneTurn(id));
                            (
                                ContinuationEvidence::PendingContinuationRefinement,
                                Some(TurnOptionGeneratorSession::new(root, self.config.generator)),
                            )
                        }
                        Err(error) => (ContinuationEvidence::ConstructionFailed(error), None),
                    }
                }
            };
            self.prospects
                .push(OptionProspect::new(id, option, immediate, continuation));
            self.continuations.push(continuation_generator);
            self.synced_options = self.synced_options.saturating_add(1);
        }
    }

    fn prospect(&self, id: OptionProspectId) -> &OptionProspect {
        &self.prospects[id.0 as usize]
    }

    fn prospect_mut(&mut self, id: OptionProspectId) -> &mut OptionProspect {
        &mut self.prospects[id.0 as usize]
    }
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

fn subgenerator_grant(
    generator: &TurnOptionGeneratorSession,
    config: CombatPlannerAgendaConfig,
    remaining_generation: usize,
    remaining_engine: usize,
    deadline: Option<Instant>,
) -> Option<CombatPlanningQuantum> {
    let granted = generator.granted_budget();
    let used = generator.counters();
    let available_generation = granted.generation_work.saturating_sub(used.generation_work);
    let generation_grant = config
        .generation_work_per_item
        .max(1)
        .saturating_sub(available_generation)
        .min(remaining_generation);
    if available_generation.saturating_add(generation_grant) == 0 {
        return None;
    }
    let available_engine = granted.engine_steps.saturating_sub(used.engine_steps);
    let transition_reservation = config.generator.max_engine_steps_per_transition.max(1);
    let required_engine = transition_reservation.saturating_sub(available_engine);
    let engine_grant = (remaining_engine >= required_engine)
        .then_some(required_engine)
        .unwrap_or(0);
    Some(CombatPlanningQuantum {
        additional_generation_work: generation_grant,
        additional_engine_steps: engine_grant,
        deadline,
    })
}
