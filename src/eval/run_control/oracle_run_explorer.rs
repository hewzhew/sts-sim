use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::ai::deck_mutation_compiler_v1::{
    compile_deck_mutation_decision_v1, DeckMutationCommitmentModeV1, DeckMutationCompilerOutputV1,
    DeckMutationCompilerRequestV1,
};
use crate::eval::combat_guidance_bundle::CombatGuidanceBundleV1;
use crate::state::core::{ClientInput, EngineState, RunResult};
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};

use super::oracle_combat_work::{OracleRunCombatWorkCheckpointV1, OracleRunCombatWorkV1};
use super::oracle_selection_cursor::LazyUnorderedSelectionCursorV1;
use super::{
    build_decision_surface, positive_ranked_run_policy_prior_v1, DecisionCandidateKey,
    NeowOracleExpansionV1, RunControlCombatSearchQuantum, RunControlCombatSearchRejection,
    RunControlCombatWorkAdvanceV1, RunControlHpLossLimit, RunControlSearchCombatOptions,
    RunControlSession, RunControlSessionCheckpointV1, RunControlTraceAnnotationV1,
    RunDecisionAction, RunPolicyCandidateV1, RunPolicyPriorFnV1, RunProgressJournalV1,
    RunProgressStepV1, StrategicProbeShadowOrderKeyV1,
};

mod checkpoint;

