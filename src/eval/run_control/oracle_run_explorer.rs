use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::state::core::{ClientInput, EngineState, RunResult};
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};

use super::{
    build_decision_surface, DecisionCandidateKey, NeowOracleExpansionV1,
    RunControlCombatSearchQuantum, RunControlCombatSearchRejection, RunControlCombatWorkAdvanceV1,
    RunControlCombatWorkV1, RunControlSearchCombatOptions, RunControlSession,
    RunControlSessionCheckpointV1, RunDecisionAction, RunProgressJournalV1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct OracleRunReplayStepV1 {
    pub candidate_id: String,
    pub label: String,
    pub action: RunDecisionAction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LazyOracleRunDecisionV1 {
    pub parent_branch_id: usize,
    pub parent_state_fingerprint: String,
    pub neow_root_candidate_id: String,
    pub kind: OracleRunWorkKindV1,
    pub candidate_id: String,
    pub label: String,
    pub action: RunDecisionAction,
    pub stable_work_key: String,
}

#[derive(Clone, Debug)]
pub struct OracleRunBranchV1 {
    pub branch_id: usize,
    pub parent_branch_id: Option<usize>,
    pub neow_root_candidate_id: String,
    pub neow_root_label: String,
    pub state_fingerprint: String,
    pub boundary: OracleRunBoundaryV1,
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

#[derive(Clone, Debug)]
pub struct OracleRunUnresolvedCombatV1 {
    pub branch_id: usize,
    pub rejection: RunControlCombatSearchRejection,
    pub nodes_expanded: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct OraclePendingCombatEnemyV1 {
    pub monster_type: usize,
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
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
    pub quantum_count: usize,
    pub remaining_nodes: usize,
    pub remaining_wall_ms: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct OracleRunCombatBudgetsV1 {
    pub hallway: RunControlSearchCombatOptions,
    pub elite: RunControlSearchCombatOptions,
    pub boss: RunControlSearchCombatOptions,
}

impl OracleRunCombatBudgetsV1 {
    pub fn uniform(options: RunControlSearchCombatOptions) -> Self {
        Self {
            hallway: options.clone(),
            elite: options.clone(),
            boss: options,
        }
    }

    fn for_session(&self, session: &RunControlSession) -> RunControlSearchCombatOptions {
        let Some(active) = session.active_combat.as_ref() else {
            return self.hallway.clone();
        };
        if active.combat_state.meta.is_boss_fight {
            self.boss.clone()
        } else if active.combat_state.meta.is_elite_fight {
            self.elite.clone()
        } else {
            self.hallway.clone()
        }
    }
}

#[derive(Clone, Debug)]
pub struct OracleRunExploreBudgetV1 {
    pub max_work_items: usize,
    pub wall_ms: Option<u64>,
    pub combat: OracleRunCombatBudgetsV1,
    pub combat_quantum_nodes: usize,
    pub combat_quantum_ms: Option<u64>,
    pub decision_order: Option<OracleRunDecisionOrderFnV1>,
}

pub type OracleRunDecisionOrderFnV1 = fn(&RunControlSession) -> Vec<String>;

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
    work: RunControlCombatWorkV1,
}

enum FinishedOracleCombatV1 {
    Resolved(usize),
    ExactDuplicate,
    Unresolved { branch_id: usize },
}

fn should_rotate_after_finished_combat(
    advance: RunControlCombatWorkAdvanceV1,
    finished: &FinishedOracleCombatV1,
) -> bool {
    advance == RunControlCombatWorkAdvanceV1::AllowanceExhausted
        && matches!(finished, FinishedOracleCombatV1::Unresolved { .. })
}

pub struct OracleRunExplorerV1 {
    pub branches: Vec<OracleRunBranchV1>,
    pub pending_decisions: VecDeque<LazyOracleRunDecisionV1>,
    pub retired_exact_duplicates: Vec<ExactDuplicateOracleRunBranchV1>,
    pub unresolved_combats: Vec<OracleRunUnresolvedCombatV1>,
    pending_combats: VecDeque<PendingOracleCombatV1>,
    active_branch_id: Option<usize>,
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
            pending_combats: VecDeque::new(),
            active_branch_id: None,
            next_branch_id: 0,
            state_index: BTreeMap::new(),
            registered_work_keys: BTreeSet::new(),
        }
    }

    pub fn pending_combat_count(&self) -> usize {
        self.pending_combats.len()
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
                Ok(OraclePendingCombatSummaryV1 {
                    branch_id: branch.branch_id,
                    act: branch.session.run_state.act_num,
                    floor: branch.session.run_state.floor_num,
                    player_hp: branch.session.run_state.current_hp,
                    player_max_hp: branch.session.run_state.max_hp,
                    elite: active.combat_state.meta.is_elite_fight,
                    boss: active.combat_state.meta.is_boss_fight,
                    enemies,
                    nodes_expanded: pending.work.nodes_expanded(),
                    quantum_count: pending.work.quantum_count(),
                    remaining_nodes: pending.work.remaining_nodes(),
                    remaining_wall_ms: pending.work.remaining_wall_ms(),
                })
            })
            .collect()
    }

    fn next_branch_with_work(&self) -> Option<usize> {
        self.pending_decisions
            .front()
            .map(|work| work.parent_branch_id)
            .or_else(|| self.pending_combats.front().map(|work| work.branch_id))
    }

    fn take_decision_for_branch(
        &mut self,
        branch_id: usize,
        decision_order: Option<OracleRunDecisionOrderFnV1>,
    ) -> Option<LazyOracleRunDecisionV1> {
        let preferred_ids = decision_order
            .and_then(|order| {
                self.branches
                    .iter()
                    .find(|branch| branch.branch_id == branch_id)
                    .map(|branch| order(&branch.session))
            })
            .unwrap_or_default();
        let index = preferred_ids
            .iter()
            .find_map(|candidate_id| {
                self.pending_decisions.iter().position(|work| {
                    work.parent_branch_id == branch_id && work.candidate_id == *candidate_id
                })
            })
            .or_else(|| {
                self.pending_decisions
                    .iter()
                    .position(|work| work.parent_branch_id == branch_id)
            })?;
        self.pending_decisions.remove(index)
    }

    fn take_combat_for_branch(&mut self, branch_id: usize) -> Option<PendingOracleCombatV1> {
        let index = self
            .pending_combats
            .iter()
            .position(|work| work.branch_id == branch_id)?;
        self.pending_combats.remove(index)
    }

    /// An exhausted combat should not make every nearby sibling monopolize the
    /// remaining run budget. Keep all work, but defer the closest ancestor's
    /// sibling group behind the other already-registered decision work.
    fn rotate_nearest_ancestor_decisions_after_unresolved_combat(
        &mut self,
        combat_branch_id: usize,
    ) -> Option<usize> {
        let mut cursor = Some(combat_branch_id);
        while let Some(branch_id) = cursor {
            if self
                .pending_decisions
                .iter()
                .any(|work| work.parent_branch_id == branch_id)
            {
                let mut retained = VecDeque::with_capacity(self.pending_decisions.len());
                let mut deferred = VecDeque::new();
                while let Some(work) = self.pending_decisions.pop_front() {
                    if work.parent_branch_id == branch_id {
                        deferred.push_back(work);
                    } else {
                        retained.push_back(work);
                    }
                }
                retained.append(&mut deferred);
                self.pending_decisions = retained;
                return Some(branch_id);
            }
            cursor = self
                .branches
                .iter()
                .find(|branch| branch.branch_id == branch_id)
                .and_then(|branch| branch.parent_branch_id);
        }
        None
    }

    fn accept_branch(&mut self, branch: OracleRunBranchV1) -> Option<usize> {
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
        prefer_next: bool,
    ) -> Result<(), String> {
        let branch = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
            .ok_or_else(|| format!("missing oracle run branch {branch_id}"))?;
        let mut work = decision_work_for_branch(branch)?;
        work.retain(|item| {
            self.registered_work_keys
                .insert(item.stable_work_key.clone())
        });
        if prefer_next {
            for item in work.into_iter().rev() {
                self.pending_decisions.push_front(item);
            }
        } else {
            self.pending_decisions.extend(work);
        }
        Ok(())
    }

    fn schedule_branch(
        &mut self,
        branch_id: usize,
        combat_budgets: &OracleRunCombatBudgetsV1,
        prefer_next: bool,
    ) -> Result<(), String> {
        let branch = self
            .branches
            .iter()
            .find(|branch| branch.branch_id == branch_id)
            .ok_or_else(|| format!("missing oracle run branch {branch_id}"))?;
        match branch.boundary {
            OracleRunBoundaryV1::Combat => {
                let key = format!("combat:{}", branch.state_fingerprint);
                if !self.registered_work_keys.insert(key) {
                    return Ok(());
                }
                let work = RunControlCombatWorkV1::new(
                    &branch.session,
                    combat_budgets.for_session(&branch.session),
                )?;
                self.pending_combats
                    .push_back(PendingOracleCombatV1 { branch_id, work });
                Ok(())
            }
            OracleRunBoundaryV1::TerminalVictory | OracleRunBoundaryV1::TerminalDefeat => Ok(()),
            _ => self.register_decision_work(branch_id, prefer_next),
        }
    }

    fn materialize_decision(
        &mut self,
        work: LazyOracleRunDecisionV1,
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

        let mut session = parent.session.clone();
        let outcome = session.apply_owner_candidate(&work.candidate_id, work.action.clone())?;
        if outcome.progress_steps.len() != 1 {
            return Err(format!(
                "oracle decision '{}' committed {} progress steps; expected one",
                work.candidate_id,
                outcome.progress_steps.len()
            ));
        }
        let mut journal = parent.journal;
        journal.append_committed_steps(outcome.progress_steps)?;
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
            state_fingerprint: run_session_fingerprint_v1(&session),
            boundary: classify_run_boundary(&session),
            replay,
            journal,
            session,
        };
        self.next_branch_id = self.next_branch_id.saturating_add(1);
        Ok(self.accept_branch(child))
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
        let nodes_expanded = pending.work.snapshot().nodes_expanded;
        let mut session = parent.session.clone();
        let outcome = pending
            .work
            .finish_and_apply(&mut session, finalization_deadline)?;
        if outcome.progress_steps.is_empty() {
            let rejection = outcome.combat_search_rejection.ok_or_else(|| {
                format!(
                    "oracle combat branch {} made no progress without typed rejection",
                    parent.branch_id
                )
            })?;
            self.unresolved_combats.push(OracleRunUnresolvedCombatV1 {
                branch_id: parent.branch_id,
                rejection,
                nodes_expanded,
            });
            return Ok(FinishedOracleCombatV1::Unresolved {
                branch_id: parent.branch_id,
            });
        }
        if outcome.progress_steps.len() != 1 {
            return Err(format!(
                "oracle combat branch {} committed {} progress steps; expected one",
                parent.branch_id,
                outcome.progress_steps.len()
            ));
        }
        let mut journal = parent.journal;
        journal.append_committed_steps(outcome.progress_steps)?;
        let child = OracleRunBranchV1 {
            branch_id: self.next_branch_id,
            parent_branch_id: Some(parent.branch_id),
            neow_root_candidate_id: parent.neow_root_candidate_id,
            neow_root_label: parent.neow_root_label,
            state_fingerprint: run_session_fingerprint_v1(&session),
            boundary: classify_run_boundary(&session),
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

pub fn seed_oracle_run_explorer_v1(
    expansion: NeowOracleExpansionV1,
) -> Result<OracleRunExplorerV1, String> {
    if !expansion.unresolved.is_empty() {
        return Err(format!(
            "cannot seed oracle run while {} Neow outcomes remain unresolved",
            expansion.unresolved.len()
        ));
    }
    let mut explorer = OracleRunExplorerV1::empty();
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
        for work in decision_work_for_branch(branch)? {
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
        if !has_decision && !has_combat {
            break OracleRunExploreStopV1::WorkExhausted;
        }

        let active_branch_id = if let Some(branch_id) = explorer.active_branch_id {
            branch_id
        } else {
            let branch_id = explorer.next_branch_with_work().expect("work exists");
            explorer.active_branch_id = Some(branch_id);
            branch_id
        };

        if let Some(mut pending) = explorer.take_combat_for_branch(active_branch_id) {
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
                    let finished = explorer.finish_combat(pending, deadline)?;
                    let rotate_nearby_siblings =
                        should_rotate_after_finished_combat(advance, &finished);
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
                            explorer.schedule_branch(branch_id, &budget.combat, true)?;
                            explorer.active_branch_id = Some(branch_id);
                        }
                        FinishedOracleCombatV1::Unresolved { branch_id } => {
                            if rotate_nearby_siblings {
                                explorer.rotate_nearest_ancestor_decisions_after_unresolved_combat(
                                    branch_id,
                                );
                            }
                            explorer.active_branch_id = None;
                        }
                        FinishedOracleCombatV1::ExactDuplicate => {
                            explorer.active_branch_id = None;
                        }
                    }
                }
            }
            continue;
        }

        let Some(decision) =
            explorer.take_decision_for_branch(active_branch_id, budget.decision_order)
        else {
            explorer.active_branch_id = None;
            continue;
        };
        let service_started = Instant::now();
        work_items = work_items.saturating_add(1);
        if let Some(branch_id) = explorer.materialize_decision(decision)? {
            let boundary = explorer
                .branches
                .iter()
                .find(|branch| branch.branch_id == branch_id)
                .map(|branch| branch.boundary)
                .ok_or_else(|| format!("missing materialized oracle branch {branch_id}"))?;
            if boundary == OracleRunBoundaryV1::TerminalVictory {
                break OracleRunExploreStopV1::Victory { branch_id };
            }
            explorer.schedule_branch(branch_id, &budget.combat, true)?;
            explorer.active_branch_id = Some(branch_id);
        } else {
            explorer.active_branch_id = None;
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

fn decision_work_for_branch(
    branch: &OracleRunBranchV1,
) -> Result<Vec<LazyOracleRunDecisionV1>, String> {
    let kind = work_kind(branch.boundary)?;
    if matches!(
        branch.session.engine_state,
        EngineState::RunPendingChoice(_)
    ) {
        return run_choice_work_for_branch(branch, kind);
    }
    let surface = build_decision_surface(&branch.session);
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
    Ok(work)
}

fn run_choice_work_for_branch(
    branch: &OracleRunBranchV1,
    kind: OracleRunWorkKindV1,
) -> Result<Vec<LazyOracleRunDecisionV1>, String> {
    let EngineState::RunPendingChoice(choice) = &branch.session.engine_state else {
        unreachable!("run choice work requires RunPendingChoice")
    };
    let request = choice.selection_request(&branch.session.run_state);
    let mut selections = Vec::new();
    for count in choice.min_choices..=choice.max_choices.min(request.targets.len()) {
        combinations(&request.targets, count, 0, &mut Vec::new(), &mut selections);
    }
    if selections.is_empty() && choice.min_choices == 0 {
        selections.push(Vec::new());
    }
    let surface = build_decision_surface(&branch.session);
    selections
        .into_iter()
        .map(|selected| {
            let action =
                RunDecisionAction::Input(ClientInput::SubmitSelection(SelectionResolution {
                    scope: SelectionScope::Deck,
                    selected,
                }));
            let candidate = surface
                .view
                .candidates
                .iter()
                .find(|candidate| candidate.action.executable_action().as_ref() == Some(&action))
                .or_else(|| {
                    surface.view.candidates.iter().find(|candidate| {
                        matches!(
                            candidate.key,
                            Some(DecisionCandidateKey::SelectionSubmit { .. })
                        )
                    })
                })
                .ok_or_else(|| {
                    "run choice has no bindable decision-surface candidate".to_string()
                })?;
            Ok(lazy_decision(
                branch,
                kind,
                candidate.id.clone(),
                candidate.label.clone(),
                action,
            ))
        })
        .collect()
}

fn lazy_decision(
    branch: &OracleRunBranchV1,
    kind: OracleRunWorkKindV1,
    candidate_id: String,
    label: String,
    action: RunDecisionAction,
) -> LazyOracleRunDecisionV1 {
    let stable_work_key = crate::eval::fingerprint::hash_serializable(&StableOracleWorkKeyInput {
        parent_state_fingerprint: &branch.state_fingerprint,
        candidate_id: &candidate_id,
        action: &action,
    });
    LazyOracleRunDecisionV1 {
        parent_branch_id: branch.branch_id,
        parent_state_fingerprint: branch.state_fingerprint.clone(),
        neow_root_candidate_id: branch.neow_root_candidate_id.clone(),
        kind,
        candidate_id,
        label,
        action,
        stable_work_key,
    }
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

fn combinations(
    targets: &[SelectionTargetRef],
    count: usize,
    start: usize,
    current: &mut Vec<SelectionTargetRef>,
    out: &mut Vec<Vec<SelectionTargetRef>>,
) {
    if current.len() == count {
        out.push(current.clone());
        return;
    }
    let remaining = count.saturating_sub(current.len());
    if targets.len().saturating_sub(start) < remaining {
        return;
    }
    for index in start..targets.len() {
        current.push(targets[index]);
        combinations(targets, count, index + 1, current, out);
        current.pop();
    }
}

fn run_session_fingerprint_v1(session: &RunControlSession) -> String {
    let mut normalized = session.clone();
    normalized.decision_step = 0;
    normalized.run_state.emitted_events.clear();
    normalized.combat_sequence = 0;
    normalized.auto_capture_last_combat_sequence = None;
    let mut checkpoint = RunControlSessionCheckpointV1::from_session(&normalized);
    checkpoint.clear_combat_diagnostics_for_external_checkpoint();
    crate::eval::fingerprint::hash_serializable(&checkpoint)
}

fn classify_run_boundary(session: &RunControlSession) -> OracleRunBoundaryV1 {
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
    use crate::eval::run_control::{expand_oracle_neow_candidates_v1, RunControlConfig};
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
        }
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
        let explorer = seed_oracle_run_explorer_v1(expansion).expect("oracle run seed");
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
    fn decision_order_changes_only_the_first_choice_and_keeps_fallbacks() {
        fn prefer_second(_: &RunControlSession) -> Vec<String> {
            vec!["second".to_string()]
        }

        let session = RunControlSession::new(RunControlConfig::default());
        let mut explorer = OracleRunExplorerV1::empty();
        explorer.branches.push(OracleRunBranchV1 {
            branch_id: 7,
            parent_branch_id: None,
            neow_root_candidate_id: "root".to_string(),
            neow_root_label: "root".to_string(),
            state_fingerprint: "state".to_string(),
            boundary: OracleRunBoundaryV1::MapDecision,
            replay: Vec::new(),
            journal: RunProgressJournalV1::default(),
            session,
        });
        for candidate_id in ["first", "second"] {
            explorer
                .pending_decisions
                .push_back(LazyOracleRunDecisionV1 {
                    parent_branch_id: 7,
                    parent_state_fingerprint: "state".to_string(),
                    neow_root_candidate_id: "root".to_string(),
                    kind: OracleRunWorkKindV1::MapTravel,
                    candidate_id: candidate_id.to_string(),
                    label: candidate_id.to_string(),
                    action: RunDecisionAction::Input(ClientInput::Proceed),
                    stable_work_key: candidate_id.to_string(),
                });
        }

        let preferred = explorer
            .take_decision_for_branch(7, Some(prefer_second))
            .expect("preferred candidate");
        assert_eq!(preferred.candidate_id, "second");
        assert_eq!(explorer.pending_decisions.len(), 1);

        let fallback = explorer
            .take_decision_for_branch(7, Some(prefer_second))
            .expect("unpreferred fallback remains available");
        assert_eq!(fallback.candidate_id, "first");
        assert!(explorer.pending_decisions.is_empty());
    }

    #[test]
    fn unresolved_combat_defers_only_the_nearest_ancestor_sibling_group() {
        let mut explorer = OracleRunExplorerV1::empty();
        explorer.branches = vec![
            test_branch(0, None),
            test_branch(1, Some(0)),
            test_branch(2, Some(1)),
            test_branch(7, None),
        ];
        explorer.pending_decisions = VecDeque::from([
            test_decision(1, "near/a"),
            test_decision(1, "near/b"),
            test_decision(0, "older"),
            test_decision(7, "unrelated"),
        ]);
        let before = explorer
            .pending_decisions
            .iter()
            .map(|work| work.stable_work_key.clone())
            .collect::<BTreeSet<_>>();

        let rotated = explorer.rotate_nearest_ancestor_decisions_after_unresolved_combat(2);

        assert_eq!(rotated, Some(1));
        assert_eq!(
            explorer
                .pending_decisions
                .iter()
                .map(|work| work.candidate_id.as_str())
                .collect::<Vec<_>>(),
            vec!["older", "unrelated", "near/a", "near/b"]
        );
        assert_eq!(
            explorer
                .pending_decisions
                .iter()
                .map(|work| work.stable_work_key.clone())
                .collect::<BTreeSet<_>>(),
            before
        );
    }

    #[test]
    fn unresolved_combat_falls_back_to_an_older_ancestor_without_losing_work() {
        let mut explorer = OracleRunExplorerV1::empty();
        explorer.branches = vec![
            test_branch(0, None),
            test_branch(1, Some(0)),
            test_branch(2, Some(1)),
            test_branch(7, None),
        ];
        explorer.pending_decisions = VecDeque::from([
            test_decision(0, "older/a"),
            test_decision(7, "unrelated"),
            test_decision(0, "older/b"),
        ]);

        let rotated = explorer.rotate_nearest_ancestor_decisions_after_unresolved_combat(2);

        assert_eq!(rotated, Some(0));
        assert_eq!(
            explorer
                .pending_decisions
                .iter()
                .map(|work| work.candidate_id.as_str())
                .collect::<Vec<_>>(),
            vec!["unrelated", "older/a", "older/b"]
        );
    }

    #[test]
    fn sibling_rotation_requires_both_allowance_exhaustion_and_typed_unresolved() {
        let unresolved = FinishedOracleCombatV1::Unresolved { branch_id: 2 };
        assert!(should_rotate_after_finished_combat(
            RunControlCombatWorkAdvanceV1::AllowanceExhausted,
            &unresolved
        ));
        assert!(!should_rotate_after_finished_combat(
            RunControlCombatWorkAdvanceV1::ReadyToFinish,
            &unresolved
        ));
        assert!(!should_rotate_after_finished_combat(
            RunControlCombatWorkAdvanceV1::AllowanceExhausted,
            &FinishedOracleCombatV1::Resolved(3)
        ));
        assert!(!should_rotate_after_finished_combat(
            RunControlCombatWorkAdvanceV1::AllowanceExhausted,
            &FinishedOracleCombatV1::ExactDuplicate
        ));
    }

    #[test]
    fn consecutive_combat_quanta_stay_on_the_active_branch_until_resolved() {
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
            .schedule_branch(branch_id, &combat_budgets, false)
            .expect("combat work should schedule");

        let result = drive_oracle_run_explorer_v1(
            explorer,
            OracleRunExploreBudgetV1 {
                max_work_items: 2,
                wall_ms: None,
                combat: combat_budgets,
                combat_quantum_nodes: 1,
                combat_quantum_ms: None,
                decision_order: None,
            },
        )
        .expect("one explorer quantum");

        assert_eq!(result.stop, OracleRunExploreStopV1::WorkBudgetExhausted);
        assert_eq!(result.combat_quanta, 2);
        assert_eq!(result.explorer.pending_combat_count(), 1);
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
    }
}
