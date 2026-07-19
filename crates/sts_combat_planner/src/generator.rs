use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::time::Instant;

use sts_core::ai::combat_state_key::{combat_exact_state_key, CombatExactStateKey};
use sts_core::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};
use sts_core::state::core::{ClientInput, EngineState};

use super::policy::{
    normalized_probabilities, uniform_policy, CombatPolicyChoice, CombatStateGuideRank,
    SharedCombatActionPolicy,
};
use super::selection_transaction::SelectionTransactionCursor;
use super::types::{
    exact_hash, supported_boundary, CombatDecisionRoot, CombatPlanningCounters,
    CombatPlanningQuantum, CompleteTurnOption, GenerationInterruption, TurnOptionAction,
    TurnOptionGenerationDiagnostics, TurnOptionGenerationGap, TurnOptionGenerationGapKind,
    TurnOptionGenerationReport, TurnOptionGenerationStatus, TurnOptionGeneratorConfig,
};

#[derive(Clone, Debug)]
struct PartialTurnOption {
    position: CombatPosition,
    actions: Vec<TurnOptionAction>,
    atomic_depth: usize,
    negative_log_policy: f64,
}

#[derive(Clone, Debug)]
struct ActionTransitionWork {
    parent: PartialTurnOption,
    input: ClientInput,
    atomic_depth: usize,
    negative_log_policy: f64,
}

#[derive(Clone, Debug)]
struct StructuredSelectionWork {
    parent: PartialTurnOption,
    cursor: SelectionTransactionCursor,
    family_negative_log_policy: f64,
    remaining_conditional_mass: f64,
}

#[derive(Clone, Debug)]
enum GeneratorWork {
    Expand(PartialTurnOption),
    ApplyAction(ActionTransitionWork),
    StructuredSelection(StructuredSelectionWork),
}

impl GeneratorWork {
    fn position(&self) -> &CombatPosition {
        match self {
            Self::Expand(partial) => &partial.position,
            Self::ApplyAction(action) => &action.parent.position,
            Self::StructuredSelection(selection) => &selection.parent.position,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GeneratorWorkPriority {
    levin_log_priority: f64,
    atomic_depth: usize,
    negative_log_policy: f64,
}

impl GeneratorWorkPriority {
    fn for_path(atomic_depth: usize, negative_log_policy: f64) -> Self {
        Self {
            levin_log_priority: (atomic_depth.max(1) as f64).ln() + negative_log_policy,
            atomic_depth,
            negative_log_policy,
        }
    }
}

impl Eq for GeneratorWorkPriority {}

impl PartialEq for GeneratorWorkPriority {
    fn eq(&self, other: &Self) -> bool {
        self.levin_log_priority.to_bits() == other.levin_log_priority.to_bits()
    }
}

impl Ord for GeneratorWorkPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is a max-heap; reverse the finite Levin cost so the least
        // expensive retained path is selected first.
        other.levin_log_priority.total_cmp(&self.levin_log_priority)
    }
}

impl PartialOrd for GeneratorWorkPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

struct GeneratorQueueEntry {
    priority: GeneratorWorkPriority,
    sequence_id: u64,
    work_id: usize,
}

#[derive(Clone, Debug)]
struct GuidedGeneratorQueueEntry {
    guide_index: usize,
    work_id: usize,
    sequence_id: u64,
    guide_rank: CombatStateGuideRank,
    anchor_priority: GeneratorWorkPriority,
}

impl Eq for GuidedGeneratorQueueEntry {}

impl PartialEq for GuidedGeneratorQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.guide_index == other.guide_index
            && self.work_id == other.work_id
            && self.sequence_id == other.sequence_id
    }
}

impl Ord for GuidedGeneratorQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.guide_rank
            .cmp(&other.guide_rank)
            .then_with(|| self.anchor_priority.cmp(&other.anchor_priority))
            .then_with(|| other.sequence_id.cmp(&self.sequence_id))
    }
}