pub use checkpoint::{
    OracleRunActiveCombatCheckpointV1, OracleRunBranchCheckpointV1,
    OracleRunDeferredCombatCheckpointV1, OracleRunExplorerCheckpointV1,
    OracleRunJournalNodeCheckpointV1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleRunWorkKindV1 {
    MapTravel,
    RewardAction,
    EventOption,
    ShopAction,
    CampfireAction,
    RunChoice,
    TreasureAction,
    BossRelicChoice,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleRunBoundaryV1 {
    MapDecision,
    Combat,
    Reward,
    Event,
    Shop,
    Campfire,
    RunChoice,
    Treasure,
    BossRelic,
    TerminalVictory,
    TerminalDefeat,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OracleRunReplayStepV1 {
    pub candidate_id: String,
    pub label: String,
    pub action: RunDecisionAction,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LazyOracleRunDecisionV1 {
    pub parent_branch_id: usize,
    pub parent_state_fingerprint: String,
    pub neow_root_candidate_id: String,
    pub kind: OracleRunWorkKindV1,
    pub candidate_id: String,
    pub label: String,
    pub action: RunDecisionAction,
    pub stable_work_key: String,
    pub path_negative_log_policy: f64,
    pub path_discrepancy: u64,
    pub path_depth: u64,
    #[serde(default)]
    pub parent_act: u8,
    #[serde(default)]
    pub parent_floor: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat_edge_probe: Option<OracleRunCombatEdgeProbeV1>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LazyOracleRunSelectionFamilyV1 {
    pub family_key: String,
    pub parent_branch_id: usize,
    pub parent_state_fingerprint: String,
    pub neow_root_candidate_id: String,
    pub kind: OracleRunWorkKindV1,
    pub candidate_id: String,
    pub label: String,
    pub path_negative_log_policy: f64,
    pub path_discrepancy: u64,
    pub path_depth: u64,
    pub parent_act: u8,
    pub parent_floor: i32,
    pub public_probability: f64,
    cursor: LazyUnorderedSelectionCursorV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outstanding_work_key: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OracleRunCombatEdgeProbeV1 {
    NotImmediateCombat,
    HeuristicEstimate {
        order_key: StrategicProbeShadowOrderKeyV1,
    },
}

#[derive(Clone, Debug)]
pub struct OracleRunBranchV1 {
    pub branch_id: usize,
    pub parent_branch_id: Option<usize>,
    pub neow_root_candidate_id: String,
    pub neow_root_label: String,
    pub state_fingerprint: String,
    pub boundary: OracleRunBoundaryV1,
    pub path_negative_log_policy: f64,
    pub path_discrepancy: u64,
    pub path_depth: u64,
    pub replay: Vec<OracleRunReplayStepV1>,
    pub journal: RunProgressJournalV1,
    pub session: RunControlSession,
}

#[derive(Clone, Debug)]
pub struct ExactDuplicateOracleRunBranchV1 {
    pub branch_id: usize,
    pub parent_branch_id: Option<usize>,
    pub survivor_branch_id: usize,
    pub neow_root_candidate_id: String,
    pub state_fingerprint: String,
    pub replay: Vec<OracleRunReplayStepV1>,
    pub journal: RunProgressJournalV1,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunUnresolvedCombatV1 {
    pub branch_id: usize,
    pub rejection: RunControlCombatSearchRejection,
    pub evidence_kind: String,
    pub last_status: Option<String>,
    pub nodes_expanded: u64,
    pub exact_states: usize,
    pub applied_action_transitions: usize,
    pub unique_successor_states: usize,
    pub duplicate_exact_successors: usize,
    pub completed_turn_options: usize,
    pub retained_state_work: usize,
    pub max_player_turn: u32,
    pub max_path_atomic_depth: usize,
    pub generation_gap_count: usize,
    pub incumbent_final_hp: Option<i32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OraclePendingCombatEnemyV1 {
    pub monster_type: usize,
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleCombatSearchResumeKindV1 {
    Fresh,
    /// Reserved for a future checkpoint that serializes the tactical frontier.
    SearchResumeExact,
    StateReplayExactSearchRestarted,
}

#[derive(Clone, Debug, Serialize)]
pub struct OraclePendingCombatSummaryV1 {
    pub branch_id: usize,
    pub act: u8,
    pub floor: i32,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub elite: bool,
    pub boss: bool,
    pub enemies: Vec<OraclePendingCombatEnemyV1>,
    pub nodes_expanded: u64,
    pub engine_steps: usize,
    pub exact_states: usize,
    pub applied_action_transitions: usize,
    pub unique_successor_states: usize,
    pub duplicate_exact_successors: usize,
    pub completed_turn_options: usize,
    pub retained_state_work: usize,
    pub queued_anchor_entries: usize,
    pub queued_guided_entries: Vec<usize>,
    pub max_player_turn: u32,
    pub max_path_atomic_depth: usize,
    pub max_completed_turn_options_at_state: usize,
    pub generation_gap_count: usize,
    pub pending_witness_replay: bool,
    pub quantum_count: usize,
    pub last_quantum_generation_work: usize,
    pub last_quantum_engine_steps: usize,
    pub incumbent_final_hp: Option<i32>,
    pub incumbent_hp_loss: Option<i32>,
    pub incumbent_action_count: Option<usize>,
    pub incumbent_revision: u64,
    pub quanta_since_incumbent_improvement: usize,
    pub last_status: Option<&'static str>,
    pub remaining_nodes: usize,
    pub remaining_wall_ms: Option<u64>,
    pub resume_kind: OracleCombatSearchResumeKindV1,
    pub restart_count: usize,
}

#[derive(Clone, Debug)]
pub struct OracleRunCombatBudgetsV1 {
    pub hallway: RunControlSearchCombatOptions,
    pub elite: RunControlSearchCombatOptions,
    pub boss: RunControlSearchCombatOptions,
    /// Determines whether each configured search satisfaction is used
    /// literally or whether non-boss combat derives the shared strategic
    /// quality target from the exact run state.
    pub quality_policy: OracleRunCombatQualityPolicyV1,
    /// A value greater than one enables a two-fidelity schedule. The first
    /// exact attempt receives `1 / initial_divisor` of the configured
    /// allowance. A budget-unknown result remains a live exact edge and may
    /// later earn one full-budget restart.
    pub initial_divisor: u32,
    /// Optional immutable learned guidance. Exact simulation, legality,
    /// terminal checks, and replay remain authoritative.
    pub guidance_bundle: Option<Arc<CombatGuidanceBundleV1>>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OracleRunCombatQualityPolicyV1 {
    /// Preserve the satisfaction carried by each configured search option.
    #[default]
    Configured,
    /// Stop refinement once an exact witness satisfies the run's shared
    /// survival-and-quality reserve. A boss that reaches a full act heal or
    /// the requested run end keeps first-witness semantics after one complete
    /// independent local-search challenge.
    StrategicRun,
}

impl OracleRunCombatBudgetsV1 {
    pub fn uniform(options: RunControlSearchCombatOptions) -> Self {
        Self {
            hallway: options.clone(),
            elite: options.clone(),
            boss: options,
            quality_policy: OracleRunCombatQualityPolicyV1::Configured,
            initial_divisor: 1,
            guidance_bundle: None,
        }
    }

    pub fn with_guidance_bundle(mut self, bundle: Option<CombatGuidanceBundleV1>) -> Self {
        self.guidance_bundle = bundle.map(Arc::new);
        self
    }

    pub(super) fn for_session(&self, session: &RunControlSession) -> RunControlSearchCombatOptions {
        self.for_session_stage(session, 1)
    }

    fn for_session_stage(
        &self,
        session: &RunControlSession,
        stage: u8,
    ) -> RunControlSearchCombatOptions {
        let Some(active) = session.active_combat.as_ref() else {
            return scale_combat_options(self.hallway.clone(), self.stage_divisor(stage));
        };
        let mut options = if active.combat_state.meta.is_boss_fight {
            self.boss.clone()
        } else if active.combat_state.meta.is_elite_fight {
            self.elite.clone()
        } else {
            self.hallway.clone()
        };
        if self.quality_policy == OracleRunCombatQualityPolicyV1::StrategicRun {
            options.satisfaction = Some(
                if super::strategic_combat_persistent_payoff_matters_v1(session) {
                    crate::ai::combat_search_v2::CombatSearchV2Satisfaction::PersistentRunValueGain
                } else {
                    match super::strategic_combat_quality_hp_loss_limit_v1(session) {
                    RunControlHpLossLimit::Limit(limit) => {
                        crate::ai::combat_search_v2::CombatSearchV2Satisfaction::HpLossAtMost(limit)
                    }
                    RunControlHpLossLimit::Unlimited => {
                        crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin
                    }
                    }
                },
            );
        }
        if stage == 0 && self.uses_potion_conserving_primary(session, &options) {
            options.potion_policy =
                Some(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never);
            options.max_potions_used = Some(0);
        }
        scale_combat_options(options, self.stage_divisor(stage))
    }

    fn stage_divisor(&self, stage: u8) -> u32 {
        if stage == 0 {
            self.initial_divisor.max(1)
        } else {
            1
        }
    }

    fn has_later_stage(&self, session: &RunControlSession, stage: u8) -> bool {
        stage == 0
            && (self.initial_divisor > 1
                || self
                    .uses_potion_conserving_primary(session, &self.for_session_stage(session, 1)))
    }

    fn uses_potion_conserving_primary(
        &self,
        session: &RunControlSession,
        options: &RunControlSearchCombatOptions,
    ) -> bool {
        if self.quality_policy != OracleRunCombatQualityPolicyV1::StrategicRun
            || options.max_potions_used.is_some()
            || options.potion_policy.is_some()
        {
            return false;
        }
        session.active_combat.as_ref().is_some_and(|active| {
            !active.combat_state.meta.is_boss_fight
                && active
                    .combat_state
                    .entities
                    .potions
                    .iter()
                    .flatten()
                    .any(|potion| potion.can_use)
        })
    }
}

fn scale_combat_options(
    mut options: RunControlSearchCombatOptions,
    divisor: u32,
) -> RunControlSearchCombatOptions {
    let divisor = usize::try_from(divisor.max(1)).unwrap_or(usize::MAX);
    options.max_nodes = options
        .max_nodes
        .map(|value| value.saturating_add(divisor - 1) / divisor)
        .map(|value| value.max(1));
    options.wall_ms = options
        .wall_ms
        .map(|value| value.saturating_add(divisor as u64 - 1) / divisor as u64)
        .map(|value| value.max(1));
    options
}

#[derive(Clone, Debug)]
pub struct OracleRunExploreBudgetV1 {
    pub max_work_items: usize,
    pub wall_ms: Option<u64>,
    pub combat: OracleRunCombatBudgetsV1,
    pub combat_quantum_nodes: usize,
    pub combat_quantum_ms: Option<u64>,
    pub decision_prior: Option<RunPolicyPriorFnV1>,
    pub decision_annotation: Option<OracleRunDecisionAnnotationFnV1>,
    pub combat_edge_order: Option<OracleRunCombatEdgeOrderFnV1>,
}

pub type OracleRunDecisionAnnotationFnV1 =
    fn(&RunControlSession, &str) -> Option<RunControlTraceAnnotationV1>;
pub type OracleRunCombatEdgeOrderFnV1 =
    fn(&RunControlSession, &str, &RunDecisionAction) -> Option<StrategicProbeShadowOrderKeyV1>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OracleRunExploreStopV1 {
    Victory { branch_id: usize },
    WorkExhausted,
    WorkBudgetExhausted,
    WallDeadlineReached,
}

struct PendingOracleCombatV1 {
    branch_id: usize,
    stage: u8,
    work: OracleRunCombatWorkV1,
}

struct DeferredOracleCombatV1 {
    branch_id: usize,
    stage: u8,
    prior_work: OracleRunCombatWorkCheckpointV1,
}

enum FinishedOracleCombatV1 {
    Resolved(usize),
    ExactDuplicate,
    Unresolved(OracleRunUnresolvedCombatV1),
}

enum ScheduledOracleRunWorkV1 {
    Decision(LazyOracleRunDecisionV1),
    DeferredCombat(DeferredOracleCombatV1),
}

pub struct OracleRunExplorerV1 {
    pub branches: Vec<OracleRunBranchV1>,
    pub pending_decisions: VecDeque<LazyOracleRunDecisionV1>,
    pub retired_exact_duplicates: Vec<ExactDuplicateOracleRunBranchV1>,
    pub unresolved_combats: Vec<OracleRunUnresolvedCombatV1>,
    pub combat_search_restarts: usize,
    pending_combats: VecDeque<PendingOracleCombatV1>,
    deferred_combats: VecDeque<DeferredOracleCombatV1>,
    pending_selection_families: VecDeque<LazyOracleRunSelectionFamilyV1>,
    last_served_neow_root: Option<String>,
    next_branch_id: usize,
    state_index: BTreeMap<String, usize>,
    registered_work_keys: BTreeSet<String>,
}

pub struct OracleRunExploreResultV1 {
    pub explorer: OracleRunExplorerV1,
    pub stop: OracleRunExploreStopV1,
    pub work_items: usize,
    pub combat_quanta: usize,
    pub decision_service_ms: u64,
    pub combat_service_ms: u64,
    pub combat_edge_probe_evaluations: usize,
    pub immediate_combat_edge_hints: usize,
    pub elapsed_ms: u64,
}

impl OracleRunExploreResultV1 {
    pub fn witness(&self) -> Option<&OracleRunBranchV1> {
        let OracleRunExploreStopV1::Victory { branch_id } = self.stop else {
            return None;
        };
        self.explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
    }

    pub fn furthest_branch(&self) -> Option<&OracleRunBranchV1> {
        self.explorer.branches.iter().max_by_key(|branch| {
            (
                branch.session.run_state.act_num,
                branch.session.run_state.floor_num,
                branch.journal.len(),
                branch.branch_id,
            )
        })
    }
}

impl OracleRunExplorerV1 {
    fn empty() -> Self {
        Self {
            branches: Vec::new(),
            pending_decisions: VecDeque::new(),
            retired_exact_duplicates: Vec::new(),
            unresolved_combats: Vec::new(),
            combat_search_restarts: 0,
            pending_combats: VecDeque::new(),
            deferred_combats: VecDeque::new(),
            pending_selection_families: VecDeque::new(),
            last_served_neow_root: None,
            next_branch_id: 0,
            state_index: BTreeMap::new(),
            registered_work_keys: BTreeSet::new(),
        }
    }

    pub fn pending_combat_count(&self) -> usize {
        self.pending_combats.len()
    }

    pub fn deferred_combat_count(&self) -> usize {
        self.deferred_combats.len()
    }

    pub fn deferred_combat_branch_ids(&self) -> Vec<usize> {
        self.deferred_combats
            .iter()
            .map(|combat| combat.branch_id)
            .collect()
    }

    pub fn pending_decision_discrepancy_counts(&self) -> BTreeMap<u64, usize> {
        let mut counts = BTreeMap::new();
        for decision in &self.pending_decisions {
            *counts.entry(decision.path_discrepancy).or_insert(0) += 1;
        }
        counts
    }

    pub fn deferred_combat_effective_discrepancy_counts(&self) -> BTreeMap<u64, usize> {
        let mut counts = BTreeMap::new();
        for deferred in &self.deferred_combats {
            let Some(branch) = self
                .branches
                .iter()
                .find(|branch| branch.branch_id == deferred.branch_id)
            else {
                continue;
            };
            *counts.entry(branch.path_discrepancy).or_insert(0) += 1;
        }
        counts
    }

    pub fn pending_combat_summaries(&self) -> Result<Vec<OraclePendingCombatSummaryV1>, String> {
        self.pending_combats
            .iter()
            .map(|pending| {
                let branch = self
                    .branches
                    .iter()
                    .find(|branch| branch.branch_id == pending.branch_id)
                    .ok_or_else(|| {
                        format!(
                            "pending combat references missing branch {}",
                            pending.branch_id
                        )
                    })?;
                let active = branch.session.active_combat.as_ref().ok_or_else(|| {
                    format!(
                        "pending combat branch {} has no active combat",
                        pending.branch_id
                    )
                })?;
                let enemies = active
                    .combat_state
                    .entities
                    .monsters
                    .iter()
                    .filter(|monster| !monster.is_dying && !monster.is_escaped)
                    .map(|monster| OraclePendingCombatEnemyV1 {
                        monster_type: monster.monster_type,
                        name: super::view_model::monster_name(monster.monster_type),
                        current_hp: monster.current_hp,
                        max_hp: monster.max_hp,
                    })
                    .collect();
                let progress = pending.work.progress();
                Ok(OraclePendingCombatSummaryV1 {
                    branch_id: branch.branch_id,
                    act: branch.session.run_state.act_num,
                    floor: branch.session.run_state.floor_num,
                    player_hp: branch.session.run_state.current_hp,
                    player_max_hp: branch.session.run_state.max_hp,
                    elite: active.combat_state.meta.is_elite_fight,
                    boss: active.combat_state.meta.is_boss_fight,
                    enemies,
                    nodes_expanded: progress.generation_work,
                    engine_steps: progress.engine_steps,
                    exact_states: progress.exact_states,
                    applied_action_transitions: progress.applied_action_transitions,
                    unique_successor_states: progress.unique_successor_states,
                    duplicate_exact_successors: progress.duplicate_exact_successors,
                    completed_turn_options: progress.completed_turn_options,
                    retained_state_work: progress.retained_state_work,
                    queued_anchor_entries: progress.queued_anchor_entries,
                    queued_guided_entries: progress.queued_guided_entries,
                    max_player_turn: progress.max_player_turn,
                    max_path_atomic_depth: progress.max_path_atomic_depth,
                    max_completed_turn_options_at_state: progress
                        .max_completed_turn_options_at_state,
                    generation_gap_count: progress.generation_gap_count,
                    pending_witness_replay: progress.pending_witness_replay,
                    quantum_count: pending.work.quantum_count(),
                    last_quantum_generation_work: progress.last_quantum_generation_work,
                    last_quantum_engine_steps: progress.last_quantum_engine_steps,
                    incumbent_final_hp: progress.incumbent_final_hp,
                    incumbent_hp_loss: progress.incumbent_hp_loss,
                    incumbent_action_count: progress.incumbent_action_count,
                    incumbent_revision: progress.incumbent_revision,
                    quanta_since_incumbent_improvement: progress.quanta_since_incumbent_improvement,
                    last_status: progress.last_status,
                    remaining_nodes: pending.work.remaining_nodes(),
                    remaining_wall_ms: pending.work.remaining_wall_ms(),
                    resume_kind: if pending.work.restart_count() == 0 {
                        OracleCombatSearchResumeKindV1::Fresh
                    } else {
                        OracleCombatSearchResumeKindV1::StateReplayExactSearchRestarted
                    },
                    restart_count: pending.work.restart_count(),
                })
            })
            .collect()
    }

    #[cfg(test)]
    fn take_best_decision(&mut self) -> Option<LazyOracleRunDecisionV1> {
        let index = self.best_decision_index()?;
        self.pending_decisions.remove(index)
    }

    #[cfg(test)]
    fn best_decision_index(&self) -> Option<usize> {
        self.pending_decisions
            .iter()
            .enumerate()
            .min_by(|(left_index, left), (right_index, right)| {
                oracle_run_decision_priority_order(*left_index, left, *right_index, right)
            })
            .map(|(index, _)| index)
    }

    fn next_neow_root_for_service(&self) -> Option<String> {
        let mut roots = BTreeSet::new();
        roots.extend(
            self.pending_decisions
                .iter()
                .map(|decision| decision.neow_root_candidate_id.clone()),
        );
        for deferred in &self.deferred_combats {
            let branch = self
                .branches
                .iter()
                .find(|branch| branch.branch_id == deferred.branch_id)
                .expect("deferred combat branch must remain live");
            roots.insert(branch.neow_root_candidate_id.clone());
        }
        let after_cursor = self.last_served_neow_root.as_ref().and_then(|last| {
            roots
                .iter()
                .find(|candidate| candidate.as_str() > last.as_str())
                .cloned()
        });
        after_cursor.or_else(|| roots.first().cloned())
    }

    fn best_decision_index_for_root(&self, root: &str) -> Option<usize> {
        self.pending_decisions
            .iter()
            .enumerate()
            .filter(|(_, decision)| decision.neow_root_candidate_id == root)
            .min_by(|(left_index, left), (right_index, right)| {
                oracle_run_decision_priority_order(*left_index, left, *right_index, right)
            })
            .map(|(index, _)| index)
    }

    fn best_deferred_combat_index_for_root(&self, root: &str) -> Option<usize> {
        self.deferred_combats
            .iter()
            .enumerate()
            .filter(|(_, deferred)| {
                self.branches
                    .iter()
                    .find(|branch| branch.branch_id == deferred.branch_id)
                    .is_some_and(|branch| branch.neow_root_candidate_id == root)
            })
            .min_by(|(left_index, left), (right_index, right)| {
                let left_branch = self
                    .branches
                    .iter()
                    .find(|branch| branch.branch_id == left.branch_id)
                    .expect("deferred combat branch must remain live");
                let right_branch = self
                    .branches
                    .iter()
                    .find(|branch| branch.branch_id == right.branch_id)
                    .expect("deferred combat branch must remain live");
                left_branch
                    .path_discrepancy
                    .cmp(&right_branch.path_discrepancy)
                    .then_with(|| {
                        right_branch
                            .session
                            .run_state
                            .act_num
                            .cmp(&left_branch.session.run_state.act_num)
                    })
                    .then_with(|| {
                        right_branch
                            .session
                            .run_state
                            .floor_num
                            .cmp(&left_branch.session.run_state.floor_num)
                    })
                    .then_with(|| right_branch.path_depth.cmp(&left_branch.path_depth))
                    .then_with(|| {
                        left_branch
                            .path_negative_log_policy
                            .total_cmp(&right_branch.path_negative_log_policy)
                    })
                    .then_with(|| left_index.cmp(right_index))
            })
            .map(|(index, _)| index)
    }

    fn take_next_scheduled_work(&mut self) -> Option<ScheduledOracleRunWorkV1> {
        let root = self.next_neow_root_for_service()?;
        let decision_index = self.best_decision_index_for_root(&root);
        let deferred_index = self.best_deferred_combat_index_for_root(&root);
        let take_deferred = match (decision_index, deferred_index) {
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (Some(decision_index), Some(deferred_index)) => {
                let decision = &self.pending_decisions[decision_index];
                let deferred = &self.deferred_combats[deferred_index];
                let branch = self
                    .branches
                    .iter()
                    .find(|branch| branch.branch_id == deferred.branch_id)
                    .expect("deferred combat branch must remain live");
                match branch.path_discrepancy.cmp(&decision.path_discrepancy) {
                    std::cmp::Ordering::Less => true,
                    std::cmp::Ordering::Greater => false,
                    std::cmp::Ordering::Equal => {
                        (
                            branch.session.run_state.act_num,
                            branch.session.run_state.floor_num,
                            branch.path_depth,
                        )
                            .cmp(&(
                                decision.parent_act,
                                decision.parent_floor,
                                decision.path_depth,
                            ))
                            .then_with(|| {
                                decision
                                    .path_negative_log_policy
                                    .total_cmp(&branch.path_negative_log_policy)
                            })
                            == std::cmp::Ordering::Greater
                    }
                }
            }
            (None, None) => return None,
        };
        self.last_served_neow_root = Some(root);
        if take_deferred {
            self.deferred_combats
                .remove(deferred_index.expect("deferred index selected"))
                .map(ScheduledOracleRunWorkV1::DeferredCombat)
        } else {
            self.pending_decisions
                .remove(decision_index.expect("decision index selected"))
                .map(ScheduledOracleRunWorkV1::Decision)
        }
    }

    fn refresh_combat_edge_probes(
        &mut self,
        edge_order: Option<OracleRunCombatEdgeOrderFnV1>,
    ) -> Result<(usize, usize), String> {
        let Some(edge_order) = edge_order else {
            return Ok((0, 0));
        };
        let mut evaluations = 0usize;
        let mut immediate = 0usize;
        for index in 0..self.pending_decisions.len() {
            if self.pending_decisions[index].combat_edge_probe.is_some() {
                continue;
            }
            let work = &self.pending_decisions[index];
            let branch = self
                .branches
                .iter()
                .find(|branch| branch.branch_id == work.parent_branch_id)
                .ok_or_else(|| {
                    format!(
                        "oracle decision edge probe references missing parent branch {}",
                        work.parent_branch_id
                    )
                })?;
            let order_key = edge_order(&branch.session, &work.candidate_id, &work.action);
            evaluations = evaluations.saturating_add(1);
            let probe = if let Some(order_key) = order_key {
                immediate = immediate.saturating_add(1);
                OracleRunCombatEdgeProbeV1::HeuristicEstimate { order_key }
            } else {
                OracleRunCombatEdgeProbeV1::NotImmediateCombat
            };
            self.pending_decisions[index].combat_edge_probe = Some(probe);
        }
        Ok((evaluations, immediate))
    }

    pub(super) fn accept_branch(&mut self, branch: OracleRunBranchV1) -> Option<usize> {
        if let Some(survivor_branch_id) = self.state_index.get(&branch.state_fingerprint).copied() {
            self.retired_exact_duplicates
                .push(ExactDuplicateOracleRunBranchV1 {
                    branch_id: branch.branch_id,
                    parent_branch_id: branch.parent_branch_id,
                    survivor_branch_id,
                    neow_root_candidate_id: branch.neow_root_candidate_id,
                    state_fingerprint: branch.state_fingerprint,
                    replay: branch.replay,
                    journal: branch.journal,
                });
            return None;
        }
        let branch_id = branch.branch_id;
        self.state_index
            .insert(branch.state_fingerprint.clone(), branch_id);
        self.branches.push(branch);
        Some(branch_id)
    }

    fn register_decision_work(
        &mut self,
        branch_id: usize,
        decision_prior: Option<RunPolicyPriorFnV1>,
    ) -> Result<(), String> {
        let branch = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
            .ok_or_else(|| format!("missing oracle run branch {branch_id}"))?;
        let mut supply = decision_supply_for_branch(branch, decision_prior)?;
        supply.decisions.retain(|item| {
            self.registered_work_keys
                .insert(item.stable_work_key.clone())
        });
        self.pending_decisions.extend(supply.decisions);
        if let Some(family) = supply.selection_family {
            if self.registered_work_keys.insert(family.family_key.clone()) {
                self.pending_selection_families.push_back(family);
            }
        }
        Ok(())
    }

    pub(super) fn register_explicit_decisions_for_branch(
        &mut self,
        branch_id: usize,
        decision_prior: Option<RunPolicyPriorFnV1>,
    ) -> Result<(), String> {
        let boundary = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
            .map(|branch| branch.boundary)
            .ok_or_else(|| format!("missing oracle run branch {branch_id}"))?;
        match boundary {
            OracleRunBoundaryV1::Combat
            | OracleRunBoundaryV1::TerminalVictory
            | OracleRunBoundaryV1::TerminalDefeat => Ok(()),
            _ => self.register_decision_work(branch_id, decision_prior),
        }
    }

    fn release_next_selection_member(&mut self, completed_work_key: &str) -> Result<(), String> {
        let Some(index) = self
            .pending_selection_families
            .iter()
            .position(|family| family.outstanding_work_key.as_deref() == Some(completed_work_key))
        else {
            return Ok(());
        };
        let mut family = self
            .pending_selection_families
            .remove(index)
            .expect("located selection family must remain present");
        family.outstanding_work_key = None;
        let Some(action) = selection_family_next_action(&mut family) else {
            return Ok(());
        };
        let decision = selection_family_decision(&mut family, action)?;
        if !self
            .registered_work_keys
            .insert(decision.stable_work_key.clone())
        {
            return Err(format!(
                "selection family '{}' emitted duplicate exact work '{}'",
                family.family_key, decision.stable_work_key
            ));
        }
        self.pending_decisions.push_back(decision);
        if !family.cursor.is_exhausted() {
            self.pending_selection_families.push_back(family);
        }
        Ok(())
    }

    fn schedule_branch(
        &mut self,
        branch_id: usize,
        combat_budgets: &OracleRunCombatBudgetsV1,
        decision_prior: Option<RunPolicyPriorFnV1>,
    ) -> Result<(), String> {
        let branch = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
            .ok_or_else(|| format!("missing oracle run branch {branch_id}"))?;
        match branch.boundary {
            OracleRunBoundaryV1::Combat => {
                if !self.pending_combats.is_empty() {
                    return Err(format!(
                        "oracle attempted to start combat branch {branch_id} while another lazy combat edge was active"
                    ));
                }
                let key = format!("combat:{}", branch.state_fingerprint);
                if !self.registered_work_keys.insert(key) {
                    return Ok(());
                }
                let work = OracleRunCombatWorkV1::new_with_guidance(
                    &branch.session,
                    combat_budgets.for_session_stage(&branch.session, 0),
                    combat_budgets.guidance_bundle.as_deref(),
                )?;
                self.pending_combats.push_back(PendingOracleCombatV1 {
                    branch_id,
                    stage: 0,
                    work,
                });
                Ok(())
            }
            OracleRunBoundaryV1::TerminalVictory | OracleRunBoundaryV1::TerminalDefeat => Ok(()),
            _ => self.register_decision_work(branch_id, decision_prior),
        }
    }

    fn start_deferred_combat(
        &mut self,
        deferred: DeferredOracleCombatV1,
        combat_budgets: &OracleRunCombatBudgetsV1,
    ) -> Result<(), String> {
        if !self.pending_combats.is_empty() {
            return Err(
                "oracle cannot resume a deferred combat while another edge is active".into(),
            );
        }
        let branch = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == deferred.branch_id)
            .ok_or_else(|| {
                format!(
                    "missing deferred oracle combat branch {}",
                    deferred.branch_id
                )
            })?;
        let work = OracleRunCombatWorkV1::restart_for_higher_fidelity_with_guidance(
            &branch.session,
            combat_budgets.for_session_stage(&branch.session, deferred.stage),
            deferred.prior_work,
            combat_budgets.guidance_bundle.as_deref(),
        )?;
        self.pending_combats.push_back(PendingOracleCombatV1 {
            branch_id: deferred.branch_id,
            stage: deferred.stage,
            work,
        });
        self.combat_search_restarts = self.combat_search_restarts.saturating_add(1);
        Ok(())
    }

    pub(super) fn materialize_decision(
        &mut self,
        work: LazyOracleRunDecisionV1,
        decision_annotation: Option<OracleRunDecisionAnnotationFnV1>,
    ) -> Result<Option<usize>, String> {
        let parent = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == work.parent_branch_id)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "oracle decision references missing parent branch {}",
                    work.parent_branch_id
                )
            })?;
        if parent.state_fingerprint != work.parent_state_fingerprint {
            return Err(format!(
                "oracle decision parent fingerprint changed for branch {}",
                work.parent_branch_id
            ));
        }

        let annotation =
            decision_annotation.and_then(|annotate| annotate(&parent.session, &work.candidate_id));
        let successor = super::exact_run_decision_successor_v1(
            &parent.session,
            &work.candidate_id,
            work.action.clone(),
        )?;
        let mut session = successor.session;
        let mut transaction = successor.transaction;
        if let Some(annotation) = annotation {
            transaction.trace_annotations.push(annotation);
        }
        let forced_steps = settle_oracle_forced_transitions(&mut session)?;
        let successor_fingerprint = run_session_fingerprint_v2(&session);
        if successor_fingerprint == parent.state_fingerprint {
            return Err(format!(
                "oracle decision '{}' ({}) produced no state change at branch {}; \
                 executable decision surfaces must not expose no-op actions",
                work.label, work.candidate_id, parent.branch_id
            ));
        }
        let mut journal = parent.journal;
        journal.append_committed_steps(vec![RunProgressStepV1::Decision(transaction)])?;
        journal.append_committed_steps(forced_steps)?;
        let mut replay = parent.replay;
        replay.push(OracleRunReplayStepV1 {
            candidate_id: work.candidate_id,
            label: work.label,
            action: work.action,
        });
        let child = OracleRunBranchV1 {
            branch_id: self.next_branch_id,
            parent_branch_id: Some(parent.branch_id),
            neow_root_candidate_id: parent.neow_root_candidate_id,
            neow_root_label: parent.neow_root_label,
            state_fingerprint: successor_fingerprint,
            boundary: classify_run_boundary(&session),
            path_negative_log_policy: work.path_negative_log_policy,
            path_discrepancy: work.path_discrepancy,
            path_depth: work.path_depth,
            replay,
            journal,
            session,
        };
        self.next_branch_id = self.next_branch_id.saturating_add(1);
        Ok(self.accept_branch(child))
    }

    pub(super) fn materialize_explicit_decision(
        &mut self,
        work: LazyOracleRunDecisionV1,
        decision_annotation: Option<OracleRunDecisionAnnotationFnV1>,
    ) -> Result<usize, String> {
        let duplicate_count = self.retired_exact_duplicates.len();
        if let Some(branch_id) = self.materialize_decision(work, decision_annotation)? {
            return Ok(branch_id);
        }
        self.retired_exact_duplicates
            .get(duplicate_count)
            .map(|duplicate| duplicate.survivor_branch_id)
            .ok_or_else(|| {
                "explicit oracle decision was discarded without an exact-duplicate record"
                    .to_string()
            })
    }

    pub(super) fn note_explicit_decision_service(
        &mut self,
        stable_work_key: &str,
    ) -> Result<(), String> {
        // Analysis variations do not consume the parent's legal choices.
        // For a parameterized selection, explicitly trying the currently
        // exposed member only widens the immutable parent by one more exact
        // member. Production scheduling removes serviced work separately.
        self.release_next_selection_member(stable_work_key)
    }

    pub(super) fn drain_pending_combats(&mut self) -> Vec<(usize, OracleRunCombatWorkV1)> {
        self.pending_combats
            .drain(..)
            .map(|pending| (pending.branch_id, pending.work))
            .collect()
    }

    pub(super) fn materialize_explicit_combat(
        &mut self,
        branch_id: usize,
        work: OracleRunCombatWorkV1,
    ) -> Result<Option<usize>, String> {
        match self.finish_combat(
            PendingOracleCombatV1 {
                branch_id,
                stage: 0,
                work,
            },
            None,
        )? {
            FinishedOracleCombatV1::Resolved(branch_id) => Ok(Some(branch_id)),
            FinishedOracleCombatV1::ExactDuplicate => self
                .retired_exact_duplicates
                .last()
                .map(|duplicate| Some(duplicate.survivor_branch_id))
                .ok_or_else(|| {
                    "explicit oracle combat duplicated without a survivor record".to_string()
                }),
            FinishedOracleCombatV1::Unresolved(unresolved) => {
                self.unresolved_combats.push(unresolved);
                Ok(None)
            }
        }
    }

    pub(super) fn materialize_explicit_smoke_bomb_escape(
        &mut self,
        branch_id: usize,
    ) -> Result<Option<usize>, String> {
        let parent = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
            .cloned()
            .ok_or_else(|| format!("missing oracle combat branch {branch_id}"))?;
        let mut session = parent.session.clone();
        let outcome =
            super::combat_no_win_fallback::try_apply_smoke_bomb_survival_fallback_after_rejection(
                &mut session,
                "explicit oracle escape",
            )?
            .ok_or_else(|| {
                format!(
                    "oracle combat branch {branch_id} has no currently usable Smoke Bomb escape"
                )
            })?;
        let finished =
            self.accept_resolved_combat_branch(parent, session, outcome.progress_steps)?;
        match finished {
            FinishedOracleCombatV1::Resolved(branch_id) => Ok(Some(branch_id)),
            FinishedOracleCombatV1::ExactDuplicate => self
                .retired_exact_duplicates
                .last()
                .map(|duplicate| Some(duplicate.survivor_branch_id))
                .ok_or_else(|| {
                    "explicit Smoke Bomb escape duplicated without a survivor record".to_string()
                }),
            FinishedOracleCombatV1::Unresolved(_) => {
                Err("explicit Smoke Bomb escape unexpectedly remained unresolved".to_string())
            }
        }
    }

    fn finish_combat(
        &mut self,
        pending: PendingOracleCombatV1,
        finalization_deadline: Option<Instant>,
    ) -> Result<FinishedOracleCombatV1, String> {
        let parent = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == pending.branch_id)
            .cloned()
            .ok_or_else(|| format!("missing oracle combat branch {}", pending.branch_id))?;
        let progress = pending.work.progress();
        let nodes_expanded = progress.generation_work;
        let _ = finalization_deadline;
        let mut session = parent.session.clone();
        let outcome = pending
            .work
            .finish_and_apply(&mut session)
            .map_err(|error| {
                format!(
                "oracle combat branch {} at Act {} Floor {} failed to commit its witness: {error}",
                parent.branch_id,
                parent.session.run_state.act_num,
                parent.session.run_state.floor_num
            )
            })?;
        if outcome.progress_steps.is_empty() {
            let rejection = outcome.combat_search_rejection.ok_or_else(|| {
                format!(
                    "oracle combat branch {} made no progress without typed rejection",
                    parent.branch_id
                )
            })?;
            let unresolved = OracleRunUnresolvedCombatV1 {
                branch_id: parent.branch_id,
                rejection,
                evidence_kind: match progress.last_status {
                    Some("frontier_exhausted") if progress.generation_gap_count == 0 => {
                        "exhaustive_refutation"
                    }
                    Some("mechanics_gap") | Some("replay_mismatch") => "setup_or_mechanics_error",
                    _ => "budget_unknown",
                }
                .to_string(),
                last_status: progress.last_status.map(str::to_string),
                nodes_expanded,
                exact_states: progress.exact_states,
                applied_action_transitions: progress.applied_action_transitions,
                unique_successor_states: progress.unique_successor_states,
                duplicate_exact_successors: progress.duplicate_exact_successors,
                completed_turn_options: progress.completed_turn_options,
                retained_state_work: progress.retained_state_work,
                max_player_turn: progress.max_player_turn,
                max_path_atomic_depth: progress.max_path_atomic_depth,
                generation_gap_count: progress.generation_gap_count,
                incumbent_final_hp: progress.incumbent_final_hp,
            };
            return Ok(FinishedOracleCombatV1::Unresolved(unresolved));
        }
        self.accept_resolved_combat_branch(parent, session, outcome.progress_steps)
    }

    fn accept_resolved_combat_branch(
        &mut self,
        parent: OracleRunBranchV1,
        session: RunControlSession,
        progress_steps: Vec<RunProgressStepV1>,
    ) -> Result<FinishedOracleCombatV1, String> {
        if progress_steps.len() != 1 {
            return Err(format!(
                "oracle combat branch {} committed {} progress steps; expected one",
                parent.branch_id,
                progress_steps.len()
            ));
        }
        let mut journal = parent.journal;
        journal.append_committed_steps(progress_steps)?;
        let child = OracleRunBranchV1 {
            branch_id: self.next_branch_id,
            parent_branch_id: Some(parent.branch_id),
            neow_root_candidate_id: parent.neow_root_candidate_id,
            neow_root_label: parent.neow_root_label,
            state_fingerprint: run_session_fingerprint_v2(&session),
            boundary: classify_run_boundary(&session),
            path_negative_log_policy: parent.path_negative_log_policy,
            path_discrepancy: parent.path_discrepancy,
            path_depth: parent.path_depth.saturating_add(1),
            replay: parent.replay,
            journal,
            session,
        };
        self.next_branch_id = self.next_branch_id.saturating_add(1);
        Ok(match self.accept_branch(child) {
            Some(branch_id) => FinishedOracleCombatV1::Resolved(branch_id),
            None => FinishedOracleCombatV1::ExactDuplicate,
        })
    }
}

fn settle_oracle_forced_transitions(
    session: &mut RunControlSession,
) -> Result<Vec<RunProgressStepV1>, String> {
    let mut steps = Vec::new();
    if matches!(session.engine_state, EngineState::Campfire)
        && crate::engine::campfire_handler::get_available_options(&session.run_state).is_empty()
    {
        let transition = session
            .execute_forced_transition(super::RunForcedTransitionKindV1::EmptyCampfireExit)?;
        steps.push(RunProgressStepV1::ForcedTransition(transition));
    }
    Ok(steps)
}

pub fn seed_oracle_run_explorer_v1(
    expansion: NeowOracleExpansionV1,
    decision_prior: Option<RunPolicyPriorFnV1>,
) -> Result<OracleRunExplorerV1, String> {
    if !expansion.unresolved.is_empty() {
        return Err(format!(
            "cannot seed oracle run while {} Neow outcomes remain unresolved",
            expansion.unresolved.len()
        ));
    }
    let mut explorer = OracleRunExplorerV1::empty();
    let root_count = expansion.completed.len().max(1);
    let root_negative_log_policy = (root_count as f64).ln();
    let mut root_ids = Vec::new();
    for candidate in expansion.completed {
        if !candidate.session.engine_state.is_map_surface() {
            return Err(format!(
                "completed Neow candidate '{}' is not at a map boundary",
                candidate.root_candidate_id
            ));
        }
        let branch_id = explorer.next_branch_id;
        explorer.next_branch_id = explorer.next_branch_id.saturating_add(1);
        let session = candidate.session;
        let branch = OracleRunBranchV1 {
            branch_id,
            parent_branch_id: None,
            neow_root_candidate_id: candidate.root_candidate_id,
            neow_root_label: candidate.root_label,
            state_fingerprint: run_session_fingerprint_v2(&session),
            boundary: classify_run_boundary(&session),
            path_negative_log_policy: root_negative_log_policy,
            path_discrepancy: 0,
            path_depth: 1,
            replay: candidate
                .replay
                .into_iter()
                .map(|step| OracleRunReplayStepV1 {
                    candidate_id: step.candidate_id,
                    label: step.label,
                    action: step.action,
                })
                .collect(),
            journal: candidate.journal,
            session,
        };
        if let Some(root_id) = explorer.accept_branch(branch) {
            root_ids.push(root_id);
        }
    }

    let mut work_by_root = BTreeMap::<String, VecDeque<LazyOracleRunDecisionV1>>::new();
    for branch_id in root_ids {
        let branch = explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
            .ok_or_else(|| format!("missing oracle root branch {branch_id}"))?;
        let supply = decision_supply_for_branch(branch, decision_prior)?;
        for work in supply.decisions {
            if explorer
                .registered_work_keys
                .insert(work.stable_work_key.clone())
            {
                work_by_root
                    .entry(branch.neow_root_candidate_id.clone())
                    .or_default()
                    .push_back(work);
            }
        }
        if let Some(family) = supply.selection_family {
            if !explorer
                .registered_work_keys
                .insert(family.family_key.clone())
            {
                return Err(format!(
                    "oracle root selection family '{}' was registered twice",
                    family.family_key
                ));
            }
            explorer.pending_selection_families.push_back(family);
        }
    }
    loop {
        let mut added = false;
        for root_work in work_by_root.values_mut() {
            if let Some(work) = root_work.pop_front() {
                explorer.pending_decisions.push_back(work);
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    Ok(explorer)
}

/// Resume exact oracle execution from one already committed run state.
///
/// This deliberately restores no historical sibling frontier. The journal is
/// carried forward solely so a later victory remains replayable from the
/// original run start.
pub fn seed_oracle_run_explorer_from_session_v1(
    session: RunControlSession,
    journal: RunProgressJournalV1,
    combat_budgets: &OracleRunCombatBudgetsV1,
    decision_prior: Option<RunPolicyPriorFnV1>,
) -> Result<OracleRunExplorerV1, String> {
    let mut explorer = OracleRunExplorerV1::empty();
    let branch_id = explorer.next_branch_id;
    explorer.next_branch_id = explorer.next_branch_id.saturating_add(1);
    let branch = OracleRunBranchV1 {
        branch_id,
        parent_branch_id: None,
        neow_root_candidate_id: "continued-exact-state".to_string(),
        neow_root_label: "continued exact state".to_string(),
        state_fingerprint: run_session_fingerprint_v2(&session),
        boundary: classify_run_boundary(&session),
        path_negative_log_policy: 0.0,
        path_discrepancy: 0,
        path_depth: 1,
        replay: Vec::new(),
        journal,
        session,
    };
    let branch_id = explorer
        .accept_branch(branch)
        .ok_or_else(|| "continued oracle state was unexpectedly duplicated".to_string())?;
    explorer.schedule_branch(branch_id, combat_budgets, decision_prior)?;
    Ok(explorer)
}

pub fn seed_oracle_run_explorer_from_checkpoint_v1(
    checkpoint: OracleRunExplorerCheckpointV1,
    combat_budgets: &OracleRunCombatBudgetsV1,
) -> Result<OracleRunExplorerV1, String> {
    let OracleRunExplorerCheckpointV1 {
        state_fingerprint_algorithm,
        next_branch_id,
        branches,
        pending_decisions,
        pending_selection_families,
        active_combat_branch_id,
        active_combat,
        deferred_combats,
        journal_nodes,
        combat_search_restarts,
        last_served_neow_root,
        unresolved_combats,
    } = checkpoint;
    let migrate_state_fingerprints = match state_fingerprint_algorithm.as_deref() {
        None | Some(ORACLE_RUN_STATE_FINGERPRINT_ALGORITHM_V1) => true,
        Some(ORACLE_RUN_STATE_FINGERPRINT_ALGORITHM) => false,
        Some(algorithm) => {
            return Err(format!(
                "unsupported oracle run state fingerprint algorithm '{algorithm}'"
            ));
        }
    };
    let mut explorer = OracleRunExplorerV1::empty();
    explorer.next_branch_id = next_branch_id;
    explorer.combat_search_restarts = combat_search_restarts;
    explorer.last_served_neow_root = last_served_neow_root;
    explorer.unresolved_combats = unresolved_combats;
    for saved in branches {
        let journal =
            checkpoint::restore_frontier_journal(saved.journal, saved.journal_tip, &journal_nodes)?;
        let session = saved.session.into_session()?;
        let actual_fingerprint = run_session_fingerprint_v2(&session);
        if !migrate_state_fingerprints && actual_fingerprint != saved.state_fingerprint {
            return Err(format!(
                "oracle frontier branch {} fingerprint changed while restoring",
                saved.branch_id
            ));
        }
        let branch = OracleRunBranchV1 {
            branch_id: saved.branch_id,
            parent_branch_id: saved.parent_branch_id,
            neow_root_candidate_id: saved.neow_root_candidate_id,
            neow_root_label: saved.neow_root_label,
            state_fingerprint: actual_fingerprint,
            boundary: saved.boundary,
            path_negative_log_policy: saved.path_negative_log_policy,
            path_discrepancy: saved.path_discrepancy,
            path_depth: saved.path_depth,
            replay: saved.replay,
            journal,
            session,
        };
        if explorer.accept_branch(branch).is_none() {
            return Err("oracle frontier checkpoint contained duplicate states".to_string());
        }
    }
    explorer.next_branch_id = explorer.next_branch_id.max(
        explorer
            .branches
            .iter()
            .map(|branch| branch.branch_id.saturating_add(1))
            .max()
            .unwrap_or(0),
    );
    let mut migrated_work_keys = BTreeMap::new();
    for mut decision in pending_decisions {
        let parent = explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == decision.parent_branch_id)
            .ok_or_else(|| {
                format!(
                    "oracle frontier decision references missing branch {}",
                    decision.parent_branch_id
                )
            })?;
        if !migrate_state_fingerprints
            && parent.state_fingerprint != decision.parent_state_fingerprint
        {
            return Err(format!(
                "oracle frontier decision parent fingerprint changed for branch {}",
                decision.parent_branch_id
            ));
        }
        if migrate_state_fingerprints {
            let old_work_key = decision.stable_work_key.clone();
            decision.parent_state_fingerprint = parent.state_fingerprint.clone();
            decision.stable_work_key = stable_oracle_work_key(
                &decision.parent_state_fingerprint,
                &decision.candidate_id,
                &decision.action,
            );
            migrated_work_keys.insert(old_work_key, decision.stable_work_key.clone());
        }
        decision.parent_act = parent.session.run_state.act_num;
        decision.parent_floor = parent.session.run_state.floor_num;
        if explorer
            .registered_work_keys
            .insert(decision.stable_work_key.clone())
        {
            explorer.pending_decisions.push_back(decision);
        }
    }
    for mut family in pending_selection_families {
        let parent = explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == family.parent_branch_id)
            .ok_or_else(|| {
                format!(
                    "oracle frontier selection family references missing branch {}",
                    family.parent_branch_id
                )
            })?;
        if !migrate_state_fingerprints
            && parent.state_fingerprint != family.parent_state_fingerprint
        {
            return Err(format!(
                "oracle frontier selection family parent fingerprint changed for branch {}",
                family.parent_branch_id
            ));
        }
        if family.cursor.is_exhausted() {
            return Err(format!(
                "oracle frontier selection family '{}' persisted after exhaustion",
                family.family_key
            ));
        }
        if migrate_state_fingerprints {
            family.parent_state_fingerprint = parent.state_fingerprint.clone();
            let (min_count, max_count) = family.cursor.selection_bounds();
            family.family_key = selection_family_work_key(
                &family.parent_state_fingerprint,
                &family.candidate_id,
                min_count,
                max_count,
            );
            family.outstanding_work_key = family
                .outstanding_work_key
                .as_ref()
                .and_then(|key| migrated_work_keys.get(key))
                .cloned();
        }
        let Some(outstanding_work_key) = family.outstanding_work_key.as_deref() else {
            return Err(format!(
                "oracle frontier selection family '{}' has no outstanding exact member",
                family.family_key
            ));
        };
        if !explorer
            .pending_decisions
            .iter()
            .any(|decision| decision.stable_work_key == outstanding_work_key)
        {
            return Err(format!(
                "oracle frontier selection family '{}' lost outstanding member '{}'",
                family.family_key, outstanding_work_key
            ));
        }
        if !explorer
            .registered_work_keys
            .insert(family.family_key.clone())
        {
            return Err(format!(
                "oracle frontier duplicated selection family '{}'",
                family.family_key
            ));
        }
        explorer.pending_selection_families.push_back(family);
    }
    if let (Some(legacy_branch_id), Some(active)) = (active_combat_branch_id, &active_combat) {
        if legacy_branch_id != active.branch_id {
            return Err(format!(
                "oracle frontier names conflicting active combat branches {legacy_branch_id} and {}",
                active.branch_id
            ));
        }
    }
    if let Some(active) = active_combat {
        let branch_id = active.branch_id;
        let branch = explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
            .ok_or_else(|| {
                format!("oracle frontier combat references missing branch {branch_id}")
            })?;
        if branch.boundary != OracleRunBoundaryV1::Combat {
            return Err(format!(
                "oracle frontier active branch {branch_id} is not at a combat boundary"
            ));
        }
        let key = format!("combat:{}", branch.state_fingerprint);
        if !explorer.registered_work_keys.insert(key) {
            return Err(format!(
                "oracle frontier active combat branch {branch_id} duplicates registered work"
            ));
        }
        if explorer.last_served_neow_root.is_none() {
            explorer.last_served_neow_root = Some(branch.neow_root_candidate_id.clone());
        }
        let work = OracleRunCombatWorkV1::restart_from_checkpoint_with_guidance(
            &branch.session,
            combat_budgets.for_session_stage(&branch.session, active.stage),
            active.work,
            combat_budgets.guidance_bundle.as_deref(),
        )?;
        explorer.pending_combats.push_back(PendingOracleCombatV1 {
            branch_id,
            stage: active.stage,
            work,
        });
        explorer.combat_search_restarts = explorer.combat_search_restarts.saturating_add(1);
    } else if let Some(branch_id) = active_combat_branch_id {
        let branch = explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
            .ok_or_else(|| {
                format!("oracle frontier combat references missing branch {branch_id}")
            })?;
        if branch.boundary != OracleRunBoundaryV1::Combat {
            return Err(format!(
                "oracle frontier active branch {branch_id} is not at a combat boundary"
            ));
        }
        let key = format!("combat:{}", branch.state_fingerprint);
        if !explorer.registered_work_keys.insert(key) {
            return Err(format!(
                "oracle frontier active combat branch {branch_id} duplicates registered work"
            ));
        }
        if explorer.last_served_neow_root.is_none() {
            explorer.last_served_neow_root = Some(branch.neow_root_candidate_id.clone());
        }
        let work = OracleRunCombatWorkV1::restart_from_exact_state_with_guidance(
            &branch.session,
            combat_budgets.for_session(&branch.session),
            combat_budgets.guidance_bundle.as_deref(),
        )?;
        explorer.pending_combats.push_back(PendingOracleCombatV1 {
            branch_id,
            stage: 0,
            work,
        });
        explorer.combat_search_restarts = explorer.combat_search_restarts.saturating_add(1);
    }
    for deferred in deferred_combats {
        let branch = explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == deferred.branch_id)
            .ok_or_else(|| {
                format!(
                    "oracle frontier deferred combat references missing branch {}",
                    deferred.branch_id
                )
            })?;
        if branch.boundary != OracleRunBoundaryV1::Combat {
            return Err(format!(
                "oracle frontier deferred branch {} is not at a combat boundary",
                deferred.branch_id
            ));
        }
        let key = format!("combat:{}", branch.state_fingerprint);
        if !explorer.registered_work_keys.insert(key) {
            return Err(format!(
                "oracle frontier deferred combat branch {} duplicates registered work",
                deferred.branch_id
            ));
        }
        explorer.deferred_combats.push_back(DeferredOracleCombatV1 {
            branch_id: deferred.branch_id,
            stage: deferred.stage,
            prior_work: deferred.prior_work,
        });
    }
    Ok(explorer)
}

pub fn drive_oracle_run_explorer_v1(
    mut explorer: OracleRunExplorerV1,
    budget: OracleRunExploreBudgetV1,
) -> Result<OracleRunExploreResultV1, String> {
    if budget.max_work_items == 0 {
        return Err("oracle run work budget must be positive".to_string());
    }
    if budget.combat_quantum_nodes == 0 {
        return Err("oracle combat quantum node budget must be positive".to_string());
    }
    let started = Instant::now();
    let deadline = budget
        .wall_ms
        .and_then(|wall_ms| started.checked_add(Duration::from_millis(wall_ms)));
    let quantum = RunControlCombatSearchQuantum {
        label: "oracle_run_quantum",
        additional_nodes: budget.combat_quantum_nodes,
        soft_wall_ms: budget.combat_quantum_ms,
    };
    let mut work_items = 0usize;
    let mut combat_quanta = 0usize;
    let mut combat_edge_probe_evaluations = 0usize;
    let mut immediate_combat_edge_hints = 0usize;
    let mut decision_service = Duration::ZERO;
    let mut combat_service = Duration::ZERO;

    let stop = loop {
        if work_items >= budget.max_work_items {
            break OracleRunExploreStopV1::WorkBudgetExhausted;
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            break OracleRunExploreStopV1::WallDeadlineReached;
        }
        let has_decision = !explorer.pending_decisions.is_empty();
        let has_combat = !explorer.pending_combats.is_empty();
        let has_deferred_combat = !explorer.deferred_combats.is_empty();
        if !has_decision && !has_combat && !has_deferred_combat {
            break OracleRunExploreStopV1::WorkExhausted;
        }

        // A combat is an expensive, incrementally evaluated edge on the
        // currently selected strategic prefix.  While it is active, no
        // sibling strategic prefix may start another combat.
        if has_combat {
            let mut pending = explorer
                .pending_combats
                .pop_front()
                .expect("combat existence checked above");
            let service_started = Instant::now();
            let advance = pending.work.advance(&quantum, deadline);
            let service_elapsed = service_started.elapsed();
            combat_service = combat_service.saturating_add(service_elapsed);
            work_items = work_items.saturating_add(1);
            combat_quanta = combat_quanta.saturating_add(1);
            match advance {
                RunControlCombatWorkAdvanceV1::Pending => {
                    explorer.pending_combats.push_front(pending);
                }
                RunControlCombatWorkAdvanceV1::GlobalDeadlineReached => {
                    explorer.pending_combats.push_front(pending);
                    break OracleRunExploreStopV1::WallDeadlineReached;
                }
                RunControlCombatWorkAdvanceV1::ReadyToFinish
                | RunControlCombatWorkAdvanceV1::AllowanceExhausted => {
                    let stage = pending.stage;
                    let has_later_stage = explorer
                        .branches
                        .iter()
                        .find(|branch| branch.branch_id == pending.branch_id)
                        .is_some_and(|branch| {
                            budget.combat.has_later_stage(&branch.session, stage)
                        });
                    let prior_work = has_later_stage.then(|| pending.work.checkpoint());
                    if has_later_stage
                        && pending.work.has_verified_witness()
                        && !pending.work.has_quality_satisfying_witness()
                    {
                        explorer.deferred_combats.push_back(DeferredOracleCombatV1 {
                            branch_id: pending.branch_id,
                            stage: stage.saturating_add(1),
                            prior_work: prior_work.expect("later stage preserves prior work"),
                        });
                        continue;
                    }
                    let finished = explorer.finish_combat(pending, deadline)?;
                    match finished {
                        FinishedOracleCombatV1::Resolved(branch_id) => {
                            let boundary = explorer
                                .branches
                                .iter()
                                .find(|branch| branch.branch_id == branch_id)
                                .map(|branch| branch.boundary)
                                .ok_or_else(|| {
                                    format!("missing resolved combat branch {branch_id}")
                                })?;
                            if boundary == OracleRunBoundaryV1::TerminalVictory {
                                break OracleRunExploreStopV1::Victory { branch_id };
                            }
                            explorer.schedule_branch(
                                branch_id,
                                &budget.combat,
                                budget.decision_prior,
                            )?;
                        }
                        FinishedOracleCombatV1::Unresolved(unresolved) => {
                            if unresolved.evidence_kind == "budget_unknown" {
                                if let Some(prior_work) = prior_work {
                                    explorer.deferred_combats.push_back(DeferredOracleCombatV1 {
                                        branch_id: unresolved.branch_id,
                                        stage: stage.saturating_add(1),
                                        prior_work,
                                    });
                                    continue;
                                }
                            }
                            explorer.unresolved_combats.push(unresolved);
                        }
                        FinishedOracleCombatV1::ExactDuplicate => {}
                    }
                }
            }
            continue;
        }

        let (probe_evaluations, immediate_hints) =
            explorer.refresh_combat_edge_probes(budget.combat_edge_order)?;
        combat_edge_probe_evaluations =
            combat_edge_probe_evaluations.saturating_add(probe_evaluations);
        immediate_combat_edge_hints = immediate_combat_edge_hints.saturating_add(immediate_hints);
        let scheduled = explorer
            .take_next_scheduled_work()
            .expect("strategic work existence checked above");
        let service_started = Instant::now();
        work_items = work_items.saturating_add(1);
        match scheduled {
            ScheduledOracleRunWorkV1::Decision(decision) => {
                explorer.release_next_selection_member(&decision.stable_work_key)?;
                if let Some(branch_id) =
                    explorer.materialize_decision(decision, budget.decision_annotation)?
                {
                    let boundary = explorer
                        .branches
                        .iter()
                        .find(|branch| branch.branch_id == branch_id)
                        .map(|branch| branch.boundary)
                        .ok_or_else(|| format!("missing materialized oracle branch {branch_id}"))?;
                    if boundary == OracleRunBoundaryV1::TerminalVictory {
                        break OracleRunExploreStopV1::Victory { branch_id };
                    }
                    explorer.schedule_branch(branch_id, &budget.combat, budget.decision_prior)?;
                }
            }
            ScheduledOracleRunWorkV1::DeferredCombat(deferred) => {
                explorer.start_deferred_combat(deferred, &budget.combat)?;
            }
        }
        let service_elapsed = service_started.elapsed();
        decision_service = decision_service.saturating_add(service_elapsed);
    };

    Ok(OracleRunExploreResultV1 {
        explorer,
        stop,
        work_items,
        combat_quanta,
        decision_service_ms: duration_ms(decision_service),
        combat_service_ms: duration_ms(combat_service),
        combat_edge_probe_evaluations,
        immediate_combat_edge_hints,
        elapsed_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
    })
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}

#[derive(Serialize)]
struct StableOracleWorkKeyInput<'a> {
    parent_state_fingerprint: &'a str,
    candidate_id: &'a str,
    action: &'a RunDecisionAction,
}

fn stable_oracle_work_key(
    parent_state_fingerprint: &str,
    candidate_id: &str,
    action: &RunDecisionAction,
) -> String {
    crate::eval::fingerprint::hash_serializable(&StableOracleWorkKeyInput {
        parent_state_fingerprint,
        candidate_id,
        action,
    })
}

fn selection_family_work_key(
    parent_state_fingerprint: &str,
    candidate_id: &str,
    min_count: usize,
    max_count: usize,
) -> String {
    crate::eval::fingerprint::hash_serializable(&(
        "oracle_run_selection_family_v1",
        parent_state_fingerprint,
        candidate_id,
        min_count,
        max_count,
    ))
}

struct OracleRunDecisionSupplyV1 {
    decisions: Vec<LazyOracleRunDecisionV1>,
    selection_family: Option<LazyOracleRunSelectionFamilyV1>,
}

fn decision_supply_for_branch(
    branch: &OracleRunBranchV1,
    decision_prior: Option<RunPolicyPriorFnV1>,
) -> Result<OracleRunDecisionSupplyV1, String> {
    let kind = work_kind(branch.boundary)?;
    let surface = build_decision_surface(&branch.session);
    let has_symbolic_selection = matches!(
        branch.session.engine_state,
        EngineState::RunPendingChoice(_)
    ) && surface.view.candidates.iter().any(|candidate| {
        matches!(
            candidate.key,
            Some(DecisionCandidateKey::SelectionSubmit { .. })
        ) && candidate.action.executable_action().is_none()
    });
    if has_symbolic_selection {
        let (decision, selection_family) =
            run_choice_family_for_branch(branch, kind, &surface, decision_prior)?;
        return Ok(OracleRunDecisionSupplyV1 {
            decisions: vec![decision],
            selection_family,
        });
    }

    let mut work = Vec::new();
    for candidate in surface.view.candidates {
        let Some(action) = candidate.action.executable_action() else {
            continue;
        };
        if should_normalize_navigation_away(&branch.session, &action) {
            continue;
        }
        work.push(lazy_decision(
            branch,
            kind,
            candidate.id,
            candidate.label,
            action,
        ));
    }
    if work.is_empty() {
        return Err(format!(
            "oracle {:?} branch {} exposed no executable strategic action",
            branch.boundary, branch.branch_id
        ));
    }
    apply_decision_policy(branch, &mut work, decision_prior)?;
    Ok(OracleRunDecisionSupplyV1 {
        decisions: work,
        selection_family: None,
    })
}

fn apply_decision_policy(
    branch: &OracleRunBranchV1,
    work: &mut [LazyOracleRunDecisionV1],
    decision_prior: Option<RunPolicyPriorFnV1>,
) -> Result<(), String> {
    let prior = {
        let legal = work
            .iter()
            .map(|candidate| RunPolicyCandidateV1 {
                candidate_id: &candidate.candidate_id,
                label: &candidate.label,
                action: &candidate.action,
            })
            .collect::<Vec<_>>();
        let prior = match decision_prior {
            Some(policy) => policy(&branch.session, &legal)?,
            None => positive_ranked_run_policy_prior_v1(&legal, std::iter::empty())?,
        };
        prior.validate_for(&legal)?;
        prior
    };

    let work_indices = work
        .iter()
        .enumerate()
        .map(|(index, candidate)| (candidate.candidate_id.clone(), index))
        .collect::<BTreeMap<_, _>>();
    for (rank, entry) in prior.entries.into_iter().enumerate() {
        let index = work_indices
            .get(&entry.candidate_id)
            .copied()
            .expect("validated policy prior must reference one legal candidate");
        work[index].path_negative_log_policy =
            branch.path_negative_log_policy - entry.probability.ln();
        work[index].path_discrepancy = branch.path_discrepancy.saturating_add(rank as u64);
        work[index].path_depth = branch.path_depth.saturating_add(1);
    }
    Ok(())
}

const RUN_SELECTION_PREFERRED_PREFIX: usize = 4;

fn run_choice_family_for_branch(
    branch: &OracleRunBranchV1,
    kind: OracleRunWorkKindV1,
    surface: &super::DecisionSurface,
    decision_prior: Option<RunPolicyPriorFnV1>,
) -> Result<
    (
        LazyOracleRunDecisionV1,
        Option<LazyOracleRunSelectionFamilyV1>,
    ),
    String,
> {
    let EngineState::RunPendingChoice(choice) = &branch.session.engine_state else {
        unreachable!("run choice work requires RunPendingChoice")
    };
    let request = choice.selection_request(&branch.session.run_state);
    let candidate = surface
        .view
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.key,
                Some(DecisionCandidateKey::SelectionSubmit { .. })
            )
        })
        .ok_or_else(|| "run choice has no bindable decision-surface candidate".to_string())?;
    let preferred = preferred_run_choice_selections(branch, choice);
    let cursor = LazyUnorderedSelectionCursorV1::new(
        request.targets,
        choice.min_choices,
        choice.max_choices,
        preferred,
    )?;
    let total_count = cursor.total_count();
    if total_count == 0 {
        return Err("run choice parameterized family contains no legal selections".to_string());
    }

    let family_key = selection_family_work_key(
        &branch.state_fingerprint,
        &candidate.id,
        choice.min_choices,
        choice.max_choices,
    );
    let mut family = LazyOracleRunSelectionFamilyV1 {
        family_key,
        parent_branch_id: branch.branch_id,
        parent_state_fingerprint: branch.state_fingerprint.clone(),
        neow_root_candidate_id: branch.neow_root_candidate_id.clone(),
        kind,
        candidate_id: candidate.id.clone(),
        label: candidate.label.clone(),
        path_negative_log_policy: branch.path_negative_log_policy,
        path_discrepancy: branch.path_discrepancy,
        path_depth: branch.path_depth.saturating_add(1),
        parent_act: branch.session.run_state.act_num,
        parent_floor: branch.session.run_state.floor_num,
        public_probability: 1.0,
        cursor,
        outstanding_work_key: None,
    };
    let first_action = selection_family_next_action(&mut family)
        .ok_or_else(|| "run choice selection cursor did not emit its first member".to_string())?;
    let legal = [RunPolicyCandidateV1 {
        candidate_id: &family.candidate_id,
        label: &family.label,
        action: &first_action,
    }];
    let prior = match decision_prior {
        Some(policy) => policy(&branch.session, &legal)?,
        None => positive_ranked_run_policy_prior_v1(&legal, std::iter::empty())?,
    };
    prior.validate_for(&legal)?;
    family.public_probability = prior.entries[0].probability;
    let first = selection_family_decision(&mut family, first_action)?;
    let remaining_family = (!family.cursor.is_exhausted()).then_some(family);
    Ok((first, remaining_family))
}

