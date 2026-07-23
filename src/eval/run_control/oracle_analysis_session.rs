use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_combat_planner::OracleCombatRootActionFamilySnapshot;

use crate::content::potions::Potion;
use crate::content::relics::RelicState;
use crate::content::{cards, monsters::EnemyId};
use crate::runtime::combat::CombatCard;
use crate::runtime::monster_move::MonsterMoveSpec;
use crate::sim::combat::CombatPosition;
use crate::state::core::ClientInput;

use crate::eval::combat_case::{
    CombatCase, CombatCaseGap, CombatCasePathStep, CombatCaseRngSummary, CombatCaseRunSummary,
    CombatCaseSource,
};

use super::oracle_combat_work::{
    OracleRunCombatWorkCheckpointV1, OracleRunCombatWorkProgressV1, OracleRunCombatWorkV1,
};
use super::oracle_run_explorer::{
    decision_work_for_branch, seed_oracle_run_explorer_from_checkpoint_v1, LazyOracleRunDecisionV1,
    OracleCombatSearchResumeKindV1, OracleRunBoundaryV1, OracleRunCombatBudgetsV1,
    OracleRunDecisionAnnotationFnV1, OracleRunDecisionOrderFnV1, OracleRunExplorerCheckpointV1,
    OracleRunExplorerV1, OracleRunReplayStepV1, OracleRunWorkKindV1,
};
use super::{
    CombatAutomationMonsterStateV1, CombatAutomationTrajectoryRecordV1,
    RunControlCombatSearchQuantum, RunControlCombatWorkAdvanceV1, RunControlSessionCheckpointV1,
    RunControlTraceAnnotationV1, RunDecisionAction, RunProgressJournalV1, RunProgressStepV1,
};

