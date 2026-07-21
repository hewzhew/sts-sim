use std::sync::{Arc, Mutex};

use std::time::{Duration, Instant};

use sts_combat_planner::{
    CombatActionPolicy, CombatGuideLaneId, CombatPolicyChoice, CombatPolicyWitnessProposal,
    CombatStateGuide, CombatStateGuideRank, DeferredCombatGuideRefinement, TurnOptionAction,
};

use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};

const ADVISOR_TOTAL_NODE_LIMIT: usize = 800_000;
const ADVISOR_TOTAL_WALL_LIMIT: Duration = Duration::from_millis(2_000);
const ADVISOR_POTION_BASELINE_NODE_LIMIT: usize = 80_000;
const ADVISOR_POTION_BASELINE_WALL_LIMIT: Duration = Duration::from_millis(1_000);
// V2's root rollout is currently one bounded, non-resumable computation. A
// shorter wall slice discards it and pays the same startup cost again. Give
// each newly created V2 phase one explicit coherent service slice; all later
// frontier work remains preemptible.
const ADVISOR_INITIAL_COHERENT_WALL: Duration = Duration::from_millis(1_000);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AdvisorPhaseV1 {
    Conserved,
    MixedPotion,
}

#[derive(Clone, Debug)]
pub enum ExistingCombatKnowledgeAdvisorAdvanceV1 {
    Pending,
    Proposal(CombatPolicyWitnessProposal),
    Exhausted,
}

/// Stateful, explicitly bounded access to the mature V2 tactical search.
///
/// This is deliberately not a `CombatActionPolicy`: it owns a resumable
/// frontier and therefore has a lifecycle, work accounting, and a terminal
/// outcome.  The returned line remains an untrusted proposal until the new
/// planner replays it exactly.
pub struct ExistingCombatKnowledgeAdvisorV1 {
    root: CombatPosition,
    search: crate::ai::combat_search_v2::CombatSearchV2Session,
    phase: AdvisorPhaseV1,
    has_potion: bool,
    phase_node_limit: usize,
    phase_wall_limit: Duration,
    phase_elapsed: Duration,
    total_nodes: u64,
    total_elapsed: Duration,
    max_engine_steps_per_action: usize,
    finished: bool,
}

impl ExistingCombatKnowledgeAdvisorV1 {
    pub fn new(root: &CombatPosition, max_engine_steps_per_action: usize) -> Self {
        let has_potion = root.combat.entities.potions.iter().any(Option::is_some);
        let (phase_node_limit, phase_wall_limit) = if has_potion {
            (
                ADVISOR_POTION_BASELINE_NODE_LIMIT,
                ADVISOR_POTION_BASELINE_WALL_LIMIT,
            )
        } else {
            (ADVISOR_TOTAL_NODE_LIMIT, ADVISOR_TOTAL_WALL_LIMIT)
        };
        let search = new_advisor_search(
            root,
            max_engine_steps_per_action,
            crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never,
            phase_node_limit,
            phase_wall_limit,
        );
        Self {
            root: root.clone(),
            search,
            phase: AdvisorPhaseV1::Conserved,
            has_potion,
            phase_node_limit,
            phase_wall_limit,
            phase_elapsed: Duration::ZERO,
            total_nodes: 0,
            total_elapsed: Duration::ZERO,
            max_engine_steps_per_action: max_engine_steps_per_action.max(1),
            finished: false,
        }
    }

    /// Restores charged allowance after a process-level search restart. The
    /// V2 frontier itself is intentionally not claimed to be serializable, but
    /// restarting can never mint fresh advisor budget.
    pub fn restore_charged_usage(&mut self, nodes: u64, elapsed: Duration) {
        self.total_nodes = nodes.min(ADVISOR_TOTAL_NODE_LIMIT as u64);
        self.total_elapsed = elapsed.min(ADVISOR_TOTAL_WALL_LIMIT);
        if self.total_nodes >= ADVISOR_TOTAL_NODE_LIMIT as u64
            || self.total_elapsed >= ADVISOR_TOTAL_WALL_LIMIT
        {
            self.finished = true;
        }
    }