fn preferred_run_choice_selections(
    branch: &OracleRunBranchV1,
    choice: &crate::state::core::RunPendingChoiceState,
) -> Vec<Vec<SelectionTargetRef>> {
    let compiled = compile_deck_mutation_decision_v1(
        &branch.session.run_state,
        choice,
        DeckMutationCompilerRequestV1 {
            output: DeckMutationCompilerOutputV1::BranchTopK {
                max_active: RUN_SELECTION_PREFERRED_PREFIX,
            },
            commitment: DeckMutationCommitmentModeV1::CommittedForced,
        },
    );
    let mut seen = BTreeSet::new();
    compiled
        .selected_plan
        .iter()
        .chain(compiled.candidate_plans.iter())
        .filter_map(|plan| {
            let selected = plan
                .step
                .deck_indices
                .iter()
                .map(|index| {
                    branch
                        .session
                        .run_state
                        .master_deck
                        .get(*index)
                        .map(|card| SelectionTargetRef::CardUuid(card.uuid))
                })
                .collect::<Option<Vec<_>>>()?;
            let key = selected
                .iter()
                .map(|target| target.card_uuid())
                .collect::<Vec<_>>();
            seen.insert(key).then_some(selected)
        })
        .take(RUN_SELECTION_PREFERRED_PREFIX)
        .collect()
}