pub const ORACLE_ANALYSIS_SESSION_SCHEMA_NAME: &str = "OracleAnalysisSession";
pub const ORACLE_ANALYSIS_SESSION_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleAnalysisEdgeKindV1 {
    Decision,
    CombatWitness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisEdgeV1 {
    pub edge_id: u64,
    pub parent_node_id: usize,
    pub child_node_id: usize,
    pub kind: OracleAnalysisEdgeKindV1,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice_ref: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisChoiceViewV1 {
    pub choice_ref: String,
    pub kind: OracleRunWorkKindV1,
    pub candidate_id: String,
    pub label: String,
    pub action: RunDecisionAction,
    pub owner_rank: u64,
    pub path_discrepancy: u64,
    pub path_negative_log_policy: f64,
    pub annotation: Option<RunControlTraceAnnotationV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisNodeSummaryV1 {
    pub node_id: usize,
    pub canonical_parent_node_id: Option<usize>,
    pub boundary: OracleRunBoundaryV1,
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub replay_len: usize,
    pub is_cursor: bool,
    pub is_mainline_tip: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisTreeViewV1 {
    pub roots: Vec<usize>,
    pub cursor_node_id: usize,
    pub mainline_node_id: usize,
    pub nodes: Vec<OracleAnalysisNodeSummaryV1>,
    pub edges: Vec<OracleAnalysisEdgeV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisChildViewV1 {
    pub edge_id: u64,
    pub child_node_id: usize,
    pub kind: OracleAnalysisEdgeKindV1,
    pub label: String,
    pub is_on_mainline: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatProgressV1 {
    pub historical_generation_work: u64,
    pub current_search_generation_work: u64,
    pub generation_work: u64,
    pub exact_states: usize,
    pub completed_turn_options: usize,
    pub retained_state_work: usize,
    pub root_state: Option<sts_combat_planner::OracleCombatWitnessStateProgressSnapshot>,
    pub max_player_turn: u32,
    pub deepest_survival_state: Option<sts_combat_planner::OracleCombatDeepStateSnapshot>,
    pub deepest_progress_state: Option<sts_combat_planner::OracleCombatDeepStateSnapshot>,
    pub deepest_survival_actions: Vec<sts_combat_planner::TurnOptionAction>,
    pub deepest_progress_actions: Vec<sts_combat_planner::TurnOptionAction>,
    pub recent_turn_survival_envelope: Vec<sts_combat_planner::OracleCombatDeepStateSnapshot>,
    pub pending_witness_replay: bool,
    pub policy_witness_proposals: usize,
    pub advisor_nodes: u64,
    pub advisor_elapsed_ms: u64,
    pub advisor_active: bool,
    pub advisor_failure: Option<String>,
    pub incumbent_discovery_source: Option<sts_combat_planner::OracleCombatWitnessDiscoverySource>,
    pub incumbent_final_hp: Option<i32>,
    pub incumbent_hp_loss: Option<i32>,
    pub incumbent_action_count: Option<usize>,
    pub quantum_count: usize,
    pub remaining_nodes: usize,
    pub remaining_wall_ms: Option<u64>,
    pub resume_kind: OracleCombatSearchResumeKindV1,
    pub restart_count: usize,
    pub last_status: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisMonsterViewV1 {
    pub slot: u8,
    pub label: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub planned_move_id: u8,
    pub intent: Option<MonsterMoveSpec>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisEncounterViewV1 {
    pub turn: u32,
    pub phase: String,
    pub energy: u8,
    pub player_block: i32,
    pub hand: Vec<CombatCard>,
    pub draw_pile_count: usize,
    pub discard_pile_count: usize,
    pub exhaust_pile_count: usize,
    pub is_elite: bool,
    pub is_boss: bool,
    pub monsters: Vec<OracleAnalysisMonsterViewV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatTurnV1 {
    pub turn: u32,
    pub start_hp: i32,
    pub end_hp: i32,
    pub hp_loss: i32,
    pub ended_turn: bool,
    pub actions: Vec<String>,
    pub player_block_after: i32,
    pub monsters_after: Vec<CombatAutomationMonsterStateV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatSummaryV1 {
    pub node_id: usize,
    pub parent_node_id: usize,
    pub encounter_start_hp: i32,
    pub encounter_start_max_hp: i32,
    pub combat_end_hp: i32,
    pub post_combat_hp: i32,
    pub post_combat_max_hp: i32,
    pub combat_hp_loss: i32,
    pub post_combat_healing: i32,
    pub action_count: usize,
    pub turns: Vec<OracleAnalysisCombatTurnV1>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisEventViewV1 {
    pub id: String,
    pub screen: usize,
    pub completed: bool,
    pub combat_pending: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisNodeViewV1 {
    pub node_id: usize,
    pub canonical_parent_node_id: Option<usize>,
    pub is_cursor: bool,
    pub is_on_mainline: bool,
    pub boundary: OracleRunBoundaryV1,
    pub state_fingerprint: String,
    pub neow_root_label: String,
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub keys: [bool; 3],
    pub deck: Vec<CombatCard>,
    pub relics: Vec<RelicState>,
    pub potions: Vec<Option<Potion>>,
    pub replay_len: usize,
    pub recent_replay: Vec<OracleRunReplayStepV1>,
    pub choices: Vec<OracleAnalysisChoiceViewV1>,
    pub children: Vec<OracleAnalysisChildViewV1>,
    pub event: Option<OracleAnalysisEventViewV1>,
    pub encounter: Option<OracleAnalysisEncounterViewV1>,
    pub combat: Option<OracleAnalysisCombatProgressV1>,
}

fn oracle_analysis_choice_label(deck: &[CombatCard], choice: &LazyOracleRunDecisionV1) -> String {
    let RunDecisionAction::Input(ClientInput::SubmitSelection(resolution)) = &choice.action else {
        return choice.label.clone();
    };
    let selected = resolution
        .selected_card_uuids()
        .into_iter()
        .map(|uuid| {
            deck.iter()
                .find(|card| card.uuid == uuid)
                .map(|card| {
                    let upgrade = if card.upgrades == 0 {
                        String::new()
                    } else {
                        format!("+{}", card.upgrades)
                    };
                    format!("{}{} (#{uuid})", cards::java_id(card.id), upgrade)
                })
                .unwrap_or_else(|| format!("card #{uuid}"))
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        choice.label.clone()
    } else {
        format!("Select {}", selected.join(", "))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum OracleAnalysisAdvanceStatusV1 {
    SearchPending,
    BoundaryReached { child_node_id: usize },
    BudgetUnknown,
    ExhaustiveRefutation,
    SetupOrMechanicsError,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisAdvanceReportV1 {
    pub source_node_id: usize,
    pub status: OracleAnalysisAdvanceStatusV1,
    pub quanta_served: usize,
    pub elapsed_ms: u64,
    pub combat: Option<OracleAnalysisCombatProgressV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisAdvanceRequestV1 {
    pub max_quanta: usize,
    pub quantum_nodes: usize,
    pub quantum_ms: Option<u64>,
    pub wall_ms: Option<u64>,
}

impl Default for OracleAnalysisAdvanceRequestV1 {
    fn default() -> Self {
        Self {
            max_quanta: 1,
            quantum_nodes: 50_000,
            quantum_ms: Some(1_000),
            wall_ms: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisCombatJobCheckpointV1 {
    pub branch_id: usize,
    pub work: OracleRunCombatWorkCheckpointV1,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OracleAnalysisSessionCheckpointV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub cursor_node_id: usize,
    pub cursor_edge_path: Vec<u64>,
    pub mainline_node_id: usize,
    pub mainline_edge_path: Vec<u64>,
    pub next_edge_id: u64,
    pub edges: Vec<OracleAnalysisEdgeV1>,
    pub explorer: OracleRunExplorerCheckpointV1,
    pub combat_jobs: Vec<OracleAnalysisCombatJobCheckpointV1>,
}

pub struct OracleAnalysisSessionV1 {
    explorer: OracleRunExplorerV1,
    cursor_node_id: usize,
    cursor_edge_path: Vec<u64>,
    mainline_node_id: usize,
    mainline_edge_path: Vec<u64>,
    next_edge_id: u64,
    edges: Vec<OracleAnalysisEdgeV1>,
    combat_jobs: BTreeMap<usize, OracleRunCombatWorkV1>,
    combat_budgets: OracleRunCombatBudgetsV1,
    decision_order: Option<OracleRunDecisionOrderFnV1>,
    decision_annotation: Option<OracleRunDecisionAnnotationFnV1>,
}

impl OracleAnalysisSessionV1 {
    pub fn from_explorer(
        mut explorer: OracleRunExplorerV1,
        preferred_cursor_node_id: Option<usize>,
        combat_budgets: OracleRunCombatBudgetsV1,
        decision_order: Option<OracleRunDecisionOrderFnV1>,
        decision_annotation: Option<OracleRunDecisionAnnotationFnV1>,
    ) -> Result<Self, String> {
        let cursor_node_id = preferred_cursor_node_id
            .filter(|branch_id| {
                explorer
                    .branches
                    .iter()
                    .any(|branch| branch.branch_id == *branch_id)
            })
            .or_else(|| {
                explorer
                    .branches
                    .iter()
                    .max_by_key(|branch| {
                        (
                            branch.session.run_state.act_num,
                            branch.session.run_state.floor_num,
                            branch.journal.len(),
                            branch.branch_id,
                        )
                    })
                    .map(|branch| branch.branch_id)
            })
            .ok_or_else(|| "oracle analysis session requires at least one branch".to_string())?;
        let combat_jobs = explorer
            .drain_pending_combats()
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        let mut session = Self {
            explorer,
            cursor_node_id,
            cursor_edge_path: Vec::new(),
            mainline_node_id: cursor_node_id,
            mainline_edge_path: Vec::new(),
            next_edge_id: 0,
            edges: Vec::new(),
            combat_jobs,
            combat_budgets,
            decision_order,
            decision_annotation,
        };
        session.seed_canonical_edges();
        session.cursor_edge_path = session.path_to_node(cursor_node_id).ok_or_else(|| {
            format!("analysis cursor node {cursor_node_id} is not reachable from any root")
        })?;
        session.mainline_edge_path = session.cursor_edge_path.clone();
        Ok(session)
    }

    pub fn restore(
        checkpoint: OracleAnalysisSessionCheckpointV1,
        combat_budgets: OracleRunCombatBudgetsV1,
        decision_order: Option<OracleRunDecisionOrderFnV1>,
        decision_annotation: Option<OracleRunDecisionAnnotationFnV1>,
    ) -> Result<Self, String> {
        if checkpoint.schema_name != ORACLE_ANALYSIS_SESSION_SCHEMA_NAME
            || checkpoint.schema_version != ORACLE_ANALYSIS_SESSION_SCHEMA_VERSION
        {
            return Err("unsupported oracle analysis session schema".to_string());
        }
        let explorer =
            seed_oracle_run_explorer_from_checkpoint_v1(checkpoint.explorer, &combat_budgets)?;
        let mut combat_jobs = BTreeMap::new();
        for saved in checkpoint.combat_jobs {
            let branch = explorer
                .branches
                .iter()
                .find(|branch| branch.branch_id == saved.branch_id)
                .ok_or_else(|| {
                    format!(
                        "analysis combat job references missing node {}",
                        saved.branch_id
                    )
                })?;
            let work = OracleRunCombatWorkV1::restart_from_checkpoint(
                &branch.session,
                combat_budgets.for_session(&branch.session),
                saved.work,
            )?;
            if combat_jobs.insert(saved.branch_id, work).is_some() {
                return Err(format!(
                    "analysis checkpoint duplicated combat node {}",
                    saved.branch_id
                ));
            }
        }
        let session = Self {
            explorer,
            cursor_node_id: checkpoint.cursor_node_id,
            cursor_edge_path: checkpoint.cursor_edge_path,
            mainline_node_id: checkpoint.mainline_node_id,
            mainline_edge_path: checkpoint.mainline_edge_path,
            next_edge_id: checkpoint.next_edge_id,
            edges: checkpoint.edges,
            combat_jobs,
            combat_budgets,
            decision_order,
            decision_annotation,
        };
        session.validate_navigation_state()?;
        Ok(session)
    }

    pub fn checkpoint(&self) -> Result<OracleAnalysisSessionCheckpointV1, String> {
        self.validate_navigation_state()?;
        Ok(OracleAnalysisSessionCheckpointV1 {
            schema_name: ORACLE_ANALYSIS_SESSION_SCHEMA_NAME.to_string(),
            schema_version: ORACLE_ANALYSIS_SESSION_SCHEMA_VERSION,
            cursor_node_id: self.cursor_node_id,
            cursor_edge_path: self.cursor_edge_path.clone(),
            mainline_node_id: self.mainline_node_id,
            mainline_edge_path: self.mainline_edge_path.clone(),
            next_edge_id: self.next_edge_id,
            edges: self.edges.clone(),
            explorer: self.explorer.analysis_checkpoint()?,
            combat_jobs: self
                .combat_jobs
                .iter()
                .map(|(branch_id, work)| OracleAnalysisCombatJobCheckpointV1 {
                    branch_id: *branch_id,
                    work: work.checkpoint(),
                })
                .collect(),
        })
    }

    pub fn cursor_node_id(&self) -> usize {
        self.cursor_node_id
    }

    pub fn mainline_node_id(&self) -> usize {
        self.mainline_node_id
    }

    pub fn root_node_ids(&self) -> Vec<usize> {
        let branch_ids = self
            .explorer
            .branches
            .iter()
            .map(|branch| branch.branch_id)
            .collect::<BTreeSet<_>>();
        self.explorer
            .branches
            .iter()
            .filter(|branch| {
                branch
                    .parent_branch_id
                    .is_none_or(|parent| !branch_ids.contains(&parent))
            })
            .map(|branch| branch.branch_id)
            .collect()
    }

    pub fn focus_node(&mut self, node_id: usize) -> Result<(), String> {
        self.require_branch(node_id)?;
        self.cursor_node_id = node_id;
        self.cursor_edge_path = self
            .path_to_node(node_id)
            .ok_or_else(|| format!("analysis node {node_id} is not reachable from any root"))?;
        Ok(())
    }

    pub fn follow_edge(&mut self, edge_id: u64) -> Result<(), String> {
        let edge = self
            .edges
            .iter()
            .find(|edge| edge.edge_id == edge_id)
            .ok_or_else(|| format!("unknown oracle analysis edge {edge_id}"))?;
        if edge.parent_node_id != self.cursor_node_id {
            return Err(format!(
                "analysis edge {edge_id} starts at node {}, cursor is node {}",
                edge.parent_node_id, self.cursor_node_id
            ));
        }
        self.cursor_node_id = edge.child_node_id;
        self.cursor_edge_path.push(edge.edge_id);
        Ok(())
    }

    pub fn back(&mut self) -> Result<usize, String> {
        let edge_id = self
            .cursor_edge_path
            .pop()
            .ok_or_else(|| "oracle analysis cursor is already at a root".to_string())?;
        let edge = self
            .edges
            .iter()
            .find(|edge| edge.edge_id == edge_id)
            .ok_or_else(|| format!("analysis cursor references missing edge {edge_id}"))?;
        self.cursor_node_id = edge.parent_node_id;
        Ok(self.cursor_node_id)
    }

    pub fn promote_cursor(&mut self) {
        self.mainline_node_id = self.cursor_node_id;
        self.mainline_edge_path = self.cursor_edge_path.clone();
    }

    pub fn replay(&self, node_id: usize) -> Result<Vec<OracleRunReplayStepV1>, String> {
        Ok(self.require_branch(node_id)?.replay.clone())
    }

    pub fn journal_entries(&self, node_id: usize) -> Result<&[RunProgressStepV1], String> {
        Ok(self.require_branch(node_id)?.journal.entries())
    }

    pub fn continuation_parts(
        &self,
        node_id: usize,
    ) -> Result<(RunProgressJournalV1, RunControlSessionCheckpointV1), String> {
        let branch = self.require_branch(node_id)?;
        Ok((
            branch.journal.clone(),
            RunControlSessionCheckpointV1::from_session(&branch.session),
        ))
    }

    pub fn combat_trajectory(
        &self,
        node_id: usize,
    ) -> Result<Option<&CombatAutomationTrajectoryRecordV1>, String> {
        let branch = self.require_branch(node_id)?;
        Ok(branch
            .journal
            .entries()
            .iter()
            .rev()
            .find_map(RunProgressStepV1::as_combat_resolution)
            .map(|resolution| &resolution.trajectory)
            .or_else(|| branch.session.last_combat_automation_trajectory()))
    }

    pub fn combat_summary(&self, node_id: usize) -> Result<OracleAnalysisCombatSummaryV1, String> {
        let branch = self.require_branch(node_id)?;
        let parent_node_id = branch
            .parent_branch_id
            .ok_or_else(|| format!("oracle node {node_id} has no parent combat boundary"))?;
        let parent = self.require_branch(parent_node_id)?;
        let trajectory = self
            .combat_trajectory(node_id)?
            .ok_or_else(|| format!("oracle node {node_id} has no recorded combat trajectory"))?;
        let encounter_start_hp = parent.session.run_state.current_hp;
        let encounter_start_max_hp = parent.session.run_state.max_hp;
        let mut last_hp = encounter_start_hp;
        let mut active_turn: Option<OracleAnalysisCombatTurnV1> = None;
        let mut turns = Vec::new();

        for action in &trajectory.actions {
            let turn = action
                .opportunity_before
                .as_ref()
                .map(|opportunity| opportunity.turn)
                .unwrap_or_else(|| active_turn.as_ref().map(|turn| turn.turn).unwrap_or(0));
            if active_turn
                .as_ref()
                .is_some_and(|summary| summary.turn != turn)
            {
                turns.push(active_turn.take().expect("active turn checked above"));
            }
            let summary = active_turn.get_or_insert_with(|| OracleAnalysisCombatTurnV1 {
                turn,
                start_hp: last_hp,
                end_hp: last_hp,
                hp_loss: 0,
                ended_turn: false,
                actions: Vec::new(),
                player_block_after: 0,
                monsters_after: Vec::new(),
            });
            summary.actions.push(action.action_key.clone());
            if let Some(after) = &action.combat_after {
                last_hp = after.player_hp;
                summary.end_hp = last_hp;
                summary.hp_loss = summary.start_hp.saturating_sub(last_hp).max(0);
                summary.player_block_after = after.player_block;
                summary.monsters_after = after.monsters.clone();
            }
            if matches!(action.input, crate::state::core::ClientInput::EndTurn) {
                summary.ended_turn = true;
            }
        }
        if let Some(summary) = active_turn {
            turns.push(summary);
        }

        let post_combat_hp = branch.session.run_state.current_hp;
        Ok(OracleAnalysisCombatSummaryV1 {
            node_id,
            parent_node_id,
            encounter_start_hp,
            encounter_start_max_hp,
            combat_end_hp: last_hp,
            post_combat_hp,
            post_combat_max_hp: branch.session.run_state.max_hp,
            combat_hp_loss: encounter_start_hp.saturating_sub(last_hp).max(0),
            post_combat_healing: post_combat_hp.saturating_sub(last_hp).max(0),
            action_count: trajectory.action_count,
            turns,
        })
    }

    pub(crate) fn combat_root_action_families(
        &self,
        node_id: usize,
    ) -> Result<Vec<OracleCombatRootActionFamilySnapshot>, String> {
        self.combat_jobs
            .get(&node_id)
            .map(OracleRunCombatWorkV1::root_action_families)
            .ok_or_else(|| format!("oracle node {node_id} has no resident combat search"))
    }

    pub fn combat_case(
        &self,
        node_id: usize,
        seed: u64,
        ascension: u8,
        search_nodes: usize,
        search_ms: u64,
    ) -> Result<CombatCase, String> {
        let branch = self.require_branch(node_id)?;
        let position: CombatPosition = branch.session.current_active_combat_position()?;
        let path: Vec<CombatCasePathStep> = branch
            .journal
            .entries()
            .iter()
            .filter_map(RunProgressStepV1::as_decision)
            .map(|record| CombatCasePathStep {
                key: Value::Null,
                label: record.result.chosen_label.clone(),
                state_before: Some(json!({
                    "title": record.before.title,
                    "location": record.before.location,
                })),
                decision_evidence: Some(json!({
                    "candidate_id": record.selection.candidate_id,
                    "source": record.selection.source,
                    "candidates": record.before.candidates.iter().map(|candidate| &candidate.label).collect::<Vec<_>>(),
                })),
            })
            .collect();
        Ok(CombatCase::new(
            CombatCaseSource {
                seed,
                ascension,
                generation: path.len(),
                branch_id: branch.branch_id,
                parent_id: branch.parent_branch_id,
            },
            CombatCaseGap {
                boundary: format!(
                    "Act {} Floor {} oracle analysis combat",
                    branch.session.run_state.act_num, branch.session.run_state.floor_num
                ),
                reason: "oracle_analysis_export".to_string(),
                search_nodes,
                search_ms,
                rescue_search_nodes: 0,
                rescue_search_ms: 0,
            },
            CombatCaseRunSummary {
                act: branch.session.run_state.act_num,
                floor: branch.session.run_state.floor_num,
                hp: branch.session.run_state.current_hp,
                max_hp: branch.session.run_state.max_hp,
                gold: branch.session.run_state.gold,
                deck_size: branch.session.run_state.master_deck.len(),
                relic_count: branch.session.run_state.relics.len(),
                potion_slots: branch.session.run_state.potions.len(),
            },
            Vec::new(),
            None,
            path,
            CombatCaseRngSummary::from_pool(&branch.session.run_state.rng_pool),
            position,
        ))
    }

    pub fn tree(&self) -> OracleAnalysisTreeViewV1 {
        OracleAnalysisTreeViewV1 {
            roots: self.root_node_ids(),
            cursor_node_id: self.cursor_node_id,
            mainline_node_id: self.mainline_node_id,
            nodes: self
                .explorer
                .branches
                .iter()
                .map(|branch| OracleAnalysisNodeSummaryV1 {
                    node_id: branch.branch_id,
                    canonical_parent_node_id: branch.parent_branch_id,
                    boundary: branch.boundary,
                    act: branch.session.run_state.act_num,
                    floor: branch.session.run_state.floor_num,
                    current_hp: branch.session.run_state.current_hp,
                    max_hp: branch.session.run_state.max_hp,
                    gold: branch.session.run_state.gold,
                    replay_len: branch.replay.len(),
                    is_cursor: branch.branch_id == self.cursor_node_id,
                    is_mainline_tip: branch.branch_id == self.mainline_node_id,
                })
                .collect(),
            edges: self.edges.clone(),
        }
    }

    pub fn view_cursor(&self) -> Result<OracleAnalysisNodeViewV1, String> {
        self.view_node(self.cursor_node_id)
    }

    pub fn view_node(&self, node_id: usize) -> Result<OracleAnalysisNodeViewV1, String> {
        let branch = self.require_branch(node_id)?;
        let mut choices = if matches!(
            branch.boundary,
            OracleRunBoundaryV1::Combat
                | OracleRunBoundaryV1::TerminalVictory
                | OracleRunBoundaryV1::TerminalDefeat
        ) {
            Vec::new()
        } else {
            decision_work_for_branch(branch, self.decision_order)?
        };
        choices.sort_by(|left, right| {
            left.path_discrepancy
                .cmp(&right.path_discrepancy)
                .then_with(|| {
                    left.path_negative_log_policy
                        .total_cmp(&right.path_negative_log_policy)
                })
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });
        let deck = &branch.session.run_state.master_deck;
        let choices = choices
            .into_iter()
            .map(|choice| {
                let label = oracle_analysis_choice_label(deck, &choice);
                OracleAnalysisChoiceViewV1 {
                    choice_ref: choice_ref(&choice),
                    kind: choice.kind,
                    candidate_id: choice.candidate_id.clone(),
                    label,
                    action: choice.action.clone(),
                    owner_rank: choice
                        .path_discrepancy
                        .saturating_sub(branch.path_discrepancy),
                    path_discrepancy: choice.path_discrepancy,
                    path_negative_log_policy: choice.path_negative_log_policy,
                    annotation: self
                        .decision_annotation
                        .and_then(|annotate| annotate(&branch.session, &choice.candidate_id)),
                }
            })
            .collect();
        let mainline_edges = self
            .mainline_edge_path
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let children = self
            .edges
            .iter()
            .filter(|edge| edge.parent_node_id == node_id)
            .map(|edge| OracleAnalysisChildViewV1 {
                edge_id: edge.edge_id,
                child_node_id: edge.child_node_id,
                kind: edge.kind,
                label: edge.label.clone(),
                is_on_mainline: mainline_edges.contains(&edge.edge_id),
            })
            .collect();
        let replay_len = branch.replay.len();
        let recent_replay = branch
            .replay
            .iter()
            .skip(replay_len.saturating_sub(12))
            .cloned()
            .collect();
        let run = &branch.session.run_state;
        let event = run
            .event_state
            .as_ref()
            .map(|event| OracleAnalysisEventViewV1 {
                id: format!("{:?}", event.id),
                screen: event.current_screen,
                completed: event.completed,
                combat_pending: event.combat_pending,
            });
        let encounter = branch.session.active_combat.as_ref().map(|active| {
            let combat = &active.combat_state;
            OracleAnalysisEncounterViewV1 {
                turn: combat.turn.turn_count,
                phase: format!("{:?}", combat.turn.current_phase),
                energy: combat.turn.energy,
                player_block: combat.entities.player.block,
                hand: combat.zones.hand.clone(),
                draw_pile_count: combat.zones.draw_pile.len(),
                discard_pile_count: combat.zones.discard_pile.len(),
                exhaust_pile_count: combat.zones.exhaust_pile.len(),
                is_elite: combat.meta.is_elite_fight,
                is_boss: combat.meta.is_boss_fight,
                monsters: combat
                    .entities
                    .monsters
                    .iter()
                    .map(|monster| OracleAnalysisMonsterViewV1 {
                        slot: monster.slot,
                        label: EnemyId::from_id(monster.monster_type)
                            .map(|enemy| enemy.get_name().to_string())
                            .unwrap_or_else(|| format!("monster_type:{}", monster.monster_type)),
                        current_hp: monster.current_hp,
                        max_hp: monster.max_hp,
                        block: monster.block,
                        alive: !monster.is_dead_or_escaped(),
                        planned_move_id: monster.planned_move_id(),
                        intent: monster.move_state.planned_visible_spec.clone(),
                    })
                    .collect(),
            }
        });
        Ok(OracleAnalysisNodeViewV1 {
            node_id,
            canonical_parent_node_id: branch.parent_branch_id,
            is_cursor: node_id == self.cursor_node_id,
            is_on_mainline: node_id == self.mainline_node_id
                || self
                    .mainline_edge_path
                    .iter()
                    .filter_map(|edge_id| {
                        self.edges
                            .iter()
                            .find(|edge| edge.edge_id == *edge_id)
                            .map(|edge| edge.parent_node_id)
                    })
                    .any(|parent| parent == node_id),
            boundary: branch.boundary,
            state_fingerprint: branch.state_fingerprint.clone(),
            neow_root_label: branch.neow_root_label.clone(),
            act: run.act_num,
            floor: run.floor_num,
            current_hp: run.current_hp,
            max_hp: run.max_hp,
            gold: run.gold,
            keys: run.keys,
            deck: run.master_deck.clone(),
            relics: run.relics.clone(),
            potions: run.potions.clone(),
            replay_len,
            recent_replay,
            choices,
            children,
            event,
            encounter,
            combat: self.combat_progress(node_id),
        })
    }

    pub fn try_choice(&mut self, requested_ref: &str) -> Result<usize, String> {
        let (parent_node_id, _) = parse_choice_ref(requested_ref)?;
        let parent = self.require_branch(parent_node_id)?;
        let work = decision_work_for_branch(parent, self.decision_order)?
            .into_iter()
            .find(|work| choice_ref(work) == requested_ref)
            .ok_or_else(|| {
                format!("choice reference is stale or is not legal at node {parent_node_id}")
            })?;
        let label = work.label.clone();
        self.explorer.remove_pending_decision(&work.stable_work_key);
        let child_node_id = self
            .explorer
            .materialize_explicit_decision(work, self.decision_annotation)?;
        let edge_id = self.record_edge(
            parent_node_id,
            child_node_id,
            OracleAnalysisEdgeKindV1::Decision,
            label,
            Some(requested_ref.to_string()),
        );
        self.move_cursor_after_edge(parent_node_id, edge_id, child_node_id);
        Ok(child_node_id)
    }

    pub fn advance_cursor(
        &mut self,
        request: OracleAnalysisAdvanceRequestV1,
    ) -> Result<OracleAnalysisAdvanceReportV1, String> {
        if request.max_quanta == 0 || request.quantum_nodes == 0 {
            return Err("oracle analysis advance requires positive quantum budgets".to_string());
        }
        let source_node_id = self.cursor_node_id;
        let branch = self.require_branch(source_node_id)?;
        if branch.boundary != OracleRunBoundaryV1::Combat {
            return Err(format!(
                "oracle analysis node {source_node_id} is at {:?}, not combat",
                branch.boundary
            ));
        }
        if !self.combat_jobs.contains_key(&source_node_id) {
            let work = OracleRunCombatWorkV1::new(
                &branch.session,
                self.combat_budgets.for_session(&branch.session),
            )?;
            self.combat_jobs.insert(source_node_id, work);
        } else {
            let work = self
                .combat_jobs
                .get_mut(&source_node_id)
                .expect("analysis combat job exists");
            work.mark_search_resume_exact();
            let requested_nodes = request.quantum_nodes.saturating_mul(request.max_quanta);
            let requested_wall_ms = request.wall_ms.or_else(|| {
                request.quantum_ms.map(|quantum_ms| {
                    quantum_ms.saturating_mul(u64::try_from(request.max_quanta).unwrap_or(u64::MAX))
                })
            });
            work.ensure_requested_allowance(
                requested_nodes,
                requested_wall_ms.map(Duration::from_millis),
            );
        }
        let started = Instant::now();
        let deadline = request
            .wall_ms
            .and_then(|wall_ms| started.checked_add(Duration::from_millis(wall_ms)));
        let quantum = RunControlCombatSearchQuantum {
            label: "oracle_analysis_session",
            additional_nodes: request.quantum_nodes,
            soft_wall_ms: request.quantum_ms,
        };
        let mut quanta_served = 0usize;
        let mut ready_to_finish = false;
        let mut allowance_exhausted = false;
        for _ in 0..request.max_quanta {
            let work = self
                .combat_jobs
                .get_mut(&source_node_id)
                .expect("analysis combat job inserted above");
            match work.advance(&quantum, deadline) {
                RunControlCombatWorkAdvanceV1::Pending => {
                    quanta_served = quanta_served.saturating_add(1);
                }
                RunControlCombatWorkAdvanceV1::GlobalDeadlineReached => break,
                RunControlCombatWorkAdvanceV1::ReadyToFinish => {
                    quanta_served = quanta_served.saturating_add(1);
                    ready_to_finish = true;
                    break;
                }
                RunControlCombatWorkAdvanceV1::AllowanceExhausted => {
                    quanta_served = quanta_served.saturating_add(1);
                    allowance_exhausted = true;
                    break;
                }
            }
            if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                break;
            }
        }
        if allowance_exhausted {
            return Ok(OracleAnalysisAdvanceReportV1 {
                source_node_id,
                status: OracleAnalysisAdvanceStatusV1::BudgetUnknown,
                quanta_served,
                elapsed_ms: elapsed_ms(started),
                combat: self.combat_progress(source_node_id),
            });
        }
        if !ready_to_finish {
            return Ok(OracleAnalysisAdvanceReportV1 {
                source_node_id,
                status: OracleAnalysisAdvanceStatusV1::SearchPending,
                quanta_served,
                elapsed_ms: elapsed_ms(started),
                combat: self.combat_progress(source_node_id),
            });
        }

        let work = self
            .combat_jobs
            .remove(&source_node_id)
            .expect("ready analysis combat job exists");
        let final_progress = combat_progress_view(&work);
        let child_node_id = self.materialize_combat_work(source_node_id, work)?;
        let status = if let Some(child_node_id) = child_node_id {
            OracleAnalysisAdvanceStatusV1::BoundaryReached { child_node_id }
        } else {
            match self
                .explorer
                .unresolved_combats
                .iter()
                .rev()
                .find(|unresolved| unresolved.branch_id == source_node_id)
                .map(|unresolved| unresolved.evidence_kind)
            {
                Some("exhaustive_refutation") => {
                    OracleAnalysisAdvanceStatusV1::ExhaustiveRefutation
                }
                Some("setup_or_mechanics_error") => {
                    OracleAnalysisAdvanceStatusV1::SetupOrMechanicsError
                }
                _ => OracleAnalysisAdvanceStatusV1::BudgetUnknown,
            }
        };
        Ok(OracleAnalysisAdvanceReportV1 {
            source_node_id,
            status,
            quanta_served,
            elapsed_ms: elapsed_ms(started),
            combat: Some(final_progress),
        })
    }

    /// Commits the current combat's already verified incumbent without asking
    /// the search to spend more quality-improvement budget. This is an
    /// explicit analyst action; BudgetUnknown never commits itself.
    pub fn accept_cursor_combat_incumbent(&mut self) -> Result<usize, String> {
        let source_node_id = self.cursor_node_id;
        let branch = self.require_branch(source_node_id)?;
        if branch.boundary != OracleRunBoundaryV1::Combat {
            return Err(format!(
                "oracle analysis node {source_node_id} is at {:?}, not combat",
                branch.boundary
            ));
        }
        let Some(work) = self.combat_jobs.remove(&source_node_id) else {
            return Err(format!(
                "oracle analysis node {source_node_id} has no resident combat search"
            ));
        };
        if !work.has_verified_witness() {
            self.combat_jobs.insert(source_node_id, work);
            return Err(format!(
                "oracle analysis node {source_node_id} has no verified combat incumbent"
            ));
        }
        self.materialize_combat_work(source_node_id, work)?
            .ok_or_else(|| "verified combat incumbent did not materialize a child".to_string())
    }

    pub fn accept_cursor_combat_actions(
        &mut self,
        actions: &[ClientInput],
    ) -> Result<usize, String> {
        let source_node_id = self.cursor_node_id;
        let mut work = if let Some(work) = self.combat_jobs.remove(&source_node_id) {
            work
        } else {
            let branch = self.require_branch(source_node_id)?;
            if branch.boundary != OracleRunBoundaryV1::Combat {
                return Err(format!(
                    "oracle analysis node {source_node_id} is at {:?}, not combat",
                    branch.boundary
                ));
            }
            OracleRunCombatWorkV1::restart_from_exact_state(
                &branch.session,
                self.combat_budgets.for_session(&branch.session),
            )?
        };
        if let Err(error) = work.verify_and_restore_action_witness(actions) {
            self.combat_jobs.insert(source_node_id, work);
            return Err(error);
        }
        self.materialize_combat_work(source_node_id, work)?
            .ok_or_else(|| "verified combat action witness did not materialize a child".to_string())
    }

    /// Discards only the cursor combat's retained search work and starts a
    /// fresh tactical job from the same exact simulator state. Historical run
    /// state, journal entries, siblings, and navigation remain unchanged.
    pub fn restart_cursor_combat_search(&mut self) -> Result<(), String> {
        let node_id = self.cursor_node_id;
        let work = {
            let branch = self.require_branch(node_id)?;
            if branch.boundary != OracleRunBoundaryV1::Combat {
                return Err(format!(
                    "oracle analysis node {node_id} is at {:?}, not combat",
                    branch.boundary
                ));
            }
            OracleRunCombatWorkV1::restart_from_exact_state(
                &branch.session,
                self.combat_budgets.for_session(&branch.session),
            )?
        };
        self.combat_jobs.insert(node_id, work);
        Ok(())
    }

    fn materialize_combat_work(
        &mut self,
        source_node_id: usize,
        work: OracleRunCombatWorkV1,
    ) -> Result<Option<usize>, String> {
        let child_node_id = self
            .explorer
            .materialize_explicit_combat(source_node_id, work)?;
        if let Some(child_node_id) = child_node_id {
            let child = self.require_branch(child_node_id)?;
            let edge_id = self.record_edge(
                source_node_id,
                child_node_id,
                OracleAnalysisEdgeKindV1::CombatWitness,
                format!(
                    "combat witness -> {} HP",
                    child.session.run_state.current_hp
                ),
                None,
            );
            self.move_cursor_after_edge(source_node_id, edge_id, child_node_id);
        }
        Ok(child_node_id)
    }

    fn require_branch(
        &self,
        node_id: usize,
    ) -> Result<&super::oracle_run_explorer::OracleRunBranchV1, String> {
        self.explorer
            .branches
            .iter()
            .find(|branch| branch.branch_id == node_id)
            .ok_or_else(|| format!("unknown oracle analysis node {node_id}"))
    }

    fn combat_progress(&self, node_id: usize) -> Option<OracleAnalysisCombatProgressV1> {
        self.combat_jobs.get(&node_id).map(combat_progress_view)
    }

    fn seed_canonical_edges(&mut self) {
        let parents = self
            .explorer
            .branches
            .iter()
            .filter_map(|branch| {
                branch
                    .parent_branch_id
                    .map(|parent| (parent, branch.branch_id))
            })
            .collect::<Vec<_>>();
        for (parent, child) in parents {
            if self
                .edges
                .iter()
                .any(|edge| edge.parent_node_id == parent && edge.child_node_id == child)
            {
                continue;
            }
            let label = self
                .edge_label_from_branches(parent, child)
                .unwrap_or_else(|| "continued variation".to_string());
            self.record_edge(
                parent,
                child,
                OracleAnalysisEdgeKindV1::Decision,
                label,
                None,
            );
        }
    }

    fn edge_label_from_branches(&self, parent: usize, child: usize) -> Option<String> {
        let parent = self.require_branch(parent).ok()?;
        let child = self.require_branch(child).ok()?;
        if child.replay.len() > parent.replay.len() {
            child.replay.last().map(|step| step.label.clone())
        } else if child.boundary != parent.boundary
            || child.session.run_state.current_hp != parent.session.run_state.current_hp
        {
            Some(format!(
                "combat witness -> {} HP",
                child.session.run_state.current_hp
            ))
        } else {
            None
        }
    }

    fn record_edge(
        &mut self,
        parent_node_id: usize,
        child_node_id: usize,
        kind: OracleAnalysisEdgeKindV1,
        label: String,
        choice_ref: Option<String>,
    ) -> u64 {
        if let Some(existing) = self.edges.iter().find(|edge| {
            edge.parent_node_id == parent_node_id
                && edge.child_node_id == child_node_id
                && edge.kind == kind
                && edge.choice_ref == choice_ref
        }) {
            return existing.edge_id;
        }
        let edge_id = self.next_edge_id;
        self.next_edge_id = self.next_edge_id.saturating_add(1);
        self.edges.push(OracleAnalysisEdgeV1 {
            edge_id,
            parent_node_id,
            child_node_id,
            kind,
            label,
            choice_ref,
        });
        edge_id
    }

    fn move_cursor_after_edge(
        &mut self,
        parent_node_id: usize,
        edge_id: u64,
        child_node_id: usize,
    ) {
        if self.cursor_node_id != parent_node_id {
            self.cursor_edge_path = self
                .path_to_node(parent_node_id)
                .expect("materialized analysis parent is reachable");
        }
        self.cursor_edge_path.push(edge_id);
        self.cursor_node_id = child_node_id;
    }

    fn path_to_node(&self, target: usize) -> Option<Vec<u64>> {
        if !self
            .explorer
            .branches
            .iter()
            .any(|branch| branch.branch_id == target)
        {
            return None;
        }
        let roots = self.root_node_ids();
        let mut queue = roots
            .iter()
            .map(|root| (*root, Vec::<u64>::new()))
            .collect::<VecDeque<_>>();
        let mut visited = BTreeSet::new();
        while let Some((node, path)) = queue.pop_front() {
            if !visited.insert(node) {
                continue;
            }
            if node == target {
                return Some(path);
            }
            for edge in self.edges.iter().filter(|edge| edge.parent_node_id == node) {
                let mut child_path = path.clone();
                child_path.push(edge.edge_id);
                queue.push_back((edge.child_node_id, child_path));
            }
        }
        None
    }

    fn validate_navigation_state(&self) -> Result<(), String> {
        self.require_branch(self.cursor_node_id)?;
        self.require_branch(self.mainline_node_id)?;
        let roots = self.root_node_ids().into_iter().collect::<BTreeSet<_>>();
        if self.cursor_edge_path.is_empty() && !roots.contains(&self.cursor_node_id) {
            return Err(format!(
                "analysis cursor node {} has no path from a root",
                self.cursor_node_id
            ));
        }
        if self.mainline_edge_path.is_empty() && !roots.contains(&self.mainline_node_id) {
            return Err(format!(
                "analysis mainline node {} has no path from a root",
                self.mainline_node_id
            ));
        }
        validate_edge_path(
            &self.edges,
            self.cursor_node_id,
            &self.cursor_edge_path,
            "cursor",
        )?;
        validate_edge_path(
            &self.edges,
            self.mainline_node_id,
            &self.mainline_edge_path,
            "mainline",
        )?;
        Ok(())
    }
}

fn choice_ref(work: &LazyOracleRunDecisionV1) -> String {
    format!(
        "choice-v1/{}/{}",
        work.parent_branch_id, work.stable_work_key
    )
}

fn parse_choice_ref(value: &str) -> Result<(usize, &str), String> {
    let mut parts = value.splitn(3, '/');
    if parts.next() != Some("choice-v1") {
        return Err("unsupported oracle analysis choice reference".to_string());
    }
    let node = parts
        .next()
        .ok_or_else(|| "choice reference is missing its node".to_string())?
        .parse::<usize>()
        .map_err(|_| "choice reference contains an invalid node".to_string())?;
    let key = parts
        .next()
        .filter(|key| !key.is_empty())
        .ok_or_else(|| "choice reference is missing its fingerprint".to_string())?;
    Ok((node, key))
}

fn combat_progress_view(work: &OracleRunCombatWorkV1) -> OracleAnalysisCombatProgressV1 {
    let progress: OracleRunCombatWorkProgressV1 = work.progress();
    OracleAnalysisCombatProgressV1 {
        historical_generation_work: progress.historical_generation_work,
        current_search_generation_work: progress.current_search_generation_work,
        generation_work: progress.generation_work,
        exact_states: progress.exact_states,
        completed_turn_options: progress.completed_turn_options,
        retained_state_work: progress.retained_state_work,
        root_state: progress.root_state,
        max_player_turn: progress.max_player_turn,
        deepest_survival_state: progress.deepest_survival_state,
        deepest_progress_state: progress.deepest_progress_state,
        deepest_survival_actions: progress.deepest_survival_actions,
        deepest_progress_actions: progress.deepest_progress_actions,
        recent_turn_survival_envelope: progress.recent_turn_survival_envelope,
        pending_witness_replay: progress.pending_witness_replay,
        policy_witness_proposals: progress.policy_witness_proposals,
        advisor_nodes: progress.advisor_nodes,
        advisor_elapsed_ms: progress.advisor_elapsed_ms,
        advisor_active: progress.advisor_active,
        advisor_failure: progress.advisor_failure,
        incumbent_discovery_source: progress.incumbent_discovery_source,
        incumbent_final_hp: progress.incumbent_final_hp,
        incumbent_hp_loss: progress.incumbent_hp_loss,
        incumbent_action_count: progress.incumbent_action_count,
        quantum_count: work.quantum_count(),
        remaining_nodes: work.remaining_nodes(),
        remaining_wall_ms: work.remaining_wall_ms(),
        resume_kind: if work.restart_count() > 0 {
            OracleCombatSearchResumeKindV1::StateReplayExactSearchRestarted
        } else if work.search_resume_exact() {
            OracleCombatSearchResumeKindV1::SearchResumeExact
        } else {
            OracleCombatSearchResumeKindV1::Fresh
        },
        restart_count: work.restart_count(),
        last_status: progress.last_status,
    }
}

fn validate_edge_path(
    edges: &[OracleAnalysisEdgeV1],
    expected_tip: usize,
    path: &[u64],
    label: &str,
) -> Result<(), String> {
    let Some(first_edge_id) = path.first() else {
        return Ok(());
    };
    let first = edges
        .iter()
        .find(|edge| edge.edge_id == *first_edge_id)
        .ok_or_else(|| format!("analysis {label} path references missing edge {first_edge_id}"))?;
    let mut node = first.parent_node_id;
    for edge_id in path {
        let edge = edges
            .iter()
            .find(|edge| edge.edge_id == *edge_id)
            .ok_or_else(|| format!("analysis {label} path references missing edge {edge_id}"))?;
        if edge.parent_node_id != node {
            return Err(format!(
                "analysis {label} path is disconnected before edge {edge_id}"
            ));
        }
        node = edge.child_node_id;
    }
    if node != expected_tip {
        return Err(format!(
            "analysis {label} path ends at node {node}, expected {expected_tip}"
        ));
    }
    Ok(())
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}