    pub fn total_nodes(&self) -> u64 {
        self.total_nodes
    }

    pub fn total_elapsed(&self) -> Duration {
        self.total_elapsed
    }

    pub fn advance(
        &mut self,
        soft_wall_time: Option<Duration>,
        hard_wall_time: Option<Duration>,
    ) -> Result<ExistingCombatKnowledgeAdvisorAdvanceV1, String> {
        if self.finished {
            return Ok(ExistingCombatKnowledgeAdvisorAdvanceV1::Exhausted);
        }

        loop {
            let phase_nodes = self.search.nodes_expanded() as usize;
            let remaining_nodes = self
                .phase_node_limit
                .saturating_sub(phase_nodes)
                .min(ADVISOR_TOTAL_NODE_LIMIT.saturating_sub(self.total_nodes as usize));
            let remaining_wall = self
                .phase_wall_limit
                .saturating_sub(self.phase_elapsed)
                .min(ADVISOR_TOTAL_WALL_LIMIT.saturating_sub(self.total_elapsed));
            if remaining_nodes == 0 || remaining_wall.is_zero() {
                if self.start_mixed_potion_phase_if_available() {
                    continue;
                }
                self.finished = true;
                return Ok(ExistingCombatKnowledgeAdvisorAdvanceV1::Exhausted);
            }

            let requested_wall = soft_wall_time.unwrap_or(remaining_wall);
            let coherent_wall = if self.phase_elapsed.is_zero() {
                ADVISOR_INITIAL_COHERENT_WALL.min(remaining_wall)
            } else {
                Duration::ZERO
            };
            let wall = requested_wall
                .max(coherent_wall)
                .min(remaining_wall)
                .min(hard_wall_time.unwrap_or(remaining_wall));
            if wall.is_zero() {
                return Ok(ExistingCombatKnowledgeAdvisorAdvanceV1::Pending);
            }
            let before_nodes = self.search.nodes_expanded();
            let started = Instant::now();
            let stop =
                self.search
                    .advance(crate::ai::combat_search_v2::CombatSearchV2WorkQuantum {
                        additional_nodes: remaining_nodes,
                        soft_wall_time: Some(wall),
                    });
            let elapsed = started.elapsed();
            let expanded = self.search.nodes_expanded().saturating_sub(before_nodes);
            self.phase_elapsed = self.phase_elapsed.saturating_add(elapsed);
            self.total_elapsed = self.total_elapsed.saturating_add(elapsed);
            self.total_nodes = self.total_nodes.saturating_add(expanded);

            let phase_done = matches!(
                stop,
                crate::ai::combat_search_v2::CombatSearchV2AdvanceStop::CandidateSatisfied
                    | crate::ai::combat_search_v2::CombatSearchV2AdvanceStop::FrontierExhausted
                    | crate::ai::combat_search_v2::CombatSearchV2AdvanceStop::AlreadyComplete
            ) || self.search.nodes_expanded() as usize >= self.phase_node_limit
                || self.phase_elapsed >= self.phase_wall_limit;
            if phase_done {
                if let Some(win) = self.search.snapshot().best_win {
                    let proposal = materialize_advisor_proposal(&self.root, win)?;
                    self.finished = true;
                    return Ok(ExistingCombatKnowledgeAdvisorAdvanceV1::Proposal(proposal));
                }
                if self.start_mixed_potion_phase_if_available() {
                    continue;
                }
                self.finished = true;
                return Ok(ExistingCombatKnowledgeAdvisorAdvanceV1::Exhausted);
            }
            return Ok(ExistingCombatKnowledgeAdvisorAdvanceV1::Pending);
        }
    }