fn selection_family_next_action(
    family: &mut LazyOracleRunSelectionFamilyV1,
) -> Option<RunDecisionAction> {
    family.cursor.next_member().map(|member| {
        RunDecisionAction::Input(ClientInput::SubmitSelection(SelectionResolution {
            scope: SelectionScope::Deck,
            selected: member.selected,
        }))
    })
}

fn selection_family_decision(
    family: &mut LazyOracleRunSelectionFamilyV1,
    action: RunDecisionAction,
) -> Result<LazyOracleRunDecisionV1, String> {
    let exact_count = family.cursor.total_count() as f64;
    let exact_probability = family.public_probability / exact_count;
    if !exact_probability.is_finite() || exact_probability <= 0.0 {
        return Err(format!(
            "selection family '{}' produced invalid exact probability {exact_probability}",
            family.family_key
        ));
    }
    let rank = family.cursor.emitted_count().saturating_sub(1);
    let stable_work_key = stable_oracle_work_key(
        &family.parent_state_fingerprint,
        &family.candidate_id,
        &action,
    );
    family.outstanding_work_key = Some(stable_work_key.clone());
    Ok(LazyOracleRunDecisionV1 {
        parent_branch_id: family.parent_branch_id,
        parent_state_fingerprint: family.parent_state_fingerprint.clone(),
        neow_root_candidate_id: family.neow_root_candidate_id.clone(),
        kind: family.kind,
        candidate_id: family.candidate_id.clone(),
        label: family.label.clone(),
        action,
        stable_work_key,
        path_negative_log_policy: family.path_negative_log_policy - exact_probability.ln(),
        path_discrepancy: family.path_discrepancy.saturating_add(rank),
        path_depth: family.path_depth,
        parent_act: family.parent_act,
        parent_floor: family.parent_floor,
        combat_edge_probe: None,
    })
}

