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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunBranchCheckpointV1 {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal: Option<RunProgressJournalV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_tip: Option<usize>,
    pub session: RunControlSessionCheckpointV1,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunJournalNodeCheckpointV1 {
    pub parent: Option<usize>,
    pub entry: super::RunProgressStepV1,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunActiveCombatCheckpointV1 {
    pub branch_id: usize,
    #[serde(default)]
    pub stage: u8,
    pub work: OracleRunCombatWorkCheckpointV1,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunDeferredCombatCheckpointV1 {
    pub branch_id: usize,
    pub stage: u8,
    pub prior_work: OracleRunCombatWorkCheckpointV1,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleRunExplorerCheckpointV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_fingerprint_algorithm: Option<String>,
    pub next_branch_id: usize,
    pub branches: Vec<OracleRunBranchCheckpointV1>,
    pub pending_decisions: Vec<LazyOracleRunDecisionV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_selection_families: Vec<LazyOracleRunSelectionFamilyV1>,
    /// Legacy checkpoints only recorded the exact combat state and therefore
    /// had to restart search with a fresh allowance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_combat_branch_id: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_combat: Option<OracleRunActiveCombatCheckpointV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deferred_combats: Vec<OracleRunDeferredCombatCheckpointV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub journal_nodes: Vec<OracleRunJournalNodeCheckpointV1>,
    #[serde(default)]
    pub combat_search_restarts: usize,
    /// The last top-level Neow option that received strategic service.
    /// Persisting this cursor prevents a wide option from regaining the first
    /// slot every time a continuation is resumed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_served_neow_root: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_combats: Vec<OracleRunUnresolvedCombatV1>,
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

    pub fn frontier_checkpoint(&self) -> Result<Option<OracleRunExplorerCheckpointV1>, String> {
        if self.pending_combats.len() > 1 {
            return Err(format!(
                "oracle frontier cannot checkpoint {} simultaneous combat edges",
                self.pending_combats.len()
            ));
        }
        let active_combat =
            self.pending_combats
                .front()
                .map(|pending| OracleRunActiveCombatCheckpointV1 {
                    branch_id: pending.branch_id,
                    stage: pending.stage,
                    work: pending.work.checkpoint(),
                });
        let active_combat_branch_id = active_combat.as_ref().map(|active| active.branch_id);
        let mut live_branch_ids = self
            .pending_decisions
            .iter()
            .map(|decision| decision.parent_branch_id)
            .collect::<BTreeSet<_>>();
        live_branch_ids.extend(
            self.pending_selection_families
                .iter()
                .map(|family| family.parent_branch_id),
        );
        if let Some(branch_id) = active_combat_branch_id {
            live_branch_ids.insert(branch_id);
        }
        live_branch_ids.extend(self.deferred_combats.iter().map(|combat| combat.branch_id));
        if live_branch_ids.is_empty() {
            return Ok(None);
        }
        live_branch_ids.extend(
            self.unresolved_combats
                .iter()
                .map(|combat| combat.branch_id),
        );
        self.checkpoint_for_branches(live_branch_ids, active_combat)
            .map(Some)
    }

    pub(super) fn analysis_checkpoint(&self) -> Result<OracleRunExplorerCheckpointV1, String> {
        if !self.pending_combats.is_empty() {
            return Err(
                "analysis checkpoint requires combat work to be owned by the analysis session"
                    .to_string(),
            );
        }
        let branch_ids = self
            .branches
            .iter()
            .map(|branch| branch.branch_id)
            .collect::<BTreeSet<_>>();
        self.checkpoint_for_branches(branch_ids, None)
    }

    fn checkpoint_for_branches(
        &self,
        branch_ids: BTreeSet<usize>,
        active_combat: Option<OracleRunActiveCombatCheckpointV1>,
    ) -> Result<OracleRunExplorerCheckpointV1, String> {
        let mut journal_nodes = Vec::<OracleRunJournalNodeCheckpointV1>::new();
        let mut journal_index = BTreeMap::<(Option<usize>, String), usize>::new();
        let branch_by_id = self
            .branches
            .iter()
            .map(|branch| (branch.branch_id, branch))
            .collect::<BTreeMap<_, _>>();
        let mut checkpointed_journals = BTreeMap::<usize, (Option<usize>, usize)>::new();
        let mut branches = Vec::with_capacity(branch_ids.len());
        for branch_id in branch_ids {
            let branch = branch_by_id
                .get(&branch_id)
                .copied()
                .ok_or_else(|| format!("missing live oracle branch {branch_id}"))?;
            let entries = branch.journal.entries();
            let (mut journal_tip, inherited_entries) = branch
                .parent_branch_id
                .and_then(|parent_id| {
                    let (parent_tip, parent_len) =
                        checkpointed_journals.get(&parent_id).copied()?;
                    let parent = branch_by_id.get(&parent_id).copied()?;
                    let parent_entries = parent.journal.entries();
                    (parent_entries.len() == parent_len
                        && entries.len() >= parent_len
                        && entries[..parent_len] == *parent_entries)
                        .then_some((parent_tip, parent_len))
                })
                .unwrap_or((None, 0));
            for entry in entries.iter().skip(inherited_entries) {
                let hash = crate::eval::fingerprint::hash_serializable(entry);
                let key = (journal_tip, hash);
                let node_id = if let Some(node_id) = journal_index.get(&key).copied() {
                    if journal_nodes[node_id].entry != *entry {
                        return Err("oracle journal fingerprint collision".to_string());
                    }
                    node_id
                } else {
                    let node_id = journal_nodes.len();
                    journal_nodes.push(OracleRunJournalNodeCheckpointV1 {
                        parent: journal_tip,
                        entry: entry.clone(),
                    });
                    journal_index.insert(key, node_id);
                    node_id
                };
                journal_tip = Some(node_id);
            }
            checkpointed_journals.insert(branch_id, (journal_tip, entries.len()));
            let mut session = RunControlSessionCheckpointV1::from_session(&branch.session);
            session.clear_combat_diagnostics_for_external_checkpoint();
            branches.push(OracleRunBranchCheckpointV1 {
                branch_id: branch.branch_id,
                parent_branch_id: branch.parent_branch_id,
                neow_root_candidate_id: branch.neow_root_candidate_id.clone(),
                neow_root_label: branch.neow_root_label.clone(),
                state_fingerprint: branch.state_fingerprint.clone(),
                boundary: branch.boundary,
                path_negative_log_policy: branch.path_negative_log_policy,
                path_discrepancy: branch.path_discrepancy,
                path_depth: branch.path_depth,
                replay: branch.replay.clone(),
                journal: None,
                journal_tip,
                session,
            });
        }
        Ok(OracleRunExplorerCheckpointV1 {
            state_fingerprint_algorithm: Some(ORACLE_RUN_STATE_FINGERPRINT_ALGORITHM.to_string()),
            next_branch_id: self.next_branch_id,
            branches,
            pending_decisions: self.pending_decisions.iter().cloned().collect(),
            pending_selection_families: self.pending_selection_families.iter().cloned().collect(),
            active_combat_branch_id: None,
            active_combat,
            deferred_combats: self
                .deferred_combats
                .iter()
                .map(|combat| OracleRunDeferredCombatCheckpointV1 {
                    branch_id: combat.branch_id,
                    stage: combat.stage,
                    prior_work: combat.prior_work.clone(),
                })
                .collect(),
            journal_nodes,
            combat_search_restarts: self.combat_search_restarts,
            last_served_neow_root: self.last_served_neow_root.clone(),
            unresolved_combats: self.unresolved_combats.clone(),
        })
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
        let successor_fingerprint = run_session_fingerprint_v1(&session);
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
            state_fingerprint: run_session_fingerprint_v1(&session),
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
            state_fingerprint: run_session_fingerprint_v1(&session),
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
        state_fingerprint: run_session_fingerprint_v1(&session),
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
    let legacy_state_fingerprints = state_fingerprint_algorithm.is_none();
    if let Some(algorithm) = state_fingerprint_algorithm.as_deref() {
        if algorithm != ORACLE_RUN_STATE_FINGERPRINT_ALGORITHM {
            return Err(format!(
                "unsupported oracle run state fingerprint algorithm '{algorithm}'"
            ));
        }
    }
    let mut explorer = OracleRunExplorerV1::empty();
    explorer.next_branch_id = next_branch_id;
    explorer.combat_search_restarts = combat_search_restarts;
    explorer.last_served_neow_root = last_served_neow_root;
    explorer.unresolved_combats = unresolved_combats;
    for saved in branches {
        let journal = restore_frontier_journal(saved.journal, saved.journal_tip, &journal_nodes)?;
        let session = saved.session.into_session()?;
        let actual_fingerprint = run_session_fingerprint_v1(&session);
        if !legacy_state_fingerprints && actual_fingerprint != saved.state_fingerprint {
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
        if !legacy_state_fingerprints
            && parent.state_fingerprint != decision.parent_state_fingerprint
        {
            return Err(format!(
                "oracle frontier decision parent fingerprint changed for branch {}",
                decision.parent_branch_id
            ));
        }
        if legacy_state_fingerprints {
            decision.parent_state_fingerprint = parent.state_fingerprint.clone();
            decision.stable_work_key = stable_oracle_work_key(
                &decision.parent_state_fingerprint,
                &decision.candidate_id,
                &decision.action,
            );
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
    for family in pending_selection_families {
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
        if !legacy_state_fingerprints && parent.state_fingerprint != family.parent_state_fingerprint
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

fn restore_frontier_journal(
    legacy_journal: Option<RunProgressJournalV1>,
    mut tip: Option<usize>,
    nodes: &[OracleRunJournalNodeCheckpointV1],
) -> Result<RunProgressJournalV1, String> {
    if let Some(journal) = legacy_journal {
        return Ok(journal);
    }
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    while let Some(node_id) = tip {
        if !seen.insert(node_id) {
            return Err("oracle frontier journal contains a cycle".to_string());
        }
        let node = nodes
            .get(node_id)
            .ok_or_else(|| format!("oracle frontier journal node {node_id} is missing"))?;
        entries.push(node.entry.clone());
        tip = node.parent;
    }
    entries.reverse();
    RunProgressJournalV1::from_committed_steps(entries)
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

    let family_key = crate::eval::fingerprint::hash_serializable(&(
        "oracle_run_selection_family_v1",
        branch.state_fingerprint.as_str(),
        candidate.id.as_str(),
        choice.min_choices,
        choice.max_choices,
    ));
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

const ORACLE_RUN_STATE_FINGERPRINT_ALGORITHM: &str = "blake2b_256_canonical_json_value_v1";

pub(super) fn run_session_fingerprint_v1(session: &RunControlSession) -> String {
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
mod tests {
    use super::*;
    use crate::content::potions::{Potion, PotionId};
    use crate::eval::run_control::{
        expand_oracle_neow_candidates_v1, CardRewardFunctionV1, CardRewardObligationDeltaV1,
        CardRewardObligationSourceV1, CardRewardOwnerProvenanceV1, RunCombatResolutionKindV1,
        RunControlConfig,
    };
    use crate::state::core::{ActiveCombat, CombatContext, RoomCombatContext};
    use crate::state::map::node::RoomType;

    fn test_branch(branch_id: usize, parent_branch_id: Option<usize>) -> OracleRunBranchV1 {
        OracleRunBranchV1 {
            branch_id,
            parent_branch_id,
            neow_root_candidate_id: "root".to_string(),
            neow_root_label: "root".to_string(),
            state_fingerprint: format!("state/{branch_id}"),
            boundary: OracleRunBoundaryV1::MapDecision,
            path_negative_log_policy: 0.0,
            path_discrepancy: 0,
            path_depth: 1,
            replay: Vec::new(),
            journal: RunProgressJournalV1::default(),
            session: RunControlSession::new(RunControlConfig::default()),
        }
    }

    fn test_decision(parent_branch_id: usize, candidate_id: &str) -> LazyOracleRunDecisionV1 {
        LazyOracleRunDecisionV1 {
            parent_branch_id,
            parent_state_fingerprint: format!("state/{parent_branch_id}"),
            neow_root_candidate_id: "root".to_string(),
            kind: OracleRunWorkKindV1::MapTravel,
            candidate_id: candidate_id.to_string(),
            label: candidate_id.to_string(),
            action: RunDecisionAction::Input(ClientInput::Proceed),
            stable_work_key: candidate_id.to_string(),
            path_negative_log_policy: 0.0,
            path_discrepancy: 0,
            path_depth: 2,
            parent_act: 0,
            parent_floor: 0,
            combat_edge_probe: None,
        }
    }

    #[test]
    fn parameterized_run_selection_releases_one_exact_member_at_a_time() {
        let mut branch = test_branch(0, None);
        branch.boundary = OracleRunBoundaryV1::RunChoice;
        branch.session.run_state.master_deck = (0..3)
            .map(|uuid| {
                crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::Strike, uuid)
            })
            .collect();
        branch.session.engine_state =
            EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
                min_choices: 2,
                max_choices: 2,
                reason: crate::state::core::RunPendingChoiceReason::PurgeNonBottled,
                source: crate::state::selection::DomainEventSource::Selection(
                    crate::state::core::RunPendingChoiceReason::PurgeNonBottled.into(),
                ),
                return_state: Box::new(EngineState::MapNavigation),
            });

        let mut explorer = OracleRunExplorerV1::empty();
        explorer.accept_branch(branch);
        explorer.register_decision_work(0, None).unwrap();

        assert_eq!(explorer.pending_decisions.len(), 1);
        assert_eq!(explorer.pending_selection_families.len(), 1);
        assert_eq!(
            explorer.pending_selection_families[0].cursor.total_count(),
            3
        );
        let mut emitted = Vec::new();
        for expected_rank in 0..3 {
            let work = explorer.take_best_decision().unwrap();
            assert_eq!(work.candidate_id, "select");
            assert_eq!(work.path_discrepancy, expected_rank);
            assert!(((-work.path_negative_log_policy).exp() - 1.0 / 3.0).abs() < 1.0e-9);
            emitted.push(work.stable_work_key.clone());
            explorer
                .release_next_selection_member(&work.stable_work_key)
                .unwrap();
            assert!(
                explorer.pending_decisions.len() <= 1,
                "the frontier must never contain the whole combination family"
            );
        }
        assert_eq!(emitted.into_iter().collect::<BTreeSet<_>>().len(), 3);
        assert!(explorer.pending_decisions.is_empty());
        assert!(explorer.pending_selection_families.is_empty());
    }

    #[test]
    fn analysis_selection_widens_without_mutating_parent_choices() {
        let mut branch = test_branch(0, None);
        branch.boundary = OracleRunBoundaryV1::RunChoice;
        branch.session.run_state.master_deck = (0..3)
            .map(|uuid| {
                crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::Strike, uuid)
            })
            .collect();
        branch.session.engine_state =
            EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
                min_choices: 2,
                max_choices: 2,
                reason: crate::state::core::RunPendingChoiceReason::PurgeNonBottled,
                source: crate::state::selection::DomainEventSource::Selection(
                    crate::state::core::RunPendingChoiceReason::PurgeNonBottled.into(),
                ),
                return_state: Box::new(EngineState::MapNavigation),
            });

        let mut explorer = OracleRunExplorerV1::empty();
        explorer.accept_branch(branch);
        explorer.register_decision_work(0, None).unwrap();
        let first = explorer.pending_decisions[0].stable_work_key.clone();

        explorer.note_explicit_decision_service(&first).unwrap();
        assert_eq!(
            explorer.pending_decisions.len(),
            2,
            "trying a variation must preserve the exact parent choice while exposing one sibling"
        );
        assert!(explorer
            .pending_decisions
            .iter()
            .any(|decision| decision.stable_work_key == first));

        explorer.note_explicit_decision_service(&first).unwrap();
        assert_eq!(
            explorer.pending_decisions.len(),
            2,
            "retrying an old member must not widen the family twice"
        );
    }

    #[test]
    fn parameterized_run_selection_cursor_survives_frontier_checkpoint() {
        let mut branch = test_branch(0, None);
        branch.boundary = OracleRunBoundaryV1::RunChoice;
        branch.session.run_state.master_deck = (0..20)
            .map(|uuid| {
                crate::runtime::combat::CombatCard::new(crate::content::cards::CardId::Strike, uuid)
            })
            .collect();
        branch.session.engine_state =
            EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
                min_choices: 3,
                max_choices: 3,
                reason: crate::state::core::RunPendingChoiceReason::TransformUpgraded,
                source: crate::state::selection::DomainEventSource::Relic(
                    crate::content::relics::RelicId::Astrolabe,
                ),
                return_state: Box::new(EngineState::MapNavigation),
            });
        branch.state_fingerprint = run_session_fingerprint_v1(&branch.session);

        let mut explorer = OracleRunExplorerV1::empty();
        explorer.accept_branch(branch);
        explorer.register_decision_work(0, None).unwrap();
        let checkpoint = explorer.frontier_checkpoint().unwrap().unwrap();

        assert_eq!(checkpoint.pending_decisions.len(), 1);
        assert_eq!(checkpoint.pending_selection_families.len(), 1);
        let encoded = serde_json::to_vec(&checkpoint).unwrap();
        assert!(
            encoded.len() < 50_000,
            "checkpoint must not contain all 1,140 Astrolabe combinations"
        );
        let decoded = serde_json::from_slice(&encoded).unwrap();
        let mut restored = seed_oracle_run_explorer_from_checkpoint_v1(
            decoded,
            &OracleRunCombatBudgetsV1::uniform(RunControlSearchCombatOptions::default()),
        )
        .unwrap();
        assert_eq!(restored.pending_decisions.len(), 1);
        assert_eq!(restored.pending_selection_families.len(), 1);

        let first = restored.take_best_decision().unwrap();
        restored
            .release_next_selection_member(&first.stable_work_key)
            .unwrap();
        assert_eq!(restored.pending_decisions.len(), 1);
        assert_eq!(
            restored.pending_selection_families[0]
                .cursor
                .emitted_count(),
            2
        );
    }

    #[test]
    fn explicit_smoke_bomb_escape_materializes_a_typed_run_successor() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = false;
        combat.entities.player.current_hp = 37;
        combat.entities.player.max_hp = 80;
        let mut jaw_worm =
            crate::test_support::test_monster(crate::content::monsters::EnemyId::JawWorm);
        let plan = crate::content::monsters::roll_monster_turn_plan(
            &mut combat.rng.ai_rng,
            &jaw_worm,
            combat.meta.ascension_level,
            99,
            std::slice::from_ref(&jaw_worm),
            &[],
        );
        jaw_worm.set_planned_move_id(plan.move_id);
        jaw_worm.set_planned_steps(plan.steps);
        jaw_worm.set_planned_visible_spec(plan.visible_spec);
        combat.entities.monsters = vec![jaw_worm];
        combat.entities.potions = vec![Some(Potion::new(PotionId::SmokeBomb, 41))];
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let mut explorer = OracleRunExplorerV1::empty();
        explorer.next_branch_id = 1;
        explorer
            .accept_branch(OracleRunBranchV1 {
                branch_id: 0,
                parent_branch_id: None,
                neow_root_candidate_id: "test_root".to_string(),
                neow_root_label: "test root".to_string(),
                state_fingerprint: run_session_fingerprint_v1(&session),
                boundary: OracleRunBoundaryV1::Combat,
                path_negative_log_policy: 0.0,
                path_discrepancy: 0,
                path_depth: 1,
                replay: Vec::new(),
                journal: RunProgressJournalV1::default(),
                session,
            })
            .expect("unique combat branch");

        let child_id = explorer
            .materialize_explicit_smoke_bomb_escape(0)
            .expect("exact escape")
            .expect("escape child");
        let child = explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == child_id)
            .expect("materialized child");

        assert_eq!(child.boundary, OracleRunBoundaryV1::Reward);
        assert_eq!(child.session.run_state.current_hp, 37);
        let [RunProgressStepV1::CombatResolution(resolution)] = child.journal.entries() else {
            panic!("escape should append exactly one typed combat resolution");
        };
        assert_eq!(resolution.kind, RunCombatResolutionKindV1::SmokeBombEscape);
    }

    fn shadow_key(enemy_hp_delta: i32, survival_margin: i32) -> StrategicProbeShadowOrderKeyV1 {
        StrategicProbeShadowOrderKeyV1 {
            terminal_win_seen: false,
            non_loss_endpoint_seen: true,
            living_enemy_delta: 0,
            total_enemy_hp_delta: enemy_hp_delta,
            survival_margin,
            pollution_avoidance: 0,
            depth_turns: 1,
        }
    }

    fn empty_combat_work_checkpoint() -> OracleRunCombatWorkCheckpointV1 {
        OracleRunCombatWorkCheckpointV1 {
            consumed_nodes: 10,
            remaining_nodes: 0,
            remaining_engine_steps: 0,
            remaining_wall_ms: Some(0),
            quantum_count: 1,
            restart_count: 0,
            incumbent_revision: 0,
            policy_witness_proposals: 0,
            policy_witness_proposal_rejections: 0,
            quanta_since_incumbent_improvement: 1,
            incumbent: None,
            advisor_nodes: 0,
            advisor_elapsed_ms: 0,
            advisor_complete: true,
            advisor_failure: None,
        }
    }

    #[test]
    fn staged_combat_budget_uses_a_cheap_first_pass_and_full_retry() {
        let options = RunControlSearchCombatOptions {
            max_nodes: Some(101),
            wall_ms: Some(101),
            ..RunControlSearchCombatOptions::default()
        };
        let budgets = OracleRunCombatBudgetsV1 {
            hallway: options.clone(),
            elite: options.clone(),
            boss: options,
            quality_policy: OracleRunCombatQualityPolicyV1::Configured,
            initial_divisor: 4,
            guidance_bundle: None,
        };
        let session = RunControlSession::new(RunControlConfig::default());

        let first = budgets.for_session_stage(&session, 0);
        assert_eq!(first.max_nodes, Some(26));
        assert_eq!(first.wall_ms, Some(26));
        let retry = budgets.for_session_stage(&session, 1);
        assert_eq!(retry.max_nodes, Some(101));
        assert_eq!(retry.wall_ms, Some(101));
    }

    #[test]
    fn strategic_quality_policy_derives_a_nonboss_target_from_exact_run_state() {
        let options = RunControlSearchCombatOptions {
            max_nodes: Some(101),
            wall_ms: Some(101),
            satisfaction: Some(
                crate::ai::combat_search_v2::CombatSearchV2Satisfaction::ZeroLossOrBudget,
            ),
            ..RunControlSearchCombatOptions::default()
        };
        let budgets = OracleRunCombatBudgetsV1 {
            hallway: options.clone(),
            elite: options.clone(),
            boss: options,
            quality_policy: OracleRunCombatQualityPolicyV1::StrategicRun,
            initial_divisor: 1,
            guidance_bundle: None,
        };
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.player.current_hp = 72;
        combat.entities.player.max_hp = 80;
        session.active_combat = Some(crate::state::core::ActiveCombat::new(
            crate::state::core::EngineState::CombatPlayerTurn,
            combat,
            crate::state::core::CombatContext::Room(crate::state::core::RoomCombatContext {
                room_type: crate::state::map::node::RoomType::MonsterRoom,
            }),
        ));

        let resolved = budgets.for_session(&session);

        assert_eq!(
            resolved.satisfaction,
            Some(crate::ai::combat_search_v2::CombatSearchV2Satisfaction::HpLossAtMost(16))
        );

        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.meta.master_deck_snapshot = vec![crate::runtime::combat::CombatCard::new(
            crate::content::cards::CardId::Reaper,
            92,
        )]
        .into();
        assert_eq!(
            budgets.for_session(&session).satisfaction,
            Some(crate::ai::combat_search_v2::CombatSearchV2Satisfaction::HpLossAtMost(0)),
            "an already-damaged run with combat healing should search for a net-zero line"
        );
    }

    #[test]
    fn strategic_nonboss_search_conserves_potions_before_exact_rescue() {
        let options = RunControlSearchCombatOptions {
            max_nodes: Some(101),
            wall_ms: Some(101),
            ..RunControlSearchCombatOptions::default()
        };
        let budgets = OracleRunCombatBudgetsV1 {
            hallway: options.clone(),
            elite: options.clone(),
            boss: options,
            quality_policy: OracleRunCombatQualityPolicyV1::StrategicRun,
            initial_divisor: 1,
            guidance_bundle: None,
        };
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.potions = vec![Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::BlockPotion,
            7,
        ))];
        session.active_combat = Some(crate::state::core::ActiveCombat::new(
            crate::state::core::EngineState::CombatPlayerTurn,
            combat,
            crate::state::core::CombatContext::Room(crate::state::core::RoomCombatContext {
                room_type: crate::state::map::node::RoomType::MonsterRoom,
            }),
        ));

        let primary = budgets.for_session_stage(&session, 0);
        assert_eq!(primary.max_potions_used, Some(0));
        assert_eq!(
            primary.potion_policy,
            Some(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never)
        );
        assert!(budgets.has_later_stage(&session, 0));

        let rescue = budgets.for_session_stage(&session, 1);
        assert_ne!(rescue.max_potions_used, Some(0));
        assert_ne!(
            rescue.potion_policy,
            Some(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never)
        );
        assert!(!budgets.has_later_stage(&session, 1));
    }

    #[test]
    fn strategic_quality_stops_when_an_a0_intermediate_boss_materializes_payoff() {
        let options = RunControlSearchCombatOptions {
            satisfaction: Some(
                crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin,
            ),
            ..RunControlSearchCombatOptions::default()
        };
        let budgets = OracleRunCombatBudgetsV1 {
            hallway: options.clone(),
            elite: options.clone(),
            boss: options,
            quality_policy: OracleRunCombatQualityPolicyV1::StrategicRun,
            initial_divisor: 1,
            guidance_bundle: None,
        };
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = true;
        combat.meta.master_deck_snapshot = vec![crate::runtime::combat::CombatCard::new(
            crate::content::cards::CardId::HandOfGreed,
            91,
        )]
        .into();
        session.active_combat = Some(ActiveCombat::new(
            crate::state::core::EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomBoss,
            }),
        ));

        assert_eq!(
            budgets.for_session(&session).satisfaction,
            Some(
                crate::ai::combat_search_v2::CombatSearchV2Satisfaction::PersistentRunValueGain
            ),
            "an A0 act heal removes combat HP pressure, but persistent payoff remains a finite exact target"
        );

        session.run_state.act_num = 3;
        assert_eq!(
            budgets.for_session(&session).satisfaction,
            Some(crate::ai::combat_search_v2::CombatSearchV2Satisfaction::FirstCompleteWin),
            "persistent payoff must not delay the requested terminal boss witness"
        );
    }

    #[test]
    fn deferred_retry_stays_on_the_existing_deep_first_discrepancy_contour() {
        let mut explorer = OracleRunExplorerV1::empty();
        let mut branch = test_branch(0, None);
        branch.boundary = OracleRunBoundaryV1::Combat;
        branch.path_depth = 10;
        explorer.branches.push(branch);
        explorer.deferred_combats.push_back(DeferredOracleCombatV1 {
            branch_id: 0,
            stage: 1,
            prior_work: empty_combat_work_checkpoint(),
        });
        let mut shallow_same_contour = test_decision(0, "shallow-same-contour-decision");
        shallow_same_contour.path_discrepancy = 0;
        shallow_same_contour.path_depth = 2;
        shallow_same_contour.parent_act = explorer.branches[0].session.run_state.act_num;
        shallow_same_contour.parent_floor = explorer.branches[0].session.run_state.floor_num;
        explorer.pending_decisions.push_back(shallow_same_contour);

        assert!(matches!(
            explorer.take_next_scheduled_work(),
            Some(ScheduledOracleRunWorkV1::DeferredCombat(_))
        ));

        explorer.deferred_combats.push_back(DeferredOracleCombatV1 {
            branch_id: 0,
            stage: 1,
            prior_work: empty_combat_work_checkpoint(),
        });
        let mut deeper_same_contour = test_decision(0, "deeper-same-contour-decision");
        deeper_same_contour.path_discrepancy = 0;
        deeper_same_contour.path_depth = 20;
        deeper_same_contour.parent_act = explorer.branches[0].session.run_state.act_num;
        deeper_same_contour.parent_floor = explorer.branches[0].session.run_state.floor_num;
        explorer.pending_decisions.push_back(deeper_same_contour);
        assert!(matches!(
            explorer.take_next_scheduled_work(),
            Some(ScheduledOracleRunWorkV1::Decision(_))
        ));
    }

    #[test]
    fn deferred_combat_survives_frontier_checkpoint_without_a_live_search() {
        let mut explorer = OracleRunExplorerV1::empty();
        let mut branch = test_branch(0, None);
        branch.boundary = OracleRunBoundaryV1::Combat;
        explorer.branches.push(branch);
        explorer.next_branch_id = 1;
        explorer.deferred_combats.push_back(DeferredOracleCombatV1 {
            branch_id: 0,
            stage: 1,
            prior_work: empty_combat_work_checkpoint(),
        });

        let checkpoint = explorer
            .frontier_checkpoint()
            .expect("checkpoint")
            .expect("deferred combat is live work");
        assert_eq!(checkpoint.branches.len(), 1);
        assert_eq!(checkpoint.deferred_combats.len(), 1);
        assert_eq!(checkpoint.deferred_combats[0].branch_id, 0);
        assert_eq!(checkpoint.deferred_combats[0].stage, 1);
    }

    #[test]
    fn unresolved_combat_evidence_survives_a_live_frontier_checkpoint() {
        let mut explorer = OracleRunExplorerV1::empty();
        let mut unresolved_branch = test_branch(0, None);
        unresolved_branch.boundary = OracleRunBoundaryV1::Combat;
        let live_branch = test_branch(1, None);
        explorer.branches = vec![unresolved_branch, live_branch];
        explorer.next_branch_id = 2;
        explorer
            .pending_decisions
            .push_back(test_decision(1, "live-work"));
        explorer
            .unresolved_combats
            .push(OracleRunUnresolvedCombatV1 {
                branch_id: 0,
                rejection: RunControlCombatSearchRejection::NoCompleteWinningCandidate,
                evidence_kind: "budget_unknown".to_string(),
                last_status: Some("partial".to_string()),
                nodes_expanded: 10,
                exact_states: 9,
                applied_action_transitions: 8,
                unique_successor_states: 7,
                duplicate_exact_successors: 6,
                completed_turn_options: 5,
                retained_state_work: 4,
                max_player_turn: 3,
                max_path_atomic_depth: 2,
                generation_gap_count: 1,
                incumbent_final_hp: None,
            });

        let checkpoint = explorer
            .frontier_checkpoint()
            .expect("checkpoint")
            .expect("live decision keeps a continuation");
        assert!(checkpoint
            .branches
            .iter()
            .any(|branch| branch.branch_id == 0));
        assert_eq!(checkpoint.unresolved_combats.len(), 1);
        let encoded = serde_json::to_string(&checkpoint).expect("serialize checkpoint");
        let decoded: OracleRunExplorerCheckpointV1 =
            serde_json::from_str(&encoded).expect("deserialize checkpoint");
        assert_eq!(decoded.unresolved_combats.len(), 1);
        assert_eq!(
            decoded.unresolved_combats[0].evidence_kind,
            "budget_unknown"
        );
    }

    fn test_owner_annotation(
        _session: &RunControlSession,
        _candidate_id: &str,
    ) -> Option<RunControlTraceAnnotationV1> {
        Some(RunControlTraceAnnotationV1::CardRewardOwnerDecision {
            provenance: CardRewardOwnerProvenanceV1 {
                functions: vec![CardRewardFunctionV1::Access],
                obligations: vec![CardRewardObligationDeltaV1 {
                    source: CardRewardObligationSourceV1::KnownBoss,
                    subject: "test_boss".to_string(),
                    deadline_nodes: Some(16),
                    gaps_before: 1,
                    gaps_after: 1,
                }],
                strengthened_capabilities: Vec::new(),
                hard_startup_liability: false,
                hard_duplicate_liability: false,
                component_debt_count: 0,
                access_saturated: false,
                stable_surface_index: 0,
                owner_rank: 1,
                tie_break_applied: false,
            },
        })
    }

    #[test]
    fn oracle_settles_empty_campfire_as_typed_forced_progress() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;
        session
            .run_state
            .relics
            .push(crate::content::relics::RelicState::new(
                crate::content::relics::RelicId::CoffeeDripper,
            ));
        session
            .run_state
            .relics
            .push(crate::content::relics::RelicState::new(
                crate::content::relics::RelicId::FusionHammer,
            ));

        let steps = settle_oracle_forced_transitions(&mut session)
            .expect("an optionless campfire should settle without owner input");

        assert!(matches!(session.engine_state, EngineState::MapNavigation));
        assert!(matches!(
            steps.as_slice(),
            [RunProgressStepV1::ForcedTransition(transition)]
                if transition.kind
                    == crate::eval::run_control::RunForcedTransitionKindV1::EmptyCampfireExit
        ));
    }

    #[test]
    fn materialized_oracle_decision_commits_owner_provenance_to_the_journal() {
        let session = RunControlSession::new(RunControlConfig::default());
        let surface = build_decision_surface(&session);
        let candidate = surface
            .view
            .candidates
            .iter()
            .find_map(|candidate| {
                candidate
                    .action
                    .executable_input()
                    .map(|input| (candidate, input.clone()))
            })
            .expect("one executable initial candidate");
        let fingerprint = run_session_fingerprint_v1(&session);
        let parent = OracleRunBranchV1 {
            branch_id: 0,
            parent_branch_id: None,
            neow_root_candidate_id: "root".to_string(),
            neow_root_label: "root".to_string(),
            state_fingerprint: fingerprint.clone(),
            boundary: classify_run_boundary(&session),
            path_negative_log_policy: 0.0,
            path_discrepancy: 0,
            path_depth: 0,
            replay: Vec::new(),
            journal: RunProgressJournalV1::default(),
            session,
        };
        let work = LazyOracleRunDecisionV1 {
            parent_branch_id: 0,
            parent_state_fingerprint: fingerprint,
            neow_root_candidate_id: "root".to_string(),
            kind: OracleRunWorkKindV1::EventOption,
            candidate_id: candidate.0.id.clone(),
            label: candidate.0.label.clone(),
            action: RunDecisionAction::Input(candidate.1),
            stable_work_key: "test-owner-provenance".to_string(),
            path_negative_log_policy: 0.0,
            path_discrepancy: 0,
            path_depth: 1,
            parent_act: parent.session.run_state.act_num,
            parent_floor: parent.session.run_state.floor_num,
            combat_edge_probe: None,
        };
        let mut explorer = OracleRunExplorerV1::empty();
        explorer.branches.push(parent);
        explorer.next_branch_id = 1;

        let child_id = explorer
            .materialize_decision(work, Some(test_owner_annotation))
            .expect("materialize decision")
            .expect("unique child");
        let transaction = explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == child_id)
            .and_then(|branch| branch.journal.entries().first())
            .and_then(RunProgressStepV1::as_decision)
            .expect("journaled decision transaction");
        assert!(matches!(
            transaction.trace_annotations.as_slice(),
            [RunControlTraceAnnotationV1::CardRewardOwnerDecision { .. }]
        ));
    }

    #[test]
    fn heuristic_probe_only_orders_two_immediate_combat_edges() {
        let mut explorer = OracleRunExplorerV1::empty();
        let mut owner_first = test_decision(0, "owner-first");
        owner_first.path_discrepancy = 0;
        owner_first.combat_edge_probe = Some(OracleRunCombatEdgeProbeV1::HeuristicEstimate {
            order_key: shadow_key(5, 5),
        });
        let mut owner_second = test_decision(0, "owner-second");
        owner_second.path_discrepancy = 1;
        owner_second.combat_edge_probe = Some(OracleRunCombatEdgeProbeV1::HeuristicEstimate {
            order_key: shadow_key(20, 40),
        });
        explorer.pending_decisions.push_back(owner_first);
        explorer.pending_decisions.push_back(owner_second);

        let selected = explorer.take_best_decision().expect("one edge selected");
        assert_eq!(selected.candidate_id, "owner-second");
        assert_eq!(selected.path_discrepancy, 1);
        assert_eq!(explorer.pending_decisions.len(), 1);
    }

    #[test]
    fn edge_probe_never_promotes_a_noncombat_decision_over_owner_order() {
        let mut explorer = OracleRunExplorerV1::empty();
        let owner_first = test_decision(0, "owner-first");
        let mut unrelated_noncombat = test_decision(0, "noncombat");
        unrelated_noncombat.path_discrepancy = 1;
        unrelated_noncombat.combat_edge_probe =
            Some(OracleRunCombatEdgeProbeV1::NotImmediateCombat);
        explorer.pending_decisions.push_back(owner_first);
        explorer.pending_decisions.push_back(unrelated_noncombat);

        let selected = explorer
            .take_best_decision()
            .expect("one decision selected");
        assert_eq!(selected.candidate_id, "owner-first");
    }

    #[test]
    fn seed006_registers_all_completed_neow_roots_without_selecting_one() {
        let session = RunControlSession::new(RunControlConfig {
            seed: 6,
            ascension_level: 0,
            ..RunControlConfig::default()
        });
        let expansion = expand_oracle_neow_candidates_v1(&session).expect("Neow expansion");
        let completed = expansion.completed.len();
        let explorer = seed_oracle_run_explorer_v1(expansion, None).expect("oracle run seed");
        assert_eq!(explorer.branches.len(), completed);
        assert!(!explorer.pending_decisions.is_empty());
        assert_eq!(explorer.pending_combat_count(), 0);
    }

    #[test]
    fn changing_act_number_does_not_create_an_artificial_act_completion_boundary() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.engine_state = EngineState::MapNavigation;
        assert_eq!(
            classify_run_boundary(&session),
            OracleRunBoundaryV1::MapDecision
        );
    }

    #[test]
    fn canonical_oracle_hash_ignores_hash_map_insertion_order() {
        let mut left = std::collections::HashMap::new();
        left.insert("z", 1);
        left.insert("a", 2);
        let mut right = std::collections::HashMap::new();
        right.insert("a", 2);
        right.insert("z", 1);

        assert_eq!(canonical_oracle_hash(&left), canonical_oracle_hash(&right));
    }

    #[test]
    fn decision_policy_prefers_owner_order_and_keeps_every_fallback_positive() {
        fn prefer_second(
            _: &RunControlSession,
            legal: &[RunPolicyCandidateV1<'_>],
        ) -> Result<crate::eval::run_control::RunPolicyPriorV1, String> {
            positive_ranked_run_policy_prior_v1(legal, ["second".to_string()])
        }

        let branch = test_branch(7, None);
        let mut work = vec![test_decision(7, "first"), test_decision(7, "second")];
        apply_decision_policy(&branch, &mut work, Some(prefer_second))
            .expect("valid complete policy prior");

        let first = work
            .iter()
            .find(|work| work.candidate_id == "first")
            .unwrap();
        let second = work
            .iter()
            .find(|work| work.candidate_id == "second")
            .unwrap();
        assert!(second.path_negative_log_policy < first.path_negative_log_policy);
        assert!(first.path_negative_log_policy.is_finite());
        assert_eq!(second.path_discrepancy, 0);
        assert_eq!(first.path_discrepancy, 1);
        assert_eq!(first.path_depth, 2);
        assert_eq!(second.path_depth, 2);
    }

    #[test]
    fn zero_discrepancy_mainline_continues_before_a_shallower_sibling() {
        let mut explorer = OracleRunExplorerV1::empty();
        let early_branch = test_branch(0, None);
        let mut deep_branch = test_branch(9, Some(0));
        deep_branch.session.run_state.floor_num = 10;
        explorer.branches = vec![early_branch, deep_branch];
        let mut deep = test_decision(9, "deep-policy-head");
        deep.path_depth = 20;
        deep.path_negative_log_policy = 8.0;
        let mut early = test_decision(0, "early-alternative");
        early.path_depth = 3;
        early.path_negative_log_policy = 2.0;
        explorer.pending_decisions = VecDeque::from([deep, early]);

        let selected = explorer.take_best_decision().expect("global work");
        assert_eq!(selected.candidate_id, "deep-policy-head");
        assert_eq!(explorer.pending_decisions.len(), 1);
    }

    #[test]
    fn exact_run_progress_precedes_journal_depth_within_one_discrepancy_contour() {
        let mut explorer = OracleRunExplorerV1::empty();
        let mut early_but_busy = test_decision(0, "act-1-many-actions");
        early_but_busy.path_discrepancy = 1;
        early_but_busy.parent_act = 1;
        early_but_busy.parent_floor = 7;
        early_but_busy.path_depth = 200;
        let mut later_run_state = test_decision(1, "act-2-boss");
        later_run_state.path_discrepancy = 1;
        later_run_state.parent_act = 2;
        later_run_state.parent_floor = 32;
        later_run_state.path_depth = 20;
        explorer.pending_decisions = VecDeque::from([early_but_busy, later_run_state]);

        let selected = explorer.take_best_decision().expect("global work");
        assert_eq!(selected.candidate_id, "act-2-boss");
    }

    #[test]
    fn another_root_mainline_precedes_a_deviation_from_the_first_root() {
        let mut explorer = OracleRunExplorerV1::empty();
        explorer.branches = vec![test_branch(0, None), test_branch(1, None)];
        let mut first_root_deviation = test_decision(0, "root-0-rank-1");
        first_root_deviation.path_discrepancy = 1;
        first_root_deviation.path_depth = 20;
        let mut second_root_mainline = test_decision(1, "root-1-rank-0");
        second_root_mainline.path_discrepancy = 0;
        second_root_mainline.path_depth = 2;
        explorer.pending_decisions = VecDeque::from([first_root_deviation, second_root_mainline]);

        let selected = explorer.take_best_decision().expect("strategic work");
        assert_eq!(selected.candidate_id, "root-1-rank-0");
    }

    #[test]
    fn a_wide_neow_root_cannot_monopolize_strategic_service() {
        let mut explorer = OracleRunExplorerV1::empty();
        let mut root_zero_first = test_decision(0, "root-0-first");
        root_zero_first.neow_root_candidate_id = "0".to_string();
        let mut root_zero_second = test_decision(1, "root-0-second");
        root_zero_second.neow_root_candidate_id = "0".to_string();
        root_zero_second.path_depth = 100;
        let mut root_one = test_decision(2, "root-1");
        root_one.neow_root_candidate_id = "1".to_string();
        root_one.path_discrepancy = 20;
        explorer.pending_decisions = VecDeque::from([root_zero_first, root_zero_second, root_one]);

        let first = explorer
            .take_next_scheduled_work()
            .expect("first root service");
        assert!(matches!(
            first,
            ScheduledOracleRunWorkV1::Decision(ref decision)
                if decision.neow_root_candidate_id == "0"
        ));
        let second = explorer
            .take_next_scheduled_work()
            .expect("second root service");
        assert!(matches!(
            second,
            ScheduledOracleRunWorkV1::Decision(ref decision)
                if decision.neow_root_candidate_id == "1"
        ));
        let third = explorer
            .take_next_scheduled_work()
            .expect("wrapped root service");
        assert!(matches!(
            third,
            ScheduledOracleRunWorkV1::Decision(ref decision)
                if decision.neow_root_candidate_id == "0"
        ));
    }

    #[test]
    fn a_single_combat_remains_exactly_resumable_across_quanta() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::planned_monster(
            crate::content::monsters::EnemyId::JawWorm,
            1,
        )];
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let mut explorer = OracleRunExplorerV1::empty();
        let branch = OracleRunBranchV1 {
            branch_id: 0,
            parent_branch_id: None,
            neow_root_candidate_id: "test_root".to_string(),
            neow_root_label: "test root".to_string(),
            state_fingerprint: run_session_fingerprint_v1(&session),
            boundary: OracleRunBoundaryV1::Combat,
            path_negative_log_policy: 0.0,
            path_discrepancy: 0,
            path_depth: 1,
            replay: Vec::new(),
            journal: RunProgressJournalV1::default(),
            session,
        };
        explorer.next_branch_id = 1;
        let branch_id = explorer.accept_branch(branch).expect("unique branch");
        let combat_budgets = OracleRunCombatBudgetsV1::uniform(RunControlSearchCombatOptions {
            max_nodes: Some(8),
            wall_ms: None,
            rollout_policy: Some(
                crate::ai::combat_search_v2::CombatSearchV2RolloutPolicy::Disabled,
            ),
            satisfaction: Some(
                crate::ai::combat_search_v2::CombatSearchV2Satisfaction::BudgetOrExhaustion,
            ),
            ..RunControlSearchCombatOptions::default()
        });
        explorer
            .schedule_branch(branch_id, &combat_budgets, None)
            .expect("combat work should schedule");
        let mut sibling = test_decision(0, "strategic-sibling");
        sibling.parent_state_fingerprint = explorer.branches[0].state_fingerprint.clone();
        explorer.pending_decisions.push_back(sibling);

        let result = drive_oracle_run_explorer_v1(
            explorer,
            OracleRunExploreBudgetV1 {
                max_work_items: 2,
                wall_ms: None,
                combat: combat_budgets.clone(),
                combat_quantum_nodes: 1,
                combat_quantum_ms: None,
                decision_prior: None,
                decision_annotation: None,
                combat_edge_order: None,
            },
        )
        .expect("one explorer quantum");

        assert_eq!(result.stop, OracleRunExploreStopV1::WorkBudgetExhausted);
        assert_eq!(result.combat_quanta, 2);
        assert_eq!(result.explorer.pending_combat_count(), 1);
        assert_eq!(result.explorer.pending_decisions.len(), 1);
        assert_eq!(
            result.explorer.pending_decisions[0].candidate_id,
            "strategic-sibling"
        );
        assert!(result.explorer.unresolved_combats.is_empty());
        let pending = result
            .explorer
            .pending_combat_summaries()
            .expect("pending combat summary");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].branch_id, 0);
        assert_eq!(pending[0].enemies.len(), 1);
        assert_eq!(pending[0].enemies[0].name, "Jaw Worm");
        assert_eq!(pending[0].quantum_count, 2);
        assert!(pending[0].last_quantum_generation_work <= 1);
        assert!(pending[0].exact_states >= 1);
        assert!(pending[0].retained_state_work >= 1);
        assert_eq!(pending[0].quanta_since_incumbent_improvement, 2);
        assert_eq!(pending[0].incumbent_revision, 0);
        assert_eq!(
            pending[0].resume_kind,
            OracleCombatSearchResumeKindV1::Fresh
        );
        assert_eq!(pending[0].restart_count, 0);
        let consumed_before_restart = pending[0].nodes_expanded;
        let remaining_before_restart = pending[0].remaining_nodes;

        let checkpoint = result
            .explorer
            .frontier_checkpoint()
            .expect("frontier checkpoint")
            .expect("live frontier");
        let encoded = serde_json::to_vec(&checkpoint).expect("serialize frontier");
        let decoded: OracleRunExplorerCheckpointV1 =
            serde_json::from_slice(&encoded).expect("deserialize frontier");
        assert!(decoded.active_combat_branch_id.is_none());
        assert!(decoded.active_combat.is_some());
        let restored = seed_oracle_run_explorer_from_checkpoint_v1(decoded, &combat_budgets)
            .expect("restore frontier");

        assert_eq!(restored.pending_combat_count(), 1);
        assert_eq!(restored.combat_search_restarts, 1);
        assert_eq!(restored.pending_decisions.len(), 1);
        assert_eq!(
            restored.pending_decisions[0].candidate_id,
            "strategic-sibling"
        );
        let restored_pending = restored
            .pending_combat_summaries()
            .expect("restored pending combat summary");
        assert_eq!(restored_pending[0].nodes_expanded, consumed_before_restart);
        assert_eq!(
            restored_pending[0].remaining_nodes,
            remaining_before_restart
        );
        assert_eq!(restored_pending[0].quantum_count, 2);
        assert_eq!(
            restored_pending[0].incumbent_revision,
            pending[0].incumbent_revision
        );
        assert_eq!(
            restored_pending[0].quanta_since_incumbent_improvement,
            pending[0].quanta_since_incumbent_improvement
        );
        assert_eq!(restored_pending[0].restart_count, 1);
        assert_eq!(
            restored_pending[0].resume_kind,
            OracleCombatSearchResumeKindV1::StateReplayExactSearchRestarted
        );
    }
}