    fn start_mixed_potion_phase_if_available(&mut self) -> bool {
        if !self.has_potion || self.phase != AdvisorPhaseV1::Conserved {
            return false;
        }
        self.phase = AdvisorPhaseV1::MixedPotion;
        self.phase_node_limit =
            ADVISOR_TOTAL_NODE_LIMIT.saturating_sub(ADVISOR_POTION_BASELINE_NODE_LIMIT);
        self.phase_wall_limit =
            ADVISOR_TOTAL_WALL_LIMIT.saturating_sub(ADVISOR_POTION_BASELINE_WALL_LIMIT);
        self.phase_elapsed = Duration::ZERO;
        self.search = new_advisor_search(
            &self.root,
            self.max_engine_steps_per_action,
            crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::All,
            self.phase_node_limit,
            self.phase_wall_limit,
        );
        true
    }
}

fn new_advisor_search(
    root: &CombatPosition,
    max_engine_steps_per_action: usize,
    potion_policy: crate::ai::combat_search_v2::CombatSearchV2PotionPolicy,
    max_nodes: usize,
    wall_time: Duration,
) -> crate::ai::combat_search_v2::CombatSearchV2Session {
    let mut config = crate::ai::combat_search_v2::CombatSearchV2Config::default();
    config.max_nodes = max_nodes;
    config.max_engine_steps_per_action = max_engine_steps_per_action.max(1);
    config.wall_time = Some(wall_time);
    // Publish a useful incumbent, not whichever survivable line happens to
    // cross a wall-slice boundary first. Split quanta may expose different
    // first wins, while zero-loss-or-budget preserves the advisor's bounded
    // quality-improvement role.
    config.satisfaction = crate::ai::combat_search_v2::CombatSearchV2Satisfaction::ZeroLossOrBudget;
    config.potion_policy = potion_policy;
    config.max_potions_used = match potion_policy {
        crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never => Some(0),
        crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::All
        | crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::SemanticBudgeted => Some(3),
    };
    crate::ai::combat_search_v2::CombatSearchV2Session::new(&root.engine, &root.combat, config)
}

fn materialize_advisor_proposal(
    root: &CombatPosition,
    trajectory: crate::ai::combat_search_v2::CombatSearchV2TrajectoryReport,
) -> Result<CombatPolicyWitnessProposal, String> {
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut actions = Vec::with_capacity(trajectory.actions.len());
    for (index, action) in trajectory.actions.into_iter().enumerate() {
        if stepper
            .choice_for_legal_input(&position, &action.input)
            .is_none()
        {
            return Err(format!(
                "V2 advisor action {index} is not legal at its exact replay state"
            ));
        }
        let step = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: 512,
                deadline: None,
            },
        );
        if step.truncated || step.timed_out {
            return Err(format!(
                "V2 advisor action {index} could not reach a stable exact successor"
            ));
        }
        actions.push(TurnOptionAction {
            input: action.input,
            expected_successor_hash: crate::ai::combat_state_key::combat_exact_state_hash_v1(
                &step.position.engine,
                &step.position.combat,
            ),
            engine_steps: step.engine_steps,
        });
        position = step.position;
    }
    if stepper.terminal(&position) != CombatTerminal::Win || position.combat.runtime.combat_smoked {
        return Err("V2 advisor proposal did not replay to a true combat victory".to_string());
    }
    Ok(CombatPolicyWitnessProposal {
        actions,
        final_hp_hint: position.combat.entities.player.current_hp,
    })
}

pub(super) struct ExistingCombatKnowledgePolicy {
    rollout_guide: Option<Mutex<crate::ai::combat_search_v2::OracleRolloutGuideV1>>,
}

impl Default for ExistingCombatKnowledgePolicy {
    fn default() -> Self {
        Self {
            rollout_guide: None,
        }
    }
}

const GUIDE_PROGRESS: CombatGuideLaneId = CombatGuideLaneId::new(1);
const GUIDE_SURVIVAL: CombatGuideLaneId = CombatGuideLaneId::new(2);
const GUIDE_HORIZON: CombatGuideLaneId = CombatGuideLaneId::new(3);
const GUIDE_SETUP: CombatGuideLaneId = CombatGuideLaneId::new(4);
const GUIDE_TURN_DEPTH: CombatGuideLaneId = CombatGuideLaneId::new(5);
const GUIDE_ROLLOUT: CombatGuideLaneId = CombatGuideLaneId::new(6);