fn lazy_decision(
    branch: &OracleRunBranchV1,
    kind: OracleRunWorkKindV1,
    candidate_id: String,
    label: String,
    action: RunDecisionAction,
) -> LazyOracleRunDecisionV1 {
    let stable_work_key = stable_oracle_work_key(&branch.state_fingerprint, &candidate_id, &action);
    LazyOracleRunDecisionV1 {
        parent_branch_id: branch.branch_id,
        parent_state_fingerprint: branch.state_fingerprint.clone(),
        neow_root_candidate_id: branch.neow_root_candidate_id.clone(),
        kind,
        candidate_id,
        label,
        action,
        stable_work_key,
        path_negative_log_policy: branch.path_negative_log_policy,
        path_discrepancy: branch.path_discrepancy,
        path_depth: branch.path_depth.saturating_add(1),
        parent_act: branch.session.run_state.act_num,
        parent_floor: branch.session.run_state.floor_num,
        combat_edge_probe: None,
    }
}

fn combat_edge_probe_order(
    left: &LazyOracleRunDecisionV1,
    right: &LazyOracleRunDecisionV1,
) -> std::cmp::Ordering {
    match (left.combat_edge_probe, right.combat_edge_probe) {
        (
            Some(OracleRunCombatEdgeProbeV1::HeuristicEstimate {
                order_key: left_key,
            }),
            Some(OracleRunCombatEdgeProbeV1::HeuristicEstimate {
                order_key: right_key,
            }),
        ) => right_key.cmp(&left_key),
        _ => std::cmp::Ordering::Equal,
    }
}