impl PartialOrd for GuidedGeneratorQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for GeneratorQueueEntry {}

impl PartialEq for GeneratorQueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence_id == other.sequence_id
    }
}

impl Ord for GeneratorQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence_id.cmp(&self.sequence_id))
    }
}

impl PartialOrd for GeneratorQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct TurnOptionGeneratorSession {
    root: CombatDecisionRoot,
    config: TurnOptionGeneratorConfig,
    policy: SharedCombatActionPolicy,
    work: Vec<Option<GeneratorWork>>,
    anchor_frontier: BinaryHeap<GeneratorQueueEntry>,
    guided_frontiers: Vec<BinaryHeap<GuidedGeneratorQueueEntry>>,
    next_scheduler_lane: usize,
    live_work_items: usize,
    next_sequence_id: u64,
    seen: HashSet<CombatExactStateKey>,
    completed: Vec<CompleteTurnOption>,
    gaps: Vec<TurnOptionGenerationGap>,
    applied_action_transitions: usize,
    duplicate_exact_successors: usize,
    used: CombatPlanningCounters,
    granted: CombatPlanningCounters,
}

impl TurnOptionGeneratorSession {
    pub fn new(root: CombatDecisionRoot, config: TurnOptionGeneratorConfig) -> Self {
        Self::with_policy(root, config, uniform_policy())
    }