pub fn existing_combat_knowledge_policy_v1() -> sts_combat_planner::SharedCombatActionPolicy {
    Arc::new(ExistingCombatKnowledgePolicy::default())
}

pub fn existing_combat_knowledge_policy_with_rollout_guide_v1(
) -> sts_combat_planner::SharedCombatActionPolicy {
    Arc::new(ExistingCombatKnowledgePolicy {
        rollout_guide: Some(Mutex::new(
            crate::ai::combat_search_v2::OracleRolloutGuideV1::new(
                384,
                80,
                Duration::from_millis(10),
            ),
        )),
    })
}

impl ExistingCombatKnowledgePolicy {
    fn rollout_guide(&self, position: &CombatPosition) -> Option<CombatStateGuide> {
        let guide = self
            .rollout_guide
            .as_ref()?
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guide.cached_components(position) {
            Some(components) => Some(CombatStateGuide::new(GUIDE_ROLLOUT, components)),
            None => Some(CombatStateGuide::deferred(
                GUIDE_ROLLOUT,
                crate::ai::combat_search_v2::OracleRolloutGuideV1::initial_components(),
            )),
        }
    }
}

impl CombatActionPolicy for ExistingCombatKnowledgePolicy {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        let atomic_inputs = choices
            .iter()
            .filter_map(|choice| match choice {
                CombatPolicyChoice::Atomic(input) => Some((*input).clone()),
                CombatPolicyChoice::StructuredSelection(_) => None,
            })
            .collect::<Vec<_>>();
        let mut atomic_weights =
            crate::ai::combat_search_v2::oracle_action_policy::oracle_atomic_action_policy_weights(
                position,
                &atomic_inputs,
            )
            .into_iter();
        choices
            .iter()
            .map(|choice| match choice {
                CombatPolicyChoice::Atomic(_) => atomic_weights.next().unwrap_or(1.0),
                CombatPolicyChoice::StructuredSelection(_) => 1.0,
            })
            .collect()
    }

    fn state_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        let mut guides = vec![
            CombatStateGuide::new(
                GUIDE_PROGRESS,
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(position),
            ),
            CombatStateGuide::new(
                GUIDE_SURVIVAL,
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(position),
            ),
            CombatStateGuide::new(
                GUIDE_HORIZON,
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(position),
            ),
            CombatStateGuide::new(
                GUIDE_SETUP,
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(position),
            ),
        ];
        guides.extend(self.rollout_guide(position));
        guides
    }

    fn turn_generation_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        let mut guides = vec![
            CombatStateGuide::new(
                GUIDE_PROGRESS,
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(position),
            ),
            CombatStateGuide::new(
                GUIDE_SURVIVAL,
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(position),
            ),
            CombatStateGuide::new(
                GUIDE_TURN_DEPTH,
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_turn_generation_guide_components(position),
            ),
            CombatStateGuide::new(
                GUIDE_SETUP,
                crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(position),
            ),
        ];
        guides.extend(self.rollout_guide(position));
        guides
    }

    fn refine_deferred_guide(
        &self,
        lane: CombatGuideLaneId,
        position: &CombatPosition,
        deadline: Option<Instant>,
    ) -> DeferredCombatGuideRefinement {
        if lane != GUIDE_ROLLOUT {
            return DeferredCombatGuideRefinement::Unsupported;
        }
        let Some(rollout_guide) = self.rollout_guide.as_ref() else {
            return DeferredCombatGuideRefinement::Unsupported;
        };
        let mut guide = rollout_guide
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guide.evaluate(position, deadline) {
            crate::ai::combat_search_v2::OracleRolloutGuideEvaluationV1::Ready(components) => {
                DeferredCombatGuideRefinement::Ready(CombatStateGuideRank::new(components))
            }
            crate::ai::combat_search_v2::OracleRolloutGuideEvaluationV1::RetryLater => {
                DeferredCombatGuideRefinement::RetryLater
            }
            crate::ai::combat_search_v2::OracleRolloutGuideEvaluationV1::BudgetExhausted => {
                DeferredCombatGuideRefinement::Unsupported
            }
        }
    }
}