fn oracle_run_decision_priority_order(
    left_index: usize,
    left: &LazyOracleRunDecisionV1,
    right_index: usize,
    right: &LazyOracleRunDecisionV1,
) -> std::cmp::Ordering {
    combat_edge_probe_order(left, right)
        .then_with(|| left.path_discrepancy.cmp(&right.path_discrepancy))
        .then_with(|| right.parent_act.cmp(&left.parent_act))
        .then_with(|| right.parent_floor.cmp(&left.parent_floor))
        .then_with(|| right.path_depth.cmp(&left.path_depth))
        .then_with(|| {
            left.path_negative_log_policy
                .total_cmp(&right.path_negative_log_policy)
        })
        .then_with(|| left_index.cmp(&right_index))
}

fn work_kind(boundary: OracleRunBoundaryV1) -> Result<OracleRunWorkKindV1, String> {
    match boundary {
        OracleRunBoundaryV1::MapDecision => Ok(OracleRunWorkKindV1::MapTravel),
        OracleRunBoundaryV1::Reward => Ok(OracleRunWorkKindV1::RewardAction),
        OracleRunBoundaryV1::Event => Ok(OracleRunWorkKindV1::EventOption),
        OracleRunBoundaryV1::Shop => Ok(OracleRunWorkKindV1::ShopAction),
        OracleRunBoundaryV1::Campfire => Ok(OracleRunWorkKindV1::CampfireAction),
        OracleRunBoundaryV1::RunChoice => Ok(OracleRunWorkKindV1::RunChoice),
        OracleRunBoundaryV1::Treasure => Ok(OracleRunWorkKindV1::TreasureAction),
        OracleRunBoundaryV1::BossRelic => Ok(OracleRunWorkKindV1::BossRelicChoice),
        unsupported => Err(format!(
            "oracle boundary {unsupported:?} does not own a noncombat action surface"
        )),
    }
}