    pub fn with_policy(
        root: CombatDecisionRoot,
        config: TurnOptionGeneratorConfig,
        policy: SharedCombatActionPolicy,
    ) -> Self {
        let mut seen = HashSet::new();
        seen.insert(combat_exact_state_key(
            &root.position().engine,
            &root.position().combat,
        ));
        let root_work = GeneratorWork::Expand(PartialTurnOption {
            position: root.position().clone(),
            actions: Vec::new(),
            atomic_depth: 0,
            negative_log_policy: 0.0,
        });
        let mut session = Self {
            root,
            config,
            policy,
            work: Vec::new(),
            anchor_frontier: BinaryHeap::new(),
            guided_frontiers: Vec::new(),
            next_scheduler_lane: 0,
            live_work_items: 0,
            next_sequence_id: 0,
            seen,
            completed: Vec::new(),
            gaps: Vec::new(),
            applied_action_transitions: 0,
            duplicate_exact_successors: 0,
            used: CombatPlanningCounters::default(),
            granted: CombatPlanningCounters::default(),
        };
        session.push_work(root_work, GeneratorWorkPriority::for_path(0, 0.0));
        session
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

    pub fn retained_work_items(&self) -> usize {
        self.live_work_items
    }

    pub fn diagnostics(&self) -> TurnOptionGenerationDiagnostics {
        TurnOptionGenerationDiagnostics {
            applied_action_transitions: self.applied_action_transitions,
            unique_successor_states: self.seen.len().saturating_sub(1),
            duplicate_exact_successors: self.duplicate_exact_successors,
            completed_turn_options: self.completed.len(),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.live_work_items == 0
    }

    pub(crate) fn best_retained_path_bound(&mut self) -> Option<(usize, f64)> {
        while let Some(entry) = self.anchor_frontier.peek() {
            if self.work.get(entry.work_id).is_some_and(Option::is_some) {
                return Some((
                    entry.priority.atomic_depth,
                    entry.priority.negative_log_policy,
                ));
            }
            self.anchor_frontier.pop();
        }
        None
    }

    pub(crate) fn best_retained_path_bound_snapshot(&self) -> Option<(usize, f64)> {
        self.anchor_frontier
            .iter()
            .filter(|entry| self.work.get(entry.work_id).is_some_and(Option::is_some))
            .min_by(|left, right| {
                left.priority
                    .levin_log_priority
                    .total_cmp(&right.priority.levin_log_priority)
                    .then_with(|| {
                        left.priority
                            .negative_log_policy
                            .total_cmp(&right.priority.negative_log_policy)
                    })
                    .then_with(|| left.priority.atomic_depth.cmp(&right.priority.atomic_depth))
            })
            .map(|entry| {
                (
                    entry.priority.atomic_depth,
                    entry.priority.negative_log_policy,
                )
            })
    }

    pub(crate) fn release_unused_grant(&mut self) -> CombatPlanningCounters {
        let released = CombatPlanningCounters {
            generation_work: self
                .granted
                .generation_work
                .saturating_sub(self.used.generation_work),
            engine_steps: self
                .granted
                .engine_steps
                .saturating_sub(self.used.engine_steps),
        };
        self.granted = self.used;
        released
    }

    pub fn advance(
        &mut self,
        stepper: &dyn CombatStepper,
        quantum: CombatPlanningQuantum,
    ) -> TurnOptionGenerationReport {
        let before = self.used;
        let before_diagnostics = self.diagnostics();
        let completed_before = self.completed.len();
        self.granted = self.granted.saturating_add(CombatPlanningCounters {
            generation_work: quantum.additional_generation_work,
            engine_steps: quantum.additional_engine_steps,
        });

        let interruption = loop {
            if self.is_finished() {
                break None;
            }
            if deadline_reached(quantum.deadline) {
                break Some(GenerationInterruption::Deadline);
            }
            if self.used.generation_work >= self.granted.generation_work {
                break Some(GenerationInterruption::GenerationWorkBudget);
            }
            let transition_reservation = self.config.max_engine_steps_per_transition.max(1);
            if self.next_scheduled_work_is_apply_action()
                && self
                    .granted
                    .engine_steps
                    .saturating_sub(self.used.engine_steps)
                    < transition_reservation
            {
                break Some(GenerationInterruption::EngineStepBudget);
            }

            let work = self
                .pop_scheduled_work()
                .expect("non-empty generator has scheduled work");
            self.used.generation_work = self.used.generation_work.saturating_add(1);
            match work {
                GeneratorWork::Expand(partial) => self.expand(stepper, partial),
                GeneratorWork::StructuredSelection(mut selection) => {
                    if let Some(input) = selection.cursor.next_input() {
                        let input_conditional_mass = if selection.cursor.is_exhausted() {
                            selection.remaining_conditional_mass
                        } else {
                            selection.remaining_conditional_mass * 0.5
                        };
                        if !selection.cursor.is_exhausted() {
                            selection.remaining_conditional_mass -= input_conditional_mass;
                            let residual_negative_log = selection.family_negative_log_policy
                                - selection.remaining_conditional_mass.ln();
                            let residual_priority = GeneratorWorkPriority::for_path(
                                selection.parent.atomic_depth.saturating_add(1),
                                residual_negative_log,
                            );
                            self.push_work(
                                GeneratorWork::StructuredSelection(selection.clone()),
                                residual_priority,
                            );
                        }
                        let negative_log_policy =
                            selection.family_negative_log_policy - input_conditional_mass.ln();
                        let atomic_depth = selection.parent.atomic_depth.saturating_add(1);
                        let priority =
                            GeneratorWorkPriority::for_path(atomic_depth, negative_log_policy);
                        self.push_work(
                            GeneratorWork::ApplyAction(ActionTransitionWork {
                                parent: selection.parent,
                                input,
                                atomic_depth,
                                negative_log_policy,
                            }),
                            priority,
                        );
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
                        let priority = GeneratorWorkPriority::for_path(
                            action.atomic_depth,
                            action.negative_log_policy,
                        );
                        self.push_work(GeneratorWork::ApplyAction(action), priority);
                        break Some(GenerationInterruption::Deadline);
                    }
                    if result.truncated {
                        self.record_gap(
                            TurnOptionGenerationGapKind::TransitionStepLimit,
                            &action.parent,
                        );
                        continue;
                    }

                    self.applied_action_transitions =
                        self.applied_action_transitions.saturating_add(1);

                    let mut actions = action.parent.actions;
                    actions.push(TurnOptionAction {
                        input: action.input,
                        expected_successor_hash: exact_hash(&result.position),
                        engine_steps: result.engine_steps,
                    });
                    let key =
                        combat_exact_state_key(&result.position.engine, &result.position.combat);
                    if self.seen.insert(key) {
                        let priority = GeneratorWorkPriority::for_path(
                            action.atomic_depth,
                            action.negative_log_policy,
                        );
                        self.push_work(
                            GeneratorWork::Expand(PartialTurnOption {
                                position: result.position,
                                actions,
                                atomic_depth: action.atomic_depth,
                                negative_log_policy: action.negative_log_policy,
                            }),
                            priority,
                        );
                    } else {
                        self.duplicate_exact_successors =
                            self.duplicate_exact_successors.saturating_add(1);
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
            before_diagnostics,
            after_diagnostics: self.diagnostics(),
            retained_work_items: self.live_work_items,
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
                partial.negative_log_policy,
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
        let policy_choices = surface
            .atomic_actions
            .iter()
            .map(CombatPolicyChoice::Atomic)
            .chain(
                surface
                    .selection_families
                    .iter()
                    .map(CombatPolicyChoice::StructuredSelection),
            )
            .collect::<Vec<_>>();
        let weights = self.policy.weights(&partial.position, &policy_choices);
        let weights = (weights.len() == policy_choices.len())
            .then_some(weights)
            .unwrap_or_else(|| vec![1.0; policy_choices.len()]);
        let probabilities = normalized_probabilities(weights, self.config.uniform_exploration_ppm);
        let mut probabilities = probabilities.into_iter();
        for input in surface.atomic_actions {
            let probability = probabilities.next().expect("one probability per action");
            let negative_log_policy = partial.negative_log_policy - probability.ln();
            let atomic_depth = partial.atomic_depth.saturating_add(1);
            self.push_work(
                GeneratorWork::ApplyAction(ActionTransitionWork {
                    parent: partial.clone(),
                    input,
                    atomic_depth,
                    negative_log_policy,
                }),
                GeneratorWorkPriority::for_path(atomic_depth, negative_log_policy),
            );
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
                let probability = probabilities
                    .next()
                    .expect("one probability per structured family");
                match SelectionTransactionCursor::new(&family) {
                    Ok(cursor) if !cursor.is_exhausted() => {
                        let family_negative_log_policy =
                            partial.negative_log_policy - probability.ln();
                        self.push_work(
                            GeneratorWork::StructuredSelection(StructuredSelectionWork {
                                parent: partial.clone(),
                                cursor,
                                family_negative_log_policy,
                                remaining_conditional_mass: 1.0,
                            }),
                            GeneratorWorkPriority::for_path(
                                partial.atomic_depth.saturating_add(1),
                                family_negative_log_policy,
                            ),
                        );
                    }
                    Ok(_) => {}
                    Err(kind) => self.record_gap(kind, &partial),
                }
            }
        }
        debug_assert!(probabilities.next().is_none());
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

    fn push_work(&mut self, work: GeneratorWork, priority: GeneratorWorkPriority) {
        debug_assert!(priority.levin_log_priority.is_finite());
        let guide_ranks = self.policy.state_guide_ranks(work.position());
        let work_id = self.work.len();
        self.work.push(Some(work));
        let entry = GeneratorQueueEntry {
            priority,
            sequence_id: self.next_sequence_id,
            work_id,
        };
        self.anchor_frontier.push(entry);
        if self.guided_frontiers.len() < guide_ranks.len() {
            self.guided_frontiers
                .resize_with(guide_ranks.len(), BinaryHeap::new);
        }
        for (guide_index, guide_rank) in guide_ranks.into_iter().enumerate() {
            self.guided_frontiers[guide_index].push(GuidedGeneratorQueueEntry {
                guide_index,
                work_id,
                sequence_id: self.next_sequence_id,
                guide_rank,
                anchor_priority: priority,
            });
        }
        self.next_sequence_id = self.next_sequence_id.saturating_add(1);
        self.live_work_items = self.live_work_items.saturating_add(1);
    }

    fn next_scheduled_work_is_apply_action(&mut self) -> bool {
        self.peek_scheduled_work_id()
            .and_then(|work_id| self.work.get(work_id))
            .and_then(Option::as_ref)
            .is_some_and(|work| matches!(work, GeneratorWork::ApplyAction(_)))
    }

    fn peek_scheduled_work_id(&mut self) -> Option<usize> {
        let lane_count = self.guided_frontiers.len().saturating_add(1);
        for offset in 0..lane_count {
            let lane = (self.next_scheduler_lane + offset) % lane_count;
            let work_id = if lane == 0 {
                self.peek_anchor_work_id()
            } else {
                self.peek_guided_work_id(lane - 1)
            };
            if work_id.is_some() {
                return work_id;
            }
        }
        None
    }

    fn pop_scheduled_work(&mut self) -> Option<GeneratorWork> {
        let lane_count = self.guided_frontiers.len().saturating_add(1);
        for offset in 0..lane_count {
            let lane = (self.next_scheduler_lane + offset) % lane_count;
            let work_id = if lane == 0 {
                self.pop_anchor_work_id()
            } else {
                self.pop_guided_work_id(lane - 1)
            };
            let Some(work_id) = work_id else {
                continue;
            };
            let work = self.work[work_id]
                .take()
                .expect("scheduled generator work must still be live");
            self.live_work_items = self.live_work_items.saturating_sub(1);
            self.next_scheduler_lane = (lane + 1) % lane_count;
            return Some(work);
        }
        None
    }

    fn peek_anchor_work_id(&mut self) -> Option<usize> {
        while let Some(entry) = self.anchor_frontier.peek() {
            if self.work.get(entry.work_id).is_some_and(Option::is_some) {
                return Some(entry.work_id);
            }
            self.anchor_frontier.pop();
        }
        None
    }

    fn pop_anchor_work_id(&mut self) -> Option<usize> {
        self.peek_anchor_work_id()?;
        self.anchor_frontier.pop().map(|entry| entry.work_id)
    }

    fn peek_guided_work_id(&mut self, guide_index: usize) -> Option<usize> {
        let frontier = self.guided_frontiers.get_mut(guide_index)?;
        while let Some(entry) = frontier.peek() {
            if self.work.get(entry.work_id).is_some_and(Option::is_some) {
                return Some(entry.work_id);
            }
            frontier.pop();
        }
        None
    }

    fn pop_guided_work_id(&mut self, guide_index: usize) -> Option<usize> {
        self.peek_guided_work_id(guide_index)?;
        self.guided_frontiers[guide_index]
            .pop()
            .map(|entry| entry.work_id)
    }
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

#[cfg(test)]
mod priority_tests {
    use super::*;

    fn guided_entry(
        guide: i32,
        cumulative_negative_log_policy: f64,
        atomic_depth: usize,
        sequence_id: u64,
    ) -> GuidedGeneratorQueueEntry {
        GuidedGeneratorQueueEntry {
            guide_index: 0,
            work_id: sequence_id as usize,
            sequence_id,
            guide_rank: CombatStateGuideRank::new(vec![guide]),
            anchor_priority: GeneratorWorkPriority::for_path(
                atomic_depth,
                cumulative_negative_log_policy,
            ),
        }
    }

    #[test]
    fn guided_prefix_priority_uses_exact_state_before_anchor_policy() {
        let improved_after_setup = guided_entry(10, 8.0, 3, 0);
        let locally_greedy = guided_entry(9, 0.01, 1, 1);

        assert!(improved_after_setup > locally_greedy);
    }
}