fn should_normalize_navigation_away(
    session: &RunControlSession,
    action: &RunDecisionAction,
) -> bool {
    if !matches!(action, RunDecisionAction::Input(ClientInput::Cancel)) {
        return false;
    }
    matches!(
        session.engine_state,
        EngineState::RewardScreen(ref reward) if reward.pending_card_choice.is_some()
    ) || matches!(
        session.engine_state,
        EngineState::RewardOverlay {
            ref reward_state,
            ..
        } if reward_state.pending_card_choice.is_some()
    )
}

const ORACLE_RUN_STATE_FINGERPRINT_ALGORITHM_V1: &str = "blake2b_256_canonical_json_value_v1";
const ORACLE_RUN_STATE_FINGERPRINT_ALGORITHM: &str = "blake2b_256_canonical_run_checkpoint_v2";

pub(super) fn run_session_fingerprint_v2(session: &RunControlSession) -> String {
    let mut normalized = session.clone();
    normalized.decision_step = 0;
    normalized.run_state.emitted_events.clear();
    normalized.combat_sequence = 0;
    normalized.auto_capture_last_combat_sequence = None;
    let mut checkpoint = RunControlSessionCheckpointV1::from_session(&normalized);
    checkpoint.clear_combat_diagnostics_for_external_checkpoint();
    canonical_oracle_hash(&checkpoint)
}

fn canonical_oracle_hash<T: Serialize>(value: &T) -> String {
    let canonical_value = serde_json::to_value(value)
        .expect("oracle run checkpoint should serialize into canonical JSON value");
    crate::eval::fingerprint::hash_serializable(&canonical_value)
}

pub(super) fn classify_run_boundary(session: &RunControlSession) -> OracleRunBoundaryV1 {
    if session.active_combat.is_some() {
        return OracleRunBoundaryV1::Combat;
    }
    match session.engine_state {
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => {
            OracleRunBoundaryV1::MapDecision
        }
        EngineState::CombatStart(_)
        | EngineState::CombatProcessing
        | EngineState::CombatPlayerTurn
        | EngineState::PendingChoice(_) => OracleRunBoundaryV1::Combat,
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => {
            OracleRunBoundaryV1::Reward
        }
        EngineState::EventRoom => OracleRunBoundaryV1::Event,
        EngineState::Shop(_) => OracleRunBoundaryV1::Shop,
        EngineState::Campfire => OracleRunBoundaryV1::Campfire,
        EngineState::RunPendingChoice(_) => OracleRunBoundaryV1::RunChoice,
        EngineState::TreasureRoom(_) => OracleRunBoundaryV1::Treasure,
        EngineState::BossRelicSelect(_) => OracleRunBoundaryV1::BossRelic,
        EngineState::GameOver(RunResult::Victory) => OracleRunBoundaryV1::TerminalVictory,
        EngineState::GameOver(RunResult::Defeat) => OracleRunBoundaryV1::TerminalDefeat,
    }
}

#[cfg(test)]
mod tests;
